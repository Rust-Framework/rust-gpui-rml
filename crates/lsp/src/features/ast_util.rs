//! AST 工具：定位光标、推算 span、提取事件处理器名
//!
//! 所有 features 模块共用的 AST 遍历辅助函数集中于此，避免跨模块重复实现。

use rust_rml_engine::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use rust_rml_engine::parser::Span;

/// 递归查找包含字节偏移的元素
///
/// 先在子节点中查找（更精确的定位），未命中则返回当前元素。
/// 与 hover.rs 原实现一致，抽出为 pub 供 definition/references/rename 复用。
pub fn find_element_at_offset(node: &Node, offset: usize) -> Option<&Element> {
    match node {
        Node::Element(elem) => {
            if !elem.span.contains(offset) {
                return None;
            }
            for child in &elem.children {
                if let Some(found) = find_element_at_offset(child, offset) {
                    return Some(found);
                }
            }
            Some(elem)
        }
        _ => None,
    }
}

/// 推算标签名字节区间
///
/// `<div ...>` 的标签名区间为 `elem.span.start + 1 .. elem.span.start + 1 + tag.len()`。
/// 闭标签 `</div>` 不在此推算范围（references 单独处理闭标签位置）。
///
/// 近似推算：假设 `<` 紧跟标签名（无空格）。若 `<` 后有空格，区间会偏移，
/// 但 definition/references 的命中检查仍可工作（光标在标签名上时 offset 落在推算区间内）。
pub fn tag_name_span(elem: &Element) -> Span {
    let start = elem.span.start + 1;
    let end = start + elem.tag.len();
    Span::new(start, end)
}

/// 在元素属性中定位 offset 命中的属性
pub fn find_attribute_at_offset<'a>(
    elem: &'a Element,
    offset: usize,
) -> Option<&'a Attribute> {
    elem.attributes
        .iter()
        .find(|attr| attr_span(attr).contains(offset))
}

/// 取属性的 span（三种变体统一）
fn attr_span(attr: &Attribute) -> Span {
    match attr {
        Attribute::Static { span, .. } => *span,
        Attribute::Bind { span, .. } => *span,
        Attribute::Event { span, .. } => *span,
    }
}

/// 遍历元素所有指令携带的表达式（If/Show/Key/Html/Model/Each.iterable）
///
/// 指令本身无独立 span 字段，无法精确定位光标在哪个指令上。
/// symbol 分类时遍历所有指令表达式，靠表达式内容匹配符号名。
pub fn iter_directive_exprs(elem: &Element) -> impl Iterator<Item = &str> {
    elem.directives.iter().filter_map(directive_expr)
}

/// 取指令携带的表达式（若有）
pub fn directive_expr(d: &Directive) -> Option<&str> {
    match d {
        Directive::If { expr, .. } | Directive::Show { expr, .. } | Directive::Key { expr, .. } | Directive::Html { expr, .. } => {
            Some(expr)
        }
        Directive::Model { field, .. } => Some(field),
        Directive::Each { clause: each, .. } => Some(&each.iterable),
        _ => None,
    }
}

/// 提取事件处理器名（Ident/MethodName/WithArgs 三态统一）
pub fn event_handler_name(h: &EventHandler) -> &str {
    match h {
        EventHandler::Ident(name) | EventHandler::MethodName(name) | EventHandler::WithArgs(name, _) => {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rml_engine::parser::ast::EachClause;

    #[test]
    fn tag_name_span_basic() {
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 10),
            ..Default::default()
        };
        let s = tag_name_span(&elem);
        assert_eq!(s.start, 1);
        assert_eq!(s.end, 4);
        assert!(s.contains(1));
        assert!(s.contains(3));
        assert!(!s.contains(4));
    }

    #[test]
    fn event_handler_name_variants() {
        assert_eq!(event_handler_name(&EventHandler::Ident("on_click".into())), "on_click");
        assert_eq!(event_handler_name(&EventHandler::MethodName("on_click".into())), "on_click");
        assert_eq!(
            event_handler_name(&EventHandler::WithArgs("on_click".into(), vec![])),
            "on_click"
        );
    }

    #[test]
    fn directive_expr_extracts() {
        use rust_rml_engine::parser::Span;
        assert_eq!(directive_expr(&Directive::If { expr: "count".into(), span: Span::empty() }), Some("count"));
        assert_eq!(directive_expr(&Directive::Show { expr: "visible".into(), span: Span::empty() }), Some("visible"));
        assert_eq!(directive_expr(&Directive::Key { expr: "id".into(), span: Span::empty() }), Some("id"));
        assert_eq!(directive_expr(&Directive::Html { expr: "raw".into(), span: Span::empty() }), Some("raw"));
        assert_eq!(
            directive_expr(&Directive::Model { field: "name".into(), converter: None, span: Span::empty() }),
            Some("name")
        );
        assert_eq!(
            directive_expr(&Directive::Each {
                clause: EachClause {
                    item: "x".into(),
                    index: None,
                    iterable: "items".into(),
                },
                span: Span::empty(),
            }),
            Some("items")
        );
        assert_eq!(directive_expr(&Directive::Else { span: Span::empty() }), None);
        assert_eq!(directive_expr(&Directive::Once { span: Span::empty() }), None);
        assert_eq!(directive_expr(&Directive::Ref { name: "input".into(), span: Span::empty() }), None);
    }
}
