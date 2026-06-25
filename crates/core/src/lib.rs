//! RML 框架核心基础层
//!
//! 定义所有基础 trait（I 前缀）、事件类型、元素引用与绑定路径。
//! 本 crate 不依赖 GPUI 的渲染系统，仅重导出 `SharedString` 等基础类型。

#![forbid(unsafe_code)]

pub mod binding;
pub mod command;
pub mod component;
pub mod converter;
pub mod element_ref;
pub mod event;
pub mod events;
pub mod lifecycle;
pub mod model;
pub mod two_way_binding;
pub mod view;
pub mod view_model;

pub mod prelude;

/// 重导出 GPUI 基础类型，供框架各层统一使用
pub use gpui::{App, Context, Entity, IntoElement, Keystroke, Modifiers, Pixels, Point, Render, SharedString, WeakEntity, Window};
