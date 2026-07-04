# RML App/Context 扩展统一为 IAppContext（IServiceProvider 风格）实施计划

## 摘要

当前 RML 框架对 `gpui::App` / `gpui::Context<T>` 的扩展**分散在 4 个文件、2 个 crate 中**，采用**双轨制存储**（`OnceLock` 静态 + GPUI `Global`），且**无中央聚合点**。业务代码为弥补框架缺失，自创 `DemoShellHost` 作为 `Global` 服务定位器，在 3 处文件重复 `try_global + upgrade + update` 样板。本计划引入 `IAppContext` trait（IServiceProvider 动态查询风格），统一全局状态访问与操作能力，同时移除违反项目记忆约束的过度设计（`entity_cache`、`host_handle` channel 桥接、runtime stubs）。

---

## 一、当前状态分析（回答用户的 4 个问题）

### 问题 1：框架内是否分散编写？

**结论：高度分散，无中央聚合点。**

通过 grep 全量扫描，App/Context 扩展共 6 处 `impl`，分布在 4 个文件、2 个 crate：

| # | 扩展 trait | 实现目标 | 文件路径 | 行号 | 所属 crate |
|---|---|---|---|---|---|
| 1 | `I18nExt` | `App` | `crates/core/src/i18n.rs` | 173 | rml_core |
| 2 | `I18nExt` | `Context<'_, T>` | `crates/core/src/i18n.rs` | 230 | rml_core |
| 3 | `ThemeExt` | `App` | `crates/core/src/theme.rs` | 171 | rml_core |
| 4 | `ThemeExt` | `Context<'_, T>` | `crates/core/src/theme.rs` | 249 | rml_core |
| 5 | `ContributionRegistryExt` | `App` only | `crates/app/src/contribution/global.rs` | 55 | rml_app |
| 6 | `WorkbenchManagerExt` | `App` only | `crates/app/src/workbench/global.rs` | 26 | rml_app |

**证据**：
- 搜索 `(extension\|context_ext\|app_ext\|extensions)\s*mod` 返回 **0 匹配**——无统一扩展模块。
- `crates/app/src/workbench/global.rs:6-8` 文件注释自承"镜像 `ContributionRegistryExt`"，说明作者意识到这是平行模式复制，但未聚合。
- `crates/app/src/workbench/mod.rs` 仅 3 行，`crates/app/src/contribution/mod.rs` 仅 14 行——子模块拆分过细。

### 问题 2：支持的内容是否统一规范？

**结论：双轨制存储 + 不对称的 Context 委托，规范不统一。**

存在两类完全不同的存储范式：

| 范式 | 扩展 trait | 存储 | 返回类型 | Context 委托 |
|---|---|---|---|---|
| GPUI Global | `I18nExt`、`ThemeExt` | `cx.set_global(State)` / `cx.global::<State>()` | 借用 `App` 的引用 | ✅ 有（`Borrow`/`BorrowMut` 委托） |
| OnceLock 静态 | `ContributionRegistryExt`、`WorkbenchManagerExt` | 进程级 `OnceLock<T>` | `&'static` 或 `Option<&'static>` | ❌ 无（仅 `impl for App`） |

**不对称后果**：
- 业务在 `Context<Self>` 内可调 `cx.set_i18n(...)` / `cx.set_theme(...)`（i18n/theme 有 Context 委托）
- 但**不能**在 `Context<Self>` 内调 `cx.get_contribution_registry()` / `cx.set_workbench_manager(...)`（contribution/workbench 仅 App 实现）
- 业务在 ViewModel 内必须先取得 `&App` 才能访问 contribution/workbench——ergonomic 不一致

### 问题 3：是否过度设计或不明确设计？

**结论：存在 3 处明确过度设计 + 1 处不明确设计，与项目记忆约束直接冲突。**

#### 过度设计 1：`entity_cache.rs`（45 行）

- 文件：`crates/app/src/contribution/entity_cache.rs`
- 实现：`OnceLock<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>` 缓存 `WeakEntity<T>`
- **违反约束**：项目记忆明确写道 _"ContributedEntry and ComponentEntityCache are unnecessary; framework does not store registrations/cache, delegating to IContributionHost"_
- 公开 API：`get_or_create_entity<T>(cx)` / `visual_entity<T>(cx)`，被 `demo/src/shell/main_window.rml.rs:108,133` 调用

#### 过度设计 2：`host_handle.rs` 的 channel 桥接（78 行）

- 文件：`crates/app/src/contribution/host_handle.rs`
- 实现：`EntityHostHandle<T>` + `flume::Sender<HostOp>` channel + `drain_host_ops` 模式
- **违反约束**：项目记忆明确写道 _"HostHandle is unnecessary; contributions should be directly delivered to IContributionHost implementations via the registry"_ 和 _"Current IContributionHost implementation requires excessive manual update/bridge logic"_
- 业务侧使用：`Self::__rml_install_host(cx.entity(), cx)` + `rml_app::contribution::drain_host_ops(rx, self)` 样板在 `main_window.rml.rs:75,80`、`activity_panel.rml.rs:68,73` 重复

