//! 文档符号：遍历 AST 构建 DocumentSymbol 树
//!
//! 每个元素 → 一个 DocumentSymbol；根元素用 MODULE，其余用 CLASS。
//! 子节点递归构建 children（仅 Element 节点，跳过 Text/Interpolation）。

use lsp_types::{DocumentSymbol, DocumentSymbolResponse, SymbolKind};

use rust_rml_engine::parser::ast::{Element, Node};
use rust_rml_engine::tags;

use crate::server::conv;
use crate::workspace::Workspace;

/// 构建文档符号树
pub fn document_symbol(
    uri: &lsp_types::Url,
    workspace: &Workspace,
) -> Option<DocumentSymbolResponse> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let root = tree.root.as_ref()?;

    let symbol = build_symbol(root, source, line_starts)?;
    Some(DocumentSymbolResponse::Nested(vec![symbol]))
}

fn build_symbol(node: &Node, source: &str, line_starts: &[u32]) -> Option<DocumentSymbol> {
    match node {
        Node::Element(elem) => Some(build_element_symbol(elem, source, line_starts)),
        _ => None,
    }
}

fn build_element_symbol(
    elem: &Element,
    source: &str,
    line_starts: &[u32],
) -> DocumentSymbol {
    let range = conv::span_to_range(elem.span, source, line_starts);
    let kind = if tags::is_root_tag(&elem.tag) {
        SymbolKind::MODULE
    } else if tags::component_lookup(&elem.tag).is_some() {
        SymbolKind::CLASS
    } else if tags::is_builtin_html_tag(&elem.tag) {
        SymbolKind::OBJECT
    } else {
        SymbolKind::CLASS
    };
    let detail = format!(
        "{} attrs, {} children",
        elem.attributes.len(),
        elem.children.len()
    );
    let children: Vec<DocumentSymbol> = elem
        .children
        .iter()
        .filter_map(|c| build_symbol(c, source, line_starts))
        .collect();
    #[allow(deprecated)]
    DocumentSymbol {
        name: elem.tag.clone(),
        detail: Some(detail),
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range: range,
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}
