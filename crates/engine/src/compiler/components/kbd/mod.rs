//! Kbd 组件代码生成 —— 子模块聚合
//!
//! Kbd 构造器：`Kbd::new(Keystroke)`，接受 `Keystroke` 类型。
//! Kbd 是 RenderOnce，无 ElementId。
//!
//! ## 子模块
//!
//! - `gen.rs`：构造代码生成（`gen_kbd`）
//! - `setters.rs`：Kbd 专用属性 setter（`kbd_static_setter`）

mod gen;
mod setters;

pub use gen::gen_kbd;
