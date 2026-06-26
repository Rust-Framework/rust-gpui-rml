//! `IComponent` trait —— RML 组件基础契约
//!
//! 所有可在 `.rml` 中使用的 UI 类型均实现此 trait。
//! `#[component]` 宏自动实现。
//!
//! 合并自旧 `IRmlView`（`rml_template()`）+ 旧 `IComponent`（`rml_tag()`）。

use crate::view_model::IViewModel;

/// RML 组件基础 trait。
///
/// 组件拥有：
/// - `.rml` 模板路径（定义视觉结构）
/// - 标签名（PascalCase，用于 `.rml` 中的 `<MyComponent>` 引用）
///
/// 由 `#[component]` 宏自动实现。
///
/// # 组件与窗口的区别
///
/// - 组件可被嵌套，窗口是顶层
/// - 组件支持插槽，窗口不支持
/// - 窗口（`IWindow`）继承自 `IComponent`，额外拥有窗口生命周期操作
pub trait IComponent: IViewModel {
    /// 关联的 `.rml` 模板路径（相对于 `src` 目录）。
    /// 由 `#[component]` 宏根据命名约定或 `template=` 参数生成。
    fn rml_template() -> &'static str;

    /// 组件标签名（PascalCase），用于 `.rml` 中的 `<MyComponent>`。
    /// 默认返回结构体名，由 `#[component]` 宏生成。
    fn rml_tag() -> &'static str;
}
