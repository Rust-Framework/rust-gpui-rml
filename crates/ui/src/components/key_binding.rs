//! KeyBinding —— 声明式键盘快捷键
//!
//! RML `<KeyBinding key="Ctrl+S" on-press={handle_save} />` 在子树获得焦点时
//! 监听键盘事件，匹配快捷键组合后触发 on_press 回调。
//!
//! 组件作为容器包裹子元素，通过 GPUI 事件冒泡机制接收子元素的 keydown 事件。
//! 修饰键语法遵循 GPUI `Keystroke::parse`：ctrl / alt / shift / cmd / win / super / fn / secondary。

use gpui::{
    AnyElement, App, InteractiveElement, IntoElement, KeyDownEvent, Keystroke, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
};
use gpui_component::StyledExt as _;

/// 声明式键盘快捷键
///
/// 包裹子元素，监听 keydown 事件，匹配 `key` 属性指定的快捷键组合后触发 `on_press`。
/// 通过 `when` 属性控制是否启用（默认 true）。
#[derive(IntoElement)]
pub struct KeyBinding {
    key: SharedString,
    when: bool,
    on_press: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    children: Vec<AnyElement>,
    style: StyleRefinement,
}

impl Default for KeyBinding {
    fn default() -> Self {
        Self {
            key: SharedString::default(),
            when: true,
            on_press: None,
            children: Vec::new(),
            style: StyleRefinement::default(),
        }
    }
}

impl KeyBinding {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置快捷键组合（如 "Ctrl+S"）
    pub fn key(mut self, key: impl Into<SharedString>) -> Self {
        self.key = key.into();
        self
    }

    /// 设置是否启用（默认 true）
    pub fn when(mut self, when: bool) -> Self {
        self.when = when;
        self
    }

    /// 设置快捷键触发回调
    pub fn on_press(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_press = Some(Box::new(handler));
        self
    }
}

impl Styled for KeyBinding {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for KeyBinding {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for KeyBinding {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let target = Keystroke::parse(self.key.as_ref()).ok();
        let when = self.when;
        let on_press = self.on_press;

        let mut container = div()
            .on_key_down(move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
                if !when {
                    return;
                }
                let Some(target) = target.as_ref() else {
                    return;
                };
                if event.keystroke.key == target.key
                    && event.keystroke.modifiers == target.modifiers
                {
                    if let Some(handler) = &on_press {
                        handler(window, cx);
                        cx.stop_propagation();
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
