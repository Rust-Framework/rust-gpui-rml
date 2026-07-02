# RML 贡献点架构重构计划（精简版）

## 摘要

本计划聚焦**贡献点架构**这一单一关注点，重构 `IContributionHost` 为主动受理方（自带 `add`/`remove` 受理代码），引入 `IContributionRegistry` 作为框架内桥接器，将 `register` 调用按 `host_id` 路由到对应 host 的 `add` 方法。框架不再存储任何贡献数据——`ContributedEntry`、`ComponentEntityCache`、`VisualRenderer`、`RenderContext`、`EntityCache` 等过度设计类型一律删除。`IContribution`（能力贡献点）与 `IVisualContribution`（可视化贡献点）作为语义独立的两个 trait 保留——不添加 `as_visual`/`render_view` 等桥接方法，仅 `IContribution` supertraits 增加 `Any` marker bound，配合宏生成的 `VisualExtractor` 函数实现 `Arc<dyn IVisualContribution>` 向下转型。`IVisualContribution::render` 参数由 `&mut RenderContext` 调整为 `(&mut Window, &mut App)`（`RenderContext` 删除的必要后果）。host 自由决定存储策略（`Vec` / `HashMap` / 不存储），不由框架限定。宏生成的注册代码精简为单行 `cx.get_contribution_registry().register(...)` 调用，移除 `register_contribution` 中间层。

### 迭代目标

| 阶段      | 目标          | 交付物                                                                                                                                                                          |
| ------- | ----------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 | 核心类型精简      | `IContribution`（能力贡献 + `Any` supertrait）+ `IVisualContribution`（可视化贡献）+ `IContributionHost`（受理方）+ `IContributionRegistry` trait + `HostHandle` 内部桥接 + `VisualExtractor` 函数类型 |
| Phase 2 | App 层重写     | `ContributionRegistry`（RwLock + pending 队列）+ `get_contribution_registry()` 扩展                                                                                                |
| Phase 3 | 宏与 build 简化 | `#[contribute]` 单行注册 / `#[contributehost]` 仅生成 ID + 断言                                                                                                                       |
| Phase 4 | Demo 受理代码   | `MainWindow::add`/`remove` 受理分发 + `register_host(cx)` + `cx.observe` 替代 `subscribe_host_changes`                                                                             |

### 范围界定

**包含**：贡献点架构（核心类型、App 层、宏、build、Demo 验证）

**不包含**（独立关注点，本计划不修改）：

* `ObservableVec<T>` 核心类型（原 Phase A）——若业务需要可作为独立工具类型单独立项

* 版本系统集成（原 Phase B）——现有 `__rml_get_version` / `ComputedCache` 基础设施已存在，无需调整

* RML `each=` + `key=` keyed diffing（原 Phase D）——RML 模板层独立关注点

***

## 用户决策汇总

| # | 决策                                                                                                                               | 影响                                                                                                                                                                               |
| - | -------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1 | Phase C 否决，不增加新宏，`contribution_entries` 不出现在业务代码                                                                                 | 移除 `#[computed_with_cx]` 设想；业务代码不读框架存储                                                                                                                                           |
| 2 | `IContributionHost` 含 `id`/`add`/`remove`，业务编写受理代码                                                                               | host 主动处置贡献，框架不代劳                                                                                                                                                                |
| 3 | `ContributedEntry` 无必要性                                                                                                          | 框架不存储贡献条目                                                                                                                                                                        |
| 4 | `ComponentEntityCache` 无必要性                                                                                                      | 框架不缓存组件 Entity                                                                                                                                                                   |
| 5 | `IContributionRegistry` 定义 `add`/`remove`/`register`/`unregister`                                                                | 框架实现桥接 contribute → host                                                                                                                                                         |
| 6 | 扩展 App/Context 提供 `get_contribution_registry()`                                                                                  | 宏生成代码通过接口操作                                                                                                                                                                      |
| 7 | host 不强制使用 `ObservableVec`，甚至可不存储；存储策略由 host 实现决定                                                                                | 框架不限定 host 行为                                                                                                                                                                    |
| 8 | 清理所有过度设计；宏展开应简洁——直接 `cx.get_contribution_registry().method()` 调用；`IContribution`/`IVisualContribution` trait 方法签名禁止修改            | 移除 `register_contribution` 中间层、`RenderContext`/`EntityCache`/`VisualRenderer` 等。`IContribution` supertraits 增加 `Any` marker bound（不改方法），配合宏生成 `VisualExtractor` 实现视觉贡献向下转型       |
| 9 | **`IContribution`** **禁止添加** **`as_visual()`** ——`IContribution`（能力贡献）与 `IVisualContribution`（可视化贡献）语义已清晰，**禁止修改两个 trait 的方法签名** | 视觉贡献向下转型改用 `Any` supertrait + 宏生成 `VisualExtractor` **自由函数**实现（非 trait 方法）。`IContribution` 仅 supertraits 增 `Any` marker bound，方法零修改；`IVisualContribution: IContribution` 继承关系零修改 |

***

## 过度设计清单（待清理）

| 类型/文件                                                                   | 位置                                                  | 清理动作                                                                                                                                        |
| ----------------------------------------------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `ContributedEntry` 结构体                                                  | `crates/core/src/contribution.rs`                   | **删除** —— 框架不存储贡献条目                                                                                                                         |
| `VisualRenderer` 类型别名                                                   | `crates/core/src/contribution.rs`                   | **删除** —— `IVisualContribution::render` 直接接 `&mut Window, &mut App`                                                                         |
| `ComponentEntityCache` trait                                            | `crates/core/src/contribution.rs`                   | **删除** —— 框架不缓存 Entity                                                                                                                      |
| `RenderContext` 结构体                                                     | `crates/core/src/contribution.rs`                   | **删除** —— `IVisualContribution::render` 直接接 `&mut Window, &mut App`                                                                         |
| `ComponentEntityCacheImpl` 结构体                                          | `crates/core/src/contribution_cache.rs`             | **删除整文件**                                                                                                                                   |
| `ContributionHost` 结构体                                                  | `crates/app/src/contribution/host.rs`               | **删除整文件** —— 框架不存储 host                                                                                                                     |
| `Registerable` trait                                                    | `crates/app/src/contribution/registerable.rs`       | **删除整文件** —— 宏不生成 `Registerable` impl                                                                                                       |
| `data_entry`/`component_entry`/`add_entry`                              | `crates/app/src/contribution/entry.rs`              | **删除整文件** —— 不再构建 `ContributedEntry`                                                                                                        |
| `render_component_view`/`render_contribution_visual`                    | `crates/app/src/contribution/render.rs`             | **删除整文件** —— host 通过 `rml_app::contribution::extract_visual(&contribution)` 自由函数获取 `Arc<dyn IVisualContribution>` 后直接调 `render(window, cx)` |
| `map_activity_panels`/`ContributedActivityPanel`                        | `crates/app/src/contribution/activity_panel.rs`     | **移至 demo**（业务投影代码）                                                                                                                         |
| `register_contribution` 中间函数                                            | `crates/app/src/contribution/global.rs`             | **删除** —— 宏直接调 `cx.get_contribution_registry().register(...)`                                                                               |
| `ContributionExt` trait（add/remove/register/unregister）                 | `crates/app/src/contribution/global.rs`             | **替换为** `ContributionRegistryExt::get_contribution_registry()`                                                                              |
| `contribution_entries`/`contribution_revision`/`subscribe_host_changes` | `crates/app/src/contribution/global.rs`             | **删除** —— host 自管存储；跨 Entity 用 `cx.observe`                                                                                                 |
| `HostRegistrar` + `parse_host_registrars` + host 扫描                     | `crates/engine/src/build/contribution_generator.rs` | **删除** —— host 不再 bootstrap 预注册                                                                                                             |

