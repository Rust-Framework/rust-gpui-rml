# RML ObservableCollection 与贡献点架构重构计划

## 摘要

本计划基于用户六项决策重构原计划，核心转变：**框架不再存储贡献数据**，`IContributionHost` 成为主动的受理方（自带 `add`/`remove` 受理代码），`IContributionRegistry` 仅作桥接器将 `register` 调用路由到对应 host。由此消除 `ContributedEntry`、`ComponentEntityCache`、`VisualRenderer` 等框架侧存储类型，并否决 `#[computed_with_cx]` 新宏——业务代码不再调用 `contribution_entries`，数据直接存储在 host 自身的 `ObservableVec` 字段中，`#[computed]` 通过版本号自动感知变更。

### 迭代目标

| 迭代      | 目标                | 交付物                                                    |
| ------- | ----------------- | ------------------------------------------------------ |
| Phase A | 集合响应式核心类型         | `ObservableVec<T>`                                     |
| Phase B | 版本系统集成            | `#[computed]`/`#[command]` 自动感知 ObservableVec          |
| Phase C | 贡献点架构重构（核心）       | `IContributionHost`（受理方）+ `IContributionRegistry`（桥接器） |
| Phase D | RML keyed diffing | `each=` + `key=` 元素复用                                  |
| Phase E | Demo 样板消除         | 受理代码替代 `refresh_shell_chrome`/`map_shell_chrome`       |

***

## 用户决策

| # | 决策                                                                | 影响                                    |
| - | ----------------------------------------------------------------- | ------------------------------------- |
| 1 | Phase C 否决，不增加新宏，`contribution_entries` 不出现在业务代码                  | 移除 `#[computed_with_cx]`；数据存储下沉到 host |
| 2 | `IContributionHost` 含 `id`/`add`/`remove`，业务编写受理代码                | host 主动处置贡献，框架不代劳                     |
| 3 | `ContributedEntry` 无必要性                                           | 框架不存储贡献条目，host 自管存储                   |
| 4 | `ComponentEntityCache` 无必要性                                       | 框架不缓存组件 Entity，host 自管缓存              |
| 5 | `IContributionRegistry` 定义 `add`/`remove`/`register`/`unregister` | 框架实现桥接 contribute → host              |
| 6 | 扩展 App/Context 提供 `get_contribution_registry()`                   | 宏生成代码通过接口操作                           |

***

## 当前状态分析

### 框架侧（将被重构）

* **`crates/core/src/contribution.rs`**：`IContributionHost` 仅有 `const ID`（无 add/remove）；存在 `ContributedEntry`、`ComponentEntityCache` trait、`VisualRenderer` 类型

* **`crates/core/src/contribution_cache.rs`**：`ComponentEntityCacheImpl` 框架侧 Entity 缓存实现

* **`crates/app/src/contribution/host.rs`**：`ContributionHost` 框架存储 `Vec<ContributedEntry>` + `revision: AtomicU64`

* **`crates/app/src/contribution/registry.rs`**：`ContributionRegistry` 框架集中存储 hosts + entity\_cache + listeners

* **`crates/app/src/contribution/entry.rs`**：`data_entry`/`component_entry` 构建 `ContributedEntry`

* **`crates/app/src/contribution/render.rs`**：`render_component_view` 通过全局 registry entity\_cache 渲染

* **`crates/app/src/contribution/global.rs`**：`ContributionExt` 扩展 App，`contribution_entries`/`contribution_revision`/`subscribe_host_changes` 读框架存储

### 业务侧（将被简化）

* **`demo/src/shell/main_window.rml.rs`**：`refresh_shell_chrome` 手动桥接 + `subscribe_host_changes` 回调

* **`demo/src/shell/shell_chrome.rs`**：`map_shell_chrome`/`map_menu_items`/`map_status_items` 从 `contribution_entries` 投影

* **`demo/src/shell/activity_panel.rml.rs`**：`ActivityPanel` 通过 `subscribe_host_changes` + `map_case_tree_items` 刷新树

### 宏侧（将调整）

* **`crates/macros/src/contribute.rs`**：生成 `Registerable` impl + `register_contribution` 调用

* **`crates/macros/src/contributehost.rs`**：生成 `ID` const + `cx.add(ID)` 注册

* **`crates/engine/src/build/contribution_generator.rs`**：build.rs 扫描生成 `register_rml_contributions`

***

## 新架构概览

### 数据流转变

```
旧：contribute宏 → register_contribution → 框架存储 ContributedEntry
    → contribution_entries 读取 → map_shell_chrome 投影 → refresh_shell_chrome 更新字段
    → #[computed_with_cx] 缓存 → UI 更新

新：contribute宏 → registry.register → 路由到 host.add（受理代码）
    → host.push 到 ObservableVec → version bump
    → #[computed] 缓存失效 → UI 更新（零样板）
```

