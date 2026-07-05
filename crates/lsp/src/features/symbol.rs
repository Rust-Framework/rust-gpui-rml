//! 符号分类：识别光标位置命中的符号种类（Tag/Field/Command）
//!
//! references/rename 共用本模块的分类逻辑定位光标处的目标符号。

use rust_rml_engine::parser::ast::{Attribute, Node};

use crate::crosslang::resolver::parse_binding_path;
use crate::features::ast_util::{
    event_handler_name, find_attribute_at_offset, find_element_at_offset, iter_directive_exprs,
    tag_name_span,
};

/// 光标处识别出的符号
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Symbol {
    /// 标签名（如 `<MyComponent>` 中的 `MyComponent`）
    Tag(String),
    /// ViewModel 字段或 computed 方法（绑定表达式的根标识符）
    Field(String),
    /// #[command] 方法（事件处理器名）
    Command(String),
}

/// 在 .rml AST 中识别光标处的符号
///
/// 返回 None 表示光标未命中任何可识别符号（空白、静态属性值等）。
pub fn classify_symbol_at(root: &Node, _source: &str, offset: usize) -> Option<Symbol> {
    let elem = find_element_at_offset(root, offset)?;

    // 1. 标签名位置
    let tag_span = tag_name_span(elem);
    if tag_span.contains(offset) {
        return Some(Symbol::Tag(elem.tag.clone()));
    }

    // 2. 属性位置
    if let Some(attr) = find_attribute_at_offset(elem, offset) {
        match attr {
            Attribute::Bind { expr, .. } => {
                let path = parse_binding_path(expr)?;
                if is_builtin(&path.root) {
                    return None;
                }
                return Some(Symbol::Field(path.root));
            }
            Attribute::Event { handler, .. } => {
                return Some(Symbol::Command(event_handler_name(handler).to_string()));
            }
            Attribute::Static { .. } => {}
        }
    }

    // 3. 指令表达式（If/Show/Key/Html/Model/Each.iterable）
    // 指令无独立 span，仅当光标在元素 span 内时按表达式内容匹配。
    // 这里返回第一个非 builtin 根标识符作为候选（与 definition.rs 一致的简化策略）。
    for expr in iter_directive_exprs(elem) {
        if let Some(path) = parse_binding_path(expr) {
            if !is_builtin(&path.root) {
                return Some(Symbol::Field(path.root));
            }
        }
    }

    // 4. 文本插值（递归子节点中的 Interpolation）
    if let Some(symbol) = classify_in_interpolation(root, offset) {
        return Some(symbol);
    }

    None
}

/// 在 Node 树中查找光标位置命中的文本插值，返回其根标识符符号
fn classify_in_interpolation(node: &Node, offset: usize) -> Option<Symbol> {
    match node {
        Node::Element(elem) => {
            if !elem.span.contains(offset) {
                return None;
            }
            for child in &elem.children {
                if let Some(s) = classify_in_interpolation(child, offset) {
                    return Some(s);
                }
            }
            None
        }
        Node::Interpolation(expr) => {
            let path = parse_binding_path(expr)?;
            if is_builtin(&path.root) {
                return None;
            }
            Some(Symbol::Field(path.root))
        }
        Node::MixedText(segs) => {
            // MixedText 无独立 span，遍历所有插值段，取第一个非 builtin 根标识符
            for seg in segs {
                if let rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) = seg {
                    if let Some(path) = parse_binding_path(expr) {
                        if !is_builtin(&path.root) {
                            return Some(Symbol::Field(path.root));
                        }
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// 内置标识符白名单（与 crosslang::coordinator::is_builtin_ident 保持一致）
fn is_builtin(s: &str) -> bool {
    matches!(s, "cx" | "_window" | "true" | "false" | "self" | "Self")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rml_engine::parser::ast::{Element, EachClause, EventHandler};
    use rust_rml_engine::parser::Span;

    #[test]
    fn classify_tag_position() {
        // <div> 在 offset 1..4（标签名 div）
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 10),
            ..Default::default()
        };
        let root = Node::Element(elem);
        assert_eq!(
            classify_symbol_at(&root, "", 2),
            Some(Symbol::Tag("div".to_string()))
        );
    }

    #[test]
    fn classify_bind_attr_field() {
        // 构造 <div count={count}> 的属性
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 30),
            attributes: vec![Attribute::Bind {
                name: "count".to_string(),
                expr: "count".to_string(),
                span: Span::new(5, 20),
            }],
            ..Default::default()
        };
        let root = Node::Element(elem);
        assert_eq!(
            classify_symbol_at(&root, "", 10),
            Some(Symbol::Field("count".to_string()))
        );
    }

    #[test]
    fn classify_event_attr_command() {
        let elem = Element {
            tag: "button".to_string(),
            span: Span::new(0, 40),
            attributes: vec![Attribute::Event {
                name: "onclick".to_string(),
                handler: EventHandler::Ident("on_click".to_string()),
                span: Span::new(8, 35),
            }],
            ..Default::default()
        };
        let root = Node::Element(elem);
        assert_eq!(
            classify_symbol_at(&root, "", 15),
            Some(Symbol::Command("on_click".to_string()))
        );
    }

    #[test]
    fn classify_bind_with_builtin_returns_none() {
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 30),
            attributes: vec![Attribute::Bind {
                name: "value".to_string(),
                expr: "cx".to_string(),
                span: Span::new(5, 20),
            }],
            ..Default::default()
        };
        let root = Node::Element(elem);
        assert_eq!(classify_symbol_at(&root, "", 10), None);
    }

    #[test]
    fn classify_static_attr_returns_none() {
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 30),
            attributes: vec![Attribute::Static {
                name: "class".to_string(),
                value: "container".to_string(),
                span: Span::new(5, 22),
            }],
            ..Default::default()
        };
        let root = Node::Element(elem);
        assert_eq!(classify_symbol_at(&root, "", 10), None);
    }

    #[test]
    fn classify_directive_each_iterable() {
        // 指令无独立 span，靠表达式内容匹配
        let elem = Element {
            tag: "li".to_string(),
            span: Span::new(0, 40),
            directives: vec![rust_rml_engine::parser::ast::Directive::Each(EachClause {
                item: "item".to_string(),
                index: None,
                iterable: "items".to_string(),
            })],
            ..Default::default()
        };
        let root = Node::Element(elem);
        // 光标在元素 span 内任意位置（非属性），应识别为 Field(items)
        assert_eq!(
            classify_symbol_at(&root, "", 5),
            Some(Symbol::Field("items".to_string()))
        );
    }
}
