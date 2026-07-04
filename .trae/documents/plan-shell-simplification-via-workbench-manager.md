# Shell 简化计划：基于 IWorkbenchManager + MVVM 重构 demo

## Summary

将 demo shell 从「god object + 单桶存储 + 投影函数 + 全局回调」重构为 WPF 风格的四层分离：
**xaml (.rml) + xaml.cs (精简 .rml.rs) + ViewModel (MainWindow 持有 cases/menus/status 三个类型化集合) + IWorkbenchManager 实现**。

核心迁移：
1. **消除 `entries: RwLock<Vec<ContribEntry>>` 单桶存储** → MainWindow 直接持有 `cases: Vec<CaseViewModel>` / `menus: Vec<Arc<dyn IMenuItem>>` / `status: Vec<Arc<dyn IStatusBarItem>>`，`IContributionHost::add` 按 `kind` 路由直接入桶。
2. **消除 `shell_chrome.rs` 投影函数** → 投影逻辑内联到 `add` 路由 + `CaseViewModel` 构造器。
3. **Tab/资源生命周期迁移到 `IWorkbenchManager`** → `open_case`/`open_lsp_file`/`active_case_view`/`lsp_tabs` 全部由 manager 接管；按 URI schema 路由到 `CaseWorkbenchProvider`/`LspWorkbenchProvider`。
4. **消除 `DemoShellHost` 全局回调** → ActivityPanel/LspExplorerPanel 直接调用 `cx.get_workbench_manager().open(uri)`；`DemoShellHost` 仅保留为 ActivityPanel 只读访问 MainWindow.cases 的弱引用载体（不再承载 open_case 回调）。
5. **消除 `OpenTab` 并行数据结构** → Tab 元信息直接由 `IWorkbench` 实现（`CaseWorkbench`/`LspWorkbench` 同时 impl `IContribution + IVisualContribution + IWorkbench`，TabWindowShell 仍用 `as_contribution()`/`as_visual()` 渲染）。

预期代码量：shell 相关代码从 ~625 行（main_window.rml.rs 309 + activity_panel.rml.rs 113 + shell_chrome.rs 173 + catalog.rs 21）降至 ~350 行，同时职责清晰、可维护性大幅提升。

## Current State Analysis

### 当前 shell 文件清单

| 文件 | 行数 | 职责 | 问题 |
|---|---|---|---|
| [main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) | 309 | god object：host + tab 管理 + chrome 投影 + LSP 分流 + 主题/i18n 切换 | 混杂 5 种职责；`active_case_view` 硬编码 `lsp://` 分流；`entries` 单桶存储 |
| [main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) | 35 | 声明式模板 | 已足够精简，保持不变 |
| [activity_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/activity_panel.rml.rs) | 113 | 双重角色：visual contribution + host；Tree 渲染 + on_case_activate 回调 | host 仪式代码（case_entries/host_rx/IContributionHost impl）；`DemoShellHost` 回调链 |
| [shell_chrome.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_chrome.rs) | 173 | 投影函数：map_status_items/map_menu_items/map_case_tree_items/build_activity_panels_from | 全部可被 `add` 路由 + CaseViewModel 构造器替代 |
| [menu_shell_contribs.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs) | 323 | 11 个手写菜单贡献 | 结构不变，但 `with_main_window` helper 简化（主题/i18n 命令直接操作 cx，不走 MainWindow） |
| [catalog.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs) | 21 | `OpenTab` struct + `case_title_key` | `OpenTab` 被 `CaseWorkbench` 取代；`case_title_key` 保留（i18n key 映射） |

### 当前 MainWindow 的 5 种混杂职责

1. **IContributionHost**：`entries: RwLock<Vec<ContribEntry>>` 单桶 + `add`/`remove`（框架仪式，无法消除）
2. **Tab/资源管理**：`open_tabs`/`selected_tab`/`active_case_id`/`lsp_tabs` + `open_case`/`open_lsp_file`/`on_tab_click`/`active_case_view`（应迁移到 IWorkbenchManager）
3. **Chrome 投影**：`refresh_shell_chrome()` + `status_items`/`menu_items` 字段（应内联到 `add` 路由）
4. **ActivityBar 构建**：`build_activity_panels_from(&entries)` + observe ActivityPanel/LspExplorerPanel entity（保留，但数据源改为直接从 `add` 收集的 activity 贡献）
5. **App 级操作**：`apply_toggle_theme`/`apply_switch_en`（应提取为自由函数或 AppService，菜单命令直接调用）

### IWorkbenchManager 框架现状

[crates/core/src/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs) 已定义三接口（`IWorkbenchManager`/`IWorkbench`/`IWorkbenchProvider`），[crates/app/src/workbench/global.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/workbench/global.rs) 已提供 `WorkbenchManagerExt`（`set_workbench_manager`/`get_workbench_manager`，OnceLock 单例）。**demo 零实现**——本计划填补这一空白。

### 关键约束（来自 project_memory）

