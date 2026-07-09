//! menu / MenuBar 专用 bind setter。
//!
//! 由 `component::component_bind_setter` 在 tag 匹配 menu 类标签时委托调用。
//!
//! 框架不定义 IMenuItem/IStatusBarItem 数据结构，`items={...}` 绑定路径已移除。
//! 业务侧经命令式 `render_menu_bar()` / `render_status_bar()` 构建。

/// menu / MenuBar 专用 bind setter
///
/// `items` 绑定已移除（框架不定义 IMenuItem）；返回 `None` 透传到通用 setter。
pub fn bind_setter(
    _name: &str,
    _expr_str: &str,
    _loop_vars: &[&str],
    _computed: &[&str],
    _tag: &str,
) -> Option<String> {
    None
}