### 类型关系

```
IContribution（元数据 + render_view）
  └─ IVisualContribution（render 方法）

IContributionHost（id + add + remove —— 受理方）
  ↑ EntityHostHandle<T> 包装 WeakEntity<T>，实现 HostHandle trait

IContributionRegistry（add + remove + register + unregister —— 桥接器）
  └─ ContributionRegistry（框架实现，RwLock 内部可变性）

EntityCache（host 拥有的工具结构，非 trait）
  └─ RenderContext.entity_cache 字段
```

### 注册时序

```
1. #[ctor::ctor] → register_contribution(cx, host_id, contribution, options)
   → registry.register → host 不存在 → 存入 pending 队列

2. 窗口创建 → MainWindow::on_loaded → register_host(cx)
   → registry.add(EntityHostHandle { weak }) → 重放 pending 队列
   → 逐条调用 host.add(contribution, options, cx) → host 存储到 ObservableVec
   → cx.notify() → 重渲
```

***

## Phase A：`ObservableVec<T>` 核心类型

**不变，详见原计划。**

### 新建文件

**`crates/core/src/observable.rs`** —— 版本号驱动的可观察集合

```rust
pub struct ObservableVec<T> {
    inner: Vec<T>,
    version: AtomicU64,
}
```

* 无 `DerefMut`：强制通过 mutation 方法修改，确保 version bump

* `AtomicU64` version：lock-free 读取

* mutation 方法（push/insert/remove/swap/clear/replace\_range/retain/sort\_by\_mut）自动 bump

* 只读方法（iter/get/as\_slice/len/is\_empty/version）

**`sort_by_mut`**：为 `ContributionHost::add` 的 dedup+sort 需求提供有意 mutation 入口。

**导出**：`crates/core/src/lib.rs` 添加 `pub mod observable;` + `pub use observable::ObservableVec;`，`prelude.rs` 导出。

***

## Phase B：版本系统 + `#[computed]` 集成

**不变，详见原计划。**

### 修改要点

* **`crates/engine/src/build/scanner.rs`**：检测 `ObservableVec<...>` 字段类型，记入 `observable_vec_fields`

* **`crates/engine/src/compiler/codegen/observable.rs`**：`get_arms` 对 ObservableVec 字段路由到 `self.field.version()`；`bump_arms` 跳过（no-op）

* **`crates/macros/src/component.rs`**：跳过 ObservableVec 字段的 `__rml_<field>_version` 注入（ObservableVec 内部已有 version）

### 效果

`#[computed]` 方法依赖 `ObservableVec` 字段时，缓存键自动包含集合版本。`#[command]` 中的 `__rml_bump_version` 对 ObservableVec 字段为 no-op。**无需** **`#[computed_with_cx]`**——computed 方法通过 `&self` 读取 host 自身字段即可。

***

## Phase C：贡献点架构重构（核心）

### C1：`IContributionHost` trait 重设计

**文件**：`crates/core/src/contribution.rs`

```rust
/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
pub trait IContributionHost: Send + Sync + 'static {
    const ID: &'static str;

    /// 受理代码：接收并处置贡献。host 自行决定存储方式
    /// （如 push 到 ObservableVec<MenuItem>、ObservableVec<StatusBarItem> 等）。
    fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);

    /// 移除贡献。host 自行决定清理方式。
    fn remove(&mut self, contribution_id: &str, cx: &mut App);
}
```

**设计要点**：

* `const ID`：编译期已知，宏生成代码使用（`#[contributehost]` 生成 `Self::ID`）

* `add`/`remove`：业务实现的受理代码，接收 `Arc<dyn IContribution>` + `ContributionOptions`，按 slot/group 等分发到 host 自有数据结构

* `&mut App`（非 `&mut Context<Self>`）：host 通过 `ObservableVec::push` 的 version bump 驱动响应式，`HostHandle` 在 `entity.update` 后自动调用 `cx.notify()`

### C2：`IContribution` trait 扩展

**文件**：`crates/core/src/contribution.rs`

```rust
pub trait IContribution: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> SharedString;
    fn description(&self) -> SharedString;
    fn icon(&self) -> Option<SharedString>;

    /// 视觉渲染（视觉贡献覆盖此方法；非视觉贡献返回 None）。
    /// 消除 VisualRenderer 闭包类型——host 直接调用此方法。
    fn render_view(&self, ctx: &mut RenderContext<'_>) -> Option<gpui::AnyElement> { None }
}

pub trait IVisualContribution: IContribution {
    fn render(&self, ctx: &mut RenderContext<'_>) -> gpui::AnyElement;
}
```

**`#[contribute] + #[component]`** **宏生成覆盖**：

```rust
impl IContribution for MyCase {
    // ... 元数据方法 ...
    fn render_view(&self, ctx: &mut RenderContext) -> Option<gpui::AnyElement> {
        Some(self.render(ctx))  // 委托给 IVisualContribution::render
    }
}
```

