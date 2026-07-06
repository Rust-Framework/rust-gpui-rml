//! gpui-component CodeEditor provider 实现：桥接 trait → LspClient IPC。

pub mod completion;
pub mod definition;
pub mod hover;
pub mod semantic_tokens;

pub use completion::RmlCompletionProvider;
pub use definition::RmlDefinitionProvider;
pub use hover::RmlHoverProvider;
pub use semantic_tokens::RmlSemanticTokensProvider;
