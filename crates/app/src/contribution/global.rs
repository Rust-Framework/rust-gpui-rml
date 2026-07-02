//! GPUI Global 与 `ContributionExt`（`App` 统一贡献注册器）

use std::sync::{Arc, Mutex};

use gpui::{App, BorrowAppContext, Context, Global, Render};
use rml_core::contribution::{ContributedEntry, ContributionOptions, IContribution};

use super::registerable::Registerable;
use super::registry::ContributionRegistry;

static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App)>> = Mutex::new(None);

/// 由 build.rs 生成的 `#[ctor::ctor]` 函数调用，安装贡献点自动注册回调。
pub fn install_contribution_bootstrap(f: fn(&mut App)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

/// 若用户 crate 通过 `embed_contributions!` 提供了注册表，则执行一次。
pub fn bootstrap_contributions(cx: &mut App) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx);
    }
}

/// GPUI 全局贡献注册表（框架内部）
#[doc(hidden)]
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

/// 统一贡献注册器：扩展 `App`。
pub trait ContributionExt {
    /// 注册贡献点主机 slot（`#[contributehost]` 生成代码调用）。
    fn add(&mut self, host_id: &str);

    /// 移除 host 及其全部贡献条目。
    fn remove(&mut self, host_id: &str);

    /// 向 host 注册数据贡献。
    fn register(
        &mut self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    );

    /// 从 host 注销贡献；返回是否移除成功。
    fn unregister(&mut self, host_id: &str, contribution_id: &str) -> bool;

    /// 只读访问内部注册表（读取 `entries` / `revision` 等）。
    fn contribution_registry(&self) -> &ContributionRegistry;
}

impl ContributionExt for App {
    fn add(&mut self, host_id: &str) {
        ensure_contribution_registry(self);
        self.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.ensure_host(host_id);
        });
    }

    fn remove(&mut self, host_id: &str) {
        if !self.has_global::<ContributionRegistryGlobal>() {
            return;
        }
        self.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.remove_host(host_id);
        });
    }

    fn register(
        &mut self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    ) {
        ensure_contribution_registry(self);
        self.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
            global.0.register_dyn(host_id, contribution, options, cx);
        });
    }

    fn unregister(&mut self, host_id: &str, contribution_id: &str) -> bool {
        if !self.has_global::<ContributionRegistryGlobal>() {
            return false;
        }
        self.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
            global.0.unregister(host_id, contribution_id, cx)
        })
    }

    fn contribution_registry(&self) -> &ContributionRegistry {
        &self.global::<ContributionRegistryGlobal>().0
    }
}

/// 类型化注册入口（`#[contribute]` + `Registerable` 使用）。
pub fn register_contribution<T>(
    cx: &mut App,
    host_id: &str,
    contribution: Arc<T>,
    options: ContributionOptions,
) where
    T: Registerable + 'static,
{
    ensure_contribution_registry(cx);
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global.0.register_typed(host_id, contribution, options, cx);
    });
}

/// 读取 host 条目（`Context` 侧便捷函数）。
pub fn contribution_entries<'a, C>(host_id: &str, cx: &'a Context<C>) -> &'a [ContributedEntry] {
    if !cx.has_global::<ContributionRegistryGlobal>() {
        return &[];
    }
    cx.global::<ContributionRegistryGlobal>().0.entries(host_id)
}

/// 读取 host 条目版本（供 `#[computed]` 或缓存键）。
pub fn contribution_revision<C>(host_id: &str, cx: &Context<C>) -> u64 {
    if !cx.has_global::<ContributionRegistryGlobal>() {
        return 0;
    }
    cx.global::<ContributionRegistryGlobal>()
        .0
        .revision(host_id)
}

/// 订阅 host 贡献变更（统一通知通道；`ActivityPanel` 等组件使用）。
pub fn subscribe_host_changes<C, F>(host_id: &str, cx: &mut Context<C>, listener: F)
where
    C: Render + 'static,
    F: Fn(&mut C, &mut Context<C>) + Send + Sync + 'static,
{
    let weak = cx.weak_entity();
    cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
        global.0.subscribe_host(
            host_id,
            Box::new(move |app| {
                if let Some(entity) = weak.upgrade() {
                    entity.update(app, |this, cx| {
                        listener(this, cx);
                    });
                }
            }),
        );
    });
}
