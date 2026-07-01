//! StatusBar 文本贡献（纯元数据，由 ViewModel 映射为 `StatusBarItems`）

use std::sync::Arc;

use gpui::{App, BorrowAppContext, SharedString};
use rml_app::contribution::{data_registerable, ContributionRegistryGlobal, Registerable};
use rml_core::contribution::{
    ContributionOptions, IContribution, VisualMode, VisualPlacement,
};

use crate::shell::hosts;

#[derive(Clone)]
struct TextStatusContribution {
    id: &'static str,
    name_key: &'static str,
}

impl TextStatusContribution {
    fn register(self, cx: &mut App) {
        let contribution = Arc::new(self);
        let options = ContributionOptions::new()
            .visual_mode(VisualMode::Inline)
            .placement(VisualPlacement::Left)
            .order(0);
        cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
            global
                .0
                .register(hosts::STATUS, contribution, options, cx);
        });
    }
}

impl IContribution for TextStatusContribution {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> SharedString {
        rml_core::i18n::t_static(self.name_key).into()
    }

    fn description(&self) -> SharedString {
        SharedString::default()
    }

    fn icon(&self) -> Option<SharedString> {
        None
    }
}

impl Registerable for TextStatusContribution {
    fn into_entry(
        contribution: Arc<Self>,
        options: ContributionOptions,
    ) -> rml_core::contribution::ContributedEntry {
        data_registerable(contribution, options)
    }
}

pub fn register_status_text(cx: &mut App) {
    TextStatusContribution {
        id: "status.ready",
        name_key: "shell.status_ready",
    }
    .register(cx);
}
