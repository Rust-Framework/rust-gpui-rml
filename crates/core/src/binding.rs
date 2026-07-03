//! 绑定路径与绑定上下文
//!
//! 编译期解析 `{user.name}` 等绑定表达式为 `BindingPath`，
//! 运行时通过 `IBindingContext` 建立订阅关系。
//! 详见文档 §3.6 绑定引擎原理。
//!
//! ## `IBindingContext` 演进（Phase B-3）
//!
//! 从 marker trait 扩展为订阅管理接口，新增 `record_version` / `is_field_changed`
//! 默认方法。默认实现保持 marker 行为（不检测变更），用户可通过 [`BindingContext`]
//! 获得基于版本号快照 diff 的真实实现。
//!
//! ## 关于 codegen 的 sum-of-versions
//!
//! `#[computed]` 缓存键当前采用依赖字段版本号之和（`__rml_computed_deps_version`）。
//! 由于 `__rml_bump_version` 通过 `fetch_add(1, Relaxed)` 单调递增，版本号永不回退，
//! 因此 sum 在任何依赖字段变更时必然变化——sum 等价于 per-field diff 的
//! 「任一依赖字段变更」语义。codegen 不切换到 per-field diff，sum 已经是正确的。
//!
//! 本 trait 的方法供用户代码（如自定义订阅、诊断）opt-in 使用，不影响 codegen 默认行为。

use std::collections::HashMap;
use std::sync::Mutex;

/// 绑定路径段
#[derive(Debug, Clone, PartialEq)]
pub enum BindingSegment {
    /// ViewModel 字段
    Field(String),
    /// 嵌套字段访问（`a.b` 中的 `b`）
    Member(String),
    /// 索引访问（`items[0]`）
    Index(usize),
    /// 方法调用（`items.len()`）
    MethodCall(String),
}

/// 绑定路径，由编译期从 `{a.b.c}` 解析而来
#[derive(Debug, Clone, PartialEq)]
pub struct BindingPath {
    pub segments: Vec<BindingSegment>,
}

impl BindingPath {
    /// 从点分字符串创建绑定路径（`"user.name"` → `[Field("user"), Member("name")]`）
    pub fn parse(expr: &str) -> Self {
        let segments = expr
            .split('.')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, s)| {
                if i == 0 {
                    BindingSegment::Field(s)
                } else {
                    BindingSegment::Member(s)
                }
            })
            .collect();
        Self { segments }
    }

    /// 根路径字段名
    pub fn root_field(&self) -> Option<&str> {
        match self.segments.first()? {
            BindingSegment::Field(s) | BindingSegment::Member(s) => Some(s),
            _ => None,
        }
    }
}

/// 绑定上下文 trait（运行时由 View 持有，供绑定引擎使用）
///
/// Phase B-3：从 marker trait 扩展为订阅管理接口。新增两个默认方法：
///
/// - [`record_version`](Self::record_version)：记录字段当前版本号到快照
/// - [`is_field_changed`](Self::is_field_changed)：查询字段当前版本号是否与快照不同
///
/// 默认实现保持 marker 行为（不记录、不检测），用户可通过 [`BindingContext`] 获得
/// 真实的 per-field 变更检测能力。
///
/// # codegen 不使用本 trait
///
/// `#[computed]` 缓存键采用 sum-of-versions（`__rml_computed_deps_version`），
/// 由于版本号单调递增，sum 已正确等价于「任一依赖字段变更」。本 trait 供用户代码
/// opt-in 使用（如自定义订阅、诊断），codegen 不切换到 per-field diff。
pub trait IBindingContext {
    /// 标记绑定已建立
    fn bind(&mut self, path: &BindingPath);

    /// 记录字段当前版本号到快照（建立下次比较基准）
    ///
    /// 默认实现无操作，保持 marker trait 行为。
    /// [`BindingContext`] 提供了基于 `Mutex<HashMap>` 的真实实现。
    fn record_version(&mut self, _field: &str, _version: u64) {}

    /// 查询字段当前版本号是否与上次 [`record_version`](Self::record_version) 记录的版本不同
    ///
    /// 默认实现返回 `false`（保持 marker 行为，不报告变更）。
    ///
    /// # 返回值语义
    ///
    /// - `true`：字段已变更（或从未记录过快照），调用方应重算
    /// - `false`：字段未变更，调用方可跳过重算
    ///
    /// 默认实现返回 `false`，意味着未 override 时假定「未变更」——
    /// 这保留了 marker trait 的「无操作」语义。需要真实检测时使用 [`BindingContext`]。
    fn is_field_changed(&self, _field: &str, _current_version: u64) -> bool {
        false
    }
}

/// 默认的 `IBindingContext` 实现：基于 `Mutex<HashMap<String, u64>>` 存储版本号快照
///
/// Send + Sync 兼容（通过 `Mutex`），可嵌入 `Entity<T>` 要求 `T: Send + Sync` 的 ViewModel。
///
/// # 使用示例
///
/// ```rust,ignore
/// #[component]
/// struct Counter {
///     pub count: i32,
///     binding_ctx: BindingContext,
/// }
///
/// impl Counter {
///     pub fn check(&mut self) -> bool {
///         let v = self.__rml_get_version("count");
///         let changed = self.binding_ctx.is_field_changed("count", v);
///         self.binding_ctx.record_version("count", v);
///         changed
///     }
/// }
/// ```
pub struct BindingContext {
    snapshots: Mutex<HashMap<String, u64>>,
}

