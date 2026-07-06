//! textDocument/hover 处理

use anyhow::Result;
use lsp_types::{Hover, HoverContents, HoverParams, MarkupContent, MarkupKind};

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

    if doctype::is_rust_file(&uri) {
        Ok(state
            .rust_query
            .hover(&uri, position)
            .map(|info| Hover {
                range: info.range,
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info.content,
                }),
            }))
    } else {
        Ok(hover::hover(
            &uri,
            position,
            &state.workspace,
            state.rust_query.as_ref(),
            &state.i18n_index,
            &state.css_index,
        ))
    }
}
