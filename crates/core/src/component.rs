//! `IComponent` trait —— 可复用组件契约
//!
//! `#[component]` 标记的结构体实现此 trait，可在 `.rml` 中作为
//! PascalCase 标签被父视图引用。
//! 详见文档 §6.2 自定义组件。

use crate::view::IRmlView;

/// 可复用组件 trait。
///
/// 组件与视图（`#[view]`）的区别：
/// - 组件可被嵌套，视图是顶层
/// - 组件支持插槽，视图不支持
/// - 组件不能作为 `RmlApplication::run` 的根视图
pub trait IComponent: IRmlView {
    /// 组件标签名（PascalCase），用于 `.rml` 中的 `<MyComponent>`。
    /// 默认返回结构体名，由 `#[component]` 宏生成。
    fn rml_tag() -> &'static str;
}
