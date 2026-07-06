//! rust-rml-client: 语言服务客户端整合 crate
//!
//! 统一封装语法服务的客户端逻辑（高内聚低耦合）：
//! - `LanguageClient` —— 高级语言服务客户端（LSP server 进程 + IPC + provider 工厂 + grammar 注册）
//! - `DebugClient` —— 调试服务客户端骨架（DAP 协议，后续实现）
//! - `grammar` —— RML tree-sitter 静态着色（语法 + 查询）
//! - `lsp_client` —— LSP 子进程通信（JSON-RPC over stdio，LanguageClient 内部 IPC 层）
//! - `providers` —— gpui-component CodeEditor provider 实现（completion/hover/definition/semantic_tokens）
//! - `language_profile` —— 语言配置预设（LanguageProfile / DebugProfile）
//!
//! ## demo 侧集成示例
//!
//! ```ignore
//! use rust_rml_client::LanguageClient;
//!
//! let client = LanguageClient::unified(&workspace_root)?;
//! client.open_document(&uri, &text);
//! client.install_providers(&mut state, uri);
//! ```

pub mod debug_client;
pub mod grammar;
pub mod language_client;
pub mod language_profile;
pub mod lsp_client;
pub mod prelude;
pub mod providers;

// 顶层 re-exports
pub use debug_client::DebugClient;
pub use language_client::LanguageClient;
pub use language_profile::{DebugProfile, LanguageDescriptor, LanguageProfile, TreeSitterGrammar};
pub use lsp_client::{file_path_to_uri, LspClient};
pub use providers::{
    LspCompletionProvider, LspDefinitionProvider, LspHoverProvider, LspSemanticTokensProvider,
};
