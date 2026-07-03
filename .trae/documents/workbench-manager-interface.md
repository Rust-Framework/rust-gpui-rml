# 工作台管理器抽象接口（IWorkbenchManager / IWorkbench）

## Summary

在框架层新增「工作台管理器」抽象：`IWorkbenchManager`（资源打开/关闭/查询）与 `IWorkbench`（单个已打开资源的会话句柄）。框架仅定义接口与注册/访问入口，由业务自行实现。设计严格镜像现有 `IContributionHost` / `ContributionRegistryExt` 模式：

* 接口定义放在 `rml-core`（与 `contribution.rs` 并列的新模块）。

* App 扩展 + `OnceLock` 进程级槽位放在 `rml-app`（与 `contribution/global.rs` 并列）。

* 所有方法 `&self` + 内部可变性，UI 相关工作由业务延迟到具备 `&mut App` 的时机处理（与 `IContributionHost::add/remove` 一致）。

* `Uri` 复用 `url::Url`（业务回答确认）。

本任务**只交付框架接口与注册/访问入口**，不含业务实现（用户明确「由业务自行实现」）。

## Current State Analysis

* 框架核心 trait 集中在 [contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs)：`IContribution`/`IVisualContribution`/`IContributionHost`/`IContributionRegistry`，全部 `&self` + 内部可变性，`IContribution` 带 `Any` supertrait 以支持能力查询。

* App 扩展在 [global.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/global.rs)：`static REGISTRY: OnceLock<ContributionRegistry>` + `ContributionRegistryExt` on `App`，`get_contribution_registry() -> &'static dyn IContributionRegistry`。这是 `rml-app` 持有 `OnceLock` 槽位、返回 `&'static` 引用的现成范本。

