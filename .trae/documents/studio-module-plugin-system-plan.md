# Studio 系统模块与扩展插件系统设计 Plan

## 摘要

本计划落地两件事：

1. **DI 重构**：消除 `crates/di` 自实现 `ServiceCollection`/`ServiceProvider` 的技术债务，主容器与子容器统一基于 `rust-dix 0.6` 构建；RML 通过 `IServiceProvider` trait 保持独立性，对 `IAppContext` 仅做小侵入扩展。
2. **模块/插件系统**：新增 `IModule`（系统模块）/`IPackage`（插件扩展）/`IPackageContext`（插件上下文，主从容器桥接 + Handle + 自动清理）三大接口；将 `studio/editor`、`studio/explorer`、`studio/chat` 迁移为 `IModule` 实现；dll 动态加载暂不实现，仅建立接口契约与静态注册流程，未来切换 dll 加载只需替换加载器。

设计原则：**低认知负荷**——开发者专注使用接口能力即可构建应用，无需关心容器细节、贡献点清理、加载顺序等。

***

## 一、当前状态分析

### 1.1 现有 DI 架构（技术债务）

| 文件                               | 现状                                                                                                                                  | 问题                           |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- |
| `crates/core/src/context.rs`     | 定义 `IServiceProvider` trait（object-safe）+ `IAppContext: IServiceProvider` + `DefaultServiceProvider` + `ServiceProviderSlot` Global | 抽象层正确，无问题                    |
| `crates/di/src/collection.rs`    | 自实现 `ServiceCollection`（`HashMap<TypeId, FactoryFn>`）                                                                               | 与 rust-dix API 高度重合，重复造轮子    |
| `crates/di/src/provider.rs`      | 自实现 `ServiceProvider`（factory map + cache）                                                                                          | 同上，缺少作用域、循环依赖检测、Cross-DLL 支持 |
| `crates/di/src/auto_register.rs` | `#[ctor::ctor]` + `OnceLock<Mutex<Vec<RegisterFn>>>` 全局注册表                                                                          | 模式合理，需保留                     |
| `crates/di/src/configure.rs`     | `RmlApplicationExt::configure` 链式 API                                                                                               | 模式合理，需保留                     |
| `Cargo.toml`                     | `rust-dix = "0.6"` 在 workspace.dependencies 声明                                                                                      | **未被实际引用**                   |

### 1.2 现有 Studio 扩展模式

| 文件                           | 现状                                                                                   |
| ---------------------------- | ------------------------------------------------------------------------------------ |
| `studio/editor/src/lib.rs`   | `#[ctor::ctor]` + `auto_register` 注册 `EditorProvider` 为 `IWorkbenchProvider("file")` |
| `studio/explorer/src/lib.rs` | 同上模式                                                                                 |
| `studio/chat/src/lib.rs`     | 同上模式 + 注册 `IChatProvider`                                                            |
| `studio/shell/src/lib.rs`    | `#[ctor::ctor]` 注册 `WelcomeProvider` 为 `IWorkbenchProvider("rml")`                   |
| `studio/app/src/main.rs`     | `extern crate studio_editor as _;` 强制链接 feature crate                                |
| `studio/shell/src/di.rs`     | `build_runtime_provider` 构造主容器，注册 `ArcShellManager` 等                                |

**问题**：扩展模块的注册逻辑分散在 `lib.rs` 的 `#[ctor::ctor]` 中，无统一生命周期入口；`auto_register` 只能注册 DI 服务，无法表达"主窗口加载后才能注册面板"等阶段化需求。

### 1.3 现有启动流程

`RmlApplication::new().main_window::<MainWindow>().configure(|s|{...}).run::<Startup>()` 内部：

