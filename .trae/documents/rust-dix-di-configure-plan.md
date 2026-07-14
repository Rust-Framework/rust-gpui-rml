# rust-dix DI 容器对接改进方案 v2 — ConfigureContainer 模式

## 摘要

将 RML 服务容器从"怪味"的 `run_with(before_launch)` 模式重构为 **ASP.NET Core `ConfigureContainer<TBuilder>` 风格**：

- `RmlApplication` 提供通用 key-value 存储（`get/set`）
- `rust-rml-di` 扩展 `RmlApplication`，提供 `configure::<T: IServiceRegister>()` 方法
- `IServiceRegister::build(s: &mut ServiceCollection)` 类型驱动配置
- `configure` 内部创建 collection → 自动注册 → `T::build` → build provider → `set` 进 RmlApplication
- `run` 时 `get` 取出 provider，`cx.use_provider` 注入
- `IAppContext` 作为 `IServiceProvider` 对外暴露
- **零污染**：不暴露 rust-dix 细节，无 "dix" 字眼

---

## 一、当前状态分析

### 已完成（上一轮工作）

| 文件 | 状态 |
|------|------|
| `crates/core/src/context.rs` | `IServiceProvider`（object-safe）+ `ServiceSlot<T>` + `ServiceProviderExt`（get_trait）+ `DefaultServiceProvider` + `IAppContext`（use_provider/set_service）+ `ServiceProviderSlot`（单数 provider + runtime 注册表） |
| `crates/app/src/application.rs` | `RmlApplication<W>::run_with(before_launch)` 已添加 |
| `crates/core/src/lib.rs` + `prelude.rs` | 导出 `IServiceProvider, ServiceSlot, ServiceProviderExt, IAppContext, DefaultServiceProvider` |

### 未完成 / 有问题

| 文件 | 问题 |
|------|------|
| `crates/app/src/extensions.rs:25,35` | **编译错误**：`IAppContextExt::get_contribution_registry` 调用 `get_required_service`（`where Self: Sized`），但 trait 方法无此约束 |
| `studio/shell/src/di.rs` | 直接 `use rust_dix::{ServiceCollection, ServiceProvider}`，未抽象 |
| `studio/shell/src/shell_manager.rs:16,25,51,73,103` | `OnceLock<Arc<rust_dix::ServiceProvider>>` + `provider.get_keyed::<dyn T>(schema)` 直接依赖 rust-dix |
| `studio/shell/src/main_window.rml.rs:20,90` | `use rust_dix::ServiceProvider` + `cx.set_service::<ServiceProvider>(provider)` 双层解析模式 |
| `studio/explorer/src/explorer_panel.rml.rs:21,71,85` | `cx.get_service::<ServiceProvider>().and_then(|p| p.get::<dyn T>())` 双层解析 |
| `studio/shell/Cargo.toml` + `explorer/Cargo.toml` + `editor/Cargo.toml` | 直接依赖 `rust-dix` |
| `crates/di/` | 不存在，需创建 `rust-rml-di` crate |

### 核心设计矛盾（rust-dix API 兼容性）

经查验 rust-dix 0.6 源码（`~/.cargo/registry/src/index.crates.io-*/rust-dix-0.6.0/`）：

1. **factory 签名不兼容**：rust-dix 用 `Fn(&dyn IServiceResolver) -> Arc<T>`，RML 抽象需要 `Fn(&dyn IServiceProvider) -> Arc<T>`
2. **存储格式不兼容**：rust-dix 内部存储 `Arc<Arc<T>>` as `Arc<dyn Any>`（见 `collection.rs::push` 的 `Arc::new(val)`），RML 期望 `Arc<T>` as `Arc<dyn Any>`（直接 downcast）
3. **`IServiceResolver::get_by_type_id` 返回双层 Arc**：虽支持 TypeId 查询，但返回值需 `extract` 解包，不能直接委托给 `IServiceProvider::get_service_any`

