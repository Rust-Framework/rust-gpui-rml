# IWindow 自管理 API —— 收尾与验证计划

## 摘要

本计划是先前会话的延续。先前已完成 Phase 1–5（核心源码改造）与 Phase 6 Batch A/B/C（文档 `#[view]` → `#[component]` 批量替换）。本计划仅覆盖剩余收尾工作：

- **Phase 6 Batch D**：`docs/09-architecture/responsibility.md` 的两项编辑（批量替换 + 定向 run-pattern 更新）
- **Phase 6 验证**：Grep 确认 `docs/` 中 `#[view]` 与 `IRmlView` 均为 0 处
- **Phase 7 全量验证**：`cargo build --workspace` / `cargo test --workspace` / `cargo run -p rust-rml-demo`

用户核心诉求（已在前序会话落地于源码）：
1. `RmlApplication.main_window` 必须是 `IWindow` 类型组件
2. `RmlApplicationExt` 不应存在——`main_window` 是内置功能
3. 定义抽象接口 `IComponent` 与 `IWindow`，参考 WPF/MAUI 设计
4. `IWindow` 自管理窗口操作（`open`/`show`/`close`/`state`），无需扩展
5. 充分发挥 Rust + GPUI + gpui-component 特性

## 当前状态分析（Phase 1 探索已确认）

### 源码层（已完成，无需改动）

- `crates/core/src/window.rs`：`IWindow` trait，含 `close`/`show`/`hide`/`activate`/`state`/`chrome`/`window_options` 默认实现，均基于 `handle()` 调用 GPUI API。必需方法仅 6 个：`title`/`width`/`height`/`handle`/`set_handle`/`open`。trait 层级 `IModel → ILifecycle → IViewModel → IComponent → IWindow`。
- `crates/core/src/component.rs`：`IComponent` trait（合并自旧 `IRmlView`），含 `rml_template()` 与 `rml_tag()`。
- `crates/app/src/application.rs`：`RmlApplication<W>` 类型状态模式，`NoWindow` 标记类型，内置 `main_window::<NewW>()` 方法。无 `RmlApplicationExt`（源码与文档均确认 0 处）。
- `crates/macros/src/window.rs`：`#[window]` 宏仅生成核心方法（title/width/height/open/handle/set_handle）+ 句柄字段；窗口操作由 trait 默认实现提供。
- `crates/ui/src/window/builtin_window.rs`：内置 `Window` 与 `ModernWindow`，手动 impl 完整 trait 层级（IModel/ILifecycle/IViewModel/IComponent/IWindow），使用 `String` 而非 `SharedString` 以满足 `Send + Sync`。

### 文档层（Phase 6 剩余）

- `docs/` 中 `#[view]` 残留：**9 处，全部位于 `docs/09-architecture/responsibility.md`**（行 L32 / L73 / L100 / L142 / L152 / L158 / L191 / L208 / L285）。
- `docs/` 中 `IRmlView` / `rml_view` 残留：**0 处** ✅
- `docs/` 中 `RmlApplicationExt` 残留：**0 处** ✅
- 旧启动模式 `run::<MyViewModel>()` 残留：**1 处**，位于 `responsibility.md:229`。
- 其他文档（`quick-start.md`、`macros.md`、`viewmodel-structure.md`、`state-management.md`）已正确使用 `main_window::<...>().run()` 模式 ✅。

## 待办变更

### 变更 1：`docs/09-architecture/responsibility.md` —— 批量替换 `#[view]` → `#[component]`

- **文件**：`e:\GitCode\RF\rust-gpui-rml\docs\09-architecture\responsibility.md`
- **操作**：`Edit`（`replace_all = true`），`#[view]` → `#[component]`
- **影响**：9 处（L32 / L73 / L100 / L142 / L152 / L158 / L191 / L208 / L285），含 prose 反引号引用、代码注释、代码属性。
- **为什么**：与已完成的 Batch A/B/C 保持一致；`#[view]` 宏已废弃，统一为 `#[component]`。
- **安全性**：`#[view]` 是唯一 token，全仓无 `#[view(...)]` 参数化形式，`replace_all` 安全。

### 变更 2：`docs/09-architecture/responsibility.md` —— 更新启动模式

- **文件**：同上
- **操作**：定向 `Edit`（非 replace_all）
- **old_string**：`` - 通过 `RmlApplication::new().run::<MyViewModel>()` 启动根视图 ``
- **new_string**：`` - 通过 `RmlApplication::new().main_window::<MyWindow>().run()` 启动根窗口 ``
- **为什么**：反映新的内置 `main_window` API；`MyViewModel` → `MyWindow` 体现"主窗口必须是 IWindow 类型"的设计决策；"根视图" → "根窗口" 与窗口化定位一致。
- **为什么定向而非 replace_all**：这是唯一一处 run-pattern 残留，且需同时改类型参数名与中文描述。

### 变更 3：Phase 6 验证（只读）

- **操作**：两个 Grep 调用
  - `Grep` pattern=`#\[view\]` path=`docs/` → 期望 **0**
  - `Grep` pattern=`IRmlView|rml_view` path=`docs/` → 期望 **0**
- **为什么**：确认文档与源码命名完全统一，无遗漏。

### 变更 4：Phase 7 全量验证

- **操作**：依次执行
  1. `cargo build --workspace`
  2. `cargo test --workspace`
  3. `cargo run -p rust-rml-demo`
- **为什么**：确认源码改造后编译、测试、运行均通过；demo 验证端到端窗口启动正常。

## 假设与决策

1. **决策**：所有 `#[view]` → `#[component]`（含 prose、注释、属性），与 Batch A/B/C 一致。
2. **决策**：`replace_all` 安全——`#[view]` 是唯一 token，无参数化形式。
3. **决策**：`responsibility.md` 中的 `mod views;` 等模块声明保持不变（非宏属性，不在替换范围）。
4. **决策**：`MyViewModel` → `MyWindow` 作为类型参数名，体现"主窗口必须是 IWindow"。
5. **决策**：历史计划文档（`wpf-style-window-and-application-api-plan.md`）不更新——仅作历史上下文。
6. **假设**：先前 Phase 1–5 源码改造已稳定，无需再改动源码（Phase 1 探索已验证 `window.rs` / `component.rs` / `application.rs` / `macros/window.rs` / `builtin_window.rs` 均符合预期）。

## 验证步骤

1. `Edit`（replace_all）on `responsibility.md`：`#[view]` → `#[component]`（9 处）
2. `Edit`（定向）on `responsibility.md`：run-pattern 更新（L229）
3. `Grep` `#\[view\]` in `docs/` → 期望 **0**
4. `Grep` `IRmlView|rml_view` in `docs/` → 期望 **0**
5. `cargo build --workspace` → 期望成功
6. `cargo test --workspace` → 期望通过
7. `cargo run -p rust-rml-demo` → 期望窗口正常打开

## 执行顺序

变更 1 → 变更 2（同文件，顺序执行避免冲突）→ 变更 3（两个 Grep 可并行）→ 变更 4（build / test / run 顺序执行，后依赖前）。
