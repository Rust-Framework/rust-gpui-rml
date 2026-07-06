use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.native_status_bar",
    kind = "case",
    group = "components",
    order = 32,
)]
#[component]
#[derive(Default)]
pub struct NativeStatusBarCase {
    pub status_text: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for NativeStatusBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.native_status_bar.title")
    }
}

impl ILifecycle for NativeStatusBarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.status_text = "就绪".into();
        let (cols, rows) = build_api_table(&[
            ("子节点", "元素[]", "中央区域内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl NativeStatusBarCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<NativeStatusBar>
    <span>就绪</span>
</NativeStatusBar>"#
            .to_string()
    }

    #[command]
    pub fn on_show_ready(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "就绪".into();
    }

    #[command]
    pub fn on_show_warning(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "警告:请检查配置".into();
    }

    #[command]
    pub fn on_show_error(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.status_text = "错误:连接失败".into();
    }
}
