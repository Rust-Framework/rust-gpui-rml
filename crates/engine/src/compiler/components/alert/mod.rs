//! Alert 组件 codegen —— 子模块聚合
//!
//! Alert 构造器：`Alert::new(id, message)` 或 `Alert::info(id, message)` 等关联函数。
//!
//! ## 子模块
//!
//! - `gen.rs`：构造代码生成（`gen_alert`）+ variant/event 辅助函数

mod gen;

pub use gen::gen_alert;