- `IContribution`/`IVisualContribution`/`IWorkbench` trait 方法签名不可修改
- `IVisualContribution::render(&self, &mut Window, &mut App) -> AnyElement`（无 RenderContext 包装）
- `IContributionHost` 必须含 `id`/`add`/`remove`，业务自受理
- `#[computed]` 方法只有 `&self`（无 cx），不能调用 `cx.get_workbench_manager()`
- Tab 数据须为 `Vec<Arc<dyn IValue>>`（TabWindowShell 用 `as_contribution()`/`as_visual()` 渲染）
- `as_visual()` 不加到 `IContribution` trait；用 `VisualAbilityExt` 自由函数

## Target Architecture

```
demo/src/
├── main.rs                          (不变)
├── app.rs                           (不变)
├── shell/
│   ├── mod.rs                       (更新导出)
│   ├── main_window.rml              (微调：tabs/selected-index 绑定名)
│   ├── main_window.rml.rs           (大幅精简：~150 行)
│   ├── activity_panel.rml           (不变)
│   ├── activity_panel.rml.rs        (精简：~50 行，移除 host 角色)
│   ├── login_dialog.rml.rs          (不变)
│   ├── case_view_model.rs           (新增：~40 行，CaseViewModel 结构)
│   ├── workbench.rs                 (新增：~200 行，DemoWorkbenchManager + Provider + Workbench 实现)
│   └── shell_chrome.rs              (删除，逻辑吸收到 add 路由 + CaseViewModel)
├── lsp/
│   ├── lsp_explorer_panel.rml.rs    (微调：on_file_activate 调用 manager.open)
│   └── ...                          (其余不变)
└── cases/
    ├── catalog.rs                   (精简：移除 OpenTab，保留 case_title_key)
    └── *.rml.rs                     (不变)
```

## Proposed Changes

### 1. 新增 [demo/src/shell/case_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_view_model.rs) — CaseViewModel 结构

**What**: 将 `(Arc<dyn IContribution>, ContributionOptions)` 元组解包为类型化结构，承载 case 类视觉贡献的视图数据。

**Why**: 消除 `ContribEntry` 元组 + `shell_chrome.rs::map_case_tree_items` 投影函数；Tree 绑定直接消费 `Vec<CaseViewModel>`，分组/排序逻辑内联到构造器。

**How**:

```rust
use std::sync::Arc;
use gpui::SharedString;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};

/// 案例视图模型：解包 (IVisualContribution, ContributionOptions) 为类型化结构。
/// 供 MainWindow.cases 集合持有，ActivityPanel Tree 直接消费。
#[derive(Clone)]
pub struct CaseViewModel {
    pub id: SharedString,
    pub name: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
    pub uri: SharedString,              // "rml://{id}"，供 IWorkbenchManager 路由
    pub visual: Arc<dyn IVisualContribution>,
}

impl CaseViewModel {
    /// 从贡献条目构造；非视觉贡献返回 None。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        let visual = c.as_visual()?;
        Some(Self {
            id: c.id().into(),
            name: c.name(),
            group: opts.group,
            order: opts.order,
            uri: format!("rml://{}", c.id()).into(),
            visual,
        })
    }

    /// 构建 TreeItem（含分组）：顶层分组节点 expanded，子节点为案例。
    /// 替代 shell_chrome.rs::map_case_tree_items。
    pub fn build_tree_items(cases: &[Self]) -> Vec<rml_ui::TreeItem> {
        // 按 group 分组 → 按 order 排序 → 构建 TreeItem 层级
        // (内联 map_case_tree_items 逻辑，~30 行)
    }
}
```

**Assumption**: CaseViewModel 字段包含 id/name/group/order/uri/visual。若用户截断消息中还有其他字段需求（如 command 能力引用），可在 review 时追加。

### 2. 新增 [demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) — IWorkbenchManager 实现 + Provider + Workbench

**What**: 实现 `DemoWorkbenchManager`（`IWorkbenchManager`）+ `CaseWorkbenchProvider`/`LspWorkbenchProvider`（`IWorkbenchProvider`）+ `CaseWorkbench`/`LspWorkbench`（`IWorkbench + IContribution + IVisualContribution`）。

**Why**: 将 Tab/资源生命周期从 MainWindow 迁出，统一通过 `manager.open(uri)` 路由。消除 `active_case_view` 中的 `lsp://` 硬编码分流，消除 `lsp_tabs: HashMap` 直接管理。

**How**:

