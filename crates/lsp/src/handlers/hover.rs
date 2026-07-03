//! textDocument/hover 处理

use anyhow::Result;
use lsp_types::{Hover, HoverContents, HoverParams, MarkedString};

use crate::features::hover;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_hover(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Hover>> {
    let params: HoverParams = serde_json::from_value(params)?;
    let uri = params.text_document_position_params.text_document.uri.clone();
    let position = params.text_document_position_params.position;

    if doctype::is_rust_codebehind(&uri) {
        Ok(state
            .rust_query
            .hover(&uri, position)
            .map(|info| Hover {
                range: info.range,
                contents: HoverContents::Scalar(MarkedString::String(info.content)),
            }))
    } else {
        Ok(hover::hover(&uri, position, &state.workspace))
    }
}
