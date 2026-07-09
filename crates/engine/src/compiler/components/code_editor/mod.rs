//! CodeEditor 组件 codegen 模块入口。
//!
//! 构造器使用 `as_ref().expect()` 支持 `Option<Entity<InputState>>` 字段，
//! 并自动应用 mono 字体 + size_full 样式（类似 Tree 的独立 codegen 分支）。

pub mod gen;

pub use gen::gen_code_editor;
