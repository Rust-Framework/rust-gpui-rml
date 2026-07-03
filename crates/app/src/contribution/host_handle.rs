//! Entity host 桥接器
//!
//! `Entity<T>` 不能直接 `Arc<dyn IContributionHost>`（`update` 需 `cx`）。
//! 提供 `EntityHostHandle<T>` + flume channel 桥接：
//! - `EntityHostHandle` 实现 `IContributionHost`，所有方法将操作入 channel
//! - Entity 持有 `Receiver<HostOp>`，在 `on_loaded`/render 中 drain 并分派到自身 `IContributionHost` 实现
//!
//! `#[contributehost]` 宏生成 `__rml_install_host`，内部调用 `install_entity_host`：
//! 1. 创建 `EntityHostHandle` 并 `add_host` 到 registry
//! 2. 调用 `bootstrap_host_contributions(cx, id)` 触发该 host_id 的所有贡献注册
//!    （同步：register_visual → handle.add_visual → tx.send）
//! 3. 返回 `Receiver<HostOp>` 供 Entity drain

use std::sync::Arc;

use gpui::{App, WeakEntity};
use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IVisualContribution,
};
use rml_core::flume;

use super::global::bootstrap_host_contributions;

/// Host 操作队列（Entity host 在 `on_loaded`/render 中 drain）。
pub enum HostOp {
    Add(Arc<dyn IContribution>, ContributionOptions),
    AddVisual(Arc<dyn IVisualContribution>, ContributionOptions),
    Remove(String),
}

/// Entity host 的 `IContributionHost` 桥接器。
///
/// 所有方法将操作入 channel，Entity 持有 `Receiver` 在 `update` 闭包内 drain。
/// `weak` 字段保留 `WeakEntity<T>` 以备未来扩展（如生命周期跟踪），当前未使用。
pub struct EntityHostHandle<T: 'static> {
    id: &'static str,
    #[allow(dead_code)]
    weak: WeakEntity<T>,
    tx: flume::Sender<HostOp>,
}

impl<T: 'static> IContributionHost for EntityHostHandle<T> {
    fn id(&self) -> &'static str {
        self.id
    }

    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions) {
        let _ = self.tx.send(HostOp::Add(contribution, options));
    }

    fn add_visual(
        &self,
        contribution: Arc<dyn IVisualContribution>,
        options: ContributionOptions,
    ) {
        let _ = self.tx.send(HostOp::AddVisual(contribution, options));
    }

    fn remove(&self, contribution_id: &str) {
        let _ = self.tx.send(HostOp::Remove(contribution_id.to_string()));
    }
}

/// 由 `#[contributehost]` 宏生成的 `__rml_install_host` 调用。
///
/// 注册 handle + 触发该 host_id 的所有贡献注册，返回 `Receiver` 供 Entity drain。
pub fn install_entity_host<T: IContributionHost + 'static>(
    id: &'static str,
    entity: gpui::Entity<T>,
    cx: &mut App,
) -> flume::Receiver<HostOp> {
    use super::global::ContributionRegistryExt;
    let (tx, rx) = flume::unbounded();
    let handle = EntityHostHandle {
        id,
        weak: entity.downgrade(),
        tx,
    };
    cx.get_contribution_registry().add_host(Arc::new(handle));
    // 触发该 host_id 的所有贡献注册（同步：register_visual → handle.add_visual → tx.send）
    bootstrap_host_contributions(cx, id);
    rx
}

/// Entity host 在 `on_loaded`/render 中调用：drain 接收到的操作，分派到自身 `IContributionHost` 实现。
///
/// 调用方应在 drain 后调 `cx.notify()` 触发重渲。
pub fn drain_host_ops<T: IContributionHost>(rx: &flume::Receiver<HostOp>, host: &T) {
    for op in rx.try_iter() {
        match op {
            HostOp::Add(c, o) => host.add(c, o),
            HostOp::AddVisual(c, o) => host.add_visual(c, o),
            HostOp::Remove(id) => host.remove(&id),
        }
    }
}
