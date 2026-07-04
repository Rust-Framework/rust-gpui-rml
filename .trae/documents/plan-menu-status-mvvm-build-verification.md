# Menu/StatusBar MVVM 重构 — 构建验证与收尾

## 摘要

前序会话已完成 menu/status-bar MVVM 数据绑定 + slot/template UI 设计的全部代码改动。本计划聚焦**剩余的唯一工作**：对 `rust-rml-ui` 和 `rust-rml-demo` 两个 crate 执行构建验证，修复编译错误，完成收尾。

`rust-rml-core` 与 `rust-rml-engine` 已在前序会话构建通过（仅 deprecation 警告）。

## 当前状态分析（已实际验证）

### 框架侧（全部 DONE）

| 文件 | 状态 | 说明 |
|------|------|------|
| [crates/ui/src/components/menu.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs) | ✓ | 纯 `ParentElement` 容器，`IMenuItem`/`MenuItem`/`render_menu_bar_from_items` 已删除 |
| [crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs) | ✓ | 仅 `StatusBarAlign` + `NativeStatusBar` re-export |
| [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs) | ✓ | 导出 `Menu/MenuBar/NativeStatusBar/StatusBarAlign/configure_menu_bar_popup/menu_bar_button` |
| [crates/engine/src/compiler/props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) | ✓ | `MenuBar`/`StatusBar` 的 `items` 注册已移除 |
| [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) | ✓ | `StatusBar` 注册已移除，保留 `NativeStatusBar`/`MenuBar` |
| [crates/engine/src/compiler/menu/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/setters.rs) | ✓ | `bind_setter` 始终返回 `None` |
| [crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs) | ✓ | 新增 `pub fn register_visual_ability<T: IVisualContribution + 'static>()` |
| [crates/core/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) | ✓ | 导出 `register_visual_ability` |

### 业务侧（全部 DONE）

| 文件 | 状态 | 关键内容 |
|------|------|----------|
| [demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs) | ✓ | `MenuViewModel` (id/label/group/order/command/children) + `root`/`leaf`/`child`/`has_children` + `build_popup_menu` 递归构建 |
| [demo/src/shell/status_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/status_view_model.rs) | ✓ | `StatusViewModel` (id/align/order/contribution) + `from_contribution` + `render` + `build_status_view_models` |
| [demo/src/shell/case_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_view_model.rs) | ✓ | `CaseViewModel` (id/name/group/order/uri/contribution) + `from_contribution` + `render` + `build_tree_items` |
| [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) | ✓ | 7 个 `RelayCommand` 字段 + `build_menu_tree()` + `render_menu_bar()` (dropdown_menu 闭包) + `render_status_bar()` (left/right/center) + `project_entries()` + `on_loaded()` 初始化命令 + `ensure_status_ready_registered()` |
| [demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) | ✓ | `<template slot="menu">` + `<template slot="footer">` 经 `<component content={self.render_xxx(_window, cx)} />` 绑定 |
| [demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) | ✓ | 使用 `rml_core::contribution::register_visual_ability` |
| [demo/src/cases/status_bar_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml.rs) | ✓ | `ensure_status_ready_registered()` 函数 |

### 已删除文件（DONE）

- `demo/src/shell/shell_chrome.rs` — 已删除
- `demo/src/shell/menu_shell_contribs.rs` — 已删除
- `demo/src/shell/mod.rs` — 不再引用上述两个文件

## 剩余工作：构建验证

唯一未完成的工作：对 `rust-rml-ui` 和 `rust-rml-demo` 执行构建验证，修复编译错误。

### 实施步骤

#### Step 1：构建 `rust-rml-ui`

```bash
cargo build -p rust-rml-ui 2>&1
```

**预期**：成功（框架侧 cleanup 已完成，仅 deprecation 警告）。

**若失败**：根据编译错误修复 `crates/ui/src/components/menu.rs`、`status_bar.rs`、`lib.rs` 等文件。

#### Step 2：构建 `rust-rml-demo`

```bash
cargo build -p rust-rml-demo 2>&1
```

**潜在风险点**（需在编译错误时排查）：

1. **`main_window.rml.rs` 第 11 行**：`use rml_ui::{ActivityBar, IActivityPanel, VisualActivityPanel};` — 需确认 `IActivityPanel`/`VisualActivityPanel` 从 `rml_ui` 导出
2. **`main_window.rml.rs` 第 13-14 行**：`use crate::lsp::LspClient; use crate::lsp::lsp_explorer_panel::LspExplorerPanel;` — 需确认 `lsp` 模块存在
3. **`main_window.rml.rs` 第 362 行**：`use gpui_component::menu::{DropdownMenu as _, PopupMenu};` — 需确认 `DropdownMenu` trait 在 `gpui_component::menu` 中
4. **`main_window.rml.rs` 第 424 行**：`#[computed] pub fn tab_bar_items(&self) -> Vec<Arc<dyn IValue>>` — 需确认 `#[computed]` 宏支持无参数方法
5. **`main_window.rml` 第 19/31 行**：`<component content={self.render_menu_bar(_window, cx)} />` — 需确认 codegen 支持 `self.method(_window, cx)` 表达式语法
6. **`main_window.rml.rs` 第 481-484 行**：`apply_switch_en` 中重新构建 `status` — 需确认 `entries.read().unwrap()` 借用不冲突
7. **`status_view_model.rs` 第 13 行**：`pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);` — `main_window.rml.rs` 第 18 行 import 此别名，需确认路径一致

**修复策略**：逐个错误读取上下文 → 修复 → 重新构建，直到通过。

#### Step 3：运行测试（可选）

```bash
cargo test -p rust-rml-engine --test props_registry_complete 2>&1
```

验证 `MenuBar`/`StatusBar` 的 `items` 属性未注册的测试仍通过。

#### Step 4：最终验证

- `cargo build -p rust-rml-demo` 成功（仅 deprecation 警告）
- 如有运行时验证条件：启动 demo，确认菜单栏（File/View/Help + 子菜单）+ 状态栏（left/right/center 项）正常渲染

## 假设与决策

### 决策 1：不重新设计已完成的 MVVM 实现

前序会话已选定**命令式 `render_menu_bar()` + `render_status_bar()` + slot/template** 方案（而非声明式 `each` 指令），理由：
- 状态栏需 left/right/center 三向对齐，`each` 无法表达
- 菜单 `dropdown_menu` 闭包要求 `'static` bound，嵌套 `each` codegen 复杂

本计划**不改动此决策**，仅修复编译错误。

### 决策 2：保留手工 `build_menu_tree()` 而非贡献注册

菜单树整体硬编码（标签经 `t_static()` 获取 i18n），消除 323 行 `menu_shell_contribs.rs` 样板。`RelayCommand` 字段持有 `Arc<dyn ICommand>`，叶子节点命令直接绑定。

### 假设

- `rust-rml-core` / `rust-rml-engine` 构建状态在前序会话已验证通过
- demo crate 的 `lsp` 模块结构未在本会话改动
- `#[window]` / `#[contributehost]` / `#[computed]` / `#[command]` 宏行为符合既有约定

## 验证清单

- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] 编译错误全部修复（如有）
- [ ] `cargo test -p rust-rml-engine --test props_registry_complete` 通过
- [ ] MVVM 数据流：cases/status/activities 经贡献投影，menus 经 `build_menu_tree()` 手工构建
- [ ] Slot/template：`<template slot="menu">` + `<template slot="footer">` 经 `<component content={...} />` 绑定
