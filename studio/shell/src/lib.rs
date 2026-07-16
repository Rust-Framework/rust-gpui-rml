//! Arc Studio IDE 主窗口外壳
//!
//! 此 crate 实现:
//! - [`di`] —— DI 容器构建(`ServiceCollection` → `ServiceProvider`),注册所有公共接口
//! - [`shell_manager`] —— `ArcShellManager` 纯逻辑(impl `IWorkbenchManager` + `IWorkspaceManager`)
//! - [`welcome_workbench`] —— `WelcomeWorkbench`(`rml://welcome` 工作台)
//! - [`welcome_provider`] —— `WelcomeProvider`(`rml://` URI 工作台工厂)
//! - [`main_window`] —— `MainWindow` GPUI `#[window]` 主窗口
//!
//! 服务自注册: `#[ctor::ctor]` 自动注册 `WelcomeProvider` 为 `IWorkbenchProvider("rml")`,
//! 并注册 `WelcomeWorkbench` 的能力 cast（IContribution + IVisual + IWorkbench）。

// 包名统一为 rust-rml-* 前缀,通过 extern crate 别名保留源码中的短名引用
// `rml` 别名是 RML 宏(`#[window]`/`#[computed]`/`#[command]`)展开后生成代码的约定
extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;
extern crate studio_core as studio_core;

#[path = "main_window.rml.rs"]
pub mod main_window;
pub mod di;
pub mod menu_commands;
pub mod menu_view_model;
pub mod shell_manager;
pub mod status_items;
pub mod status_view_model;
#[path = "welcome_workbench.rml.rs"]
pub mod welcome_workbench;
pub mod welcome_provider;

pub use main_window::MainWindow;

// 引入 build.rs 生成的贡献注册代码（`#[contribute]` 宏生成的 `__rml_register_*` 函数路由）。
// `#[rml::main]` 宏会自动为 bin crate 注入此宏，但 lib crate 需手动调用。
rml::embed_contributions!();

/// 自动注册 —— `#[ctor::ctor]` 在 `main` 之前执行:
/// 1. 注册能力 cast（IContribution + IVisual + IWorkbench）
/// 2. 注册 `WelcomeProvider` 为 keyed `IWorkbenchProvider("rml")`
#[rml_core::ctor::ctor]
fn register_welcome_services() {
    use std::sync::Arc;
    use rml_core::workbench::IWorkbenchProvider;
    use studio_core::di::{auto_register, ServiceCollection, ServiceCollectionExt};

    crate::welcome_workbench::register_welcome_abilities();
    auto_register(|s: &mut ServiceCollection| {
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("rml", |_| {
            Arc::new(crate::welcome_provider::WelcomeProvider) as Arc<dyn IWorkbenchProvider>
        });
    });
}
