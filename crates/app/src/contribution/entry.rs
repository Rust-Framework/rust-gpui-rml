//! `ContributedEntry` 构建：数据贡献 vs 组件 visual 贡献

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{App, Render};
use rml_core::component::IComponent;
use rml_core::contribution::{
    ComponentEntityCache, ContributedEntry, ContributionOptions, IContribution, VisualRenderer,
};

use rml_core::contribution_cache::ComponentEntityCacheImpl;

use super::host::ContributionHost;

/// 纯数据贡献条目
pub fn data_entry<T: IContribution + 'static>(
    contribution: Arc<T>,
    options: ContributionOptions,
) -> ContributedEntry {
    ContributedEntry {
        contribution: contribution as Arc<dyn IContribution>,
        visual: None,
        options,
    }
}

/// 组件贡献条目：`#[contribute]` + `#[component]` 时组件本身即面板 UI
pub fn component_entry<T>(contribution: Arc<T>, options: ContributionOptions) -> ContributedEntry
where
    T: IContribution + IComponent + Render + Default + Send + Sync + 'static,
{
    let id = contribution.id().to_string();
    let renderer: VisualRenderer = Arc::new(move |ctx, cache: &mut ComponentEntityCacheImpl| {
        cache.render_view(&id, T::default(), ctx)
    });
    ContributedEntry {
        contribution: contribution as Arc<dyn IContribution>,
        visual: Some(renderer),
        options,
    }
}

/// `Arc<dyn IContribution>` 数据注册入口
pub fn data_entry_dyn(
    contribution: Arc<dyn IContribution>,
    options: ContributionOptions,
) -> ContributedEntry {
    ContributedEntry {
        contribution,
        visual: None,
        options,
    }
}

/// 将条目加入 host（内部复用）
pub fn add_entry(
    hosts: &mut HashMap<String, ContributionHost>,
    host_id: &str,
    entry: ContributedEntry,
    cx: &mut App,
) {
    if let Some(host) = hosts.get_mut(host_id) {
        host.add(entry, cx);
    } else {
        eprintln!("rml: unknown contribution host_id: {host_id}");
    }
}