***

## 当前状态分析

### 框架侧（待清理）

* **`crates/core/src/contribution.rs`**（132 行）：定义 7 类实体——`VisualRenderer` 别名、`ContributionOptions`、`IContribution`（仅元数据）、`IVisualContribution`（含 render）、`IContributionHost`（仅 `const ID`）、`ContributedEntry`（框架存储）、`RenderContext`、`ComponentEntityCache` trait。其中 `ContributionOptions`/`IContribution`/`IVisualContribution` 保留（IVisualContribution 签名调整），其余待清理或重写。

* **`crates/core/src/contribution_cache.rs`**（77 行）：`ComponentEntityCacheImpl` —— 框架侧 Entity 缓存，待整文件删除。

* **`crates/core/src/prelude.rs`**：导出 `ContributedEntry/ContributionOptions/IContribution/IContributionHost/IVisualContribution/RenderContext`，需调整。

* **`crates/app/src/contribution/`**（8 文件）：`mod.rs`、`global.rs`、`host.rs`、`registry.rs`、`entry.rs`、`registerable.rs`、`render.rs`、`activity_panel.rs`。除 `mod.rs`/`global.rs`/`registry.rs` 重写外，其余 5 个文件全部删除或迁移。

* **`crates/macros/src/contribute.rs`**（327 行）：生成 `IContribution` impl + `Registerable` impl + `IVisualContribution` impl（其 `render` 委托 `render_component_view`） + `__rml_register_<name>` 函数（调用 `register_contribution`）。其中 `Registerable` impl 与 `render_component_view` 委托待移除；`IVisualContribution` impl 保留（签名调整）；`__rml_register_<name>` 改为单行 `cx.get_contribution_registry().register(...)`。

* **`crates/macros/src/contributehost.rs`**：生成 `const ID` + 断言 + `__rml_register_<name>` 调用 `cx.add(ID)`。

* **`crates/engine/src/build/contribution_generator.rs`**：扫描 `#[contributehost]` 与 `#[contribute]`，生成 `register_rml_contributions(cx)` 调用各 `__rml_register_<name>`。

### 业务侧（待重构）

* **`demo/src/shell/main_window.rml.rs`**（261 行）：`IContributionHost` 仅 `const ID`；`on_loaded` 内调用 `cx.update_global::<ContributionRegistryGlobal, _>(|g, _| g.0.entity_cache_mut().pre_register(...))` 直接侵入框架缓存；`refresh_shell_chrome` 调用 `map_shell_chrome` 投影；`active_case_view` 调用 `contribution_entries` + `render_contribution_visual`。

* **`demo/src/shell/shell_chrome.rs`**（160 行）：`map_shell_chrome`/`map_menu_items`/`map_status_items`/`map_case_tree_items` 投影函数，从 `contribution_entries` 读取。

* **`demo/src/shell/activity_panel.rml.rs`**（64 行）：`subscribe_host_changes(MainWindow::ID, cx, ...)` + `map_case_tree_items(MainWindow::ID, cx)`。

***

## 提议变更

### Phase 1：核心类型精简

#### 1.1 重写 `crates/core/src/contribution.rs`

```rust
use std::any::Any;
use std::sync::Arc;
use gpui::{AnyElement, App, SharedString, Window};

/// 能力贡献点：仅元数据，不渲染。
/// 业务贡献（菜单项、状态栏项、案例树节点等）实现此 trait。
/// 添加 `Any` supertrait——使 `dyn IContribution` 支持 trait upcasting 到 `dyn Any`，
/// 配合宏生成的视觉提取器实现 `Arc<dyn IVisualContribution>` 向下转型。
pub trait IContribution: Send + Sync + Any {
    fn id(&self) -> &str;
    fn name(&self) -> SharedString;
    fn description(&self) -> SharedString { SharedString::default() }
    fn icon(&self) -> Option<SharedString> { None }
}

/// 可视化贡献点：能渲染 UI 元素的贡献。
/// `IVisualContribution: IContribution`——视觉贡献同时是能力贡献（含元数据）。
/// 业务视觉贡献（如 `ActivityPanel`）实现此 trait，由 `#[contribute]` + `#[component]` 宏自动生成。
pub trait IVisualContribution: IContribution {
    /// 渲染贡献视图。host 调用此方法获取 `AnyElement`，自行决定是否缓存结果。
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}

/// 贡献点注册元数据（保留原结构，纯数据）。
pub struct ContributionOptions { /* order, parent_id, group, slot, properties ... */ }
impl ContributionOptions { /* builder 方法不变 */ }

/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
/// host 可使用 Vec/HashMap/任何自定义结构，甚至不存储——框架不限定。
pub trait IContributionHost: Send + Sync + 'static {
    const ID: &'static str;

    /// 受理代码：接收并处置贡献。host 按 options.slot/group 等分发到自有数据结构。
    fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);

    /// 移除贡献。host 自行清理对应数据。
    fn remove(&mut self, contribution_id: &str, cx: &mut App);
}

/// 内部桥接 trait：类型擦除的 host 句柄，包装 WeakEntity<T>。
#[doc(hidden)]
pub trait HostHandle: Send + Sync {
    fn id(&self) -> &str;
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
    fn remove(&self, contribution_id: &str, cx: &mut App);
}

