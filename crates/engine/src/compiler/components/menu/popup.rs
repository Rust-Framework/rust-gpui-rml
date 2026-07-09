//! PopupMenu 容器配置属性 codegen

use crate::compiler::CodegenError;
use crate::parser::ast::{Attribute, Element};

/// 从容器元素属性生成 PopupMenu 配置语句
pub fn apply_popup_config(elem: &Element) -> Result<String, CodegenError> {
    let mut lines = Vec::new();
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => match name.as_str() {
                "scrollable" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
                    lines.push("menu = menu.scrollable(true);".to_string());
                }
                "external_link_icon" => {
                    let v = value.is_empty() || value.eq_ignore_ascii_case("true");
                    lines.push(format!("menu = menu.external_link_icon({v});"));
                }
                "min_w" | "max_w" | "max_h" => {
                    let method = name.as_str();
                    if let Ok(px) = value.parse::<f32>() {
                        lines.push(format!(
                            "menu = menu.{method}(gpui::px({px}.));"
                        ));
                    }
                }
                "check_side" => {
                    let side = match value.as_str() {
                        "Right" => "Side::Right",
                        _ => "Side::Left",
                    };
                    lines.push(format!("menu = menu.check_side({side});"));
                }
                "anchor" => {
                    // anchor on DropdownMenu handled in dropdown.rs
                }
                _ => {}
            },
            Attribute::Bind { name, expr, .. } if name == "max_h" || name == "min_w" || name == "max_w" => {
                lines.push(format!("menu = menu.{name}(gpui::px(self.{expr} as f32));"));
            }
            _ => {}
        }
    }
    if lines.is_empty() {
        return Ok(String::new());
    }
    Ok(lines.join("\n                "))
}

pub fn anchor_from_elem(elem: &Element) -> String {
    static_attr(elem, "anchor")
        .map(|a| format!("gpui::Anchor::{a}"))
        .unwrap_or_else(|| "gpui::Anchor::TopLeft".to_string())
}

fn static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|a| match a {
        Attribute::Static { name: n, value, .. } if n == name => Some(value.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::parser::ast::Node;

    #[test]
    fn scrollable_config() {
        let src = r#"<DropdownMenu scrollable="" max-h="300" />"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else { panic!() };
        let code = apply_popup_config(&elem).unwrap();
        assert!(code.contains("scrollable(true)"));
        assert!(code.contains("max_h"));
        assert!(code.contains("300"));
    }
}
