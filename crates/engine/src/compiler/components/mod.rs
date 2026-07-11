//! 扩展组件 codegen 实现统一收纳
//!
//! 每个组件独占一个文件或子目录，与 `translator/component/` 下的薄包装 translator
//! 形成对称：本目录提供 codegen 实现（构造器、属性 setter、子节点分发），
//! translator 负责接入 `IRmlTranslator` 接口并委托本目录的生成函数。
//!
//! ## 子模块
//!
//! - `accordion/` / `alert/` / `alert_dialog/` / `avatar/` / `badge/` / `card/` / `code_editor/` /
//!   `description_list/` / `dialog/` / `hover_card/` / `input/` / `kbd/` / `menu/` / `popover/` / `sheet/` / `table/` /
//!   `tabs/` / `tab_bar/` / `tree/`：复杂组件 codegen（含 gen.rs / setters.rs / item.rs 等）
//! - `icon.rs` / `label.rs` / `radio_group.rs` / `separator.rs` / `tag.rs`：单文件组件 codegen

pub mod accordion;
pub mod alert;
pub mod alert_dialog;
pub mod avatar;
pub mod badge;
pub mod card;
pub mod code_editor;
pub mod description_list;
pub mod dialog;
pub mod hover_card;
pub mod icon;
pub mod input;
pub mod kbd;
pub mod label;
pub mod menu;
pub mod notification;
pub mod otp_input;
pub mod popover;
pub mod radio_group;
pub mod separator;
pub mod sheet;
pub mod state_event;
pub mod stepper;
pub mod tab_bar;
pub mod table;
pub mod tabs;
pub mod tag;
pub mod tree;
