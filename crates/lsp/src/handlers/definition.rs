//! textDocument/definition 处理

use anyhow::Result;
use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse};

use crate::features::definition;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_definition(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<GotoDefinitionResponse>> {
    let params: GotoDefinitionParams = serde_json::from_value(params)?;
    let uri = params.text_document_position_params.text_document.uri.clone();
    let position = params.text_document_position_params.position;

    if doctype::is_rust_codebehind(&uri) {
        Ok(definition::find_definition_rust(
            params.text_document_position_params,
            &*state.rust_query,
        ))
    } else {
        Ok(definition::find_definition(
            &uri,
            position,
            &state.workspace,
            &*state.rust_query,
        ))
    }
}
