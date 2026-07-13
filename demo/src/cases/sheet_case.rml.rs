use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.sheet",
    kind = "case",
    group = "components",
    order = 81,
)]
#[component]
#[derive(Default)]
pub struct SheetCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    // 受控模式状态：每个 Sheet 对应一个 bool 字段
    pub show_sheet1: bool,
    pub show_sheet2: bool,
    pub show_sheet3: bool,
    pub show_sheet4: bool,
    pub show_sheet5: bool,
    pub show_sheet6: bool,
    pub show_sheet7: bool,
    pub show_sheet8: bool,
}

impl IContribution for SheetCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.sheet.title")
    }
}

impl ILifecycle for SheetCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("title", "string", "抽屉标题文本，渲染在标题栏左侧"),
            ("footer", "string", "抽屉页脚文本，渲染在底部区域"),
            ("size", "number", "抽屉面板尺寸，支持 px/百分比/裸数字，如 350px / 50% / 400"),
            ("resizable", "bool", "是否可拖拽调整大小，默认 true；resizable=false 禁用"),
            ("overlay", "bool", "是否显示背景遮罩，默认 true；overlay=false 关闭"),
            ("overlay-closable", "bool", "点击遮罩是否关闭抽屉，默认 true；overlay-closable=false 禁用"),
            ("open", "bind", "受控模式字段绑定，如 open={show_sheet}；为 true 时渲染抽屉，on_close 自动回写 false"),
            ("on-close", "event", "抽屉关闭时回调（受控模式下与自动回写合并）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SheetCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("sheet_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("sheet_case.rml.rs").to_string()
    }

    #[command]
    pub fn open_sheet1(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet1 = true;
        self.__rml_bump_version("show_sheet1");
        cx.notify();
    }

    #[command]
    pub fn open_sheet2(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet2 = true;
        self.__rml_bump_version("show_sheet2");
        cx.notify();
    }

    #[command]
    pub fn open_sheet3(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet3 = true;
        self.__rml_bump_version("show_sheet3");
        cx.notify();
    }

    #[command]
    pub fn open_sheet4(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet4 = true;
        self.__rml_bump_version("show_sheet4");
        cx.notify();
    }

    #[command]
    pub fn open_sheet5(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet5 = true;
        self.__rml_bump_version("show_sheet5");
        cx.notify();
    }

    #[command]
    pub fn open_sheet6(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet6 = true;
        self.__rml_bump_version("show_sheet6");
        cx.notify();
    }

    #[command]
    pub fn open_sheet7(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet7 = true;
        self.__rml_bump_version("show_sheet7");
        cx.notify();
    }

    #[command]
    pub fn open_sheet8(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_sheet8 = true;
        self.__rml_bump_version("show_sheet8");
        cx.notify();
    }
}
