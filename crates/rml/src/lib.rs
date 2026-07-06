//! rust-rml-client: RML 客户端整合 crate
//!
//! 统一封装 RML 语法服务的客户端逻辑：
//! - `grammar`: tree-sitter 静态着色（语法 + 查询）
//! - `lsp_client`: LSP 子进程通信（JSON-RPC over stdio）
//! - `providers`: gpui-component CodeEditor provider 实现（completion/hover/definition/semantic_tokens）
//! - `editor`: 一行集成所有 providers 到 InputState
//! - `registry`: 一行注册 RML 语言到 LanguageRegistry
//!
//! demo 侧集成示例：
//! ```ignore
//! use rust_rml_client::prelude::*;
//!
//! register_rml_language();
//! let client = LspClient::spawn(&workspace_root)?;
//! install_lsp_providers(&mut state, Arc::new(client), uri);
//! ```

pub mod editor;
pub mod grammar;
pub mod lsp_client;
pub mod prelude;
pub mod providers;
pub mod registry;

// 顶层 re-exports
pub use editor::install_lsp_providers;
pub use grammar::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};
pub use lsp_client::{file_path_to_uri, LspClient};
pub use providers::{
    RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider,
};
pub use registry::register_rml_language;
