//! `ElementRef<T>` —— 元素引用包装
//!
//! 通过 `.rml` 中的 `ref="name"` 属性关联到 ViewModel 中 `#[element]` 标记的字段，
//! 实现命令式访问（focus、scroll、measure 等）。
//! 详见文档 §4.3 元素引用。
//!
//! ## 设计
//!
//! `ElementRef<T>` 包装一个 GPUI `Entity<T>` 句柄：
//! - `T` 是元素的状态类型（如 `InputState`、`ButtonState` 等）
//! - 初始为 `None`，由 RML Runtime 在首次渲染后填充
//! - 在 `#[on_loaded]` 之前为空，调用方法会安全地返回 `None`
//!
//! ## ref 属性与 codegen
//!
//! `.rml` 中 `ref="name"` 会让 codegen 为元素生成稳定 ID `("rml_ref", "name")`，
//! 便于 Runtime 通过 `window.elements` 等 GPUI API 定位元素。

use gpui::{Entity, WeakEntity};

/// 元素引用，包装 GPUI `Entity<T>` 句柄。
///
/// ```rust,ignore
/// #[derive(IModel)]
/// #[component]
/// pub struct MyView {
///     pub user_name: SharedString,
///     #[element]
///     pub username_input: ElementRef<rml_ui::InputState>,
/// }
/// ```
///
/// ```html
/// <input ref="username_input" model={user_name} />
/// ```
pub struct ElementRef<T> {
    handle: Option<Entity<T>>,
}

impl<T> ElementRef<T> {
    /// 创建空的引用（编译期由 `#[element]` 宏初始化）
    pub fn new() -> Self {
        Self { handle: None }
    }

    /// 设置底层 Entity 句柄（由运行时在首次渲染后注入）
    pub fn set(&mut self, handle: Entity<T>) {
        self.handle = Some(handle);
    }

    /// 清空引用（视图卸载时调用）
    pub fn clear(&mut self) {
        self.handle = None;
    }

    /// 获取底层 Entity 句柄
    pub fn get(&self) -> Option<&Entity<T>> {
        self.handle.as_ref()
    }

    /// 引用是否已设置
    pub fn is_set(&self) -> bool {
        self.handle.is_some()
    }

    /// 获取弱引用
    pub fn downgrade(&self) -> Option<WeakEntity<T>>
    where
        T: 'static,
    {
        self.handle.as_ref().map(|h| h.downgrade())
    }

    /// 不可变访问底层实体。
    ///
    /// 若引用未设置或读取失败，返回 `None`。
    /// 在 `#[on_loaded]` 之前调用会安全返回 `None`。
    pub fn with<R>(&self, cx: &gpui::App, f: impl FnOnce(&T) -> R) -> Option<R>
    where
        T: 'static,
    {
        let handle = self.handle.as_ref()?;
        Some(f(handle.read(cx)))
    }

    /// 可变访问底层实体。
    ///
    /// 若引用未设置或访问失败，返回 `None`。
    pub fn with_mut<R>(&self, cx: &mut gpui::App, f: impl FnOnce(&mut T) -> R) -> Option<R>
    where
        T: 'static,
    {
        let handle = self.handle.as_ref()?;
        let mut ret: Option<R> = None;
        handle.update(cx, |t, _cx| {
            ret = Some(f(t));
        });
        ret
    }
}

impl<T> Default for ElementRef<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ElementRef<T> {
    fn clone(&self) -> Self {
        Self {
            handle: self.handle.clone(),
        }
    }
}

impl<T> std::fmt::Debug for ElementRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElementRef")
            .field("handle", &self.handle.is_some())
            .finish()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_unset() {
        let r: ElementRef<i32> = ElementRef::new();
        assert!(!r.is_set());
        assert!(r.get().is_none());
    }

    #[test]
    fn default_is_unset() {
        let r: ElementRef<i32> = ElementRef::default();
        assert!(!r.is_set());
    }

    #[test]
    fn clear_resets_handle() {
        let mut r: ElementRef<i32> = ElementRef::new();
        r.clear();
        assert!(!r.is_set());
    }

    #[test]
    fn debug_shows_set_status() {
        let r: ElementRef<i32> = ElementRef::new();
        let s = format!("{:?}", r);
        assert!(s.contains("false"));
    }
}
