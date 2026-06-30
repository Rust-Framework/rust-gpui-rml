//! `ComputedCache` —— `#[computed]` 方法的缓存存储
//!
//! Phase B-2：为每个 `#[computed]` 方法提供基于版本号的缓存。
//! 当依赖字段的版本号未变时，直接命中缓存返回克隆值；
//! 版本号变化时调用原方法重算并写入缓存。
//!
//! ## Send + Sync 保证
//!
//! GPUI `Entity<T>` 要求 `T: Send + Sync`。`ComputedCache` 使用
//! `Mutex<HashMap<...>>` 提供线程同步，并通过 `unsafe impl Send + Sync`
//! 满足约束。这是因为缓存值通过 `Box<dyn Any>` 类型擦除存储，
//! 其中可能包含非 `Send` 的 GPUI 类型（如 `Vec<TabItem>` 含 `Rc`）。
//!
//! **安全性保证**：
//! - `Mutex` 确保同一时刻只有一个线程访问缓存内容
//! - `#[computed]` 方法仅在 GPUI render 线程调用
//! - 缓存值不会被移动到其他线程（仅克隆返回）
//!
//! ## 嵌套调用安全
//!
//! `get_or_compute` 在调用 `compute` 前释放 `MutexGuard`，
//! 支持 `#[computed] A` 调用 `#[computed] B` 的嵌套场景，避免死锁。

use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

/// `#[computed]` 方法的运行时缓存
///
/// codegen 为每个 `#[computed]` 方法生成的包装方法通过此类型
/// 实现基于版本号的缓存命中/失效。
///
/// # 示例（codegen 生成代码示意）
///
/// ```rust,ignore
/// pub fn doubled(&self) -> i32 {
///     let v = self.__rml_computed_deps_version("doubled");
///     self.__rml_computed_cache.get_or_compute("doubled", v, || self.__rml_computed_doubled())
/// }
/// ```
pub struct ComputedCache {
    inner: Mutex<HashMap<String, (u64, Box<dyn Any>)>>,
}

// SAFETY: ComputedCache 通过 Mutex 提供线程同步。
// 缓存值通过 Box<dyn Any> 类型擦除存储，可能包含非 Send 的 GPUI 类型（如 Vec<TabItem>）。
// 在 GPUI 模型中，#[computed] 方法仅在 render 线程调用，缓存值不会被移动到其他线程。
#[allow(unsafe_code)]
unsafe impl Send for ComputedCache {}
#[allow(unsafe_code)]
unsafe impl Sync for ComputedCache {}

impl ComputedCache {
    /// 创建空缓存
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 命中缓存返回克隆值；未命中调用 `compute` 计算并写入缓存。
    ///
    /// 关键：`compute` 在 `MutexGuard` 释放后执行，避免 `#[computed]`
    /// 嵌套调用导致死锁（A 调 B 时 B 再次 `get_or_compute` 同一缓存）。
    ///
    /// # 类型约束
    ///
    /// - `T: Clone`：缓存命中时返回克隆值（避免返回引用穿过 `MutexGuard`）
    /// - `T: 'static`：满足 `Box<dyn Any>` 类型擦除存储
    pub fn get_or_compute<T: Clone + 'static>(
        &self,
        key: &str,
        version: u64,
        compute: impl FnOnce() -> T,
    ) -> T {
        // 1. 尝试命中（锁内只读）
        {
            let inner = self.inner.lock().unwrap();
            if let Some((cached_ver, cached_val)) = inner.get(key) {
                if *cached_ver == version {
                    return cached_val.downcast_ref::<T>().unwrap().clone();
                }
            }
        } // ← MutexGuard 释放

        // 2. 锁外计算（支持嵌套调用）
        let value = compute();

        // 3. 写入缓存
        let mut inner = self.inner.lock().unwrap();
        inner.insert(key.to_string(), (version, Box::new(value.clone())));
        value
    }

    /// 失效单个缓存项
    pub fn invalidate(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }

    /// 清空所有缓存
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