/// 视觉提取器函数类型：从 `Arc<dyn IContribution>` 提取 `Arc<dyn IVisualContribution>`。
/// 由 `#[contribute]` 宏为视觉贡献生成，注册到 registry。
/// 利用 `Any` supertrait + trait upcasting coercion（Rust 1.86+）：
///   `Arc<dyn IContribution>` → `Arc<dyn Any + Send + Sync>` → `Arc::downcast::<T>()` → `Arc<T> as Arc<dyn IVisualContribution>`
#[doc(hidden)]
pub type VisualExtractor = fn(&Arc<dyn IContribution>) -> Option<Arc<dyn IVisualContribution>>;

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add 方法。
/// trait 仅含 4 个用户决策方法——视觉提取器注册/查找为 `#[doc(hidden)]` 自由函数，
/// 由 `#[ctor::ctor]` 在进程启动期写入进程级静态表，host 通过 `rml_app::contribution::extract_visual` 查找。
pub trait IContributionRegistry: Send + Sync {
    fn add(&self, host: Box<dyn HostHandle>, cx: &mut App);
    fn remove(&self, host_id: &str, cx: &mut App);
    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
```

**要点**：

* `IContribution` **不添加任何新方法**（无 `as_visual`/`render_view` 桥接）——仅 supertraits 增加 `Any` marker bound，`description`/`icon` 增加默认实现（向后兼容）。`IVisualContribution: IContribution` 继承关系不变。

* `IVisualContribution::render` 签名由 `fn render(&self, ctx: &mut RenderContext<'_>) -> AnyElement` 调整为 `fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement`——这是删除 `RenderContext`（用户决策 #8 批准清理）的必要后果：`RenderContext` 类型不再存在，方法不能继续接此参数。方法语义不变（仍为"渲染贡献视图"），仅参数类型从框架包装类型改为 gpui 标准渲染签名。

* **不加** **`as_visual()`** **方法**——用户禁止修改 trait 方法签名。改用 `Any` supertrait + 宏生成的 `VisualExtractor` 函数实现类型识别。

* **视觉提取器工作原理**（利用 Rust 1.86+ trait upcasting coercion）：

  1. `IContribution: Any` 使 `Arc<dyn IContribution>` 可 upcast 为 `Arc<dyn Any + Send + Sync>`
  2. 宏为视觉贡献生成提取器：`fn(contrib: &Arc<dyn IContribution>) -> Option<Arc<dyn IVisualContribution>>`
  3. 提取器内部：`let any: Arc<dyn Any + Send + Sync> = contrib.clone(); any.downcast::<T>().ok().map(|a| a as Arc<dyn IVisualContribution>)`
  4. `#[contribute]` 宏生成 `#[ctor::ctor]` 函数，在进程启动期将 `TypeId::of::<T>()` → 提取器 写入 `rml_app::contribution` 模块内的进程级静态 `RwLock<HashMap<TypeId, VisualExtractor>>`（无需 `App` 上下文，因 ctor 先于 App 存在）
  5. host 在 `add` 内调 `rml_app::contribution::extract_visual(&contribution)` 查找提取器，若命中则获得 `Arc<dyn IVisualContribution>` 并存储

* **`IContributionHost::add`** **签名不变**——接收 `Arc<dyn IContribution>`。host 通过 `options.slot()` 分发，视觉贡献的渲染由 host 内部调用 `IVisualContribution::render` 完成。host 通过 `rml_app::contribution::extract_visual(&contribution)` 获取 `Option<Arc<dyn IVisualContribution>>`，**无需 host 知晓具体视觉类型**——提取器由宏在 `#[ctor::ctor]` 阶段注册到进程级静态表，按 `TypeId` 自动查找。保持 `IContributionHost` 仅有 `id`/`add`/`remove` 三方法，trait 签名零修改；`IContributionRegistry` trait 仅含 4 个用户决策方法。

* `IVisualContribution::render(&self, window: &mut Window, cx: &mut App) -> AnyElement` 直接接 gpui 标准渲染签名——无 `RenderContext`/`VisualRenderer` 包装。

* `HostHandle` 内部 trait `#[doc(hidden)]`，用户不直接接触。通过 `register_host(cx)` 封装构造。

* `IContributionRegistry` trait 方法 `&self` + 内部 `RwLock` 可变性，使 `get_contribution_registry()` 可返回 `&dyn IContributionRegistry`。

#### 1.2 删除 `crates/core/src/contribution_cache.rs`

整文件删除。`crates/core/src/lib.rs` 移除 `pub mod contribution_cache;` 声明。

#### 1.3 调整 `crates/core/src/lib.rs`

```rust
pub mod contribution;  // 保留
// pub mod contribution_cache;  // 删除
```

#### 1.4 调整 `crates/core/src/prelude.rs`

```rust
pub use crate::contribution::{
    ContributionOptions, HostHandle, IContribution, IContributionHost,
    IContributionRegistry,  // 新增
    IVisualContribution,   // 保留（视觉贡献 trait）
};
// 移除：ContributedEntry, RenderContext, VisualRenderer, ComponentEntityCache
```

***

### Phase 2：App 层重写

#### 2.1 重写 `crates/app/src/contribution/registry.rs`

```rust
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::OnceLock;

use gpui::App;
use rml_core::contribution::{
    ContributionOptions, HostHandle, IContribution, IContributionHost,
    IContributionRegistry, IVisualContribution, VisualExtractor,
};

/// 进程级视觉提取器表——由 `#[contribute]` 宏生成的 `#[ctor::ctor]` 在进程启动期写入。
/// `OnceLock<RwLock<...>>` 保证 ctor 早期即可写入（ctor 先于 App 存在）。
static VISUAL_EXTRACTORS: OnceLock<RwLock<HashMap<TypeId, VisualExtractor>>> = OnceLock::new();

fn visual_extractors() -> &'static RwLock<HashMap<TypeId, VisualExtractor>> {
    VISUAL_EXTRACTORS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// `#[contribute]` 宏在 `#[ctor::ctor]` 中调用：注册视觉提取器。
#[doc(hidden)]
pub fn register_visual_extractor(type_id: TypeId, extractor: VisualExtractor) {
    visual_extractors().write().unwrap().insert(type_id, extractor);
}

/// host 在 `add` 内调用：按 `TypeId` 查找提取器，返回 `Arc<dyn IVisualContribution>`。
pub fn extract_visual(contribution: &Arc<dyn IContribution>) -> Option<Arc<dyn IVisualContribution>> {
    let type_id = (**contribution).type_id();
    let extractors = visual_extractors().read().unwrap();
    extractors.get(&type_id).and_then(|f| f(contribution))
}

/// 框架内部实现：桥接 contribute → host
pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, Box<dyn HostHandle>>>,
    pending: RwLock<HashMap<String, Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
        }
    }

    pub fn has_host(&self, host_id: &str) -> bool {
        self.hosts.read().unwrap().contains_key(host_id)
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add(&self, host: Box<dyn HostHandle>, cx: &mut App) {
        let id = host.id().to_string();
        let mut hosts = self.hosts.write().unwrap();
        hosts.insert(id.clone(), host);
        drop(hosts);

        // 重放 pending 队列
        let mut pending = self.pending.write().unwrap();
        let queue = pending.remove(&id).unwrap_or_default();
        drop(pending);

        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(&id) {
            for (contribution, options) in queue {
                host.add(contribution, options, cx);
            }
        }
    }

    fn remove(&self, host_id: &str, _cx: &mut App) {
        self.hosts.write().unwrap().remove(host_id);
    }

    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options, cx);
        } else {
            drop(hosts);
            self.pending.write().unwrap()
                .entry(host_id.to_string()).or_default()
                .push((contribution, options));
        }
    }

    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.remove(contribution_id, cx);
            true
        } else {
            false
        }
    }
}
```

#### 2.2 重写 `crates/app/src/contribution/global.rs`

```rust
use gpui::{App, Global};
use rml_core::contribution::{
    HostHandle, IContribution, IContributionHost, IContributionRegistry,
    ContributionOptions,
};