1. `bootstrap_runtime(cx)` —— `ensure_service_provider(cx)` + `cx.set_service(ContributionRegistry::new())` + i18n/theme/gpui\_component init
2. 注入 configure 阶段的 provider 到 `cx.use_provider`
3. `Startup::on_launch(cx)` —— 设置样式/i18n/主题
4. `MainWindow::open(cx)` —— 打开主窗口，触发 `on_loaded`

***

## 二、rust-dix 0.6 关键能力对照

| rust-dix 能力                                                             | 用途                                                                     |
| ----------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `ServiceCollection::singleton::<T>(f)` / `keyed_singleton::<T>(key, f)` | 主容器服务注册（与现有 API 一致）                                                    |
| `ServiceProvider`                                                       | 根 DI 容器，`get::<T>()` / `get_keyed::<T>(key)` 返回 `Arc<T>`               |
| `ServiceProviderWrapper`                                                | **child-first layered container** —— 子容器先查，root 回退，正是子容器桥接主容器所需        |
| `IProvider` trait                                                       | 统一 resolution + named registry，支持 Cross-DLL named services（为未来 dll 预留） |
| `IServiceResolver`                                                      | 解析 trait（`get_any(type_name)` object-safe）                             |
| `Scope` / `ServiceScope`                                                | 作用域容器（本次不使用，保留扩展空间）                                                    |
| `#[derive(Inject)]` / `#[rust_dix::inject]`                             | 构造函数注入 + 自动注册（本次不强制推广，保留为未来选项）                                         |
| Build-time 循环依赖检测                                                       | 重构后免费获得                                                                |

***

## 三、设计决策

### D1. RML 独立性边界

* `IServiceProvider` trait（在 `crates/core/src/context.rs`）保持不变 —— 这是 RML 对外契约

* `crates/di` 内部用 rust-dix 实现，对外保持现有 `ServiceCollection`/`ServiceProvider`/`RmlApplicationExt::configure`/`auto_register` API 不变 —— 现有调用点零修改

* rust-dix 类型不暴露到 `crates/core`、`crates/app`、`studio/*` —— 仅 `crates/di` 依赖 rust-dix

### D2. 子容器桥接策略

* 主容器：`rust_dix::ServiceProvider` 包装为 RML `IServiceProvider`

* 子容器：`rust_dix::ServiceProviderWrapper`（child-first + root fallback）包装为 RML `IServiceProvider`

* `PackageContext` impl `IServiceProvider` 时，先查 `ServiceProviderWrapper`（自动 child-first → root fallback），无需手动拼接查询逻辑

### D3. IModule 与 IPackage 角色区分

* `IModule`：studio 内置系统模块（编译期已知，静态注册），通过 `IModuleRegistry` 按 `order` 排序后调用 `initialize`/`loaded`

* `IPackage`：插件扩展（当前静态注册，未来 dll 动态加载），通过 `IPackageManager` 调用 `load`/`unload`，每个 IPackage 拥有独立子容器 + 自动贡献点清理

* 二者均注册到主容器，但生命周期管理独立

### D4. IPackage.load 参数语义

* `IPackage::load(&mut self, cx: &mut App)` / `unload(&mut self, cx: &mut App)` —— 主程序调度时透传 `IAppContext` 能力

* `IPackage` 实现通过 **factory 注入** 持有 `Arc<dyn IPackageContext>`（构造时由 `PackageManager` 创建并注入）

* 插件内部组件通过 `IPackageContext` 访问能力，主程序通过 `IAppContext` 调度插件

### D5. IPackageContext 能力范围

* `IPackageContext: IServiceProvider`（主从容器桥接访问）

* 携带 `PackageHandle`（id/name/version 元数据）

* 提供 `register_contribution` 便捷方法（代理 `IContributionRegistry`，自动记录用于 unload 清理）

* 提供 `register_service` 注册插件内部服务到子容器

* unload 时 `PackageManager` 自动 `unregister` 本插件所有贡献点 + drop 子容器

### D6. 业务模块迁移范围