#### 过度设计 3：`engine/src/runtime/` 三个 stub

- 文件：`crates/engine/src/runtime/component_registry.rs`、`styling.rs`、`watcher.rs`（各 4 行）
- 状态：标注 `Phase A stub`，但项目记忆明确 _"Phase C is rejected; new macros should not be added"_
- **结论**：未完成的死代码，应清理

#### 不明确设计：`ability.rs` 的 unsafe 类型擦除（113 行）

- 文件：`crates/core/src/ability.rs` + `crates/core/src/contribution.rs:90-127`
- 实现：`unsafe` `erase` / `query` / `restore` 函数，支持 `dyn IValue` → `dyn IContribution` / `dyn IVisualContribution` / `dyn ICommand` 多向能力查询
- `crates/core/src/lib.rs` 标注 `#![deny(unsafe_code)]`，但 `ability.rs` 与 `contribution.rs` 用 `#[allow(unsafe_code)]` 局部绕过
- 评估：相当重的元编程机制，当前仅服务于贡献点能力 downcast，**不在本次重构范围**，但需记录为后续评估项

### 问题 4：IAppContext 在业务代码中的访问模式 & 能否统一设计？

**结论：IAppContext 不存在；业务访问模式 4 套并存且不统一；引入 IAppContext（IServiceProvider 风格）完全可行且必要。**

#### 当前业务访问模式（4 套并存）

| 模式 | 是否框架推荐 | 典型调用 | 重复样板 |
|---|---|---|---|
| 框架 `*Ext` trait 扩展 | ✅ 推荐 | `cx.set_i18n()` / `cx.t()` / `cx.set_theme()` | 无 |
| `rml_app::contribution::xxx` 模块函数 | ✅ 推荐 | `drain_host_ops(rx, self)` / `visual_entity::<T>(cx)` | 无 |
| 宏生成方法 | ✅ 推荐 | `Self::__rml_install_host(cx.entity(), cx)` | 无 |
| gpui 原生 + 自创 `DemoShellHost` Global | ❌ 临时方案 | `cx.try_global::<DemoShellHost>().and_then(\|h\| h.0.upgrade()).map(...).update(cx, ...)` | **3 处重复** |

#### 业务自创服务定位器证据

- 定义：`demo/src/shell/main_window.rml.rs:20-22` —— `pub struct DemoShellHost(pub WeakEntity<MainWindow>); impl Global for DemoShellHost {}`
- 注册：`demo/src/shell/main_window.rml.rs:94-95` —— `cx.set_global(DemoShellHost(shell_weak));`
- 消费（3 处重复样板）：
  - `demo/src/shell/activity_panel.rml.rs:104-107`
  - `demo/src/lsp/lsp_explorer_panel.rml.rs:59-67`
  - `demo/src/shell/menu_shell_contribs.rs:19-26`（封装为 `with_main_window` helper，6 个 leaf command 共用）
- **作者承认**：`menu_shell_contribs.rs:14` 文件注释写道 _"统一 6 处 try_global+upgrade+update 样板"_ —— 即作者意识到这是临时方案

#### 抽象边界破坏证据

- `demo/src/cases/i18n_case.rml.rs:28` —— `cx.observe_global::<I18nState>()` 直接引用框架内部 `I18nState` 类型，因为 `I18nExt` 未提供"监听 i18n 变化"扩展方法，业务被迫下钻到 Global

#### 统一设计可行性

完全可行。理由：
1. 现有 `ContributionRegistryExt` 和 `WorkbenchManagerExt` 的 `OnceLock` 静态存储本质就是单例服务，可平滑迁移到 `IServiceProvider` 模式
2. `I18nExt` / `ThemeExt` 的 GPUI Global 存储可作为"领域特定状态"保留（因为 `observe_global` 是 GPUI 原生机制），同时通过 `IAppContext` 提供"查询当前 i18n/theme 服务"统一入口
3. 业务 `DemoShellHost` 模式正是 `IServiceProvider.GetService<T>()` 的手写版——框架提供正式接口后可直接替换
4. `IAppContext` 为 `App` 和 `Context<'_, T>` 同时实现（通过 `Borrow`/`BorrowMut` 委托），解决"contribution/workbench 不能在 Context 内调用"的不对称问题

---

## 二、提议变更（分阶段实施）

### 阶段一：定义与实现 IAppContext 核心

#### Step 1：在 `rml_core` 中定义 `IAppContext` trait 与 `ServiceCollection`

**文件**（新增）：`crates/core/src/context.rs`

**设计**：采用 IServiceProvider 动态查询风格——核心仅 3 个方法，按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`。

```rust
//! IAppContext —— 贯穿整个 RML 应用的统一上下文接口（IServiceProvider 风格）
//!
//! 借鉴 C# `System.IServiceProvider`：所有全局服务（注册表、管理器、业务单例）
//! 通过 `get_service::<T>()` 动态查询。框架提供 trait + `ServiceCollection` 存储，
//! 为 `App` 和 `Context<'_, T>` 同时实现，业务代码可在任意上下文统一访问。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{App, Context};
use gpui::BorrowAppContext;

