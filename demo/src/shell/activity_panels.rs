//! 贡献点 → ActivityBar 面板适配

use std::sync::Arc;

use gpui::{AnyElement, App, BorrowAppContext, SharedString, Window};
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::contribution::{
    ContributedEntry, ContributionRenderContext, VisualMode, VisualPlacement, VisualRenderer,
};
use rml_ui::{ActivityPanels, IconName, IActivityPanel};

use super::contributions::{host_entries, icon_from_name, kind_of, KIND_ACTIVITY, SHELL_HOST};

struct ContributedActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
    visual: Option<VisualRenderer>,
}

impl IActivityPanel for ContributedActivityPanel {
    fn id(&self) -> SharedString {
        self.id.clone()
    }

    fn icon(&self) -> IconName {
        self.icon.clone()
    }

    fn title(&self) -> SharedString {
        self.title.clone()
    }

    fn is_activated(&self) -> bool {
        false
    }

    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let visual = self.visual.as_ref()?;
        let mut rendered = None;
        cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
            let cache = global.0.entity_cache_mut();
            let mut ctx = ContributionRenderContext {
                window,
                cx,
                active: true,
                mode: VisualMode::Panel,
                placement: VisualPlacement::Left,
            };
            rendered = Some(visual(&mut ctx, cache));
        });
        rendered
    }
}

/// host → ActivityPanels（kind=activity）；面板内容由各 `IActivityPanel::panel` 自提供
pub fn build_activity_panels<C>(cx: &gpui::Context<C>) -> ActivityPanels {
    let mut entries: Vec<&ContributedEntry> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_ACTIVITY))
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .map(|e| {
            let id = e.contribution.id();
            let icon = e
                .contribution
                .icon()
                .map(|s| icon_from_name(s.as_ref()))
                .unwrap_or(IconName::Frame);
            Arc::new(ContributedActivityPanel {
                id: id.into(),
                icon,
                title: e.contribution.name(),
                visual: e.visual.clone(),
            }) as Arc<dyn IActivityPanel>
        })
        .collect()
}