* `studio/editor`、`studio/explorer`、`studio/chat` 迁移为 `IModule` 实现

* `studio/shell` 作为主窗口外壳不迁移，继续作为 `IModuleRegistry` + `IPackageManager` 宿主

* 现有 `#[ctor::ctor]` + `auto_register` 注册 `IWorkbenchProvider` 的逻辑保留，迁入 `IModule::initialize` 内部调用（不破坏现有服务注册模式）

### D7. dll 动态加载预留

* 本计划不实现 dll 加载，`IPackage` 仅支持静态 crate 注册

* 接口设计预留未来 dll 加载扩展点：`IPackage` trait 的 `Send + Sync + 'static` 约束 + `PackageHandle` 元数据 + rust-dix `IProvider` named registry 已支持 Cross-DLL

* 未来切换 dll 加载仅需：新增 `PackageLoader`（dlopen + extern "C" 入口）、扩展 `PackageManager::register_dynamic`，无需修改 `IPackage`/`IPackageContext` 接口

***

## 四、变更清单

### 4.1 DI 重构（crates/di）

#### `crates/di/Cargo.toml`

* 添加 `rust-dix = { workspace = true }` 依赖

#### `crates/di/src/collection.rs` （重写）

* `ServiceCollection` 改为薄包装 `rust_dix::ServiceCollection`

* 保持对外 API 不变：`new()` / `add_singleton::<T>(factory)` / `add_keyed_singleton::<T>(key, factory)` / `build() -> Arc<dyn IServiceProvider + Send + Sync>`

* 内部通过 `rust_dix::ServiceCollection::singleton::<ServiceSlot<T>>` 注册（保留 `ServiceSlot` 桥接 trait object 的现有模式）

* `build()` 调用 `rust_dix::ServiceCollection::build()`，包装为 `ServiceProvider`，再 impl RML `IServiceProvider`

#### `crates/di/src/provider.rs` （重写）

* `ServiceProvider` 改为包装 `Arc<rust_dix::ServiceProvider>`

* impl RML `IServiceProvider`：`get_service_any`/`get_keyed_service_any`/`has_service_any` 委托给内部 rust-dix 容器

* 保持 `Send + Sync`（rust-dix ServiceProvider 已支持）

#### `crates/di/src/wrapper.rs` （新增）

* `ServiceProviderWrapper` 包装 `rust_dix::ServiceProviderWrapper`

* 提供 `child_first` 子容器桥接：构造时接收 `parent: Arc<dyn IServiceProvider + Send + Sync>` 与 `child_collection: ServiceCollection`

* impl RML `IServiceProvider`：委托给 rust-dix wrapper（自动 child-first → root fallback）

* 对外 API：`fn new(parent, child) -> Self` / `fn service_provider(&self) -> Arc<dyn IServiceProvider + Send + Sync>`

#### `crates/di/src/lib.rs` （更新导出）

* 新增 `pub use wrapper::ServiceProviderWrapper;`

* 其他导出保持不变

#### `crates/di/src/auto_register.rs` / `configure.rs`

* **保持不变** —— `auto_register` 全局注册表 + `RmlApplicationExt::configure` 链式 API 向后兼容

### 4.2 RML Core 扩展（crates/core）

#### `crates/core/src/context.rs` （新增扩展 trait）

* 新增 `IAppContextExt: IAppContext` trait，提供便捷访问方法（blanket impl 对所有 `IAppContext` 实现）：

  * `fn modules(&self) -> Arc<dyn IModuleRegistry>` —— 解析模块注册表

  * `fn packages(&self) -> Arc<dyn IPackageManager>` —— 解析插件管理器

  * `fn contributions(&self) -> Arc<dyn IContributionRegistry>` —— 解析贡献注册表

* 注意：`IModuleRegistry`/`IPackageManager` 定义在 `studio-core`，`crates/core` 不依赖 `studio-core`，故 `IAppContextExt` 应放在 `studio-core` 而非 `crates/core`（见 4.3）

