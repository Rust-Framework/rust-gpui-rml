//! IWorkbenchManager 实现 —— Tab/资源生命周期从 MainWindow 迁入。
//!
//! - `DemoWorkbenchManager`（`IWorkbenchManager`）：按 URI schema 路由到 provider，
//!   维护 `Vec<DemoWorkbench>` + 激活态。
//! - `CaseWorkbenchProvider` / `LspWorkbenchProvider`（`IWorkbenchProvider`）：
//!   分别处理 `rml://` / `lsp://` URI。
//! - `CaseWorkbench` / `LspWorkbench`：`IWorkbench + IContribution + IVisualContribution`
//!   三 trait impl，供 TabWindowShell 经 `as_contribution()`/`as_visual()` 渲染。
//! - `DemoWorkbench` 枚举：封装两种具体 workbench，便于 manager 内部存储与 render 分发
//!   （`IWorkbench` trait 无 `render`/`id`，需枚举桥接）。

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Once, RwLock};

use gpui::{AnyElement, App, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::{
    register_contribution_ability, register_visual_ability, IContribution, IVisualContribution,
};
use rml_core::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};

use crate::lsp::{CodeEditorTab, LspClient};
use crate::shell::case_view_model::CaseViewModel;

// ──────────────────────────────────────────────────────────────────────────
//  能力注册：CaseWorkbench / LspWorkbench 需注册 IContribution + IVisualContribution
//  能力 cast，使 TabWindowShell 的 `as_contribution()`/`as_visual()` 查询生效。
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
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}
}

// ──────────────────────────────────────────────────────────────────────────
//  DemoWorkbench 枚举：manager 内部存储 + render 分发
// ──────────────────────────────────────────────────────────────────────────

/// 封装两种具体 workbench，便于 manager 内部 `Vec<DemoWorkbench>` 存储。
///
/// `IWorkbench` trait 无 `render`/`id`，需经枚举分发到具体类型。
#[derive(Clone)]
pub enum DemoWorkbench {
    Case(Arc<CaseWorkbench>),
    Lsp(Arc<LspWorkbench>),
}

impl DemoWorkbench {
    pub fn as_workbench(&self) -> Arc<dyn IWorkbench> {
        match self {
            Self::Case(c) => c.clone(),
            Self::Lsp(l) => l.clone(),
        }
    }

    pub fn as_value(&self) -> Arc<dyn IValue> {
        match self {
            Self::Case(c) => c.clone(),
            Self::Lsp(l) => l.clone(),
        }
    }

