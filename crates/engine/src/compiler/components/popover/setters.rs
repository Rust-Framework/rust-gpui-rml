//! Popover 专用属性 setter
//!
//! ## 属性映射
//!
//! - `anchor="top-left"` → `.anchor(rml_ui::Anchor::TopLeft)`
//! - `mouse_button="left"` → `.mouse_button(gpui::MouseButton::Left)`
//! - `appearance="false"` → `.appearance(false)`
//! - `overlay_closable="false"` → `.overlay_closable(false)`
//! - `default_open="true"` → `.default_open(true)`

/// Popover 专用静态属性 setter
///
/// - `anchor="top-left"` → `.anchor(rml_ui::Anchor::TopLeft)`
/// - `mouse_button="left"` → `.mouse_button(gpui::MouseButton::Left)`
/// - `appearance="false"` → `.appearance(false)`
/// - `overlay_closable="false"` → `.overlay_closable(false)`
/// - `default_open="true"` → `.default_open(true)`
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "anchor" => {
            let anchor = match value {
                "top-left" => "TopLeft",
                "top-center" => "TopCenter",
                "top-right" => "TopRight",
                "bottom-left" => "BottomLeft",
                "bottom-center" => "BottomCenter",
                "bottom-right" => "BottomRight",
                "left-center" => "LeftCenter",
                "right-center" => "RightCenter",
                _ => return None,
            };
            Some(format!(".anchor(gpui::Anchor::{})", anchor))
        }
        "mouse_button" => {
            let btn = match value {
                "left" => "Left",
                "right" => "Right",
                "middle" => "Middle",
                _ => return None,
            };
            Some(format!(".mouse_button(gpui::MouseButton::{})", btn))
        }
        "appearance" => {
            // appearance 默认 true，仅在 false 时显式设置
            if value.eq_ignore_ascii_case("false") {
                Some(".appearance(false)".into())
            } else {
                Some(String::new())
            }
        }
        "overlay_closable" => {
            // overlay_closable 默认 true，仅在 false 时显式设置
            if value.eq_ignore_ascii_case("false") {
                Some(".overlay_closable(false)".into())
            } else {
                Some(String::new())
            }
        }
        "default_open" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".default_open(true)".into())
            } else {
                Some(String::new())
            }
        }
        _ => None,
    }
}

/// Popover 专用绑定属性 setter
///
/// 当前仅支持 `default_open` 绑定（非受控初始状态）。
/// 受控模式（`open` + `on_open_change`）需要特殊的回调签名适配，
/// 待真正需求出现时再添加。
pub fn bind_setter(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str]) -> Option<String> {
    match name {
        "default_open" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, loop_vars, computed);
            Some(format!(".default_open({})", rust_expr))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_anchor() {
        assert_eq!(
            static_setter("anchor", "top-left"),
            Some(".anchor(gpui::Anchor::TopLeft)".to_string())
        );
        assert_eq!(
            static_setter("anchor", "bottom-center"),
            Some(".anchor(gpui::Anchor::BottomCenter)".to_string())
        );
    }

    #[test]
    fn static_setter_mouse_button() {
        assert_eq!(
            static_setter("mouse_button", "left"),
            Some(".mouse_button(gpui::MouseButton::Left)".to_string())
        );
        assert_eq!(
            static_setter("mouse_button", "right"),
            Some(".mouse_button(gpui::MouseButton::Right)".to_string())
        );
    }

    #[test]
    fn static_setter_appearance_false() {
        assert_eq!(static_setter("appearance", "false"), Some(".appearance(false)".into()));
    }

    #[test]
    fn static_setter_appearance_true_no_op() {
        // appearance=true 是默认值，不生成代码
        let s = static_setter("appearance", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_overlay_closable_false() {
        assert_eq!(
            static_setter("overlay_closable", "false"),
            Some(".overlay_closable(false)".into())
        );
    }

    #[test]
    fn static_setter_default_open() {
        assert_eq!(static_setter("default_open", "true"), Some(".default_open(true)".into()));
        assert_eq!(static_setter("default_open", ""), Some(".default_open(true)".into()));
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }
}