```rust
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use gpui::{AnyElement, App, SharedString, Window, Entity};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IVisualContribution, VisualAbilityExt};
use rml_core::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};

// ── DemoWorkbench 枚举：封装两种具体 workbench，便于 downcast 与 render 分发 ──
pub enum DemoWorkbench {
    Case(Arc<CaseWorkbench>),
    Lsp(Arc<LspWorkbench>),
}

impl DemoWorkbench {
    fn as_workbench(&self) -> Arc<dyn IWorkbench> {
        match self {
            Self::Case(c) => c.clone(),
            Self::Lsp(l) => l.clone(),
        }
    }
    fn as_value(&self) -> Arc<dyn IValue> {
        match self {
            Self::Case(c) => c.clone() as Arc<dyn IValue>,
            Self::Lsp(l) => l.clone() as Arc<dyn IValue>,
        }
    }
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        match self {
            Self::Case(c) => c.render(window, cx),
            Self::Lsp(l) => l.render(window, cx),
        }
    }
    fn id(&self) -> &str {
        match self {
            Self::Case(c) => c.id(),
            Self::Lsp(l) => l.id(),
        }
    }
}

// ── CaseWorkbench：rml:// URI 的工作台，包装 CaseViewModel.visual ──
pub struct CaseWorkbench {
    uri: SharedString,
    case: CaseViewModel,  // 持有 visual + 元数据
}

impl CaseWorkbench {
    pub fn new(uri: SharedString, case: CaseViewModel) -> Self { Self { uri, case } }
}

impl IContribution for CaseWorkbench {
    fn id(&self) -> &str { &self.case.id }
    fn name(&self) -> SharedString { self.case.name.clone() }
}

impl IVisualContribution for CaseWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.case.visual.render(window, cx)
    }
}

impl IWorkbench for CaseWorkbench {
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}
}

// ── LspWorkbench：lsp:// URI 的工作台，包装 CodeEditorTab Entity ──
pub struct LspWorkbench {
    uri: SharedString,
    title: SharedString,
    tab: Entity<CodeEditorTab>,
}

impl IContribution for LspWorkbench {
    fn id(&self) -> &str { &self.uri }
    fn name(&self) -> SharedString { self.title.clone() }
}

impl IVisualContribution for LspWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.tab.update(cx, |tab, cx| tab.render(window, cx).into_any_element())
    }
}

impl IWorkbench for LspWorkbench { /* close/activate/set 同上 */ }

// ── CaseWorkbenchProvider：schema="rml"，从 MainWindow.cases 查找 case ──
pub struct CaseWorkbenchProvider {
    /// 弱引用访问 MainWindow.cases（通过 DemoShellHost 全局或直接弱引用）
    main: WeakEntity<MainWindow>,
}

impl IContribution for CaseWorkbenchProvider {
    fn id(&self) -> &str { "case-provider" }
    fn name(&self) -> SharedString { "Case Provider".into() }
}

impl IWorkbenchProvider for CaseWorkbenchProvider {
    fn schema(&self) -> SharedString { "rml".into() }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let case_id = uri.path().trim_start_matches('/');  // "components.button"
        let main = self.main.upgrade().expect("MainWindow dropped");
        // 在 main 的 cases 集合中查找（需 read cx —— 但 IWorkbenchProvider::render 无 cx 参数）
        // 问题：render(&self, uri: &Uri) -> Arc<dyn IWorkbench>，无 cx。
        // 解决：provider 内部缓存 cases 副本（由 MainWindow 在 open 前刷新），或
        //       改为 provider 持有 RwLock<Vec<CaseViewModel>> 副本，MainWindow.add 时同步推送。
        // 详见下方"设计决策 D3"。
        todo!()
    }
}

// ── LspWorkbenchProvider：schema="lsp"，构造 CodeEditorTab ──
pub struct LspWorkbenchProvider {
    lsp_client: Option<Arc<LspClient>>,
}

impl IWorkbenchProvider for LspWorkbenchProvider {
    fn schema(&self) -> SharedString { "lsp".into() }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        // uri = lsp://relative_path
        let relative_path = uri.path().trim_start_matches('/');
        // 构造 CodeEditorTab（但 render 无 cx/window 参数！）
        // 问题：CodeEditorTab::new 需要 &mut Window, &mut App。
        // 解决：provider.render 只创建数据壳，实际 Entity 创建延迟到 LspWorkbench.render 首次调用。
        //       或：provider 持有 RwLock<HashMap<String, Entity<CodeEditorTab>>> 缓存，
        //       由 MainWindow 在具备 cx 的时机预创建。
        // 详见下方"设计决策 D4"。
        todo!()
    }
}

// ── DemoWorkbenchManager：IWorkbenchManager 实现 ──
pub struct DemoWorkbenchManager {
    workbenches: RwLock<Vec<DemoWorkbench>>,
    activated: RwLock<Option<DemoWorkbench>>,
    providers: RwLock<HashMap<String, Arc<dyn IWorkbenchProvider>>>,
}

impl DemoWorkbenchManager {
    pub fn new(providers: Vec<Arc<dyn IWorkbenchProvider>>) -> Self {
        let map = providers.into_iter().map(|p| (p.schema().to_string(), p)).collect();
        Self { workbenches: RwLock::new(Vec::new()), activated: RwLock::new(None), providers: RwLock::new(map) }
    }

    /// 供 MainWindow 的 #[computed] tab_bar_items 调用：返回 IValue 列表供 TabWindowShell 渲染。
    pub fn get_all_as_values(&self) -> Vec<Arc<dyn IValue>> {
        self.workbenches.read().unwrap().iter().map(|w| w.as_value()).collect()
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
        activated.as_ref().and_then(|a| workbenches.iter().position(|w| w.id() == a.id()))
    }
}

impl IWorkbenchManager for DemoWorkbenchManager {
    fn open(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        // 1. 若已打开，激活并返回
        if let Some(wb) = self.workbenches.read().unwrap().iter().find(|w| w.id() == uri.as_str()) {
            *self.activated.write().unwrap() = Some(wb.clone());
            return wb.as_workbench();
        }
        // 2. 按 schema 路由到 provider
        let scheme = uri.scheme();
        let providers = self.providers.read().unwrap();
        let provider = providers.get(scheme).expect("no provider for schema");
        let wb_arc = provider.render(uri);
        drop(providers);
        // 3. 包装为 DemoWorkbench（需知道是 Case 还是 Lsp —— 通过 downcast）
        let demo_wb = {
            let any: &dyn Any = wb_arc.as_ref();
            if let Some(c) = any.downcast_ref::<CaseWorkbench>() {
                DemoWorkbench::Case(Arc::new(c.clone()))
            } else if let Some(l) = any.downcast_ref::<LspWorkbench>() {
                DemoWorkbench::Lsp(Arc::new(l.clone()))
            } else {
                panic!("unknown workbench type from provider")
            }
        };
        self.workbenches.write().unwrap().push(demo_wb.clone());
        *self.activated.write().unwrap() = Some(demo_wb.clone());
        demo_wb.as_workbench()
    }
    fn close(&self, uri: &Uri) {
        let mut workbenches = self.workbenches.write().unwrap();
        workbenches.retain(|w| w.id() != uri.as_str());
        let mut activated = self.activated.write().unwrap();
        if activated.as_ref().map(|a| a.id() == uri.as_str()).unwrap_or(false) {
            *activated = workbenches.first().cloned();
        }
    }
    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> {
        self.workbenches.read().unwrap().iter().map(|w| w.as_workbench()).collect()
    }
    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> {
        self.activated.read().unwrap().as_ref().map(|w| w.as_workbench())
    }
    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        self.workbenches.read().unwrap().iter()
            .find(|w| w.id() == uri.as_str())
            .map(|w| w.as_workbench())
    }
}
```

