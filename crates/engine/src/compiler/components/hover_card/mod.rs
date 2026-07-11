//! HoverCard 容器 codegen —— 子模块聚合
//!
//! 将 `<HoverCard>` 转译为 `rml_ui::HoverCard::new(id).trigger(...).anchor(...).child(...)`。
//!
//! ## 子模块
//!
//! - `gen.rs`：构造代码生成（`gen_hover_card`）
//! - `setters.rs`：HoverCard 专用属性 setter（`static_setter`）

mod gen;
mod setters;

pub use gen::gen_hover_card;
