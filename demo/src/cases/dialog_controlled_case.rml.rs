use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.dialog_controlled",
    kind = "case",
    group = "components",
    order = 83,
)]
#[component]
#[derive(Default)]
pub struct DialogControlledCase {
    /// 基础受控模式
    pub show_basic: bool,
    /// 带 on-close 合并回调
    pub show_with_callback: bool,
    pub close_count: u32,
    /// 表单弹窗
    pub show_form: bool,
    pub form_name: ElementRef<rml_ui::InputState>,
    pub form_email: ElementRef<rml_ui::InputState>,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for DialogControlledCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.dialog_controlled.title")
    }
}

impl ILifecycle for DialogControlledCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("open", "bind: bool", "受控开关字段。true 时渲染对话框，false 时渲染空元素。仅支持简单字段引用。"),
            ("on-close", "event", "关闭回调，与 open 自动回写合并执行：先回写 field=false，再调用用户 handler"),
            ("title", "string", "对话框标题文本"),
            ("width", "长度", "对话框宽度，支持 px/裸数字，如 500px / 600"),
            ("overlay", "bool", "是否显示背景遮罩，默认 true"),
            ("overlay-closable", "bool", "点击遮罩是否关闭对话框，默认 true"),
            ("close-button", "bool", "是否显示关闭按钮，默认 true"),
            ("keyboard", "bool", "是否支持 ESC 键关闭，默认 true"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl DialogControlledCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("dialog_controlled_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("dialog_controlled_case.rml.rs").to_string()
    }

    #[command]
    pub fn open_basic(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_basic = true;
        self.__rml_bump_version("show_basic");
        cx.notify();
    }

    #[command]
    pub fn open_with_callback(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_with_callback = true;
        self.__rml_bump_version("show_with_callback");
        cx.notify();
    }

    #[command]
    pub fn on_dialog_closed(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.close_count += 1;
        self.__rml_bump_version("close_count");
        cx.notify();
    }

    #[command]
    pub fn open_form(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_form = true;
        self.__rml_bump_version("show_form");
        cx.notify();
    }

    #[command]
    pub fn on_form_closed(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if let (Some(name), Some(email)) = (self.form_name.get(), self.form_email.get()) {
            let name_val = name.read(cx).value().to_string();
            let email_val = email.read(cx).value().to_string();
            if !name_val.is_empty() || !email_val.is_empty() {
                // 实际场景中可在此处理表单数据
            }
        }
        cx.notify();
    }
}
