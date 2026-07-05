//! Rust 语义集成层
//!
//! 通过 `RustSemanticQuery` trait 隔离 rust-analyzer API。
//! - `query.rs`：trait + 中性类型定义（无 RA 依赖）
//! - `adapter.rs`：RaAdapter 实现（`#[cfg(feature = "rust-backend")]`，Phase 2）
//! - `host.rs`：AnalysisHost 生命周期管理（`#[cfg(feature = "rust-backend")]`，Phase 2）
//!
//! 无 `rust-backend` feature 时使用 `NoopQuery` 降级，所有查询返回空。

pub mod query;

#[cfg(feature = "rust-backend")]
pub mod adapter;
#[cfg(feature = "rust-backend")]
pub mod host;

pub use query::{
    ComponentInfo, CompletionEntry, HoverInfo, RustDiagnostic, RustSemanticQuery, SymbolInfo,
    SymbolKind, SymbolLocation,
};

#[cfg(feature = "rust-backend")]
pub use adapter::RaAdapter;
#[cfg(feature = "rust-backend")]
pub use host::RaHost;

use lsp_types::{Position, Url};

/// 空实现：RA 不可用或 workspace 未加载完成时使用
pub struct NoopQuery;

impl RustSemanticQuery for NoopQuery {
    fn open_document(&mut self, _uri: &Url, _text: &str) {}
    fn apply_change(&mut self, _uri: &Url, _text: &str) {}
    fn close_document(&mut self, _uri: &Url) {}

    fn goto_definition(&self, _uri: &Url, _pos: Position) -> Vec<SymbolLocation> {
        Vec::new()
    }
    fn hover(&self, _uri: &Url, _pos: Position) -> Option<HoverInfo> {
        None
    }
    fn completion(&self, _uri: &Url, _pos: Position) -> Vec<CompletionEntry> {
        Vec::new()
    }
    fn diagnostics(&self, _uri: &Url) -> Vec<RustDiagnostic> {
        Vec::new()
    }

    fn resolve_member(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _member: &str,
    ) -> Option<SymbolInfo> {
        None
    }
    fn find_struct(&self, _struct_name: &str) -> Option<SymbolLocation> {
        None
    }
    fn struct_slots(&self, _rml_rs_uri: &Url, _struct_name: &str) -> Vec<String> {
        Vec::new()
    }
    fn command_signature(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _method: &str,
    ) -> Option<SymbolInfo> {
        None
    }

    fn list_components(&self, _prefix: &str) -> Vec<ComponentInfo> {
        Vec::new()
    }

    fn is_ready(&self) -> bool {
        false
    }

    fn find_references(
        &self,
        _uri: &Url,
        _pos: Position,
        _include_declaration: bool,
    ) -> Vec<SymbolLocation> {
        Vec::new()
    }

    fn rename_member(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _member: &str,
        _new_name: &str,
    ) -> Vec<lsp_types::TextEdit> {
        Vec::new()
    }

    fn rename_struct(
        &self,
        _old_name: &str,
        _new_name: &str,
    ) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>> {
        std::collections::HashMap::new()
    }
}
