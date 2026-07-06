//! 一行集成所有 RML LSP providers 到 InputState。
//!
//! 调用 `install_lsp_providers(&mut state, client, uri)` 即可同时安装
//! completion / hover / definition / semantic_tokens 四个 provider。

use std::rc::Rc;
use std::sync::Arc;

use gpui_component::input::InputState;
use lsp_types::Uri;

use crate::lsp_client::LspClient;
use crate::providers::{
    RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider,
};

pub fn install_lsp_providers(state: &mut InputState, client: Arc<LspClient>, uri: Uri) {
    state.lsp.completion_provider =
        Some(Rc::new(RmlCompletionProvider::new(client.clone(), uri.clone())));
    state.lsp.hover_provider = Some(Rc::new(RmlHoverProvider::new(client.clone(), uri.clone())));
    state.lsp.definition_provider =
        Some(Rc::new(RmlDefinitionProvider::new(client.clone(), uri.clone())));
    if let Some(legend) = client.semantic_tokens_legend() {
        state.lsp.semantic_tokens_provider =
            Some(Rc::new(RmlSemanticTokensProvider::new(client, uri, legend)));
    }
}
