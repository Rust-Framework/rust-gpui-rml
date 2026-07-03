//! 跨语言协调层：桥接 .rml 语义与 .rml.rs 符号
//!
//! - `resolver`：绑定表达式解析器（纯函数，无 IO）
//! - `coordinator`：协调 .rml 绑定路径 ↔ .rml.rs struct/field 符号

pub mod coordinator;
pub mod resolver;

pub use coordinator::{find_component, goto_def_for_binding, resolve_binding};
pub use resolver::{parse_binding_path, BindingPath};