/// 服务集合——`IAppContext` 的存储后端。
///
/// 按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`，内部 `RwLock` 可变性。
/// 作为 GPUI `Global` 存储，借用 `App`，支持 `observe_global` 触发刷新。
#[derive(Default)]
pub struct ServiceCollection {
    services: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
}

impl ServiceCollection {
    /// 查询服务实例。返回 `Option<Arc<T>>`——未注册返回 `None`。
    pub fn get<T: 'static>(&self) -> Option<Arc<T>> {
        self.services
            .read()
            .unwrap()
            .get(&TypeId::of::<T>())
            .and_then(|any| any.clone().downcast::<T>().ok())
    }

    /// 注册单例服务。重复注册覆盖旧实例。
    pub fn set<T: 'static + Send + Sync>(&self, service: Arc<T>) {
        self.services
            .write()
            .unwrap()
            .insert(TypeId::of::<T>(), service);
    }

    /// 是否已注册。
    pub fn contains<T: 'static>(&self) -> bool {
        self.services
            .read()
            .unwrap()
            .contains_key(&TypeId::of::<T>())
    }
}

impl gpui::Global for ServiceCollection {}

/// IAppContext——贯穿整个 RML 应用的统一上下文接口。
///
/// 等价于 C# `IServiceProvider` + `IServiceCollection`：
/// - `get_service::<T>()` 类比 `GetService<T>()`
/// - `get_required_service::<T>()` 类比 `GetRequiredService<T>()`
/// - `set_service::<T>(instance)` 类比 `TryAddSingleton<T>(instance)`
///
/// 为 `App` 和 `Context<'_, T>` 同时实现，业务代码 `cx.get_service::<T>()`
/// 在任意上下文（启动回调、ViewModel、命令处理器）统一可用。
pub trait IAppContext {
    /// 查询服务实例。未注册返回 `None`。
    fn get_service<T: 'static>(&self) -> Option<Arc<T>>;

    /// 查询必需服务。未注册时 panic 并报告类型名。
    fn get_required_service<T: 'static>(&self) -> Arc<T> {
        self.get_service::<T>().unwrap_or_else(|| {
            panic!(
                "required service `{}` not registered in IAppContext",
                std::any::type_name::<T>()
            )
        })
    }

    /// 注册单例服务。
    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>);

    /// 是否已注册某服务。
    fn has_service<T: 'static>(&self) -> bool;
}

/// 确保 `ServiceCollection` Global 已初始化。
pub fn ensure_service_collection(cx: &mut App) {
    if !cx.has_global::<ServiceCollection>() {
        cx.set_global(ServiceCollection::default());
    }
}

impl IAppContext for App {
    fn get_service<T: 'static>(&self) -> Option<Arc<T>> {
        self.try_global::<ServiceCollection>()
            .and_then(|sc| sc.get::<T>())
    }

    fn set_service<T: 'static + Send + Sync>(&mut self, service: Arc<T>) {
        ensure_service_collection(self);
        self.update_global::<ServiceCollection, _>(|sc, _| sc.set(service));
    }

    fn has_service<T: 'static>(&self) -> bool {
        self.try_global::<ServiceCollection>()
            .map(|sc| sc.contains::<T>())
            .unwrap_or(false)
    }
}

impl<T> IAppContext for Context<'_, T> {
    fn get_service<U: 'static>(&self) -> Option<Arc<U>> {
        IAppContext::get_service(Borrow::<App>::borrow(self))
    }

    fn set_service<U: 'static + Send + Sync>(&mut self, service: Arc<U>) {
        IAppContext::set_service(BorrowMut::<App>::borrow_mut(self), service);
    }

    fn has_service<U: 'static>(&self) -> bool {
        IAppContext::has_service(Borrow::<App>::borrow(self))
    }
}
```

**为什么这样设计**：
- **核心 3 方法**——与 IServiceProvider 完全对应，简洁
- **`Arc<T>` 返回值**——避免生命周期纠缠，`Arc` clone 是原子操作，性能可接受
- **`TypeId` 索引**——Rust 标准做法，类型安全
- **`ServiceCollection` 作为 GPUI Global**——而非 `OnceLock` 静态，与 i18n/theme 范式对齐，支持 `observe_global` 触发刷新
- **同时 impl App + Context**——解决现有 contribution/workbench 仅 App 的不对称问题

#### Step 2：在 `rml_core/src/lib.rs` 导出 IAppContext

**文件**（修改）：`crates/core/src/lib.rs`（41 行）

**变更**：在现有 `pub mod` 声明后新增 `pub mod context;`，并在 `prelude.rs` 中 re-export `IAppContext` 和 `ServiceCollection`。

```rust
// crates/core/src/lib.rs 新增
pub mod context;
pub use context::{IAppContext, ServiceCollection, ensure_service_collection};
```

```rust
// crates/core/src/prelude.rs 新增
pub use crate::context::{IAppContext, ServiceCollection};
```

#### Step 3：在 `RmlApplication::bootstrap_runtime` 中初始化 ServiceCollection

**文件**（修改）：`crates/app/src/application.rs:28-35`

**变更**：在 `bootstrap_runtime` 函数开头初始化 `ServiceCollection`，并在此时注册 `ContributionRegistry` 单例。

```rust
fn bootstrap_runtime(cx: &mut App) {
    // 新增：初始化 IAppContext 的 ServiceCollection
    rml_core::context::ensure_service_collection(cx);
    // 新增：注册 ContributionRegistry 为单例服务
    use std::sync::Arc;
    cx.set_service(Arc::new(crate::contribution::ContributionRegistry::new()));

    ensure_i18n(cx);
    ensure_theme(cx);
    gpui_component::init(cx);
    gpui_component::Theme::global_mut(cx).font_size = px(14.);
    // 删除：crate::contribution::ensure_contribution_registry(cx);
    //       （OnceLock 静态初始化已被 ServiceCollection 替代）
}
```

---

### 阶段二：迁移现有扩展到 IServiceProvider 模式

#### Step 4：迁移 `ContributionRegistry` —— 移除 `ContributionRegistryExt`

**文件**（修改）：`crates/app/src/contribution/global.rs`（48 行 → 精简）

**变更**：
1. 移除 `static REGISTRY: OnceLock<ContributionRegistry>` 和 `fn registry()`
2. 移除 `pub trait ContributionRegistryExt` 和 `impl ContributionRegistryExt for App`
3. 移除 `pub fn ensure_contribution_registry(_cx: &mut App)`（被 Step 3 的 `set_service` 替代）
4. 保留 `static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App, &str)>>` + `install_contribution_bootstrap` + `bootstrap_host_contributions`（这些是 build.rs 回调机制，与存储无关）

**保留后**的文件内容（约 25 行）：

```rust
//! 贡献注册表——构建期回调与 host_id 路由
//!
//! 注册表实例本身存储在 `ServiceCollection`（通过 `IAppContext::get_service` 查询）。
//! 此模块仅保留 build.rs 生成的 `#[ctor::ctor]` 回调安装与 host_id 路由逻辑。