### C3：`RenderContext` + `EntityCache` 重设计

**文件**：`crates/core/src/contribution.rs` + `crates/core/src/contribution_cache.rs`（重命名）

```rust
/// 渲染上下文（host 创建，包含 host 拥有的 EntityCache）
pub struct RenderContext<'a> {
    pub window: &'a mut gpui::Window,
    pub cx: &'a mut gpui::App,
    pub active: bool,
    pub entity_cache: &'a mut EntityCache,  // 新增：host 拥有
}

/// 组件 Entity 缓存（工具结构，非 trait；host 作为字段持有）
/// 替代原 ComponentEntityCache trait + ComponentEntityCacheImpl
pub struct EntityCache {
    entries: HashMap<String, (TypeId, Box<dyn Any + Send + Sync>)>,
}

impl EntityCache {
    pub fn new() -> Self { ... }

    /// 查找或创建 Entity<V>，缓存按 contribution_id
    pub fn render_view<V: Render + Send + Sync + 'static>(
        &mut self, contribution_id: &str, view: V, ctx: &mut RenderContext,
    ) -> AnyElement { ... }

    pub fn pre_register<T: Render + Send + Sync + 'static>(
        &mut self, contribution_id: &str, entity: Entity<T>,
    );

    pub fn clear(&mut self, contribution_id: &str);
    pub fn clear_all(&mut self);
}
```

**设计要点**：

* `EntityCache` 是普通结构体（非 trait），host 直接作为字段持有

* `RenderContext` 包含 `&mut EntityCache`，视觉贡献通过 `ctx.entity_cache` 复用 Entity

* **框架不存储** EntityCache——它由 host 创建并拥有

### C4：`IContributionRegistry` trait 定义

**文件**：`crates/core/src/contribution.rs`

```rust
/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，负责按 host_id 路由 register 调用到对应 host 的 add 方法。
pub trait IContributionRegistry: Send + Sync {
    /// 注册 host（host 在 on_loaded 时调用）
    fn add(&self, host: Box<dyn HostHandle>);

    /// 注销 host
    fn remove(&self, host_id: &str);

    /// 向 host 注册贡献（#[contribute] 宏生成代码调用）
    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);

    /// 从 host 注销贡献
    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
```

**`HostHandle`** **trait（内部，`#[doc(hidden)]`）**：

```rust
/// 类型擦除的 host 句柄，包装 WeakEntity<T>
#[doc(hidden)]
pub trait HostHandle: Send + Sync {
    fn id(&self) -> &str;
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
    fn remove(&self, contribution_id: &str, cx: &mut App);
}

/// Entity<T> 的 HostHandle 实现
#[doc(hidden)]
pub struct EntityHostHandle<T: IContributionHost> {
    weak: WeakEntity<T>,
}

impl<T: IContributionHost + Render + 'static> HostHandle for EntityHostHandle<T> {
    fn id(&self) -> &str { T::ID }

    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
        if let Some(entity) = self.weak.upgrade() {
            entity.update(cx, |host, ctx| {
                host.add(contribution, options, ctx);  // 受理代码
                ctx.notify();  // 自动触发重渲
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

/// 构造函数（宏 / register_host 调用）
#[doc(hidden)]
pub fn entity_host_handle<T: IContributionHost + Render + 'static>(weak: WeakEntity<T>) -> Box<dyn HostHandle> {
    Box::new(EntityHostHandle { weak })
}
```

### C5：`ContributionRegistry` 框架实现

