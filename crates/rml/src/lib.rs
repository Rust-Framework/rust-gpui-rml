//! RML 解析引擎与编译器
//!
//! 将 `.rml` 模板编译为原生 GPUI 渲染代码。
//! 包含：词法分析、AST 构建、语义验证、代码生成、构建集成。

#![forbid(unsafe_code)]

pub mod build;
pub mod compiler;
pub mod parser;
pub mod runtime;
pub mod tags;

pub mod prelude;

pub use rml_core;
pub use rml_macros::*;
