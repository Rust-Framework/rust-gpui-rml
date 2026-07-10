//! Stepper codegen 模块入口（步骤指示器）。
//!
//! - `gen.rs`：Stepper 容器构造 + 属性 + 子节点 `.item(StepperItem::new()...)` 注入
//! - `setters.rs`：Stepper 专用属性 → builder 方法映射

pub mod gen;
pub mod setters;

pub use gen::gen_stepper;
