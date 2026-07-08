use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[derive(Clone, Default)]
pub struct KeyItem {
    pub id: SharedString,
    pub label: SharedString,
}

#[contribute(
    host_id = "demo.shell",
    id = "framework.key",
    kind = "case",
    group = "framework",
    order = 53,
)]
#[component]
#[derive(Default)]
pub struct KeyCase {
    pub items: Vec<KeyItem>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for KeyCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.key.title")
    }
}

impl ILifecycle for KeyCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.items = vec![
            KeyItem {
                id: "i1".into(),
                label: "第一项".into(),
            },
            KeyItem {
                id: "i2".into(),
                label: "第二项".into(),
            },
            KeyItem {
                id: "i3".into(),
                label: "第三项".into(),
            },
        ];
        let (cols, rows) = build_api_table(&[
            ("key={expr}", "指令", "为 each 项提供稳定 ElementId（NamedInteger）"),
            ("key 优先级", "说明", "ref > key > 事件处理器（同时存在时按优先级消费）"),
            ("key 表达式作用域", "说明", "each 作用域内引用循环变量（如 item.id），非 self.item.id"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl KeyCase {
    #[computed]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("key_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("key_case.rml.rs").to_string()
    }

    /// 命令式构建单个列表项的渲染树。
    /// 由模板 `<component each={item in items} key={item.id} content={...} />` 调用。
    /// key={item.id} 提供稳定 ElementId，列表重排时 GPUI 能正确识别移动项，
    /// 保留元素状态（焦点、动画、内部 state）。
    pub fn render_item(
        &self,
        item: &KeyItem,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, px, IntoElement, ParentElement, Styled};
        use rml_ui::Tag;

        div()
            .px(px(8.))
            .py(px(4.))
            .child(Tag::new().child(item.id.clone()))
            .child(Tag::new().child(item.label.clone()))
            .into_any_element()
    }

    /// 命令式构建带 key 的列表渲染树。
    /// 由模板 `<component content={self.render_items(_window, cx)} />` 调用。
    /// 每项通过 .id(("rml_key", from_key(&item.id))) 提供稳定 ElementId，
    /// 列表重排时 GPUI 能正确识别移动项，保留元素状态。
    pub fn render_items(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, px, IntoElement, InteractiveElement, ParentElement, Styled};
        use rml_core::element_id;
        use rml_ui::Tag;

        div()
            .flex()
            .flex_row()
            .gap(px(8.))
            .flex_wrap()
            .children(self.items.iter().map(|item| {
                div()
                    .id(("rml_key", element_id::from_key(&item.id)))
                    .px(px(8.))
                    .py(px(4.))
                    .child(Tag::new().child(item.id.clone()))
                    .child(Tag::new().child(item.label.clone()))
            }))
            .into_any_element()
    }

    #[command]
    pub fn on_prepend(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let idx = self.items.len() + 1;
        self.items.insert(
            0,
            KeyItem {
                id: format!("i{}", idx).into(),
                label: format!("插入项 {}", idx).into(),
            },
        );
        self.__rml_bump_version("items");
    }

    #[command]
    pub fn on_clear(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.items.clear();
        self.__rml_bump_version("items");
    }
}