#[doc(hidden)]
pub struct ContributionRegistryGlobal(pub super::registry::ContributionRegistry);
impl Global for ContributionRegistryGlobal {}

pub fn ensure_contribution_registry(cx: &mut App) {
    if !cx.has_global::<ContributionRegistryGlobal>() {
        cx.set_global(ContributionRegistryGlobal(super::registry::ContributionRegistry::new()));
    }
}

/// App 扩展：获取 IContributionRegistry 接口。
/// 方法通过 RwLock 内部可变性操作，返回 &dyn IContributionRegistry。
pub trait ContributionRegistryExt {
    fn get_contribution_registry(&mut self) -> &dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&mut self) -> &dyn IContributionRegistry {
        ensure_contribution_registry(self);
        &self.global::<ContributionRegistryGlobal>().0
    }
}

/// host 在 on_loaded 中调用：注册自身为贡献 host。
/// registry 会重放此前通过 #[ctor::ctor] 注册的 pending 贡献到 host.add。
pub fn register_host<T: IContributionHost + gpui::Render + 'static>(cx: &mut gpui::Context<T>) {
    let weak = cx.weak_entity();
    cx.get_contribution_registry().add(EntityHostHandleBox { weak }, cx);
}

#[doc(hidden)]
pub struct EntityHostHandleBox<T: IContributionHost + gpui::Render + 'static> {
    weak: gpui::WeakEntity<T>,
}

impl<T: IContributionHost + gpui::Render + 'static> HostHandle for EntityHostHandleBox<T> {
    fn id(&self) -> &str { T::ID }

    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
        if let Some(entity) = self.weak.upgrade() {
            entity.update(cx, |host, ctx| {
                host.add(contribution, options, ctx);
                ctx.notify();
            });
        }
    }

    fn remove(&self, contribution_id: &str, cx: &mut App) {
        if let Some(entity) = self.weak.upgrade() {
            entity.update(cx, |host, ctx| {
                host.remove(contribution_id, ctx);
                ctx.notify();
            });
        }
    }
}
```

**要点**：

* `EntityHostHandleBox<T>` 在 app 层定义（依赖 `gpui::Render` bound，core 层不依赖 gpui render）。

* `register_host<T>` 从 `Context<T>::weak_entity()` 获取 WeakEntity，构造 handle 后 `registry.add`。

* `get_contribution_registry()` 返回 `&dyn IContributionRegistry`，宏生成代码与业务代码均通过此接口操作。

#### 2.3 删除文件

| 文件                                              | 操作                                   |
| ----------------------------------------------- | ------------------------------------ |
| `crates/app/src/contribution/host.rs`           | **删除**                               |
| `crates/app/src/contribution/entry.rs`          | **删除**                               |
| `crates/app/src/contribution/registerable.rs`   | **删除**                               |
| `crates/app/src/contribution/render.rs`         | **删除**                               |
| `crates/app/src/contribution/activity_panel.rs` | **移至** **`demo/src/shell/`**（业务投影代码） |

#### 2.4 重写 `crates/app/src/contribution/mod.rs`

```rust
mod global;
mod registry;

pub use global::{
    ContributionRegistryExt, ensure_contribution_registry, register_host,
};
pub use registry::extract_visual;
#[doc(hidden)]
pub use global::{ContributionRegistryGlobal, EntityHostHandleBox};
#[doc(hidden)]
pub use registry::{ContributionRegistry, register_visual_extractor};
```

**移除导出**：`bootstrap_contributions`、`contribution_entries`、`register_contribution`、`subscribe_host_changes`、`Registerable`、`ensure_contribution_registry`（如未被外部使用）。保留 `ensure_contribution_registry` 供 `#[ctor::ctor]` bootstrap 调用。

#### 2.5 调整 `crates/app/src/lib.rs`

```rust
pub use contribution::{
    ContributionRegistryExt, ensure_contribution_registry, extract_visual, register_host,
};
```

移除 `bootstrap_contributions`、`contribution_entries`、`register_contribution`、`subscribe_host_changes`、`Registerable` 导出。

***

### Phase 3：宏与 build 简化

#### 3.1 简化 `crates/macros/src/contribute.rs`

**移除**：`Registerable` impl 生成、`use rml_app::contribution::register_contribution;` 引用、`render_component_view` 委托。

**保留并精简**：`IContribution` impl（方法签名不变）+ `IVisualContribution` impl（仅视觉贡献）+ `__rml_register_<name>(cx)` 单行注册函数 + 视觉提取器 `#[ctor::ctor]` 注册（仅视觉贡献）。

