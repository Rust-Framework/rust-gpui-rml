use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::{I18nExt, I18nState};
use rml_core::theme::{ThemeExt, ThemeState};

/// SettingsDialog：系统配置对话框（`#[window]` + `<dialog>` 根）。
///
/// 由 `SettingsAct::execute` 调用 `SettingsDialog::default().open(window, cx)` 打开。
/// 主题/语言变更实时同步到全局状态（ThemeState / I18nState），
/// 反向亦通过 `observe_global` 监听外部变更保持字段一致。
#[window]
#[derive(Default)]
pub struct SettingsDialog {
    pub is_dark: bool,
    pub language: SharedString,
    pub language_options: Vec<(SharedString, SharedString)>,
    pub font_size: f64,
    pub auto_save: bool,
    pub show_line_numbers: bool,
}

impl ILifecycle for SettingsDialog {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.is_dark = cx.current_theme() == "dark";
        self.language = cx.current_locale();
        self.language_options = vec![
            ("中文".into(), "zh-CN".into()),
            ("English".into(), "en-US".into()),
        ];
        self.font_size = 14.0;
        self.auto_save = true;
        self.show_line_numbers = true;

        cx.observe_global::<ThemeState>(|this, cx| {
            this.is_dark = cx.current_theme() == "dark";
            cx.notify();
        })
        .detach();

        cx.observe_global::<I18nState>(|this, cx| {
            this.language = cx.current_locale();
            cx.notify();
        })
        .detach();
    }
}

impl SettingsDialog {
    fn on_dark_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.is_dark = val;
        cx.set_theme(if val { "dark" } else { "light" });
        cx.notify();
    }

    fn on_language_change(&mut self, val: SharedString, cx: &mut Context<Self>) {
        self.language = val.clone();
        cx.set_i18n(val);
        cx.notify();
    }

    fn on_font_size_change(&mut self, val: f64, cx: &mut Context<Self>) {
        self.font_size = val;
        cx.notify();
    }

    fn on_auto_save_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.auto_save = val;
        cx.notify();
    }

    fn on_line_numbers_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.show_line_numbers = val;
        cx.notify();
    }
}
