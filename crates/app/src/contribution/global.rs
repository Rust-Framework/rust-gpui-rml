//! 贡献注册表 App 扩展
//!
//! 框架内部：维护进程级 `ContributionRegistry` 静态实例；提供 `get_contribution_registry()`
//! 扩展方法供宏生成代码与业务代码统一操作。
//!
//! 注册表采用 `OnceLock` 进程级静态存储（而非 GPUI Global），使 `get_contribution_registry()`
//! 返回 `&'static` 引用，所有方法 `&self` + 内部 `RwLock` 可变性。
//!
//! 贡献注册由 host 在 `on_loaded` 中触发：`__rml_install_host` 调用 `bootstrap_host_contributions`
//! 回调 build.rs 生成的 `register_rml_contributions_for(cx, host_id)`，按 host_id 分组注册。

use std::sync::{Mutex, OnceLock};

use gpui::App;
use rml_core::contribution::IContributionRegistry;

use super::registry::ContributionRegistry;

/// 进程级回调：build.rs 生成的 `register_rml_contributions_for(cx, host_id)`。
/// 按 host_id 路由调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*` 函数。
static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App, &str)>> = Mutex::new(None);

static REGISTRY: OnceLock<ContributionRegistry> = OnceLock::new();

fn registry() -> &'static ContributionRegistry {
    REGISTRY.get_or_init(ContributionRegistry::new)
}

/// 由 build.rs 生成的 `#[ctor::ctor]` 函数调用，安装按 host_id 路由的注册回调。
pub fn install_contribution_bootstrap(f: fn(&mut App, &str)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

/// 由 `__rml_install_host` 调用：触发指定 host_id 的所有贡献注册。
///
/// 内部回调 build.rs 生成的 `register_rml_contributions_for(cx, host_id)`，
/// 该函数按 host_id match 调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*`。
pub fn bootstrap_host_contributions(cx: &mut App, host_id: &str) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx, host_id);
    }
}

/// 确保全局注册表已初始化（兼容旧调用点；实际由 `OnceLock::get_or_init` 自动初始化）
pub fn ensure_contribution_registry(_cx: &mut App) {
    let _ = registry();
}

/// App 扩展：获取 `IContributionRegistry` 接口。
/// 返回 `&'static` 引用——不借用 `App`，所有方法 `&self` + 内部 `RwLock` 可变性。
pub trait ContributionRegistryExt {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry {
        registry()
    }
}
