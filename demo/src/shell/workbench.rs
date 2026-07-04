//! Workbench 实现 —— IWorkbench 实例与 LSP 工厂。
//!
//! - `CaseWorkbench` / `LspWorkbench`：`IWorkbench + IContribution + IVisualContribution`
//!   三 trait impl，供 MainWindow 经 `as_visual()` 渲染。
//! - `LspWorkbenchProvider`（`IWorkbenchProvider`）：处理 `lsp://` URI。
//!
//! `IWorkbenchManager` 由 MainWindow 直接实现，管理 `Vec<Arc<dyn IWorkbench>>` + 激活态。
//! Tab/资源生命周期（open/close/activate）经 `IWorkbenchManager` trait 方法驱动，
//! 状态存储在 MainWindow 的 `RwLock` 字段中。

use std::any::Any;
use std::sync::{Arc, Once, RwLock};

use gpui::{AnyElement, App, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::{
    register_contribution_ability, register_visual_ability, IContribution, IVisualContribution,
};
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri};

use crate::lsp::{CodeEditorTab, LspClient};
use crate::shell::case_view_model::CaseViewModel;

// ──────────────────────────────────────────────────────────────────────────
//  能力注册：CaseWorkbench / LspWorkbench 需注册 IContribution + IVisualContribution
//  能力 cast，使 MainWindow 的 `as_visual()` 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub(crate) fn register_workbench_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<CaseWorkbench>();
        register_visual_ability::<CaseWorkbench>();
        register_contribution_ability::<LspWorkbench>();
        register_visual_ability::<LspWorkbench>();
    });
}

// ──────────────────────────────────────────────────────────────────────────
//  CaseWorkbench：rml:// URI 的工作台，包装 CaseViewModel
// ──────────────────────────────────────────────────────────────────────────

/// `rml://{case_id}` URI 的工作台。包装 `CaseViewModel`，委托 render。
pub struct CaseWorkbench {
    uri: SharedString,
    case: CaseViewModel,
}

impl CaseWorkbench {
    pub fn new(uri: SharedString, case: CaseViewModel) -> Self {
        Self { uri, case }
    }
}

impl IContribution for CaseWorkbench {
    fn id(&self) -> &str {
        &self.case.id
    }
    fn name(&self) -> SharedString {
        self.case.contribution_name()
    }
}

impl IVisualContribution for CaseWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.case.render(window, cx)
    }
}

impl IWorkbench for CaseWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}
}

// ──────────────────────────────────────────────────────────────────────────
//  LspWorkbench：lsp:// URI 的工作台，懒加载 CodeEditorTab Entity
// ──────────────────────────────────────────────────────────────────────────

/// `lsp://{relative_path}` URI 的工作台。
///
/// `CodeEditorTab` Entity 在首次 `render` 时创建（此时有 `&mut Window, &mut App`），
/// 后续调用复用缓存 Entity。
pub struct LspWorkbench {
    uri: SharedString,
    title: SharedString,
    lsp_client: Option<Arc<LspClient>>,
    tab: RwLock<Option<Entity<CodeEditorTab>>>,
}

impl LspWorkbench {
    pub fn new(uri: SharedString, title: SharedString, lsp_client: Option<Arc<LspClient>>) -> Self {
        Self {
            uri,
            title,
            lsp_client,
            tab: RwLock::new(None),
        }
    }
}

impl IContribution for LspWorkbench {
    fn id(&self) -> &str {
        &self.uri
    }
    fn name(&self) -> SharedString {
        self.title.clone()
    }
}

impl IVisualContribution for LspWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let mut tab_lock = self.tab.write().unwrap();
        if tab_lock.is_none() {
            if let Some(client) = self.lsp_client.clone() {
                let relative_path = self
                    .uri
                    .strip_prefix("lsp://")
                    .unwrap_or(&self.uri)
                    .to_string();
                let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src")
                    .join(&relative_path);
                let tab = CodeEditorTab::new(&relative_path, &full_path, client, window, cx);
                *tab_lock = Some(tab);
            }
        }
        match tab_lock.as_ref() {
            Some(tab) => tab.update(cx, |tab, cx| tab.render(window, cx).into_any_element()),
            None => gpui::div().into_any_element(),
        }
    }
}

impl IWorkbench for LspWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}
}

// ──────────────────────────────────────────────────────────────────────────
//  LspWorkbenchProvider：schema="lsp"，构造 LspWorkbench
// ──────────────────────────────────────────────────────────────────────────

/// `lsp://` URI 的 workbench 工厂。
///
/// `LspWorkbench` 的 `CodeEditorTab` Entity 延迟到首次 `render` 时创建——
/// `IWorkbenchProvider::render` 无 window/cx 参数，无法创建 Entity。
pub struct LspWorkbenchProvider {
    lsp_client: Option<Arc<LspClient>>,
}

impl LspWorkbenchProvider {
    pub fn new(lsp_client: Option<Arc<LspClient>>) -> Self {
        Self { lsp_client }
    }

    /// 构造 LspWorkbench（inherent 方法，避免与 trait `render` 同名阴影）。
    pub(crate) fn build_workbench(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let relative_path = uri
            .as_str()
            .strip_prefix("lsp://")
            .unwrap_or(uri.as_str())
            .to_string();
        let title = std::path::Path::new(&relative_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&relative_path)
            .into();
        Arc::new(LspWorkbench::new(
            uri.as_str().into(),
            title,
            self.lsp_client.clone(),
        ))
    }
}

impl IContribution for LspWorkbenchProvider {
    fn id(&self) -> &str {
        "lsp-provider"
    }
    fn name(&self) -> SharedString {
        "LSP Provider".into()
    }
}

impl IWorkbenchProvider for LspWorkbenchProvider {
    fn schema(&self) -> SharedString {
        "lsp".into()
    }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        self.build_workbench(uri)
    }
}
