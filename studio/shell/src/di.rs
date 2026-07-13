//! DI 容器构建 —— 注册所有公共接口到 rust-dix ServiceProvider。
//!
//! 解析方式: `cx.get_service::<Arc<ServiceProvider>>()?.get::<dyn IWorkbenchManager>()`
//!
//! # 循环依赖解决
//!
//! `ArcShellManager` 需要 ServiceProvider 来解析 IWorkbenchProvider,
//! 而 ServiceProvider 需要 ArcShellManager 已注册才能构建。
//! 使用 `OnceLock` 二阶段注入:
//! 1. MainWindow::default() 创建 `Arc<ArcShellManager::new()>`(无 provider)
//! 2. `build_provider(manager)` 构建 ServiceCollection + ServiceProvider
//! 3. `manager.set_provider(provider.clone())`

use std::sync::Arc;

use rml_core::workbench::{IWorkbenchManager, IWorkbenchProvider};
use rust_dix::{ServiceCollection, ServiceProvider};
use studio_core::workspace::IWorkspaceManager;

use crate::shell_manager::ArcShellManager;
use crate::welcome::WelcomeProvider;

/// 构建 DI 容器,注册所有公共接口。
///
/// 接受外部已创建的 `Arc<ArcShellManager>`(由 `MainWindow::default()` 创建),
/// 将其注册为 `IWorkbenchManager` + `IWorkspaceManager` singleton,
/// 构建 ServiceProvider 并经 `set_provider()` 反向注入到 manager。
///
/// 返回的 `Arc<ServiceProvider>` 经 `cx.set_service()` 注册到 IAppContext,
/// 业务代码经 `cx.get_service::<Arc<ServiceProvider>>()` 解析。
pub fn build_provider(manager: Arc<ArcShellManager>) -> anyhow::Result<Arc<ServiceProvider>> {
    let manager_for_wbm = manager.clone();
    let manager_for_wsm = manager.clone();

    let provider = ServiceCollection::new()
        // 管理器(singleton,同一实例实现两个 trait)
        .singleton::<dyn IWorkbenchManager>(move |_| {
            manager_for_wbm.clone() as Arc<dyn IWorkbenchManager>
        })
        .singleton::<dyn IWorkspaceManager>(move |_| {
            manager_for_wsm.clone() as Arc<dyn IWorkspaceManager>
        })
        // WelcomeProvider:keyed "rml" —— ArcShellManager::open 经 schema 路由
        .keyed_singleton::<dyn IWorkbenchProvider>("rml", move |_| {
            Arc::new(WelcomeProvider) as Arc<dyn IWorkbenchProvider>
        })
        .build()?;

    manager.set_provider(provider.clone());
    Ok(provider)
}
