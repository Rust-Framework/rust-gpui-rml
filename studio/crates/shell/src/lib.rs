//! Arc Studio IDE 主窗口外壳
//!
//! 此 crate 实现:
//! - [`di`] —— DI 容器构建(`ServiceCollection` → `ServiceProvider`),注册所有公共接口
//! - [`shell_manager`] —— `ArcShellManager` 纯逻辑(impl `IWorkbenchManager` + `IWorkspaceManager`)
//! - [`arc_shell`] —— `ArcShell` GPUI `#[window]`(待实现,需 RML 模板)

// 包名统一为 rust-rml-* 前缀,通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;
extern crate studio_core as studio_core;

pub mod di;
pub mod shell_manager;