#### 设计决策

**D1 — CaseWorkbench/LspWorkbench 同时 impl 三 trait**：`IWorkbench`（生命周期）+ `IContribution`（Tab 元数据 id/name）+ `IVisualContribution`（render）。TabWindowShell 仍用 `as_contribution()`/`as_visual()` 渲染 Tab，无需修改框架组件。能力查询（`VisualAbilityExt::as_visual`）需要 `CaseWorkbench` 注册 ability cast —— 通过 `#[contribute]` 宏（`visual` 标志）或手动 `ability::register::<CaseWorkbench, dyn IVisualContribution>(...)` 在 `#[ctor::ctor]` 中注册。

**D2 — DemoWorkbench 枚举**：`IWorkbench` trait 不含 `render`/`id`，无法从 `Arc<dyn IWorkbench>` 直接渲染。引入 `DemoWorkbench` 枚举封装两种具体类型，manager 内部存储 `Vec<DemoWorkbench>`，提供 `get_all_as_values()`/`get_activated_demo()`/`activate_by_index()`/`activated_index()` 辅助方法供 MainWindow 调用。`IWorkbenchManager` trait 方法（`get_all`/`get_activated`/`get`）返回 `Arc<dyn IWorkbench>`，由 `as_workbench()` 转换。

**D3 — CaseWorkbenchProvider 数据访问**：`IWorkbenchProvider::render(&self, uri: &Uri) -> Arc<dyn IWorkbench>` 无 `cx`/`window` 参数，无法读取 MainWindow Entity。**方案**：Provider 内部持有 `RwLock<HashMap<String, CaseViewModel>>` 副本（key=case_id）。MainWindow 在 `IContributionHost::add` 受理 case 时，同步将 `CaseViewModel` 推送到 provider 的缓存。provider.render 从缓存查找 case，构造 `CaseWorkbench`。**替代方案**：provider 持有 `WeakEntity<MainWindow>`，但 render 无 cx 无法 read —— 排除。

**D4 — LspWorkbenchProvider 的 Entity 创建**：`CodeEditorTab::new` 需要 `&mut Window, &mut App`，但 `IWorkbenchProvider::render` 无这些参数。**方案**：`LspWorkbenchProvider::render` 只创建 `LspWorkbench` 数据壳（持有 uri/title/lsp_client），`CodeEditorTab` Entity 延迟到 `LspWorkbench::render` 首次调用时创建（此时有 `&mut Window, &mut App`）。`LspWorkbench` 内部用 `OnceCell<Entity<CodeEditorTab>>` 或 `Option<Entity<CodeEditorTab>>` + `RwLock` 实现懒加载。

**D5 — Manager 安装时机**：`MainWindow::on_loaded` 中 `cx.set_workbench_manager(Arc::new(manager))`。manager 在安装前注入 providers（构造时传入）。`CaseWorkbenchProvider` 的 cases 缓存在 `add` 受理时同步 —— 但 `add` 在 `__rml_install_host` 触发的 `drain_host_ops` 期间调用，此时 manager 尚未安装。**解决**：`add` 时 cases 直接推入 MainWindow.cases 集合；manager 安装后，MainWindow 将 cases 集合一次性同步到 provider 缓存（`provider.sync_cases(cases)`）。后续 `add` 若动态新增 case（demo 不支持，但预留），再同步推送。