* `rml-core` 当前**不依赖** `url`（[core/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/Cargo.toml) 仅 gpui/serde\_json/ctor/flume）；`url = "2"` 仅 demo 使用（[demo/Cargo.toml:17](file:///e:/GitCode/RF/rust-gpui-rml/demo/Cargo.toml#L17)）。需在 core 引入。

* `rml-core/src/lib.rs` 重导出 `ctor`/`flume` crate 供业务免显式依赖；同样模式可重导出 `url`。

* [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) 集中导出常用 trait；[app/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/lib.rs) 顶层 `pub mod` + `pub use` 暴露运行时扩展。

* 工作台概念在仓库中**完全不存在**（grep `workbench|Workbench` 无命中），属全新设计。

* `IContributionHost` 已确立「`&self` 方法 + 业务延迟 cx 工作」范式（host 在 `on_loaded(&mut App,&mut Window)` 才做实际 UI 构建），本设计沿用。

## Assumptions & Decisions

1. **Uri =** **`url::Url`**（用户确认）。在 core 的 `workbench` 模块中 `pub use url::Url as Uri;`，并在 `lib.rs` 重导出 `url` crate（与 ctor/flume 一致），业务 crate 无需显式依赖 url。
2. **纯** **`&self`** **方法，无 cx/window 参数**（用户确认）。镜像 `IContributionHost`：业务用 `RwLock`/channel 内部可变性，cx 相关 UI 工作延迟到具备 `&mut App` 的时机（如宿主实体的 `on_loaded`/observe 回调）。
3. **注册/访问 = App 扩展 + OnceLock 槽位**（用户确认）。镜像 `ContributionRegistryExt`，但 impl 由业务提供，故槽位存 `Arc<dyn IWorkbenchManager>`，`set` 仅首次生效，`get` 返回 `Option<&'static dyn IWorkbenchManager>`。
4. **`IWorkbench`** **加** **`Any`** **supertrait**（与 `IContribution: Send + Sync + Any` 一致），便于业务按需 downcast 到具体工作台类型；不增加方法，符合现有约定。
5. **`set(key, value)`** **值类型 =** **`Box<dyn Any + Send + Sync>`**：对象安全要求非泛型；类型擦除值，业务按 key 自行 downcast。key 用 `SharedString`（与 contribution 模块一致）。
6. **返回类型用** **`Arc<dyn IWorkbench>`**：对象安全 + 共享所有权（多个查询者可持有同一句柄）。`get_all` 返回 `Vec<Arc<dyn IWorkbench>>`，`get_activated`/`get` 返回 `Option<Arc<dyn IWorkbench>>`。
7. **单例 manager**：`IWorkbenchManager` 为进程级单例（OnceLock 一次性 set）。如需多实例/替换，后续再扩展；当前不预设。
8. **不含业务实现**：本任务不写 demo 侧 `IWorkbenchManager`/`IWorkbench` 实现，仅保证框架层编译通过、接口可被业务实现。

## Proposed Changes

### 1. [crates/core/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/Cargo.toml) — 引入 url 依赖

在 `[dependencies]` 末尾追加：

```toml
url = "2"
```

### 2. 新建 [crates/core/src/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs) — 接口定义

定义 `IWorkbench`、`IWorkbenchManager` 两个 trait 与 `Uri` 别名。完整内容：

```rust
//! 工作台管理器契约 —— 资源打开/关闭/激活的抽象接口
//!
//! 框架仅定义接口，由业务实现。manager 经 `rml_app::WorkbenchManagerExt` 安装到 App，
//! 通过 OnceLock 进程级槽位访问。所有方法 `&self` + 内部可变性，UI 相关工作由业务
//! 自行延迟到具备 `&mut App` 的时机处理（镜像 `IContributionHost` 模式）。

use std::any::Any;
use std::sync::Arc;

use gpui::SharedString;

/// Uri 类型：复用 `url::Url`。
pub use url::Url as Uri;

/// 工作台：一个已打开资源的会话句柄。
///
/// 业务实现此 trait；实例由 `IWorkbenchManager::open` 返回。
/// `close`/`activate`/`set` 均为 `&self`——业务使用内部可变性，
/// 并将 cx 相关 UI 工作延迟到具备 `&mut App` 的时机（如宿主实体的 `on_loaded`/observe 回调）。
/// 加 `Any` supertrait——与 `IContribution` 一致，便于业务按需 downcast 到具体工作台类型。
pub trait IWorkbench: Send + Sync + Any {
    /// 关闭此工作台。
    fn close(&self);

    /// 激活此工作台。
    fn activate(&self);

    /// 向此工作台设置数据（类型擦除值，业务按 key 自行 downcast）。
    fn set(&self, key: SharedString, value: Box<dyn Any + Send + Sync>);
}

/// 工作台管理器：资源的打开/关闭/查询。
///
/// 业务实现并经 `rml_app::WorkbenchManagerExt::set_workbench_manager` 安装。
/// 所有方法 `&self`——业务用 `RwLock`/channel 等内部可变性，UI 工作延迟处理。
pub trait IWorkbenchManager: Send + Sync + 'static {
    /// 打开资源；若已打开则激活现有工作台并返回。
    fn open(&self, uri: &Uri) -> Arc<dyn IWorkbench>;

    /// 关闭资源对应的工作台。
    fn close(&self, uri: &Uri);

    /// 当前所有已打开的工作台。
    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>>;

    /// 当前激活的工作台。
    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>>;

    /// 按 Uri 获取工作台。
    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>>;
}
```

### 3. [crates/core/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs) — 注册模块 + 重导出 url

* 在第 14 行（`pub use flume;` 之后）追加：`pub use url;`（与 ctor/flume 一致，业务免显式依赖）。

* 在模块声明区（`pub mod contribution;` 之后，约第 25 行）追加：`pub mod workbench;`。

### 4. [crates/core/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) — prelude 导出

在 `contribution` 导出块之后追加：

```rust
pub use crate::workbench::{IWorkbench, IWorkbenchManager, Uri};
```

### 5. 新建 [crates/app/src/workbench/global.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/workbench/global.rs) — App 扩展槽位

镜像 [contribution/global.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/global.rs)。完整内容：

```rust
//! 工作台管理器 App 扩展
//!
//! 框架内部：维护进程级 `Arc<dyn IWorkbenchManager>` 静态槽位；提供
//! `set_workbench_manager`/`get_workbench_manager` 扩展方法。
//!
//! 镜像 `ContributionRegistryExt`：OnceLock 进程级存储，`get_workbench_manager`
//! 返回 `Option<&'static dyn IWorkbenchManager>`，所有方法 `&self`，不借用 App。
//! manager 实现由业务提供，在启动时（如宿主 `on_loaded`）调用 `set_workbench_manager` 安装。

use std::sync::{Arc, OnceLock};

use gpui::App;
use rml_core::workbench::IWorkbenchManager;

static WORKBENCH_MANAGER: OnceLock<Arc<dyn IWorkbenchManager>> = OnceLock::new();

/// App 扩展：安装/获取 `IWorkbenchManager`。
pub trait WorkbenchManagerExt {
    /// 安装工作台管理器。仅首次调用生效；重复调用返回 `false` 并 warn。
    fn set_workbench_manager(&self, manager: Arc<dyn IWorkbenchManager>) -> bool;

    /// 获取已安装的工作台管理器（`&'static`，不借用 App）。
    fn get_workbench_manager(&self) -> Option<&'static dyn IWorkbenchManager>;
}

impl WorkbenchManagerExt for App {
    fn set_workbench_manager(&self, manager: Arc<dyn IWorkbenchManager>) -> bool {
        WORKBENCH_MANAGER.set(manager).is_ok()
    }

    fn get_workbench_manager(&self) -> Option<&'static dyn IWorkbenchManager> {
        WORKBENCH_MANAGER.get().map(|a| a.as_ref())
    }
}
```

### 6. 新建 [crates/app/src/workbench/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/workbench/mod.rs) — 模块入口

```rust
//! 工作台管理器运行时：App 扩展槽位

