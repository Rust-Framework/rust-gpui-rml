# IAppContext 迁移收尾——Demo 业务代码替换 DemoShellHost

## 摘要

前序会话已完成 IAppContext 核心设计（`crates/core/src/context.rs`）、框架扩展聚合（`crates/app/src/extensions.rs`）、`ContributionRegistry` / `WorkbenchManager` 迁移到 `ServiceCollection`、`entity_cache` 改写为基于 IAppContext 的实现，以及 `engine/src/runtime/` stub 清理。框架侧 `rust-rml-core` / `rust-rml-app` 已编译通过。

**本计划聚焦唯一剩余工作**：原计划 Step 11——将 demo 中自创的 `DemoShellHost` Global 服务定位器替换为正式的 `IAppContext::get_service::<MainWindowRef>()` 查询模式。

Step 12（`visual_entity::<T>(cx)` 调用替换）已因 `entity_cache.rs` 改写为基于 IAppContext 的实现而**自动消解**——`visual_entity` 函数仍保留导出，demo 调用点无需修改。Step 13（`drain_host_ops` 替换）因 Step 9 host_handle 简化降级而**跳过**。

---

## 一、当前状态分析

### 已完成（框架侧）

| 文件 | 状态 | 说明 |
|---|---|---|
| `crates/core/src/context.rs` | ✅ 新建 | `IAppContext` trait + `ServiceCollection` + App/Context 双实现 |
| `crates/core/src/lib.rs` | ✅ 修改 | 导出 `pub mod context` + re-export |
| `crates/core/src/prelude.rs` | ✅ 修改 | `IAppContext` / `ServiceCollection` / `ensure_service_collection` 已入 prelude |
| `crates/app/src/application.rs` | ✅ 修改 | `bootstrap_runtime` 初始化 `ServiceCollection` + 注册 `ContributionRegistry` 单例 |
| `crates/app/src/contribution/global.rs` | ✅ 重写 | 移除 `OnceLock` + `ContributionRegistryExt`，仅保留 bootstrap 回调 |
| `crates/app/src/contribution/host_handle.rs` | ✅ 修改 | 改用 `cx.get_required_service::<ContributionRegistry>()` 替代 `ContributionRegistryExt` |
| `crates/app/src/contribution/entity_cache.rs` | ✅ 改写 | 内部存储改为 `VisualEntityCache` 经 `IAppContext::set_service` 注册，对外 API 不变 |
| `crates/app/src/contribution/mod.rs` | ✅ 修改 | 重新导出 `get_or_create_entity` / `visual_entity` / `VisualEntityCache` |
| `crates/app/src/workbench/global.rs` | ✅ 重写 | 仅保留 `WorkbenchManagerSlot` newtype |
| `crates/app/src/workbench/mod.rs` | ✅ 修改 | 导出 `WorkbenchManagerSlot` |
| `crates/app/src/extensions.rs` | ✅ 新建 | `IAppContextExt` 便利方法 + 中央聚合 re-export |
| `crates/app/src/prelude.rs` | ✅ 新建 | `pub use crate::extensions::*` |
| `crates/app/src/lib.rs` | ✅ 重写 | 导出 `IAppContext` / `IAppContextExt` / `ServiceCollection` |
| `crates/engine/src/runtime/mod.rs` | ✅ 修改 | 移除 3 个 stub，仅保留 `event_flow` |

### 待完成（demo 侧）

demo 仍使用 `DemoShellHost` 作为 makeshift Global 服务定位器，共 **5 处文件、18 处引用**：

| 文件 | 行号 | 引用类型 |
|---|---|---|
| `demo/src/shell/main_window.rml.rs` | 5 | `use rml_app::WorkbenchManagerExt;`（已移除的 trait） |
| `demo/src/shell/main_window.rml.rs` | 23-25 | `DemoShellHost` 结构定义 + `impl Global` |
| `demo/src/shell/main_window.rml.rs` | 88 | `cx.set_global(DemoShellHost(shell_weak));` |
| `demo/src/shell/main_window.rml.rs` | 101 | `cx.set_workbench_manager(manager.clone());`（需 `IAppContextExt` trait） |
| `demo/src/shell/mod.rs` | 12 | `pub use main_window::{DemoShellHost, MainWindow};` |
| `demo/src/shell/activity_panel.rml.rs` | 7 | `use crate::shell::DemoShellHost;` |
| `demo/src/shell/activity_panel.rml.rs` | 40 | `cx.try_global::<DemoShellHost>()` |
| `demo/src/shell/activity_panel.rml.rs` | 73 | `cx.try_global::<DemoShellHost>()` |
| `demo/src/lsp/lsp_explorer_panel.rml.rs` | 6 | `use crate::shell::DemoShellHost;` |
| `demo/src/lsp/lsp_explorer_panel.rml.rs` | 60 | `cx.try_global::<DemoShellHost>()` |
| `demo/src/shell/menu_shell_contribs.rs` | 11 | `use crate::shell::{DemoShellHost, MainWindow};` |
| `demo/src/shell/menu_shell_contribs.rs` | 21 | `ctx.app.try_global::<DemoShellHost>()` |

