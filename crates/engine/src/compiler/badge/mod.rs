//! Badge 组件 codegen 模块入口。
//!
//! 构造器由 `component::gen_component` 的 `StatelessNoId` 分支统一处理，
//! 本模块仅提供专用 setter（count/max/dot/icon）。
//!
//! Badge 三种 variant：
//! - Number（默认）：`count="5"` 设置计数，`count=0` 时隐藏；`max="99"` 限制显示上限（超出显示 `99+`）
//! - Dot：`dot=""` 切换为小红点
//! - Icon：`icon="Bell"` 切换为图标徽标（右下角）

pub mod setters;

pub use setters::{bind_setter, static_setter};
