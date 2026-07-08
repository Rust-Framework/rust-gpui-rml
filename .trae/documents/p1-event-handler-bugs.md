# P1 事件处理器传递循环变量 — 待解决 Bug 列表

> 本文档细化 [rml-iteration-plan.md](./rml-iteration-plan.md) §2.3（P1）为可执行的 Bug 工单。
> 范围：RML 事件处理器 `on-click={method, {arg}}` 语法在 codegen 各路径的缺陷。
> 目标：修复后 `welcome_case` 可回退为声明式 `each` + 事件传参，消除命令式 `render_group` 绕过。

## 背景

RML 事件处理器三种形式（`crates/engine/src/parser/ast.rs::EventHandler`）：

| 形式 | 语法 | 签名约定 |
|------|------|---------|
| `Ident` | `on-click={method}` | `fn(&mut self, &Event, &mut Context<Self>)` |
| `MethodName` | `on-click={method}`（同上，语义等价） | 同上 |
| `WithArgs` | `on-click={method, {arg1}, 'literal'}}` | `fn(&mut self, arg: T, &Event, &mut Context<Self>)` |

`WithArgs` 由 `parse_event_handler`（`parser/mod.rs:333`）按逗号分割解析，codegen 由 `apply_event`（`event.rs:90`）和 `component_event_setter`（`component.rs:752`）两条路径处理。两条路径均存在缺陷。

## Bug 列表

### BUG-P1-01 `WithArgs` 参数表达式未经 `gen_expr_code` 处理，缺失 `self.`/`__rml_self_ref.` 前缀替换

- **严重度**：高（直接导致编译失败）
- **位置**：
  - `crates/engine/src/compiler/event.rs:169`（slot 路径：`let p0 = {arg}.clone();`）
  - `crates/engine/src/compiler/event.rs:183`（非 slot 路径）
  - `crates/engine/src/compiler/component.rs:776`（Pagination）
  - `crates/engine/src/compiler/component.rs:802`（RadioGroup）
  - `crates/engine/src/compiler/component.rs:830`（Checkbox/Switch/Radio）
  - `crates/engine/src/compiler/component.rs:854`（Button 等普通组件）
- **现象**：参数表达式 `arg` 以字符串原样拼入 `let p0 = {arg}.clone();`。当 `arg` 为 ViewModel 字段（如 `selected_id`）时：
  - 顶层 render 应生成 `let p0 = self.selected_id.clone();`，实际生成 `let p0 = selected_id.clone();` → E0425（unresolved identifier）
  - slot 闭包内应生成 `let p0 = __rml_self_ref.selected_id.clone();`，实际生成 `let p0 = selected_id.clone();` → E0425
- **根因**：`parse_event_handler` 将参数以 `String` 形式存入 `WithArgs.args`，codegen 直接 `format!("let p0 = {arg}.clone();")` 拼接，未调用 `expr::parse(&arg)` + `expr::to_rust_code_with_ctx(&parsed, &loop_vars)` 走前缀替换路径（`expr.rs:194`）。
- **循环变量场景**：`arg = "item.id"`（loop_var）应生成 `let p0 = item.id.clone();`，恰好正确（loop_var 不加前缀）。但只要 `arg` 涉及 ViewModel 字段（如 `arg = "current_case_id"`），即失败。
- **修复方向**：
  ```rust
  // 替换原 format!("let p0 = {arg}.clone();")
  let arg_expr = expr::parse(arg).map_err(|e| ...)?;
  let arg_code = expr::to_rust_code_with_ctx(&arg_expr, &loop_vars);
  format!("let p0 = {arg_code}.clone();")
  ```
  注意 `apply_event` 当前签名 `apply_event(name, handler, ctx)` 未接收 `loop_vars`，需扩展签名透传循环变量上下文。

---

### BUG-P1-02 `component_event_setter` 完全不处理 slot 上下文，组件事件在 slot 闭包内必失败

