//! RML 源映射
//!
//! 桥接 `.rml` 声明层与生成的 `.rml.rs` 代码层，对标 lsp crate 的 `crosslang` 职责。
//! 引擎在 `.rml.rs`（生成代码）上工作，用户在 `.rml` 上交互，本模块负责双向翻译。

pub mod mapper;

pub use mapper::{FilePairMapper, SourceMapper};
