//! LspHoverProvider: 桥接 gpui-component HoverProvider → LspClient。

use std::sync::Arc;

use anyhow::Result;
use gpui::{App, Task, Window};
use gpui_component::input::HoverProvider;
use lsp_types::{Hover, Uri};
use ropey::Rope;

use super::position_util::offset_to_position_utf16;
use crate::lsp_client::LspClient;

pub struct LspHoverProvider {
    client: Arc<LspClient>,
    uri: Uri,
}

impl LspHoverProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri) -> Self {
        Self { client, uri }
    }
}

impl HoverProvider for LspHoverProvider {
    fn hover(
        &self,
        text: &Rope,
        offset: usize,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<Option<Hover>>> {
        let position = offset_to_position_utf16(text, offset);
        log::debug!("[rml-lsp] client hover: offset={}, pos={:?}", offset, position);
        let rx = self.client.hover(&self.uri, position);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let result: Option<Hover> = serde_json::from_value(resp)?;
            Ok(result)
        })
    }
}
