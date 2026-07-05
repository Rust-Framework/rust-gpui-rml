//! Tooltip 通用属性 setter
//!
//! Tooltip 在 gpui-component 中通过组件的 `.tooltip(impl Into<SharedString>)` 方法使用，
//! 而非独立元素。支持 `.tooltip()` 的组件：Button、Checkbox、Clipboard、DropdownButton、
//! Toggle、Radio、Switch。
//!
//! ## 属性映射
//!
//! - `tooltip="Save file"` → `.tooltip("Save file")`
//! - `tooltip={tooltip_text}` → `.tooltip(self.tooltip_text.clone())`

/// Tooltip 静态属性 setter
///
/// 将 `tooltip="text"` 映射为 `.tooltip("text")`。
/// 适用于所有实现了 `.tooltip(impl Into<SharedString>)` 的组件。
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    if name != "tooltip" {
        return None;
    }
    // 仅在支持 tooltip 的组件上生成 setter，避免对不支持的组件误调
    if !supports_tooltip(tag) {
        return None;
    }
    Some(format!(".tooltip({:?})", value))
}

/// Tooltip 绑定属性 setter
///
/// 将 `tooltip={expr}` 映射为 `.tooltip(rust_expr)`。
/// `rust_expr` 应为已由 `component_bind_rust_expr` 转换的 Rust 表达式（如 `self.field` / `self.method()`）。
pub fn bind_setter(name: &str, rust_expr: &str, tag: &str) -> Option<String> {
    if name != "tooltip" {
        return None;
    }
    if !supports_tooltip(tag) {
        return None;
    }
    Some(format!(".tooltip({})", rust_expr))
}

/// 检查组件是否支持 `.tooltip()` 方法
pub fn supports_tooltip(tag: &str) -> bool {
    matches!(
        tag,
        "Button"
            | "IconButton"
            | "DropdownButton"
            | "Toggle"
            | "Checkbox"
            | "Clipboard"
            | "Radio"
            | "Switch"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_tooltip_on_button() {
        let s = static_setter("tooltip", "Save file", "Button");
        assert_eq!(s, Some(r#".tooltip("Save file")"#.to_string()));
    }

    #[test]
    fn static_setter_tooltip_on_checkbox() {
        let s = static_setter("tooltip", "Accept terms", "Checkbox");
        assert_eq!(s, Some(r#".tooltip("Accept terms")"#.to_string()));
    }

    #[test]
    fn static_setter_tooltip_unsupported_component() {
        // Input 不支持 tooltip，应返回 None
        let s = static_setter("tooltip", "text", "Input");
        assert_eq!(s, None);
    }

    #[test]
    fn static_setter_non_tooltip_attribute() {
        let s = static_setter("label", "Save", "Button");
        assert_eq!(s, None);
    }

    #[test]
    fn bind_setter_tooltip_on_button() {
        // rust_expr 已由 component_bind_rust_expr 转换为 self.tooltip_text
        let s = bind_setter("tooltip", "self.tooltip_text", "Button");
        assert_eq!(s, Some(".tooltip(self.tooltip_text)".to_string()));
    }

    #[test]
    fn bind_setter_tooltip_unsupported() {
        let s = bind_setter("tooltip", "self.text", "Input");
        assert_eq!(s, None);
    }
}