#### `crates/core/src/context.rs` 实际改动

* **不改** `IServiceProvider`/`IAppContext`/`DefaultServiceProvider`/`ServiceProviderSlot` —— RML 独立性保持

* 仅可能新增 `ServiceProviderExt` 的辅助方法（如有需要）

### 4.3 Studio Core 新增模块（studio/core）

#### `studio/core/src/module.rs` （新增）

```rust
//! 系统模块契约 —— studio 内置可扩展平台的统一入口

use gpui::App;

/// 系统模块接口
pub trait IModule: Send + Sync + 'static {
    /// 模块唯一标识
    fn id(&self) -> &'static str;
    /// 初始化（应用启动时，主窗口创建前）
    fn initialize(&mut self, cx: &mut App);
    /// 主窗口加载完成时调用（默认空实现）
    fn loaded(&mut self, cx: &mut App) {}
    /// 执行顺序（升序，越小越先执行，默认 0）
    fn order(&self) -> i32 { 0 }
}

/// 模块注册表接口
pub trait IModuleRegistry: Send + Sync {
    fn register(&self, module: Box<dyn IModule>);
    fn initialize_all(&self, cx: &mut App);
    fn loaded_all(&self, cx: &mut App);
}

/// 框架内部实现
pub struct ModuleRegistry {
    modules: RwLock<Vec<Box<dyn IModule>>>,
}

impl IModuleRegistry for ModuleRegistry {
    fn register(&self, mut module: Box<dyn IModule>) {
        self.modules.write().unwrap().push(module);
    }
    fn initialize_all(&self, cx: &mut App) {
        let mut mods = self.modules.write().unwrap();
        mods.sort_by_key(|m| m.order());
        for m in mods.iter_mut() { m.initialize(cx); }
    }
    fn loaded_all(&self, cx: &mut App) {
        let mut mods = self.modules.write().unwrap();
        mods.sort_by_key(|m| m.order());
        for m in mods.iter_mut() { m.loaded(cx); }
    }
}
```

#### `studio/core/src/package.rs` （新增）

```rust
//! 插件扩展契约 —— IPackage/IPackageContext/PackageHandle

use std::any::TypeId;
use std::sync::{Arc, RwLock};
use gpui::{App, SharedString};
use rml_core::context::IServiceProvider;
use rml_core::contribution::{IContribution, ContributionOptions, IContributionRegistry};

/// 插件元数据
#[derive(Debug, Clone)]
pub struct PackageHandle {
    pub id: SharedString,
    pub name: SharedString,
    pub version: SharedString,
}

/// 插件上下文接口 —— 主从容器桥接 + 元数据 + 贡献点便捷注册 + 自动清理
pub trait IPackageContext: IServiceProvider {
    fn handle(&self) -> &PackageHandle;
    fn register_contribution(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    );
    fn register_service<T: ?Sized + 'static + Send + Sync>(
        &self,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    );
}

/// 插件接口 —— 主程序调度时透传 IAppContext
pub trait IPackage: Send + Sync + 'static {
    fn handle(&self) -> PackageHandle;
    fn load(&mut self, cx: &mut App);
    fn unload(&mut self, cx: &mut App);
}

/// 框架内部实现 —— 子容器+主容器桥接 + 贡献点自动清理
pub struct PackageContext {
    handle: PackageHandle,
    /// 子容器（rust-dix ServiceProviderWrapper，桥接主容器）
    child: Arc<dyn IServiceProvider + Send + Sync>,
    /// 主容器引用
    parent: Arc<dyn IServiceProvider + Send + Sync>,
    /// 已注册贡献点 (host_id, contribution_id)，unload 时自动清理
    contributions: RwLock<Vec<(String, String)>>,
}

impl PackageContext {
    pub fn new(
        handle: PackageHandle,
        child: Arc<dyn IServiceProvider + Send + Sync>,
        parent: Arc<dyn IServiceProvider + Send + Sync>,
    ) -> Self { /* ... */ }
}

impl IServiceProvider for PackageContext {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        // child-first → parent fallback（rust-dix ServiceProviderWrapper 内置）
        self.child.get_service_any(type_id).or_else(|| self.parent.get_service_any(type_id))
    }
    // get_keyed_service_any / has_service_any 同上模式
}

impl IPackageContext for PackageContext {
    fn handle(&self) -> &PackageHandle { &self.handle }
    fn register_contribution(&self, host_id, contribution, options) {
        let cid = contribution.id().to_string();
        let registry: Arc<dyn IContributionRegistry> = 
            self.parent.get_required_trait::<dyn IContributionRegistry>();
        registry.register(host_id, contribution, options);
        self.contributions.write().unwrap().push((host_id.into(), cid));
    }
    fn register_service<T>(...) {
        // 调用 child 容器的 ServiceCollection API（需暴露子容器注册能力）
    }
}

impl PackageContext {
    /// unload 时由 PackageManager 调用 —— 自动 unregister 所有贡献点
    pub fn cleanup(&self) {
        let registry: Arc<dyn IContributionRegistry> = 
            self.parent.get_required_trait::<dyn IContributionRegistry>();
        for (host_id, cid) in self.contributions.read().unwrap().iter() {
            registry.unregister(host_id, cid);
        }
    }
}
```

