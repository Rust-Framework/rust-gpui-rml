use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.code_editor",
    kind = "case",
    group = "components",
    order = 38,
)]
#[component]
#[derive(Default)]
pub struct CodeEditorCase {
    /// 通过 `ref="editor_state"` 指令关联，
    /// 首次渲染后由 `__rml_populate_refs` 注入 `Entity<InputState>` 句柄。
    ///
    /// `code_editor("rust").multi_line(true)` 等 builder 配置需要 InputState 实例，
    /// 而 on_loaded 阶段 ref_entities 尚未填充，故这些配置应在首次 render 后通过
    /// ElementRef.with_mut 设置，或后续通过其他生命周期钩子（如 on_rendered，待 M5' 实现）。
    pub editor_state: ElementRef<InputState>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CodeEditorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.code_editor.title")
    }
}

impl ILifecycle for CodeEditorCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("InputState::code_editor", "语言字符串", "启用代码编辑器模式"),
            ("InputState::multi_line", "布尔", "多行编辑"),
            ("InputState::default_value", "字符串", "默认代码内容"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CodeEditorCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("code_editor_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("code_editor_case.rml.rs").to_string()
    }
}
