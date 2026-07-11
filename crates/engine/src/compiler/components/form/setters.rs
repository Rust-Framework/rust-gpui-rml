//! Form 专用属性 setter
//!
//! ## 属性映射
//!
//! - `horizontal` → `Form::horizontal()` 构造器选择（独立布尔属性）
//! - `vertical` → `Form::vertical()` 构造器选择（独立布尔属性，默认）
//! - `label_width="200"` → `.label_width(gpui::px(200.))`
//! - `label_text_size="0.875"` → `.label_text_size(gpui::rems(0.875))`
//! - `columns="2"` → `.columns(2)`

/// Form variant 布尔属性名 → 构造器方法名
///
/// `horizontal` → `horizontal`，`vertical` → `vertical`
pub fn form_variant_from_attr(name: &str) -> Option<&'static str> {
    match name {
        "horizontal" => Some("horizontal"),
        "vertical" => Some("vertical"),
        _ => None,
    }
}

/// Form 专用静态属性 setter（不含 variant 构造器选择）
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "label_width" => {
            let px = parse_px(value)?;
            Some(format!(".label_width(gpui::px({}.))", px))
        }
        "label_text_size" => {
            let rems = value.parse::<f64>().ok()?;
            Some(format!(".label_text_size(gpui::rems({}))", rems))
        }
        "columns" => {
            let n = value.parse::<usize>().ok()?;
            Some(format!(".columns({})", n))
        }
        _ => None,
    }
}

/// 解析像素值（支持 "200" 和 "200px"）
fn parse_px(value: &str) -> Option<f64> {
    let v = value.trim_end_matches("px");
    v.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_label_width() {
        assert_eq!(
            static_setter("label_width", "200"),
            Some(".label_width(gpui::px(200.))".to_string())
        );
    }

    #[test]
    fn static_setter_label_width_with_px() {
        assert_eq!(
            static_setter("label_width", "140px"),
            Some(".label_width(gpui::px(140.))".to_string())
        );
    }

    #[test]
    fn static_setter_columns() {
        assert_eq!(static_setter("columns", "2"), Some(".columns(2)".to_string()));
    }

    #[test]
    fn static_setter_label_text_size() {
        assert_eq!(
            static_setter("label_text_size", "0.875"),
            Some(".label_text_size(gpui::rems(0.875))".to_string())
        );
    }

    #[test]
    fn static_setter_invalid_number() {
        assert_eq!(static_setter("columns", "abc"), None);
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn variant_from_attr_horizontal() {
        assert_eq!(form_variant_from_attr("horizontal"), Some("horizontal"));
    }

    #[test]
    fn variant_from_attr_vertical() {
        assert_eq!(form_variant_from_attr("vertical"), Some("vertical"));
    }

    #[test]
    fn variant_from_attr_unknown() {
        assert_eq!(form_variant_from_attr("foo"), None);
    }
}
