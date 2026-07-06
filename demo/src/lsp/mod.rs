//! LSP 集成模块：活动栏 LspExplorerPanel + CodeEditorTab + 状态栏贡献。
//!
//! LSP client / providers / grammar 由 `rust-rml-client` crate 提供，
//! 本模块仅保留 demo 专属的 UI 与状态管理。

#[path = "code_editor_tab.rml.rs"]
pub mod code_editor_tab;
pub mod file_tree;
pub mod lsp_status;
#[path = "lsp_explorer_panel.rml.rs"]
pub mod lsp_explorer_panel;

pub use code_editor_tab::CodeEditorTab;
pub use lsp_status::{ensure_lsp_status_item_registered, LspStatusState, LspStatusStateRef};
