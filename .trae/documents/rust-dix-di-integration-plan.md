# rust-dix DI 容器对接改进方案

## 摘要

将 RML 核心库的服务容器从直接耦合 `rust-dix` 改为 **ASP.NET Core 风格抽象**：核心层定义 `IServiceProvider` 解析接口，独立适配 crate `rust-rml-di` 对接 rust-dix 实现层。引入 `ServiceSlot<T>` 模式解决 trait object 在类型擦除层的查询问题，提供 `DiRmlApplication` 链式 API + `ctor` 自动注册机制。

---

## 一、当前状态分析

### 已完成（Task 1-2）

- `crates/core/src/context.rs`：已重写为 `IServiceProvider`（object-safe，类型擦除 `_any` 方法 + `where Self: Sized` 泛型便利方法）+ `DefaultServiceProvider` + `ServiceProviderSlot`（双层查询：正式后端 → 运行时注册表）+ `IAppContext`
- `crates/core/src/lib.rs` + `prelude.rs`：导出已更新为 `ensure_service_provider, DefaultServiceProvider, IAppContext, IServiceProvider`

### 未完成

| 文件 | 问题 |
|------|------|
| `crates/app/src/application.rs` | `RmlApplication<W>`（窗口变体）缺 `run_with` 方法 |
| `crates/app/src/extensions.rs:45` | **编译错误**：`pub use rml_core::context::{ensure_service_collection, ServiceCollection}` — 旧名称已不存在 |
| `crates/app/src/lib.rs:28` | **编译错误**：`pub use extensions::{..., ServiceCollection, ensure_service_collection}` — 旧名称 |
| `crates/di/` | 不存在，需创建 `rust-rml-di` crate |
| `studio/shell/src/di.rs` | 直接使用 `rust_dix::ServiceCollection` 构建，返回 `Arc<rust_dix::ServiceProvider>` |
| `studio/shell/src/main_window.rml.rs:90` | `cx.set_service::<ServiceProvider>(provider)` — 两层解析模式 |
| `studio/shell/src/shell_manager.rs` | `OnceLock<Arc<rust_dix::ServiceProvider>>` — 直接依赖 rust-dix 类型 |
| `studio/explorer/src/explorer_panel.rml.rs:71,85` | `cx.get_service::<ServiceProvider>().and_then(|p| p.get::<dyn T>())` — 两层解析 |
| `studio/shell/Cargo.toml` | 依赖 `rust-dix` 而非 `rust-rml-di` |

### 核心挑战：Trait Object 查询

`IServiceProvider::get_service_any(TypeId) -> Option<Arc<dyn Any + Send + Sync>>` 通过 `Arc<dyn Any>::downcast::<T>()` 还原类型，但 `downcast` 要求 `T: Sized`。因此 `get_service::<dyn IWorkspaceManager>()` **无法工作**（`dyn Trait` 是 `!Sized`）。

当前 studio 代码大量使用 `provider.get::<dyn IWorkspaceManager>()` 查询 trait object，需要解决方案。

---

## 二、架构设计

### 2.1 ServiceSlot<T> — Trait Object 查询桥接

在 `rml_core::context` 中引入 `ServiceSlot<T: ?Sized>` 包装类型：

```rust
/// Trait object 服务槽位 —— 将 `Arc<dyn Trait>` 包装为 Sized 类型，
/// 使其可通过 `IServiceProvider` 的类型擦除层注册/查询。
pub struct ServiceSlot<T: ?Sized + 'static>(pub Arc<T>);
```

`ServiceSlot<dyn IWorkspaceManager>` 是 **Sized** 结构体（包裹 `Arc<dyn IWorkspaceManager>`），可存入 `Arc<dyn Any + Send + Sync>` 并 `downcast` 还原。

**查询模式**：
- 具体类型：`cx.get_service::<MyStruct>()` — 直接查询（与现有方式一致）
- Trait object：`cx.get_service::<ServiceSlot<dyn ITrait>>()?.0` — 经 ServiceSlot 查询

### 2.2 ServiceProviderExt — 便捷扩展方法

在 `rml_core::context` 中引入扩展 trait，提供 trait object 查询的便捷方法：

