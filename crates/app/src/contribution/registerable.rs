//! 贡献注册：`Registerable` 将数据/组件贡献统一为 `ContributedEntry`

use std::sync::Arc;

use gpui::Render;
use rml_core::component::IComponent;
use rml_core::contribution::{ContributedEntry, ContributionOptions, IContribution};

use super::entry::{component_entry, data_entry};

/// 可由 `ContributionRegistry::register` 统一注册的贡献类型
pub trait Registerable: IContribution + Sized {
    fn into_entry(contribution: Arc<Self>, options: ContributionOptions) -> ContributedEntry
    where
        Self: 'static;
}

/// 数据贡献默认实现
pub fn data_registerable<T: IContribution + 'static>(
    contribution: Arc<T>,
    options: ContributionOptions,
) -> ContributedEntry {
    data_entry(contribution, options)
}

/// 组件贡献默认实现（`#[component]` 类型即 visual 面板）
pub fn component_registerable<T>(
    contribution: Arc<T>,
    options: ContributionOptions,
) -> ContributedEntry
where
    T: IContribution + IComponent + Render + Default + Send + Sync + 'static,
{
    component_entry(contribution, options)
}
