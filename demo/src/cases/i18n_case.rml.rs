use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::{t_static, I18nState};
use rml_core::theme::ThemeExt;

#[contribute(
    host_id = "demo.shell",
    id = "i18n.basic",
    kind = "case",
    group = "i18n",
    order = 21,
)]
#[component]
#[derive(Default)]
pub struct I18nCase {}

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
        cx.observe_global::<I18nState>(|_this, cx| {
            cx.notify();
        })
        .detach();
    }
}

impl I18nCase {
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
    }

    #[command]
    pub fn on_toggle_theme(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" {
            "light"
        } else {
            "dark"
        };
        cx.set_theme(next);
    }
}