**文件**：`crates/app/src/contribution/registry.rs`（重写）

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::App;
use rml_core::contribution::{
    HostHandle, IContribution, IContributionRegistry, ContributionOptions,
};

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

    /// 读取 host 条目（host 自管存储，registry 不提供 entries 读取）
    /// 保留 revision 供调试/监控
    pub fn has_host(&self, host_id: &str) -> bool {
        self.hosts.read().unwrap().contains_key(host_id)
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add(&self, host: Box<dyn HostHandle>) {
        let id = host.id().to_string();
        let mut hosts = self.hosts.write().unwrap();
        hosts.insert(id.clone(), host);
        drop(hosts);

        // 重放 pending 队列
        let mut pending = self.pending.write().unwrap();
        if let Some(queue) = pending.remove(&id) {
            drop(pending);
            // 注意：重放需要 cx，但 IContributionRegistry::add 不接受 cx
            // 解决方案：见下方 C6 register_host 设计——重放在 register_host 中完成
            // 或：pending 存储 (contribution, options)，重放在 add 中无法完成
            // 修正：add 接受 cx 参数
        }
    }

    // ...
}
```

**修正：`add`** **需要** **`cx`** **参数以重放 pending**。调整 trait 签名：

```rust
pub trait IContributionRegistry: Send + Sync {
    fn add(&self, host: Box<dyn HostHandle>, cx: &mut App);
    fn remove(&self, host_id: &str, cx: &mut App);
    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
```

**`add`** **实现（重放 pending）**：

```rust
fn add(&self, host: Box<dyn HostHandle>, cx: &mut App) {
    let id = host.id().to_string();
    let mut hosts = self.hosts.write().unwrap();
    hosts.insert(id.clone(), host);
    drop(hosts);

    // 重放 pending 注册
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
```

**`register`** **实现（路由到 host 或入队 pending）**：

```rust
fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
    let hosts = self.hosts.read().unwrap();
    if let Some(host) = hosts.get(host_id) {
        host.add(contribution, options, cx);
    } else {
        drop(hosts);
        let mut pending = self.pending.write().unwrap();
        pending
            .entry(host_id.to_string())
            .or_default()
            .push((contribution, options));
    }
}
```

**`unregister`** **实现**：

```rust
fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool {
    let hosts = self.hosts.read().unwrap();
    if let Some(host) = hosts.get(host_id) {
        host.remove(contribution_id, cx);
        true
    } else {
        false
    }
}
```

### C6：App/Context 扩展

**文件**：`crates/app/src/contribution/global.rs`（重写）

```rust
use std::sync::{Arc, Mutex};

use gpui::{App, Global};
use rml_core::contribution::{
    EntityCache, HostHandle, IContribution, IContributionHost, IContributionRegistry,
    ContributionOptions,
};

use super::registry::ContributionRegistry;

/// GPUI 全局贡献注册表
#[doc(hidden)]
pub struct ContributionRegistryGlobal(pub ContributionRegistry);

impl Global for ContributionRegistryGlobal {}

/// 确保全局注册表已初始化
pub fn ensure_contribution_registry(cx: &mut App) {
    if !cx.has_global::<ContributionRegistryGlobal>() {
        cx.set_global(ContributionRegistryGlobal(ContributionRegistry::new()));
    }
}

/// App 扩展：贡献注册表访问
pub trait ContributionRegistryExt {
    /// 获取 IContributionRegistry 接口进行操作。
    /// 返回 &dyn（不可变引用），方法通过 RwLock 内部可变性操作。
    fn get_contribution_registry(&mut self) -> &dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&mut self) -> &dyn IContributionRegistry {
        ensure_contribution_registry(self);
        &self.global::<ContributionRegistryGlobal>().0
    }
}

/// host 注册自身（在 on_loaded 中调用）
pub fn register_host<T: IContributionHost + gpui::Render + 'static>(cx: &mut gpui::Context<T>) {
    let weak = cx.weak_entity();
    cx.get_contribution_registry().add(rml_core::contribution::entity_host_handle(weak), cx);
}

/// 注销 host
pub fn unregister_host(cx: &mut App, host_id: &str) {
    if cx.has_global::<ContributionRegistryGlobal>() {
        cx.get_contribution_registry().remove(host_id, cx);
    }
}

/// 统一贡献注册入口（#[contribute] 宏生成代码调用）
pub fn register_contribution(
    cx: &mut App,
    host_id: &str,
    contribution: Arc<dyn IContribution>,
    options: ContributionOptions,
) {
    ensure_contribution_registry(cx);
    cx.get_contribution_registry().register(host_id, contribution, options, cx);
}

/// 注销贡献
pub fn unregister_contribution(cx: &mut App, host_id: &str, contribution_id: &str) -> bool {
    if !cx.has_global::<ContributionRegistryGlobal>() {
        return false;
    }
    cx.get_contribution_registry().unregister(host_id, contribution_id, cx)
}
```

### C7：`#[contributehost]` 宏调整

**文件**：`crates/macros/src/contributehost.rs`

移除 `__rml_register_*` 函数（不再需要 bootstrap 时预注册 host slot）。宏只生成：

```rust
quote! {
    #(#items)*

    impl #struct_name {
        pub const ID: &'static str = #id;
    }

    // 编译期断言：必须实现 IContributionHost（含 add/remove 受理代码）
    const _: () = {
        fn assert_contribution_host<T: rml_core::contribution::IContributionHost>() {}
        fn check() { assert_contribution_host::<#struct_name>(); }
    };
}
```

**build.rs 调整**：`crates/engine/src/build/contribution_generator.rs` 移除 host 注册扫描（`HostRegistrar` / `parse_host_registrars`），只保留 `#[contribute]` 扫描。`register_rml_contributions` 只调用贡献注册函数。

### C8：`#[contribute]` 宏调整

**文件**：`crates/macros/src/contribute.rs`

移除 `Registerable` impl + `component_registerable`/`data_registerable` 引用。改为：