### 关键依赖确认

- `IAppContext` trait 已在 `rml_core::prelude` 导出，demo 的 `use rml::prelude::*;`（经 `rml_engine::prelude → rml_core::prelude`）自动获得
- `IAppContextExt` 在 `rml_app::lib.rs` 导出，提供 `set_workbench_manager` / `workbench_manager` 便利方法
- `WeakEntity<MainWindow>` 是 `Send + Sync`（原 `DemoShellHost` 作为 `Global` 已验证），故 `MainWindowRef` newtype 自动满足 `Send + Sync` 约束
- `IWorkbenchManager: Send + Sync + 'static`（`crates/core/src/workbench.rs:45`），`WorkbenchManagerSlot` 包装合法

---

## 二、提议变更

### Step 1：在 `main_window.rml.rs` 中引入 `MainWindowRef` newtype 并替换 `DemoShellHost`

**文件**：`demo/src/shell/main_window.rml.rs`

**变更 1.1**——替换 import（行 5）：

```rust
// 替换前
use rml_app::WorkbenchManagerExt;

// 替换后
use rml_app::IAppContextExt;
```

**说明**：`IAppContext` trait 经 `rml::prelude::*`（行 4）已入域；`IAppContextExt` 提供 `set_workbench_manager` 方法（行 101 调用）。无需额外 `use rml_core::context::IAppContext`——已由 prelude 提供。

**变更 1.2**——替换 `DemoShellHost` 定义（行 22-25）为 `MainWindowRef` newtype：

```rust
// 替换前（行 22-25）
/// Demo：ActivityPanel / LspExplorerPanel 通过它回调 MainWindow::open_case / open_lsp_file。
pub struct DemoShellHost(pub WeakEntity<MainWindow>);

impl Global for DemoShellHost {}

// 替换后
/// MainWindow 弱引用槽位——经 IAppContext::set_service 注册为单例，
/// ActivityPanel / LspExplorerPanel / 菜单命令通过 get_service::<MainWindowRef>() 查询。
pub struct MainWindowRef(pub WeakEntity<MainWindow>);
```

**说明**：移除 `impl Global for DemoShellHost`——不再需要 GPUI Global，改由 `ServiceCollection`（自身是 Global）托管。`Global` import 若不再被本文件其他代码使用，需从行 3 的 `use gpui::{...}` 中移除。

**变更 1.3**——替换注册调用（行 88）：

```rust
// 替换前
cx.set_global(DemoShellHost(shell_weak));

// 替换后
cx.set_service(std::sync::Arc::new(MainWindowRef(shell_weak)));
```

**说明**：`set_service` 由 `IAppContext` trait 提供（`Context<'_, T>` 实现经 `BorrowMut` 委托到 `App`）。`Arc` 已在行 1 导入。

**变更 1.4**——更新文档注释（行 86, 229, 243）：

将注释中的 `DemoShellHost` 替换为 `MainWindowRef / IAppContext`。

### Step 2：更新 `demo/src/shell/mod.rs` 导出

**文件**：`demo/src/shell/mod.rs`（行 12）

```rust
// 替换前
pub use main_window::{DemoShellHost, MainWindow};

// 替换后
pub use main_window::{MainWindow, MainWindowRef};
```

### Step 3：更新 `activity_panel.rml.rs`

**文件**：`demo/src/shell/activity_panel.rml.rs`

**变更 3.1**——替换 import（行 7）：

```rust
// 替换前
use crate::shell::DemoShellHost;

// 替换后
use crate::shell::MainWindowRef;
```

**变更 3.2**——替换 `on_loaded` 中的查询（行 40）：

```rust
// 替换前
if let Some(host) = cx.try_global::<DemoShellHost>() {
    self.main = Some(host.0.clone());
}

// 替换后
if let Some(host) = cx.get_service::<MainWindowRef>() {
    self.main = Some(host.0.clone());
}
```

**说明**：`get_service` 由 `IAppContext` trait 提供，经 `rml::prelude::*` 已入域。返回 `Option<Arc<MainWindowRef>>`，`host.0` 为 `WeakEntity<MainWindow>`，与原逻辑等价。

**变更 3.3**——替换 `on_case_activate` 中的查询（行 72-74）：

