//! content 绑定统一转换层
//!
//! `content={expr}` 绑定时，通过 `IntoContent` trait 将表达式值统一转为 `AnyElement`。
//! 支持三类输入：
//! - `IntoElement` 类型（String/SharedString/AnyElement/Entity<T: Render> 等）→ 直接转换
//! - `ToString` 类型（i32/f64/bool/usize 等）→ 格式化为 SharedString
//! - `IVisual` trait 对象（&dyn IVisual/Box<dyn IVisual>/Arc<dyn IVisual>）→ 调用 render()
//!
//! 分派由 Rust 编译器在编译期完成（trait impl 选择），无运行时开销。
//!
//! **设计说明**：不使用 `impl<T: IntoElement> IntoContent for T` blanket impl，
//! 因为 Rust 一致性检查器会拒绝与 `impl IntoContent for i32` 等原生类型 impl 共存
//! （上游 crate 未来可能为 i32 实现 IntoElement）。
//! 因此对 IntoElement 类型逐一显式实现，覆盖常用场景。
//! 用户自定义类型可自行 impl IntoContent 或通过 `.into_any_element()` 转换。

use std::sync::Arc;

use gpui::{AnyElement, App, IntoElement, Render, SharedString, Window};

use crate::contribution::IVisual;

/// 将值转换为 `AnyElement` 用于 content 绑定
///
/// 编译器根据表达式类型自动选择 impl：
/// - `SharedString`/`String`/`AnyElement`/`Entity<T: Render>` 等 → 直接 `into_any_element()`
/// - `i32`/`f64`/`bool` 等 → 格式化为 SharedString
/// - `&dyn IVisual`/`Box<dyn IVisual>` → 调用 `IVisual::render`
pub trait IntoContent {
    fn into_content(self, window: &mut Window, cx: &mut App) -> AnyElement;
}

/// 辅助函数：将值转换为 `AnyElement`
///
/// codegen 生成 `.child(rml_core::content::into_content({expr}, _window, cx))`
pub fn into_content<T: IntoContent>(value: T, window: &mut Window, cx: &mut App) -> AnyElement {
    value.into_content(window, cx)
}

// ── IntoElement 类型：直接转换为 AnyElement ──
// 逐个显式实现（不使用 blanket impl 以避免一致性冲突）

impl IntoContent for SharedString {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self.into_any_element()
    }
}

impl IntoContent for String {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self.into_any_element()
    }
}

impl IntoContent for &'static str {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self.into_any_element()
    }
}

impl IntoContent for AnyElement {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self
    }
}

impl<T: Render> IntoContent for gpui::Entity<T> {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        self.into_any_element()
    }
}

// ── ToString 类型：格式化为 SharedString ──

macro_rules! impl_into_content_to_string {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntoContent for $ty {
                fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
                    SharedString::from(self.to_string()).into_any_element()
                }
            }
        )*
    };
}

impl_into_content_to_string!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64, bool);

// ── IVisual trait 对象：调用 render() ──

impl IntoContent for &dyn IVisual {
    fn into_content(self, window: &mut Window, cx: &mut App) -> AnyElement {
        IVisual::render(self, window, cx)
    }
}

impl IntoContent for Box<dyn IVisual> {
    fn into_content(self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.render(window, cx)
    }
}

impl IntoContent for Arc<dyn IVisual> {
    fn into_content(self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.render(window, cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_content_trait_is_implemented_for_shared_string() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<SharedString>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_any_element() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<AnyElement>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_string() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<String>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_numeric_types() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<i32>();
        assert_into_content::<f64>();
        assert_into_content::<bool>();
        assert_into_content::<usize>();
    }
}
