//! Card 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter`
//! 在 tag 为 "Card" 时委托调用。未命中返回 None，由公共 setter 回退到通用属性
//! （Sizable、disabled 等）。
//!
//! ## 静态属性
//!
//! - `title="..."` → `.title("...")`（SharedString）
//! - `extra="..."` / `cover="..."` / `footer="..."` → `.method("...")`（SharedString）
//! - `bordered="true"` / `bordered=""` → `.bordered(true)`
//! - `bordered="false"` → `.bordered(false)`
//! - `borderless=""` / `borderless="true"` → `.borderless()`（标志，等价于 `bordered="false"`）
//! - `hoverable=""` / `hoverable="true"` → `.hoverable(true)`
//! - `hoverable="false"` → `.hoverable(false)`
//!
//! ## 绑定属性
//!
//! - `title={expr}` → `.title(expr)`（IntoElement，不 clone）
//! - `extra={expr}` / `cover={expr}` / `footer={expr}` → `.method(expr)`（IntoElement，不 clone）
//! - `bordered={expr}` → `.bordered(expr)`（bool 表达式）
//! - `hoverable={expr}` → `.hoverable(expr)`（bool 表达式）
//! - `borderless={expr}` → 不支持（用户应使用 `bordered={!expr}`）

/// 静态属性 → builder 方法
///
/// 仅在 `tag == "Card"` 时匹配，避免误匹配其他组件的同名属性。
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    if tag != "Card" {
        return None;
    }
    match name {
        "title" | "extra" | "cover" | "footer" => {
            // 这四个属性都接受 impl IntoElement，静态字符串作为 SharedString 自动转换。
            Some(format!(".{}({:?})", name, value))
        }
        "bordered" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".bordered({})", bool_val))
        }
        "borderless" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".borderless()".to_string())
            } else {
                None
            }
        }
        "hoverable" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".hoverable({})", bool_val))
        }
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// 仅在 `tag == "Card"` 时匹配。
///
/// - `title`/`extra`/`cover`/`footer` → `.method(expr)`（接受 `impl IntoElement`，不 clone）
/// - `bordered`/`hoverable` → `.method(expr)`（接受 bool 表达式）
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    if tag != "Card" {
        return None;
    }
    match name {
        "title" => {
            let rust_expr =
                super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".title({})", rust_expr))
        }
        "extra" | "cover" | "footer" => {
            let rust_expr =
                super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        "bordered" | "hoverable" => {
            let rust_expr =
                super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── static_setter: title ───

    #[test]
    fn static_setter_title() {
        let code = static_setter("title", "Card Title", "Card").unwrap();
        assert_eq!(code, r#".title("Card Title")"#);
    }

    #[test]
    fn static_setter_title_with_special_chars() {
        let code = static_setter("title", "Hello \"World\"", "Card").unwrap();
        assert_eq!(code, r#".title("Hello \"World\"")"#);
    }

    // ─── static_setter: extra / cover / footer ───

    #[test]
    fn static_setter_extra() {
        // extra="More" → .extra("More")（SharedString 自动转 IntoElement）
        let code = static_setter("extra", "More", "Card").unwrap();
        assert_eq!(code, r#".extra("More")"#);
    }

    #[test]
    fn static_setter_cover() {
        let code = static_setter("cover", "Cover Text", "Card").unwrap();
        assert_eq!(code, r#".cover("Cover Text")"#);
    }

    #[test]
    fn static_setter_footer() {
        let code = static_setter("footer", "Footer Text", "Card").unwrap();
        assert_eq!(code, r#".footer("Footer Text")"#);
    }

    #[test]
    fn static_setter_extra_with_special_chars() {
        let code = static_setter("extra", "Hello \"World\"", "Card").unwrap();
        assert_eq!(code, r#".extra("Hello \"World\"")"#);
    }

    #[test]
    fn static_setter_extra_other_tag_returns_none() {
        // 非 Card 组件不应匹配 extra 属性
        assert!(static_setter("extra", "x", "Button").is_none());
        assert!(static_setter("cover", "x", "Avatar").is_none());
        assert!(static_setter("footer", "x", "Div").is_none());
    }

    // ─── static_setter: bordered ───

    #[test]
    fn static_setter_bordered_empty() {
        let code = static_setter("bordered", "", "Card").unwrap();
        assert_eq!(code, ".bordered(true)");
    }

    #[test]
    fn static_setter_bordered_true() {
        let code = static_setter("bordered", "true", "Card").unwrap();
        assert_eq!(code, ".bordered(true)");
    }

    #[test]
    fn static_setter_bordered_false() {
        let code = static_setter("bordered", "false", "Card").unwrap();
        assert_eq!(code, ".bordered(false)");
    }

    // ─── static_setter: borderless ───

    #[test]
    fn static_setter_borderless_empty() {
        let code = static_setter("borderless", "", "Card").unwrap();
        assert_eq!(code, ".borderless()");
    }

    #[test]
    fn static_setter_borderless_true() {
        let code = static_setter("borderless", "true", "Card").unwrap();
        assert_eq!(code, ".borderless()");
    }

    #[test]
    fn static_setter_borderless_false_returns_none() {
        assert!(static_setter("borderless", "false", "Card").is_none());
    }

    // ─── static_setter: hoverable ───

    #[test]
    fn static_setter_hoverable_empty() {
        let code = static_setter("hoverable", "", "Card").unwrap();
        assert_eq!(code, ".hoverable(true)");
    }

    #[test]
    fn static_setter_hoverable_true() {
        let code = static_setter("hoverable", "true", "Card").unwrap();
        assert_eq!(code, ".hoverable(true)");
    }

    #[test]
    fn static_setter_hoverable_false() {
        let code = static_setter("hoverable", "false", "Card").unwrap();
        assert_eq!(code, ".hoverable(false)");
    }

    // ─── static_setter: unknown ───

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "Card").is_none());
        assert!(static_setter("on_click", "", "Card").is_none());
        assert!(static_setter("primary", "", "Card").is_none());
    }

    #[test]
    fn static_setter_other_tag_returns_none() {
        assert!(static_setter("title", "x", "Button").is_none());
        assert!(static_setter("bordered", "true", "Avatar").is_none());
    }

    // ─── bind_setter: title ───

    #[test]
    fn bind_setter_title() {
        let code = bind_setter("title", "card_title", &[], &[], "Card").unwrap();
        assert_eq!(code, ".title(self.card_title)");
    }

    #[test]
    fn bind_setter_title_nested() {
        let code = bind_setter("title", "user.name", &[], &[], "Card").unwrap();
        assert_eq!(code, ".title(self.user.name)");
    }

    // ─── bind_setter: extra / cover / footer ───

    #[test]
    fn bind_setter_extra() {
        let code = bind_setter("extra", "action_button", &[], &[], "Card").unwrap();
        assert_eq!(code, ".extra(self.action_button)");
    }

    #[test]
    fn bind_setter_cover() {
        let code = bind_setter("cover", "cover_img", &[], &[], "Card").unwrap();
        assert_eq!(code, ".cover(self.cover_img)");
    }

    #[test]
    fn bind_setter_footer() {
        let code = bind_setter("footer", "footer_element", &[], &[], "Card").unwrap();
        assert_eq!(code, ".footer(self.footer_element)");
    }

    // ─── bind_setter: bordered / hoverable ───

    #[test]
    fn bind_setter_bordered() {
        let code = bind_setter("bordered", "show_border", &[], &[], "Card").unwrap();
        assert_eq!(code, ".bordered(self.show_border)");
    }

    #[test]
    fn bind_setter_hoverable() {
        let code = bind_setter("hoverable", "is_hoverable", &[], &[], "Card").unwrap();
        assert_eq!(code, ".hoverable(self.is_hoverable)");
    }

    // ─── bind_setter: borderless (unsupported) ───

    #[test]
    fn bind_setter_borderless_unsupported() {
        assert!(bind_setter("borderless", "x", &[], &[], "Card").is_none());
    }

    // ─── bind_setter: unknown ───

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("label", "x", &[], &[], "Card").is_none());
        assert!(bind_setter("value", "x", &[], &[], "Card").is_none());
    }

    #[test]
    fn bind_setter_other_tag_returns_none() {
        assert!(bind_setter("title", "x", &[], &[], "Button").is_none());
        assert!(bind_setter("bordered", "x", &[], &[], "Avatar").is_none());
    }

    // ─── bind_setter: with loop_vars ───

    #[test]
    fn bind_setter_title_with_loop_var() {
        let code = bind_setter("title", "item.name", &["item"], &[], "Card").unwrap();
        assert_eq!(code, ".title(item.name)");
    }
}
