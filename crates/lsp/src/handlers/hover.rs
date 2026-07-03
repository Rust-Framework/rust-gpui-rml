//! textDocument/hover 处理

use anyhow::Result;
use lsp_types::{Hover, HoverParams};

use crate::features::hover;
use crate::server::connection::ServerState;

pub fn handle_hover(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Hover>> {
    let params: HoverParams = serde_json::from_value(params)?;
    let uri = &params.text_document_position_params.text_document.uri;
    let position = params.text_document_position_params.position;

    Ok(hover::hover(uri, position, &state.workspace))
}
