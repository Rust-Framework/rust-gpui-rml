//! KeyBinding —— 声明式键盘快捷键
//!
//! ## 推荐 RML 写法（焦点宿主子节点）
//!
//! ```rml
//! <Input ref="demo_input">
//!   <KeyBinding key="Ctrl+S" on-press={on_save} />
//!   <KeyBinding key="Escape" on-press={on_clear} />
//! </Input>
//! ```
//!
//! 编译器将 Input 包裹在 KeyBinding 链中，子树获得焦点时通过事件冒泡监听 keydown。

use gpui::{
    div, AnyElement, App, InteractiveElement, IntoElement, KeyDownEvent, Keystroke, ParentElement,
    RenderOnce, SharedString, StyleRefinement, Styled, Window,
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

/// 将 `Ctrl+S` / `Ctrl-S` / `ctrl+s` 等写法归一化为 GPUI `Keystroke::parse` 语法（`ctrl-s`）。
pub fn normalize_key_source(source: &str) -> String {
    source
        .split(|c| c == '+' || c == '-')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| part.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

impl RenderOnce for KeyBinding {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let normalized = normalize_key_source(self.key.as_ref());
        let target = Keystroke::parse(&normalized).ok();
        let when = self.when;
        let on_press = self.on_press;

        let mut container = div()
            .on_key_down(
                move |event: &KeyDownEvent, window: &mut Window, cx: &mut App| {
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
                },
            )
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

    #[test]
    fn normalize_ctrl_plus_s() {
        assert_eq!(normalize_key_source("Ctrl+S"), "ctrl-s");
    }

    #[test]
    fn normalize_ctrl_dash_o() {
        assert_eq!(normalize_key_source("ctrl-o"), "ctrl-o");
    }

    #[test]
    fn normalize_escape() {
        assert_eq!(normalize_key_source("Escape"), "escape");
    }

    #[test]
    fn parse_normalized_ctrl_s() {
        let normalized = normalize_key_source("Ctrl+S");
        let ks = Keystroke::parse(&normalized).expect("valid keystroke");
        assert!(ks.modifiers.control);
        assert_eq!(ks.key, "s");
    }
}
