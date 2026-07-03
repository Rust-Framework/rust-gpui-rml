//! Accordion codegen 模块入口。
//!
//! ## 模块结构
//!
//! - `gen.rs`：Accordion 容器的构造 + 属性处理 + 子节点分发
//! - `item.rs`：AccordionItem 闭包式 builder 生成
//! - `setters.rs`：Accordion/AccordionItem 专用属性 → builder 方法映射

pub mod gen;
pub mod item;
pub mod setters;

pub use gen::gen_accordion;
