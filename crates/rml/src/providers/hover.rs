//! LspHoverProvider: 桥接 gpui-component HoverProvider → LspClient。

use std::sync::Arc;

use anyhow::Result;
use gpui::{App, Task, Window};
use gpui_component::{input::HoverProvider, RopeExt};
use lsp_types::{Hover, Uri};
use ropey::Rope;

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
        let position = text.offset_to_position(offset);
        let rx = self.client.hover(&self.uri, position);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let result: Option<Hover> = serde_json::from_value(resp)?;
            Ok(result)
        })
    }
}
