//! 原生 TabBar codegen 模块入口（纯 header 标签栏，无 body/无 close）。
//!
//! - `gen.rs`：TabBar 容器构造 + 属性 + 子节点 `.child(TabItem::new()...)` 注入
//! - `setters.rs`：TabBar 专用属性 → builder 方法映射（不含 bordered/on_close*）
//!
//! `<tab>` 子节点 codegen 复用 `tabs::tab::gen_tab_child`（生成 `TabItem::new()...`），
//! 因 TabBar 的 `child()` 接受 `impl Into<TabItem>`（通过 `From<Tab> for TabItem` 兼容 Tab）。

pub mod gen;
pub mod setters;

pub use gen::gen_tab_bar;
