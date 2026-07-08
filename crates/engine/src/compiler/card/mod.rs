//! Card 组件 codegen 模块入口。
//!
//! 构造器由 `StatelessComponentTranslator` 统一处理
//! （`rml_ui::Card::new(("rml_el", N))`），本模块仅提供专用 setter
//! （title/extra/cover/footer/bordered/borderless/hoverable）。

pub mod setters;

pub use setters::{bind_setter, static_setter};
