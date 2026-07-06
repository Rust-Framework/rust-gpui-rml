//! LSP 集成模块：活动栏 LspExplorerPanel + CodeEditorTab + LSP providers + 状态栏贡献。

#[path = "code_editor_tab.rml.rs"]
pub mod code_editor_tab;
pub mod completion_provider;
pub mod definition_provider;
pub mod file_tree;
pub mod hover_provider;
pub mod lsp_client;
pub mod lsp_status;
#[path = "lsp_explorer_panel.rml.rs"]
pub mod lsp_explorer_panel;

pub use code_editor_tab::CodeEditorTab;
pub use completion_provider::RmlCompletionProvider;
pub use definition_provider::RmlDefinitionProvider;
pub use hover_provider::RmlHoverProvider;
pub use lsp_client::{file_path_to_uri, LspClient};
pub use lsp_status::{ensure_lsp_status_item_registered, LspStatusState, LspStatusStateRef};
