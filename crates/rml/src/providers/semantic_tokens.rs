//! RmlSemanticTokensProvider: 桥接 gpui-component DocumentRangeSemanticTokensProvider → LspClient。
//!
//! gpui-component 的 `Lsp::update_semantic_tokens` 内部已实现 100ms debounce +
//! viewport 二分过滤 + delta 解码。本 provider 仅需在 trait 方法被调用时通过
//! IPC 拉取完整 `SemanticTokens` 返回，由 gpui-component 端做后续处理。

use std::ops::Range;
use std::sync::Arc;

use anyhow::Result;
use gpui::{App, Task, Window};
use gpui_component::input::DocumentRangeSemanticTokensProvider;
use lsp_types::{SemanticTokens, SemanticTokensLegend, Uri};
use ropey::Rope;

use crate::lsp_client::LspClient;

pub struct RmlSemanticTokensProvider {
    client: Arc<LspClient>,
    uri: Uri,
    legend: SemanticTokensLegend,
}

impl RmlSemanticTokensProvider {
    pub fn new(client: Arc<LspClient>, uri: Uri, legend: SemanticTokensLegend) -> Self {
        Self {
            client,
            uri,
            legend,
        }
    }
}

impl DocumentRangeSemanticTokensProvider for RmlSemanticTokensProvider {
    fn legend(&self) -> SemanticTokensLegend {
        self.legend.clone()
    }

    fn semantic_tokens(
        &self,
        _text: &Rope,
        _range: Range<usize>,
        _window: &mut Window,
        cx: &mut App,
    ) -> Task<Result<SemanticTokens>> {
        let rx = self.client.semantic_tokens_full(&self.uri);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()
                .map_err(|e| anyhow::anyhow!("semantic tokens channel closed: {e}"))??;
            let tokens: SemanticTokens = serde_json::from_value(resp)?;
            Ok(tokens)
        })
    }
}
