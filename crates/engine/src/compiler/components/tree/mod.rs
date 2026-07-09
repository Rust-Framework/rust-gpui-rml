//! Tree 组件 codegen 模块入口。
//!
//! Tree 构造器使用 `as_ref()` 而非 `&` 引用（与其他 Stateful 组件不同），
//! 因此从 `StatefulComponentTranslator` 独立出来。

pub mod gen;
pub mod setters;

pub use gen::gen_tree;
pub use setters::event_setter;