```rust
// 宏生成的最终代码形态（以 #[contribute(host_id="demo.shell", id="menu.file", name="menu.file", kind="menu", order=0)] 为例）：
//
// // —— 能力贡献 impl（所有贡献均生成，方法签名不变）——
// impl rml_core::contribution::IContribution for MenuFileRoot {
//     fn id(&self) -> &str { "menu.file" }
//     fn name(&self) -> gpui::SharedString { rml_core::i18n::t_static("menu.file").into() }
//     fn description(&self) -> gpui::SharedString { /* from args or default */ }
//     fn icon(&self) -> Option<gpui::SharedString> { /* from args or None */ }
// }
//
// // —— 可视化贡献 impl（仅当 args.visual || has_component 时生成）——
// // impl rml_core::contribution::IVisualContribution for MenuFileRoot {
// //     fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
// //         <Self as rml_core::component::IComponent>::render(self, window, cx)
// //     }
// // }
//
// // —— 视觉提取器注册（仅视觉贡献生成，#[ctor::ctor] 在进程启动时注册到进程级静态表）——
// // #[rml_core::ctor::ctor]
// // fn __rml_register_visual_extractor_menufileroot() {
// //     rml_app::contribution::register_visual_extractor(
// //         std::any::TypeId::of::<MenuFileRoot>(),
// //         |contrib: &std::sync::Arc<dyn rml_core::contribution::IContribution>|
// //             -> Option<std::sync::Arc<dyn rml_core::contribution::IVisualContribution>>
// //         {
// //             // 利用 trait upcasting coercion (Rust 1.86+): Arc<dyn IContribution> → Arc<dyn Any + Send + Sync>
// //             let any: std::sync::Arc<dyn std::any::Any + Send + Sync> = contrib.clone();
// //             any.downcast::<MenuFileRoot>().ok().map(|a| a as std::sync::Arc<dyn rml_core::contribution::IVisualContribution>)
// //         },
// //     );
// // }
//
// pub fn __rml_register_menufileroot(cx: &mut gpui::App) {
//     use rml_app::contribution::ContributionRegistryExt;
//     cx.get_contribution_registry().register(
//         "demo.shell",
//         std::sync::Arc::new(MenuFileRoot::default()),
//         rml_core::contribution::ContributionOptions::new()
//             .slot("menu").order(0),
//         cx,
//     );
// }
```

**关键变化**：

* `Registerable` impl 删除

* `IContribution` impl 方法签名完全不变——不加 `as_visual`（trait 无此方法），不加 `render_view`。仅 `IContribution` supertraits 在 core 层声明为 `Send + Sync + Any`，由宏生成的 impl 自动满足（无需手动标注）。

* `IVisualContribution` impl 保留：仅视觉贡献生成，`render` 方法委托给 `IComponent::render(self, window, cx)`。无 `render_component_view` 中间层、无 `VisualRenderer` 闭包。

* **视觉提取器**（`VisualExtractor`）：宏为视觉贡献额外生成 `#[ctor::ctor]` 函数，在进程启动时调 `rml_app::contribution::register_visual_extractor(TypeId::of::<T>(), extractor)` 将提取器写入进程级静态表（不依赖 `App` 上下文）。提取器利用 `Any` supertrait + Rust 1.86+ trait upcasting coercion 实现 `Arc<dyn IContribution>` → `Arc<dyn Any + Send + Sync>` → `Arc::downcast::<T>()` → `Arc<dyn IVisualContribution>` 转型链。host 在 `add` 内调 `rml_app::contribution::extract_visual(&contribution)` 查找。

* `__rml_register_<name>` 函数体从 4 行（创建 Arc + 创建 options + 调 `register_contribution`）压缩为单行 `cx.get_contribution_registry().register(...)` 调用

#### 3.2 简化 `crates/macros/src/contributehost.rs`

**移除**：`__rml_register_<name>` 函数生成（host 不再 bootstrap 时预注册）、隐藏 `mod __rml_host_*` 模块。

**保留**：`const ID` 生成 + 编译期断言。

```rust
// 宏生成的最终代码形态：
//
// impl MainWindow {
//     pub const ID: &'static str = "demo.shell";
// }
//
// const _: () = {
//     fn assert_host<T: rml_core::contribution::IContributionHost>() {}
//     fn check() { assert_host::<MainWindow>(); }
// };
```

#### 3.3 调整 `crates/engine/src/build/contribution_generator.rs`

**移除**：`HostRegistrar` 结构、`parse_host_registrars` 函数、`dedup_hosts` 函数、host 扫描循环。

**保留**：`ContributionRegistrar` 结构、`parse_contribution_registrars`、`dedup_contributions`、贡献扫描。

**生成函数简化**：

```rust
pub fn generate(
    contributions: &[ContributionRegistrar],
    output_dir: &Path,
) -> Result<(), BuildError> {
    // ... 生成 rml_contributions.rs：
    //
    // pub fn register_rml_contributions(cx: &mut gpui::App) {
    //     crate::shell::menu_shell_contribs::__rml_register_menufileroot(cx);
    //     crate::shell::menu_shell_contribs::__rml_register_menufilenew(cx);
    //     // ... 仅贡献注册函数调用
    // }
    //
    // #[rml_core::ctor::ctor]
    // fn __rml_install_contribution_bootstrap() {
    //     rml_app::contribution::install_contribution_bootstrap(register_rml_contributions);
    // }
}
```

`scan_contribution_registrars` 返回签名改为 `Vec<ContributionRegistrar>`（不再返回 host 元组）。

#### 3.4 调整 build.rs 调用点

调用 `contribution_generator::scan_contribution_registrars` 的 `build.rs`（在 `crates/engine/src/build.rs` 或 `demo/build.rs`）相应调整：不再接收 host 元组。

***

### Phase 4：Demo 受理代码

#### 4.1 重写 `demo/src/shell/main_window.rml.rs` 的 `IContributionHost` impl