**结论**：rust-rml-di 采用**自维护 factory map + cache** 策略实现完整 DI 支持，不直接使用 rust-dix 的 factory 机制。rust-dix crate 作为可选依赖保留（未来 API 演进后可切换后端）。此方案能完整支持 `Fn(&dyn IServiceProvider)` 签名 + factory 内依赖注入 + TypeId 运行时查询。

---

## 二、架构设计

### 2.1 整体分层（零污染）

```
┌─────────────────────────────────────────────────────────────┐
│ rml_core: IServiceProvider (解析抽象) + IAppContext (注入)  │
│   + ServiceSlot<T> + ServiceProviderExt (get_trait)         │
│   不依赖任何 DI 实现                                         │
├─────────────────────────────────────────────────────────────┤
│ rml_app: RmlApplication (get/set 键值对存储 + run 调度)     │
│   IAppLifecycle (on_launch 内 cx 可解析服务)                │
│   不依赖 rust-rml-di                                         │
├─────────────────────────────────────────────────────────────┤
│ rust-rml-di: ServiceCollection + ServiceProvider            │
│   + IServiceRegister trait + RmlApplicationExt::configure   │
│   + auto_register (ctor 全局注册表)                         │
│   依赖 rml_core + rml_app                                    │
├─────────────────────────────────────────────────────────────┤
│ 应用层: AppServiceRegister impl IServiceRegister            │
│   configure::<AppServiceRegister>() 注册服务                │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 RmlApplication 通用 key-value 存储（rml_app）

`RmlApplication<W>` 添加 `properties` 字段，提供 `get/set`：

```rust
pub struct RmlApplication<W = NoWindow> {
    _window: PhantomData<W>,
    properties: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl<W> RmlApplication<W> {
    pub fn set<T: 'static + Send + Sync>(&mut self, value: T) {
        self.properties.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.properties.get(&TypeId::of::<T>())?.downcast_ref::<T>()
    }

    /// 取出（移动）属性，run 阶段用于取出 provider
    fn take<T: 'static + Send + Sync>(&mut self) -> Option<T> {
        self.properties.remove(&TypeId::of::<T>())?
            .downcast::<T>().ok().map(|b| *b)
    }
}
```

`run::<L>()` 内部：

```rust
pub fn run<L: IAppLifecycle + 'static>(mut self) {
    let provider = self.take::<Arc<dyn IServiceProvider>>();  // 取出 configure 注册的 provider
    gpui_platform::application()
        .with_assets(crate::assets::CompositeAssets)
        .run(move |cx: &mut App| {
            bootstrap_runtime(cx);
            if let Some(p) = provider {
                cx.use_provider(p);  // 注入为正式后端
            }
            L::default().on_launch(cx);  // cx 可解析所有注册的服务
            W::default().open(cx);
        });
}
```

### 2.3 ServiceProviderSlot 改为 provider 链（rml_core）

当前 `ServiceProviderSlot` 持有单数 `provider: RwLock<Arc<dyn IServiceProvider>>`，`use_provider` 覆盖。改为 **provider 链（Vec）+ runtime 注册表**，支持多阶段注入：

```rust
struct ServiceProviderSlot {
    /// provider 链：configure 阶段 + on_loaded 阶段注入的 provider 依次查询
    providers: RwLock<Vec<Arc<dyn IServiceProvider>>>,
    /// 运行时注册表（set_service 写入，任何 provider 链下都生效）
    runtime: DefaultServiceProvider,
}