```rust
pub trait ServiceProviderExt: IServiceProvider {
    /// 查询 trait object 服务（经 ServiceSlot 桥接）
    fn get_trait<T: ?Sized + 'static + Send + Sync>(&self) -> Option<Arc<T>> {
        self.get_service_any(TypeId::of::<ServiceSlot<T>>())
            .and_then(|any| any.downcast::<ServiceSlot<T>>().ok())
            .map(|slot| slot.0)
    }
    /// 查询 keyed trait object 服务
    fn get_keyed_trait<T: ?Sized + 'static + Send + Sync>(&self, key: &str) -> Option<Arc<T>> {
        self.get_keyed_service_any(TypeId::of::<ServiceSlot<T>>(), key)
            .and_then(|any| any.downcast::<ServiceSlot<T>>().ok())
            .map(|slot| slot.0)
    }
}
// blanket impl —— 对所有 IServiceProvider 实现（含 dyn IServiceProvider）
impl<S: IServiceProvider + ?Sized> ServiceProviderExt for S {}
```

业务代码：`cx.get_trait::<dyn IWorkspaceManager>()` 或 `provider.get_keyed_trait::<dyn IWorkbenchProvider>("rml")`。

### 2.3 rust-rml-di Crate 结构

```
crates/di/
├── Cargo.toml              # package = "rust-rml-di", deps: rust-rml-core, rust-rml-app, rust-dix, anyhow, ctor, log
├── src/
│   ├── lib.rs              # 模块声明 + re-exports
│   ├── collection.rs       # ServiceCollection（builder，包装 rust_dix::ServiceCollection + resolver map）
│   ├── provider.rs         # DixServiceProvider（impl IServiceProvider，包装 rust_dix::ServiceProvider）
│   ├── application.rs      # DiRmlApplication<W> + RmlApplicationExt（use_dix 链式 API）
│   ├── auto_register.rs    # 全局自动注册表 + apply_auto_registrations
│   └── prelude.rs          # re-exports
```

### 2.4 ServiceCollection — 适配器 Builder

```rust
pub struct ServiceCollection {
    inner: rust_dix::ServiceCollection,
    resolvers: HashMap<TypeId, ResolverFn>,
    keyed_resolvers: HashMap<(TypeId, String), ResolverFn>,
}

type ResolverFn = Box<dyn Fn(&rust_dix::ServiceProvider) -> Option<Arc<dyn Any + Send + Sync>> + Send + Sync>;
```

**两组注册方法**（通过 `Sized` 约束区分）：

| 方法 | 约束 | Resolver Key | 返回方式 |
|------|------|-------------|---------|
| `singleton::<T>(factory)` | `T: Sized` | `TypeId::of::<T>()` | `Arc<T>` 直接作为 `Arc<dyn Any>` |
| `singleton_trait::<T>(factory)` | `T: ?Sized` | `TypeId::of::<ServiceSlot<T>>()` | `Arc<ServiceSlot<T>>` 作为 `Arc<dyn Any>` |
| `keyed_singleton::<T>(key, factory)` | `T: Sized` | `(TypeId::of::<T>(), key)` | 同上 |
| `keyed_singleton_trait::<T>(key, factory)` | `T: ?Sized` | `(TypeId::of::<ServiceSlot<T>>(), key)` | 同上 |

Resolver 闭包内部调用 `rust_dix::ServiceProvider::get::<T>()` 委托给 rust-dix 的完整 DI 能力（构造注入、工厂等）。

### 2.5 DixServiceProvider — 适配器 Provider

```rust
pub struct DixServiceProvider {
    inner: Arc<rust_dix::ServiceProvider>,  // rust-dix build() 返回 Arc<ServiceProvider>
    resolvers: HashMap<TypeId, ResolverFn>,
    keyed_resolvers: HashMap<(TypeId, String), ResolverFn>,
}

impl IServiceProvider for DixServiceProvider {
    fn get_service_any(&self, type_id: TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.resolvers.get(&type_id).and_then(|f| (f)(&self.inner))
    }
    fn get_keyed_service_any(&self, type_id: TypeId, key: &str) -> Option<Arc<dyn Any + Send + Sync>> {
        self.keyed_resolvers.get(&(type_id, key.to_string())).and_then(|f| (f)(&self.inner))
    }
    fn has_service_any(&self, type_id: TypeId) -> bool {
        self.resolvers.contains_key(&type_id)
    }
}
```

### 2.6 DiRmlApplication — 链式 API

```rust
pub struct DiRmlApplication<W = NoWindow> {
    inner: RmlApplication<W>,
    configurations: Vec<Box<dyn FnOnce(&mut ServiceCollection) + 'static>>,
}

// 扩展 trait（在 rust-rml-di 中定义，为 RmlApplication 添加 use_dix 方法）
pub trait RmlApplicationExt<W> {
    fn use_dix(self) -> DiRmlApplication<W>;
}
impl<W> RmlApplicationExt<W> for RmlApplication<W> {
    fn use_dix(self) -> DiRmlApplication<W> {
        DiRmlApplication { inner: self, configurations: Vec::new() }
    }
}
```

