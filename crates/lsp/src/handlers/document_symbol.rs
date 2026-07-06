//! textDocument/documentSymbol 处理

use anyhow::Result;
use lsp_types::{DocumentSymbolParams, DocumentSymbolResponse};

use crate::features::document_symbol;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_document_symbol(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<DocumentSymbolResponse>> {
    let params: DocumentSymbolParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;

    // .rs / .rml.rs 文档符号委托 rust-analyzer 后端
    if doctype::is_rust_file(&uri) {
        let symbols = state.rust_query.document_symbol(&uri);
        return Ok(symbols.map(DocumentSymbolResponse::Nested));
    }

    Ok(document_symbol::document_symbol(&uri, &state.workspace))
}
