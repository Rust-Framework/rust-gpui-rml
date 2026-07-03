# 完成 ICommand : IContribution 统一改造（收尾）

## Summary

承接上一会话的工作。`ICommand : IContribution` 统一改造的 13 步计划中，Step 1–11 已完成，Step 12（更新 demo case 文件）完成 7/12，Step 13（build.rs）无需改动，Step 14（验证）待执行。

本计划收尾剩余工作：修改 5 个 case 文件 + 运行构建/测试验证。

## Current State Analysis

### 已确认完成（无需再动）

- `crates/core/src/command.rs` — `ICommand : IContribution`、`CallContext`、`RelayCommand` 实现 `IContribution`
- `crates/core/src/contribution.rs` — `add_command`/`register_command`，移除 `slot` 字段
- `crates/core/src/prelude.rs` — 导出 `CallContext`/`ICommand`/`RelayCommand`
- `crates/app/src/contribution/host_handle.rs` — `HostOp::AddCommand`、`EntityHostHandle::add_command`
- `crates/app/src/contribution/registry.rs` — `register_command` 实现
- `crates/macros/src/contribute.rs` — 拒绝 `name`/`description`/`icon`，编译期断言 `IContribution`/`ICommand`，按 flag 路由
- `crates/ui/src/components/menu.rs` — `on_click` 改用 `CallContext::new(_window, cx)`
- `demo/src/shell/shell_chrome.rs` — `CommandEntry` 类型别名，`map_menu_items` 合并 `ContribEntry` + `CommandEntry`
- `demo/src/shell/main_window.rml.rs` — `command_entries: RwLock<Vec<CommandEntry>>`，覆写 `add_command`
- `demo/src/shell/menu_shell_contribs.rs` — 14 个结构体重写，8 个叶节点 `impl ICommand`，6 个中间节点仅 `impl IContribution`
- `demo/src/shell/activity_panel.rml.rs` — 手写 `impl IContribution`
- 7 个已完成 case 文件：`accordion_case`、`button_case`、`counter_case`、`i18n_case`、`slot_case`、`two_way_case`、`menu_context_case`

### 待修改（5 个文件，编译将失败）

宏已拒绝 `name = "..."`，下列文件仍带 `name` 参数，必须移除并手写 `impl IContribution`：

| 文件 | 结构体 | i18n key | 备注 |
|------|--------|----------|------|
| `demo/src/cases/menu_custom_case.rml.rs` | `MenuCustomCase` | `case.menu.custom.title` | 单结构体 |
| `demo/src/cases/menu_dropdown_case.rml.rs` | `MenuDropdownCase` | `case.menu.dropdown.title` | 单结构体 |
| `demo/src/cases/menu_editor_case.rml.rs` | `MenuEditorCase` | `case.menu.editor.title` | 单结构体 |
| `demo/src/cases/menu_features_case.rml.rs` | `MenuFeaturesCase` | `case.menu.features.title` | 单结构体 |
| `demo/src/cases/status_bar_case.rml.rs` | `StatusBarCase` + `StatusReady` | `case.status_bar.title` + `shell.status_ready` | **两个结构体**，`StatusReady` 无 `#[component]`，是纯贡献（非 visual 非 command） |

### 标准改造模式（参照 `menu_context_case.rml.rs`）

1. 顶部 imports 增加：
   ```rust
   use gpui::SharedString;
   use rml_core::i18n::t_static;
   ```
   （`rml::prelude::*` 已有则不重复）
2. `#[contribute(...)]` 中删除 `name = "..."` 一行
3. 紧接结构体定义后添加：
   ```rust
   impl IContribution for XxxCase {
       fn id(&self) -> &str { Self::CONTRIBUTION_ID }
       fn name(&self) -> SharedString { t_static("case.xxx.title").into() }
   }
   ```

### `status_bar_case.rml.rs` 特殊处理

该文件含两个 `#[contribute]`：
- `StatusBarCase`（line 3-10）：`#[component]` 叠加，按标准模式处理
- `StatusReady`（line 18）：无 `#[component]`、无 `visual`/`command` flag → 纯贡献路径（宏路由到 `register`）。仍需手写 `impl IContribution`，`name()` 返回 `t_static("shell.status_ready").into()`