**D6 — open_tab 点击激活**：`on_tab_click(index)` 调用 `manager.activate_by_index(index)` + 同步 `self.selected_tab` + `cx.notify()`。无需查 `open_tabs` —— 但 `selected_tab` 仍需作为字段保留供 `selected-index={selected_tab}` 绑定。`open_tabs` 字段替换为 manager 状态的派生缓存：在每次 open/activate/close 命令后，`self.open_tabs = manager.get_all_as_values()` + `self.selected_tab = manager.activated_index().unwrap_or(0)`。

### 3. 重构 [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) — 精简为 ~150 行

**What**: MainWindow 从 god object 精简为 ViewModel 角色，仅持有类型化集合 + 最小命令委托。

**Why**: 实现用户指定的「cases/menus/status 三个集合直接绑定」+ 将资源生命周期委托给 manager。

**How**: 新结构如下（伪代码，~150 行）：

```rust
#[window]
#[contributehost(id = "demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    // ── ViewModel 集合（直接绑定模板）──
    cases: Vec<CaseViewModel>,                      // 绑定 ActivityPanel Tree
    menus: Vec<Arc<dyn IMenuItem>>,                 // 绑定 <menu-bar items={menus}>
    status: Vec<Arc<dyn IStatusBarItem>>,           // 绑定 <status_bar items={status}>
    activities: Vec<Arc<dyn IActivityPanel>>,       // ActivityBar panels（on_loaded 一次性构建）

    // ── Tab 状态（manager 派生缓存，命令后同步）──
    open_tabs: Vec<Arc<dyn IValue>>,                // 绑定 tabs={tab_bar_items}
    selected_tab: usize,
    show_chrome: bool,
    slot_left_size: gpui::Pixels,

    // ── 框架仪式 ──
    activity_bar: Option<Entity<ActivityBar>>,
    host_rx: Option<Receiver<HostOp>>,
    manager: Option<Arc<DemoWorkbenchManager>>,     // 持有 Arc 供命令调用
    lsp_client: Option<Arc<LspClient>>,             // 供 LspWorkbenchProvider 构造

    // ── 决策 D5：cases 暂存（drain 期间 manager 未安装）──
    // cases 已在字段中；manager 安装后一次性同步到 provider。
}

impl IContributionHost for MainWindow {
    fn id(&self) -> &'static str { Self::ID }
    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let opts = options.unwrap_or_default();
        let kind = opts.effective_slot();
        match kind {
            Some("case") => {
                if let Some(cvm) = CaseViewModel::from_contribution(contribution, opts) {
                    self.cases.write().unwrap().push(cvm);
                }
            }
            Some("menu") => {
                // 累积到 menu_entries，重建 menus（或直接重建）
                // 详见 D7
            }
            Some("status") => {
                let align = opts.properties.get("align").map(|s| s.as_ref()) == Some("right");
                let item = StatusBarItem::new(contribution.name()).align(...).into_arc();
                self.status.write().unwrap().push(item);
            }
            Some("activity") => {
                if let Some(panel) = VisualActivityPanel::new(contribution) {
                    self.activities.write().unwrap().push(Arc::new(panel));
                }
            }
            _ => {}
        }
    }
    fn remove(&self, id: &str) {
        // 从对应集合移除（demo 不动态移除，简单 retain）
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window, cx) {
        // 1. 框架仪式：install host + drain ops（add 期间填充 cases/menus/status/activities）
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);
        if let Some(rx) = &self.host_rx { drain_host_ops(rx, self); }

        // 2. 构建 manager（providers 注入 cases 缓存）
        let case_provider = Arc::new(CaseWorkbenchProvider::with_cases(self.cases.clone()));
        let lsp_provider = Arc::new(LspWorkbenchProvider::new(self.lsp_client.clone()));
        let manager = Arc::new(DemoWorkbenchManager::new(vec![case_provider, lsp_provider]));
        cx.set_workbench_manager(manager.clone());
        self.manager = Some(manager);

        // 3. 构建 ActivityBar（从 activities 集合）
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(self.activities.clone())));

        // 4. observe ActivityBar + ActivityPanel/LspExplorerPanel entity（保留现有逻辑）
        // ...

        // 5. 打开 welcome tab（经 manager）
        let manager = self.manager.as_ref().unwrap();
        manager.open(&"rml://welcome".parse().unwrap());
        self.sync_tab_state(manager);

        self.show_chrome = true;
        self.slot_left_size = gpui::px(260.);
        cx.notify();
    }
}

impl MainWindow {
    /// 同步 tab 状态：从 manager 派生 open_tabs + selected_tab
    fn sync_tab_state(&mut self, manager: &DemoWorkbenchManager) {
        self.open_tabs = manager.get_all_as_values();
        self.selected_tab = manager.activated_index().unwrap_or(0);
    }

    /// 渲染激活的 workbench 视图（替代旧 active_case_view）
    pub fn active_view(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        if let Some(manager) = &self.manager {
            if let Some(wb) = manager.get_activated_demo() {
                return wb.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<Arc<dyn IValue>> {
        self.open_tabs.clone()
    }

    #[command]
    pub fn on_chrome_toggle(&mut self, cx) { self.show_chrome = !self.show_chrome; }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx) {
        if let Some(manager) = &self.manager {
            manager.activate_by_index(index);
            self.sync_tab_state(manager);
            cx.notify();
        }
    }

    /// 由 ActivityPanel::on_case_activate 调用（经 manager.open）
    #[command]
    pub fn open_case(&mut self, case_id: String, cx) {
        if let Some(manager) = &self.manager {
            let uri: Uri = format!("rml://{}", case_id).parse().unwrap();
            manager.open(&uri);
            self.sync_tab_state(manager);
            cx.notify();
        }
    }

    /// 由 LspExplorerPanel::on_file_activate 调用（经 manager.open）
    #[command]
    pub fn open_lsp_file(&mut self, relative_path: String, cx) {
        if let Some(manager) = &self.manager {
            let uri: Uri = format!("lsp://{}", relative_path).parse().unwrap();
            manager.open(&uri);
            self.sync_tab_state(manager);
            cx.notify();
        }
    }
}
```