#### `studio/core/src/app_context_ext.rs` （新增）

```rust
//! IAppContext 便捷扩展 —— 低认知负荷访问容器能力

use std::sync::Arc;
use gpui::App;
use rml_core::context::{IAppContext, IServiceProvider, ServiceProviderExt};
use crate::module::IModuleRegistry;
use crate::package::IPackageManager;
use rml_core::contribution::IContributionRegistry;

pub trait IAppContextExt: IAppContext {
    fn modules(&self) -> Arc<dyn IModuleRegistry> {
        self.get_required_trait::<dyn IModuleRegistry>()
    }
    fn packages(&self) -> Arc<dyn IPackageManager> {
        self.get_required_trait::<dyn IPackageManager>()
    }
    fn contributions(&self) -> Arc<dyn IContributionRegistry> {
        self.get_required_trait::<dyn IContributionRegistry>()
    }
}
impl<T: IAppContext + ?Sized> IAppContextExt for T {}
```

#### `studio/core/src/lib.rs` （更新导出）

* 新增 `pub mod module;` / `pub mod package;` / `pub mod app_context_ext;`

* 导出 `IModule`/`IModuleRegistry`/`ModuleRegistry`/`IPackage`/`IPackageContext`/`PackageHandle`/`PackageContext`/`IAppContextExt`

### 4.4 Studio Shell 新增插件管理器（studio/shell）

#### `studio/shell/src/package_manager.rs` （新增）

