//! 引用查找：在 .rml AST 中收集指定符号的所有引用位置
//!
//! references/rename 共用本模块的引用收集逻辑。
//! Tag 引用：所有同名开标签的 tag_name_span。
//! Field 引用：所有绑定属性/指令/插值中根标识符匹配的属性 span。
//! Command 引用：所有事件属性中处理器名匹配的属性 span。

use lsp_types::{Location, Position, Url};

use rust_rml_engine::parser::ast::{Attribute, Node};

use crate::crosslang::resolver::parse_binding_path;
use crate::features::ast_util::{event_handler_name, tag_name_span};
use crate::features::definition::find_definition;
use crate::features::symbol::{classify_symbol_at, Symbol};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::Workspace;

/// 在 .rml 中查找符号的所有引用
///
/// 返回的 Location 列表按 AST 遍历顺序排列（深度优先）。
/// `include_declaration == true` 时，定义点（若能解析到）插在列表头部。
pub fn find_references(
    uri: &Url,
    position: Position,
    include_declaration: bool,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Vec<Location> {
    let doc = match workspace.document(uri) {
        Some(d) => d,
        None => return Vec::new(),
    };
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = match tree.root.as_ref() {
        Some(r) => r,
        None => return Vec::new(),
    };

    let symbol = match classify_symbol_at(root, source, byte_offset) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut locations = Vec::new();
    if include_declaration {
        if let Some(def_resp) = find_definition(uri, position, workspace, rust_query) {
            if let lsp_types::GotoDefinitionResponse::Array(defs) = def_resp {
                locations.extend(defs);
            }
        }
    }

    let mut collector = RefCollector {
        symbol: &symbol,
        source,
        line_starts,
        uri,
        locations: &mut locations,
    };
    collect_in_node(root, &mut collector);

    locations
}

/// 引用收集器（避免重复传参）
struct RefCollector<'a> {
    symbol: &'a Symbol,
    source: &'a str,
    line_starts: &'a [u32],
    uri: &'a Url,
    locations: &'a mut Vec<Location>,
}

impl<'a> RefCollector<'a> {
    fn push(&mut self, byte_start: usize, byte_end: usize) {
        let range = conv::span_to_range(
            rust_rml_engine::parser::Span::new(byte_start, byte_end),
            self.source,
            self.line_starts,
        );
        self.locations.push(Location {
            uri: self.uri.clone(),
            range,
        });
    }
}

/// 递归遍历 Node 收集引用
fn collect_in_node(node: &Node, c: &mut RefCollector<'_>) {
    match node {
        Node::Element(elem) => {
            // Tag 引用：tag_name_span
            if let Symbol::Tag(name) = c.symbol {
                if &elem.tag == name {
                    let span = tag_name_span(elem);
                    c.push(span.start, span.end);
                }
            }

            // 属性引用
            for attr in &elem.attributes {
                match attr {
                    Attribute::Bind { expr, span, .. } => {
                        if let Symbol::Field(name) = c.symbol {
                            if let Some(path) = parse_binding_path(expr) {
                                if &path.root == name {
                                    c.push(span.start, span.end);
                                }
                            }
                        }
                    }
                    Attribute::Event { handler, span, .. } => {
                        if let Symbol::Command(name) = c.symbol {
                            if event_handler_name(handler) == name {
                                c.push(span.start, span.end);
                            }
                        }
                    }
                    Attribute::Static { .. } => {}
                }
            }

            // 指令表达式引用（Field）
            if let Symbol::Field(name) = c.symbol {
                use crate::features::ast_util::directive_expr;
                for d in &elem.directives {
                    if let Some(expr) = directive_expr(d) {
                        if let Some(path) = parse_binding_path(expr) {
                            if &path.root == name {
                                // 指令无独立 span，用 elem.span 近似（与 symbol 分类一致）
                                c.push(elem.span.start, elem.span.end);
                            }
                        }
                    }
                }
            }

            // 递归子节点
            for child in &elem.children {
                collect_in_node(child, c);
            }
        }
        Node::Interpolation(expr) => {
            if let Symbol::Field(name) = c.symbol {
                if let Some(path) = parse_binding_path(expr) {
                    if &path.root == name {
                        // Interpolation 无独立 span，跳过精确位置（无法定位）
                        // 这里不收集，避免误报整段文本
                    }
                }
            }
        }
        Node::MixedText(segs) => {
            for seg in segs {
                if let rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) = seg {
                    if let Symbol::Field(name) = c.symbol {
                        if let Some(path) = parse_binding_path(expr) {
                            if &path.root == name {
                                // MixedText 段无独立 span，跳过
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::ast_util::find_element_at_offset;
    use crate::features::symbol::classify_symbol_at;
    use crate::rust::NoopQuery;
    use crate::server::conv::offset_to_position;
    use crate::workspace::Workspace;

    fn ws_with_doc(rml_uri: &Url, source: &str) -> Workspace {
        let mut ws = Workspace::new();
        ws.open_document(rml_uri.clone(), source, 0);
        ws
    }

    #[test]
    fn find_tag_references_counts_open_tags() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><div><div></div></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;
        // 光标在第一个 div 标签名上（component 之后 offset 12..15）
        let pos = Position { line: 0, character: 12 };
        let locs = find_references(&rml, pos, false, &ws, &q);
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn find_field_references_in_bind_attrs() {
        let rml = Url::parse("file:///x.rml").unwrap();
        // 用 component 包裹避免 RML 单根元素约束
        let source = "<component><div count={count}></div><div count={count}></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;

        // 用 AST 找首个 Bind 属性的实际 span，定位光标到该 span 中点
        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        // 光标 offset 在第一个 div 内（component > div）
        let elem = find_element_at_offset(root, 15).unwrap();
        let bind_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Bind { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (bind_attr.start + bind_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        assert_eq!(
            classify_symbol_at(root, source, mid),
            Some(Symbol::Field("count".to_string()))
        );
        let locs = find_references(&rml, pos, false, &ws, &q);
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn find_command_references_in_event_attrs() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><button onclick={on_click}></button><button onclick={on_click}></button></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;

        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        let elem = find_element_at_offset(root, 15).unwrap();
        let event_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Event { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (event_attr.start + event_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        assert_eq!(
            classify_symbol_at(root, source, mid),
            Some(Symbol::Command("on_click".to_string()))
        );
        let locs = find_references(&rml, pos, false, &ws, &q);
        assert_eq!(locs.len(), 2);
    }

    #[test]
    fn find_references_returns_empty_when_cursor_on_static() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><div class=\"x\"></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;
        // class 属性 span 中点
        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        let elem = find_element_at_offset(root, 15).unwrap();
        let static_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (static_attr.start + static_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        let locs = find_references(&rml, pos, false, &ws, &q);
        assert!(locs.is_empty());
    }
}
