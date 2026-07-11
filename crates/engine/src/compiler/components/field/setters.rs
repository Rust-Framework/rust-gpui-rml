//! Field 专用属性 setter
//!
//! ## 属性映射
//!
//! - `label="用户名"` → `.label("用户名")`
//! - `description="帮助文本"` → `.description("帮助文本")`
//! - `required` → `.required(true)`（parse_bool_enabled，默认 false，显式 true 生成方法调用）
//! - `visible="false"` → `.visible(false)`（parse_bool_disabled，默认 true，显式 false 关闭）
//! - `col_span="2"` → `.col_span(2)`
//! - `col_start="1"` → `.col_start(1)`
//! - `col_end="3"` → `.col_end(3)`
//! - `label_indent="false"` → `.label_indent(false)`（parse_bool_disabled，默认 true）

/// Field 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "label" => Some(format!(".label({:?})", value)),
        "description" => Some(format!(".description({:?})", value)),
        // required：默认 false，显式 true → .required(true)
        "required" => parse_bool_enabled(name, value),
        // visible：默认 true，显式 false → .visible(false)
        "visible" => parse_bool_disabled(name, value),
        // label_indent：默认 true，显式 false → .label_indent(false)
        "label_indent" => parse_bool_disabled(name, value),
        "col_span" => {
            let n = value.parse::<u16>().ok()?;
            Some(format!(".col_span({})", n))
        }
        "col_start" => {
            let n = value.parse::<i16>().ok()?;
            Some(format!(".col_start({})", n))
        }
        "col_end" => {
            let n = value.parse::<i16>().ok()?;
            Some(format!(".col_end({})", n))
        }
        _ => None,
    }
}

/// 布尔属性：默认 false，显式 "true" → .method(true)
fn parse_bool_enabled(name: &str, value: &str) -> Option<String> {
    if value.is_empty() || value.eq_ignore_ascii_case("true") {
        Some(format!(".{}(true)", name))
    } else {
        Some(String::new())
    }
}

/// 布尔属性：默认 true，显式 "false" → .method(false)
fn parse_bool_disabled(name: &str, value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("false") {
        Some(format!(".{}(false)", name))
    } else {
        Some(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_label() {
        assert_eq!(
            static_setter("label", "用户名"),
            Some(".label(\"用户名\")".to_string())
        );
    }

    #[test]
    fn static_setter_description() {
        assert_eq!(
            static_setter("description", "请输入用户名"),
            Some(".description(\"请输入用户名\")".to_string())
        );
    }

    #[test]
    fn static_setter_required_empty() {
        assert_eq!(
            static_setter("required", ""),
            Some(".required(true)".to_string())
        );
    }

    #[test]
    fn static_setter_required_true() {
        assert_eq!(
            static_setter("required", "true"),
            Some(".required(true)".to_string())
        );
    }

    #[test]
    fn static_setter_required_false_no_op() {
        let s = static_setter("required", "false").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_visible_false() {
        assert_eq!(
            static_setter("visible", "false"),
            Some(".visible(false)".to_string())
        );
    }

    #[test]
    fn static_setter_visible_true_no_op() {
        let s = static_setter("visible", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_col_span() {
        assert_eq!(static_setter("col_span", "2"), Some(".col_span(2)".to_string()));
    }

    #[test]
    fn static_setter_col_start() {
        assert_eq!(
            static_setter("col_start", "1"),
            Some(".col_start(1)".to_string())
        );
    }

    #[test]
    fn static_setter_col_end() {
        assert_eq!(static_setter("col_end", "3"), Some(".col_end(3)".to_string()));
    }

    #[test]
    fn static_setter_label_indent_false() {
        assert_eq!(
            static_setter("label_indent", "false"),
            Some(".label_indent(false)".to_string())
        );
    }

    #[test]
    fn static_setter_invalid_number() {
        assert_eq!(static_setter("col_span", "abc"), None);
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }
}