use std::sync::Mutex;
use gpui::App;

static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App, &str)>> = Mutex::new(None);

pub fn install_contribution_bootstrap(f: fn(&mut App, &str)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

pub fn bootstrap_host_contributions(cx: &mut App, host_id: &str) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx, host_id);
    }
}
```

**文件**（修改）：`crates/app/src/contribution/registry.rs`（70 行）

**变更**：将 `ContributionRegistry` 改为 `pub`（原本可能是 crate 私有），使其可被 `set_service::<ContributionRegistry>()` 注册。

#### Step 5：提供 `IAppContextExt` 便利方法（语法糖）

**文件**（新增）：`crates/app/src/extensions.rs`

**目的**：为常用服务提供语义化方法，避免业务侧写 `cx.get_required_service::<ContributionRegistry>()` 这种冗长调用。

```rust
//! IAppContext 扩展——为常用服务提供语义化便利方法
//!
//! 这些方法是 `IAppContext::get_service::<T>()` 的语法糖，
//! 不引入新的存储机制，仅转发到 `ServiceCollection`。

use std::sync::Arc;
use gpui::App;
use rml_core::context::IAppContext;
use rml_core::contribution::IContributionRegistry;
use rml_core::workbench::IWorkbenchManager;

use crate::contribution::ContributionRegistry;
use crate::workbench::WorkbenchManagerSlot;

pub trait IAppContextExt: IAppContext {
    /// 获取贡献注册表（必需服务）。
    fn contribution_registry(&self) -> Arc<ContributionRegistry> {
        self.get_required_service::<ContributionRegistry>()
    }

    /// 获取贡献注册表的 trait object 视图。
    fn contribution_registry_dyn(&self) -> Arc<dyn IContributionRegistry> {
        self.contribution_registry() as Arc<dyn IContributionRegistry>
    }

    /// 安装工作台管理器。
    fn set_workbench_manager(&mut self, manager: Arc<dyn IWorkbenchManager>) {
        self.set_service(Arc::new(WorkbenchManagerSlot(manager)));
    }

    /// 获取已安装的工作台管理器。
    fn workbench_manager(&self) -> Option<Arc<dyn IWorkbenchManager>> {
        self.get_service::<WorkbenchManagerSlot>().map(|s| s.0.clone())
    }
}

impl IAppContextExt for App {}
```

**注意**：`WorkbenchManagerSlot` 是为存储 `dyn IContributionHost` 这类 trait object 而引入的 newtype（因为 `Arc<dyn Trait>` 不能直接作为 `T: 'static + Send + Sync` 泛型参数被 downcast）。在 `crates/app/src/workbench/global.rs` 中定义：

```rust
// crates/app/src/workbench/global.rs（重写后，约 10 行）
use std::sync::Arc;
use rml_core::workbench::IWorkbenchManager;

/// 工作台管理器注册槽位（newtype 包装 `Arc<dyn IWorkbenchManager>` 以便存入 `ServiceCollection`）。
pub struct WorkbenchManagerSlot(pub Arc<dyn IWorkbenchManager + Send + Sync>);
```

