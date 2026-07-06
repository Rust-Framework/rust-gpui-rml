//! LspCompletionProvider: 桥接 gpui-component CompletionProvider → LspClient。

use std::sync::Arc;

use anyhow::Result;
use gpui::{Context, Task, Window};
use gpui_component::{input::{CompletionProvider, InputState}, RopeExt};
use lsp_types::{CompletionContext, CompletionResponse, Uri};
use ropey::Rope;

use crate::lsp_client::LspClient;

pub struct LspCompletionProvider {
    client: Arc<LspClient>,
    uri: Uri,
}

impl LspCompletionProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri) -> Self {
        Self { client, uri }
    }
}

impl CompletionProvider for LspCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let position = text.offset_to_position(offset);
        let rx = self.client.completion(&self.uri, position);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let result: CompletionResponse = serde_json::from_value(resp)?;
            Ok(result)
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        new_text
            .chars()
            .any(|c| c.is_alphanumeric() || c == '.' || c == '<' || c == ' ' || c == ':' || c == '{')
    }
}