mod global;

pub use global::WorkbenchManagerExt;
```

### 7. [crates/app/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/lib.rs) — 注册模块 + 导出

* 在 `pub mod resources;` 之后（约第 13 行）追加：`pub mod workbench;`。

* 在 `pub use contribution::{...};` 之后追加：`pub use workbench::WorkbenchManagerExt;`。

## Verification

1. `cargo build -p rust-rml-core` —— core 引入 url 依赖 + 新 workbench 模块编译通过；prelude 导出正常。
2. `cargo build -p rust-rml-app` —— app 新增 workbench 模块 + `WorkbenchManagerExt` 编译通过；`App` 上可调用 `set_workbench_manager`/`get_workbench_manager`。
3. `cargo build -p rust-rml-demo` —— demo 仍编译通过（接口新增不破坏现有调用）。
4. 对象安全性自检：`dyn IWorkbench` 与 `dyn IWorkbenchManager` 可构造（无泛型方法，返回值均为 `Arc<dyn IWorkbench>`/`Vec`/`Option`），`cargo build` 通过即验证。
5. （可选，非本任务范围）业务后续实现 `IWorkbenchManager`/`IWorkbench` 后，在宿主 `on_loaded` 中 `cx.set_workbench_manager(Arc::new(...))`，即可全局 `cx.get_workbench_manager()` 访问。

## Out of Scope

* 业务侧 `IWorkbenchManager`/`IWorkbench` 的具体实现（用户明确「由业务自行实现」）。

* `#[workbench]` 之类的宏支持（当前仅手写 impl，与 `IContributionHost` 手写 impl 一致）。

* 多 manager 实例 / 运行时替换（当前 OnceLock 单例足够，后续按需扩展）。

* Uri 的 schema 约定/校验（由业务 `open` 实现自行解释 `url::Url`）。