**注意**：需要确保 `IWorkbenchManager` 在 trait 定义中包含 `Send + Sync` supertrait，或在 `WorkbenchManagerSlot` 中显式约束。需在 Step 4 实施时检查 `crates/core/src/workbench.rs` 中 `IWorkbenchManager` 定义。

#### Step 6：迁移 `WorkbenchManager` —— 移除 `WorkbenchManagerExt`

**文件**（修改）：`crates/app/src/workbench/global.rs`（27 行 → 10 行）

**变更**：
1. 移除 `static WORKBENCH_MANAGER: OnceLock<Arc<dyn IWorkbenchManager>>`
2. 移除 `pub trait WorkbenchManagerExt` 和 `impl WorkbenchManagerExt for App`
3. 仅保留 `WorkbenchManagerSlot` newtype 定义
4. `set_workbench_manager` / `get_workbench_manager` 由 Step 5 的 `IAppContextExt` 提供

#### Step 7：保留 `I18nExt` 和 `ThemeExt` 不变

**理由**：
- i18n/theme 是"领域特定状态"（带 `observe_global` 副作用），不适合通过 `get_service::<T>()` 查询
- `I18nExt::set_i18n()` 触发 `refresh_windows()`，`ThemeExt::set_theme()` 触发样式重计算——这些是有状态的领域操作，不是单纯服务查询
- 现有 `I18nExt` / `ThemeExt` 已为 App + Context 双实现，符合统一规范

**唯一补充**：在 `I18nExt` 中新增 `observe_i18n` 方法，避免业务下钻到 `cx.observe_global::<I18nState>()`：

**文件**（修改）：`crates/core/src/i18n.rs`

```rust
// 在 I18nExt trait 中新增
pub trait I18nExt {
    // ... 现有方法 ...

    /// 监听 i18n 状态变化（封装 observe_global::<I18nState>）。
    fn observe_i18n(&mut self, f: impl Fn(&mut Self, &mut App) + 'static) -> gpui::Subscription
    where
        Self: Sized;
}

// impl for App 中
impl I18nExt for App {
    fn observe_i18n(&mut self, f: impl Fn(&mut Self, &mut App) + 'static) -> gpui::Subscription {
        ensure_i18n(self);
        self.observe_global::<I18nState>(f)
    }
}

// impl for Context<'_, T> 中
impl<T> I18nExt for Context<'_, T> {
    fn observe_i18n(&mut self, f: impl Fn(&mut Self, &mut App) + 'static) -> gpui::Subscription {
        // Context 的 observe_global 签名与 App 不同，需通过 entity 桥接
        // 此处可能需要调整签名或保留业务侧用 cx.observe_global
        todo!("实施时确认 Context::observe_global 签名")
    }
}
```

**实施时注意**：`Context::observe_global` 签名是 `Fn(&mut T, &mut Context<T>)` 而非 `Fn(&mut Self, &mut App)`，可能需要调整 trait 签名。若实施时发现 Context 委托困难，可降级为只 impl for App，业务侧在 Context 内通过 `cx.app` 或 `Borrow` 取得 App 后调用。

---

### 阶段三：清理过度设计

#### Step 8：移除 `entity_cache.rs`

**文件**（删除）：`crates/app/src/contribution/entity_cache.rs`（45 行）

**理由**：项目记忆明确 _"ComponentEntityCache is unnecessary; framework does not store registrations/cache"_

**影响**：
- `crates/app/src/contribution/mod.rs` 移除 `mod entity_cache;` 和 `pub use entity_cache::{get_or_create_entity, visual_entity};`
- `crates/app/src/lib.rs:18` 移除 `get_or_create_entity, visual_entity` re-export
- 业务调用点 `demo/src/shell/main_window.rml.rs:108,133` 中的 `rml_app::contribution::visual_entity::<ActivityPanel>(cx)` 需改用 `IAppContext::get_service::<ActivityPanel>()` 查询（业务侧需在创建 ActivityPanel 时通过 `cx.set_service(Arc::new(entity.clone()))` 注册）

**业务侧替换模式**：

```rust
// 创建时注册（替代 visual_entity 缓存）
let panel_entity = cx.new(|_| ActivityBar::new(panels));
cx.set_service(panel_entity.clone());  // 注册为单例

// 查询时获取（替代 visual_entity::<T>(cx)）
let panel_entity = cx.get_required_service::<gpui::Entity<ActivityPanel>>();
```

#### Step 9：简化 `host_handle.rs` —— 移除 channel 桥接

**文件**（修改）：`crates/app/src/contribution/host_handle.rs`（78 行 → 约 30 行）

**理由**：项目记忆明确 _"HostHandle is unnecessary; contributions should be directly delivered to IContributionHost implementations via the registry"_ 和 _"Current IContributionHost implementation requires excessive manual update/bridge logic"_

**设计决策**：此步**仅做最小化简化**，不彻底重构 host 机制（彻底重构需修改 `#[contributehost]` 宏生成代码，影响面过大，超出本次范围）。具体：

