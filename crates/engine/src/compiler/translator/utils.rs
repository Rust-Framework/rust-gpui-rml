//! Translator 公共辅助函数
//!
//! 供各 translator 在 to_rust / to_rml 过程中复用。

use crate::parser::ast::{Attribute, Directive, Element};

/// 判断元素是否声明了指定静态属性
pub fn has_static_attr(elem: &Element, name: &str) -> bool {
    elem.attributes.iter().any(|attr| match attr {
        Attribute::Static { name: n, .. } => n == name,
        _ => false,
    })
}

/// 读取静态属性值
pub fn static_attr_value<'a>(elem: &'a Element, name: &'a str) -> Option<&'a str> {
    elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name: n, value, .. } if n == name => Some(value.as_str()),
        _ => None,
    })
}

/// 读取 ref 指令指定的名称
pub fn ref_name(elem: &Element) -> Option<&str> {
    elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    })
}

/// 读取 key 指令表达式
pub fn key_expr(elem: &Element) -> Option<&str> {
    elem.directives.iter().find_map(|d| match d {
        Directive::Key { expr, .. } => Some(expr.as_str()),
        _ => None,
    })
}

/// 判断元素是否有事件属性
pub fn has_event_attr(elem: &Element) -> bool {
    elem.attributes.iter().any(|attr| matches!(attr, Attribute::Event { .. }))
}

/// 解析 RML 布尔属性值
pub fn parse_bool(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        "true"
    } else {
        "false"
    }
}

/// 生成 ElementId：ref 优先，其次 key，最后自增计数器
pub fn element_id(elem: &Element, id_counter: &mut usize) -> String {
    if let Some(name) = ref_name(elem) {
        return format!("{:?}", format!("rml_ref:{}", name));
    }
    if let Some(key) = key_expr(elem) {
        return format!(
            "(\"rml_key\", rml_core::element_id::from_key(&{}))",
            key
        );
    }
    let id = *id_counter;
    *id_counter += 1;
    format!("(\"rml_el\", {}usize)", id)
}

/// 判断元素是否为空（无子节点且无文本内容）
pub fn is_empty_element(elem: &Element) -> bool {
    elem.children.is_empty()
}

/// 转义 RML 属性值中的引号
pub fn escape_attr_value(value: &str) -> String {
    value.replace('"', "&quot;")
}
