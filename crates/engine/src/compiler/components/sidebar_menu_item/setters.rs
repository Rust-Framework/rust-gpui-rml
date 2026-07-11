//! SidebarMenuItem 专用属性 setter
//!
//! ## 属性映射
//!
//! - `icon` → `.icon(rml_ui::IconName::X)`（IconName 字符串）
//! - `active` → `.active(bool)`（parse_bool_enabled）
//! - `default_open` → `.default_open(bool)`（parse_bool_enabled）
//! - `click_to_open` → `.click_to_open(bool)`（parse_bool_enabled）
//! - `click_to_toggle` → `.click_to_toggle(bool)`（parse_bool_enabled）
//! - `disabled` → `.disable(bool)`（注意：方法名是 `disable`，非 `disabled`；parse_bool_enabled）
//!
//! 注意：`label` 属性由构造器处理（`SidebarMenuItem::new(label)`），不在此处 setter。
//! `on_click` 事件由通用 `component_event_setter` 处理。

/// SidebarMenuItem 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "icon" => Some(format!(".icon(rml_ui::IconName::{})", value)),
        "active" => Some(format!(".active({})", parse_bool_enabled(value))),
        "default_open" => Some(format!(".default_open({})", parse_bool_enabled(value))),
        "click_to_open" => Some(format!(".click_to_open({})", parse_bool_enabled(value))),
        "click_to_toggle" => Some(format!(".click_to_toggle({})", parse_bool_enabled(value))),
        // 注意：SidebarMenuItem 的方法名是 `disable`，非 `disabled`
        "disabled" => Some(format!(".disable({})", parse_bool_enabled(value))),
        _ => None,
    }
}

/// SidebarMenuItem 专用 bind 属性 setter
pub fn bind_setter(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str]) -> Option<String> {
    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, loop_vars, computed);
    match name {
        "active" => Some(format!(".active({})", rust_expr)),
        "default_open" => Some(format!(".default_open({})", rust_expr)),
        "disabled" => Some(format!(".disable({})", rust_expr)),
        _ => None,
    }
}

/// parse_bool_enabled: 空值或 "true" → "true"，"false" → "false"
fn parse_bool_enabled(value: &str) -> &'static str {
    if value.is_empty() || value.eq_ignore_ascii_case("true") {
        "true"
    } else {
        "false"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_home() {
        assert_eq!(
            static_setter("icon", "Home"),
            Some(".icon(rml_ui::IconName::Home)".to_string())
        );
    }

    #[test]
    fn active_presence() {
        assert_eq!(static_setter("active", ""), Some(".active(true)".to_string()));
    }

    #[test]
    fn active_true() {
        assert_eq!(static_setter("active", "true"), Some(".active(true)".to_string()));
    }

    #[test]
    fn active_false() {
        assert_eq!(static_setter("active", "false"), Some(".active(false)".to_string()));
    }

    #[test]
    fn default_open() {
        assert_eq!(static_setter("default_open", ""), Some(".default_open(true)".to_string()));
    }

    #[test]
    fn click_to_open() {
        assert_eq!(static_setter("click_to_open", "true"), Some(".click_to_open(true)".to_string()));
    }

    #[test]
    fn click_to_toggle() {
        assert_eq!(static_setter("click_to_toggle", ""), Some(".click_to_toggle(true)".to_string()));
    }

    #[test]
    fn disabled_maps_to_disable() {
        assert_eq!(static_setter("disabled", ""), Some(".disable(true)".to_string()));
        assert_eq!(static_setter("disabled", "false"), Some(".disable(false)".to_string()));
    }

    #[test]
    fn unknown_attr_returns_none() {
        assert!(static_setter("unknown", "x").is_none());
    }

    #[test]
    fn bind_active() {
        let s = bind_setter("active", "is_active", &[], &[]).unwrap();
        assert_eq!(s, ".active(self.is_active)");
    }

    #[test]
    fn bind_disabled() {
        let s = bind_setter("disabled", "is_disabled", &[], &[]).unwrap();
        assert_eq!(s, ".disable(self.is_disabled)");
    }

    #[test]
    fn bind_unknown_returns_none() {
        assert!(bind_setter("unknown", "x", &[], &[]).is_none());
    }
}
