//! `ElementRef<T>` —— 元素引用包装
//!
//! 通过 `.rml` 中的 `ref="name"` 指令关联到 ViewModel 中 `#[element]` 标记的字段，
//! 实现命令式访问（focus、scroll、measure 等）。
//! 详见文档 §4.3 元素引用。

use gpui::{Entity, WeakEntity};

/// 元素引用，包装 GPUI `Entity<T>` 句柄。
///
/// ```rust
/// #[derive(IModel)]
/// #[view]
/// pub struct MyView {
///     pub user_name: SharedString,
///     #[element]
///     pub username_input: ElementRef<MyInput>,
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

    /// 获取底层 Entity 句柄
    pub fn get(&self) -> Option<&Entity<T>> {
        self.handle.as_ref()
    }

    /// 获取弱引用
    pub fn downgrade(&self) -> Option<WeakEntity<T>>
    where
        T: 'static,
    {
        self.handle.as_ref().map(|h| h.downgrade())
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
