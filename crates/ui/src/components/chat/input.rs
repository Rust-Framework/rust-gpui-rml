//! 聊天输入组件。
//!
//! 提供文本输入框 + 发送/停止按钮，通过 [`ChatInputEvent`] 向外通知：
//! - 回车（无 Shift）或点击发送按钮 → [`ChatInputEvent::Send`]
//! - 流式响应中点击停止按钮 → [`ChatInputEvent::Stop`]

use gpui::*;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::{ActiveTheme, IconName, Sizable as _};
use gpui_component::button::{Button, ButtonVariants as _};

use super::event::ChatInputEvent;

/// 聊天输入区域。
pub struct ChatInput {
    input_state: Entity<InputState>,
    input_has_text: bool,
    is_streaming: bool,
    _input_sub: Option<Subscription>,
}

impl ChatInput {
    pub fn new(placeholder: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder.to_string())
                .auto_grow(1, 8)
                .soft_wrap(true)
        });

        let sub = cx.subscribe_in(
            &input_state,
            window,
            |this: &mut ChatInput,
             input_state: &Entity<InputState>,
             event: &InputEvent,
             window: &mut Window,
             cx: &mut Context<ChatInput>| {
                match event {
                    InputEvent::PressEnter { secondary: false, .. } => {
                        let text = input_state.update(cx, |state, cx| {
                            let value = state.value().to_string();
                            state.set_value("", window, cx);
                            value
                        });
                        if !text.trim().is_empty() {
                            this.input_has_text = false;
                            cx.emit(ChatInputEvent::Send(text));
                            cx.notify();
                        }
                    }
                    InputEvent::Change => {
                        let has_text = !input_state.read(cx).value().trim().is_empty();
                        if this.input_has_text != has_text {
                            this.input_has_text = has_text;
                            cx.notify();
                        }
                    }
                    _ => {}
                }
            },
        );

        Self {
            input_state,
            input_has_text: false,
            is_streaming: false,
            _input_sub: Some(sub),
        }
    }

    pub fn set_streaming(&mut self, streaming: bool, cx: &mut Context<Self>) {
        if self.is_streaming != streaming {
            self.is_streaming = streaming;
            cx.notify();
        }
    }

    pub fn set_placeholder(&mut self, text: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.input_state.update(cx, |state, cx| {
            state.set_placeholder(text.to_string(), window, cx);
        });
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    }
}

impl EventEmitter<ChatInputEvent> for ChatInput {}

impl Focusable for ChatInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input_state.read(cx).focus_handle(cx)
    }
}

impl Render for ChatInput {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let has_text = self.input_has_text;

        let trailing: AnyElement = if self.is_streaming {
            Button::new("chat-input-stop")
                .icon(IconName::CircleX)
                .danger()
                .small()
                .tooltip("停止")
                .on_click(cx.listener(|_this, _: &ClickEvent, _window, cx| {
                    cx.emit(ChatInputEvent::Stop);
                }))
                .into_any_element()
        } else if has_text {
            Button::new("chat-input-send")
                .icon(IconName::ArrowUp)
                .primary()
                .small()
                .tooltip("发送")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    let text = this.input_state.read(cx).value().to_string();
                    if text.trim().is_empty() {
                        return;
                    }
                    this.input_state.update(cx, |s, cx| {
                        s.set_value("", window, cx);
                    });
                    this.input_has_text = false;
                    cx.emit(ChatInputEvent::Send(text));
                    cx.notify();
                }))
                .into_any_element()
        } else {
            div().into_any_element()
        };

        div()
            .flex_shrink_0()
            .w_full()
            .px_3()
            .py_2()
            .border_t_1()
            .border_color(theme.border)
            .bg(theme.background)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_end()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                Input::new(&self.input_state)
                                    .appearance(false)
                                    .bordered(false)
                                    .xsmall()
                                    .px_0()
                                    .py_0()
                                    .w_full(),
                            ),
                    )
                    .child(trailing),
            )
    }
}
