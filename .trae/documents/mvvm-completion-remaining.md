# MVVM 补全剩余计划（B-2 收尾 + B-1 + B-3）

> **本计划目标**：在 Phase A 已完成、Phase B-2 基础设施已铺好但**未收尾**（导致当前 build 失败）的基础上，完成 B-2 收尾、B-1 命令绑定、B-3 事件补全，使 RML 框架的 MVVM 支持达到「功能完备、build 通过、测试覆盖」的稳定形态。
>
> **前置基础**：
> - Phase A（A-1 空 impl 回退 / A-2 demo 清理 / A-3 ObservableVec deprecated）已完成，553 个测试全过
> - Phase B-2 的 AST/Parser/CodegenCtx/`collect_model_converters`/`gen_field_assign_with_converter` 已就位
> - **当前 build 失败**：`observable.rs:155` 仍以旧 3 参数调用 `gen_field_assign_expr`，而 `binding.rs:96` 已改为 4 参数签名
>
> **阅读时长**：约 12 分钟。本计划只写决策与步骤，不写完整代码。

---

## 1. 现状分析

### 1.1 已完成（不要重复）

| 项 | 状态 | 实现位置 |
|---|---|---|
| A-1 ILifecycle 空 impl 回退 | ✅ | `codegen/lifecycle.rs` — 无钩子时生成 `impl ILifecycle for X {}` |
| A-2 demo 手动 ILifecycle 清理 | ✅ | 9 个空 impl 已删，5 个含逻辑的保留 |
| A-3 ObservableVec deprecated | ✅ | `core/src/observable.rs` — `#[deprecated]` 已加 |
| B-2 AST 扩展 | ✅ | `parser/ast.rs` — `Directive::Model { field, converter: Option<String> }` |
| B-2 Parser 解析 `\|` | ✅ | `parser/mod.rs:179-188` — `split_once('\|')` |
| B-2 所有 match 臂更新 | ✅ | `validator.rs` / `node.rs` / `model.rs` / `binder.rs` |
| B-2 CodegenCtx 字段 | ✅ | `compiler/mod.rs:139` — `model_converters: HashMap<String, String>` |
| B-2 `collect_model_converters` | ✅ | `codegen/model.rs` + `codegen/mod.rs` 导出 + `compile()` 调用 |
| B-2 `gen_field_assign_expr` 签名 | ✅ | `codegen/binding.rs:96` — 已加 `converter: Option<&str>` 第 4 参 |
| B-2 `gen_field_assign_with_converter` | ✅ | `codegen/binding.rs:140` — 生成 `convert_back` 调用 |

### 1.2 当前阻塞点（build 失败根因）

**文件**：`crates/engine/src/compiler/codegen/observable.rs:155`

```rust
// 当前（错误，3 参数）：
let assign = gen_field_assign_expr(field, &ty, validation);

// 应改为（4 参数）：
let converter = ctx.model_converters.get(field).map(|s| s.as_str());
let assign = gen_field_assign_expr(field, &ty, validation, converter);
```

**根因**：B-2 改了 `binding.rs` 的 `gen_field_assign_expr` 签名（加了 `converter` 参数），但唯一调用方 `gen_input_state_impl` 未同步更新。这是当前唯一阻塞 build 的点。

### 1.3 待办（B-1 / B-3）

| 项 | 状态 | 范围 |
|---|---|---|
| B-1 声明式 `command={field}` | ❌ 未开始 | `menu/item.rs` 新增 `gen_command_closure` |
| B-3 oninput/onchange + stop_propagation | ❌ 未开始 | `event.rs` + `codegen/observable.rs` |
| 最终验证 | ❌ | `cargo build --workspace` + `cargo test --workspace` |

---

## 2. 分步实施计划

### Step 1：B-2 收尾（解除 build 阻塞）— 高优先

**目标**：让 `gen_input_state_impl` 把 converter 传给 `gen_field_assign_expr`，恢复 build。

**文件**：
- `crates/engine/src/compiler/codegen/observable.rs`

**改动概要**（`gen_input_state_impl` 内 reverse_arms 循环，约 L152-157）：

```rust
let mut reverse_arms = String::new();
for field in &input_fields {
    let ty = ctx.field_types.get(field).cloned().unwrap_or_default();
    let validation = ctx.field_validations.get(field);
    let converter = ctx.model_converters.get(field).map(|s| s.as_str());  // ← 新增
    let assign = gen_field_assign_expr(field, &ty, validation, converter); // ← 加参数
    reverse_arms.push_str(&format!("                \"{}\" => {{ {} }}\n", field, assign));
}
```

