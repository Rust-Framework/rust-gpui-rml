use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.ref",
    kind = "case",
    group = "framework",
    order = 55,
)]
#[component]
#[derive(Default)]
pub struct RefCase {
    /// 通过 `ref="input_state"` 指令关联到 ViewModel 同名字段。
    /// 首次渲染后即可在 command 回调中命令式访问（focus、set_value 等）。
    pub input_state: ElementRef<InputState>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for RefCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.ref.title")
    }
}

impl ILifecycle for RefCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段，如 ref=\"input_state\""),
            ("ViewModel 字段", "ref 字段", "在 code-behind 声明与 ref 同名的引用字段，用于命令式访问"),
            ("focus / set_value 等", "命令式 API", "在 #[command] 中通过引用字段调用 focus、set_value、scroll 等方法"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl RefCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("ref_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("ref_case.rml.rs").to_string()
    }
}