impl ServiceProviderSlot {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        for p in self.providers.read().unwrap().iter() {
            if let Some(svc) = p.get_service_any(type_id) {
                return Some(svc);
            }
        }
        self.runtime.get_service_any(type_id)
    }

    fn use_provider(&self, provider: Arc<dyn IServiceProvider>) {
        self.providers.write().unwrap().push(provider);  // 追加，不覆盖
    }
    // ... keyed / has 同理
}
```

**意义**：studio 的 `ArcShellManager` 有循环依赖（manager 需要 provider 解析 IWorkbenchProvider，provider 需要 manager 已注册），需在 `on_loaded` 阶段二阶段注入。provider 链使 `configure` 阶段的静态服务 provider 与 `on_loaded` 阶段的运行时 provider 共存，查询时依次尝试。

### 2.4 rust-rml-di crate 结构

```
crates/di/
├── Cargo.toml          # package = "rust-rml-di", deps: rust-rml-core, rust-rml-app, ctor, log
├── src/
│   ├── lib.rs          # 模块声明 + re-exports
│   ├── collection.rs   # ServiceCollection (自维护 factory map)
│   ├── provider.rs     # ServiceProvider (impl IServiceProvider, 自维护 cache)
│   ├── configure.rs    # IServiceRegister trait + RmlApplicationExt::configure
│   ├── auto_register.rs# 全局自动注册表 + apply_auto_registrations
│   └── prelude.rs      # re-exports
```

**注意**：不依赖 `rust-dix` crate（API 不兼容，自维护更简洁）。crate 名 `rust-rml-di` 表示"RML 的 DI 适配层"。

### 2.5 ServiceCollection — 注册容器（自维护 factory map）

```rust
type FactoryFn = Box<dyn Fn(&dyn IServiceProvider) -> Arc<dyn Any + Send + Sync> + Send + Sync>;

pub struct ServiceCollection {
    factories: HashMap<TypeId, FactoryFn>,
    keyed_factories: HashMap<(TypeId, String), FactoryFn>,
}

impl ServiceCollection {
    pub fn new() -> Self { ... }

    /// 注册单例（trait object 经 ServiceSlot 桥接）
    pub fn add_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let f = Box::new(move |p: &dyn IServiceProvider| {
            let arc_t: Arc<T> = factory(p);
            Arc::new(ServiceSlot(arc_t)) as Arc<dyn Any + Send + Sync>
        });
        self.factories.insert(TypeId::of::<ServiceSlot<T>>(), f);
    }

    /// 注册 keyed 单例
    pub fn add_keyed_singleton<T: ?Sized + 'static + Send + Sync>(
        &mut self,
        key: &str,
        factory: impl Fn(&dyn IServiceProvider) -> Arc<T> + Send + Sync + 'static,
    ) {
        let f = Box::new(move |p: &dyn IServiceProvider| {
            Arc::new(ServiceSlot(factory(p))) as Arc<dyn Any + Send + Sync>
        });
        self.keyed_factories.insert((TypeId::of::<ServiceSlot<T>>(), key.to_string()), f);
    }

    /// 构建 ServiceProvider
    pub fn build(self) -> Arc<dyn IServiceProvider> {
        Arc::new(ServiceProvider {
            factories: self.factories,
            keyed_factories: self.keyed_factories,
            cache: RwLock::new(HashMap::new()),
            keyed_cache: RwLock::new(HashMap::new()),
        })
    }
}
```

### 2.6 ServiceProvider — 解析容器（impl IServiceProvider）

```rust
pub struct ServiceProvider {
    factories: HashMap<TypeId, FactoryFn>,
    keyed_factories: HashMap<(TypeId, String), FactoryFn>,
    cache: RwLock<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    keyed_cache: RwLock<HashMap<(TypeId, String), Arc<dyn Any + Send + Sync>>>,
}

impl IServiceProvider for ServiceProvider {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        // 先查缓存（单例语义）
        if let Some(cached) = self.cache.read().unwrap().get(&type_id) {
            return Some(cached.clone());
        }
        // 调用 factory，传入 self as &dyn IServiceProvider（支持 factory 内 DI）
        let factory = self.factories.get(&type_id)?;
        let instance = factory(self);
        self.cache.write().unwrap().insert(type_id, instance.clone());
        Some(instance)
    }

    fn get_keyed_service_any(&self, type_id: TypeId, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        let cache_key = (type_id, key.to_string());
        if let Some(cached) = self.keyed_cache.read().unwrap().get(&cache_key) {
            return Some(cached.clone());
        }
        let factory = self.keyed_factories.get(&cache_key)?;
        let instance = factory(self);
        self.keyed_cache.write().unwrap().insert(cache_key, instance.clone());
        Some(instance)
    }

    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.factories.contains_key(&type_id)
    }
}
```

**factory 内依赖注入**：`factory(self)` 中 `self` 是 `&ServiceProvider`，可转 `&dyn IServiceProvider`。factory 内 `p.get_trait::<dyn IOtherService>()` 经 `ServiceProviderExt` blanket impl 查询其他服务（递归解析 + 缓存）。

### 2.7 IServiceRegister + RmlApplicationExt（configure 模式）

```rust
/// 服务注册器 —— 类型驱动配置（类比 ASP.NET Core ConfigureServices）
pub trait IServiceRegister {
    fn build(s: &mut ServiceCollection);
}

