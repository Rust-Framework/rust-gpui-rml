use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.list",
    kind = "case",
    group = "framework",
    order = 43,
)]
#[component]
#[derive(Default)]
pub struct ListCase {
    pub items: Vec<SharedString>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.list.title")
    }
}

impl ILifecycle for ListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.items = vec![
            "Rust".into(),
            "GPUI".into(),
            "RML".into(),
            "Component".into(),
            "Binding".into(),
        ];
        let (cols, rows) = build_api_table(&[
            ("each={x in items}", "指令", "遍历 Vec<T> 渲染每个元素"),
            ("content={expr}", "绑定", "每项的渲染内容（AnyElement）"),
            ("Vec<SharedString>", "字段", "可迭代的数据源"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ListCase {
    #[computed]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("list_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("list_case.rml.rs").to_string()
    }

    /// 命令式构建单个 Tag 项的渲染树。
    pub fn render_item(
        &self,
        item: &SharedString,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, px, IntoElement, ParentElement, Styled};
        use rml_ui::Tag;

        div()
            .px(px(8.))
            .py(px(4.))
            .child(Tag::new().child(item.clone()))
            .into_any_element()
    }

    /// 命令式构建整个列表的渲染树（绕过 slot 内 each 指令的 codegen 限制）。
    /// 由模板 `<component content={self.render_item_list(_window, cx)} />` 调用。
    pub fn render_item_list(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, IntoElement, ParentElement, Styled};
        let mut container = div().flex().flex_row().gap(gpui::px(8.0)).flex_wrap();
        for item in &self.items {
            container = container.child(self.render_item(item, _window, _cx));
        }
        container.into_any_element()
    }

    #[command]
    pub fn on_add_item(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let idx = self.items.len() + 1;
        self.items.push(format!("Item {}", idx).into());
        self.__rml_bump_version("items");
    }

    #[command]
    pub fn on_remove_item(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.items.pop();
        self.__rml_bump_version("items");
    }
}
