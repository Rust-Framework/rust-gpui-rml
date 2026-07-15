use gpui::{AnyElement, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// 【教学例外】演示 IVisual trait —— 可渲染为 AnyElement 的值对象
///
/// ⚠️ 本文件存在 3 处故意保留的命令式 UI 代码（`CounterBadge` 手写 IVisual、
/// `render_badge` 返回 AnyElement、`make_visual` 返回 Box<dyn IVisual>），
/// 专门用于演示 RML `content` 绑定不同类型（IntoElement / ToString / IVisual）
/// 的框架能力，**不是生产代码的最佳实践**。
///
/// 生产代码应使用 `#[component]` + `.rml` 模板声明式实现，禁止在 `.rml.rs`
/// 中构造 `div().child()` 或返回 `AnyElement`。
///
/// 此处 `content={self.make_visual()}` 返回 `Box<dyn IVisual>`，
/// codegen 通过 `IntoContent` 调用 `IVisual::render` 转为 AnyElement。
struct CounterBadge {
    value: i32,
}

impl IVisual for CounterBadge {
    fn render(&self, _window: &mut Window, cx: &mut gpui::App) -> AnyElement {
        div()
            .px(px(12.))
            .py(px(4.))
            .bg(cx.theme().primary)
            .text_color(cx.theme().primary_foreground)
            .rounded(px(6.))
            .child(format!("IVisual 渲染：{}", self.value))
            .into_any_element()
    }
}

/// 演示 each 指令遍历结构体列表，绑定字段 content
pub struct Person {
    pub name: String,
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
    /// String → IntoElement 绑定（自动 & 借用，无需 .clone()）
    pub message: String,
    /// SharedString → IntoElement 绑定（自动 & 借用，无需 .clone()）
    pub title: SharedString,
    /// Vec<String> → each 指令遍历，循环变量为 &String
    pub names: Vec<String>,
    /// Vec<Person> → each 指令遍历，绑定 item.name 字段
    pub people: Vec<Person>,
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
        self.names = vec![
            "Rust".to_string(),
            "GPUI".to_string(),
            "RML".to_string(),
            "MVVM".to_string(),
        ];
        self.people = vec![
            Person { name: "Alice".to_string() },
            Person { name: "Bob".to_string() },
            Person { name: "Charlie".to_string() },
        ];
        let (cols, rows) = build_api_table(&[
            ("content={number}", "string", "数值格式化为文本"),
            ("content={bool}", "string", "布尔格式化为文本"),
            ("content={string}", "slot", "字符串作为文本元素"),
            ("content={方法}", "slot", "方法返回的元素作为内容"),
            ("content={each 变量}", "slot", "each 循环变量直接作为内容"),
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

    /// 【教学例外】返回 AnyElement —— 演示 content 绑定方法返回值
    ///
    /// ⚠️ 故意保留的命令式 UI，仅为演示 `content` 绑定能力，非生产最佳实践。
    /// 生产代码应在 `.rml` 模板中声明式实现。
    ///
    /// `content={self.render_badge(_window, cx)}` 生成
    /// `.child(rml_core::content::into_content(self.render_badge(_window, cx), _window, cx))`
    pub fn render_badge(&self, _window: &mut Window, cx: &mut gpui::App) -> AnyElement {
        div()
            .px(px(12.))
            .py(px(4.))
            .bg(cx.theme().success)
            .text_color(cx.theme().success_foreground)
            .rounded(px(6.))
            .child(format!("AnyElement 方法：count={}", self.count))
            .into_any_element()
    }

    /// 【教学例外】返回 Box<dyn IVisual> —— 演示 content 绑定 IVisual trait 对象
    ///
    /// ⚠️ 故意保留的命令式 UI，仅为演示 `content` 绑定能力，非生产最佳实践。
    /// 生产代码应在 `.rml` 模板中声明式实现。
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
