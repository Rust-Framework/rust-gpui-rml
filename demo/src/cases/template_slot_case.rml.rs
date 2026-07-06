use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.template_slot",
    kind = "case",
    group = "framework",
    order = 44,
)]
#[component]
#[derive(Default)]
pub struct TemplateSlotCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for TemplateSlotCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.template_slot.title")
    }
}

impl ILifecycle for TemplateSlotCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("component content={expr}", "透明容器", "注入 AnyElement，不创建包装元素"),
            ("render_* 方法", "命令式", "构建可复用的 UI 块"),
            ("slot name=\"x\"", "指令", "组件模板内声明插槽渲染位置"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TemplateSlotCase {
    /// 构建信息卡片模板（带标题 + 内容 + 操作区）。
    /// 演示如何用 render 方法封装可复用的 UI 块。
    pub fn render_info_card(
        &self,
        title: &str,
        body: &str,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, px, IntoElement, ParentElement, Styled};
        use rml_ui::Card;

        Card::new(("info_card", 0usize))
            .title(title.to_string())
            .child(
                div()
                    .text_size(px(14.))
                    .child(body.to_string()),
            )
            .into_any_element()
    }

    /// 构建统计卡片模板（带标签 + 数值）。
    pub fn render_stat_card(
        &self,
        label: &str,
        value: &str,
        _window: &mut gpui::Window,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, px, FontWeight, IntoElement, ParentElement, Styled};
        use rml_ui::Card;

        Card::new(("stat_card", 0usize))
            .title(label.to_string())
            .child(
                div()
                    .text_size(px(28.))
                    .font_weight(FontWeight::BOLD)
                    .child(value.to_string()),
            )
            .into_any_element()
    }
}