**D7 — menus 集合的构建**：`map_menu_items` 的菜单树构建逻辑（~50 行，按 parent_id 建树）需保留。方案：在 `IContributionHost::add` 中将 menu 贡献累积到 `menu_entries: RwLock<Vec<ContribEntry>>`，在 `on_loaded` 的 drain 后调用 `self.menus = build_menu_tree(&self.menu_entries.read().unwrap())`（`build_menu_tree` 是从 `shell_chrome.rs::map_menu_items` 提取的纯函数，移到 `main_window.rml.rs` 或保留在精简后的 `shell_chrome.rs` 中仅留此一个函数）。

**移除的代码**：
- `entries: RwLock<Vec<ContribEntry>>` 字段 → 替换为 `cases`/`menu_entries`/`status`/`activities`
- `refresh_shell_chrome()` 方法 → 内联到 `add` + `on_loaded`
- `active_case_view()` 的 `lsp://` 分流 + `lsp_tabs` HashMap → 由 `manager.get_activated_demo().render()` 取代
- `DemoShellHost(pub WeakEntity<MainWindow>)` 全局 → 保留（ActivityPanel 只读访问 cases），但不再承载 `open_case` 回调（改经 manager）

**保留的代码**：
- `apply_toggle_theme`/`apply_switch_en`（菜单命令仍调用，但 `with_main_window` helper 简化）
- ActivityBar 构建 + observe 逻辑
- LSP 子进程启动逻辑（`LspClient::spawn`）

### 4. 重构 [demo/src/shell/activity_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/activity_panel.rml.rs) — 精简为 ~50 行

**What**: 移除 ActivityPanel 的 host 角色（`#[contributehost]` + `case_entries` + `host_rx` + `IContributionHost` impl + `ILifecycle`），改为纯视觉贡献。

**Why**: Cases 现在注册到 `demo.shell`（MainWindow 是 host），ActivityPanel 不再需要中转。消除 host 仪式代码 ~60 行。

**How**:

```rust
#[contribute(host_id = "demo.shell", id = "samples", kind = "activity", order = 0)]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<Entity<TreeState>>,
    main: Option<WeakEntity<MainWindow>>,  // 只读访问 cases 集合
}

impl IContribution for ActivityPanel {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.samples").into() }
    fn icon(&self) -> Option<SharedString> { Some("BookOpen".into()) }
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window, cx) {
        // 获取 MainWindow 弱引用（经 DemoShellHost 全局）
        if let Some(host) = cx.try_global::<DemoShellHost>() {
            self.main = Some(host.0.clone());
        }
        self.refresh_tree(cx);
        cx.notify();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx) {
        if let Some(main) = &self.main {
            if let Some(main) = main.upgrade() {
                let cases = main.read(cx).cases.clone();
                let items = CaseViewModel::build_tree_items(&cases);
                self.set_tree_items(items, cx);
            }
        }
    }

    fn set_tree_items(&mut self, items: Vec<TreeItem>, cx) { /* 同现有 */ }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &SharedString, cx) {
        // 直接调用 manager.open（无需 DemoShellHost → MainWindow.open_case 回调链）
        if let Some(manager) = cx.get_workbench_manager() {
            let uri: Uri = format!("rml://{}", item_id).parse().unwrap();
            manager.open(&uri);
            // manager.open 已更新 activated；MainWindow 需同步 open_tabs —— 
            // 但 manager 是 &self，无 cx.notify MainWindow 的能力。
            // 解决：manager.open 内部触发 MainWindow 通知（通过弱引用回调），或
            //       ActivityPanel 通过 DemoShellHost 弱引用调用 MainWindow.open_case（保留旧回调）。
            // 详见 D8。
        }
    }
}
```

**D8 — manager.open 后 MainWindow 状态同步**：`manager.open(&uri)` 是 `&self` 方法（无 cx），无法直接通知 MainWindow 重建 `open_tabs`。两种方案：
- **方案 A**：`ActivityPanel::on_case_activate` 仍通过 `DemoShellHost` 弱引用调用 `MainWindow::open_case(case_id, cx)`，`open_case` 内部调用 `manager.open` + `sync_tab_state` + `cx.notify()`。保留 `DemoShellHost` 回调链，但回调内容从「直接管理 tab」变为「触发 manager + 同步」。**采用此方案**（最简单，不引入 manager→MainWindow 反向通知机制）。
- 方案 B：manager 持有 `WeakEntity<MainWindow>`，open 后调用 `main.update(cx, |this, cx| { this.sync_tab_state(); cx.notify(); })`。但 manager 是 `Arc<dyn IWorkbenchManager>`（无 cx），需在 open 内部获取 cx —— 不可行。
- 方案 C：manager 维护版本号 `AtomicU64`，MainWindow 用定时器/observe 轮询。过于复杂。