/// RmlApplication 扩展 —— configure 链式 API
pub trait RmlApplicationExt<W>: Sized {
    fn configure<T: IServiceRegister>(self) -> Self;
}

impl<W> RmlApplicationExt<W> for RmlApplication<W> {
    fn configure<T: IServiceRegister>(mut self) -> Self {
        let mut collection = ServiceCollection::new();
        apply_auto_registrations(&mut collection);  // 应用 ctor 自动注册
        T::build(&mut collection);                   // 用户配置
        let provider = collection.build();
        self.set::<Arc<dyn IServiceProvider>>(provider);  // 存入 properties
        self
    }
}
```

**使用方式**：

```rust
struct AppServiceRegister;
impl IServiceRegister for AppServiceRegister {
    fn build(s: &mut ServiceCollection) {
        s.add_singleton::<dyn IWorkspaceManager>(|| Arc::new(ArcShellManager::new()));
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("rml", || Arc::new(WelcomeProvider));
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", || Arc::new(EditorProvider));
    }
}

fn main() {
    RmlApplication::new()
        .main_window::<MainWindow>()
        .configure::<AppServiceRegister>()
        .run::<Startup>();
}
```

### 2.8 自动注册机制（ctor + 全局注册表）

```rust
// rust-rml-di/src/auto_register.rs
type RegisterFn = Box<dyn Fn(&mut ServiceCollection) + Send + Sync>;

static AUTO_REGISTRATIONS: OnceLock<Mutex<Vec<RegisterFn>>> = OnceLock::new();

pub fn auto_register(f: impl Fn(&mut ServiceCollection) + Send + Sync + 'static) {
    AUTO_REGISTRATIONS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock().unwrap().push(Box::new(f));
}

pub fn apply_auto_registrations(collection: &mut ServiceCollection) {
    if let Some(registry) = AUTO_REGISTRATIONS.get() {
        for f in registry.lock().unwrap().iter() {
            f(collection);
        }
    }
}
```

**用户 crate 自动注册**：

```rust
// studio-editor/src/lib.rs
#[ctor::ctor]
fn register_editor() {
    rust_rml_di::auto_register(|s| {
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", || Arc::new(EditorProvider));
    });
}
```

`Fn`（非 FnOnce）+ 非 drain 式读取，支持多次 build。

---

## 三、实施步骤

### Step 1: 修复 IAppContextExt 编译错误

**文件**: `crates/app/src/extensions.rs`

为 `get_contribution_registry` 和 `register_host` 添加 `where Self: Sized` 约束（因调用 `get_required_service` 需要 Sized）：

```rust
pub trait IAppContextExt: IAppContext {
    fn get_contribution_registry(&self) -> Arc<dyn IContributionRegistry>
    where Self: Sized
    {
        self.get_required_service::<ContributionRegistry>() as Arc<dyn IContributionRegistry>
    }

