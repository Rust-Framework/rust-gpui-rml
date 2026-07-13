//! 聊天输入组件。
//!
//! 视觉规范对齐源项目 `agent/src/chat/chat_input.rs`：
//! - 胶囊模式（默认）：rounded_full, h=46, [+] [input] [model] [send]
//! - 展开模式（多行/超60字）：rounded_md, flex_col, [input] [toolbar]
//! - 底部状态栏：Local Mode + 流式指示器

use gpui::prelude::FluentBuilder as _;
use gpui::*;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{PopupMenu, PopupMenuItem};
use gpui_component::IconName;
use gpui_component::{ActiveTheme, Disableable as _, Sizable as _};

use super::event::ChatInputEvent;

/// 模型信息。
#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
}

/// 聊天输入区域。
pub struct ChatInput {
    input_state: Entity<InputState>,
    input_has_text: bool,
    is_streaming: bool,
    is_expanded: bool,
    add_menu_open: bool,
    add_popup: Option<Entity<PopupMenu>>,
    add_popup_sub: Option<Subscription>,
    model_menu_open: bool,
    model_popup: Option<Entity<PopupMenu>>,
    model_popup_sub: Option<Subscription>,
    available_models: Vec<ModelInfo>,
    active_model_id: Option<String>,
    session_usage: String,
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
                    InputEvent::PressEnter {
                        secondary: false, ..
                    } => {
                        let text = input_state.update(cx, |state, cx| {
                            let value = state.value().to_string();
                            state.set_value("", window, cx);
                            value
                        });
                        if !text.trim().is_empty() {
                            this.input_has_text = false;
                            this.is_expanded = false;
                            cx.emit(ChatInputEvent::Send(text));
                            cx.notify();
                        }
                    }
                    InputEvent::Change => {
                        let text = input_state.read(cx).value().to_string();
                        let has_text = !text.trim().is_empty();
                        let expanded = text.contains('\n') || text.chars().count() > 60;
                        if this.input_has_text != has_text || this.is_expanded != expanded {
                            this.input_has_text = has_text;
                            this.is_expanded = expanded;
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
            is_expanded: false,
            add_menu_open: false,
            add_popup: None,
            add_popup_sub: None,
            model_menu_open: false,
            model_popup: None,
            model_popup_sub: None,
            available_models: Vec::new(),
            active_model_id: None,
            session_usage: String::new(),
            _input_sub: Some(sub),
        }
    }

    pub fn set_streaming(&mut self, streaming: bool, cx: &mut Context<Self>) {
        if self.is_streaming != streaming {
            self.is_streaming = streaming;
            cx.notify();
        }
    }

    pub fn set_models(
        &mut self,
        models: Vec<ModelInfo>,
        active_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.available_models = models;
        self.active_model_id = active_id;
        cx.notify();
    }