impl Default for ComputedCache {
    fn default() -> Self {
        Self::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[test]
    fn cache_miss_computes_and_stores() {
        let cache = ComputedCache::new();
        let call_count = AtomicU64::new(0);

        let v1: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            42
        });
        assert_eq!(v1, 42);
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn cache_hit_returns_clone_without_recompute() {
        let cache = ComputedCache::new();
        let call_count = AtomicU64::new(0);

        // 首次计算
        let v1: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            42
        });
        assert_eq!(v1, 42);

        // 版本未变：命中缓存，不重算
        let v2: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            999 // 不会用到
        });
        assert_eq!(v2, 42);
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn version_change_triggers_recompute() {
        let cache = ComputedCache::new();
        let call_count = AtomicU64::new(0);

        // v=1：计算 42
        let v1: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            42
        });
        assert_eq!(v1, 42);

        // v=2：版本变化，重算为 100
        let v2: i32 = cache.get_or_compute("x", 2, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            100
        });
        assert_eq!(v2, 100);
        assert_eq!(call_count.load(Ordering::Relaxed), 2);

        // v=2 再次：命中缓存（100）
        let v3: i32 = cache.get_or_compute("x", 2, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            -1
        });
        assert_eq!(v3, 100);
        assert_eq!(call_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn different_keys_independent() {
        let cache = ComputedCache::new();

        let a: i32 = cache.get_or_compute("a", 1, || 1);
        let b: i32 = cache.get_or_compute("b", 1, || 2);

        assert_eq!(a, 1);
        assert_eq!(b, 2);

        // 修改 a 的版本不影响 b
        let a2: i32 = cache.get_or_compute("a", 2, || 10);
        assert_eq!(a2, 10);

        let b2: i32 = cache.get_or_compute("b", 1, || 999);
        assert_eq!(b2, 2); // b 仍命中缓存
    }

    #[test]
    fn invalidate_forces_recompute() {
        let cache = ComputedCache::new();
        let call_count = AtomicU64::new(0);

        let v1: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            42
        });
        assert_eq!(v1, 42);

        cache.invalidate("x");

        let v2: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            100
        });
        assert_eq!(v2, 100);
        assert_eq!(call_count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn clear_wipes_all_entries() {
        let cache = ComputedCache::new();

        let _: i32 = cache.get_or_compute("a", 1, || 1);
        let _: i32 = cache.get_or_compute("b", 1, || 2);

        cache.clear();

        let call_count = AtomicU64::new(0);
        let a: i32 = cache.get_or_compute("a", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            10
        });
        assert_eq!(a, 10);
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn works_with_clone_types() {
        let cache = ComputedCache::new();

        let v1: Vec<String> =
            cache.get_or_compute("vec", 1, || vec!["hello".to_string(), "world".to_string()]);
        assert_eq!(v1, vec!["hello".to_string(), "world".to_string()]);

        // 命中缓存返回克隆
        let v2: Vec<String> = cache.get_or_compute("vec", 1, || vec!["should".to_string(), "not".to_string(), "compute".to_string()]);
        assert_eq!(v2, vec!["hello".to_string(), "world".to_string()]);
    }

    #[test]
    fn nested_get_or_compute_no_deadlock() {
        // 模拟 #[computed] A 调用 #[computed] B 的场景
        let cache = ComputedCache::new();
        let inner_cache = &cache;

        let outer: i32 = cache.get_or_compute("outer", 1, || {
            // 外层 compute 期间，内层也调用同一 cache
            let inner: i32 = inner_cache.get_or_compute("inner", 1, || 10);
            inner + 5
        });
        assert_eq!(outer, 15);

        // 再次访问 outer 应命中缓存
        let outer2: i32 = cache.get_or_compute("outer", 1, || 999);
        assert_eq!(outer2, 15);
    }

    #[test]
    fn default_is_empty() {
        let cache = ComputedCache::default();
        let call_count = AtomicU64::new(0);
        let v: i32 = cache.get_or_compute("x", 1, || {
            call_count.fetch_add(1, Ordering::Relaxed);
            42
        });
        assert_eq!(v, 42);
        assert_eq!(call_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn send_sync_bounds() {
        // 编译期验证 Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ComputedCache>();
    }
}
