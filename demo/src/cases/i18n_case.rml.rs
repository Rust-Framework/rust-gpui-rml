use rml::prelude::*;
use rml_core::i18n::I18nState;
use rml_core::theme::ThemeExt;

#[component]
#[derive(Default)]
pub struct I18nCase {}

impl ILifecycle for I18nCase {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        cx.observe_global::<I18nState>(|_this, cx| {
            cx.notify();
        })
        .detach();
    }
}

impl I18nCase {
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
