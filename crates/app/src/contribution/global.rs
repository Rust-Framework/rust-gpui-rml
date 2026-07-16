//! 贡献注册表——构建期回调与 host_id 路由
//!
//! 注册表实例本身作为 GPUI Global 存储（`cx.set_global(Arc::new(...))`），
//! 经 `IAppContextExt::get_contribution_registry` 访问（不经过 IServiceProvider）。
//! 此模块仅保留 build.rs 生成的 `#[ctor::ctor]` 回调安装与 host_id 路由逻辑。

use std::sync::Mutex;

use gpui::App;

/// build.rs 生成的 `register_rml_contributions_for(cx, host_id)` 回调签名。
/// 按 host_id 路由调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*` 函数。
type ContributionBootstrapFn = fn(&mut App, &str);

/// 进程级回调列表：各 crate build.rs 生成的 `register_rml_contributions_for(cx, host_id)`。
/// 多 crate 可各自注册（如 studio-shell 注册菜单贡献，arc-studio 无贡献时注册空函数），
/// `bootstrap_host_contributions` 会逐一调用，按 host_id match 路由。
static CONTRIBUTION_BOOTSTRAP: Mutex<Vec<ContributionBootstrapFn>> = Mutex::new(Vec::new());

/// 由 build.rs 生成的 `#[ctor::ctor]` 函数调用，安装按 host_id 路由的注册回调。
/// 支持多 crate 各自注册：同一 host_id 的贡献可分布在不同 crate 中。
pub fn install_contribution_bootstrap(f: fn(&mut App, &str)) {
    CONTRIBUTION_BOOTSTRAP.lock().unwrap().push(f);
}

/// 由 host 的 `on_loaded` 手动调用：触发指定 host_id 的所有贡献注册。
///
/// 逐一调用所有已注册的 `register_rml_contributions_for(cx, host_id)`，
/// 每个函数按 host_id match 调用对应 `#[contribute(host_id = "...")]` 的 `__rml_register_*`。
pub fn bootstrap_host_contributions(cx: &mut App, host_id: &str) {
    let fns = CONTRIBUTION_BOOTSTRAP.lock().unwrap().clone();
    for f in fns {
        f(cx, host_id);
    }
}