```rust
//! 插件管理器 —— 注册/加载/卸载 IPackage，管理子容器生命周期

use std::sync::{Arc, RwLock};
use gpui::App;
use rust_rml_di::ServiceProviderWrapper;
use rml_core::context::IServiceProvider;
use studio_core::package::*;

pub trait IPackageManager: Send + Sync {
    fn register(&self, factory: Box<dyn Fn(Arc<dyn IPackageContext>) -> Box<dyn IPackage> + Send + Sync>);
    fn load_all(&self, cx: &mut App);
    fn unload_all(&self, cx: &mut App);
    fn unload(&self, package_id: &str, cx: &mut App);
}

pub struct PackageManager {
    parent: RwLock<Option<Arc<dyn IServiceProvider + Send + Sync>>>,
    factories: RwLock<Vec<Box<dyn Fn(Arc<dyn IPackageContext>) -> Box<dyn IPackage> + Send + Sync>>>,
    loaded: RwLock<Vec<(PackageHandle, Arc<PackageContext>, Box<dyn IPackage>)>>,
}

impl PackageManager {
    pub fn new() -> Self { /* ... */ }
    /// 二阶段注入主容器（与 ArcShellManager::set_provider 同模式）
    pub fn set_parent(&self, parent: Arc<dyn IServiceProvider + Send + Sync>) { /* ... */ }
}

impl IPackageManager for PackageManager {
    fn load_all(&self, cx: &mut App) {
        let parent = self.parent.read().unwrap().clone().expect("...");
        for factory in self.factories.read().unwrap().iter() {
            // 1. 构造子容器 ServiceProviderWrapper（child-first + parent fallback）
            let child = Arc::new(ServiceProviderWrapper::new(parent.clone(), /* empty child collection */));
            // 2. 构造 PackageContext
            // 3. 调用 factory 构造 IPackage 实例
            // 4. 调用 package.load(cx)
            // 5. 记录到 loaded
        }
    }
    fn unload_all(&self, cx: &mut App) {
        for (handle, ctx, mut pkg) in self.loaded.write().unwrap().drain(..) {
            pkg.unload(cx);
            ctx.cleanup(); // 自动 unregister 贡献点
        }
    }
}
```

#### `studio/shell/src/di.rs` （更新）

* `build_runtime_provider` 中新增注册：

  * `s.add_singleton::<dyn IModuleRegistry>(|_| Arc::new(ModuleRegistry::new()));`

  * `s.add_singleton::<dyn IPackageManager>(move |_| Arc::new(PackageManager::new()));`

* 在 `manager.set_provider(provider)` 之后新增 `package_manager.set_parent(provider.clone())`（二阶段注入主容器）

* 应用 `apply_auto_registrations` 已有的 IModule/IPackage 工厂注册

#### `studio/shell/src/main_window.rml.rs` （更新）

* 在 `MainWindow::on_loaded` 中追加调用：

  * `cx.modules().loaded_all(cx)` —— 触发所有 IModule 的 loaded 钩子

  * （可选）`cx.packages().on_main_window_loaded(cx)` —— 通知插件主窗口已加载

### 4.5 业务模块迁移（studio/editor/explorer/chat）

#### `studio/editor/src/lib.rs` （迁移为 IModule）

```rust
pub struct EditorModule;

impl IModule for EditorModule {
    fn id(&self) -> &'static str { "editor" }
    fn order(&self) -> i32 { 100 }
    fn initialize(&mut self, _cx: &mut App) {
        // 保留原 #[ctor::ctor] 内的 auto_register 调用
        crate::editor_workbench::register_editor_abilities();
        auto_register(|s: &mut ServiceCollection| {
            s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", |_| {
                Arc::new(crate::editor_provider::EditorProvider) as Arc<dyn IWorkbenchProvider>
            });
        });
    }
}

// 替换原 #[rml_core::ctor::ctor] fn register_editor_services() { ... }
// 改为通过 IModuleRegistry 注册
#[rml_core::ctor::ctor]
fn register_editor_module() {
    auto_register(|s: &mut ServiceCollection| {
        // 注册 EditorModule 工厂到 IModuleRegistry
        // （或新增 auto_register_module 专用全局表）
    });
}
```

`studio/explorer/src/lib.rs`、`studio/chat/src/lib.rs` 同样模式迁移。

**注意**：保留 `auto_register` 注册 `IWorkbenchProvider` 的现有逻辑，仅将其包装进 `IModule::initialize` 内部调用，不破坏现有服务注册模式。

### 4.6 启动流程整合（studio/app + studio/shell）

#### `studio/app/src/main.rs` （更新）

```rust
RmlApplication::new()
    .main_window::<studio_shell::MainWindow>()
    .configure(|_s| {
        // 静态服务注册点（IModuleRegistry/IPackageManager 在 build_runtime_provider 中注册）
    })
    .run::<startup::Startup>();
```

