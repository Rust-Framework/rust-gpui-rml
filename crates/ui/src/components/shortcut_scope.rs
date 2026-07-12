//! ShortcutScope —— 作用域级（非焦点宿主）键盘快捷键
//!
//! ## 推荐 RML 写法
//!
//! ```rml
//! <ShortcutScope>
//!   <Shortcut key="Ctrl+S" on-press={on_save} />
//!   <Shortcut key="Ctrl+O" on-press={on_open} />
//!   <div>...</div>
//! </ShortcutScope>
//! ```
//!
//! `<Shortcut>` 为声明式元数据子节点，不渲染；编译器将其转为 `.shortcut(...)` 调用。
//! 作用域容器通过 `on_key_down` 监听子树键盘事件冒泡，无需 Input/CodeEditor 等焦点宿主。

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, KeyDownEvent, Keystroke, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
};
use gpui_component::StyledExt as _;

use super::key_binding::normalize_key_source;

struct ShortcutEntry {
    target: Option<Keystroke>,
    when: bool,
    on_press: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
}

/// 作用域级键盘快捷键容器
///
/// 包裹应用或面板内容，在子树内通过事件冒泡监听 `keydown`，匹配已注册的快捷键后触发回调。
/// 与焦点宿主内的 `<KeyBinding>` 互补：用于 Save/Open 等全局（相对作用域）快捷键。
#[derive(IntoElement)]
pub struct ShortcutScope {
    shortcuts: Vec<ShortcutEntry>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Default for ShortcutScope {
    fn default() -> Self {
        Self {
            shortcuts: Vec::new(),
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl ShortcutScope {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一条快捷键（由 RML `<Shortcut>` 子节点 codegen 调用）
    pub fn shortcut(
        mut self,
        key: impl Into<SharedString>,
        when: bool,
        on_press: impl Fn(&mut Window, &mut App) + 'static,
    ) -> Self {
        let normalized = normalize_key_source(key.into().as_ref());
        self.shortcuts.push(ShortcutEntry {
            target: Keystroke::parse(&normalized).ok(),
            when,
            on_press: Some(Box::new(on_press)),
        });
        self
    }
}

impl Styled for ShortcutScope {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for ShortcutScope {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for ShortcutScope {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let shortcuts = self.shortcuts;
        let mut container = div()
            .w_full()
            .h_full()
            .on_key_down(move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                for entry in &shortcuts {
                    if !entry.when {
                        continue;
                    }
                    let Some(target) = entry.target.as_ref() else {
                        continue;
                    };
                    if event.keystroke.key == target.key
                        && event.keystroke.modifiers == target.modifiers
                    {
                        if let Some(handler) = &entry.on_press {
                            handler(window, cx);
                            cx.stop_propagation();
                            return;
                        }
                    }
                }
            })
            .refine_style(&self.style);

        for child in self.children {
            container = container.child(child);
        }

        container
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::key_binding::normalize_key_source;

    #[test]
    fn shortcut_scope_reuses_normalize_key_source() {
        assert_eq!(normalize_key_source("Ctrl+S"), "ctrl-s");
        let normalized = normalize_key_source("Ctrl+S");
        let ks = Keystroke::parse(&normalized).expect("valid keystroke");
        assert!(ks.modifiers.control);
        assert_eq!(ks.key, "s");
    }
}
