//! 贡献注册表 App 扩展与 host 注册
//!
//! 框架内部：维护进程级 `ContributionRegistry` 静态实例；提供 `get_contribution_registry()`
//! 扩展方法供宏生成代码与业务代码统一操作。host 通过 `register_host(cx)` 注册自身。
//!
//! 注册表采用 `OnceLock` 进程级静态存储（而非 GPUI Global），使 `get_contribution_registry()`
//! 返回 `&'static` 引用，避免与 `&mut App` 参数的借用冲突。

use std::sync::{Arc, Mutex, OnceLock};

use gpui::{App, Context, Render};
use rml_core::contribution::{
    ContributionOptions, HostHandle, IContribution, IContributionHost, IContributionRegistry,
};

use super::registry::ContributionRegistry;

static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App)>> = Mutex::new(None);

static REGISTRY: OnceLock<ContributionRegistry> = OnceLock::new();

/// 进程级 `ContributionRegistry` 静态实例（内部 RwLock 保证可变性）
fn registry() -> &'static ContributionRegistry {
    REGISTRY.get_or_init(ContributionRegistry::new)
}

/// 由 build.rs 生成的 `#[ctor::ctor]` 函数调用，安装贡献点自动注册回调。
pub fn install_contribution_bootstrap(f: fn(&mut App)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

/// 触发 `register_rml_contributions(cx)` 执行，将所有 `#[contribute]` 注册到 registry。
/// 在 `RmlApplication::new` 中调用——host 未创建时入 pending 队列。
pub fn bootstrap_contributions(cx: &mut App) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx);
    }
}

/// 确保全局注册表已初始化（兼容旧调用点；实际由 `OnceLock::get_or_init` 自动初始化）
pub fn ensure_contribution_registry(_cx: &mut App) {
    let _ = registry();
}

/// App 扩展：获取 `IContributionRegistry` 接口。
/// 返回 `&'static` 引用——不借用 `App`，避免与 `register(..., cx)` 的 `&mut App` 参数冲突。
pub trait ContributionRegistryExt {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry {
        registry()
    }
}

/// host 在 on_loaded 中调用：注册自身为贡献 host。
/// registry 会重放此前通过 `#[ctor::ctor]` 注册的 pending 贡献到 host.add。
pub fn register_host<T>(cx: &mut Context<T>)
where
    T: IContributionHost + Render + 'static,
{
    let weak = cx.weak_entity();
    registry().add(Box::new(EntityHostHandleBox::<T> { weak }), cx);
}

/// 内部桥接：将 `WeakEntity<T>` 包装为 `HostHandle` trait 实现的盒子。
/// 在 app 层定义（依赖 gpui `Render` bound，core 层不依赖 gpui render）。
#[doc(hidden)]
pub struct EntityHostHandleBox<T>
where
    T: IContributionHost + Render + 'static,
{
    weak: gpui::WeakEntity<T>,
}

impl<T> HostHandle for EntityHostHandleBox<T>
where
    T: IContributionHost + Render + 'static,
{
    fn id(&self) -> &str {
        T::ID
    }

    fn add(
        &self,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
        cx: &mut App,
    ) {
        if let Some(entity) = self.weak.upgrade() {
            // 延迟到当前 update 结束后执行，避免在 host 的 on_loaded → register_host →
            // pending replay 路径上产生嵌套 entity.update（GPUI 不允许同一 entity 重复可变借用）
            cx.defer(move |cx| {
                entity.update(cx, |host, ctx| {
                    host.add(contribution, options, ctx);
                    ctx.notify();
                });
            });
        }
    }

    fn remove(&self, contribution_id: &str, cx: &mut App) {
        if let Some(entity) = self.weak.upgrade() {
            let contribution_id = contribution_id.to_string();
            cx.defer(move |cx| {
                entity.update(cx, |host, ctx| {
                    host.remove(&contribution_id, ctx);
                    ctx.notify();
                });
            });
        }
    }
}
