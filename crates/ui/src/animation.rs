//! 入场动画预设 —— 基于 GPUI `AnimationExt::with_animation`
//!
//! 为 RML `animate` 指令提供预设动画函数。每个函数接收元素、ID、时长，
//! 返回 `AnimationElement<E>`。
//!
//! ## 支持的预设
//!
//! - `fade` — 淡入（opacity 0→1）
//! - `slide-up` — 从下滑入（top 偏移 + opacity）
//! - `slide-down` — 从上滑入（top 偏移 + opacity）
//! - `slide-left` — 从右滑入（left 偏移 + opacity）
//!
//! 时长默认 300ms，可通过 `animate="fade:500"` 指定。
//!
//! GPUI `Styled` 无 `translate_y`/`scale` 变换方法，slide 效果通过 `top()`/`left()`
//! （需 `relative()` 定位）实现，与 gpui-component `Transition` 方案一致。

use std::time::Duration;

use gpui::{Animation, AnimationElement, AnimationExt, IntoElement, Styled, ease_in_out, px};

/// 淡入动画：opacity 0→1
pub fn fade<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<gpui::ElementId>,
    duration_ms: u32,
) -> AnimationElement<E> {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(duration_ms as u64))
            .with_easing(ease_in_out),
        |el, value| el.opacity(value),
    )
}

/// 从下滑入：top(20px→0) + opacity(0→1)
pub fn slide_up<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<gpui::ElementId>,
    duration_ms: u32,
) -> AnimationElement<E> {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(duration_ms as u64))
            .with_easing(ease_in_out),
        |el, value| {
            el.relative()
                .opacity(value)
                .top(px((1.0 - value) * 20.0))
        },
    )
}

/// 从上滑入：top(-20px→0) + opacity(0→1)
pub fn slide_down<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<gpui::ElementId>,
    duration_ms: u32,
) -> AnimationElement<E> {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(duration_ms as u64))
            .with_easing(ease_in_out),
        |el, value| {
            el.relative()
                .opacity(value)
                .top(px((1.0 - value) * -20.0))
        },
    )
}

/// 从右滑入：left(20px→0) + opacity(0→1)
pub fn slide_left<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<gpui::ElementId>,
    duration_ms: u32,
) -> AnimationElement<E> {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(duration_ms as u64))
            .with_easing(ease_in_out),
        |el, value| {
            el.relative()
                .opacity(value)
                .left(px((1.0 - value) * 20.0))
        },
    )
}

/// 根据预设名调用对应动画函数，返回代码字符串供 codegen 使用
///
/// codegen 生成：`rml_ui::animation::fade(element, ("rml_anim", N), 300)`
pub fn preset_fn_name(name: &str) -> Option<&'static str> {
    match name {
        "fade" => Some("fade"),
        "slide-up" => Some("slide_up"),
        "slide-down" => Some("slide_down"),
        "slide-left" => Some("slide_left"),
        _ => None,
    }
}
