//! DI 容器构建 —— 注册运行时服务到 ServiceProvider。
//!
//! 解析方式: `cx.get_trait::<dyn IWorkbenchManager>()`（经 ServiceProviderExt）
//!
//! # 循环依赖解决
//!
//! `ArcShellManager` 需要 ServiceProvider 来解析 IWorkbenchProvider,
//! 而 ServiceProvider 需要 ArcShellManager 已注册才能构建。
//! 使用 `OnceLock` 二阶段注入:
//! 1. MainWindow::default() 创建 `Arc<ArcShellManager::new()>`(无 provider)
//! 2. `build_runtime_provider(manager)` 构建 ServiceCollection + ServiceProvider
//! 3. `manager.set_provider(provider.clone())`
//! 4. `cx.use_provider(provider)` 追加到 provider 链（configure 阶段的静态服务仍可解析）
//!
//! `IWorkbenchProvider` 由各扩展 crate 经 `#[ctor::ctor]` + `auto_register` 自注册,
//! 此处经 `apply_auto_registrations` 应用到运行时 provider。

use std::sync::Arc;

use rml_core::context::IServiceProvider;
use rml_core::workbench::IWorkbenchManager;
use rust_rml_di::{ServiceCollection, apply_auto_registrations};
use studio_core::workspace::IWorkspaceManager;

use crate::shell_manager::ArcShellManager;

/// 构建运行时 provider（ArcShellManager 二阶段注入）。
///
/// 接受外部已创建的 `Arc<ArcShellManager>`（由 `MainWindow::default()` 创建），
/// 将其注册为 `IWorkbenchManager` + `IWorkspaceManager` singleton,
/// 应用所有自动注册（`IWorkbenchProvider` 等）,
/// 构建 ServiceProvider 并经 `set_provider()` 反向注入到 manager。
///
/// 返回的 `Arc<dyn IServiceProvider + Send + Sync>` 经 `cx.use_provider()` 追加到 provider 链,
/// 业务代码经 `cx.get_trait::<dyn T>()` 解析。
pub fn build_runtime_provider(manager: Arc<ArcShellManager>) -> Arc<dyn IServiceProvider + Send + Sync> {
    let manager_for_wsm = manager.clone();
    let manager_for_wbm = manager.clone();

    let mut s = ServiceCollection::new();
    s.add_singleton::<dyn IWorkspaceManager>(move |_| {
        manager_for_wsm.clone() as Arc<dyn IWorkspaceManager>
    });
    s.add_singleton::<dyn IWorkbenchManager>(move |_| {
        manager_for_wbm.clone() as Arc<dyn IWorkbenchManager>
    });
    // 应用扩展 crate 经 #[ctor::ctor] + auto_register 自注册的服务
    // （EditorProvider("file") / WelcomeProvider("rml") 等 IWorkbenchProvider）
    apply_auto_registrations(&mut s);
    let provider = s.build();
    manager.set_provider(provider.clone());
    provider
}
