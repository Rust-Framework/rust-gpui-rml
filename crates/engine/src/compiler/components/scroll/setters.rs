//! Scroll 专用属性 setter
//!
//! ## 属性映射
//!
//! - `vertical` → `.vertical()`（独立布尔属性，默认方向）
//! - `horizontal` → `.horizontal()`（独立布尔属性）
//! - `both` → `.both()`（独立布尔属性，双向滚动）
//!
//! 三个 variant 为互斥方向选择，不写时默认 vertical。

/// Scroll variant 布尔属性名 → builder 方法名
///
/// `vertical` → `vertical`，`horizontal` → `horizontal`，`both` → `both`
pub fn scroll_variant_from_attr(name: &str) -> Option<&'static str> {
    match name {
        "vertical" => Some("vertical"),
        "horizontal" => Some("horizontal"),
        "both" => Some("both"),
        _ => None,
    }
}

/// Scroll 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    // variant 布尔属性：vertical/horizontal/both → .method()
    if let Some(method) = scroll_variant_from_attr(name) {
        if value.is_empty() || value.eq_ignore_ascii_case("true") {
            return Some(format!(".{}()", method));
        }
        return Some(String::new());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_vertical() {
        assert_eq!(
            static_setter("vertical", ""),
            Some(".vertical()".to_string())
        );
        assert_eq!(
            static_setter("vertical", "true"),
            Some(".vertical()".to_string())
        );
    }

    #[test]
    fn static_setter_horizontal() {
        assert_eq!(
            static_setter("horizontal", ""),
            Some(".horizontal()".to_string())
        );
    }

    #[test]
    fn static_setter_both() {
        assert_eq!(static_setter("both", ""), Some(".both()".to_string()));
    }

    #[test]
    fn static_setter_variant_false_no_op() {
        let s = static_setter("horizontal", "false").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn variant_from_attr_vertical() {
        assert_eq!(scroll_variant_from_attr("vertical"), Some("vertical"));
    }

    #[test]
    fn variant_from_attr_unknown() {
        assert_eq!(scroll_variant_from_attr("foo"), None);
    }
}