**新增测试**（`crates/engine/tests/codegen_two_way_binding_test.rs` 追加）：

1. `model_with_converter_generates_convert_back_call`
   - 构造 `field_types = {"price": "f64"}`、`<input model={price | Currency} />`
   - 验证生成代码包含 `Currency.convert_back(&value.to_string())`（注意：`gen_field_assign_with_converter` 用的是 `{converter}.convert_back`，不是 `Currency::default().convert_back`，需确认 converter 表达式路径）
   - 验证包含 `__rml_bump_version("price")`

2. `model_without_converter_keeps_parse_behavior`
   - 复用现有 `RML_SOURCE_WITH_MODEL`，验证 i32 字段仍走 `match value.parse::<i32>()`，不被 converter 路径劫持

**注意**：`gen_field_assign_with_converter` 生成的代码是 `match {converter}.convert_back(...)`，此处 `converter` 是字符串（如 `"Currency"`），直接作为表达式拼接，要求 `Currency` 在生成代码作用域可见且 `Currency` 是单元结构体（`pub struct Currency;`）。`Currency` 实现了 `IConverter`，但 `convert_back` 是 `&self` 方法，需 `Currency.convert_back(...)` 形式调用（单元结构体字面量直接用名）—— **当前实现正确**，无需 `::default()`。

**验证**：
- `cargo build -p rust-rml-engine` 通过
- 新增 2 个测试通过

**风险**：低。仅一处调用同步，加一个 `Option<&str>` 参数。

---

### Step 2：B-1 声明式命令绑定 `command={field}`

**目标**：在 `.rml` 中支持 `<MenuItem command={save_command} />`，codegen 生成 `.on_click` 闭包，闭包内调用 `ICommand::execute`。

**文件**：
- `crates/engine/src/compiler/menu/item.rs` — 新增 `gen_command_closure` + 在 `gen_menu_item_stmt` 两处 `MenuItem` 分支接入

**改动概要**：

1. **新增 `gen_command_closure` 函数**（item.rs，与 `gen_onclick_closure` 并列）：

```rust
fn gen_command_closure(cmd_expr: &str) -> Result<String, CodegenError> {
    // 生成代码：
    // .on_click({
    //     let weak = __rml_menu_weak.clone();
    //     move |_ev, window, app| {
    //         if let Some(entity) = weak.upgrade() {
    //             entity.update(app, |this, cx| {
    //                 let cmd = &this.{cmd_expr};       // 读 ViewModel 上的命令字段
    //                 let mut ctx = rml_core::command::CallContext::new(window, cx);
    //                 if cmd.can_execute(&mut ctx) {
    //                     cmd.execute(&mut ctx);
    //                 }
    //             });
    //         }
    //     }
    // })
    //
    // cmd_expr 可能是 self.field 或 self.field()（computed），统一用 self.{expr} 访问
}
```

   **关键决策**：
   - `can_execute` 先判断再 `execute`（对齐 WPF `ICommand` 语义）
   - 闭包参数签名 `move |_ev, window, app|` — 第三参数 GPUI on_click 是 `&mut App`（参考现有 `gen_onclick_closure` 的 `move |ev, _window, app|`），但 `CallContext::new` 需要 `&mut Window`，所以 `window` 不能加下划线前缀，需借用
   - `cmd_expr` 来源：`bind_attr(elem, "command", ...)` 返回的 rust 表达式（可能是 `self.save_command` 或 `self.save_command()`）

2. **在 `gen_menu_item_stmt` 接入**（item.rs 两处 `MenuItem` 分支）：

   - L97 `!custom_children.is_empty()` 分支：在 `gen_onclick_closure` 之前/之后检测 `command` bind 属性
   - L137 默认 `MenuItem` 分支：同上

   接入逻辑：
   ```rust
   // 优先 command，回退 onclick
   let onclick = if let Some(cmd_expr) = bind_attr(elem, "command", loop_vars, ctx, hoist)? {
       gen_command_closure(&cmd_expr)?
   } else {
       gen_onclick_closure(elem, ctx)?
   };
   ```

   **决策**：`command` 与 `onclick` 同时存在时，`command` 优先，`onclick` 被忽略（不发出 warning，保持简洁；计划 §4 已记录此决策）。