**使用方式**：
```rust
RmlApplication::new()
    .main_window::<MainWindow>()
    .use_dix()                                      // → DiRmlApplication<MainWindow>
    .singleton_trait::<dyn IWorkspaceManager>(...)  // 链式注册
    .keyed_singleton_trait::<dyn IWorkbenchProvider>("rml", ...)
    .configure(|c| { /* 复杂注册逻辑 */ })           // 闭包式注册
    .run::<Startup>();
```

`run::<L>()` 内部：
1. 创建 `ServiceCollection`
2. `apply_auto_registrations(&mut collection)` — 应用 `ctor` 自动注册
3. 依次执行 `configurations` 闭包 — 用户注册
4. `collection.build()` → `DixServiceProvider`
5. `cx.use_provider(Arc::new(provider))` — 注入为正式后端

### 2.7 自动注册机制

基于 `ctor` crate + 全局 `Mutex<Vec<(&'static str, Box<dyn Fn(&mut ServiceCollection) + Send + Sync>)>>`：

```rust
// rust-rml-di 提供：
pub fn auto_register(name: &'static str, f: impl Fn(&mut ServiceCollection) + Send + Sync + 'static);
pub fn apply_auto_registrations(collection: &mut ServiceCollection);

// 用户 crate（如 studio-shell/src/lib.rs）：
ctor::ctor!
fn register_services() {
    rust_rml_di::auto_register("studio-shell", |c| {
        c.keyed_singleton_trait::<dyn IWorkbenchProvider>("rml", |_| {
            Arc::new(WelcomeProvider) as Arc<dyn IWorkbenchProvider>
        });
    });
}
```

`Fn`（非 `FnOnce`）+ 非 drain 式读取，支持多次 build。

---

## 三、实施步骤

### Step 1: 完善 `RmlApplication<W>::run_with`（Task 3 续）

**文件**: `crates/app/src/application.rs`

为 `RmlApplication<W>`（窗口变体）添加 `run_with` 方法，镜像 `NoWindow` 变体模式：

```rust
impl<W: IWindow + Default + 'static> RmlApplication<W> {
    pub fn run<L: IAppLifecycle + 'static>(self) {
        self.run_with::<L>(None);
    }

    pub fn run_with<L: IAppLifecycle + 'static>(
        self,
        before_launch: Option<Box<dyn FnOnce(&mut App) + 'static>>,
    ) {
        gpui_platform::application()
            .with_assets(crate::assets::CompositeAssets)
            .run(move |cx: &mut App| {
                bootstrap_runtime(cx);
                if let Some(f) = before_launch {
                    f(cx);
                }
                L::default().on_launch(cx);
                W::default().open(cx);
            });
    }
}
```

**验证**: `cargo check -p rust-rml-app`

### Step 2: 修复 app crate 编译错误（Task 4）

**文件**: `crates/app/src/extensions.rs`

第 45 行旧导出：
```rust
// 删除此行
pub use rml_core::context::{ensure_service_collection, ServiceCollection};
// 替换为
pub use rml_core::context::{ensure_service_provider, DefaultServiceProvider, IServiceProvider, ServiceSlot, ServiceProviderExt};
```

同时更新注释中的 "ServiceCollection" → "DefaultServiceProvider / IServiceProvider"。

**文件**: `crates/app/src/lib.rs`

第 28 行旧导出：
```rust
// 删除此行
pub use extensions::{IAppContextExt, ServiceCollection, ensure_service_collection};
// 替换为
pub use extensions::IAppContextExt;
```
（`IServiceProvider` 等已从 `rml_core` 直接导出，无需经 app 中转）

**文件**: `crates/app/src/contribution/entity_cache.rs` + `global.rs`

更新注释中 "ServiceCollection" 引用为 "IServiceProvider / DefaultServiceProvider"（仅注释，无代码变更）。

**验证**: `cargo check -p rust-rml-app`

### Step 3: 在 core 中添加 ServiceSlot + ServiceProviderExt

**文件**: `crates/core/src/context.rs`

在现有 `IServiceProvider` 定义之后添加：

1. `ServiceSlot<T: ?Sized + 'static>` 结构体 + `pub` 字段
2. `ServiceProviderExt` trait + blanket impl
3. 更新 `lib.rs` 和 `prelude.rs` 导出 `ServiceSlot, ServiceProviderExt`

**验证**: `cargo check -p rust-rml-core`

### Step 4: 创建 rust-rml-di crate（Task 5）

