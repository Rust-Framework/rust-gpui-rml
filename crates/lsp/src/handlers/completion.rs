//! textDocument/completion 处理

use anyhow::Result;
use lsp_types::{
    CompletionItem, CompletionParams, CompletionResponse, InsertTextFormat,
};

use crate::features::completion;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_completion(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<CompletionResponse>> {
    let params: CompletionParams = serde_json::from_value(params)?;
    let uri = params.text_document_position.text_document.uri.clone();
    let position = params.text_document_position.position;

    if doctype::is_rust_file(&uri) {
        Ok(complete_rust(&uri, position, state))
    } else {
        Ok(completion::complete(
            &uri,
            position,
            &state.workspace,
            &*state.rust_query,
        ))
    }
}

/// `.rs` / `.rml.rs` 文件补全：从 rust_query 获取并转换为 LSP CompletionItem
fn complete_rust(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    state: &ServerState,
) -> Option<CompletionResponse> {
    let entries = state.rust_query.completion(uri, position);
    if entries.is_empty() {
        return None;
    }
    let items: Vec<CompletionItem> = entries
        .into_iter()
        .map(|e| CompletionItem {
            label: e.label,
            kind: Some(e.kind),
            detail: e.detail,
            insert_text: e.insert_text,
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        })
        .collect();
    Some(CompletionResponse::Array(items))
}
