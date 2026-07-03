//! Avatar / AvatarGroup 组件 codegen 模块入口。
//!
//! 构造器由 `component::gen_component` 的 `StatelessNoId` 分支统一处理，
//! 本模块仅提供专用 setter（src/name/placeholder/limit/ellipsis）。

pub mod setters;

pub use setters::{bind_setter, static_setter};
