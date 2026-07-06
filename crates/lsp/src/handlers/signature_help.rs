//! textDocument/signatureHelp 处理

use anyhow::Result;
use lsp_types::{SignatureHelp, SignatureHelpParams};

use crate::features::signature_help as feat;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_signature_help(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<SignatureHelp>> {
    let params: SignatureHelpParams = serde_json::from_value(params)?;
    let uri = params.text_document_position_params.text_document.uri.clone();
    let position = params.text_document_position_params.position;

    // .rs / .rml.rs → 不提供签名帮助（rust-analyzer 自身处理）
    if doctype::is_rust_file(&uri) {
        return Ok(None);
    }

    Ok(feat::signature_help(
        &uri,
        position,
        &state.workspace,
        &*state.rust_query,
    ))
}
