//! 贡献注册表——构建期回调与 host_id 路由
//!
//! 注册表实例本身存储在 `IServiceProvider`（通过 `IAppContext::get_service::<ContributionRegistry>()` 查询）。
//! 此模块仅保留 build.rs 生成的 `#[ctor::ctor]` 回调安装与 host_id 路由逻辑。

use std::sync::Mutex;

use gpui::App;

/// build.rs 生成的 `register_rml_contributions_for(cx, host_id)` 回调签名。
/// 按 host_id 路由调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*` 函数。
type ContributionBootstrapFn = fn(&mut App, &str);

/// 进程级回调：build.rs 生成的 `register_rml_contributions_for(cx, host_id)`。
static CONTRIBUTION_BOOTSTRAP: Mutex<Option<ContributionBootstrapFn>> = Mutex::new(None);

/// 由 build.rs 生成的 `#[ctor::ctor]` 函数调用，安装按 host_id 路由的注册回调。
pub fn install_contribution_bootstrap(f: fn(&mut App, &str)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

/// 由 host 的 `on_loaded` 手动调用：触发指定 host_id 的所有贡献注册。
///
/// 内部回调 build.rs 生成的 `register_rml_contributions_for(cx, host_id)`，
/// 该函数按 host_id match 调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*`。
pub fn bootstrap_host_contributions(cx: &mut App, host_id: &str) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx, host_id);
    }
}
