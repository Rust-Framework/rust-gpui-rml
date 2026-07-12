//! CodeEditor 组件 codegen 模块入口。
//!
//! 构造器使用 `as_ref().expect()` 支持 `Option<Entity<InputState>>` 字段。
//! 布局/视觉样式由 RML 属性与 CSS class 负责，不在 codegen 中硬编码。

pub mod gen;

pub use gen::{gen_code_editor, HANDLED_PROPS};
