use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::{t_static, I18nState};
use rml_core::theme::ThemeExt;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "i18n.basic",
    kind = "case",
    group = "i18n",
    order = 21,
)]
#[component]
#[derive(Default)]
pub struct I18nCase {
    pub switch_count: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for I18nCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.i18n.title")
    }
}

impl ILifecycle for I18nCase {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("t(\"key\")", "模板函数", "引用 i18n 资源"),
            ("cx.set_i18n", "方法", "切换 locale"),
            ("cx.set_theme", "方法", "切换 dark/light 主题"),
            ("cx.observe_global::<I18nState>", "监听", "语言变化触发重渲"),
            ("cx.current_theme", "方法", "获取当前主题名"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
        cx.observe_global::<I18nState>(|_this, cx| {
            cx.notify();
        })
        .detach();
    }
}

impl I18nCase {
    #[computed]
    pub fn lang_label(&self) -> String {
        format!("切换次数：{}", self.switch_count)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<p>{t("demo.hello")}</p>
<Button label={t("menu.lang_en")} onclick={on_switch_en} />
<Button label={t("menu.theme_toggle")} onclick={on_toggle_theme} />"#
            .to_string()
    }

    #[command]
    pub fn on_switch_en(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        self.switch_count += 1;
    }

    #[command]
    pub fn on_switch_zh(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        cx.set_i18n("zh-CN");
        self.switch_count += 1;
    }

    #[command]
    pub fn on_toggle_theme(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
        self.switch_count += 1;
    }
}
