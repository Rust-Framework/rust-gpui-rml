//! RML 框架核心基础层
//!
//! 定义基础 trait（I 前缀）、事件类型、元素引用与绑定路径。
//! 贡献点契约在 `contribution` 模块；运行时注册由 `rml_app::ContributionExt` 提供。

#![deny(unsafe_code)]

/// 重导出 `ctor` crate,供 build.rs 生成的资源自动注册代码使用
/// （用户 crate 无需显式依赖 `ctor`）。
pub use ctor;

pub mod assets;
pub mod binding;
pub mod i18n;
pub mod theme;
pub mod command;
pub mod component;
pub mod computed_cache;
pub mod contribution;
pub mod contribution_cache;
pub mod converter;
pub mod element_ref;
pub mod event;
pub mod events;
pub mod lifecycle;
pub mod model;
pub mod slot;
pub mod two_way_binding;
pub mod validate;
pub mod view_model;
pub mod window;

pub mod prelude;

/// 重导出 GPUI 基础类型,供框架各层统一使用
pub use gpui::{App, Context, Entity, IntoElement, Keystroke, Modifiers, Pixels, Point, Rgba, Render, SharedString, WeakEntity, Window};
