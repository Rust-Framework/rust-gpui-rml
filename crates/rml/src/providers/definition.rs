//! LspDefinitionProvider: 桥接 gpui-component DefinitionProvider → LspClient。

use std::sync::Arc;

use anyhow::Result;
use gpui::{App, Task, Window};
use gpui_component::input::DefinitionProvider;
use lsp_types::{Location, LocationLink, Uri};
use ropey::Rope;
use serde_json::Value;

use super::position_util::offset_to_position_utf16;
use crate::lsp_client::LspClient;

pub struct LspDefinitionProvider {
    client: Arc<LspClient>,
    uri: Uri,
}

impl LspDefinitionProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri) -> Self {
        Self { client, uri }
    }
}

impl DefinitionProvider for LspDefinitionProvider {
    fn definitions(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Vec<LocationLink>>> {
        let position = offset_to_position_utf16(text, offset);
        let rx = self.client.definition(&self.uri, position);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            Ok(parse_definition_response(resp))
        })
    }
}

/// LSP definition 响应可能是 Location、Location[]、LocationLink[] 或 null。
fn parse_definition_response(resp: Value) -> Vec<LocationLink> {
    match resp {
        Value::Null => Vec::new(),
        Value::Array(arr) if arr.is_empty() => Vec::new(),
        Value::Array(_) => {
            if let Ok(links) = serde_json::from_value::<Vec<LocationLink>>(resp.clone()) {
                return links;
            }
            if let Ok(locs) = serde_json::from_value::<Vec<Location>>(resp) {
                return locs.into_iter().map(location_to_link).collect();
            }
            Vec::new()
        }
        _ => {
            if let Ok(loc) = serde_json::from_value::<Location>(resp) {
                return vec![location_to_link(loc)];
            }
            Vec::new()
        }
    }
}

fn location_to_link(loc: Location) -> LocationLink {
    LocationLink {
        origin_selection_range: None,
        target_uri: loc.uri,
        target_range: loc.range,
        target_selection_range: loc.range,
    }
}
