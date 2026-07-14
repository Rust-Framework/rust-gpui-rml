//! EditorProvider —— "file://" URI 的工作台工厂。
//!
//! 经 DI 注册为 keyed `IWorkbenchProvider`(key="file")，
//! `ArcShellManager::open` 按 schema 路由到此 provider 构造 `EditorWorkbench`。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::IContribution;
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri};

use crate::editor_workbench::EditorWorkbench;

/// "file://" URI 的工作台工厂 —— 构造 EditorWorkbench。
pub struct EditorProvider;

impl IContribution for EditorProvider {
    fn id(&self) -> &str {
        "editor-provider"
    }
    fn name(&self) -> SharedString {
        "Editor Provider".into()
    }
}

impl IWorkbenchProvider for EditorProvider {
    fn schema(&self) -> SharedString {
        "file".into()
    }

    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let file_path = uri
            .to_file_path()
            .unwrap_or_else(|_| std::path::PathBuf::from(uri.path()));

        let mut wb = EditorWorkbench::default();
        wb.set_file(uri.as_str().into(), file_path);
        Arc::new(wb)
    }
}