    pub fn set_session_usage(&mut self, text: String, cx: &mut Context<Self>) {
        if self.session_usage != text {
            self.session_usage = text;
            cx.notify();
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.input_state.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    }

    fn close_add_menu(&mut self, cx: &mut Context<Self>) {
        self.add_menu_open = false;
        self.add_popup = None;
        self.add_popup_sub = None;
        cx.notify();
    }

    fn close_model_menu(&mut self, cx: &mut Context<Self>) {
        self.model_menu_open = false;
        self.model_popup = None;
        self.model_popup_sub = None;
        cx.notify();
    }

    fn build_add_popup(&self, window: &mut Window, cx: &mut Context<Self>) -> Entity<PopupMenu> {
        let input_state = self.input_state.clone();
        PopupMenu::build(window, cx, move |menu, _w, _cx| {
            let mut m = menu;
            let s1 = input_state.clone();
            let s2 = input_state.clone();
            let s3 = input_state.clone();
            m = m.item(
                PopupMenuItem::new("添加图片").on_click(move |_, window, cx| {
                    s1.update(cx, |state, cx| {
                        state.insert("@image ", window, cx);
                    });
                }),
            );
            m = m.item(
                PopupMenuItem::new("添加文件").on_click(move |_, window, cx| {
                    s2.update(cx, |state, cx| {
                        state.insert("@file ", window, cx);
                    });
                }),
            );
            m = m.item(
                PopupMenuItem::new("添加计划").on_click(move |_, window, cx| {
                    s3.update(cx, |state, cx| {
                        state.insert("/plan ", window, cx);
                    });
                }),
            );
            m.min_w(px(180.))
        })
    }

    fn build_model_popup(&self, window: &mut Window, cx: &mut Context<Self>) -> Entity<PopupMenu> {
        let models = self.available_models.clone();
        let active_id = self.active_model_id.clone();
        let entity = cx.entity();
        PopupMenu::build(window, cx, move |menu, _w, _cx| {
            let mut m = menu;
            for model in &models {
                let model_id = model.id.clone();
                let display = model.display_name.clone();
                let is_active = active_id.as_deref() == Some(&model_id);
                let label = if is_active {
                    format!("✓ {}", display)
                } else {
                    format!("  {}", display)
                };
                let mid = model_id.clone();
                let ent = entity.clone();
                m = m.item(PopupMenuItem::new(label).on_click(move |_, _window, cx| {
                    ent.update(cx, |this, cx| {
                        cx.emit(ChatInputEvent::SelectModel(mid.clone()));
                        this.close_model_menu(cx);
                    });
                }));
            }
            m.min_w(px(200.))
        })
    }

    /// 渲染底部状态栏。
    fn render_footer(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let usage_text = if self.is_streaming {
            "思考中...".to_string()
        } else if self.session_usage.is_empty() {
            String::new()
        } else {
            self.session_usage.clone()
        };
        let show_indicator = self.is_streaming || !self.session_usage.is_empty();

        div()
            .w_full()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .px(px(4.))
            .text_sm()
            .text_color(theme.muted_foreground)
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .child(
                        gpui_component::Icon::new(IconName::Cpu)
                            .with_size(px(13.))
                            .text_color(theme.muted_foreground),
                    )
                    .child("Local Mode"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap_2()
                    .when(show_indicator, |el| {
                        el.child(div().w(px(12.)).h(px(12.)).rounded_full().bg(
                            if self.is_streaming {
                                theme.accent.opacity(0.6)
                            } else {
                                theme.muted_foreground.opacity(0.45)
                            },
                        ))
                    })
                    .child(usage_text),
            )
    }

    /// 渲染添加按钮（+）。
    fn render_add_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mut el = div()
            .id("chat-input-add-anchor")
            .relative()
            .w(px(28.))
            .h(px(28.))
            .rounded_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .hover(|el| el.bg(theme.muted.opacity(0.5)))
            .flex()
            .items_center()
            .justify_center()
            .child(
                Button::new("input-add")
                    .ghost()
                    .xsmall()
                    .icon(IconName::Plus)
                    .cursor_pointer()
                    .tooltip("添加文件")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        if this.add_menu_open {
                            this.close_add_menu(cx);
                            return;
                        }
                        let popup = this.build_add_popup(window, cx);
                        let sub = cx.subscribe::<PopupMenu, gpui::DismissEvent>(
                            &popup,
                            |this, _, _, cx| {
                                this.close_add_menu(cx);
                            },
                        );
                        this.add_popup_sub = Some(sub);
                        this.add_popup = Some(popup);
                        this.add_menu_open = true;
                        cx.notify();
                    })),
            );

        if self.add_menu_open {
            if let Some(popup) = &self.add_popup {
                let fh = popup.read(cx).focus_handle(cx);
                if !fh.contains_focused(window, cx) {
                    fh.focus(window, cx);
                }
                el = el.child(deferred(
                    anchored()
                        .anchor(gpui::Anchor::BottomLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(div().size_full().occlude().bottom_1().child(popup.clone())),
                ));
            }
        }
        el
    }

    /// 渲染模型选择按钮。
    fn render_model_button(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model_label = if let Some(ref active_id) = self.active_model_id {
            self.available_models
                .iter()
                .find(|m| m.id == *active_id)
                .map(|m| m.display_name.clone())
                .unwrap_or_else(|| "自动模式".to_string())
        } else {
            "自动模式".to_string()
        };

        let mut el = div().id("chat-input-model-anchor").relative().child(
            Button::new("input-model-select")
                .ghost()
                .small()
                .label(model_label)
                .icon(IconName::ChevronDown)
                .cursor_pointer()
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    if this.model_menu_open {
                        this.close_model_menu(cx);
                        return;
                    }
                    if this.available_models.is_empty() {
                        return;
                    }
                    let popup = this.build_model_popup(window, cx);
                    let sub =
                        cx.subscribe::<PopupMenu, gpui::DismissEvent>(&popup, |this, _, _, cx| {
                            this.close_model_menu(cx);
                        });
                    this.model_popup_sub = Some(sub);
                    this.model_popup = Some(popup);
                    this.model_menu_open = true;
                    cx.notify();
                })),
        );

        if self.model_menu_open {
            if let Some(popup) = &self.model_popup {
                let fh = popup.read(cx).focus_handle(cx);
                if !fh.contains_focused(window, cx) {
                    fh.focus(window, cx);
                }
                el = el.child(deferred(
                    anchored()
                        .anchor(gpui::Anchor::BottomLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(div().size_full().occlude().bottom_1().child(popup.clone())),
                ));
            }
        }
        el
    }

    /// 渲染尾部操作按钮（发送/停止）。
    fn render_trailing(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();

        if self.is_streaming {
            return Button::new("stop-generation")
                .icon(IconName::CircleX)
                .danger()
                .small()
                .tooltip("停止")
                .on_click(cx.listener(|_this, _: &ClickEvent, _window, cx| {
                    cx.emit(ChatInputEvent::Stop);
                }))
                .into_any_element();
        }

        let has_text = self.input_has_text;
        div()
            .w(px(28.))
            .h(px(28.))
            .rounded_full()
            .bg(theme.muted)
            .border_1()
            .border_color(theme.border)
            .cursor_pointer()
            .hover(|el| el.bg(theme.muted.opacity(0.8)))
            .flex()
            .items_center()
            .justify_center()
            .child(
                Button::new("send-message")
                    .ghost()
                    .xsmall()
                    .icon(IconName::ArrowUp)
                    .cursor_pointer()
                    .when(!has_text, |btn| btn.disabled(true))
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
                        this.is_expanded = false;
                        cx.emit(ChatInputEvent::Send(text));
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    /// 渲染输入框外壳（胶囊/展开）。
    fn render_shell(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().clone();
        let mut input_element = Input::new(&self.input_state)
            .appearance(false)
            .bordered(false)
            .xsmall()
            .px_0()
            .py_0()
            .w_full();
        if !self.is_expanded {
            input_element = input_element.h_full();
        }

        let add_btn = self.render_add_button(window, cx);
        let model_btn = self.render_model_button(window, cx);
        let trailing = self.render_trailing(cx);

        if self.is_expanded {
            div()
                .id("chat-input-shell")
                .w_full()
                .min_w_0()
                .border_1()
                .border_color(theme.border)
                .bg(theme.input)
                .rounded_md()
                .flex()
                .flex_col()
                .overflow_hidden()
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px(px(12.))
                        .pt(px(10.))
                        .pb(px(4.))
                        .child(input_element),
                )
                .child(
                    div()
                        .flex_shrink_0()
                        .w_full()
                        .flex()
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .px(px(10.))
                        .py(px(8.))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_2()
                                .child(add_btn)
                                .child(model_btn),
                        )
                        .child(trailing),
                )
        } else {
            div()
                .id("chat-input-shell")
                .w_full()
                .min_w_0()
                .border_1()
                .border_color(theme.border)
                .bg(theme.input)
                .rounded_full()
                .h(px(46.))
                .flex()
                .flex_row()
                .items_center()
                .px(px(8.))
                .gap_1p5()
                .child(add_btn)
                .child(div().flex_1().min_w_0().child(input_element))
                .child(model_btn)
                .child(trailing)
        }
    }
}

impl EventEmitter<ChatInputEvent> for ChatInput {}

impl Focusable for ChatInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.input_state.read(cx).focus_handle(cx)
    }
}

impl Render for ChatInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let shell = self.render_shell(window, cx);
        let footer = self.render_footer(cx);

        div()
            .flex_shrink_0()
            .w_full()
            .px(px(12.))
            .pt(px(8.))
            .pb(px(10.))
            .child(div().flex().flex_col().gap_2().child(shell).child(footer))
    }
}