```rust
// 替换前
if let Some(host) = cx
    .try_global::<DemoShellHost>()
    .and_then(|h| h.0.upgrade())
{

// 替换后
if let Some(host) = cx
    .get_service::<MainWindowRef>()
    .and_then(|r| r.0.upgrade())
{

// 注意：变量名从 h 改为 r（ref），避免与外层 host 语义混淆
```

**变更 3.4**——更新文档注释（行 12）：

```rust
// 替换前
/// 点击案例 → DemoShellHost → MainWindow::open_case（D8 方案 A）。

// 替换后
/// 点击案例 → IAppContext::get_service::<MainWindowRef>() → MainWindow::open_case。
```

### Step 4：更新 `lsp_explorer_panel.rml.rs`

**文件**：`demo/src/lsp/lsp_explorer_panel.rml.rs`

**变更 4.1**——替换 import（行 6）：

```rust
// 替换前
use crate::shell::DemoShellHost;

// 替换后
use crate::shell::MainWindowRef;
```

**变更 4.2**——替换查询（行 59-61）：

```rust
// 替换前
if let Some(host) = cx
    .try_global::<DemoShellHost>()
    .and_then(|h| h.0.upgrade())
{

// 替换后
if let Some(host) = cx
    .get_service::<MainWindowRef>()
    .and_then(|r| r.0.upgrade())
{
```

**变更 4.3**——更新文档注释（行 10）：

```rust
// 替换前
/// 点击文件 → DemoShellHost → MainWindow::open_lsp_file 打开 CodeEditorTab。

// 替换后
/// 点击文件 → IAppContext::get_service::<MainWindowRef>() → MainWindow::open_lsp_file。
```

### Step 5：更新 `menu_shell_contribs.rs`

**文件**：`demo/src/shell/menu_shell_contribs.rs`

**变更 5.1**——替换 import（行 11）：

```rust
// 替换前
use crate::shell::{DemoShellHost, MainWindow};

// 替换后
use crate::shell::{MainWindow, MainWindowRef};
```

**变更 5.2**——替换 `with_main_window` helper 中的查询（行 19-22）：

```rust
// 替换前
if let Some(host) = ctx
    .app
    .try_global::<DemoShellHost>()
    .and_then(|h| h.0.upgrade())
{

// 替换后
if let Some(host) = ctx
    .app
    .get_service::<MainWindowRef>()
    .and_then(|r| r.0.upgrade())
{
```

**说明**：`ctx.app` 是 `&mut App`，`IAppContext` 已为 `App` 实现，`get_service` 可直接调用。需确认 `IAppContext` trait 在本文件可见——本文件行 7 `use rml::prelude::*;` 已包含。

**变更 5.3**——更新文档注释（行 3, 13-14）：

```rust
// 行 3 替换前
//! 叶子项通过 `DemoShellHost` 全局获取 `WeakEntity<MainWindow>`，在 `execute` 中

// 行 3 替换后
//! 叶子项通过 `IAppContext::get_service::<MainWindowRef>()` 获取 `WeakEntity<MainWindow>`，在 `execute` 中

// 行 13-14 替换前
/// 命令执行 helper：从 `DemoShellHost` 全局获取 `MainWindow` 弱引用，
/// upgrade 后在闭包中执行 MainWindow 方法。统一 6 处 `try_global`+`upgrade`+`update` 样板。

// 行 13-14 替换后
/// 命令执行 helper：从 `IAppContext::get_service::<MainWindowRef>()` 获取 `MainWindow` 弱引用，
/// upgrade 后在闭包中执行 MainWindow 方法。统一 6 处 `get_service`+`upgrade`+`update` 样板。
```

---

## 三、假设与决策

### 设计决策

1. **`MainWindowRef` newtype 而非直接注册 `WeakEntity<MainWindow>`**：避免与其他 `WeakEntity<T>` 类型在 `ServiceCollection` 中冲突（TypeId 相同会覆盖），newtype 提供唯一类型键
2. **保留 `Arc<MainWindowRef>` 返回值**：与 IAppContext 契约一致，`Arc` clone 是原子操作
3. **不修改 `#[contributehost]` 宏**：Step 9 已降级，保留 `drain_host_ops` channel 机制，demo 中 `__rml_install_host` + `drain_host_ops` 调用不变
4. **不修改 `visual_entity` 调用**：`entity_cache.rs` 已改写为基于 IAppContext 的实现，对外 API 不变，demo 调用点（`main_window.rml.rs:115,142`）无需改动

### 假设

