//! textDocument/formatting 处理

use anyhow::Result;
use lsp_types::{DocumentFormattingParams, TextEdit};

use crate::features::formatting;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_formatting(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Vec<TextEdit>>> {
    let params: DocumentFormattingParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;

    // .rs / .rml.rs 格式化由 rustfmt 处理，LSP 不参与
    if doctype::is_rust_file(&uri) {
        return Ok(None);
    }

    let doc = match state.workspace.document(&uri) {
        Some(d) => d,
        None => return Ok(None),
    };
    let source = doc.tree.text().to_string();

    Ok(formatting::format_document(&source, &params.options))
}
