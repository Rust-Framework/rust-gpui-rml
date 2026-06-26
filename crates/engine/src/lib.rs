//! RML 解析引擎与编译器
//!
//! 将 `.rml` 模板编译为原生 GPUI 渲染代码。
//! 包含：词法分析、AST 构建、语义验证、代码生成、构建集成。

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
// pub extern crate 让别名对下游可见（rml::rml_core::... 可用）
pub extern crate rust_rml_core as rml_core;
pub extern crate rust_rml_macros as rml_macros;

pub mod build;
pub mod compiler;
pub mod parser;
pub mod runtime;
pub mod tags;

pub mod prelude;

/// 构建入口：在用户 `build.rs` 中调用，扫描 `.rml`、调用编译器、输出到 `OUT_DIR`。
///
/// ```rust
/// // build.rs
/// fn main() {
///     rml::build()
///         .scan_dir("src")
///         .output_dir(std::env::var("OUT_DIR").unwrap())
///         .build()
///         .expect("RML build failed");
/// }
/// ```
pub use build::build;
