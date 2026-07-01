//! 贡献注册：`Registerable` 将数据/视图贡献统一为 `ContributedEntry`

use std::sync::Arc;

use rml_core::contribution::{ContributedEntry, ContributionOptions, IContribution};

use super::entry::data_entry;

/// 可由 `ContributionRegistry::register` 统一注册的贡献类型
///
/// - 数据贡献：在模块内 `impl Registerable` + `data_registerable`，或 `#[contribute]` 宏
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

/// 视图贡献默认实现（`IVisualContribution` + `#[component]`）
pub fn visual_registerable<T>(contribution: Arc<T>, options: ContributionOptions) -> ContributedEntry
where
    T: rml_core::contribution::IVisualContribution + 'static,
    T::View: rml_core::component::IComponent
        + gpui::Render
        + Default
        + Send
        + Sync
        + 'static,
{
    super::entry::visual_entry(contribution, options)
}