```rust
use std::collections::HashMap;
use std::sync::Arc;
use rml_core::contribution::{IContribution, IContributionHost, IVisualContribution, ContributionOptions};
use rml_app::contribution::{register_host, extract_visual};

#[contributehost(id = "demo.shell")]
#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    i18n_version: u32,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
    slot_left_size: gpui::Pixels,

    // —— host 自管存储：按业务选择 Vec / HashMap，框架不限定 ——
    // 视觉贡献在 add 时即通过 extract_visual 提取为 Arc<dyn IVisualContribution> 存储，
    // 渲染时直接调 visual.render(window, cx)，无需运行时向下转型。
    menu_items: MenuItems,                                   // 菜单树（仅元数据）
    status_items: StatusBarItems,                           // 状态栏项（仅元数据）
    activity_panels: Vec<Arc<dyn IVisualContribution>>,     // ActivityBar 面板（视觉）
    active_panel_id: Option<String>,                       // 当前激活面板 id
    case_tree_items: Vec<TreeItem>,                         // 案例树（仅元数据）
}

impl IContributionHost for MainWindow {
    const ID: &'static str = "demo.shell";

    fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, _cx: &mut App) {
        match options.effective_slot().unwrap_or("") {
            "menu" => self.add_menu_item(&contribution, &options),
            "status" => self.add_status_item(&contribution, &options),
            "activity" => self.add_activity_panel(&contribution, &options),
            "case" => self.add_case_item(&contribution, &options),
            _ => {}
        }
        // host 自行决定是否调用 cx.notify()——取决于是否需要立即重渲
    }

    fn remove(&mut self, contribution_id: &str, _cx: &mut App) {
        self.remove_menu_item(contribution_id);
        self.remove_status_item(contribution_id);
        self.remove_activity_panel(contribution_id);
        self.remove_case_item(contribution_id);
    }
}

impl MainWindow {
    fn add_menu_item(&mut self, c: &Arc<dyn IContribution>, o: &ContributionOptions) {
        // 将贡献插入 menu_items 树（按 parent_id / order）
        // ... 业务逻辑 ...
    }

    fn add_status_item(&mut self, c: &Arc<dyn IContribution>, o: &ContributionOptions) {
        // ... 业务逻辑 ...
    }

    /// ActivityBar 面板是视觉贡献——add 时通过 extract_visual 提取 Arc<dyn IVisualContribution> 后存储。
    /// 提取器由 #[contribute] 宏在 #[ctor::ctor] 阶段注册到进程级静态表，按 TypeId 自动查找。
    fn add_activity_panel(&mut self, c: &Arc<dyn IContribution>, o: &ContributionOptions) {
        if let Some(visual) = extract_visual(c) {
            // host 可在此处创建并缓存 Entity<T> 以保留组件状态（如需）
            self.activity_panels.push(visual);
        }
    }

    // ... 其他 add_* / remove_* 方法 ...

    /// 渲染当前激活的视觉贡献（host 自管，无框架介入）
    /// activity_panels 已存储 Arc<dyn IVisualContribution>，直接调 render——无需运行时向下转型。
    pub fn active_case_view(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        if let Some(visual) = self.find_active_activity_visual() {
            return visual.render(window, cx);
        }
        gpui::div().into_any_element()
    }

    fn find_active_activity_visual(&self) -> Option<&Arc<dyn IVisualContribution>> {
        let active_id = self.active_panel_id.as_ref()?;
        self.activity_panels.iter().find(|v| v.id() == active_id)
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // ... 初始化 open_tabs, menu_commands ...

        // 注册自身为贡献 host → registry 重放 pending 贡献 → 逐条调 host.add
        register_host(cx);

        // ... 创建 ActivityBar、observe 等 ...
    }
}
```

**要点**：

* host 存储字段类型由业务决定（这里用 `MenuItems`/`StatusBarItems`/`Vec<Arc<dyn IVisualContribution>>`/`Vec<TreeItem>`，**不强制** **`ObservableVec`**）。

* `add`/`remove` 是受理代码——按 `options.slot()` 分发到对应 `add_*`/`remove_*` 辅助方法。

* **视觉贡献在** **`add`** **时即通过** **`extract_visual(&contribution)`** **提取为** **`Arc<dyn IVisualContribution>`** **并存储**——`active_case_view` 直接调 `visual.render(window, cx)`，无运行时向下转型、无 `RenderContext`/`entity_cache`/`render_component_view` 中间层。host 自行决定是否额外缓存渲染结果或视觉贡献 Entity。

* `on_loaded` 调 `register_host(cx)` 注册自身；移除 `refresh_shell_chrome` 调用、移除 `cx.update_global::<ContributionRegistryGlobal, _>(...)` 侵入、移除 `subscribe_host_changes(Self::ID, ...)`。

* 视觉贡献 Entity 缓存由 host 在 `add_activity_panel` 内自行决定（如需保留状态可缓存 `HashMap<String, Entity<T>>`，框架不规定）。

#### 4.2 调整 `demo/src/shell/shell_chrome.rs`

* **删除** `map_shell_chrome` / `ShellChromeBindings`（不再需要投影层）

