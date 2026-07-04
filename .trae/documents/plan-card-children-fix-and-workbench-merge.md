# Card 子节点丢失修复 + IWorkbenchManager 合并到 MainWindow

## Context

用户报告两个问题：

1. **所有案例页面长得一样** —— 不同 `.rml` 模板（button\_case / counter\_case / welcome\_case 等）渲染出相同的 4 个空 Card。根因：RML 编译器的 `is_container` 检查只认 `StatelessNoId` 组件为容器，`Card` 注册为 `Stateless`（因需 ElementId），导致 `<Card>` 的元素子节点被静默丢弃，只保留 `title="..."` 属性。

2. **IWorkbenchManager 应由 MainWindow 直接实现** —— 当前 `DemoWorkbenchManager` 是独立结构，与 MainWindow 存在 `cases` 双写（MainWindow\.cases + manager.cases）。用户要求 `impl IWorkbenchManager for MainWindow`，消除中间层。调研发现 `cx.workbench_manager()` 全局服务从未被调用，全局注册机制为死代码，应一并移除。

## 修复 1：Card 子节点丢失

### 根因

[component.rs#L219-L222](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L219-L222)：

```rust
let is_container = matches!(component.kind, tags::ComponentKind::StatelessNoId)
    && canonical != "menu"
    && canonical != "MenuBar"
    && canonical != "Avatar";
```

`Card` 是 `Stateless`（非 `StatelessNoId`），`is_container = false` → 元素子节点被丢弃。

### 修复方案

在 `ComponentTag` 增加 `container: bool` 字段，标记组件是否实现 `ParentElement`（支持 `.child(...)`）。

**文件修改：**

1. **[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)** — `ComponentTag` 结构体

   * 新增 `pub container: bool` 字段

   * `Card`、`TitleBar`、`StatusBar`、`AvatarGroup` 设 `container: true`

   * 其他组件 `container: false`

2. **[component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)** — `gen_component` 函数

   * `is_container` 改为 `component.container && canonical != "menu" && canonical != "MenuBar" && canonical != "Avatar"`

   * 移除 `matches!(component.kind, tags::ComponentKind::StatelessNoId)` 判断

3. **[tags.rs 测试](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)** — 更新现有测试中 `ComponentTag` 构造（如有）

### 验证

* 检查生成的 `button_case.rs`：Card 应包含 `.child(...)` 调用（`<p>`、`<div>`、`<Button>` 等子节点）

* 运行 demo，点击不同案例节点，页面内容应不同

## 修复 2：IWorkbenchManager 合并到 MainWindow

### 设计

* 移除 `DemoWorkbenchManager` 结构体

* MainWindow 新增 `RwLock` 保护的 workbench 状态字段

* `impl IWorkbenchManager for MainWindow` —— 方法用 `RwLock` 内部可变性

* 移除全局注册机制（`set_workbench_manager` / `workbench_manager` / `WorkbenchManagerSlot`）

* 保留 `CaseWorkbench` / `LspWorkbench` / `LspWorkbenchProvider`（IWorkbench 实现仍需独立结构）

### 文件修改

1. **[main\_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)** — MainWindow 主体

   **字段变更：**

   * 移除 `manager: Option<Arc<DemoWorkbenchManager>>`

   * 新增 `workbenches: Arc<RwLock<Vec<Arc<dyn IWorkbench>>>>`

   * 新增 `activated: Arc<RwLock<Option<Arc<dyn IWorkbench>>>>`

   * 新增 `lsp_provider: Arc<LspWorkbenchProvider>`

   **方法变更：**

   * `init_workbench` —— 不再创建 `DemoWorkbenchManager`；直接初始化 `lsp_provider`；打开 welcome tab

   * `open_case` —— 调用 `self.open(&uri)`（IWorkbenchManager 方法）

   * `open_lsp_file` —— 调用 `self.open(&uri)`

   * `on_tab_click` —— 调用 `self.activate_by_index(index)` + `sync_tab_state`

   * `active_view` —— 读 `self.activated`，经 `as_visual()` 渲染

   * `sync_tab_state` —— 读 `self.workbenches` + `self.activated` 派生 `open_tabs` / `selected_tab`

   * 新增 `open_workbench(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>>` —— 内部方法，按 URI schema 路由

   **新增 trait impl：**

   ```rust
   impl IWorkbenchManager for MainWindow {
       fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> { ... }
       fn close(&self, uri: &Uri) { ... }
       fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> { ... }
       fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> { ... }
       fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> { ... }
   }
   ```

   **Default impl 变更：** 新字段初始化为空 `RwLock` / `Arc::new(LspWorkbenchProvider::new(...))`

2. **[workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs)** — 精简

   * 移除 `DemoWorkbenchManager` 结构体及其 `IWorkbenchManager` impl

   * 移除 `sync_cases` / `get_all_as_values` / `render_activated` / `activate_by_index` / `activated_index` / `build_workbench` / `open_workbench` 方法

   * 保留 `CaseWorkbench` / `LspWorkbench` / `LspWorkbenchProvider` / `register_workbench_abilities`

3. **[extensions.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/extensions.rs)** — 移除全局注册

   * 移除 `set_workbench_manager` / `workbench_manager` 方法

   * 移除 `use crate::workbench::WorkbenchManagerSlot`

   * 移除 `use rml_core::workbench::IWorkbenchManager`

4. **[crates/app/src/workbench/](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/workbench)** — 移除模块

   * 删除 `global.rs`（`WorkbenchManagerSlot`）

   * 删除 `mod.rs`（或清空，仅保留模块声明结构）

   * 更新 `crates/app/src/lib.rs` 移除 `mod workbench` 声明

5. **[crates/core/src/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs)** — 保留 trait 定义

   * `IWorkbenchManager` trait 不变（仍需 `Send + Sync + 'static` bound）

   * MainWindow 实现该 trait，需 `MainWindow: Send + Sync`（已验证字段均满足）

### Send + Sync 验证

MainWindow 所有字段均满足 `Send + Sync`：

* `Vec<CaseViewModel>` / `Vec<MenuViewModel>` / `Vec<StatusViewModel>` —— 内含 `Arc<dyn IContribution>` / `Arc<dyn ICommand>`，`IValue: Send + Sync`

* `Vec<Arc<dyn IActivityPanel>>` —— `IActivityPanel: IVisualContribution: IContribution: IValue: Send + Sync`

* `Option<gpui::Entity<ActivityBar>>` —— `Entity<T>: Send + Sync` when `T: Send + Sync`

* `Option<Arc<LspClient>>` —— `LspClient` 含 `Child`/`Sender`/`Arc<Mutex>`/`AtomicU64`，均 `Send + Sync`

* `RmlState` —— `ComputedCache` 有 unsafe impl Send+Sync；`SlotRenderer: Box<dyn Fn + Send + Sync>`

## 验证步骤

1. `cargo build -p rust-rml-demo` 编译通过
2. 检查生成的 `button_case.rs` —— Card 应有 `.child(...)` 调用
3. `cargo run -p rust-rml-demo` 运行 demo
4. 点击不同案例节点（button / counter / welcome），页面内容应不同
5. 点击 LSP 面板文件，CodeEditorTab 正常打开
6. 切换 tab，激活态正确同步
7. 关闭案例 tab 后再重新打开，状态正确