#### 启动顺序（在 `RmlApplication::run` + `build_runtime_provider` + `MainWindow::open` 链路中）：

1. `bootstrap_runtime(cx)` —— 不变（注册 ContributionRegistry 到运行时表）
2. `MainWindow::default()` 创建 `Arc<ArcShellManager>` + `Arc<PackageManager>`
3. `build_runtime_provider(manager, package_manager)` —— 构造主容器，注册 IModuleRegistry + IPackageManager + ArcShellManager + 应用 auto\_registrations
4. `package_manager.set_parent(provider.clone())`
5. `cx.use_provider(provider)` —— 注入主容器
6. `Startup::on_launch(cx)` —— 设置样式/i18n/主题
7. **新增**：`cx.modules().initialize_all(cx)` —— 加载并初始化所有 IModule（按 order 排序）
8. **新增**：`cx.packages().load_all(cx)` —— 构造每个 IPackage + 注入 PackageContext + 调用 load
9. `MainWindow::open(cx)` —— 打开主窗口
10. `MainWindow::on_loaded` —— 触发 `cx.modules().loaded_all(cx)` + 通知插件

***

## 五、Assumptions & Decisions

### 假设

* A1：rust-dix 0.6 的 `ServiceProviderWrapper` 提供 child-first + root fallback 语义（基于官方文档描述，需在实现时验证 API 细节）

* A2：rust-dix `ServiceCollection::singleton::<T>(f)` 的 `T` 可以是 `ServiceSlot<dyn Trait>` 这种 Sized 包装类型（与现有 crates/di 模式一致），实现时若 API 不匹配则用 `singleton(f)` 无泛型版本 + factory 内部包装

* A3：`auto_register` 全局注册表模式向后兼容，IModule/IPackage 工厂注册沿用此模式（新增独立的 `auto_register_module`/`auto_register_package` 全局表或在 `auto_register` 闭包内调用 `IModuleRegistry::register`）

* A4：dll 动态加载在本计划不实现，但接口设计已预留（`IPackage`/`PackageHandle` 满足未来 extern "C" 入口需求）

* A5：现有 `#[ctor::ctor]` + `auto_register` 注册 `IWorkbenchProvider` 的逻辑保留，仅迁移包装位置（不破坏现有服务注册模式）

### 决策

* D1：RML `IServiceProvider` trait 不变，`crates/di` 内部切换 rust-dix，对外 API 不变

* D2：`IPackageContext` 持有 `ServiceProviderWrapper`（rust-dix）实现 child-first 桥接

* D3：`IModule` 静态注册（编译期已知），`IPackage` 当前静态注册，未来支持动态加载

* D4：`IPackage::load(cx: &mut App)` 透传 IAppContext，`IPackage` 实现通过 factory 注入持有 `IPackageContext`

* D5：`IPackageContext` 提供 `register_contribution`/`register_service` 便捷方法， unload 时自动清理

* D6：editor/explorer/chat 迁移为 IModule，shell 作为宿主不迁移

* D7：`IAppContextExt` 放在 `studio-core`（不污染 RML core 独立性）

***

## 六、验证步骤

### V1. 编译验证

* `cargo build -p rust-rml-di` —— DI 重构后编译通过

* `cargo build -p studio-core` —— 新增模块/插件接口编译通过

* `cargo build -p studio-shell` —— PackageManager 集成编译通过

* `cargo build -p studio-editor` / `studio-explorer` / `studio-chat` —— IModule 迁移编译通过

* `cargo build -p studio-app` —— 整体集成编译通过

### V2. 单元测试

* `cargo test -p rust-rml-di` —— 现有 DI 测试（singleton/keyed/cache/factory\_injection）全部通过

* `cargo test -p studio-core` —— 新增 ModuleRegistry/PackageContext 测试

  * `module_registry_sorts_by_order`

  * `package_context_child_first_resolution`

  * `package_context_register_contribution_records_for_cleanup`

  * `package_context_cleanup_unregisters_all`