* `map_menu_items` / `map_status_items` / `map_case_tree_items` / `map_activity_panels` 的逻辑**移入** **`MainWindow`** **的** **`add_menu_item`** **/** **`add_status_item`** **/** **`add_case_item`** **/** **`add_activity_panel`** **方法**——贡献在 `add` 时直接写入 host 字段，无需批量投影

* 工具函数（如 `t_static("tree.group.{g}")` 等）可保留在 `shell_chrome.rs` 中，由 `MainWindow::add_*` 调用

#### 4.3 重写 `demo/src/shell/activity_panel.rml.rs`

```rust
#[contribute(host_id = "demo.shell", id = "samples", name = "shell.samples", icon = IconName::BookOpen, kind = "activity", order = 0)]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_tree(cx);

        // 通过 DemoShellHost WeakEntity observe MainWindow → 案例树变更时自动刷新
        if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
            cx.observe(&host, |this, main, cx| {
                this.set_tree_items(main.case_tree_items(), cx);
                cx.notify();
            }).detach();
        }

        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        }).detach();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
            let items = host.read(cx).case_tree_items();
            self.set_tree_items(items, cx);
        }
    }

    fn set_tree_items(&mut self, items: Vec<TreeItem>, cx: &mut Context<Self>) {
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| s.set_items(items, cx));
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
            host.update(cx, |main, cx| main.open_case(item_id.to_string(), cx));
        }
    }
}
```

**关键变化**：

* `subscribe_host_changes(MainWindow::ID, cx, ...)` → `cx.observe(&host_entity, ...)` 直接 observe MainWindow Entity

* `map_case_tree_items(MainWindow::ID, cx)` → `host.read(cx).case_tree_items()` 读 host 自身字段

* `MainWindow::case_tree_items()` 为普通方法（若需要缓存可用现有 `#[computed]`）

#### 4.4 `menu_shell_contribs.rs` 保留

`#[contribute]` 声明不变——贡献点声明本身是数据驱动的，宏自动生成注册代码。变化在于注册路由：`__rml_register_<name>(cx)` 内部直接调 `cx.get_contribution_registry().register(...)` → `registry.register` → `host.add`（受理代码分发）。

***

## 验证步骤

### Phase 1 验证

```bash
cargo build -p rust-rml-core
```

* `crates/core/src/contribution.rs` 重写后编译通过

* `contribution_cache.rs` 已删除，`lib.rs`/`prelude.rs` 无悬空引用

* `ContributedEntry`/`VisualRenderer`/`RenderContext`/`ComponentEntityCache` 类型不再存在；`IContribution`/`IVisualContribution`/`IContributionHost`/`IContributionRegistry`/`HostHandle` 正确导出

### Phase 2 验证

```bash
cargo build -p rust-rml-app
cargo test -p rust-rml-app -- contribution
```

验证：

1. `ContributionRegistry::register` 路由到 host.add（host 存在时）
2. pending 队列：host 未注册时入队，`add` 后重放
3. `get_contribution_registry()` 返回 `&dyn IContributionRegistry` 可操作
4. `register_host<T>(cx)` 正确构造 `EntityHostHandleBox` 并 `add` 到 registry
5. `register_visual_extractor(type_id, extractor)` 写入进程级静态表；`extract_visual(&contribution)` 按 `TypeId` 查找命中

### Phase 3 验证

```bash
cargo build -p rust-rml-macros -p rust-rml-engine
cargo expand -p rust-rml-demo shell::menu_shell_contribs::MenuFileRoot 2>&1 | grep -A 5 "__rml_register_menufileroot"
```

验证宏展开：

1. `__rml_register_<name>` 函数体为单行 `cx.get_contribution_registry().register(...)` 调用
2. 无 `Registerable` impl、无 `register_contribution` 引用、无 `render_component_view` 委托
3. 视觉贡献（`#[contribute]` + `#[component]`）：生成 `IVisualContribution` impl + `#[ctor::ctor]` 视觉提取器注册函数（调 `rml_app::contribution::register_visual_extractor`）
4. 非视觉贡献（`#[contribute]` 无 `#[component]`）：仅生成 `IContribution` impl，无 `IVisualContribution` impl、无提取器注册
5. `#[contributehost]` 仅生成 `const ID` + 断言，无 `__rml_register_*` 函数
6. `register_rml_contributions(cx)` 仅调用 `#[contribute]` 注册函数
7. **`IContribution`** **trait 不添加新方法**（无 `as_visual`/`render_view` 桥接）；`IVisualContribution::render` 参数由 `&mut RenderContext` 调整为 `(&mut Window, &mut App)`（`RenderContext` 已删除的必要后果）

### Phase 4 验证

```bash
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

验证：

1. Demo 启动后 menu/status/activity 面板正确显示
2. 通过菜单打开 case → tab 新增 → UI 更新（无 `refresh_shell_chrome` 调用）
3. ActivityPanel 案例树正确显示，`cx.observe(&host, ...)` 自动刷新
4. 切换语言 → 菜单标题更新
5. **无** **`contribution_entries`** **调用出现在 demo 代码中**
6. **无** **`subscribe_host_changes`** **调用出现在 demo 代码中**
7. **`MainWindow::add`/`remove`** **受理代码正确按 slot 分发**

***

## 关键文件清单

| 文件                                                  | Phase | 操作                                                                                                                                                                                                                                  |
| --------------------------------------------------- | ----- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crates/core/src/contribution.rs`                   | 1     | **重写**：`IContribution`（`Any` supertrait）+ `IVisualContribution`（`render`）+ `IContributionHost` add/remove + `IContributionRegistry` + `HostHandle` + `VisualExtractor` 类型别名                                                         |
| `crates/core/src/contribution_cache.rs`             | 1     | **删除整文件**                                                                                                                                                                                                                           |
| `crates/core/src/lib.rs`                            | 1     | 移除 `contribution_cache` 模块声明                                                                                                                                                                                                        |
| `crates/core/src/prelude.rs`                        | 1     | 调整导出：移除 `ContributedEntry`/`RenderContext`/`VisualRenderer`/`ComponentEntityCache`，新增 `IContributionRegistry`，保留 `IVisualContribution`                                                                                              |
| `crates/app/src/contribution/mod.rs`                | 2     | **重写**：模块声明与导出                                                                                                                                                                                                                      |
| `crates/app/src/contribution/global.rs`             | 2     | **重写**：`ContributionRegistryExt` + `register_host` + `EntityHostHandleBox`                                                                                                                                                          |
| `crates/app/src/contribution/registry.rs`           | 2     | **重写**：`ContributionRegistry` impl `IContributionRegistry`（RwLock + pending）+ 进程级静态 `VISUAL_EXTRACTORS` + `register_visual_extractor`/`extract_visual` 自由函数                                                                         |
| `crates/app/src/contribution/host.rs`               | 2     | **删除整文件**                                                                                                                                                                                                                           |
| `crates/app/src/contribution/entry.rs`              | 2     | **删除整文件**                                                                                                                                                                                                                           |
| `crates/app/src/contribution/registerable.rs`       | 2     | **删除整文件**                                                                                                                                                                                                                           |
| `crates/app/src/contribution/render.rs`             | 2     | **删除整文件**                                                                                                                                                                                                                           |
| `crates/app/src/contribution/activity_panel.rs`     | 2     | **移至** **`demo/src/shell/activity_panel_mapping.rs`**（业务投影代码）                                                                                                                                                                       |
| `crates/app/src/lib.rs`                             | 2     | 调整导出                                                                                                                                                                                                                                |
| `crates/macros/src/contribute.rs`                   | 3     | **简化**：移除 `Registerable` impl + `render_component_view` 委托；保留 `IVisualContribution` impl（仅视觉贡献）+ `#[ctor::ctor]` 视觉提取器注册（调 `register_visual_extractor`）；`__rml_register_<name>` 改为单行 `cx.get_contribution_registry().register(...)` |
| `crates/macros/src/contributehost.rs`               | 3     | **简化**：仅生成 `const ID` + 断言，移除 `__rml_register_*`                                                                                                                                                                                    |
| `crates/engine/src/build/contribution_generator.rs` | 3     | **简化**：移除 host 扫描，仅保留贡献扫描                                                                                                                                                                                                           |
| `demo/src/shell/main_window.rml.rs`                 | 4     | **重写**：`IContributionHost::add`/`remove` 受理代码 + `register_host(cx)` + `extract_visual` 提取视觉贡献存储 + 移除 `refresh_shell_chrome`/`subscribe_host_changes`                                                                                |
| `demo/src/shell/shell_chrome.rs`                    | 4     | **删除** `map_shell_chrome`/`ShellChromeBindings`；映射逻辑移入 `MainWindow::add_*` 方法                                                                                                                                                       |
| `demo/src/shell/activity_panel.rml.rs`              | 4     | **重写**：`cx.observe(&host, ...)` 替代 `subscribe_host_changes`                                                                                                                                                                         |
| `demo/src/shell/menu_shell_contribs.rs`             | 4     | 不变                                                                                                                                                                                                                                  |

***

## 假设与决策

### 假设

1. **pending 队列**：`#[ctor::ctor]` bootstrap 时 `register_rml_contributions(cx)` 被调用，但此时 host Entity 尚未创建（窗口未开）。`registry.register` 检测 host 不存在 → 入队 pending。窗口创建后 `on_loaded` 调 `register_host(cx)` → `registry.add` 重放 pending 到 `host.add`。
2. **跨 Entity 通知**：子 Entity（如 `ActivityPanel`）通过 `cx.observe(&host_entity, ...)` 直接 observe host Entity 变更，替代 `subscribe_host_changes`。host 的 `HostHandle::add`/`remove` 自动调 `ctx.notify()` 触发 observer。
3. **`IContributionHost::add`** **接** **`&mut App`** **而非** **`&mut Context<Self>`**：`HostHandle::add` 在 `entity.update(cx, |host, ctx| ...)` 内调用，`ctx` 是 `Context<T>` 但 trait 方法只暴露 `&mut App` 以保持通用性。host 内部若需 `Context<Self>` 特定 API 可在 `EntityHostHandleBox` 实现层适配（当前 `entity.update` 闭包参数即 `Context<T>`）。
4. **视觉贡献渲染由 host 主导**：host 在 `add` 时调 `rml_app::contribution::extract_visual(&contribution)` 提取 `Option<Arc<dyn IVisualContribution>>`，命中则存储到自有字段（如 `Vec<Arc<dyn IVisualContribution>>`）。渲染入口（如 `active_case_view`）直接调 `visual.render(window, cx)`——无运行时向下转型。框架不提供 `EntityCache`——若 host 需保留视觉贡献的内部状态（如 form/input），由 host 自行 `HashMap<String, Entity<T>>` 管理。

### 设计决策

1. **`HostHandle`** **为内部 trait，`EntityHostHandleBox<T>`** **在 app 层定义**：`core` 层不依赖 gpui `Render` bound，仅定义 trait；app 层依赖 gpui，定义具体包装类型。`#[doc(hidden)]` 隐藏细节。
2. **`IContribution`（能力贡献点）与** **`IVisualContribution`（可视化贡献点）语义独立保留，trait 方法签名零修改**：`IVisualContribution: IContribution` 继承关系不变。能力贡献仅含元数据（菜单项、状态栏项等）；可视化贡献扩展 `render` 方法（ActivityPanel、CaseView 等）。**不向** **`IContribution`** **添加** **`as_visual()`** **桥接方法**——向下转型通过 `Any` supertrait + 宏生成 `VisualExtractor` 函数在进程级静态表中按 `TypeId` 查找实现，host 调 `rml_app::contribution::extract_visual(&contribution)` 获取 `Arc<dyn IVisualContribution>`。
3. **`RenderContext`** **删除**：视觉贡献直接接 `&mut Window, &mut App`——已是 gpui 渲染 API 的标准签名，无需包装类型。
4. **`EntityCache`** **删除**：框架不缓存 Entity。host 自管缓存（若需要）。原设计假设"视觉贡献 Entity 必须缓存以保留状态"过强——host 可按需缓存。
5. **`register_contribution`** **中间函数删除**：宏直接调 `cx.get_contribution_registry().register(...)`，无中间层。
6. **`#[contributehost]`** **不生成注册函数**：host 在 `on_loaded` 调 `register_host(cx)` 注册自身。build.rs 不再扫描 host。
7. **host 存储策略自由**：可使用 `Vec`/`HashMap`/`ObservableVec`/不存储——框架不限定。本计划 demo 中 `MainWindow` 使用普通业务字段（`MenuItems`/`StatusBarItems`/`ActivityPanels`/`Vec<TreeItem>`），不强制 `ObservableVec`。
8. **`IContributionRegistry`** **trait 方法均接** **`&mut App`**：用于 `entity.update(cx, ...)` 调用。`&self` + `RwLock` 内部可变性使 `get_contribution_registry()` 返回 `&dyn IContributionRegistry`。
9. **build.rs 仅扫描** **`#[contribute]`**：移除 `HostRegistrar`/`parse_host_registrars`/`dedup_hosts`。`register_rml_contributions(cx)` 仅调用贡献注册函数。

### 风险

1. **RwLock 性能**：`ContributionRegistry` 使用 `RwLock<HashMap>`，每次 `register` 加读锁。缓解：读多写少；`#[ctor::ctor]` 注册集中在启动期；运行时动态注册少见。
2. **pending 队列内存**：若 host 永不注册，pending 永久持有贡献引用。缓解：调试模式日志告警；生产环境 host 通常在窗口创建时即注册。
3. **视觉贡献 Entity 状态丢失**：若 host 不缓存视觉贡献 Entity，每次 `IVisualContribution::render` 创建新 Entity 会丢失内部状态（如 input focus、scroll position）。缓解：host 按需缓存——demo 中 `ActivityPanel` 自身是 `#[component]` Entity，其内部状态由 GPUI 自动保留；若贡献是动态切换的视觉组件，host 应在 `add` 时缓存 `Entity<T>`。
4. **跨 Entity observe 依赖** **`DemoShellHost`** **全局**：`ActivityPanel` observe MainWindow 需通过 `DemoShellHost` WeakEntity 获取。若 MainWindow 先于 ActivityPanel 销毁，observe 自动失效（WeakEntity upgrade 返回 None）。
5. **`EntityHostHandleBox`** **命名**：避免与 core 层 `HostHandle` trait 冲突。`Box` 后缀表达"包装 Entity 为 HostHandle 实现的盒子"。

***

## 不在本计划范围

以下原计划阶段被排除（独立关注点，本计划不修改）：

* **`ObservableVec<T>`** **核心类型**（原 Phase A）：若业务需要可作为独立工具类型单独立项。host 存储策略不依赖此类型。

* **版本系统集成**（原 Phase B）：现有 `__rml_get_version`/`ComputedCache` 基础设施已存在于 `crates/engine/src/compiler/codegen/observable.rs` 与 `crates/core/src/computed_cache.rs`，本计划不修改。若 host 使用普通字段而非 `ObservableVec`，`#[computed]` 缓存键基于 `self.<field>` 访问模式扫描，不依赖集合版本号。

* **RML** **`each=`** **+** **`key=`** **keyed diffing**（原 Phase D）：RML 模板层独立关注点，与贡献点架构无关。

