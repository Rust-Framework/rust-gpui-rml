//! 贡献点运行时：Registry + 视觉 Entity 缓存
//!
//! 框架内部模块，业务代码通过 `rml_app::prelude::*` 或具体导入使用。
//! `ContributionRegistry` 与 `VisualEntityCache` 均作为 GPUI Global 存储（不经过 IServiceProvider），
//! 与 i18n/theme 范式对齐。Host 在 `on_loaded` 中调 `cx.register_host(id, self.entries.clone())` +
//! `bootstrap_host_contributions(cx, id)` 注册自身。Registry 存 `Arc<dyn IContributionHost>`
//! （`entries.clone()` 经 unsized coercion 转入），`register` 时调 `host.add(c, opts)` 路由，不经 Entity 系统。

mod entity_cache;
mod global;
mod registry;

pub use entity_cache::{
    evict_entity_by_uri, get_active_entity, get_or_create_entity, get_or_create_entity_by_uri,
    visual_entity, VisualEntityCache,
};
pub use global::{bootstrap_host_contributions, install_contribution_bootstrap};
pub use registry::ContributionRegistry;
