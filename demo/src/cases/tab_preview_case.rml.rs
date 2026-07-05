use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[derive(Clone, Default)]
pub struct TabData {
    pub title: SharedString,
    pub closable: bool,
    pub preview: bool,
}

#[contribute(
    host_id = "demo.shell",
    id = "components.tab_preview",
    kind = "case",
    group = "components",
    order = 63,
)]
#[component]
#[derive(Default)]
pub struct TabPreviewCase {
    pub tabs: Vec<TabData>,
    pub selected_index: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for TabPreviewCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tab_preview.title")
    }
}

impl ILifecycle for TabPreviewCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.reset_tabs();
        let (cols, rows) = build_api_table(&[
            ("on-close", "事件", "关闭按钮回调，签名 fn(index: usize)"),
            ("on-close-all", "事件", "关闭全部回调，签名 fn()"),
            ("on-close-others", "事件", "关闭其他回调，签名 fn(index: usize)"),
            ("on-promote", "事件", "双击 promote 回调，签名 fn(index: usize)"),
            ("Tab closable", "布尔", "显示关闭按钮"),
            ("Tab preview", "布尔", "预览模式（italic 标题）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TabPreviewCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.tabs.is_empty() {
            "没有打开的标签页".to_string()
        } else {
            let preview_count = self.tabs.iter().filter(|t| t.preview).count();
            format!(
                "共 {} 个标签页，选中第 {} 个，预览 {} 个",
                self.tabs.len(),
                self.selected_index + 1,
                preview_count
            )
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<TabBar
    selected-index={selected_index}
    on-click={on_tab_select}
    on-close={on_tab_close}
    on-close-all={on_tab_close_all}
    on-close-others={on_tab_close_others}
    on-promote={on_tab_promote}>
    <Tab each={tab in tabs}
        label={tab.title}
        closable={tab.closable}
        preview={tab.preview} />
</TabBar>"#
            .to_string()
    }

    fn reset_tabs(&mut self) {
        self.tabs = vec![
            TabData {
                title: "main.rs".into(),
                closable: false,
                preview: false,
            },
            TabData {
                title: "app.rs".into(),
                closable: true,
                preview: false,
            },
            TabData {
                title: "preview.rs".into(),
                closable: true,
                preview: true,
            },
            TabData {
                title: "config.toml".into(),
                closable: true,
                preview: false,
            },
        ];
        self.selected_index = 0;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.reset_tabs();
        cx.notify();
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            self.selected_index = index;
            cx.notify();
        }
    }

    #[command]
    pub fn on_tab_close(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.selected_index >= self.tabs.len() && !self.tabs.is_empty() {
            self.selected_index = self.tabs.len() - 1;
        } else if self.selected_index > index {
            self.selected_index -= 1;
        } else if self.tabs.is_empty() {
            self.selected_index = 0;
        }
        cx.notify();
    }

    #[command]
    pub fn on_tab_close_all(&mut self, cx: &mut Context<Self>) {
        self.tabs.clear();
        self.selected_index = 0;
        cx.notify();
    }

    #[command]
    pub fn on_tab_close_others(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        let kept = self.tabs.remove(index);
        self.tabs = vec![kept];
        self.selected_index = 0;
        cx.notify();
    }

    #[command]
    pub fn on_tab_promote(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() {
            self.tabs[index].preview = false;
            cx.notify();
        }
    }
}
