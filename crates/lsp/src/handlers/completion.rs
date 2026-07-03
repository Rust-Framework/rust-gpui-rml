//! textDocument/completion 处理

use anyhow::Result;
use lsp_types::{CompletionParams, CompletionResponse};

use crate::features::completion;
use crate::server::connection::ServerState;

pub fn handle_completion(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<CompletionResponse>> {
    let params: CompletionParams = serde_json::from_value(params)?;
    let uri = &params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    Ok(completion::complete(uri, position, &state.workspace))
}
