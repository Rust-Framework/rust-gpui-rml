//! RML 菜单 codegen 模块入口。
//!
//! 将声明式 `MenuItem` / `MenuSeparator` 转译为 gpui-component PopupMenu API。
//!
//! ## 模块结构
//!
//! - `children.rs`：菜单子节点拆分与触发器子节点生成
//! - `setters.rs`：menu / MenuBar / status-bar 专用 bind setter
//! - `item.rs` / `menu_bar.rs` / `context.rs` / `dropdown.rs` / `app_menu_bar.rs` / `popup.rs` / `hoist.rs`：
//!   各类菜单元素的具体 codegen 实现
//!
//! 标签分类（`is_menu_container`）已移至 `tags.rs`。
//! 各菜单容器的 translator 分发已移至 `translator/menu/` 下的独立 translator 文件。

mod app_menu_bar;
mod children;
mod context;
mod dropdown;
mod hoist;
mod item;
mod menu_bar;
mod popup;
mod setters;

pub(crate) use children::{gen_trigger_children, partition_menu_children};
pub use app_menu_bar::gen_app_menu_bar;
pub use context::gen_context_menu;
pub use dropdown::gen_dropdown_menu;
pub use item::is_menu_item_tag;
pub use menu_bar::gen_menu_bar;
pub use setters::bind_setter;
