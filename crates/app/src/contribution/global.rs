//! GPUI Global 与 `ContributionExt`

use gpui::{App, BorrowAppContext, Global};

use super::registry::ContributionRegistry;

/// GPUI 全局贡献注册表
pub struct ContributionRegistryGlobal(pub ContributionRegistry);

impl Global for ContributionRegistryGlobal {}

/// 确保全局注册表已初始化
pub fn ensure_contribution_registry(cx: &mut App) {
    if cx.has_global::<ContributionRegistryGlobal>() {
        return;
    }
    let registry = ContributionRegistry::new();
    cx.set_global(ContributionRegistryGlobal(registry));
}

/// 访问贡献注册表的扩展 trait
pub trait ContributionExt {
    fn with_contribution_registry<R>(&mut self, f: impl FnOnce(&mut ContributionRegistry) -> R) -> R;

    fn contribution_registry(&self) -> &ContributionRegistry;
}

impl ContributionExt for App {
    fn with_contribution_registry<R>(&mut self, f: impl FnOnce(&mut ContributionRegistry) -> R) -> R {
        self.update_global::<ContributionRegistryGlobal, R>(|global, _| f(&mut global.0))
    }

    fn contribution_registry(&self) -> &ContributionRegistry {
        &self.global::<ContributionRegistryGlobal>().0
    }
}
