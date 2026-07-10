use gpui::{AnyElement, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// 演示 IVisual trait —— 可渲染为 AnyElement 的值对象
///
/// `content={self.make_visual()}` 返回 `Box<dyn IVisual>`，
/// codegen 通过 `IntoContent` 调用 `IVisual::render` 转为 AnyElement。
struct CounterBadge {
    value: i32,
}

impl IVisual for CounterBadge {
    fn render(&self, _window: &mut Window, _cx: &mut gpui::App) -> AnyElement {
        div()
            .px(px(12.))
            .py(px(4.))
            .bg(gpui::rgb(0x1677ff))
            .text_color(gpui::rgb(0xffffff))
            .rounded(px(6.))
            .child(format!("IVisual 渲染：{}", self.value))
            .into_any_element()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "binding.content",
    kind = "case",
    group = "binding",
    order = 2,
)]
#[component]
#[derive(Default)]
pub struct ContentBindingCase {
    /// i32 → ToString 绑定
    pub count: i32,
    /// bool → ToString 绑定
    pub active: bool,
    /// String → IntoElement 绑定
    pub message: String,
    /// SharedString → IntoElement 绑定
    pub title: SharedString,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ContentBindingCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.content_binding.title")
    }
}

impl ILifecycle for ContentBindingCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.count = 42;
        self.active = true;
        self.message = "来自 String 字段的文本".to_string();
        self.title = SharedString::from("SharedString 标题");
        let (cols, rows) = build_api_table(&[
            ("content={i32}", "ToString", "数值格式化为文本"),
            ("content={bool}", "ToString", "布尔格式化为文本"),
            ("content={String}", "IntoElement", "字符串作为文本元素"),
            ("content={SharedString}", "IntoElement", "共享字符串作为文本元素"),
            ("content={AnyElement}", "IntoElement", "方法返回 AnyElement"),
            ("content={Box<dyn IVisual>}", "IVisual", "调用 IVisual::render"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ContentBindingCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("content_binding_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("content_binding_case.rml.rs").to_string()
    }

    /// 返回 AnyElement —— 演示 content 绑定方法返回值
    ///
    /// `content={self.render_badge(_window, cx)}` 生成
    /// `.child(rml_core::content::into_content(self.render_badge(_window, cx), _window, cx))`
    pub fn render_badge(&self, _window: &mut Window, _cx: &mut gpui::App) -> AnyElement {
        div()
            .px(px(12.))
            .py(px(4.))
            .bg(gpui::rgb(0x52c41a))
            .text_color(gpui::rgb(0xffffff))
            .rounded(px(6.))
            .child(format!("AnyElement 方法：count={}", self.count))
            .into_any_element()
    }

    /// 返回 Box<dyn IVisual> —— 演示 content 绑定 IVisual trait 对象
    ///
    /// `content={self.make_visual()}` 生成
    /// `.child(rml_core::content::into_content(self.make_visual(), _window, cx))`
    /// `IntoContent for Box<dyn IVisual>` 调用 `IVisual::render`
    pub fn make_visual(&self) -> Box<dyn IVisual> {
        Box::new(CounterBadge { value: self.count })
    }

    #[command]
    pub fn on_toggle(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.active = !self.active;
        cx.notify();
    }

    #[command]
    pub fn on_inc(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}