### V3. 集成验证

* `cargo test -p rust-rml-engine` —— 1343 个 engine 测试通过（确保 DI 重构不破坏 RML 核心）

* `cargo test -p rust-rml-macros` —— 16 个 macros 测试通过

* `cargo run -p studio-app` —— 启动 studio 应用，验证：

  * 主窗口正常打开

  * 文件浏览器（ExplorerModule）正常加载

  * 编辑器（EditorModule）打开文件功能正常

  * 聊天面板（ChatModule）正常显示

  * 状态栏贡献点正常显示

### V4. 架构验证

* 确认 `crates/core` 不依赖 rust-dix（保持 RML 独立性）

* 确认 `studio/*` 不直接依赖 rust-dix（仅通过 `crates/di` 间接使用）

* 确认 `IServiceProvider` trait 未被修改（API 兼容）

* 确认 `RmlApplicationExt::configure` 链式 API 不变

* 确认 `auto_register` / `apply_auto_registrations` 不变

***

## 七、文件清单

### 新增

| 文件                                    | 职责                                                             |
| ------------------------------------- | -------------------------------------------------------------- |
| `crates/di/src/wrapper.rs`            | rust-dix ServiceProviderWrapper 薄包装，impl RML IServiceProvider  |
| `studio/core/src/module.rs`           | IModule + IModuleRegistry + ModuleRegistry 实现                  |
| `studio/core/src/package.rs`          | IPackage + IPackageContext + PackageHandle + PackageContext 实现 |
| `studio/core/src/app_context_ext.rs`  | IAppContextExt 便捷访问 trait                                      |
| `studio/shell/src/package_manager.rs` | IPackageManager + PackageManager 实现                            |

### 修改

| 文件                                    | 改动                                                 |
| ------------------------------------- | -------------------------------------------------- |
| `crates/di/Cargo.toml`                | 添加 rust-dix 依赖                                     |
| `crates/di/src/collection.rs`         | 重写为 rust-dix 薄包装                                   |
| `crates/di/src/provider.rs`           | 重写为 rust-dix ServiceProvider 包装                    |
| `crates/di/src/lib.rs`                | 导出 ServiceProviderWrapper                          |
| `studio/core/src/lib.rs`              | 导出 module/package/app\_context\_ext 模块             |
| `studio/shell/src/lib.rs`             | pub mod package\_manager                           |
| `studio/shell/src/di.rs`              | 注册 IModuleRegistry + IPackageManager + set\_parent |
| `studio/shell/src/main_window.rml.rs` | on\_loaded 中调用 modules().loaded\_all               |
| `studio/editor/src/lib.rs`            | 迁移为 EditorModule impl IModule                      |
| `studio/explorer/src/lib.rs`          | 迁移为 ExplorerModule impl IModule                    |
| `studio/chat/src/lib.rs`              | 迁移为 ChatModule impl IModule                        |

### 不变

* `crates/core/src/context.rs`（IServiceProvider/IAppContext 不变）

* `crates/core/src/contribution.rs`（IContribution/IContributionRegistry 不变）

* `crates/app/src/application.rs`（RmlApplication 不变）

* `crates/app/src/contribution/registry.rs`（ContributionRegistry 不变）

* `crates/di/src/auto_register.rs` / `configure.rs`（向后兼容）

***

## 八、实施顺序（建议）

1. **DI 重构**（V1+V4 验证）→ 确保现有功能不破坏
2. **studio/core 新增接口**（module.rs + package.rs + app\_context\_ext.rs）
3. **studio/shell 新增 PackageManager**（package\_manager.rs + di.rs 注册 + main\_window\.rml.rs 触发）
4. **业务模块迁移**（editor → explorer → chat，逐一迁移并验证）
5. **整体集成测试**（V3 全量验证）

