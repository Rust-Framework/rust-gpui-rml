//! textDocument/references 处理

use anyhow::Result;
use lsp_types::{Location, ReferenceParams};

use crate::features::references as feat;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_references(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Vec<Location>>> {
    let params: ReferenceParams = serde_json::from_value(params)?;
    let uri = params.text_document_position.text_document.uri.clone();
    let position = params.text_document_position.position;
    let include_decl = params.context.include_declaration;

    if doctype::is_rust_file(&uri) {
        // .rs / .rml.rs → 委托 rust_query.find_references
        let locs = state
            .rust_query
            .find_references(&uri, position, include_decl)
            .into_iter()
            .map(|sl| Location {
                uri: sl.uri,
                range: sl.range,
            })
            .collect::<Vec<_>>();
        if locs.is_empty() {
            return Ok(None);
        }
        return Ok(Some(locs));
    }

    let locs = feat::find_references(
        &uri,
        position,
        include_decl,
        &state.workspace,
        &*state.rust_query,
    );
    if locs.is_empty() {
        Ok(None)
    } else {
        Ok(Some(locs))
    }
}
