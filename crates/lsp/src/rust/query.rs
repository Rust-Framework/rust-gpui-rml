//! Rust 语义查询隔离层
//!
//! 定义 `RustSemanticQuery` trait + 中性类型，封装所有 rust-analyzer API 调用。
//! LSP 功能代码和跨语言协调器只依赖本 trait，不接触任何 `ra_ap_*` 类型。
//! RA 升级时只需修改 `adapter.rs` 中的类型转换函数，其余代码零改动。

use lsp_types::{
    CompletionItemKind, DiagnosticSeverity, Position, Range, Url,
};

// ──────────────────────────────────────────────────────────────────────────
// 中性类型（无 RA 依赖）
// ──────────────────────────────────────────────────────────────────────────

/// 符号种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Field,
    Method,
    Struct,
    Enum,
    Trait,
    Module,
    Local,
}

/// 符号信息（字段/方法/类型）
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    /// 类型字符串，如 `"i32"`、`"String"`、`"Vec<TabItem>"`
    pub type_str: Option<String>,
    /// 文档注释
    pub doc: Option<String>,
    /// 定义位置
    pub location: Option<SymbolLocation>,
}

/// 符号定义位置
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    pub uri: Url,
    pub range: Range,
}

/// 悬停信息
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Markdown 格式内容
    pub content: String,
    pub range: Option<Range>,
}

/// 补全条目（中性类型，非 lsp_types::CompletionItem）
#[derive(Debug, Clone)]
pub struct CompletionEntry {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

/// Rust 诊断（中性类型）
#[derive(Debug, Clone)]
pub struct RustDiagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub code: Option<String>,
}

/// `#[component]` 标注的 struct 信息（用于动态组件标签补全）
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub name: String,
    pub location: Option<SymbolLocation>,
}

// ──────────────────────────────────────────────────────────────────────────
// 隔离 trait
// ──────────────────────────────────────────────────────────────────────────

/// Rust 语义查询抽象
///
/// 所有 `ra_ap_*` 类型（`FileId`/`FilePosition`/`NavigationTarget`/`Analysis` 等）
/// 绝不出现在 trait 接口中。实现方（`RaAdapter`）负责类型转换。
pub trait RustSemanticQuery: Send + Sync {
    // ── 文档同步 ──

    /// 打开 .rml.rs 文档
    fn open_document(&mut self, uri: &Url, text: &str);

    /// 更新 .rml.rs 文档内容
    fn apply_change(&mut self, uri: &Url, text: &str);

    /// 关闭 .rml.rs 文档
    fn close_document(&mut self, uri: &Url);

    // ── .rml.rs 原生 LSP 查询 ──

    /// goto definition
    fn goto_definition(&self, uri: &Url, pos: Position) -> Vec<SymbolLocation>;

    /// hover
    fn hover(&self, uri: &Url, pos: Position) -> Option<HoverInfo>;

    /// completion
    fn completion(&self, uri: &Url, pos: Position) -> Vec<CompletionEntry>;

    /// 诊断
    fn diagnostics(&self, uri: &Url) -> Vec<RustDiagnostic>;

    // ── 跨语言查询（供 crosslang::coordinator 调用）──

    /// 解析 struct 的成员（字段/方法）的类型信息
    ///
    /// 用于 .rml 绑定路径 `{field}` → .rml.rs 字段类型推导
    fn resolve_member(
        &self,
        rml_rs_uri: &Url,
        struct_name: &str,
        member: &str,
    ) -> Option<SymbolInfo>;

    /// 全 workspace 搜索 struct 定义
    ///
    /// 用于 .rml `<MyComponent>` → .rml.rs `#[component] struct MyComponent` 校验
    fn find_struct(&self, struct_name: &str) -> Option<SymbolLocation>;

    /// 获取 struct 声明的 slot 列表
    ///
    /// 用于 .rml `<template slot="x">` → `#[component(slots=["x"])]` 校验
    fn struct_slots(&self, rml_rs_uri: &Url, struct_name: &str) -> Vec<String>;

    /// 获取 #[command] 标注方法的签名
    ///
    /// 用于 .rml `onclick={fn}` → .rml.rs 方法参数类型 hover/补全
    fn command_signature(
        &self,
        rml_rs_uri: &Url,
        struct_name: &str,
        method: &str,
    ) -> Option<SymbolInfo>;

    /// RA 后端是否已就绪（workspace 加载完成）
    ///
    /// 首次加载耗时较长（30s+），加载完成前返回 false，查询降级返回空
    fn is_ready(&self) -> bool;
}