**结论**：ActivityPanel/LspExplorerPanel 的 `on_case_activate`/`on_file_activate` 仍走 `DemoShellHost` 弱引用 → `MainWindow::open_case`/`open_lsp_file`，但 MainWindow 方法内部委托给 manager。`DemoShellHost` 全局保留，但职责从「数据传递 + 命令回调」缩减为「仅命令回调触发」（cases 数据通过弱引用 read 直接访问，不再通过全局传递）。

### 5. 微调 [demo/src/lsp/lsp_explorer_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_explorer_panel.rml.rs)

**What**: `on_file_activate` 仍通过 `DemoShellHost` 调用 `MainWindow::open_lsp_file`（D8 方案 A），但 `MainWindow::open_lsp_file` 内部改为 `manager.open(lsp_uri)` + `sync_tab_state`。LspExplorerPanel 自身代码不变。

### 6. 删除 [demo/src/shell/shell_chrome.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_chrome.rs)

**What**: 删除整个文件。`map_status_items`/`map_case_tree_items`/`build_activity_panels_from` 逻辑内联到 `MainWindow::add` + `CaseViewModel::build_tree_items`。`map_menu_items` 逻辑（菜单树构建）提取为 `build_menu_tree` 纯函数，移到 `main_window.rml.rs` 或保留精简后的 `shell_chrome.rs` 仅含此函数。

**Decision**: 保留 `shell_chrome.rs` 但仅含 `build_menu_tree`（~50 行），其余函数删除。文件从 173 行降至 ~50 行。

### 7. 微调 [demo/src/shell/menu_shell_contribs.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs)

**What**: `with_main_window` helper 保留（菜单命令仍需访问 MainWindow 切换主题/i18n）。`apply_toggle_theme`/`apply_switch_en` 保留在 MainWindow。结构不变。

**可选优化**：主题/i18n 命令可直接操作 `cx.set_theme()`/`cx.set_i18n()`，无需 `with_main_window` —— 但 `apply_switch_en` 还需刷新 tab 标题（`open_tabs` 重建），所以保留 MainWindow 调用。

### 8. 精简 [demo/src/cases/catalog.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs)

**What**: 移除 `OpenTab` struct（被 `CaseWorkbench`/`LspWorkbench` 取代）。保留 `case_title_key` 函数（i18n key 映射，仍被 `CaseWorkbenchProvider` 或 `CaseViewModel` 构造时使用）。文件从 21 行降至 ~20 行（仅删除 OpenTab struct + impl）。

### 9. 微调 [demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

**What**: 绑定名调整：
- `tabs={tab_bar_items}` → 保持（`#[computed] tab_bar_items` 仍返回 `self.open_tabs.clone()`）
- `selected-index={selected_tab}` → 保持
- `items={menu_items}` → `items={menus}`（字段重命名）
- `items={status_items}` → `items={status}`（字段重命名）
- `content={self.active_case_view(_window, cx)}` → `content={self.active_view(_window, cx)}`（方法重命名）

## Assumptions & Decisions

1. **CaseViewModel 字段**：`id`/`name`/`group`/`order`/`uri`/`visual`。用户消息在「包含 IVisualContribution + ContributionOptions 解包数据和」处截断，未列出后续字段。本计划假设上述字段满足需求；若需追加（如 command 能力引用），可在 review 时扩展。

2. **LSP 纳入 IWorkbenchManager**：采用「纳入，按 schema 路由」方案。`LspWorkbenchProvider`（schema="lsp"）+ `CaseWorkbenchProvider`（schema="rml"）统一在 `DemoWorkbenchManager` 中路由。`active_case_view` 的 `lsp://` 硬编码分流删除，`lsp_tabs` HashMap 删除。

3. **DemoShellHost 全局保留**：仅作为 `WeakEntity<MainWindow>` 载体供 ActivityPanel/LspExplorerPanel 触发 `MainWindow::open_case`/`open_lsp_file`（D8 方案 A）。不再用于 cases 数据传递（cases 通过弱引用 read 直接访问）。

4. **DemoWorkbench 枚举**：因 `IWorkbench` trait 不含 `render`/`id`，引入 `DemoWorkbench` 枚举封装 `CaseWorkbench`/`LspWorkbench` 两种具体类型，manager 内部存储 `Vec<DemoWorkbench>`，提供 `get_all_as_values()`/`get_activated_demo()`/`activate_by_index()`/`activated_index()` 辅助方法。

5. **CaseWorkbenchProvider 缓存 cases 副本**：因 `IWorkbenchProvider::render` 无 cx 参数，无法读取 MainWindow Entity。Provider 内部持有 `RwLock<HashMap<String, CaseViewModel>>`，MainWindow 在 `on_loaded`（drain 后）一次性同步 cases 集合。