- **严重度**：高（组件 + slot 组合不可用）
- **位置**：`crates/engine/src/compiler/component.rs:752-862`（整个 `component_event_setter` 函数）
- **现象**：Button/Checkbox/Switch/Radio/Pagination/RadioGroup 的 `on_click` 永远生成 `cx.listener(move |this, ev, _window, cx| { ... })` 形式，不检测 `in_slot_context()`。
- **根因**：`apply_event`（`event.rs:24-26`）通过 `in_slot_context()` 检测 slot 上下文并切换为 `__rml_self_entity.update(cx, |this, cx| { ... })` 形式；但 `component_event_setter` 是独立的组件事件 codegen 路径，没有对应分支。slot 闭包内 `cx: &mut gpui::App`，`cx.listener` 不可用 → E0599（method not found）。
- **影响范围**：不仅是 `WithArgs`，`Ident`/`MethodName` 形式在"组件 + slot"组合下同样失败。当前 demo 未触发因 welcome_case 用命令式 `render_group` 绕过，未将 Button 等组件放入 slot。
- **复现**：在任意 `<template slot="...">` 内放 `<Button on-click={handle} />`，编译失败。
- **修复方向**：在 `component_event_setter` 每个 match 分支引入 `let slot = crate::compiler::event::in_slot_context();` 判断，slot 路径改用 `__rml_self_entity.update(...)` 形式（参照 `apply_event` 的 slot 分支）。建议抽取共用 codegen helper 避免重复。

---

### BUG-P1-03 方法签名参数顺序非标准：`arg` 在 `event` 之前

- **严重度**：中（API 设计异味，不直接导致编译失败）
- **位置**：
  - `event.rs:171`：`this.{method}(p0, &rml_ev, cx);`
  - `event.rs:184`：`this.{method}(p0, &rml_ev, cx);`
  - `component.rs:777/803/831/855`：`this.{method}(p0, page, cx);` / `this.{method}(p0, idx, cx);` / `this.{method}(p0, checked, cx);` / `this.{method}(p0, &rml_ev, cx);`
- **现象**：codegen 强制方法签名为 `fn method(&mut self, arg: T, ev: &Event, cx: &mut Context<Self>)`，额外参数 `arg` 置于 `event` 之前。
- **问题**：
  1. 与 GPUI/Rust 生态约定（event 在前、cx 在后）不一致，开发者需记忆特殊顺序
  2. 同一方法绑定到不同组件（ClickEvent/usize/bool）需写不同签名，无法复用
  3. 多参数场景（BUG-P1-05 扩展后）顺序歧义更大
- **`#[command]` 宏无关性**：已验证 `macros/src/command.rs::expand` 仅要求 `&self`/`&mut self` 为首参，不限制后续参数顺序与数量。故这是 codegen 约定问题，非宏限制。
- **修复方向**：统一调整为 `fn method(&mut self, ev: &Event, arg: T, cx: &mut Context<Self>)`，codegen 改为 `this.method(&rml_ev, p0, cx);`。破坏性变更，需配套 demo 与文档更新。

---

### BUG-P1-04 悬停事件（`on-hover`/`on-mouse-enter`/`on-mouse-leave`）静默丢弃 `WithArgs` 参数

- **严重度**：中（静默丢失用户输入）
- **位置**：`crates/engine/src/compiler/event.rs:252-256`（`apply_hover_event`）
- **现象**：
  ```rust
  let method = match handler {
      EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
      EventHandler::WithArgs(m, _) => m,  // args 被丢弃
  };
  ```
- **固化测试**：`event.rs:587-593` 的 `apply_hover_event_with_args_uses_method_only` 断言 `!code.contains("p0")`，将此静默丢弃行为固化为"预期特性"。
- **问题**：用户写 `on-hover={on_hover_change, {extra}}` 不会有任何编译期或运行期警告，`extra` 被静默忽略，难以调试。违反"静默失败"反模式。
- **修复方向**：二选一
  - 方案 A：实现 hover 事件的 `WithArgs` codegen（与 `apply_event` 对齐）
  - 方案 B：在 `parse_event_handler` 或 codegen 阶段对 hover + WithArgs 组合 emit warning（通过 `CodegenCtx` 收集诊断），明确告知用户参数被忽略
- **配套**：若选方案 B，删除/改写 `apply_hover_event_with_args_uses_method_only` 测试。

---

### BUG-P1-05 `WithArgs` 仅取第一个参数，多参数静默丢失

- **严重度**：低（当前无用户场景，但为隐患）
- **位置**：`event.rs:161`（`// Phase B-1 简化：仅支持单参数`）、`event.rs:162`（`let arg = &args[0];`）
- **现象**：`on-click={open, {arg1}, {arg2}}` 中 `arg2` 被静默丢弃，codegen 只生成 `let p0 = arg1.clone(); this.open(p0, &rml_ev, cx);`。
- **问题**：注释明确标记为"Phase B-1 简化"，但无后续 Phase 跟进项，且无 warning 提示用户。
- **修复方向**：二选一
  - 方案 A：扩展为多参数 codegen，生成 `let p0 = ...; let p1 = ...; this.method(p0, p1, &rml_ev, cx);`
  - 方案 B：解析期对 `args.len() > 1` emit error（明确不支持），而非静默丢失
