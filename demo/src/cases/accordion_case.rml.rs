use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.accordion",
    kind = "case",
    group = "components",
    order = 10,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub last_open: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for AccordionCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.accordion.title")
    }
}

impl ILifecycle for AccordionCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("bordered", "布尔标志", "显示边框"),
            ("multiple", "布尔标志", "允许多项同时展开"),
            ("size", "small/medium/large", "尺寸变体"),
            ("on-toggle-click", "事件", "展开状态变化回调"),
            ("item title", "字符串", "面板标题"),
            ("item open", "布尔标志", "初始展开"),
            ("item icon", "图标名", "标题图标"),
            ("item disabled", "布尔", "禁用面板"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AccordionCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<accordion bordered="">
    <item title="第一项" open="">
        <p>内容</p>
    </item>
    <item title="第二项">
        <p>内容</p>
    </item>
</accordion>"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }
}