3. **不扩展 `Attribute::Bind`**：`command={field}` 走现有 `Attribute::Bind { name: "command", expr: "field" }` 路径，`bind_attr` helper 已能识别，无需新增 AST 节点。

**新增测试**（`crates/engine/src/compiler/menu/item.rs` tests 模块，或新建 `tests/codegen_command_binding_test.rs`）：

1. `command_attr_generates_execute_call`
   - `<MenuItem command={save_command} label="Save" />`
   - 验证生成代码包含 `this.save_command`、`can_execute`、`execute`、`CallContext::new`

2. `command_takes_precedence_over_onclick`
   - `<MenuItem command={save} onclick={legacy} label="Save" />`
   - 验证生成代码包含 `execute`，不包含 `this.legacy`

3. `menu_item_without_command_uses_onclick`
   - 现有 `onclick={method}` 仍走 `gen_onclick_closure`，不被 command 路径劫持

**验证**：
- `cargo test -p rust-rml-engine` 通过
- demo 暂不接入（B-1 的 demo 验证留到 Step 4 一起做）

**风险**：中。`WeakEntity` + `update` 借用模式已由 `gen_onclick_closure` 验证可行；`CallContext::new(window, cx)` 的 `window` 借用需注意：`update` 闭包参数是 `(&mut T, &mut Context<T>)`，`Context<T>` 不是 `Window`，无法直接构造 `CallContext`。

   **⚠️ 阻塞点**：`CallContext::new(window: &'a mut Window, app: &'a mut App)`，但 `entity.update(app, |this, cx| { ... })` 的闭包内只有 `&mut Context<T>`，没有 `&mut Window`。GPUI 的 `entity.update` 有两个签名：
   - `update(&self, cx: &mut App, f: impl FnOnce(&mut T, &mut Context<T>))` — 无 window
   - `update(&mut self, window: &mut Window, cx: &mut App, f: impl FnOnce(&mut T, &mut Window, &mut Context<T>))` — 有 window（Entity 上）

   **修正方案**：用 `entity.update(app, |this, cx| { ... })` 签名，但 `CallContext` 需要 window。两个选择：
   - **方案 A（推荐）**：闭包外层用 `entity.update(window, app, |this, window, cx| { ... })` — 但这是 `Entity<T>::update(&mut self, window, cx, f)`，需要 `entity` 是 `&mut Entity<T>`；而我们拿到的是 `weak.upgrade()` 返回 `Option<Entity<T>>`（owned），可以 `entity.update(window, app, ...)`。**需核对 GPUI API**。
   - **方案 B**：`CallContext` 不强制要 window，命令闭包内不调用需要 window 的命令（大多数 ViewModel 命令只用 `cx.notify`/`cx.emit`）。

   **决策**：采用方案 A，生成代码用 `entity.update(window, app, |this, window, cx| { ... })` 签名，把 `window` 传给 `CallContext::new(window, cx)`。若 GPUI 无此签名，回退方案 B（`CallContext::new` 用 `cx.window()` 或类似 API，需在执行时核对）。

   **执行时核对**：先 grep `gpui` crate 的 `Entity::update` 签名，确认方案 A 可行；不可行则按方案 B 调整 `gen_command_closure`。

---

### Step 3：B-3 oninput/onchange + stop_propagation

**目标**：
1. 让 `<input oninput={handler} />` 生效
2. 让 `rml_ev.stop_propagation()` 真正阻止事件冒泡

**文件**：
- `crates/engine/src/compiler/event.rs` — stop_propagation 检查注入
- `crates/engine/src/compiler/codegen/observable.rs` — oninput/onchange 在 `gen_model_input` 中接入用户 handler

**改动概要**：

#### 3a. stop_propagation 生效（低风险，先做）

修改 `apply_event`（event.rs:90-120）生成的闭包，在 `this.{method}(&rml_ev, cx)` 之后追加 `if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }`：

```rust
// 改前（EventHandler::Ident / MethodName 分支）：
format!(
    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n    \
     let rml_ev = {};\n    this.{}(&rml_ev, cx);\n                }}))",
    on_method, gpui_type, convert_expr, method
)

// 改后：
format!(
    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n    \
     let rml_ev = {};\n    this.{}(&rml_ev, cx);\n    \
     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
    on_method, gpui_type, convert_expr, method
)
```

   `EventHandler::WithArgs` 的两个分支（args 空/非空）同样追加。

   **新增测试**（event.rs tests 模块）：
   - `stop_propagation_check_injected` — 验证 onclick 生成代码包含 `if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }`
   - 现有 `apply_event_onclick_ident` 等测试需更新断言（不破坏现有断言，仅追加新断言）

