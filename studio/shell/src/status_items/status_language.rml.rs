//! 状态栏语言指示项 —— 右侧显示当前 locale。

use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::IContribution;
use rml_core::i18n::{t_static, I18nExt};

#[contribute(
    host_id = "studio.shell",
    id = "status.language",
    kind = "status",
    align = "right",
    order = 11
)]
#[component]
#[derive(Default)]
pub struct StatusLanguage {
    locale: SharedString,
}

impl IContribution for StatusLanguage {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("studio.status.language")
    }
}

impl ILifecycle for StatusLanguage {
    fn before_render(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.locale = cx.current_locale().to_string().into();
    }
}
