//! RML 框架核心基础层
//!
//! 定义基础 trait（I 前缀）、事件类型、元素引用与绑定路径。
//! 贡献点契约在 `contribution` 模块；运行时注册由 `rml_app::ContributionExt` 提供。

#![deny(unsafe_code)]

/// 重导出 `ctor` crate,供 build.rs 生成的资源自动注册代码使用
/// （用户 crate 无需显式依赖 `ctor`）。
pub use ctor;

/// 重导出 `flume` crate,供 `#[contributehost]` 宏生成的 channel 代码使用
/// （用户 crate 无需显式依赖 `flume`）。
pub use flume;

/// 重导出 `url` crate,供工作台 Uri 类型使用
/// （用户 crate 无需显式依赖 `url`）。
pub use url;

pub mod ability;
pub mod assets;
pub mod binding;
pub mod context;
pub mod i18n;
pub mod theme;
pub mod command;
pub mod component;
pub mod computed_cache;
pub mod contribution;
pub mod converter;
pub mod workbench;
pub mod element_ref;
pub mod event;
pub mod events;
pub mod lifecycle;
pub mod model;
pub mod observable;
pub mod slot;
pub mod two_way_binding;
pub mod validate;
pub mod value;
pub mod view_model;
pub mod window;

pub mod prelude;

pub use context::{ensure_service_collection, IAppContext, ServiceCollection};

/// 重导出 GPUI 基础类型,供框架各层统一使用
pub use gpui::{App, Context, Entity, IntoElement, Keystroke, Modifiers, Pixels, Point, Rgba, Render, SharedString, WeakEntity, Window};

pub use observable::ObservableVec;