**新建文件**:

1. `crates/di/Cargo.toml`:
```toml
[package]
name = "rust-rml-di"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "rust-dix DI 容器适配层 —— 对接 RML IServiceProvider 抽象"

[dependencies]
rust-rml-core = { workspace = true }
rust-rml-app = { workspace = true }
rust-dix = { workspace = true }
gpui = { workspace = true }
anyhow = { workspace = true }
ctor = "0.2"
log = { workspace = true }
```

2. `crates/di/src/lib.rs`: 模块声明 + re-exports
3. `crates/di/src/collection.rs`: `ServiceCollection`（builder，包装 rust-dix + resolver map）
4. `crates/di/src/provider.rs`: `DixServiceProvider`（impl `IServiceProvider`）
5. `crates/di/src/application.rs`: `DiRmlApplication<W>` + `RmlApplicationExt<W>` trait
6. `crates/di/src/auto_register.rs`: 全局注册表 + `auto_register()` + `apply_auto_registrations()`
7. `crates/di/src/prelude.rs`: re-exports

**注册到 workspace**: `Cargo.toml` 的 `members` 添加 `"crates/di"`，`[workspace.dependencies]` 添加 `rust-rml-di = { path = "crates/di" }`

**验证**: `cargo check -p rust-rml-di`

### Step 5: Studio Shell 迁移（Task 6）

#### 5a. `studio/shell/Cargo.toml`

```toml
# 替换 rust-dix 依赖
rust-rml-di = { workspace = true }
# 删除 rust-dix = { workspace = true }
```

#### 5b. `studio/shell/src/di.rs` — 重写

```rust
use std::sync::Arc;
use rml_core::context::IServiceProvider;
use rml_core::workbench::{IWorkbenchManager, IWorkbenchProvider};
use rust_rml_di::{ServiceCollection, DixServiceProvider};
use studio_core::workspace::IWorkspaceManager;
use crate::shell_manager::ArcShellManager;
use crate::welcome::WelcomeProvider;
use studio_editor::editor_provider::EditorProvider;

pub fn build_provider(manager: Arc<ArcShellManager>) -> anyhow::Result<Arc<dyn IServiceProvider>> {
    let collection = ServiceCollection::new()
        .singleton_trait::<dyn IWorkspaceManager>(move |_| {
            manager.clone() as Arc<dyn IWorkspaceManager>
        })
        .singleton_trait::<dyn IWorkbenchManager>(move |_| {
            manager.clone() as Arc<dyn IWorkbenchManager>
        })
        .keyed_singleton_trait::<dyn IWorkbenchProvider>("rml", move |_| {
            Arc::new(WelcomeProvider) as Arc<dyn IWorkbenchProvider>
        })
        .keyed_singleton_trait::<dyn IWorkbenchProvider>("file", move |_| {
            Arc::new(EditorProvider) as Arc<dyn IWorkbenchProvider>
        });

    let provider = Arc::new(collection.build()?);
    manager.set_provider(provider.clone());
    Ok(provider)
}
```

#### 5c. `studio/shell/src/shell_manager.rs` — 解耦 rust-dix

```rust
// 替换
use rust_dix::ServiceProvider;
// 为
use rml_core::context::{IServiceProvider, ServiceProviderExt};

// 字段类型替换
provider: OnceLock<Arc<dyn IServiceProvider>>,  // 原 OnceLock<Arc<ServiceProvider>>

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
// 使用 rml_core::context::IServiceProvider

// on_loaded 中：
// 原: cx.set_service::<ServiceProvider>(provider);
// 新: cx.use_provider(provider);
```

#### 5e. `studio/shell/src/lib.rs` — 自动注册静态服务

```rust
ctor::ctor!
fn register_static_services() {
    rust_rml_di::auto_register("studio-shell-static", |c| {
        // 静态服务（无运行时依赖）可在此自动注册
        // WelcomeProvider / EditorProvider 可选移入此处
    });
}
```

（注：`ArcShellManager` 因运行时创建 + 循环依赖，仍保留在 `di::build_provider` 中手动注册。）

#### 5f. `studio/explorer/src/explorer_panel.rml.rs` — 简化查询

