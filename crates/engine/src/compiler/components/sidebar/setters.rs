//! Sidebar 专用属性 setter
//!
//! ## 属性映射
//!
//! - `side` → `.side(rml_ui::Side::Left/Right)`（静态属性，left/right）
//! - `collapsible` → `.collapsible(rml_ui::SidebarCollapsible::Icon/Offcanvas/None)`
//!   （支持 icon/offcanvas/none 字符串值，也支持 true/false 布尔简写）
//! - `collapsed` → `.collapsed(bool)`（parse_bool_enabled：存在= true，"false" = false）

/// `side` 属性值 → `Side` 枚举变体
pub fn side_from_value(value: &str) -> Option<&'static str> {
    match value {
        "left" => Some("Left"),
        "right" => Some("Right"),
        _ => None,
    }
}

/// `collapsible` 属性值 → `SidebarCollapsible` 枚举变体
pub fn collapsible_from_value(value: &str) -> Option<&'static str> {
    match value {
        "icon" | "true" | "" => Some("Icon"),
        "offcanvas" => Some("Offcanvas"),
        "none" | "false" => Some("None"),
        _ => None,
    }
}

/// Sidebar 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "side" => {
            if let Some(variant) = side_from_value(value) {
                return Some(format!(".side(rml_ui::Side::{})", variant));
            }
            return Some(String::new());
        }
        "collapsible" => {
            if let Some(variant) = collapsible_from_value(value) {
                return Some(format!(".collapsible(rml_ui::SidebarCollapsible::{})", variant));
            }
            return Some(String::new());
        }
        "collapsed" => {
            // parse_bool_enabled: 存在(空值)或 "true" → .collapsed(true)，"false" → .collapsed(false)
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                return Some(".collapsed(true)".to_string());
            }
            if value.eq_ignore_ascii_case("false") {
                return Some(".collapsed(false)".to_string());
            }
            return Some(String::new());
        }
        _ => None,
    }
}

/// Sidebar 专用 bind 属性 setter
pub fn bind_setter(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str]) -> Option<String> {
    match name {
        "collapsed" => {
            let rust_expr =
                crate::compiler::setters::component_bind_rust_expr(expr, loop_vars, computed);
            Some(format!(".collapsed({})", rust_expr))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_left() {
        assert_eq!(
            static_setter("side", "left"),
            Some(".side(rml_ui::Side::Left)".to_string())
        );
    }

    #[test]
    fn side_right() {
        assert_eq!(
            static_setter("side", "right"),
            Some(".side(rml_ui::Side::Right)".to_string())
        );
    }

    #[test]
    fn side_invalid_is_empty() {
        let s = static_setter("side", "top").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn collapsible_icon() {
        assert_eq!(
            static_setter("collapsible", "icon"),
            Some(".collapsible(rml_ui::SidebarCollapsible::Icon)".to_string())
        );
    }

    #[test]
    fn collapsible_offcanvas() {
        assert_eq!(
            static_setter("collapsible", "offcanvas"),
            Some(".collapsible(rml_ui::SidebarCollapsible::Offcanvas)".to_string())
        );
    }

    #[test]
    fn collapsible_none() {
        assert_eq!(
            static_setter("collapsible", "none"),
            Some(".collapsible(rml_ui::SidebarCollapsible::None)".to_string())
        );
    }

    #[test]
    fn collapsible_bool_true() {
        assert_eq!(
            static_setter("collapsible", "true"),
            Some(".collapsible(rml_ui::SidebarCollapsible::Icon)".to_string())
        );
    }

    #[test]
    fn collapsible_bool_false() {
        assert_eq!(
            static_setter("collapsible", "false"),
            Some(".collapsible(rml_ui::SidebarCollapsible::None)".to_string())
        );
    }

    #[test]
    fn collapsible_presence() {
        assert_eq!(
            static_setter("collapsible", ""),
            Some(".collapsible(rml_ui::SidebarCollapsible::Icon)".to_string())
        );
    }

    #[test]
    fn collapsed_presence() {
        assert_eq!(
            static_setter("collapsed", ""),
            Some(".collapsed(true)".to_string())
        );
    }

    #[test]
    fn collapsed_true() {
        assert_eq!(
            static_setter("collapsed", "true"),
            Some(".collapsed(true)".to_string())
        );
    }

    #[test]
    fn collapsed_false() {
        assert_eq!(
            static_setter("collapsed", "false"),
            Some(".collapsed(false)".to_string())
        );
    }

    #[test]
    fn collapsed_bind() {
        let s = bind_setter("collapsed", "is_collapsed", &[], &[]).unwrap();
        assert_eq!(s, ".collapsed(self.is_collapsed)");
    }

    #[test]
    fn unknown_attr_returns_none() {
        assert!(static_setter("unknown", "x").is_none());
    }
}
