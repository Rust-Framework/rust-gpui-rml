//! 便捷 re-exports：`use rust_rml_client::prelude::*;`

pub use crate::editor::install_lsp_providers;
pub use crate::grammar::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};
pub use crate::lsp_client::{file_path_to_uri, LspClient};
pub use crate::providers::{
    RmlCompletionProvider, RmlDefinitionProvider, RmlHoverProvider, RmlSemanticTokensProvider,
};
pub use crate::registry::register_rml_language;
