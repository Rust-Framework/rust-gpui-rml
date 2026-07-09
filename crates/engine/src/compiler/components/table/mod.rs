//! Table codegen 模块入口。
//!
//! ## 模块结构
//!
//! - `gen.rs`：Table 容器的构造 + 属性处理 + 子节点分发
//! - `column.rs`：Column 子标签直接构造表达式生成
//! - `template.rs`：`<template slot="header/cell/footer">` 插槽模板 codegen
//! - `setters.rs`：Table/Column 专用属性 → builder 方法映射

pub mod column;
pub mod gen;
pub mod setters;
pub mod template;

pub use gen::gen_table;