1. **`WeakEntity<MainWindow>: Send + Sync`**：原 `DemoShellHost` 作为 `Global` 已验证（`Global: Send + Sync`），故 `MainWindowRef` 自动满足
2. **`use rml::prelude::*` 包含 `IAppContext`**：经 `rml_engine::prelude → rml_core::prelude` 链路确认（`crates/core/src/prelude.rs:10`）
3. **`Context<'_, T>` 上 `get_service` 可调**：`IAppContext` 已为 `Context<'_, T>` 实现（`crates/core/src/context.rs:107-118`），经 `Borrow`/`BorrowMut` 委托到 `App`
4. **`Global` import 清理**：`main_window.rml.rs` 行 3 `use gpui::{... Global ...}`——需检查移除 `DemoShellHost` 后是否还有其他 `Global` 用法；若无则从 import 列表移除 `Global`

### 不在本次范围

1. **Step 7（`observe_i18n`）**：降级为后续评估项，业务侧仍用 `cx.observe_global::<I18nState>()`
2. **Step 9（host_handle 简化）**：降级保留 channel 机制，需修改 `#[contributehost]` 宏，单独评估
3. **`ability.rs` unsafe 类型擦除重构**：独立议题

---

## 四、验证步骤

### 编译验证

```powershell
# 在 e:\GitCode\RF\rust-gpui-rml 目录
cargo check -p rust-rml-demo
```

**预期**：编译通过，无 `DemoShellHost` 未定义错误，无 `WorkbenchManagerExt` 未找到错误。

### 架构一致性检查

```powershell
# 应返回 0 匹配（DemoShellHost 已彻底移除）
rg "DemoShellHost" demo/

# 应返回 0 匹配（WorkbenchManagerExt 已替换为 IAppContextExt）
rg "WorkbenchManagerExt" demo/

# 应返回 1 处定义（MainWindowRef newtype）
rg "struct MainWindowRef" demo/

# 应返回多处 get_service 调用（activity_panel / lsp_explorer_panel / menu_shell_contribs）
rg "get_service::<MainWindowRef>" demo/
```

### 运行时验证

```powershell
cargo run -p demo
```

**预期**：
- 应用启动正常，MainWindow 显示
- ActivityPanel 加载案例列表正常（经 `get_service::<MainWindowRef>()` 取得 `WeakEntity<MainWindow>`）
- 点击案例打开对应 tab 正常（`on_case_activate` 回调路径不变）
- LSP Explorer 加载文件树正常，点击 `.rs`/`.rml` 文件打开 CodeEditorTab 正常
- 菜单命令（File → New/Open/Exit、View → Theme Toggle/Lang EN、Help → Guide/About/Cases）全部正常执行（经 `with_main_window` helper 路径不变）
- i18n / theme 切换功能正常
- ActivityBar 点击切换面板正常

### 回归检查

- `cx.set_workbench_manager(manager.clone())` 在 `main_window.rml.rs:101` 仍可调（经 `IAppContextExt` trait）
- `rml_app::contribution::visual_entity::<ActivityPanel>(cx)` 在 `main_window.rml.rs:115,142` 仍可调（`entity_cache` 对外 API 不变）
- `Self::__rml_install_host(cx.entity(), cx)` + `drain_host_ops(rx, self)` 在 `main_window.rml.rs:77,80` 仍可调（host_handle 机制保留）

---

## 五、实施顺序

```
Step 1（main_window.rml.rs）─→ Step 2（mod.rs）─→ Step 3（activity_panel）─┐
                                                                          ├─→ 验证
                                          Step 4（lsp_explorer）──────────┤
                                          Step 5（menu_shell_contribs）───┘
```

**关键路径**：Step 1 → Step 2（导出 `MainWindowRef` 供后续文件 import）→ Step 3/4/5（可并行编辑）→ 验证

**风险点**：无显著风险——所有变更都是机械替换 `try_global::<DemoShellHost>()` → `get_service::<MainWindowRef>()`，类型签名一致（都返回 `WeakEntity<MainWindow>`）。

---

## 六、回滚方案

若 `cargo check -p rust-rml-demo` 出现意外错误：

1. **`IAppContext` trait 不可见**：在对应文件顶部显式添加 `use rml_core::context::IAppContext;`
2. **`set_service` / `get_service` 方法未找到**：确认 `use rml::prelude::*;` 存在，或显式 `use rml_app::IAppContextExt;`（仅 `main_window.rml.rs` 需要）
3. **`MainWindowRef` 未导出**：确认 `demo/src/shell/mod.rs:12` 已更新为 `pub use main_window::{MainWindow, MainWindowRef};`
4. **整体阻塞**：保留 `DemoShellHost` 作为兼容层，仅完成框架侧迁移，demo 侧后续单独处理

每个 Step 独立 commit，便于回滚。
