//! 贡献注册表 App 扩展
//!
//! 框架内部：维护进程级 `ContributionRegistry` 静态实例；提供 `get_contribution_registry()`
//! 扩展方法供宏生成代码与业务代码统一操作。
//!
//! 注册表采用 `OnceLock` 进程级静态存储（而非 GPUI Global），使 `get_contribution_registry()`
//! 返回 `&'static` 引用，所有方法 `&self` + 内部 `RwLock` 可变性。

use std::sync::{Mutex, OnceLock};

use gpui::App;
use rml_core::contribution::IContributionRegistry;

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
/// 返回 `&'static` 引用——不借用 `App`，所有方法 `&self` + 内部 `RwLock` 可变性。
pub trait ContributionRegistryExt {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry {
        registry()
    }
}
