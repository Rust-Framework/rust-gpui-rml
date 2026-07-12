use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_core::theme::ThemeExt;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.theme_switcher",
    kind = "case",
    group = "framework",
    order = 56,
)]
#[component]
#[derive(Default)]
pub struct ThemeSwitcherCase {
    pub current_theme: SharedString,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ThemeSwitcherCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.theme_switcher.title")
    }
}

impl ILifecycle for ThemeSwitcherCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.current_theme = cx.current_theme();
        let (cols, rows) = build_api_table(&[
            ("value", "string / binding", "当前主题名，如 value={theme}；支持 light / dark 等"),
            ("ThemeSwitcher", "组件", "声明式主题切换器，绑定 value 后自动切换全局主题"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ThemeSwitcherCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("theme_switcher_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("theme_switcher_case.rml.rs").to_string()
    }

    #[command]
    pub fn on_light(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_theme = "light".into();
    }

    #[command]
    pub fn on_dark(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current_theme = "dark".into();
    }
}