    fn register_host(&self, host_id: &str, host: Arc<dyn IContributionHost>)
    where Self: Sized
    {
        self.get_contribution_registry().add(host_id, host);
    }
}
```

**验证**: `cargo check -p rust-rml-app`

### Step 2: RmlApplication 添加 properties 存储

**文件**: `crates/app/src/application.rs`

1. `RmlApplication<W>` 添加字段 `properties: HashMap<TypeId, Box<dyn Any + Send + Sync>>`
2. 实现 `set<T>()` / `get<T>()` / `take<T>()` 方法
3. `new()` 初始化 `properties: HashMap::new()`
4. `main_window()` 转移 properties
5. `run::<L>()` 内部 `take::<Arc<dyn IServiceProvider>>()` 取出 provider，`cx.use_provider(provider)` 注入
6. **移除** `run_with(before_launch)` 方法（被 configure 模式取代）

**验证**: `cargo check -p rust-rml-app`

### Step 3: ServiceProviderSlot 改为 provider 链

**文件**: `crates/core/src/context.rs`

1. `ServiceProviderSlot.provider: RwLock<Arc<dyn IServiceProvider>>` → `providers: RwLock<Vec<Arc<dyn IServiceProvider>>>`
2. `use_provider` 改为 `push`（追加，不覆盖）
3. `get_service_any` / `get_keyed_service_any` / `has_service_any` 遍历 provider 链
4. `new()` 初始化 `providers: Vec::new()`（空链，仅 runtime 注册表）

**验证**: `cargo check -p rust-rml-core`

### Step 4: 创建 rust-rml-di crate

**新建文件**:

1. `crates/di/Cargo.toml`:
```toml
[package]
name = "rust-rml-di"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "RML DI 适配层 —— ServiceCollection + ServiceProvider + configure 模式"

[dependencies]
rust-rml-core = { workspace = true }
rust-rml-app = { workspace = true }
ctor = "0.2"
log = { workspace = true }
```

2. `crates/di/src/lib.rs` — 模块声明 + re-exports
3. `crates/di/src/collection.rs` — `ServiceCollection`（自维护 factory map）
4. `crates/di/src/provider.rs` — `ServiceProvider`（impl IServiceProvider + cache）
5. `crates/di/src/configure.rs` — `IServiceRegister` trait + `RmlApplicationExt::configure`
6. `crates/di/src/auto_register.rs` — 全局注册表 + `auto_register()` + `apply_auto_registrations()`
7. `crates/di/src/prelude.rs` — re-exports

**注册到 workspace**: `Cargo.toml` 的 `members` 添加 `"crates/di"`，`[workspace.dependencies]` 添加 `rust-rml-di = { path = "crates/di" }`

**验证**: `cargo check -p rust-rml-di`

### Step 5: Studio Shell 迁移

#### 5a. `studio/shell/Cargo.toml`

```toml
# 替换
rust-rml-di = { workspace = true }
# 删除 rust-dix = { workspace = true }
```

#### 5b. `studio/shell/src/di.rs` — 重写

```rust
use std::sync::Arc;
use rml_core::context::IServiceProvider;
use rml_core::workbench::{IWorkbenchManager, IWorkbenchProvider};
use rust_rml_di::ServiceCollection;
use studio_core::workspace::IWorkspaceManager;
use crate::shell_manager::ArcShellManager;
use crate::welcome::WelcomeProvider;
use studio_editor::editor_provider::EditorProvider;

/// 构建运行时 provider（ArcShellManager 二阶段注入）
pub fn build_runtime_provider(manager: Arc<ArcShellManager>) -> Arc<dyn IServiceProvider> {
    let mut s = ServiceCollection::new();
    s.add_singleton::<dyn IWorkspaceManager>(move || manager.clone() as Arc<dyn IWorkspaceManager>);
    s.add_singleton::<dyn IWorkbenchManager>(move || manager.clone() as Arc<dyn IWorkbenchManager>);
    s.add_keyed_singleton::<dyn IWorkbenchProvider>("rml", || Arc::new(WelcomeProvider) as Arc<dyn IWorkbenchProvider>);
    s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", || Arc::new(EditorProvider) as Arc<dyn IWorkbenchProvider>);
    let provider = s.build();
    manager.set_provider(provider.clone());
    provider
}
```

#### 5c. `studio/shell/src/shell_manager.rs` — 解耦 rust-dix

```rust
// 替换 use rust_dix::ServiceProvider;
use rml_core::context::{IServiceProvider, ServiceProviderExt};

