use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.alert_dialog",
    kind = "case",
    group = "components",
    order = 83,
)]
#[component]
#[derive(Default)]
pub struct AlertDialogCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for AlertDialogCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.alert_dialog.title")
    }
}

impl ILifecycle for AlertDialogCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("title", "string", "对话框标题文本"),
            ("description", "string", "对话框描述文本（AlertDialog 专属，Dialog 无此属性）"),
            ("width", "长度", "对话框宽度，支持 px/裸数字，如 420px / 500，默认 420px"),
            ("confirm", "bool", "布尔属性，存在即显示取消按钮（等同 show-cancel=true）"),
            ("show-cancel", "bool", "是否显示取消按钮，默认 false；show-cancel=true 显示"),
            ("overlay-closable", "bool", "点击遮罩是否关闭，默认 false（AlertDialog 专属默认值）；overlay-closable=true 开启"),
            ("close-button", "bool", "是否显示关闭按钮，默认 false（AlertDialog 专属默认值）；close-button=true 显示"),
            ("keyboard", "bool", "是否支持 ESC 键关闭，默认 true；keyboard=false 禁用"),
            ("on-close", "event", "关闭事件回调，签名为 Fn(&ClickEvent, &mut Window, &mut App)"),
            ("on-ok", "event -> bool", "确认回调，返回 false 阻止关闭；签名 Fn(&ClickEvent, &mut Window, &mut App) -> bool"),
            ("on-cancel", "event -> bool", "取消回调，返回 false 阻止关闭；签名 Fn(&ClickEvent, &mut Window, &mut App) -> bool"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AlertDialogCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("alert_dialog_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("alert_dialog_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_alert_ok(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
        true
    }

    #[command]
    pub fn on_alert_cancel(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
        true
    }
}
