//! textDocument/foldingRange 处理

use anyhow::Result;
use lsp_types::{FoldingRange, FoldingRangeParams};

use crate::features;
use crate::server::connection::ServerState;
use crate::server::doctype;

pub fn handle_folding_range(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<Vec<FoldingRange>>> {
    let params: FoldingRangeParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;

    // .rs / .rml.rs 委托 rust_query（缩进策略）
    if doctype::is_rust_file(&uri) {
        return Ok(Some(state.rust_query.folding_ranges(&uri)));
    }

    // .rml 委托 features::fold
    Ok(Some(features::fold::fold_ranges(&uri, &state.workspace)))
}