```rust
// 视觉贡献：IVisualContribution + IContribution::render_view 覆盖
let visual_impl = if use_component_visual {
    quote! {
        impl rml_core::contribution::IVisualContribution for #struct_name {
            fn render(&self, ctx: &mut rml_core::contribution::RenderContext) -> gpui::AnyElement {
                rml_app::contribution::render_component_view::<Self>(self, ctx)
            }
        }
    }
} else { quote! {} };

// 注册函数：使用 register_contribution
quote! {
    #(#items)*

    impl rml_core::contribution::IContribution for #struct_name {
        fn id(&self) -> &str { #id }
        fn name(&self) -> gpui::SharedString { rml_core::i18n::t_static(#name_key).into() }
        fn description(&self) -> gpui::SharedString { #description_impl }
        fn icon(&self) -> Option<gpui::SharedString> { #icon_impl }

        // 视觉贡献覆盖 render_view
        fn render_view(&self, ctx: &mut rml_core::contribution::RenderContext) -> Option<gpui::AnyElement> {
            #render_view_body  // Some(self.render(ctx)) 或 None
        }
    }

    #visual_impl

    pub fn #register_fn(cx: &mut gpui::App) {
        use std::sync::Arc;
        use rml_app::contribution::register_contribution;

        let contribution = Arc::new(#struct_name::default());
        let options = rml_core::contribution::ContributionOptions::new()
            #slot #parent_id #order #group #align;

        register_contribution(cx, #host_id, contribution, options);
    }
}
```

### C9：视觉贡献渲染重构

**文件**：`crates/app/src/contribution/render.rs`（重写）

```rust
use gpui::{AnyElement, Render};
use rml_core::component::IComponent;
use rml_core::contribution::{EntityCache, IVisualContribution, RenderContext};

/// 框架工具：渲染组件贡献视图（由 IVisualContribution::render 调用）。
/// 使用 RenderContext.entity_cache 查找或创建 Entity，host 拥有缓存。
pub fn render_component_view<T>(contribution: &T, ctx: &mut RenderContext) -> AnyElement
where
    T: IComponent + Render + Default + Send + Sync + 'static,
{
    let id = contribution.id().to_string();
    ctx.entity_cache.render_view(&id, T::default(), ctx)
}
```

### C10：移除的文件/类型

| 文件                                              | 操作                                       | 原因                                                                         |
| ----------------------------------------------- | ---------------------------------------- | -------------------------------------------------------------------------- |
| `crates/core/src/contribution_cache.rs`         | 重写为 `entity_cache.rs`（`EntityCache` 结构体） | `ComponentEntityCache` trait + `ComponentEntityCacheImpl` → 简化为工具结构        |
| `crates/app/src/contribution/host.rs`           | **删除**                                   | 框架不再存储 `ContributionHost`（host 自管存储）                                       |
| `crates/app/src/contribution/entry.rs`          | **删除**                                   | `ContributedEntry` 类型移除，`data_entry`/`component_entry` 不再需要                |
| `crates/app/src/contribution/registerable.rs`   | **删除**                                   | `Registerable` trait 移除                                                    |
| `crates/app/src/contribution/activity_panel.rs` | **移至 demo**                              | `map_activity_panels` 是业务投影代码                                              |
| `VisualRenderer` 类型                             | **删除**                                   | `IContribution::render_view` 替代                                            |
| `ContributedEntry` 类型                           | **删除**                                   | host 自管存储格式                                                                |
| `ComponentEntityCache` trait                    | **删除**                                   | `EntityCache` 工具结构替代                                                       |
| `contribution_entries` 函数                       | **删除**                                   | 业务代码不读框架存储                                                                 |
| `contribution_revision` 函数                      | **删除**                                   | ObservableVec::version() 替代                                                |
| `subscribe_host_changes` 函数                     | **删除**                                   | host ObservableVec version bump + `#[computed]` 替代；跨 Entity 用 `cx.observe` |

### C11：`crates/app/src/contribution/mod.rs` 更新

```rust
mod entity_cache;  // 原 contribution_cache
mod global;
mod registry;
mod render;

pub use global::{
    register_contribution, register_host, unregister_contribution, unregister_host,
    ContributionRegistryExt,
};
#[doc(hidden)]
pub use global::ContributionRegistryGlobal;
#[doc(hidden)]
pub use registry::ContributionRegistry;
#[doc(hidden)]
pub use render::render_component_view;
```

***

## Phase D：RML `each=` + `key=` keyed diffing

**不变，详见原计划。**

### 修改文件

* **`crates/core/src/observable.rs`**：`reconcile` 辅助函数（keyed reconciliation）

* **`crates/engine/src/compiler/codegen/mod.rs`**：`gen_node` each= 分支，ObservableVec + key= 时生成 keyed diffing

* **`crates/macros/src/component.rs`**：注入 `__rml_{field}_children: Vec<(K, AnyElement)>` 字段

* **`crates/engine/src/parser/ast.rs`**：`Directive::Key` 已解析，codegen 消费

