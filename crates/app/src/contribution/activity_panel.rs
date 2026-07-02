//! Activity slot 组件贡献 → `IActivityPanel`

extern crate rust_rml_ui as rml_ui;

use std::borrow::BorrowMut;
use std::sync::Arc;

use gpui::{App, Context, SharedString, Window};
use gpui_component::IconName;
use rml_core::contribution::{ContributedEntry, VisualRenderer};
use rml_ui::{ActivityPanel, ActivityPanels, IActivityPanel};

use super::global::contribution_entries;
use super::render::render_contribution_visual;

fn icon_from_contribution(name: &str) -> IconName {
    match name {
        "BookOpen" => IconName::BookOpen,
        "Settings" => IconName::Settings,
        "Frame" => IconName::Frame,
        _ => IconName::Frame,
    }
}

/// 组件贡献在 ActivityBar 中的呈现：元数据 + visual 渲染器
struct ContributedActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
    visual: VisualRenderer,
}

impl ContributedActivityPanel {
    fn from_entry(entry: &ContributedEntry, visual: VisualRenderer) -> Self {
        let icon = entry
            .contribution
            .icon()
            .map(|s| icon_from_contribution(s.as_ref()))
            .unwrap_or(IconName::Frame);
        Self {
            id: entry.contribution.id().into(),
            icon,
            title: entry.contribution.name(),
            visual,
        }
    }
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

    fn panel(
        &self,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<gpui::AnyElement> {
        render_contribution_visual(&self.visual, window, cx)
    }
}

/// 在 Host `Render::render` 中解析当前激活面板内容（勿在 `RenderOnce` 内调用 `panel()`）
pub fn resolve_active_panel_body<C>(
    panels: &ActivityPanels,
    active_id: Option<&SharedString>,
    window: &mut Window,
    cx: &mut Context<C>,
) -> Option<gpui::AnyElement> {
    let id = active_id?;
    let app = cx.borrow_mut();
    panels
        .iter()
        .find(|p| &p.id() == id)
        .and_then(|p| p.panel(window, app))
}

/// 读取 host 的 `activity` slot
pub fn map_activity_panels<C>(host_id: &str, cx: &Context<C>) -> ActivityPanels {
    let mut entries: Vec<&ContributedEntry> = contribution_entries(host_id, cx)
        .iter()
        .filter(|e| e.options.effective_slot() == Some("activity"))
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .filter_map(|e| {
            if let Some(visual) = e.visual.clone() {
                Some(
                    Arc::new(ContributedActivityPanel::from_entry(e, visual))
                        as Arc<dyn IActivityPanel>,
                )
            } else {
                let icon = e
                    .contribution
                    .icon()
                    .map(|s| icon_from_contribution(s.as_ref()))
                    .unwrap_or(IconName::Frame);
                Some(
                    ActivityPanel::new(e.contribution.id(), icon, e.contribution.name()).into_arc()
                        as Arc<dyn IActivityPanel>,
                )
            }
        })
        .collect()
}