```rust
// 删除 use rust_dix::ServiceProvider;
use rml_core::context::ServiceProviderExt;

// refresh_tree 中：
// 原: cx.get_service::<ServiceProvider>().and_then(|p| p.get::<dyn IWorkspaceManager>().ok()).map(...)
// 新: cx.get_trait::<dyn IWorkspaceManager>().map(...)

// on_file_activate 中：
// 原: let Some(provider) = cx.get_service::<ServiceProvider>() else { return; };
//     let Ok(workspace_mgr) = provider.get::<dyn IWorkspaceManager>() else { ... };
//     let Ok(workbench_mgr) = provider.get::<dyn IWorkbenchManager>() else { ... };
// 新: let Some(workspace_mgr) = cx.get_trait::<dyn IWorkspaceManager>() else { return; };
//     let Some(workbench_mgr) = cx.get_trait::<dyn IWorkbenchManager>() else { return; };
```

**验证**: `cargo check -p studio-shell -p studio-explorer`

### Step 6: Studio App 入口迁移

**文件**: `studio/app/Cargo.toml` — 添加 `rust-rml-di` 依赖

**文件**: `studio/app/src/main.rs`:
```rust
use rust_rml_di::prelude::*;

fn main() {
    RmlApplication::new()
        .main_window::<studio_shell::MainWindow>()
        .use_dix()  // 启用 rust-dix 后端（静态服务自动注册）
        .run::<Startup>();
}
```

（注：`use_dix()` 的 `before_launch` 仅注入空的 `DixServiceProvider`（含自动注册的静态服务）。`ArcShellManager` 等运行时服务仍在 `MainWindow::on_loaded` → `di::build_provider` 中构建并 `cx.use_provider()` 覆盖。）

**验证**: `cargo check -p arc-studio`

### Step 7: 全量构建验证（Task 7）

```powershell
cargo build --workspace
cargo test -p rust-rml-core --lib
cargo test -p rust-rml-di --lib   # 若有测试
```

确认 demo 不受影响（demo 不依赖 rust-dix，使用 `cx.set_service`/`cx.get_service::<ConcreteType>()` 模式，与新抽象完全兼容）。

---

## 四、假设与决策

### 假设

1. `rust_dix::ServiceCollection` 使用消费式 builder 模式（`self` → `Self`），与 `di.rs` 中的链式调用一致
2. `rust_dix::ServiceProvider::get::<T>()` 支持 `T: ?Sized`（`di.rs` 中 `provider.get::<dyn IWorkspaceManager>()` 已验证）
3. `rust_dix::ServiceCollection::build()` 返回 `anyhow::Result<Arc<ServiceProvider>>`（基于 `di.rs` 返回类型推断）
4. `rust_dix::ServiceProvider: Send + Sync`（DI 容器通常线程安全）
5. Demo 不使用 rust-dix，无需迁移（已验证：demo 的 `Cargo.toml` 无 rust-dix 依赖，服务查询均为具体类型）

### 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 抽象范围 | 中等（仅解析接口，不含配置阶段） | 用户明确选择 |
| 适配 crate 命名 | `rust-rml-di` | 用户指定 |
| 重构策略 | 破坏性 | 用户允许，旧 API 不兼容 |
| Trait object 查询 | `ServiceSlot<T>` 包装模式 | Rust DI 标准做法，`dyn Trait: !Sized` 无法直接 downcast |
| 自动注册 | `ctor` + 全局 `Fn` 闭包表 | 无需自定义 proc-macro，与项目现有 `ctor` 模式一致 |
| Builder API | `use_dix()` 链式 + `configure()` 闭包 | 两种注册风格互补 |
| Studio 循环依赖 | 保留 `di::build_provider` 手动构建 | `ArcShellManager` 需运行时创建 + 二阶段注入，不适合 `before_launch` 阶段 |

### 非目标

- 不实现配置阶段抽象（`IServiceCollection` trait）—— 中等抽象范围
- 不添加 proc-macro 自动注册（`#[service]` 属性宏）—— 未来增强
- 不迁移 demo —— demo 不使用 rust-dix
- 不修改 `DefaultServiceProvider` 的存储结构 —— 已在 Task 1 完成，`ServiceSlot` 自动兼容

---

## 五、验证清单

- [ ] `cargo check -p rust-rml-core` — ServiceSlot + ServiceProviderExt 编译
- [ ] `cargo check -p rust-rml-app` — extensions/lib 导出修复 + run_with 完整
- [ ] `cargo check -p rust-rml-di` — 适配 crate 编译
- [ ] `cargo check -p studio-shell` — shell_manager/di/main_window 迁移
- [ ] `cargo check -p studio-explorer` — explorer_panel 查询简化
- [ ] `cargo check -p arc-studio` — 入口 use_dix 链式
- [ ] `cargo build --workspace` — 全量编译
- [ ] `cargo test -p rust-rml-core --lib` — 核心测试不回归
- [ ] 确认 demo 无需修改即编译通过