### 性能特性

* Element 复用：相同 key 跨 render 复用，保留内部状态

* 增量构建：仅新 key 触发 builder

* O(n) reconcile

***

## Phase E：Demo 样板代码消除

### E1：`MainWindow` 受理代码

**文件**：`demo/src/shell/main_window.rml.rs`

```rust
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

    // —— ObservableVec：host 自管存储 ——
    menu_entries: ObservableVec<MenuEntry>,           // menu slot 贡献
    status_entries: ObservableVec<StatusEntry>,       // status slot 贡献
    activity_entries: ObservableVec<ActivityEntry>,   // activity slot 贡献（含 visual）
    case_entries: ObservableVec<CaseEntry>,           // case slot 贡献

    // —— host 拥有的 Entity 缓存 ——
    entity_cache: EntityCache,

    // —— derived（#[computed] 从 ObservableVec 计算）——
    slot_left_size: gpui::Pixels,
}

/// host 内部存储格式（host 自定义，非框架类型）
struct MenuEntry {
    id: String,
    name: SharedString,
    order: i32,
    parent_id: Option<String>,
}
// 类似 StatusEntry, ActivityEntry, CaseEntry ...

impl IContributionHost for MainWindow {
    const ID: &'static str = "demo.shell";

    fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
        match options.effective_slot() {
            Some("menu") => {
                self.menu_entries.push(MenuEntry::from(&contribution, &options));
            }
            Some("status") => {
                self.status_entries.push(StatusEntry::from(&contribution, &options));
            }
            Some("activity") => {
                self.activity_entries.push(ActivityEntry::from(&contribution, &options));
            }
            Some("case") => {
                self.case_entries.push(CaseEntry::from(&contribution, &options));
            }
            _ => {}
        }
        // ObservableVec::push 已 bump version → #[computed] 自动失效
        // HostHandle 自动调用 cx.notify()，无需手动触发
    }

    fn remove(&mut self, contribution_id: &str, _cx: &mut App) {
        self.menu_entries.retain(|e| e.id != contribution_id);
        self.status_entries.retain(|e| e.id != contribution_id);
        self.activity_entries.retain(|e| e.id != contribution_id);
        self.case_entries.retain(|e| e.id != contribution_id);
        self.entity_cache.clear(contribution_id);
    }
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // ... 初始化 open_tabs, menu_commands ...

        // 注册自身为贡献 host → registry 重放 pending 注册 → add 逐条调用
        register_host(cx);

        // ActivityBar 构造（从 activity_entries computed）
        let panels = self.activity_panels(cx);
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

        // observe ActivityBar active_id 变化
        if let Some(bar) = &self.activity_bar {
            cx.observe(bar, |this, bar, cx| {
                let collapsed = bar.read(cx).active_id().is_none();
                this.slot_left_size = if collapsed { gpui::px(48.) } else { gpui::px(260.) };
                cx.notify();
            }).detach();
        }

        // 无需 subscribe_host_changes —— host.add 直接修改 ObservableVec
        // 无需 refresh_shell_chrome —— #[computed] 自动从 ObservableVec 计算
    }
}

impl MainWindow {
    #[computed]
    pub fn menu_items(&self) -> MenuItems {
        // 缓存键 = self.menu_entries.version()
        build_menu_tree(&self.menu_entries, &self.menu_commands)
    }

    #[computed]
    pub fn status_items(&self) -> StatusBarItems {
        // 缓存键 = self.status_entries.version()
        build_status_items(&self.status_entries)
    }

    #[computed]
    pub fn activity_panels(&self) -> ActivityPanels {
        // 缓存键 = self.activity_entries.version()
        build_activity_panels(&self.activity_entries)
    }

    #[computed]
    pub fn case_tree_items(&self) -> Vec<TreeItem> {
        // 缓存键 = self.case_entries.version()
        build_case_tree(&self.case_entries)
    }

    /// 渲染当前激活的视觉贡献（供 RML 模板调用）
    pub fn active_case_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let mut ctx = RenderContext {
            window,
            cx,
            active: true,
            entity_cache: &mut self.entity_cache,
        };
        // 在 activity_entries 中查找激活的视觉贡献
        if let Some(entry) = self.activity_entries.iter().find(|e| e.id == self.active_case_id) {
            if let Some(element) = entry.contribution.render_view(&mut ctx) {
                return element;
            }
        }
        gpui::div().into_any_element()
    }
}
```

### E2：`ActivityPanel` 重构