- **建议**：方案 B 更保守，避免 codegen 复杂度膨胀；多参数场景可由用户在 ViewModel 内构造 struct 传入。

---

### BUG-P1-06 `WithArgs` 语法无文档、无 demo、无 E2E 测试覆盖

- **严重度**：中（feature 不可发现、不可靠）
- **现状**：
  - parser 支持（`parse_event_handler`）
  - codegen 部分支持（`apply_event` + `component_event_setter`）
  - 无 demo case 使用此语法（`welcome_case` 用命令式 `render_group` 绕过）
  - 用户文档未记载
  - 单元测试仅覆盖 `apply_event` 的 `WithArgs` 分支（`event.rs` tests），未覆盖 slot 上下文 + WithArgs 组合、组件 + WithArgs 组合
- **问题**：BUG-P1-01 ~ BUG-P1-05 长期未暴露，部分原因即缺少 E2E 验证。
- **修复方向**：修复上述 bug 后，新增：
  1. `event_with_args_case`：演示 `on-click={open, {item.id}}` 在 `each` 循环内绑定到原生 `div`
  2. `event_with_args_in_slot_case`：演示在 `<template slot>` 内绑定事件传参
  3. `event_with_args_component_case`：演示 Button/Pagination/RadioGroup + WithArgs
  4. 更新用户文档（`docs/` 下事件绑定章节）说明语法与签名约定

---

### BUG-P1-07 `welcome_case` 因上述 bug 被迫保留命令式 `render_group`

- **严重度**：阻塞 MVVM 一等优先级约束
- **位置**：
  - `demo/src/cases/welcome_case.rml`：`<component each={group in grouped_items} content={self.render_group(group, _window, cx)} />`
  - `demo/src/cases/welcome_case.rml.rs:129-181`：`render_group` 用 `Card::new(...).on_click(cx.listener(move |this, _, _, cx| { this.open_case(case_id, cx); }))` 命令式构造
- **现象**：分组卡片渲染 + 点击打开案例的逻辑全部命令式，违反 `.rml` + `.rml.rs` MVVM 约束。
- **根因**：BUG-P1-01（参数前缀替换缺失）+ BUG-P1-02（若改用组件路径）共同阻塞声明式实现。即使写 `on-click={open_case, {item.id}}` 也会因前缀替换失败而编译错误。
- **修复方向**：修复 BUG-P1-01 后，welcome_case 可改写为：
  ```xml
  <div each={group in grouped_items}>
      <div>{group.title}</div>
      <div each={item in group.items} on-click={open_case, {item.id}}>
          <Card>{item.title}</Card>
      </div>
  </div>
  ```
  并删除 `render_group` 方法。
- **验证标准**：welcome_case.rml.rs 不再包含 `Card::new`/`div()`/`into_any_element` 等命令式 API；`open_case` 方法保留 `#[command]` 标注。

## 修复优先级建议

| Bug | 修复顺序 | 理由 |
|-----|---------|------|
| BUG-P1-01 | 1 | 直接阻塞 welcome_case 回退；修复后即可验证声明式路径 |
| BUG-P1-02 | 2 | 与 BUG-P1-01 同源（slot 上下文处理），建议同批修复 |
| BUG-P1-07 | 3 | 依赖 BUG-P1-01/02，作为修复后的回归验证 case |
| BUG-P1-06 | 4 | 随 BUG-P1-01/02 修复同步新增 E2E demo |
| BUG-P1-03 | 5 | API 设计调整，破坏性变更，建议单独 PR |
| BUG-P1-04 | 6 | 可与 BUG-P1-05 合并处理（参数语义澄清） |
| BUG-P1-05 | 6 | 同上 |

## 关联

- 迭代计划总览：[rml-iteration-plan.md](./rml-iteration-plan.md) §2.3
- MVVM 一等优先级约束：`project_memory.md` → "MVVM 声明式 UI 约束"
- 已修复的同源问题：`each` 指令在 slot 闭包内的 `self.` 前缀 bug（rml-iteration-plan.md §1.1），本批 Bug 与其同属 codegen 前缀替换缺失家族
