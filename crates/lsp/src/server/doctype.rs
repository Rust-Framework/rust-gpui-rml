//! 文档类型判断：根据 URI 路由到对应后端
//!
//! - `.rml` → RML 引擎（engine + workspace）
//! - `.rml.rs` → rust-analyzer 后端（rust_query）

use lsp_types::Url;

/// `.rml.rs` 代码后置文件 → rust_query 后端
pub fn is_rust_codebehind(uri: &Url) -> bool {
    uri.path().ends_with(".rml.rs")
}

/// `.rml` 标记文件 → RML 引擎
pub fn is_rml_markup(uri: &Url) -> bool {
    uri.path().ends_with(".rml") && !is_rust_codebehind(uri)
}
