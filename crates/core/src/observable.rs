//! 版本号驱动的可观察集合。
//!
//! `ObservableVec<T>` 使用 `RwLock<Vec<T>>` 提供 `&self` 安全的 mutation，
//! 配合 `AtomicU64` version 实现变更检测，可选 `flume::Sender<()>` 实现 UI 通知。
//!
//! 典型用法：host Entity 在 `on_loaded` 中创建 `flume::unbounded::<()>()` channel，
//! 将 `Sender` 传入 `ObservableVec::with_notifier(tx)`，并在后台任务中消费 `Receiver`
//! 调用 `cx.notify()` 触发重渲。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use flume::Sender;

/// 版本号驱动的可观察集合。
///
/// - `push`/`insert`/`remove` 等 mutation 方法为 `&self`（内部 `RwLock` 写锁）
/// - 每次 mutation 自动 `fetch_add(1)` version + 可选发送 channel 通知
/// - `read()` 返回 `RwLockReadGuard`，`version()` 为 lock-free 读取
///
/// 不实现 `DerefMut`——强制通过 mutation 方法修改，确保 version bump。
pub struct ObservableVec<T> {
    inner: RwLock<Vec<T>>,
    version: AtomicU64,
    notify: Option<Sender<()>>,
}

impl<T> ObservableVec<T> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            version: AtomicU64::new(0),
            notify: None,
        }
    }

    /// 创建带通知 channel 的 `ObservableVec`。
    /// mutation 方法会发送 `()` 到 channel，供后台任务接收后调用 `cx.notify()`。
    pub fn with_notifier(notify: Sender<()>) -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            version: AtomicU64::new(0),
            notify: Some(notify),
        }
    }

    fn bump(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
        if let Some(tx) = &self.notify {
            let _ = tx.send(());
        }
    }

    // —— mutation 方法（&self，内部 RwLock 写锁）——

    pub fn push(&self, value: T) {
        self.inner.write().unwrap().push(value);
        self.bump();
    }

    pub fn insert(&self, index: usize, value: T) {
        self.inner.write().unwrap().insert(index, value);
        self.bump();
    }

    pub fn remove(&self, index: usize) -> T {
        let v = self.inner.write().unwrap().remove(index);
        self.bump();
        v
    }

    pub fn swap(&self, a: usize, b: usize) {
        let mut guard = self.inner.write().unwrap();
        guard.swap(a, b);
        drop(guard);
        self.bump();
    }

    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
        self.bump();
    }

    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut guard = self.inner.write().unwrap();
        let before = guard.len();
        guard.retain(f);
        let after = guard.len();
        drop(guard);
        if before != after {
            self.bump();
        }
    }

    pub fn sort_by_mut<F>(&self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.inner.write().unwrap().sort_by(compare);
        self.bump();
    }

    // —— 只读方法（&self，内部 RwLock 读锁或 lock-free）——

    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<T>> {
        self.inner.read().unwrap()
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }
}

impl<T> Default for ObservableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_bumps_version() {
        let v: ObservableVec<i32> = ObservableVec::new();
        assert_eq!(v.version(), 0);
        v.push(1);
        assert_eq!(v.version(), 1);
        v.push(2);
        assert_eq!(v.version(), 2);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn retain_bumps_version_when_changed() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        let ver_before = v.version();
        v.retain(|x| *x > 1);
        assert!(v.version() > ver_before);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn retain_no_bump_when_unchanged() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.push(1);
        let ver_before = v.version();
        v.retain(|_| true);
        assert_eq!(v.version(), ver_before);
    }

    #[test]
    fn notifier_sends_on_mutation() {
        let (tx, rx) = flume::unbounded::<()>();
        let v = ObservableVec::with_notifier(tx);
        v.push(42);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn no_notifier_does_not_panic() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.push(1);
        v.clear();
        assert_eq!(v.version(), 2);
    }

    #[test]
    fn read_returns_guard() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.push(10);
        v.push(20);
        let guard = v.read();
        assert_eq!(*guard, vec![10, 20]);
    }
}
