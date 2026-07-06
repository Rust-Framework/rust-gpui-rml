//! TabBar codegen 模块入口。
//!
//! ## 模块结构
//!
//! - `gen.rs`：TabBar 容器的构造 + 属性处理 + 子节点 `.child(TabItem::new()...)` 注入
//! - `tab.rs`：单个 `<Tab>` 子节点构造（WPF TabItem 模式：title + body 闭包）
//! - `setters.rs`：TabBar/Tab 专用属性 → builder 方法映射
//!
//! ## 设计
//!
//! `<tab>` 标签底层统一编译为 `rml_ui::TabItem`（WPF TabItem 模式）。
//! `<tab-item>` 标签已弃用并移除——RML 架构保持干净整洁，统一用 `<tab>` 即可。
//! TabBar 的所有子节点都是 `<tab>`，由 `tab::gen_tab_child` 生成 `TabItem::new()...` 表达式。

pub mod gen;
pub mod setters;
pub mod tab;

pub use gen::gen_tab_bar;