// 字段类型替换
provider: OnceLock<Arc<dyn IServiceProvider>>,

// set_provider 签名替换
pub fn set_provider(&self, provider: Arc<dyn IServiceProvider>) { ... }

// provider() 返回类型替换
fn provider(&self) -> &Arc<dyn IServiceProvider> { ... }

// IWorkbenchManager::open 中的查询替换
// 原: self.provider().get_keyed::<dyn IWorkbenchProvider>(schema).ok()?
// 新: self.provider().get_keyed_trait::<dyn IWorkbenchProvider>(schema)?
```

#### 5d. `studio/shell/src/main_window.rml.rs` — 切换到 use_provider

```rust
// 删除 use rust_dix::ServiceProvider;

// on_loaded 中：
// 原: cx.set_service::<ServiceProvider>(provider);
// 新: cx.use_provider(provider);  // 追加到 provider 链（configure 阶段的静态服务仍可解析）
```

**验证**: `cargo check -p studio-shell`

### Step 6: Studio Explorer + Editor 迁移

#### 6a. `studio/explorer/Cargo.toml` + `studio/editor/Cargo.toml`

```toml
# 删除 rust-dix = { workspace = true }
# explorer/editor 不需要 rust-rml-di（仅消费端，用 rml_core 的 ServiceProviderExt）
```

#### 6b. `studio/explorer/src/explorer_panel.rml.rs` — 简化查询

```rust
// 删除 use rust_dix::ServiceProvider;
use rml_core::context::ServiceProviderExt;

// refresh_tree 中：
// 原: cx.get_service::<ServiceProvider>().and_then(|p| p.get::<dyn IWorkspaceManager>().ok()).map(...)
// 新: cx.get_trait::<dyn IWorkspaceManager>().map(|mgr| mgr.list()).unwrap_or_default()

// on_file_activate 中：
// 原: let Some(provider) = cx.get_service::<ServiceProvider>() else { return; };
//     let Ok(workspace_mgr) = provider.get::<dyn IWorkspaceManager>() else { ... };
//     let Ok(workbench_mgr) = provider.get::<dyn IWorkbenchManager>() else { ... };
// 新: let Some(workspace_mgr) = cx.get_trait::<dyn IWorkspaceManager>() else { return; };
//     let Some(workbench_mgr) = cx.get_trait::<dyn IWorkbenchManager>() else { return; };
```

#### 6c. `studio/editor/src/editor_provider.rs`

无需修改（EditorProvider 本身不查询 DI，仅被注册）。

**验证**: `cargo check -p studio-explorer -p studio-editor`

### Step 7: Studio App 入口迁移

**文件**: `studio/app/Cargo.toml` — 添加 `rust-rml-di = { workspace = true }`

**文件**: `studio/app/src/main.rs`:

```rust
use rust_rml_di::prelude::*;
use rust_rml_di::IServiceRegister;
use rust_rml_di::ServiceCollection;
use rml_core::workbench::IWorkbenchProvider;
use studio_core::workspace::IWorkspaceManager;
// ... (ArcShellManager 等运行时服务不在 configure 注册，因循环依赖)

/// 静态服务注册器（无循环依赖的服务）
struct AppServiceRegister;
impl IServiceRegister for AppServiceRegister {
    fn build(s: &mut ServiceCollection) {
        // 静态服务可在此注册（WelcomeProvider/EditorProvider 也可移入 ctor 自动注册）
        // ArcShellManager 因循环依赖，在 MainWindow::on_loaded → build_runtime_provider 中注册
    }
}

fn main() {
    RmlApplication::new()
        .main_window::<studio_shell::MainWindow>()
        .configure::<AppServiceRegister>()  // 注入静态服务 provider
        .run::<Startup>();
}
```

**注**：`ArcShellManager` 的循环依赖（manager 需要 provider 解析 IWorkbenchProvider，provider 需要 manager 已注册）在 `MainWindow::on_loaded` → `build_runtime_provider(manager)` → `cx.use_provider(provider)` 中处理。provider 链保证 configure 阶段的静态服务仍可解析。

**可选增强**：`studio-editor/src/lib.rs` 添加 `#[ctor::ctor]` 自动注册 EditorProvider，`studio-shell/src/lib.rs` 自动注册 WelcomeProvider，则 `AppServiceRegister::build` 可为空。

