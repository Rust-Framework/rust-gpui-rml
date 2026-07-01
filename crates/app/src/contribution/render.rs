//! 视觉贡献渲染（封装 registry entity cache 访问）

use gpui::{AnyElement, App, BorrowAppContext, Window};
use rml_core::contribution::{ContributionRenderContext, VisualRenderer};

use super::global::ContributionRegistryGlobal;

/// 执行 `VisualRenderer` 并返回 GPUI element（应用层禁止直接访问 `entity_cache_mut`）。
pub fn render_contribution_visual(
    visual: &VisualRenderer,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let mut rendered = None;
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        let cache = global.0.entity_cache_mut();
        let mut ctx = ContributionRenderContext {
            window,
            cx,
            active: true,
        };
        rendered = Some(visual(&mut ctx, cache));
    });
    rendered
}
