//! gpui-component CodeEditor provider 实现：桥接 trait → LspClient IPC。

pub mod completion;
pub mod definition;
pub mod hover;
pub mod semantic_tokens;

pub use completion::LspCompletionProvider;
pub use definition::LspDefinitionProvider;
pub use hover::LspHoverProvider;
pub use semantic_tokens::LspSemanticTokensProvider;
