//! IWorkbenchManager 实现 —— Tab/资源生命周期从 MainWindow 迁入。
//!
//! - `DemoWorkbenchManager`（`IWorkbenchManager`）：按 URI schema 路由，
//!   `rml://` 直接查 cases，`lsp://` 委托 `LspWorkbenchProvider`。
//!   维护 `Vec<Arc<dyn IWorkbench>>` + 激活态。
//! - `LspWorkbenchProvider`（`IWorkbenchProvider`）：处理 `lsp://` URI。
//! - `CaseWorkbench` / `LspWorkbench`：`IWorkbench + IContribution + IVisualContribution`
//!   三 trait impl，供 MainWindow 经 `as_visual()` 渲染。
//!
//! `IWorkbench: IContribution`，manager 直接存储 `Arc<dyn IWorkbench>`，
//! 通过 `as_visual()` 查询 render，通过 `uri()` 去重与查找，无需枚举桥接。

use std::any::Any;
use std::sync::{Arc, Once, RwLock};

use gpui::{AnyElement, App, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::{
    register_contribution_ability, register_visual_ability, IContribution, IVisualContribution,
    VisualAbilityExt,
};
use rml_core::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};

use crate::lsp::{CodeEditorTab, LspClient};
use crate::shell::case_view_model::CaseViewModel;

// ──────────────────────────────────────────────────────────────────────────
//  能力注册：CaseWorkbench / LspWorkbench 需注册 IContribution + IVisualContribution
//  能力 cast，使 MainWindow 的 `as_visual()` 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

fn register_workbench_abilities() {
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
                let full_path = std::env::current_dir()
                    .unwrap_or_default()
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
    fn build_workbench(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let relative_path = uri.path().trim_start_matches('/').to_string();
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

// ──────────────────────────────────────────────────────────────────────────
//  DemoWorkbenchManager：IWorkbenchManager 实现
// ──────────────────────────────────────────────────────────────────────────

/// demo 工作台管理器：按 URI schema 路由，维护 `Vec<Arc<dyn IWorkbench>>` + 激活态。
///
/// `rml://` schema 直接查 `cases` 集合（单一数据源，无 provider 中转）；
/// `lsp://` schema 委托 `LspWorkbenchProvider`（无状态工厂，无需数据同步）。
///
/// `IWorkbench: IContribution`，manager 直接存储 `Arc<dyn IWorkbench>`，
/// 通过 `as_visual()` 查询 render，通过 `uri()` 去重与查找。
pub struct DemoWorkbenchManager {
    workbenches: RwLock<Vec<Arc<dyn IWorkbench>>>,
    activated: RwLock<Option<Arc<dyn IWorkbench>>>,
    cases: RwLock<Vec<CaseViewModel>>,
    lsp_provider: Arc<LspWorkbenchProvider>,
}

impl DemoWorkbenchManager {
    pub fn new(lsp_client: Option<Arc<LspClient>>) -> Self {
        register_workbench_abilities();
        Self {
            workbenches: RwLock::new(Vec::new()),
            activated: RwLock::new(None),
            cases: RwLock::new(Vec::new()),
            lsp_provider: Arc::new(LspWorkbenchProvider::new(lsp_client)),
        }
    }

    /// 同步 cases 到 manager（on_loaded 后调用）。单一数据源，无 provider 中转。
    pub fn sync_cases(&self, cases: Vec<CaseViewModel>) {
        *self.cases.write().unwrap() = cases;
    }

    /// 供 TabWindowShell 渲染：返回 IValue 列表（trait upcast）。
    pub fn get_all_as_values(&self) -> Vec<Arc<dyn IValue>> {
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .map(|w| {
                let iv: Arc<dyn IContribution> = w.clone();
                iv as Arc<dyn IValue>
            })
            .collect()
    }

    /// 供 MainWindow.active_view 调用：渲染激活的 workbench。
    pub fn render_activated(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let activated = self.activated.read().unwrap().clone()?;
        let iv: &dyn IContribution = activated.as_ref();
        Some(iv.as_visual()?.render(window, cx))
    }

    /// 供 MainWindow.on_tab_click 调用：按 index 激活。
    pub fn activate_by_index(&self, index: usize) {
        let workbenches = self.workbenches.read().unwrap();
        if let Some(wb) = workbenches.get(index) {
            *self.activated.write().unwrap() = Some(wb.clone());
        }
    }

    /// 供 MainWindow 查询当前激活 index（用于 selected_tab 绑定）。
    pub fn activated_index(&self) -> Option<usize> {
        let workbenches = self.workbenches.read().unwrap();
        let activated = self.activated.read().unwrap();
        activated
            .as_ref()
            .and_then(|a| workbenches.iter().position(|w| w.uri() == a.uri()))
    }

    /// 按 URI schema 路由构造 workbench。无法识别的 schema 或找不到的 case 返回 None。
    fn build_workbench(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        match uri.scheme() {
            "rml" => {
                let case_id = uri.path().trim_start_matches('/');
                let case = self
                    .cases
                    .read()
                    .unwrap()
                    .iter()
                    .find(|c| c.id == case_id)
                    .cloned()?;
                Some(Arc::new(CaseWorkbench::new(uri.as_str().into(), case)))
            }
            "lsp" => Some(self.lsp_provider.render(uri)),
            _ => None,
        }
    }

    /// demo 专用 open：返回 Option<Arc<dyn IWorkbench>>（内部存储 + 激活）。
    fn open_workbench(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        if let Some(wb) = self
            .workbenches
            .read()
            .unwrap()
            .iter()
            .find(|w| w.uri() == uri_str)
            .cloned()
        {
            *self.activated.write().unwrap() = Some(wb.clone());
            return Some(wb);
        }
        let wb = self.build_workbench(uri)?;
        self.workbenches.write().unwrap().push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }
}

impl IWorkbenchManager for DemoWorkbenchManager {
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        self.open_workbench(uri)
    }

    fn close(&self, uri: &Uri) {
        let uri_str = uri.as_str();
        let mut workbenches = self.workbenches.write().unwrap();
        workbenches.retain(|w| w.uri() != uri_str);
        let mut activated = self.activated.write().unwrap();
        if activated.as_ref().map(|a| a.uri() == uri_str).unwrap_or(false) {
            *activated = workbenches.first().cloned();
        }
    }

    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> {
        self.workbenches.read().unwrap().clone()
    }

    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> {
        self.activated.read().unwrap().clone()
    }

    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .find(|w| w.uri() == uri_str)
            .cloned()
    }
}
