use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.dialog",
    kind = "case",
    group = "components",
    order = 82,
)]
#[component]
#[derive(Default)]
pub struct DialogCase {
    /// on-ok/on-cancel 验证演示：输入为空时阻止关闭
    pub validate_input: ElementRef<rml_ui::InputState>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for DialogCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.dialog.title")
    }
}

impl ILifecycle for DialogCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("title", "string", "对话框标题文本，渲染在标题栏"),
            ("footer", "string / slot", "对话框页脚，支持字符串属性或 slot=footer 元素注入（slot 覆盖属性）"),
            ("width", "长度", "对话框宽度，支持 px/裸数字，如 500px / 600，默认 448px"),
            ("overlay", "bool", "是否显示背景遮罩，默认 true；overlay=false 关闭"),
            ("overlay-closable", "bool", "点击遮罩是否关闭对话框，默认 true；overlay-closable=false 禁用"),
            ("close-button", "bool", "是否显示关闭按钮，默认 true；close-button=false 隐藏"),
            ("keyboard", "bool", "是否支持 ESC 键关闭，默认 true；keyboard=false 禁用"),
            ("on-close", "event", "关闭事件回调，签名为 Fn(&ClickEvent, &mut Window, &mut App)"),
            ("on-ok", "event -> bool", "确认回调，返回 false 阻止关闭；签名 Fn(&ClickEvent, &mut Window, &mut App) -> bool"),
            ("on-cancel", "event -> bool", "取消回调，返回 false 阻止关闭；签名 Fn(&ClickEvent, &mut Window, &mut App) -> bool"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl DialogCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("dialog_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("dialog_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_validate_ok(&mut self, _: &ClickEvent, cx: &mut Context<Self>) -> bool {
        if let Some(entity) = self.validate_input.get() {
            if entity.read(cx).value().is_empty() {
                return false;
            }
        }
        true
    }

    #[command]
    pub fn on_validate_cancel(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
        true
    }
}
