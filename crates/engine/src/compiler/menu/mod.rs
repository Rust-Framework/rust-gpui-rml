//! RML 菜单 codegen 模块入口。
//!
//! 将声明式 `MenuItem` / `MenuSeparator` 转译为 gpui-component PopupMenu API。
//!
//! ## 模块结构
//!
//! - `dispatcher.rs`：标签识别（`is_menu_container` / `is_menu_tag`）+ 元素 codegen 分发（`gen_menu_element`）
//! - `children.rs`：菜单子节点拆分与触发器子节点生成
//! - `setters.rs`：menu / MenuBar / status-bar 专用 bind setter
//! - `item.rs` / `menu_bar.rs` / `context.rs` / `dropdown.rs` / `app_menu_bar.rs` / `popup.rs` / `hoist.rs`：
//!   各类菜单元素的具体 codegen 实现

mod app_menu_bar;
mod children;
mod context;
mod dispatcher;
mod dropdown;
mod hoist;
mod item;
mod menu_bar;
mod popup;
mod setters;

pub(crate) use children::{gen_trigger_children, partition_menu_children};
pub use dispatcher::{gen_menu_element, is_menu_container, is_menu_tag};
pub use item::is_menu_item_tag;
pub use setters::bind_setter;