#### 3b. oninput/onchange 接入（中风险，后做）

**设计决策**：oninput/onchange 仅对 `<input>` 有意义，而 `<input model={field}>` 已通过 `cx.subscribe(&entity, ...)` 订阅 `InputEvent::Change` 实现反向绑定。若用户同时声明 `model={field}` 和 `oninput={handler}`，handler 应在 model 同步**之后**触发（计划 §4 决策 4）。

**实现路径**：不通过 `event_binding` 路由（GPUI 元素无 `on_input` 方法），而是在 `gen_model_input`（binding.rs:19）中检测 `oninput`/`onchange` 属性，生成附加代码：

```rust
// gen_model_input 内，在 Input::new(...) 构造后、wrapper 之前：
let oninput_handler = elem.attributes.iter().find_map(|a| match a {
    Attribute::Event { name, handler } if name == "oninput" => Some(handler),
    _ => None,
});
```

   由于 `InputState` 的 `cx.subscribe` 回调已在 `__rml_get_or_init_input_state` 内部生成，oninput handler 需要**注入到该回调内**，在 `cx.notify()` 之前调用。

   **简化方案（推荐）**：在 `gen_input_state_impl` 的 `cx.subscribe` 回调内，`InputEvent::Change` 分支末尾追加用户 handler 调用（若有）：

   ```rust
   // 现有回调（observable.rs:179-192）：
   cx.subscribe(&entity, move |this, input_entity, event, cx| {
       match event {
           rml_ui::InputEvent::Change => {
               let value = input_entity.read(cx).value();
               match field {
                   "field1" => { /* 反向赋值 */ }
                   _ => {}
               }
               let v = this.__rml_get_version(field);
               this.__rml_input_state_versions.insert(field.to_string(), v);
               cx.notify();
               // ← 此处注入用户 oninput handler
           }
           _ => {}
       }
   }).detach();
   ```

   **问题**：`gen_input_state_impl` 是**统一生成**的方法，所有 input 字段共用一个 `cx.subscribe` 回调，按 `field` 分派。用户 handler 也是按字段的，需在 match arm 内追加。

   **实现**：扩展 `CodegenCtx` 新增 `model_input_handlers: HashMap<String, (Option<String>, Option<String>)>`（field → (oninput_method, onchange_method)），由 `compile()` 从 AST 收集；`gen_input_state_impl` 在 reverse_arms 生成时，把 handler 调用追加到 arm 末尾。

   **简化决策**：本期 oninput/onchange 仅支持 `<input>` 元素（与 `model` 共存），不支持其他元素。若 `<div oninput={...}>` 仍返回空字符串（不实现）。

   **新增测试**：
   - `oninput_handler_injected_into_subscribe_callback` — 验证生成代码在 `cx.notify()` 之前/之后包含 `this.handler(&rml_ev, cx)`
   - `onchange_handler_separate_from_oninput` — 两者可独立声明

**验证**：
- `cargo test -p rust-rml-engine` 通过
- 手动验证（Step 4）

**风险**：中。oninput 与 model 双向绑定可能重复触发 — 缓解：handler 在 `cx.notify()` 之前调用，用户读取的是已同步的字段值；若用户在 handler 内再次修改字段，会触发下一轮 `cx.notify`，行为可预期。

---

### Step 4：最终验证 + demo 接入

**目标**：全量 build + test，并在 demo 中添加 B-1/B-2/B-3 使用示例。

**步骤**：

1. `cargo build --workspace` — 无错误
2. `cargo test --workspace` — 全过（553 + 新增 ≈ 8 个 = ≈561 个测试，0 失败，27 ignored 不变）
3. demo 接入：
   - **B-2**：在 `demo/src/cases/counter_case.rml.rs` 或新增 `converter_case` 中，添加 `<input model={price | Currency} />` 示例
   - **B-1**：在 `demo/src/shell/main_window.rml` 的菜单中，添加 `<MenuItem command={save_command} label="Save" />`（需在 MainWindow struct 添加 `save_command: Arc<dyn ICommand>` 字段，在 `on_loaded` 中用 `RelayCommand::new` 初始化）
   - **B-3**：在 demo 某 input 上添加 `oninput={handle_input}` 示例
