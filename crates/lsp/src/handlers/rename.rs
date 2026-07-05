//! textDocument/rename 处理

use anyhow::Result;
use lsp_types::{RenameParams, WorkspaceEdit};

use crate::features::rename as feat;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_rename(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<WorkspaceEdit>> {
    let params: RenameParams = serde_json::from_value(params)?;
    let uri = params.text_document_position.text_document.uri.clone();
    let position = params.text_document_position.position;
    let new_name = params.new_name;

    // .rml.rs → 暂不支持（rust-analyzer 自身处理）
    if doctype::is_rust_codebehind(&uri) {
        return Ok(None);
    }

    Ok(feat::rename(
        &uri,
        position,
        &new_name,
        &state.workspace,
        &*state.rust_query,
    ))
}