**验证**: `cargo check -p arc-studio`

### Step 8: 全量构建验证

```powershell
cargo check --workspace
cargo build --workspace
cargo test -p rust-rml-core --lib
cargo test -p rust-rml-di --lib
```

确认 demo 不受影响（demo 不依赖 rust-dix，使用 `cx.set_service`/`cx.get_service::<ConcreteType>()` 模式，与新抽象完全兼容）。

---

## 四、假设与决策

### 假设

1. `rust_dix::ServiceProvider` 的 `IServiceResolver::get_by_type_id` 返回双层 Arc（`Arc<Arc<T>>` as `Arc<dyn Any>`），与 RML 单层 Arc 不兼容 — 已查验源码确认
2. rust-dix 的 factory 签名 `Fn(&dyn IServiceResolver)` 与 RML 抽象 `Fn(&dyn IServiceProvider)` 不一致 — 已查验源码确认
3. Demo 不使用 rust-dix，无需迁移 — 已验证 demo Cargo.toml
4. `ArcShellManager` 的循环依赖需二阶段注入，provider 链设计支持此模式
5. `ctor` crate 已在 rml_core 依赖中，rust-rml-di 可直接使用

### 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 抽象范围 | `IServiceProvider`（解析）+ `IServiceCollection`（注册，rust-rml-di 内部）| core 零污染，注册抽象在适配层 |
| factory 签名 | `Fn(&dyn IServiceProvider) -> Arc<T>` | 用户选择，支持 factory 内 DI |
| rust-dix 对接 | 自维护 factory map + cache（不直接用 rust-dix factory 机制）| API 不兼容（双层 Arc + IServiceResolver 签名），自维护更简洁 |
| provider 链 | `Vec<Arc<dyn IServiceProvider>>` + runtime 注册表 | 支持 configure + on_loaded 多阶段注入，解决循环依赖 |
| configure API | `configure::<T: IServiceRegister>()` 类型驱动 | 用户指定，职责清晰 |
| RmlApplication 存储 | `HashMap<TypeId, Box<dyn Any>>` get/set | 用户指定，零污染 |
| 自动注册 | `ctor` + 全局 `Fn` 闭包表 | 与项目现有 `ctor` 模式一致 |
| crate 命名 | `rust-rml-di`（无 dix 字眼）| 用户要求零污染 |

### 非目标

- 不直接使用 rust-dix 的 `ServiceCollection`/`ServiceProvider`（API 不兼容）
- 不添加 proc-macro 自动注册（`#[service]` 属性宏）— 未来增强
- 不迁移 demo — demo 不使用 rust-dix
- 不实现 `IServiceCollection` trait 抽象（`ServiceCollection` 是 rust-rml-di 具体类型，core 不感知）

---

## 五、验证清单

- [ ] `cargo check -p rust-rml-core` — ServiceProviderSlot provider 链编译
- [ ] `cargo check -p rust-rml-app` — RmlApplication properties + configure 编译
- [ ] `cargo check -p rust-rml-di` — 适配 crate 编译
- [ ] `cargo check -p studio-shell` — shell_manager/di/main_window 迁移
- [ ] `cargo check -p studio-explorer -p studio-editor` — 查询简化
- [ ] `cargo check -p arc-studio` — 入口 configure 链式
- [ ] `cargo build --workspace` — 全量编译
- [ ] `cargo test -p rust-rml-core --lib` — 核心测试不回归
- [ ] 确认 demo 无需修改即编译通过
- [ ] 验证 factory 内依赖注入（`p.get_trait::<dyn T>()` 递归解析）
- [ ] 验证 provider 链多阶段注入（configure + on_loaded 共存）
