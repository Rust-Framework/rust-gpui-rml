//! DI 容器构建 —— 注册运行时服务到 ServiceProvider。
//!
//! 基于 `studio_core::di`（rust-dix → IServiceProvider 桥接）。
//! 解析方式: `cx.get_service::<dyn IWorkbenchManager>()`（`IServiceProvider::get_service<T: ?Sized>`）
//!
//! # 循环依赖解决
//!
//! `ArcShellManager` 需要 ServiceProvider 来解析 IWorkbenchProvider,
//! 而 ServiceProvider 需要 ArcShellManager 已注册才能构建。
//! 使用 `OnceLock` 二阶段注入:
//! 1. MainWindow::default() 创建 `Arc<ArcShellManager::new()>`(无 provider)
//! 2. `build_runtime_provider(manager)` 构建 ServiceCollection + ServiceProvider
//! 3. `manager.set_provider(provider.clone())`
//! 4. `cx.set_provider(provider)` 注入为正式 provider
//!
//! `IWorkbenchProvider` 等服务由各扩展 crate 经 `#[ctor::ctor]` +
//! `studio_core::di::auto_register` 自注册,此处经 `apply_auto_registrations` 应用。

use std::sync::Arc;

use rml_core::context::IServiceProvider;
use rml_core::workbench::IWorkbenchManager;
use studio_core::di::{DixServiceProvider, ServiceCollection, ServiceCollectionExt, apply_auto_registrations};
use studio_core::workspace::IWorkspaceManager;

use crate::shell_manager::ArcShellManager;

/// 构建运行时 provider（ArcShellManager 二阶段注入）。
///
/// 接受外部已创建的 `Arc<ArcShellManager>`（由 `MainWindow::default()` 创建），
/// 将其注册为 `IWorkbenchManager` + `IWorkspaceManager` singleton,
/// 应用所有自动注册（`IWorkbenchProvider` / `IChatManager` 等）,
/// 构建 ServiceProvider 并经 `set_provider()` 反向注入到 manager。
///
/// 返回的 `Arc<dyn IServiceProvider + Send + Sync>` 经 `cx.set_provider()` 注入为正式 provider,
/// 业务代码经 `cx.get_service::<dyn T>()` 解析。
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
    // （EditorProvider("file") / WelcomeProvider("rml") / ChatManager / ChatWorkbenchProvider("chat") 等）
    apply_auto_registrations(&mut s);
    let provider = DixServiceProvider::build(s);
    manager.set_provider(provider.clone());
    provider
}