1. 保留 `EntityHostHandle<T>` 类型（因为 Entity 不能直接 `Arc<dyn IContributionHost>`，需包装）
2. **移除 `flume::Sender<HostOp>` channel**——改为 `EntityHostHandle` 直接持有 `Entity<T>` 的 weak 引用，`add`/`remove` 操作直接 `entity.update(cx, |host, _| host.add(...))` 同步执行
3. 移除 `HostOp` 枚举和 `drain_host_ops` 函数
4. 业务侧 `on_loaded` 不再需要调用 `drain_host_ops(rx, self)`——贡献注册时 host 已直接接收

**简化后**的 `host_handle.rs`（约 30 行）：

```rust
//! Entity host 桥接器——让 `Entity<T: IContributionHost>` 可被 registry 持有
//!
//! 简化设计：直接同步调用 entity.update，不再用 channel。

use std::sync::Arc;
use gpui::{App, Entity};
use rml_core::contribution::{IContribution, IContributionHost, ContributionOptions};

pub struct EntityHostHandle<T: IContributionHost> {
    pub(crate) entity: Entity<T>,
    pub(crate) id: &'static str,
}

impl<T: IContributionHost + 'static> EntityHostHandle<T> {
    pub fn new(id: &'static str, entity: Entity<T>) -> Self {
        Self { entity, id }
    }
}

impl<T: IContributionHost + 'static> IContributionHost for EntityHostHandle<T> {
    fn id(&self) -> &str { self.id }

    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App) {
        let entity = self.entity.clone();
        entity.update(cx, |host, _| {
            host.add(contribution, options, cx);
        });
    }

    fn remove(&self, contribution_id: &str, cx: &mut App) {
        let entity = self.entity.clone();
        entity.update(cx, |host, _| {
            host.remove(contribution_id, cx);
        });
    }
}
```

**注意**：此变更要求 `IContributionHost::add` / `remove` 签名包含 `&mut App` 参数。需检查 `crates/core/src/contribution.rs` 中 `IContributionHost` trait 定义，若签名不匹配需同步调整（这属于核心 trait 修改，需谨慎）。

**降级方案**：若 `IContributionHost` trait 修改影响面过大，本步骤可降级为"仅标记为后续重构项"，保留现有 channel 机制，仅清理 `entity_cache.rs`。

#### Step 10：清理 `engine/src/runtime/` 三个 stub

**文件**（删除）：
- `crates/engine/src/runtime/component_registry.rs`（4 行）
- `crates/engine/src/runtime/styling.rs`（4 行）
- `crates/engine/src/runtime/watcher.rs`（4 行）

**文件**（修改）：`crates/engine/src/runtime/mod.rs`（8 行 → 3 行）

移除三个 `mod` 声明，仅保留 `pub mod event_flow;`（152 行，有实际实现）。

**理由**：Phase A 残留死代码，项目记忆明确 _"Phase C is rejected"_

---

### 阶段四：业务代码更新

#### Step 11：替换 `DemoShellHost` 为 IAppContext 查询

**文件**（修改）：`demo/src/shell/main_window.rml.rs`

**变更**：
1. 删除 `DemoShellHost` 结构定义（行 20-22）
2. 删除 `cx.set_global(DemoShellHost(shell_weak));`（行 95）
3. 替换为 `cx.set_service(Arc::new(shell_weak));` 或更语义化的 newtype：

```rust
// 新增 newtype（避免与其它 WeakEntity<T> 冲突）
pub struct MainWindowRef(pub gpui::WeakEntity<MainWindow>);

// 注册（替代 DemoShellHost）
let shell_weak = cx.weak_entity();
cx.set_service(std::sync::Arc::new(MainWindowRef(shell_weak)));
```

**文件**（修改）：`demo/src/shell/activity_panel.rml.rs:104-107`

```rust
// 替换前
let host = cx
    .try_global::<DemoShellHost>()
    .and_then(|h| h.0.upgrade());

// 替换后
use rml_core::context::IAppContext;
let host = cx
    .get_service::<MainWindowRef>()
    .and_then(|r| r.0.upgrade());
```

**文件**（修改）：`demo/src/lsp/lsp_explorer_panel.rml.rs:59-67` —— 同上模式替换

**文件**（修改）：`demo/src/shell/menu_shell_contribs.rs:15-26` —— `with_main_window` helper 内的 `ctx.app.try_global::<DemoShellHost>()` 改为 `ctx.app.get_service::<MainWindowRef>()`

#### Step 12：替换业务代码中的 `visual_entity::<T>(cx)` 调用

**文件**（修改）：`demo/src/shell/main_window.rml.rs:108,133`

```rust
// 替换前
let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);

// 替换后（在创建时注册，查询时获取）
// 创建处：
let panel_entity = cx.new(|_| ActivityBar::new(panels));
cx.set_service(panel_entity.clone());

// 查询处：
use rml_core::context::IAppContext;
let panel_entity = cx.get_required_service::<gpui::Entity<ActivityPanel>>();
```

#### Step 13：替换业务代码中的 `drain_host_ops` 调用（依赖 Step 9 完成）

**文件**（修改）：`demo/src/shell/main_window.rml.rs:75,80` 和 `demo/src/shell/activity_panel.rml.rs:68,73`

若 Step 9 完成，移除 `Self::__rml_install_host` 返回的 `rx` 和 `drain_host_ops(rx, self)` 调用。

