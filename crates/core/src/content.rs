//! content 绑定统一转换层
//!
//! `content={expr}` 绑定时，通过 `IntoContent` trait 将表达式值统一转为 `AnyElement`。
//! 支持四类输入：
//! - `IntoElement` 类型（String/SharedString/AnyElement/Entity<T: Render> 等）→ 直接转换
//! - `ToString` 类型（i32/f64/bool/usize 等）→ 格式化为 SharedString
//! - `IVisual` trait 对象（&dyn IVisual/Box<dyn IVisual>/Arc<dyn IVisual>）→ 调用 render()
//! - `&T` 引用（&String/&SharedString/&i32 等）→ Clone 后委托值类型 impl
//!
//! 分派由 Rust 编译器在编译期完成（trait impl 选择），无运行时开销。
//!
//! **设计说明**：不使用 `impl<T: IntoElement> IntoContent for T` blanket impl，
//! 因为 Rust 一致性检查器会拒绝与 `impl IntoContent for i32` 等原生类型 impl 共存
//! （上游 crate 未来可能为 i32 实现 IntoElement）。
//! 因此对 IntoElement 类型逐一显式实现，覆盖常用场景。
//! 用户自定义类型可自行 impl IntoContent 或通过 `.into_any_element()` 转换。
//!
//! **引用支持**：`impl<T: IntoContent + Clone> IntoContent for &T` blanket impl
//! 解决 `render(&self)` 中无法 move 非 Copy 字段的问题。codegen 对简单字段访问
//! 自动添加 `&` 前缀（如 `self.message` → `&self.message`），由 blanket impl 接管。

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

/// `&str`（任意生命周期）→ SharedString
///
/// 不使用 `into_any_element()` 因为非 `'static` 的 `&str` 不实现 `IntoElement`。
/// 统一通过 `SharedString::from` 转换，对 `'static` 也正确。
impl<'a> IntoContent for &'a str {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        SharedString::from(self).into_any_element()
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

impl<T: Render> IntoContent for Option<gpui::Entity<T>> {
    fn into_content(self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        match self {
            Some(entity) => entity.into_any_element(),
            None => gpui::div().into_any_element(),
        }
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

// ── 引用类型：通过 Clone + IntoContent 复用值类型实现 ──
//
// 解决 `render(&self)` 中无法 move 非 Copy 字段的问题：
// codegen 对简单字段访问（如 `self.message`）自动添加 `&` 前缀，
// 生成 `into_content(&self.message, ...)`，由此 blanket impl 接管。
//
// 覆盖：`&String`、`&SharedString`、`&i32`、`&bool`、`&Entity<T>`、`&&'static str` 等
// 不覆盖：`&str`（str 是 !Sized，由 `impl<'a> IntoContent for &'a str` 专用处理）
//         `&dyn IVisual`（dyn IVisual 是 !Sized，由专用 impl 处理）
//
// 一致性安全：T 要求 Sized + IntoContent + Clone，而 str/dyn IVisual 均 !Sized，
// 不满足 Sized 约束，故不与上述专用 impl 冲突。
impl<T: IntoContent + Clone> IntoContent for &T {
    fn into_content(self, window: &mut Window, cx: &mut App) -> AnyElement {
        T::into_content(T::clone(self), window, cx)
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

    // ── 引用类型测试（blanket impl 覆盖）──

    #[test]
    fn into_content_trait_is_implemented_for_string_ref() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<&String>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_shared_string_ref() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<&SharedString>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_numeric_ref() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<&i32>();
        assert_into_content::<&bool>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_str_ref() {
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<&str>();
    }

    #[test]
    fn into_content_trait_is_implemented_for_static_str_double_ref() {
        // &&'static str 由 blanket impl 覆盖（&'static str: IntoContent + Clone + Sized）
        fn assert_into_content<T: IntoContent>() {}
        assert_into_content::<&&'static str>();
    }
}