4. 手动运行 demo，验证：
   - B-2：输入 `¥1500.00` → 反向转换为 `1500.0`，VM 字段更新
   - B-1：点击 Save 菜单项 → 命令执行
   - B-3：输入时 handler 触发；handler 内 `rml_ev.stop_propagation()` 阻止冒泡
5. 文档更新：在 `docs/09-architecture/observable-refactor-plan.md` 追加 B-1/B-2/B-3 完成状态

---

## 3. 依赖关系图

```
Step 1（B-2 收尾）─→ 解除 build 阻塞
                   │
                   ├─→ Step 2（B-1）─→ Step 4
                   │
                   └─→ Step 3a（stop_propagation）─→ Step 3b（oninput/onchange）─→ Step 4
```

**推荐执行顺序**：
1. Step 1（B-2 收尾）— 先解除 build 阻塞
2. Step 3a（stop_propagation）— 低风险，独立改动
3. Step 2（B-1）— 需核对 GPUI `Entity::update` 签名
4. Step 3b（oninput/onchange）— 依赖对 `gen_input_state_impl` 的理解
5. Step 4（最终验证 + demo）

---

## 4. 假设与决策

### 4.1 假设

- `Currency` 等内置 converter 在生成代码作用域可见（用户 `use rml_core::converter::*` 或全路径 `rml_core::converter::Currency`）
  - **风险**：若用户未 import，生成代码 `Currency.convert_back(...)` 会编译失败
  - **缓解**：codegen 生成全路径 `rml_core::converter::Currency.convert_back(...)` — 但 `gen_field_assign_with_converter` 当前用裸 `{converter}` 字符串拼接，**执行时需改为全路径或要求用户 import**
  - **决策**：本期保持裸 converter 名（`Currency`），文档要求用户在 `.rml.rs` 中 `use rml_core::converter::Currency;`；未来可扩展为 codegen 自动加全路径
- GPUI `Entity<T>::update` 有 `(&mut Window, &mut App)` 签名 — 需在 Step 2 执行时核对
- oninput handler 签名为 `fn(&InputEvent, &mut Context<Self>)`，与 onclick handler 签名模式一致

### 4.2 决策

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| 1 | B-2 converter 失败错误消息 | "转换失败" | 通用消息，未来可扩展为 converter 自定义 |
| 2 | B-1 `command` 与 `onclick` 同时存在 | command 优先，onclick 静默忽略 | 声明式优于命令式；不发出 warning 保持简洁 |
| 3 | B-1 `can_execute` 是否绑定 `disabled` 属性 | 否（本期） | 复杂度较高，延迟到 Phase C |
| 4 | B-3 oninput 与 model 同时存在时触发顺序 | model 反向同步先，oninput handler 后，cx.notify 最后 | 用户期望在 handler 内读取已同步的字段值 |
| 5 | B-3 oninput/onchange 是否支持非 `<input>` 元素 | 否 | GPUI 元素级无对应方法；本期仅 `<input>` 接入 |
| 6 | stop_propagation 是否对 hover 事件生效 | 否（本期） | hover 事件走 `apply_hover_event` 独立路径，不注入检查；未来可扩展 |
| 7 | converter 名是否生成全路径 | 否（本期用裸名） | 文档要求用户 import；未来扩展 |

---

## 5. 验证准则

每个 Step 完成后需满足：

1. **编译验证**：`cargo build -p rust-rml-engine` 无错误（Step 1 后扩展为 `cargo build --workspace`）
2. **测试验证**：`cargo test -p rust-rml-engine` 全过，新增测试覆盖改动点
3. **回归验证**：现有 553 个测试无回归
4. **最终验证**（Step 4）：`cargo build --workspace` + `cargo test --workspace` 全过，demo 手动验证

---

## 6. 不在范围内

- Phase C（onfocus/onblur、Capture/Bubble 三阶段、onsubmit/onload/onresize/onscroll）— 延迟实施
- converter 全路径自动生成（本期文档要求用户 import）
- `can_execute` 绑定 `disabled` 属性（延迟到 Phase C）
- 性能优化（如部分重渲）
- 热重载支持
- 运行时反应式系统（Proxy/反射）
