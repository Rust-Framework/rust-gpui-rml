//! Sheet 容器 codegen —— 子模块聚合
//!
//! 将 `<Sheet>` 转译为 `rml_ui::Sheet::new(_window, cx).title(...).size(...).child(...)`。
//!
//! ## 构造器特殊性
//!
//! Sheet 构造器签名为 `new(_: &mut Window, cx: &mut App)`，需要 render 上下文的
//! `_window` 和 `cx` 变量。codegen 直接生成 `rml_ui::Sheet::new(_window, cx)`，
//! 无需自动分配 ElementId。
//!
//! ## 子模块
//!
//! - `gen.rs`：构造代码生成（`gen_sheet`）
//! - `setters.rs`：Sheet 专用属性 setter（`static_setter` / `event_setter`）

mod gen;
mod setters;

pub use gen::gen_sheet;