impl BindingContext {
    /// 创建空快照上下文
    pub fn new() -> Self {
        Self {
            snapshots: Mutex::new(HashMap::new()),
        }
    }

    /// 清空所有快照（重置为初始状态）
    pub fn clear(&self) {
        self.snapshots.lock().unwrap().clear();
    }
}

impl Default for BindingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl IBindingContext for BindingContext {
    fn bind(&mut self, _path: &BindingPath) {
        // 默认实现不维护订阅列表；bind 仅作为语义标记
    }

    fn record_version(&mut self, field: &str, version: u64) {
        self.snapshots
            .lock()
            .unwrap()
            .insert(field.to_string(), version);
    }

    fn is_field_changed(&self, field: &str, current_version: u64) -> bool {
        match self.snapshots.lock().unwrap().get(field) {
            Some(&v) => v != current_version,
            None => true,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_path_parse_simple_field() {
        let p = BindingPath::parse("count");
        assert_eq!(p.segments, vec![BindingSegment::Field("count".into())]);
        assert_eq!(p.root_field(), Some("count"));
    }

    #[test]
    fn binding_path_parse_nested() {
        let p = BindingPath::parse("user.name");
        assert_eq!(
            p.segments,
            vec![
                BindingSegment::Field("user".into()),
                BindingSegment::Member("name".into()),
            ]
        );
        assert_eq!(p.root_field(), Some("user"));
    }

    #[test]
    fn binding_path_parse_empty_segments_filtered() {
        let p = BindingPath::parse(".user..name.");
        assert_eq!(
            p.segments,
            vec![
                BindingSegment::Field("user".into()),
                BindingSegment::Member("name".into()),
            ]
        );
    }

    #[test]
    fn binding_path_root_field_for_index_and_method() {
        let p = BindingPath {
            segments: vec![BindingSegment::Index(0)],
        };
        assert_eq!(p.root_field(), None);

        let p = BindingPath {
            segments: vec![BindingSegment::MethodCall("len".into())],
        };
        assert_eq!(p.root_field(), None);
    }

    /// stub impl：仅实现 `bind`，使用 trait 默认方法
    struct StubContext;
    impl IBindingContext for StubContext {
        fn bind(&mut self, _path: &BindingPath) {}
    }

    #[test]
    fn default_record_version_is_noop() {
        let mut ctx = StubContext;
        ctx.record_version("count", 1);
        // 默认实现不存储，is_field_changed 仍返回 false
        assert!(!ctx.is_field_changed("count", 1));
        assert!(!ctx.is_field_changed("count", 999));
    }

    #[test]
    fn default_is_field_changed_returns_false() {
        let ctx = StubContext;
        // 无论版本号如何，默认实现始终返回 false
        assert!(!ctx.is_field_changed("count", 0));
        assert!(!ctx.is_field_changed("count", 1));
        assert!(!ctx.is_field_changed("any_field", 42));
    }

    #[test]
    fn binding_context_records_and_diffs() {
        let mut ctx = BindingContext::new();

        // 初始：无快照，is_field_changed 返回 true
        assert!(ctx.is_field_changed("count", 0));

        // 记录快照 v=1
        ctx.record_version("count", 1);

        // 当前版本仍为 1：未变更
        assert!(!ctx.is_field_changed("count", 1));

        // 当前版本变为 2：已变更
        assert!(ctx.is_field_changed("count", 2));
    }

    #[test]
    fn binding_context_independent_fields() {
        let mut ctx = BindingContext::new();

        ctx.record_version("a", 10);
        ctx.record_version("b", 20);

        // a 未变，b 未变
        assert!(!ctx.is_field_changed("a", 10));
        assert!(!ctx.is_field_changed("b", 20));

        // a 变，b 不变
        assert!(ctx.is_field_changed("a", 11));
        assert!(!ctx.is_field_changed("b", 20));
    }

    #[test]
    fn binding_context_overwrite_snapshot() {
        let mut ctx = BindingContext::new();

        ctx.record_version("count", 1);
        assert!(!ctx.is_field_changed("count", 1));

        // 覆盖快照为 v=2
        ctx.record_version("count", 2);
        assert!(!ctx.is_field_changed("count", 2));
        assert!(ctx.is_field_changed("count", 1));
    }

    #[test]
    fn binding_context_clear_resets() {
        let mut ctx = BindingContext::new();
        ctx.record_version("count", 1);
        assert!(!ctx.is_field_changed("count", 1));

        ctx.clear();

        // 清空后视为无快照，返回 true
        assert!(ctx.is_field_changed("count", 1));
    }

    #[test]
    fn binding_context_default_is_empty() {
        let ctx = BindingContext::default();
        // 无快照时任何字段视为变更
        assert!(ctx.is_field_changed("any", 0));
    }

    #[test]
    fn binding_context_send_sync_bounds() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BindingContext>();
    }

    #[test]
    fn binding_context_bind_is_noop() {
        let mut ctx = BindingContext::new();
        // bind 不影响 is_field_changed
        ctx.bind(&BindingPath::parse("count"));
        assert!(ctx.is_field_changed("count", 0));
    }
}
