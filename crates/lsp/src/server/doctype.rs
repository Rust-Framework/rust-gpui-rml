//! 文档类型判断：根据 URI 路由到对应后端
//!
//! - `.rml` → RML 引擎（engine + workspace）
//! - `.rs` / `.rml.rs` → rust-analyzer 后端（rust_query）

use lsp_types::Url;

/// `.rml.rs` 代码后置文件 → rust_query 后端
pub fn is_rust_codebehind(uri: &Url) -> bool {
    uri.path().ends_with(".rml.rs")
}

/// `.rs` 源文件（不含 `.rml.rs`）→ rust_query 后端
pub fn is_rust_source(uri: &Url) -> bool {
    uri.path().ends_with(".rs") && !is_rust_codebehind(uri)
}

/// 任何 Rust 文件（`.rs` 或 `.rml.rs`）→ rust_query 后端
pub fn is_rust_file(uri: &Url) -> bool {
    is_rust_codebehind(uri) || is_rust_source(uri)
}

/// `.rml` 标记文件 → RML 引擎
pub fn is_rml_markup(uri: &Url) -> bool {
    uri.path().ends_with(".rml") && !is_rust_codebehind(uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uri(path: &str) -> Url {
        Url::parse(&format!("file:///workspace{path}")).unwrap()
    }

    #[test]
    fn routing_rml_markup() {
        assert!(is_rml_markup(&uri("/src/foo.rml")));
        assert!(!is_rml_markup(&uri("/src/foo.rml.rs")));
        assert!(!is_rml_markup(&uri("/src/foo.rs")));
    }

    #[test]
    fn routing_rust_codebehind() {
        assert!(is_rust_codebehind(&uri("/src/foo.rml.rs")));
        assert!(!is_rust_codebehind(&uri("/src/foo.rml")));
        assert!(!is_rust_codebehind(&uri("/src/foo.rs")));
    }

    #[test]
    fn routing_rust_source() {
        assert!(is_rust_source(&uri("/src/foo.rs")));
        assert!(is_rust_source(&uri("/src/lib.rs")));
        assert!(!is_rust_source(&uri("/src/foo.rml.rs")));
        assert!(!is_rust_source(&uri("/src/foo.rml")));
    }

    #[test]
    fn routing_rust_file_unified() {
        assert!(is_rust_file(&uri("/src/foo.rs")));
        assert!(is_rust_file(&uri("/src/foo.rml.rs")));
        assert!(!is_rust_file(&uri("/src/foo.rml")));
    }
}