**文件**：`demo/src/shell/activity_panel.rml.rs`

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
                this.refresh_tree_from_host(&main, cx);
                cx.notify();
            }).detach();
        }

        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_tree_from_global(cx);
            cx.notify();
        }).detach();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
            let items = host.read(cx).case_tree_items();  // MainWindow 的 #[computed]
            self.set_tree_items(items, cx);
        }
    }

    fn refresh_tree_from_host(&mut self, main: &MainWindow, cx: &mut Context<Self>) {
        let items = main.case_tree_items();
        self.set_tree_items(items, cx);
    }

    fn set_tree_items(&mut self, items: Vec<TreeItem>, cx: &mut Context<Self>) {
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| s.set_items(items, cx));
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }
}
```

**关键变化**：

* `subscribe_host_changes` → `cx.observe(&host_entity, ...)` 直接 observe MainWindow Entity

* `map_case_tree_items(MainWindow::ID, cx)` → `host.read(cx).case_tree_items()` 读 host 自身 computed

* 无 `contribution_entries` 调用

### E3：`shell_chrome.rs` 调整

**文件**：`demo/src/shell/shell_chrome.rs`

* `map_shell_chrome` / `ShellChromeBindings` **删除**（不再需要投影层）

* `map_menu_items` / `map_status_items` / `map_case_tree_items` 逻辑移入 `MainWindow` 的 `#[computed]` 方法（`build_menu_tree` / `build_status_items` / `build_case_tree`）

* `map_activity_panels` 移入 `demo/src/shell/`（业务投影代码，从 `rml_app` 移出）

### E4：`#[contribute]` 声明保留

`demo/src/shell/menu_shell_contribs.rs` 的 `#[contribute]` 声明不变——贡献点声明本身是数据驱动的，宏自动注册。变化在于注册路由：`register_contribution` → `registry.register` → `host.add`（受理代码分发到对应 ObservableVec）。

***

## 验证步骤

### Phase A 验证

```bash
cargo test -p rust-rml-core -- observable
```

ObservableVec mutation 后 version 递增、`Deref<[T]>` 读取、无 `DerefMut`。

### Phase B 验证

```bash
cargo build -p rust-rml-engine -p rust-rml-macros
cargo test -p rust-rml-engine -- codegen::observable
```

`__rml_get_version` 对 ObservableVec 字段路由到 `self.field.version()`。

### Phase C 验证

```bash
cargo build -p rust-rml-core -p rust-rml-app -p rust-rml-macros
cargo test -p rust-rml-app -- contribution
```

验证：

1. `IContributionHost::add`/`remove` 受理代码正确分发
2. `IContributionRegistry::register` 路由到 host.add（host 存在时）
3. pending 队列：host 未注册时入队，`add` 后重放
4. `get_contribution_registry()` 返回接口可操作
5. `ContributedEntry`/`ComponentEntityCache`/`VisualRenderer` 已删除，编译无引用

### Phase D 验证

```bash
cargo test -p rust-rml-engine -- codegen::each_key
cargo run -p rust-rml-demo
```

`each=` + `key=` 生成 keyed diffing 代码，element 复用。

### Phase E 验证