6. **LspWorkbench 懒加载 CodeEditorTab Entity**：因 `IWorkbenchProvider::render` 无 window/cx，`CodeEditorTab` Entity 延迟到 `LspWorkbench::render` 首次调用时创建（`OnceCell<Entity<CodeEditorTab>>`）。

7. **menus 集合构建**：保留 `build_menu_tree` 纯函数（从 `map_menu_items` 提取），在 `on_loaded` drain 后一次性构建。`shell_chrome.rs` 精简为仅含此函数。

8. **Trait 不修改**：`IWorkbench`/`IContribution`/`IVisualContribution`/`IContributionHost` 签名均不修改。`CaseWorkbench`/`LspWorkbench` 通过多 trait impl + ability registry 注册满足 TabWindowShell 的 `as_contribution()`/`as_visual()` 查询需求。

9. **ability 注册**：`CaseWorkbench`/`LspWorkbench` 需注册 `dyn IContribution` + `dyn IVisualContribution` ability cast。通过 `#[contribute]` 宏（带 `visual` 标志，但 host_id 设为不存在的占位，使 `__rml_register_*` 不被 bootstrap 调用）或手动 `ability::register::<CaseWorkbench, dyn IVisualContribution>(...)` 在 `#[ctor::ctor]` 中注册。**采用手动注册**（避免 `#[contribute]` 宏的 host_id 路由副作用）。

## Code Volume Comparison

| 文件 | 当前 | 目标 | 变化 |
|---|---|---|---|
| main_window.rml.rs | 309 | ~150 | -159 |
| activity_panel.rml.rs | 113 | ~50 | -63 |
| shell_chrome.rs | 173 | ~50 | -123 |
| catalog.rs | 21 | ~20 | -1 |
| case_view_model.rs (新) | 0 | ~40 | +40 |
| workbench.rs (新) | 0 | ~200 | +200 |
| menu_shell_contribs.rs | 323 | 323 | 0 |
| main_window.rml | 35 | 35 | 0 |
| **shell 总计** | **974** | **~868** | **-106** |

**注**：行数减少幅度看似不显著（-106），但**职责分离度大幅提升**：
- MainWindow 从 5 种职责降至 2 种（ViewModel 集合 + 命令委托）
- Tab/资源生命周期完全移出 MainWindow 到 `workbench.rs`
- shell_chrome.rs 投影逻辑从 4 个函数降至 1 个
- ActivityPanel 从双重角色降至单一视觉贡献

若以「MainWindow 自身复杂度」衡量，从 309 行降至 ~150 行（-51%），且不再包含 LSP 分流、tab 管理、chrome 投影等混杂逻辑。

## Verification

1. **编译验证**：
   - `cargo build -p rust-rml-demo` —— 整体编译通过
   - `cargo build -p rust-rml-core` / `cargo build -p rust-rml-app` —— 框架层未改动，应仍通过

2. **功能验证（手动）**：
   - 启动 demo，welcome tab 自动打开（经 `manager.open("rml://welcome")`）
   - ActivityPanel 树显示所有 case（经 MainWindow.cases 集合）
   - 点击 case → 打开新 tab 或激活已有 tab（经 `manager.open("rml://{case_id}")`）
   - 点击 LSP 文件 → 打开 CodeEditorTab（经 `manager.open("lsp://{path}")`）
   - Tab 切换 → 内容区切换（经 `manager.get_activated_demo().render()`）
   - 菜单 File → New/Open/Exit 功能正常
   - 菜单 View → Theme Toggle/Lang EN 功能正常
   - ActivityBar 折叠/展开 → slot_left_size 切换
   - Chrome toggle 按钮功能正常

3. **回归验证**：
   - LSP 补全/hover/跳转功能正常（CodeEditorTab 懒加载不影响）
   - i18n 切换后 tab 标题刷新（`apply_switch_en` 重建 open_tabs）
   - 主题切换后界面刷新

4. **架构验证**：
   - `grep -r "entries" demo/src/shell/` —— 无 `RwLock<Vec<ContribEntry>>` 单桶存储
   - `grep -r "lsp_tabs" demo/src/shell/` —— 无 HashMap 直接管理
   - `grep -r "active_case_id" demo/src/shell/` —— 无硬编码 lsp:// 分流
   - `grep -r "map_status_items\|map_case_tree_items\|build_activity_panels_from" demo/src/shell/` —— 无旧投影函数调用

## Out of Scope

- 框架层 trait 修改（`IWorkbench`/`IContribution`/`IVisualContribution` 签名不变）
- `TabWindowShell` 组件修改（仍消费 `Vec<Arc<dyn IValue>>` + `as_contribution()`/`as_visual()`）
- `ActivityBar` 组件修改（仍消费 `Vec<Arc<dyn IActivityPanel>>`）
- `#[contribute]`/`#[contributehost]` 宏修改
- LSP 子进程管理逻辑改动（`LspClient::spawn` 保留在 MainWindow::on_loaded）
- 多窗口/多 manager 实例（当前 OnceLock 单例足够）
