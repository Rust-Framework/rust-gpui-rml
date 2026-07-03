//! TabBar codegen 模块入口。
//!
//! ## 模块结构
//!
//! - `gen.rs`：TabBar 容器的构造 + 属性处理 + 子节点 `.child(Tab::new()...)` 注入
//! - `tab.rs`：单个 `<Tab>` 子节点构造（直接表达式，非闭包）
//! - `setters.rs`：TabBar/Tab 专用属性 → builder 方法映射

pub mod gen;
pub mod setters;
pub mod tab;

pub use gen::gen_tab_bar;