**注意**：此步骤依赖 `#[contributehost]` 宏生成代码的同步修改——`__rml_install_host` 函数签名需从返回 `Receiver<HostOp>` 改为返回 `()`。**宏修改不在本次实施范围**，Step 9 实施时若不修改宏，本步骤跳过，保留 `drain_host_ops` 调用但内部逻辑已简化。

---

### 阶段五：聚合与导出

#### Step 14：创建中央 `extensions` 聚合模块

**文件**（新增）：`crates/app/src/extensions.rs`（Step 5 已创建部分，此处补全 re-export）

```rust
//! RML App/Context 扩展中央聚合点
//!
//! 统一 re-export 所有 App/Context 扩展 trait，业务代码只需 `use rml_app::prelude::*`
//! 即可获得全部扩展方法。

// IAppContext 核心
pub use rml_core::context::{IAppContext, ServiceCollection, ensure_service_collection};

// IAppContext 便利方法
pub use crate::extensions_impl::IAppContextExt;

// 领域特定扩展（保留原状）
pub use rml_core::i18n::I18nExt;
pub use rml_core::theme::ThemeExt;

// 能力查询扩展
pub use rml_core::contribution::{VisualAbilityExt, ContributionAbilityExt};
pub use rml_core::command::CommandAbilityExt;
```

**文件**（修改）：`crates/app/src/lib.rs`

```rust
// 新增
pub mod extensions;
pub use extensions::{IAppContext, IAppContextExt, ServiceCollection};
```

**文件**（新增/修改）：`crates/app/src/prelude.rs`

```rust
//! RML 应用层 prelude——业务代码 `use rml_app::prelude::*` 获得全部扩展

pub use crate::extensions::*;
pub use crate::lifecycle::IAppLifecycle;
```

#### Step 15：更新 `crates/app/src/lib.rs` 导出

**文件**（修改）：`crates/app/src/lib.rs`（21 行 → 重写）

```rust
//! RML 应用启动器
//!
//! 提供 `RmlApplication` 作为应用入口，封装 GPUI 的窗口创建与生命周期管理。
//! 通过 `IAppContext`（IServiceProvider 风格）统一全局服务访问。

#![forbid(unsafe_code)]

extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

pub mod application;
pub mod contribution;
pub mod extensions;
pub mod lifecycle;
pub mod resources;
pub mod workbench;

pub mod prelude;

pub use application::{NoWindow, RmlApplication};
pub use lifecycle::IAppLifecycle;
pub use resources::{
    load_i18n_catalog, load_i18n_from_json, load_theme_colors, load_theme_css,
    DEFAULT_I18N_DIR, DEFAULT_THEMES_DIR,
};

// IAppContext 核心 + 便利方法
pub use extensions::{IAppContext, IAppContextExt, ServiceCollection};

// 移除以下导出（已迁移）
// pub use contribution::{ensure_contribution_registry, get_or_create_entity, visual_entity, ContributionRegistryExt};
// pub use workbench::WorkbenchManagerExt;
```

---

## 三、假设与决策

### 设计决策

1. **IServiceProvider 动态查询风格**（用户确认）：核心 3 方法 `get_service<T>()` / `get_required_service<T>()` / `set_service<T>(instance)`，按 `TypeId` 索引 `Arc<dyn Any + Send + Sync>`
2. **`ServiceCollection` 作为 GPUI Global**（非 OnceLock 静态）：与 i18n/theme 范式对齐，支持 `observe_global`，避免引入第三种存储机制
3. **`Arc<T>` 返回值**：避免生命周期纠缠，原子 clone 性能可接受
4. **保留 `I18nExt` / `ThemeExt` 不变**：它们是有状态领域操作（触发 refresh），不是单纯服务查询
5. **`IAppContextExt` 提供 trait object 便利方法**：`contribution_registry()` / `workbench_manager()` 是语法糖，不引入新存储
6. **`WorkbenchManagerSlot` newtype**：解决 `Arc<dyn Trait>` 不能直接 downcast 的问题
7. **Step 9 简化 host_handle 为最小化变更**：不彻底重构 host 机制（避免修改宏），仅移除 channel 改为同步调用

### 假设

1. **`IContributionHost::add` / `remove` 签名可调整**：若需加入 `&mut App` 参数，会修改 `crates/core/src/contribution.rs` 中 trait 定义。若不可调整，Step 9 降级为"仅标记后续重构"
2. **`IWorkbenchManager: Send + Sync`**：需检查 `crates/core/src/workbench.rs` 中 trait 定义，若不含则 `WorkbenchManagerSlot` 需显式约束
3. **业务 `demo/` 代码可同步修改**：Step 11-13 替换 `DemoShellHost` 和 `visual_entity` 调用，需业务侧配合
4. **`#[contributehost]` 宏不修改**：Step 9 不修改宏生成代码，`__rml_install_host` 签名保持不变；若 Step 9 实施后发现 channel 必须保留，则降级
5. **`Context::observe_global` 签名兼容性**：Step 7 中 `I18nExt::observe_i18n` 在 Context 上的实现可能需调整签名，实施时验证

