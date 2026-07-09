//! Popover 容器 codegen —— 子模块聚合
//!
//! 将 `<Popover>` 转译为 `rml_ui::Popover::new(id).trigger(...).anchor(...).child(...)`。
//!
//! ## 子模块
//!
//! - `gen.rs`：构造代码生成（`gen_popover`）
//! - `setters.rs`：Popover 专用属性 setter（`static_setter` / `bind_setter`）

mod gen;
mod setters;

pub use gen::gen_popover;
