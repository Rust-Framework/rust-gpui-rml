//! 视觉贡献渲染（封装 registry entity cache 访问）

use gpui::{AnyElement, App, BorrowAppContext, Render, Window};
use rml_core::component::IComponent;
use rml_core::contribution::{
    ComponentEntityCache, IVisualContribution, RenderContext, VisualRenderer,
};

use super::global::ContributionRegistryGlobal;

/// 执行 `VisualRenderer` 并返回 GPUI element。
///
/// VisualRenderer 闭包内部通过 `IVisualContribution::render` 委托给
/// [`render_component_view`]，后者从全局 registry 取 Entity 缓存。
/// 应用层无需直接访问 `entity_cache_mut`。
pub fn render_contribution_visual(
    visual: &VisualRenderer,
    window: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    let mut ctx = RenderContext {
        window,
        cx,
        active: true,
    };
    Some(visual(&mut ctx))
}

/// 框架内部：渲染组件贡献视图（由宏生成的 `IVisualContribution::render` 调用）。
///
/// 从 `ContributionRegistryGlobal` 取共享 Entity 缓存，查找或创建 Entity，
/// 委托给 `ComponentEntityCache::render_view`。开发者不直接调用此函数。
pub fn render_component_view<T>(contribution: &T, ctx: &mut RenderContext) -> AnyElement
where
    T: IVisualContribution + IComponent + Render + Default + Send + Sync + 'static,
{
    let id = contribution.id().to_string();
    let active = ctx.active;
    ctx.cx
        .update_global::<ContributionRegistryGlobal, _>(|global, cx| {
            let cache = global.0.entity_cache_mut();
            let mut inner_ctx = RenderContext {
                window: ctx.window,
                cx,
                active,
            };
            cache.render_view(&id, T::default(), &mut inner_ctx)
        })
}
