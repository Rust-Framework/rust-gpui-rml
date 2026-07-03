//! CodeEditorTab: 基于 InputState code_editor 模式的代码编辑器 Tab。
//!
//! 非贡献 Entity，由 MainWindow 直接管理。集成 LSP providers 提供补全/hover/跳转。
//! RML 声明式渲染：`<CodeEditor />` 自动应用 mono 字体 + size_full。

use std::path::Path;
use std::sync::Arc;

use gpui_component::input::{InputState, TabSize};
use lsp_types::Uri;
use rml::prelude::*;

use crate::lsp::{
    file_path_to_uri, LspClient, RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider,
};

#[component]
#[derive(Default)]
pub struct CodeEditorTab {
    editor_state: Option<Entity<InputState>>,
    #[allow(dead_code)]
    file_path: String,
    #[allow(dead_code)]
    uri: Option<Uri>,
    #[allow(dead_code)]
    lsp_client: Option<Arc<LspClient>>,
}

impl ILifecycle for CodeEditorTab {}

impl CodeEditorTab {
    pub fn new(
        file_path: &str,
        full_path: &Path,
        lsp_client: Arc<LspClient>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let text = std::fs::read_to_string(full_path).unwrap_or_default();
        let uri = file_path_to_uri(full_path).unwrap_or_else(|e| {
            log::error!("failed to create URI for {}: {e}", full_path.display());
            "file:///".parse::<lsp_types::Uri>().unwrap()
        });
        let language = if file_path.ends_with(".rml.rs") || file_path.ends_with(".rs") {
            "rust"
        } else {
            "rml"
        };

        lsp_client.open_document(&uri, &text, language);

        let editor_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    ..Default::default()
                })
                .default_value(&text);
            state.lsp.completion_provider =
                Some(std::rc::Rc::new(RmlCompletionProvider::new(
                    lsp_client.clone(),
                    uri.clone(),
                )));
            state.lsp.hover_provider =
                Some(std::rc::Rc::new(RmlHoverProvider::new(lsp_client.clone(), uri.clone())));
            state.lsp.definition_provider = Some(std::rc::Rc::new(RmlDefinitionProvider::new(
                lsp_client.clone(),
                uri.clone(),
            )));
            state
        });

        cx.new(|cx| {
            let uri_clone = uri.clone();
            let client_clone = lsp_client.clone();
            cx.observe(&editor_state, move |_, state, obs_cx| {
                let text = state.read(obs_cx).text().to_string();
                client_clone.change_document(&uri_clone, &text);
            })
            .detach();

            let mut view = Self::default();
            view.editor_state = Some(editor_state);
            view.file_path = file_path.to_string();
            view.uri = Some(uri);
            view.lsp_client = Some(lsp_client);
            view
        })
    }
}
