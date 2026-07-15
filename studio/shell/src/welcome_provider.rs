//! WelcomeProvider —— `rml://` URI 的工作台工厂。
//!
//! 经 `ArcShellManager::open` 路由,为 `rml://welcome` 构造 WelcomeWorkbench 外部实例。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::IContribution;
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri};

use crate::welcome_workbench::WelcomeWorkbench;

/// `rml://` URI 的工作台工厂 —— 构造 WelcomeWorkbench。
pub struct WelcomeProvider;

impl IContribution for WelcomeProvider {
    fn id(&self) -> &str {
        "welcome-provider"
    }
    fn name(&self) -> SharedString {
        "Welcome Provider".into()
    }
}

impl IWorkbenchProvider for WelcomeProvider {
    fn schema(&self) -> SharedString {
        "rml".into()
    }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        Arc::new(WelcomeWorkbench::new(uri.as_str().into()))
    }
}