```bash
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

验证：

1. Demo 启动后 menu/status/activity 面板正确显示
2. 通过菜单打开 case → tab 新增 → UI 更新（无 `refresh_shell_chrome` 调用）
3. ActivityPanel 案例树正确显示，observe MainWindow 自动刷新
4. 切换语言 → 菜单标题更新
5. **无** **`contribution_entries`** **调用出现在 demo 代码中**

***

## 关键文件清单

| 文件                                                  | Phase | 操作                                                                                                                               |
| --------------------------------------------------- | ----- | -------------------------------------------------------------------------------------------------------------------------------- |
| `crates/core/src/observable.rs`                     | A     | 新建                                                                                                                               |
| `crates/core/src/lib.rs`                            | A, C  | 导出 observable + 调整 contribution 模块                                                                                               |
| `crates/core/src/prelude.rs`                        | A, C  | 导出 ObservableVec，移除旧类型导出                                                                                                         |
| `crates/core/src/contribution.rs`                   | C     | 重写：IContributionHost add/remove + IContributionRegistry + HostHandle + IContribution::render\_view + EntityCache + RenderContext |
| `crates/core/src/contribution_cache.rs`             | C     | 重写为 `entity_cache.rs`（EntityCache 结构体）                                                                                           |
| `crates/engine/src/build/scanner.rs`                | B     | 检测 ObservableVec 字段                                                                                                              |
| `crates/engine/src/compiler/codegen/observable.rs`  | B     | 版本路由                                                                                                                             |
| `crates/macros/src/component.rs`                    | B, D  | 跳过 ObservableVec version 注入 + children 字段注入                                                                                      |
| `crates/macros/src/contributehost.rs`               | C     | 移除 \__rml\_register_\* 函数，仅生成 ID + 断言                                                                                            |
| `crates/macros/src/contribute.rs`                   | C     | 移除 Registerable，生成 render\_view 覆盖，调用 register\_contribution                                                                     |
| `crates/engine/src/build/contribution_generator.rs` | C     | 移除 host 扫描，只保留 contribute 扫描                                                                                                     |
| `crates/app/src/contribution/mod.rs`                | C     | 更新模块声明与导出                                                                                                                        |
| `crates/app/src/contribution/global.rs`             | C     | 重写：ContributionRegistryExt + register\_host + register\_contribution                                                             |
| `crates/app/src/contribution/registry.rs`           | C     | 重写：ContributionRegistry 实现 IContributionRegistry                                                                                 |
| `crates/app/src/contribution/render.rs`             | C     | 重写：render\_component\_view 使用 ctx.entity\_cache                                                                                  |
| `crates/app/src/contribution/host.rs`               | C     | **删除**                                                                                                                           |
| `crates/app/src/contribution/entry.rs`              | C     | **删除**                                                                                                                           |
| `crates/app/src/contribution/registerable.rs`       | C     | **删除**                                                                                                                           |
| `crates/app/src/contribution/activity_panel.rs`     | C     | **移至 demo**                                                                                                                      |
| `crates/engine/src/compiler/codegen/mod.rs`         | D     | each= + key= keyed diffing                                                                                                       |
| `demo/src/shell/main_window.rml.rs`                 | E     | 受理代码 + ObservableVec + #\[computed] + register\_host                                                                             |
| `demo/src/shell/shell_chrome.rs`                    | E     | 删除 map\_shell\_chrome，投影逻辑移入 MainWindow                                                                                          |
| `demo/src/shell/activity_panel.rml.rs`              | E     | observe MainWindow 替代 subscribe\_host\_changes                                                                                   |

***

## 假设与决策

### 假设

1. **host EntityCache**：host 将 `EntityCache` 作为字段持有，在 `RenderContext` 中传递给视觉贡献渲染。框架提供 `EntityCache` 工具结构但不存储它。
2. **pending 队列重放**：贡献在 host 注册前通过 `#[ctor::ctor]` 注册，registry 入队 pending。host 在 `on_loaded` 调用 `register_host(cx)` 后重放。
3. **跨 Entity 通知**：子 Entity（如 `ActivityPanel`）通过 `cx.observe(&host_entity, ...)` 观察 host 变更，替代 `subscribe_host_changes`。
4. **ObservableVec sort**：`sort_by_mut` 作为有意 mutation 入口，bump version。用于 host 内部排序需求。
5. **IContributionRegistry 方法签名**：`add`/`remove`/`register`/`unregister` 均接受 `&mut App` 参数（用于 `entity.update` 调用），trait 方法使用 `&self` + `RwLock` 内部可变性使 `get_contribution_registry()` 可返回 `&dyn IContributionRegistry`。

### 设计决策

1. **`const ID`** **而非** **`fn id()`**：编译期常量，宏生成代码可直接引用 `Self::ID`，无需 trait 对象。`HostHandle::id()` 方法返回 `T::ID` 供 registry 运行时查询。
2. **`HostHandle`** **为内部 trait**：`#[doc(hidden)]`，用户不直接接触。通过 `entity_host_handle(weak)` 构造，`register_host` 封装调用。
3. **`EntityCache`** **为结构体而非 trait**：简化设计，host 直接持有，无需实现 trait。原 `ComponentEntityCache` trait 的"可替换实现"需求在实际中不存在。
4. **`IContribution::render_view`** **默认 None**：非视觉贡献返回 None，视觉贡献由宏覆盖为 `Some(self.render(ctx))`。消除 `VisualRenderer` 闭包类型。
5. **`register_host`** **需要** **`Render`** **bound**：`EntityHostHandle<T>` 调用 `entity.update(cx, |host, ctx| ...)` 需要 `T: Render`（GPUI `Context<T>` 约束）。host 本身是可渲染 Entity，自然满足。
6. **build.rs 移除 host 扫描**：host 不再需要 bootstrap 时预注册 slot（无框架侧存储）。只有 `#[contribute]` 需要扫描生成注册函数。

### 风险

1. **RwLock 性能**：`ContributionRegistry` 使用 `RwLock<HashMap>`，每次 `register` 加读锁。缓解：读多写少场景，RwLock 读锁并发；`#[ctor::ctor]` 注册集中在启动期。
2. **pending 队列内存**：若 host 永不注册，pending 队列永久持有贡献引用。缓解：调试模式下日志告警；生产环境 host 通常在窗口创建时即注册。
3. **EntityCache 生命周期**：host 拥有 EntityCache，视觉贡献 Entity 生命周期绑定 host。host 销毁时 EntityCache 自动释放。`remove` 时调用 `entity_cache.clear(id)` 清理单个贡献。
4. **跨 Entity observe**：`ActivityPanel` observe `MainWindow` 需要通过 `DemoShellHost` 全局获取 WeakEntity。若 MainWindow 先于 ActivityPanel 销毁，observe 自动失效（WeakEntity upgrade 返回 None）。