## Proposed Changes

### Step 1: 修改 `menu_custom_case.rml.rs`

- 删除 line 6: `name = "case.menu.custom.title",`
- 添加 imports: `use gpui::SharedString;` + `use rml_core::i18n::t_static;`
- 在 `pub struct MenuCustomCase { ... }` 之后、`impl ILifecycle` 之前插入：
  ```rust
  impl IContribution for MenuCustomCase {
      fn id(&self) -> &str { Self::CONTRIBUTION_ID }
      fn name(&self) -> SharedString { t_static("case.menu.custom.title").into() }
  }
  ```

### Step 2: 修改 `menu_dropdown_case.rml.rs`

- 删除 line 6: `name = "case.menu.dropdown.title",`
- 添加 imports
- 插入 `impl IContribution for MenuDropdownCase`，key = `case.menu.dropdown.title`

### Step 3: 修改 `menu_editor_case.rml.rs`

- 删除 line 6: `name = "case.menu.editor.title",`
- 添加 imports
- 插入 `impl IContribution for MenuEditorCase`，key = `case.menu.editor.title`

### Step 4: 修改 `menu_features_case.rml.rs`

- 删除 line 6: `name = "case.menu.features.title",`
- 添加 imports
- 插入 `impl IContribution for MenuFeaturesCase`，key = `case.menu.features.title`

### Step 5: 修改 `status_bar_case.rml.rs`（两个结构体）

- 删除 line 6: `name = "case.status_bar.title",`（`StatusBarCase`）
- 删除 line 18 中: `name = "shell.status_ready",`（`StatusReady`）
- 添加 imports
- 插入两个 `impl IContribution`：
  - `StatusBarCase` → key = `case.status_bar.title`
  - `StatusReady` → key = `shell.status_ready`

### Step 6: 验证

按顺序运行：

1. `cargo build --workspace` — 修复任何编译错误
2. `cargo test --workspace` — 确认测试通过
3. 若有 `dead_code`/`unused import` 警告，按用户偏好（高内聚低耦合、无冗余前缀）清理

**注意**：当前有 2 个后台命令在运行（来自上一会话，可能是之前的 cargo build/test）。不要重新运行相同命令；如需验证，先停止后台任务或在新终端运行。实际操作时先检查后台任务状态，避免冲突。

## Assumptions & Decisions

1. **i18n key 已存在**：经 grep 确认 `case.menu.*.title`、`case.status_bar.title`、`shell.status_ready` 均在 `demo/assets/i18n/en-US.json` 和 `zh-CN.json` 中定义，`t_static` 可正常返回。
2. **`t_static` 返回类型**：参照已完成文件（如 `menu_context_case.rml.rs` line 23），`t_static(...)` 返回的字符串需 `.into()` 转为 `SharedString`。
3. **`StatusReady` 路由**：无 `#[component]`/`visual`/`command` → 宏走默认 `register` 路径（非 `register_visual`/`register_command`）。这与原设计一致——`StatusReady` 是纯数据贡献，由宿主自行渲染。
4. **不扩展范围**：仅完成剩余 5 文件 + 验证，不改动已完成文件，不引入新抽象。
5. **`CONTRIBUTION_ID` 常量**：由宏生成（`contribute.rs` line 315-317），手写 `impl IContribution::id()` 引用 `Self::CONTRIBUTION_ID`，与已完成文件一致。
6. **不触碰 build.rs**：`contribution_generator.rs` 仍按 `host_id` 扫描 `#[contribute]`，参数消减不影响扫描逻辑（仅读取 `host_id`/`id`）。

## Verification Steps

1. `cargo build --workspace` 成功，无错误（警告可接受，但应清理 `unused import`）
2. `cargo test --workspace` 全部通过
3. 抽查：`grep -rn "name = \"" demo/src/cases/` 应无匹配（所有 `name` 参数已移除）
4. 抽查：`grep -rn "impl IContribution for" demo/src/cases/` 应有 12 个匹配（每个 `#[contribute]` 结构体一个）