    pub fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        match self {
            Self::Case(c) => c.render(window, cx),
            Self::Lsp(l) => l.render(window, cx),
        }
    }

    pub fn uri(&self) -> &str {
        match self {
            Self::Case(c) => &c.uri,
            Self::Lsp(l) => &l.uri,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  CaseWorkbenchProvider：schema="rml"，从 cases 缓存查找
// ──────────────────────────────────────────────────────────────────────────

/// `rml://` URI 的 workbench 工厂。
///
/// 持有 `RwLock<HashMap<String, CaseViewModel>>` 副本（D3）——
/// `IWorkbenchProvider::render` 无 cx 参数，无法读取 MainWindow Entity，
/// 由 MainWindow 在 `on_loaded` drain 后一次性同步 cases 集合。
pub struct CaseWorkbenchProvider {
    cases: RwLock<HashMap<String, CaseViewModel>>,
}

impl CaseWorkbenchProvider {
    pub fn new() -> Self {
        Self {
            cases: RwLock::new(HashMap::new()),
        }
    }

    /// 同步 cases 副本（on_loaded drain 后调用）。
    pub fn sync_cases(&self, cases: Vec<CaseViewModel>) {
        let mut map = self.cases.write().unwrap();
        map.clear();
        for c in cases {
            map.insert(c.id.to_string(), c);
        }
    }

    /// demo 专用：返回 `DemoWorkbench` 供 manager 内部存储。
    fn render_demo(&self, uri: &Uri) -> DemoWorkbench {
        let case_id = uri.path().trim_start_matches('/');
        let case = self
            .cases
            .read()
            .unwrap()
            .get(case_id)
            .cloned()
            .unwrap_or_else(|| panic!("case not found: {case_id}"));
        DemoWorkbench::Case(Arc::new(CaseWorkbench::new(
            uri.as_str().into(),
            case,
        )))
    }
}

impl IContribution for CaseWorkbenchProvider {
    fn id(&self) -> &str {
        "case-provider"
    }
    fn name(&self) -> SharedString {
        "Case Provider".into()
    }
}

impl IWorkbenchProvider for CaseWorkbenchProvider {
    fn schema(&self) -> SharedString {
        "rml".into()
    }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        self.render_demo(uri).as_workbench()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  LspWorkbenchProvider：schema="lsp"，构造 LspWorkbench
// ──────────────────────────────────────────────────────────────────────────

/// `lsp://` URI 的 workbench 工厂。
///
/// `LspWorkbench` 的 `CodeEditorTab` Entity 延迟到首次 `render` 时创建（D4）——
/// `IWorkbenchProvider::render` 无 window/cx 参数，无法创建 Entity。
pub struct LspWorkbenchProvider {
    lsp_client: Option<Arc<LspClient>>,
}

impl LspWorkbenchProvider {
    pub fn new(lsp_client: Option<Arc<LspClient>>) -> Self {
        Self { lsp_client }
    }

    /// demo 专用：返回 `DemoWorkbench` 供 manager 内部存储。
    fn render_demo(&self, uri: &Uri) -> DemoWorkbench {
        let relative_path = uri.path().trim_start_matches('/').to_string();
        let title = std::path::Path::new(&relative_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&relative_path)
            .into();
        DemoWorkbench::Lsp(Arc::new(LspWorkbench::new(
            uri.as_str().into(),
            title,
            self.lsp_client.clone(),
        )))
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
        self.render_demo(uri).as_workbench()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  DemoWorkbenchManager：IWorkbenchManager 实现
// ──────────────────────────────────────────────────────────────────────────

/// demo 工作台管理器：按 URI schema 路由到 provider，维护 `Vec<DemoWorkbench>` + 激活态。
///
/// 内部存储 `DemoWorkbench` 枚举（保留具体类型供 render 分发），
/// `IWorkbenchManager` trait 方法经 `as_workbench()` 转换为 `Arc<dyn IWorkbench>`。
pub struct DemoWorkbenchManager {
    workbenches: RwLock<Vec<DemoWorkbench>>,
    activated: RwLock<Option<DemoWorkbench>>,
    case_provider: Arc<CaseWorkbenchProvider>,
    lsp_provider: Arc<LspWorkbenchProvider>,
}

impl DemoWorkbenchManager {
    pub fn new(lsp_client: Option<Arc<LspClient>>) -> Self {
        register_workbench_abilities();
        Self {
            workbenches: RwLock::new(Vec::new()),
            activated: RwLock::new(None),
            case_provider: Arc::new(CaseWorkbenchProvider::new()),
            lsp_provider: Arc::new(LspWorkbenchProvider::new(lsp_client)),
        }
    }

    /// 同步 cases 副本到 case provider（on_loaded drain 后调用）。
    pub fn sync_cases(&self, cases: Vec<CaseViewModel>) {
        self.case_provider.sync_cases(cases);
    }

    /// 供 TabWindowShell 渲染：返回 IValue 列表（DemoWorkbench.as_value）。
    pub fn get_all_as_values(&self) -> Vec<Arc<dyn IValue>> {
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .map(|w| w.as_value())
            .collect()
    }

    /// 供 MainWindow.active_view 调用：返回激活的 DemoWorkbench 用于 render。
    pub fn get_activated_demo(&self) -> Option<DemoWorkbench> {
        self.activated.read().unwrap().clone()
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

    /// demo 专用 open：返回 DemoWorkbench（内部存储 + render 分发）。
    fn open_demo(&self, uri: &Uri) -> DemoWorkbench {
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
            return wb;
        }
        let demo_wb = match uri.scheme() {
            "rml" => self.case_provider.render_demo(uri),
            "lsp" => self.lsp_provider.render_demo(uri),
            scheme => panic!("unknown workbench schema: {scheme}"),
        };
        self.workbenches.write().unwrap().push(demo_wb.clone());
        *self.activated.write().unwrap() = Some(demo_wb.clone());
        demo_wb
    }
}

impl IWorkbenchManager for DemoWorkbenchManager {
    fn open(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        self.open_demo(uri).as_workbench()
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
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .map(|w| w.as_workbench())
            .collect()
    }

    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> {
        self.activated
            .read()
            .unwrap()
            .as_ref()
            .map(|w| w.as_workbench())
    }

    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        self.workbenches
            .read()
            .unwrap()
            .iter()
            .find(|w| w.uri() == uri_str)
            .map(|w| w.as_workbench())
    }
}