### 不在本次范围

1. **`ability.rs` 的 unsafe 类型擦除机制重构**——记录为后续评估项
2. **`#[contributehost]` 宏生成代码重构**——影响面过大，单独评估
3. **`compiler/component.rs` (1324 行) / `compiler/expr.rs` (1167 行) 拆分**——与 IAppContext 无关
4. **`ContributionOptions` 弱类型 HashMap 改造**——独立议题

---

## 四、验证步骤

### 阶段一验证（IAppContext 核心）

```bash
# 在 e:\GitCode\RF\rust-gpui-rml 目录
cargo check -p rust-rml-core
cargo check -p rust-rml-app
```

**预期**：编译通过，无 warning。

### 阶段二验证（迁移现有扩展）

```bash
cargo build -p rust-rml-app
cargo build -p rust-rml-core
```

**预期**：
- `ContributionRegistryExt` / `WorkbenchManagerExt` 引用全部消失
- `I18nExt` / `ThemeExt` 保持不变
- `IAppContextExt` 提供 `contribution_registry()` / `workbench_manager()` 便利方法

### 阶段三验证（清理过度设计）

```bash
cargo build --workspace
```

**预期**：
- `entity_cache.rs` 删除后无编译错误（业务调用点已替换）
- `host_handle.rs` 简化后 `drain_host_ops` 调用仍可用（若 Step 9 降级则保留）
- `engine/src/runtime/` 三个 stub 删除后无编译错误

### 阶段四验证（业务代码更新）

```bash
cargo build -p demo
```

**预期**：
- `DemoShellHost` 定义和 `try_global::<DemoShellHost>()` 调用全部消失
- 业务代码使用 `cx.get_service::<MainWindowRef>()` 替代
- `visual_entity::<T>(cx)` 调用替换为 `cx.get_required_service::<Entity<T>>()`

### 阶段五验证（聚合与导出）

```bash
cargo build --workspace
cargo test --workspace
```

**预期**：
- `use rml_app::prelude::*` 可获得 `IAppContext` / `IAppContextExt` / `I18nExt` / `ThemeExt` / `IAppLifecycle`
- 业务代码 `cx.set_i18n(...)` / `cx.get_service::<T>()` / `cx.contribution_registry()` 在任意上下文（App / Context）统一可用

### 运行时验证

```bash
cargo run -p demo
```

**预期**：
- 应用启动正常，i18n / theme 切换功能正常
- ActivityBar 点击切换面板正常
- LSP Explorer 文件激活打开代码编辑器正常
- 菜单命令（退出、切换主题等）正常执行

### 架构一致性检查

实施完成后，重新执行 Phase 1 探索的 grep 命令：

```bash
# 应返回 0 匹配（DemoShellHost 已移除）
rg "DemoShellHost" demo/

# 应返回 0 匹配（ContributionRegistryExt 已移除）
rg "ContributionRegistryExt" crates/

# 应返回 0 匹配（WorkbenchManagerExt 已移除）
rg "WorkbenchManagerExt" crates/

# 应返回 1 处定义（IAppContext trait）
rg "trait IAppContext" crates/

# 应返回多处 impl（App + Context + 可能的扩展）
rg "impl.*IAppContext.*for" crates/
```

---

## 五、实施顺序与依赖

```
Step 1 ─┐
        ├─→ Step 2 ─→ Step 3 ─┐
Step 4 ─┤                      ├─→ Step 8 ─┐
Step 5 ─┤                      │           ├─→ Step 11 ─┐
Step 6 ─┤                      │           │           ├─→ Step 14 ─┐
Step 7 ─┘                      │           │           │           ├─→ Step 15 ─→ 验证
                                ├─→ Step 9 ─┤           │
                                │           ├─→ Step 12 ─┤
                                │           │           │
                                └─→ Step 10 ─┴─→ Step 13 ┘
```

**关键路径**：Step 1 → Step 2 → Step 3 → Step 5 → Step 14 → Step 15

**可并行**：
- Step 4 / Step 6 / Step 7 可并行（独立模块迁移）
- Step 8 / Step 10 可并行（独立文件删除）
- Step 11 / Step 12 / Step 13 可并行（不同业务文件）

**风险点**：
- Step 9（简化 host_handle）涉及 `IContributionHost` trait 签名，可能影响宏生成代码——建议作为可选步骤，先跳过验证整体流程
- Step 7（`observe_i18n` 在 Context 上的实现）可能因 `Context::observe_global` 签名困难而降级

---

## 六、回滚方案

若实施过程中发现关键阻塞：

1. **Step 9 阻塞**：保留 `host_handle.rs` 现状，仅完成其他步骤。`entity_cache.rs` 仍可安全删除（与 host_handle 解耦）
2. **Step 7 阻塞**：`I18nExt::observe_i18n` 仅 impl for App，业务侧在 Context 内通过 `Borrow` 取得 App 后调用
3. **整体阻塞**：保留 `ContributionRegistryExt` / `WorkbenchManagerExt` 作为兼容层，新增 `IAppContext` 作为推荐 API，业务侧逐步迁移

每个 Step 提交独立 commit，便于回滚。
