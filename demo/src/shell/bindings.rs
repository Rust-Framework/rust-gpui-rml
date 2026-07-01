//! Host 贡献元数据 → MVVM 控件数据映射（Demo 应用层）

use rml_core::contribution::IContributionRegistry;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::contribution::{VisualMode, VisualPlacement};
use rml_ui::{
    ActivityPanel, ActivityPanels, IconName, StatusBarAlign, StatusBarItem, StatusBarItems,
};

fn contribution_icon(name: &str) -> IconName {
    match name {
        "BookOpen" => IconName::BookOpen,
        "Settings" => IconName::Settings,
        "Frame" => IconName::Frame,
        _ => IconName::Frame,
    }
}

/// 活动栏 host → `ActivityPanels`（绑定 `<ActivityBar panels={...}>`）
pub fn activity_panels_from_host<C>(
    cx: &gpui::Context<C>,
    host_id: &str,
    active_id: &str,
) -> ActivityPanels {
    let registry = &cx.global::<ContributionRegistryGlobal>().0;
    let Some(host) = registry.host(host_id) else {
        return Vec::new();
    };

    let mut entries: Vec<_> = host
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.options.visual_mode,
                Some(VisualMode::Panel) | Some(VisualMode::Chrome) | None
            )
        })
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .map(|e| {
            let id = e.contribution.id();
            let icon = e
                .contribution
                .icon()
                .map(|s| contribution_icon(s.as_ref()))
                .unwrap_or(IconName::Frame);
            ActivityPanel::new(id, icon, e.contribution.name())
                .active(active_id == id)
                .into_arc()
        })
        .collect()
}

/// 状态栏 host → `StatusBarItems`（绑定 `<status_bar items={...}>`）
pub fn status_items_from_host<C>(cx: &gpui::Context<C>, host_id: &str) -> StatusBarItems {
    let registry = &cx.global::<ContributionRegistryGlobal>().0;
    let Some(host) = registry.host(host_id) else {
        return Vec::new();
    };

    let mut entries: Vec<_> = host
        .entries()
        .iter()
        .filter(|e| {
            matches!(
                e.options.visual_mode,
                Some(VisualMode::Inline) | None
            )
        })
        .collect();
    entries.sort_by_key(|e| e.options.order);

    entries
        .into_iter()
        .map(|e| {
            let align = match e.options.placement {
                Some(VisualPlacement::Right) => StatusBarAlign::Right,
                _ => StatusBarAlign::Left,
            };
            StatusBarItem::new(e.contribution.name())
                .align(align)
                .into_arc()
        })
        .collect()
}
