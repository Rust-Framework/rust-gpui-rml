//! RML 源映射
//!
//! 桥接 `.rml` 声明层与生成的 `.rml.rs` 代码层：
//! - 正向：用户在 `.rml` 中设置断点 → 翻译为 `.rml.rs` 位置供引擎使用
//! - 反向：引擎在 `.rml.rs` 停止 → 翻译回 `.rml` 位置供 UI 高亮
//!
//! ## 位置粒度
//!
//! trait 方法签名携带 `column: u32`（1-based），与 DAP `StackFrame.column` 一致，
//! 允许同行多插值/多元素精确反查。engine codegen 产出的 `.rml.map` 应保留列号；
//! `FilePairMapper` 等 MVP 实现可将 column 原样传递（不翻译）。
//!
//! ## MVP 约束
//!
//! `FilePairMapper` 仅做文件级配对（`.rml` ↔ `.rml.rs`），行号/列号原样传递。
//! 精确行级映射由 `LineAccurateMapper` 加载 engine codegen 输出的 `.rml.map` 实现，
//! 上层代码无需改动（依赖 `SourceMapper` trait）。

use std::collections::HashMap;

use lsp_types::Url;

/// 源映射抽象
///
/// 所有方法签名中的 `line` 与 `column` 均为 1-based（DAP 惯例）。
/// 实现方负责与具体后端的基偏移转换。
pub trait SourceMapper: Send + Sync {
    /// 正向映射：`.rml` 断点位置 → `.rml.rs` 位置（供引擎）
    ///
    /// 返回 `(rust_uri, line, column)`；column 1-based，无列号信息时返回 0 或 1。
    fn rml_to_rust(&self, rml_uri: &Url, line: u32, column: u32) -> Option<(Url, u32, u32)>;

    /// 反向映射：`.rml.rs` 位置 → `.rml` 位置（供 UI）
    ///
    /// 返回 `(rml_uri, line, column)`；column 1-based，无列号信息时返回 0 或 1。
    fn rust_to_rml(&self, rust_uri: &Url, line: u32, column: u32) -> Option<(Url, u32, u32)>;
}

/// 文件级配对映射器（MVP）
///
/// 注册 `.rml` ↔ `.rml.rs` 文件对后，按文件配对返回（行号原样传递，不翻译）。
/// 适用于：在 `.rml.rs` code-behind 中打断点（行号精确），或行号近似可接受的场景。
#[derive(Default)]
pub struct FilePairMapper {
    rml_to_rust: HashMap<Url, Url>,
    rust_to_rml: HashMap<Url, Url>,
}

impl FilePairMapper {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一对 `.rml` ↔ `.rml.rs` 文件
    pub fn register_pair(&mut self, rml_uri: Url, rust_uri: Url) {
        self.rust_to_rml.insert(rust_uri.clone(), rml_uri.clone());
        self.rml_to_rust.insert(rml_uri, rust_uri);
    }

    /// 按文件名约定自动推断配对：`foo.rml` ↔ `foo.rml.rs`
    ///
    /// 仅当配对文件确实存在（已注册）时返回映射，否则 None。
    pub fn lookup_pair(&self, rml_uri: &Url) -> Option<&Url> {
        self.rml_to_rust.get(rml_uri)
    }
}

impl SourceMapper for FilePairMapper {
    fn rml_to_rust(&self, rml_uri: &Url, line: u32, column: u32) -> Option<(Url, u32, u32)> {
        let rust_uri = self.rml_to_rust.get(rml_uri)?;
        Some((rust_uri.clone(), line, column))
    }

    fn rust_to_rml(&self, rust_uri: &Url, line: u32, column: u32) -> Option<(Url, u32, u32)> {
        let rml_uri = self.rust_to_rml.get(rust_uri)?;
        Some((rml_uri.clone(), line, column))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_mapping_returns_paired_file() {
        let mut mapper = FilePairMapper::new();
        let rml = Url::parse("file:///foo/counter.rml").unwrap();
        let rust = Url::parse("file:///foo/counter.rml.rs").unwrap();
        mapper.register_pair(rml.clone(), rust.clone());
        let (mapped_uri, line, column) = mapper.rml_to_rust(&rml, 42, 5).unwrap();
        assert_eq!(mapped_uri, rust);
        assert_eq!(line, 42);
        assert_eq!(column, 5);
    }

    #[test]
    fn reverse_mapping_returns_paired_file() {
        let mut mapper = FilePairMapper::new();
        let rml = Url::parse("file:///foo/counter.rml").unwrap();
        let rust = Url::parse("file:///foo/counter.rml.rs").unwrap();
        mapper.register_pair(rml.clone(), rust.clone());
        let (mapped_uri, line, column) = mapper.rust_to_rml(&rust, 10, 3).unwrap();
        assert_eq!(mapped_uri, rml);
        assert_eq!(line, 10);
        assert_eq!(column, 3);
    }

    #[test]
    fn unregistered_file_returns_none() {
        let mapper = FilePairMapper::new();
        let unknown = Url::parse("file:///foo/unknown.rml").unwrap();
        assert!(mapper.rml_to_rust(&unknown, 1, 1).is_none());
        assert!(mapper.rust_to_rml(&unknown, 1, 1).is_none());
    }
}
