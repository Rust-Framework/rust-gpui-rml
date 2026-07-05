//! 定义跳转：标签 → .rml.rs struct，绑定表达式 → 字段定义，事件处理器 → #[command] 方法
//!
//! 复用 crosslang::coordinator 的现有跨语言能力，仅做 .rml 光标位置识别与结果包装。

use lsp_types::{
    GotoDefinitionResponse, Location, Position, TextDocumentPositionParams, Url,
};

use rust_rml_engine::parser::ast::{Attribute, Node};

use crate::crosslang::{find_component, goto_def_for_binding};
use crate::features::ast_util::{
    event_handler_name, find_attribute_at_offset, find_element_at_offset, tag_name_span,
};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::Workspace;

/// 执行定义跳转查询
pub fn find_definition(
    uri: &Url,
    position: Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<GotoDefinitionResponse> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;
    let elem = find_element_at_offset(root, byte_offset)?;

    // 1. 标签名位置 → 跨语言跳转到 #[component] struct
    let tag_span = tag_name_span(elem);
    if tag_span.contains(byte_offset) {
        let loc = find_component(&elem.tag, rust_query)?;
        return Some(GotoDefinitionResponse::Array(vec![Location {
            uri: loc.uri,
            range: loc.range,
        }]));
    }

    // 2. 属性位置
    if let Some(attr) = find_attribute_at_offset(elem, byte_offset) {
        match attr {
            Attribute::Bind { expr, .. } => {
                let loc = goto_def_for_binding(uri, expr, workspace.index(), rust_query)?;
                return Some(GotoDefinitionResponse::Array(vec![Location {
                    uri: loc.uri,
                    range: loc.range,
                }]));
            }
            Attribute::Event { handler, .. } => {
                let cmd_name = event_handler_name(handler);
                let loc = find_command_definition(uri, cmd_name, workspace, rust_query)?;
                return Some(GotoDefinitionResponse::Array(vec![Location {
                    uri: loc.uri,
                    range: loc.range,
                }]));
            }
            Attribute::Static { .. } => {}
        }
    }

    // 3. 文本插值 / 指令表达式 → 按绑定表达式处理
    if let Some(expr) = find_expr_at_offset(root, byte_offset) {
        let loc = goto_def_for_binding(uri, expr, workspace.index(), rust_query)?;
        return Some(GotoDefinitionResponse::Array(vec![Location {
            uri: loc.uri,
            range: loc.range,
        }]));
    }

    None
}

/// 查找 #[command] 方法的定义位置
///
/// 在 .rml 配对的 .rml.rs 中，找包含该命令的 struct，通过 rust_query.resolve_member 取位置。
fn find_command_definition(
    rml_uri: &Url,
    cmd_name: &str,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<crate::rust::SymbolLocation> {
    let rml_rs_uri = workspace.codebehind_uri(rml_uri)?;
    let metadata_map = workspace.index().metadata_for(rml_uri)?;
    for (struct_name, meta) in metadata_map {
        if meta.commands.iter().any(|c| c == cmd_name) {
            let symbol = rust_query.resolve_member(rml_rs_uri, struct_name, cmd_name)?;
            return symbol.location;
        }
    }
    None
}

/// 在 AST 中查找光标位置命中的插值/指令表达式
///
/// 用于 definition 的第三类识别：光标在 `{expr}` 文本插值或指令表达式上。
/// 由于指令无独立 span，这里只处理文本插值（Interpolation/MixedText）。
fn find_expr_at_offset(root: &Node, offset: usize) -> Option<&str> {
    match root {
        Node::Interpolation(expr) => Some(expr),
        Node::MixedText(segs) => {
            // MixedText 无独立 span，无法精确定位；返回第一个插值（MVP 简化）
            segs.iter().find_map(|seg| match seg {
                rust_rml_engine::parser::ast::TextSegment::Interpolation(expr) => Some(expr.as_str()),
                _ => None,
            })
        }
        Node::Element(elem) => {
            for child in &elem.children {
                if elem.span.contains(offset) {
                    if let Some(expr) = find_expr_at_offset(child, offset) {
                        return Some(expr);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// 处理 .rml.rs 文件的定义跳转：委托 rust_query.goto_definition
pub fn find_definition_rust(
    params: TextDocumentPositionParams,
    rust_query: &dyn RustSemanticQuery,
) -> Option<GotoDefinitionResponse> {
    let uri = &params.text_document.uri;
    let pos = params.position;
    let locations = rust_query.goto_definition(uri, pos);
    if locations.is_empty() {
        return None;
    }
    Some(GotoDefinitionResponse::Array(
        locations
            .into_iter()
            .map(|loc| Location {
                uri: loc.uri,
                range: loc.range,
            })
            .collect(),
    ))
}
