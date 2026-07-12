use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.animate",
    kind = "case",
    group = "framework",
    order = 56,
)]
#[component]
#[derive(Default)]
pub struct AnimationCase {
    pub visible: bool,
    pub show_card: bool,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for AnimationCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.animation.title")
    }
}

impl ILifecycle for AnimationCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.visible = true;
        self.show_card = true;
        let (cols, rows) = build_api_table(&[
            ("animate=\"fade\"", "预设", "淡入：opacity 0→1，默认 300ms"),
            ("animate=\"slide-up\"", "预设", "从下滑入：top(20px→0) + opacity"),
            ("animate=\"slide-down\"", "预设", "从上滑入：top(-20px→0) + opacity"),
            ("animate=\"slide-left\"", "预设", "从右滑入：left(20px→0) + opacity"),
            ("animate=\"fade:500\"", "自定义时长", "冒号后指定毫秒数，覆盖默认 300ms"),
            ("animate + if", "组合", "if 切换时元素重新挂载，动画重播"),
            ("animate + show", "组合", "show 与 if 组合使用，切换时动画重播"),
            ("animate + each", "组合", "列表项逐一带入场动画"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AnimationCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("animation_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("animation_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_toggle_visible(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    #[command]
    pub fn on_toggle_show(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.show_card = !self.show_card;
        cx.notify();
    }
}
