//! `Styled` 扩展 trait —— 为任意 `Styled` 元素提供直接的 `Overflow` 设置。
//!
//! GPUI 的 `Styled` trait 仅提供 `overflow_hidden()` 等便捷方法，
//! `StatefulInteractiveElement` 提供 `overflow_scroll()` 但要求 `.id()`。
//! 本 trait 允许在任意 `Styled` 元素上直接设置 `Overflow` 枚举值，
//! 适用于 CSS `overflow: scroll/auto` 的通用映射（无需元素有 id）。

use gpui::{Overflow, Styled};

/// 为 `Styled` 元素提供直接的 overflow 设置能力。
///
/// 方法名 `overflow` / `overflow_x` / `overflow_y` 不与 `Styled` 的
/// `overflow_hidden()` 系列或 `StatefulInteractiveElement` 的
/// `overflow_scroll()` 系列冲突。
pub trait OverflowStyle: Styled {
    /// 设置双轴 overflow。
    fn overflow(mut self, value: Overflow) -> Self {
        self.style().overflow.x = Some(value);
        self.style().overflow.y = Some(value);
        self
    }

    /// 设置 x 轴 overflow。
    fn overflow_x(mut self, value: Overflow) -> Self {
        self.style().overflow.x = Some(value);
        self
    }

    /// 设置 y 轴 overflow。
    fn overflow_y(mut self, value: Overflow) -> Self {
        self.style().overflow.y = Some(value);
        self
    }
}

impl<T: Styled> OverflowStyle for T {}
