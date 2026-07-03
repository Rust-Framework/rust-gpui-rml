# MVVM 完备性补全计划

> **本计划目标**：在 Phase 1（observable 重构）已完成的基础上，补齐 RML 框架在 MVVM 支持上的剩余缺口，使其达到「功能完备、缺口显式化、风险可控」的稳定形态。
>
> **前置基础**：Phase 1 已完成 5 项改进（ComputedCache unsafe 显式化、IBindingContext 扩展、#[on_loaded]/#[on_unloaded] 自动联动、IConverter codegen 验证、debounce 实现），553 个测试全过。
>
> **阅读时长**：约 20 分钟。本计划只写决策与步骤，不写代码。

---

## 1. 现状分析：MVVM 能力矩阵

基于对核心文件的复核，当前 RML 的 MVVM 支持状况如下：

| MVVM 能力 | 状态 | 实现位置 | 缺口说明 |
|---|---|---|---|
| ViewModel（Entity） | ✅ 完备 | `core/src/view_model.rs` | `IViewModel: IModel + ILifecycle` |
| Observable 字段版本号 | ✅ 完备 | `macros/src/component.rs` | `__rml_<field>_version: AtomicU64` |
| `#[command]` 自动 bump+notify | ✅ 完备 | `macros/src/command.rs` | 含 debounce 支持 |
| `#[computed]` 缓存 | ✅ 完备 | `core/src/computed_cache.rs` + `engine/codegen/observable.rs` | sum-of-versions 依赖追踪 |
| 双向绑定 InputState | ⚠️ 半成品 | `engine/codegen/binding.rs` + `observable.rs` | **convert_back 未接入**，仅 to_string/parse |
| `IConverter` 单向转换 | ✅ 完备 | `engine/src/compiler/expr.rs` | `|` 管道符 codegen 已贯通 |
| `#[on_loaded]`/`#[on_unloaded]` | ⚠️ 半成品 | `engine/codegen/lifecycle.rs` | **无钩子时返回空字符串**，强迫手动 impl |
| `ICommand` 命令系统 | ⚠️ 半成品 | `core/src/command.rs` + `ui/src/components/menu.rs` | 运行时可用，**声明式 `command={field}` 未实现** |
| 事件绑定 onclick/onkeydown 等 | ✅ 完备 | `engine/src/compiler/event.rs` | 8 类标准事件 + 3 类悬停事件 |
| oninput/onchange 事件 | ❌ 缺失 | `engine/src/compiler/event.rs:63` | 返回 None，未实现 |
| onfocus/onblur 事件 | ❌ 缺失 | `engine/src/compiler/event.rs:63` | GPUI 元素级无对应方法 |
| onsubmit/onload/onresize/onscroll | ❌ 缺失 | `engine/src/compiler/event.rs:63` | 返回 None |
| `IEvent::stop_propagation()` | ❌ 失效 | `engine/runtime/event_flow.rs` | 标志位无调度器读取 |
| `ObservableVec<T>` | ⚠️ 孤立 | `core/src/observable.rs` | 独立版本机制，与 `__rml_<field>_version` 脱节，demo 零使用 |

### 1.1 关键缺口详述

**缺口 1：ILifecycle 空实现陷阱**
- 文件：`crates/engine/src/compiler/codegen/lifecycle.rs:20-23`
- 现状：`gen_lifecycle_impl` 在 `!ctx.lifecycle_hooks.has_any()` 时返回空字符串
- 后果：codegen 不生成 `impl ILifecycle for X`，但 `IViewModel: ILifecycle`，用户必须手动写 `impl ILifecycle for X {}`（demo 中有 16+ 处）
- 修复：无钩子时生成空 `impl ILifecycle for X {}`

**缺口 2：声明式命令绑定缺失**
- 文件：`crates/engine/src/compiler/menu/item.rs:169-185`
- 现状：`gen_onclick_closure` 只处理 `onclick={method}`，不识别 `command={field}` 属性
- 后果：用户无法在 `.rml` 中声明 `<MenuItem command={save} />`，必须写 `onclick={save_command}`
- 修复：新增 `gen_command_closure`，识别 `command` 属性，生成 `RelayCommand::execute` 调用

**缺口 3：双向绑定 convert_back 未接入**
- 文件：`crates/engine/src/parser/ast.rs:76` + `crates/engine/src/compiler/codegen/binding.rs`
- 现状：`Directive::Model(String)` 只存字段名，不解析 `|` 管道；`gen_field_assign_expr` 只做 parse/to_string
- 后果：6 个内置 converter 都实现了 `convert_back` 但从未被调用；`<input model={price | Currency} />` 的反向转换失效
- 修复：扩展 `Directive::Model` 结构体字段，parser 解析 `|`，codegen 调用 `convert_back`

**缺口 4：oninput/onchange 事件未实现**
- 文件：`crates/engine/src/compiler/event.rs:63`
- 现状：`event_binding` 对 `oninput`/`onchange` 等返回 None
- 后果：用户无法在 `.rml` 中写 `<input oninput={handle_input} />`
- 修复：GPUI InputState 已有 `on_change` 回调（cx.subscribe 模式），codegen 生成订阅代码

**缺口 5：stop_propagation 失效**
- 文件：`crates/engine/src/compiler/event.rs:96-100`
- 现状：生成的闭包调用 `this.method(&rml_ev, cx)` 后不检查 `is_propagation_stopped()`
- 后果：用户在事件处理器中调用 `rml_ev.stop_propagation()` 无效
- 修复：在 `apply_event` 生成的闭包末尾添加 `if rml_ev.is_propagation_stopped() { cx.stop_propagation(); }`

**缺口 6：ObservableVec 机制孤立**
- 文件：`crates/core/src/observable.rs`
- 现状：独立 `version: AtomicU64` + `flume::Sender` 机制，与 `__rml_<field>_version` 完全脱节
- 后果：demo 零使用，维护成本高，与框架主流机制不一致
- 修复：标记 `#[deprecated]`，文档引导使用 `Vec<T>` + `#[command]` 自动 bump

---

## 2. 分阶段实施计划

### Phase A：低风险框架清理（3 项）

#### A-1：ILifecycle 空实现回退

**目标**：当 ViewModel 无 `#[on_loaded]`/`#[on_unloaded]` 钩子时，codegen 自动生成空 `impl ILifecycle for X {}`，消除手动 impl 样板。

**文件**：
- `crates/engine/src/compiler/codegen/lifecycle.rs`

**改动概要**：

修改 `gen_lifecycle_impl`（lifecycle.rs:20-23）的早期返回分支：

```rust
// 改前：
pub(super) fn gen_lifecycle_impl(ctx: &CodegenCtx) -> String {
    if !ctx.lifecycle_hooks.has_any() {
        return String::new();  // ← 问题点
    }
    // ...
}

// 改后：
pub(super) fn gen_lifecycle_impl(ctx: &CodegenCtx) -> String {
    // 冲突检测优先（保留原逻辑）
    if ctx.has_manual_lifecycle_impl {
        if ctx.lifecycle_hooks.has_any() {
            println!(
                "cargo:warning=RML: {} 同时存在手动 `impl ILifecycle` 与 `#[on_loaded]`/`#[on_unloaded]` 标注，\
                 codegen 跳过自动生成。请删除手动 impl 或移除标注以避免歧义。",
                ctx.view_struct_name
            );
        }
        return String::new();  // 用户已手动 impl，跳过
    }

    let view_name = &ctx.view_struct_name;

    // 无钩子且无手动 impl：生成空 impl（满足 IViewModel: ILifecycle 约束）
    if !ctx.lifecycle_hooks.has_any() {
        return format!(
            r#"#[allow(dead_code)]
impl rml_core::lifecycle::ILifecycle for {view_name} {{}}
"#,
            view_name = view_name,
        );
    }

    // 有钩子：生成带方法的 impl（保留原逻辑）
    // ...
}
```

**前置依赖**：无

**风险**：低。空 impl 不覆盖 trait 默认方法，行为等价于手动写空 impl。

**验证**：
- 现有 `no_hooks_returns_empty` 测试需更新为 `no_hooks_generates_empty_impl`
- 新增测试：`empty_impl_satisfies_lifecycle_trait`（验证生成的代码包含 `impl ILifecycle for X {}`）
- demo 删除手动 impl 后编译通过

---

#### A-2：删除 demo 中冗余的手动 ILifecycle impl

**目标**：在 A-1 完成后，删除 demo 中所有 `impl ILifecycle for X {}` 空实现，验证 codegen 自动生成有效。

**文件**：
- `crates/demo/src/**/*.rs`（grep 定位所有 `impl ILifecycle` 空实现）

**改动概要**：

1. 用 Grep 定位所有 `impl ILifecycle for` 出现位置
2. 逐个检查：若是空 impl `{}` 或仅含 trait 默认方法，删除
3. 若含用户自定义逻辑，保留并确保 `has_manual_lifecycle_impl` 标记正确

**前置依赖**：A-1 完成

**风险**：低。删除空 impl 后由 codegen 自动补全。

**验证**：
- `cargo build --workspace` 通过
- demo 运行时行为不变

---

#### A-3：标记 ObservableVec 为 deprecated

**目标**：明确 `ObservableVec` 不再推荐使用，引导用户使用 `Vec<T>` + `#[command]` 自动 bump 模式。

**文件**：
- `crates/core/src/observable.rs`

**改动概要**：

在 `ObservableVec` 结构体及其 `new`/`with_notifier` 方法上添加 `#[deprecated]` 属性：

```rust
#[deprecated(
    since = "0.2.0",
    note = "ObservableVec 与框架版本号机制脱节，请使用 Vec<T> + #[command] 自动 bump_version + cx.notify()"
)]
pub struct ObservableVec<T> { /* ... */ }

impl<T> ObservableVec<T> {
    #[deprecated(since = "0.2.0", note = "使用 Vec<T> + #[command] 替代")]
    pub fn new() -> Self { /* ... */ }

    #[deprecated(since = "0.2.0", note = "使用 Vec<T> + #[command] + cx.notify() 替代")]
    pub fn with_notifier(notify: Sender<()>) -> Self { /* ... */ }
}
```

**前置依赖**：无

**风险**：极低。demo 零使用，仅影响潜在的外部使用者。

**验证**：
- 编译通过（`#[deprecated]` 仅产生 warning）
- 现有测试加 `#[allow(deprecated)]` 标记

---

### Phase B：MVVM 核心能力补全（3 项）

#### B-1：声明式命令绑定 `command={field}`

**目标**：在 `.rml` 中支持 `<MenuItem command={save_command} />` 声明式绑定，codegen 生成 `RelayCommand::execute` 调用。

**文件**：
- `crates/engine/src/compiler/menu/item.rs`
- `crates/engine/src/compiler/attribute.rs`（若需扩展 `Attribute::Bind` 识别 `command`）

**改动概要**：

1. 在 `gen_menu_item_stmt`（item.rs:46-155）中，在现有 `onclick` 处理之前，新增 `command` 属性检测：

```rust
// 新增：检查 command={field} 属性
if let Some(cmd_expr) = bind_attr(elem, "command", loop_vars, ctx, hoist)? {
    let onclick = gen_command_closure(&cmd_expr, ctx)?;
    // 生成 .on_click 闭包，调用 RelayCommand::execute
    // ...
}
```

2. 新增 `gen_command_closure` 函数：

```rust
fn gen_command_closure(cmd_expr: &str, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    // 生成代码模式：
    // .on_click({
    //     let weak = __rml_menu_weak.clone();
    //     move |_ev, _window, app| {
    //         if let Some(entity) = weak.upgrade() {
    //             entity.update(app, |this, cx| {
    //                 let cmd = &this.{field};  // 读克隆命令对象
    //                 let mut ctx = rml_core::command::CallContext::new(cx);
    //                 if cmd.can_execute(&ctx) {
    //                     cmd.execute(&mut ctx);
    //                 }
    //             });
    //         }
    //     }
    // })
    //
    // 注意：Read-clone-then-execute 模式避免 window/app 借用冲突
}
```

3. `command` 属性的 `can_execute` 可绑定到 `disabled` 属性（未来扩展，本期不做）

**前置依赖**：无

**风险**：中。需处理 `WeakEntity` + `update` 借用模式，与现有 `gen_onclick_closure` 类似。

**验证**：
- 新增单元测试：`command_attr_generates_execute_call`
- 新增集成测试：demo 中 `<MenuItem command={save} />` 触发命令执行
- `cargo test --workspace` 通过

---

#### B-2：双向绑定接入 convert_back

**目标**：让 `<input model={price | Currency} />` 的反向绑定（UI→VM）调用 `Currency::convert_back` 而非裸 `parse`。

**文件**：
- `crates/engine/src/parser/ast.rs` — 扩展 `Directive::Model`
- `crates/engine/src/parser/mod.rs` — 解析 `|` 管道
- `crates/engine/src/compiler/codegen/binding.rs` — 调用 convert_back
- `crates/engine/src/compiler/codegen/observable.rs` — `gen_model_input` 传递 converter
- `crates/engine/src/compiler/mod.rs` — `CodegenCtx` 新增 `model_converters: HashMap<String, String>`

**改动概要**：

1. **AST 扩展**（ast.rs:76）：

```rust
// 改前：
Model(String),

// 改后：
Model { field: String, converter: Option<String> },
```

2. **Parser 解析**（parser/mod.rs:179-183）：

```rust
// 改前：
"model" => {
    if let AttrValue::Binding(expr) = attr.value {
        directives.push(Directive::Model(expr));
    }
}

// 改后：
"model" => {
    if let AttrValue::Binding(expr) = attr.value {
        // 解析 "field | Converter" 语法
        if let Some((field, converter)) = expr.split_once('|') {
            directives.push(Directive::Model {
                field: field.trim().to_string(),
                converter: Some(converter.trim().to_string()),
            });
        } else {
            directives.push(Directive::Model {
                field: expr,
                converter: None,
            });
        }
    }
}
```

3. **Codegen 接入**（binding.rs `gen_field_assign_expr`）：

在 `gen_field_assign_expr_default` 等分支中，若存在 converter，替换 parse 逻辑为 `ConverterName::convert_back(&value)`：

```rust
// 改前（数字类型）：
match value.parse::<i32>() { ... }

// 改后（有 converter 时）：
match {ConverterName}::default().convert_back(&value.to_string()) {
    Some(v) => {
        this.{field} = v;
        this.__rml_field_errors.insert({field:?}.to_string(), None);
        this.__rml_bump_version({field:?});
    }
    None => {
        this.__rml_field_errors.insert({field:?}.to_string(), Some("转换失败".into()));
    }
}
```

4. **CodegenCtx 扩展**：`model_converters: HashMap<String, String>` 存储 `field → converter_name` 映射

**前置依赖**：无

**风险**：中。涉及 AST 结构变更，需更新所有 `Directive::Model` 的 match 臂。

**验证**：
- 现有 `codegen_two_way_binding_test.rs` 需更新 match 模式
- 新增测试：`model_with_converter_generates_convert_back_call`
- 新增测试：`model_without_converter_keeps_parse_behavior`
- `cargo test --workspace` 通过

---

#### B-3：oninput/onchange 事件 + stop_propagation 生效

**目标**：
1. 让 `<input oninput={handle_input} />` 生效
2. 让 `rml_ev.stop_propagation()` 真正阻止事件冒泡

**文件**：
- `crates/engine/src/compiler/event.rs` — 新增 oninput/onchange 处理 + stop_propagation 检查
- `crates/engine/src/compiler/codegen/observable.rs` — 复用 InputState 订阅机制

**改动概要**：

1. **oninput/onchange 实现**：

GPUI 元素级无 `on_input` 方法，但 `InputState` 通过 `cx.subscribe` 已实现双向同步。oninput/onchange 需复用此机制，在反向同步时调用用户 handler。

修改 `event_binding`（event.rs:63）：

```rust
// 改前：
"oninput" | "onchange" | ... => None,

// 改后：
"oninput" => Some(("rml_core::events::InputEvent", "on_input", "rml_convert::convert::input(value, prev)")),
"onchange" => Some(("rml_core::events::ChangeEvent", "on_change", "rml_convert::convert::change(value)")),
```

由于 GPUI 元素无 `on_input`/`on_change` 方法，需在 `apply_event` 中特殊处理：生成 `cx.subscribe` 订阅 InputState 变更，在回调中调用用户 handler。

**替代方案（推荐）**：不通过 `event_binding` 路由，而是在 `gen_model_input` 中检测 `oninput`/`onchange` 属性，生成附加的 `.on_input_handler(...)` builder 方法（需扩展 InputState 或在 codegen 层包装）。

2. **stop_propagation 生效**：

修改 `apply_event`（event.rs:96-100）生成的闭包：

```rust
// 改前：
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

**前置依赖**：无

**风险**：中。oninput/onchange 需深入 InputState 订阅机制，可能与现有双向绑定产生重复触发。需确保 oninput 与 model 双向绑定的触发顺序正确。

**验证**：
- 新增测试：`oninput_generates_subscription`
- 新增测试：`stop_propagation_check_injected`
- 手动验证：demo 中 oninput handler 在 model 同步之后触发
- `cargo test --workspace` 通过

---

### Phase C：延迟实施（3 项）

> 以下项目依赖 GPUI 上游能力或复杂度较高，本期不实施，仅记录待未来评估。

#### C-1：onfocus/onblur 事件支持

**问题**：GPUI 元素级（Div）无 `on_focus`/`on_blur` 方法，只有 Context 级 `cx.on_focus(handle, window, listener) -> Subscription`。

**延迟原因**：需设计 element-level focus 管理抽象，可能需扩展 GPUI 或引入 focus handle 注册机制。

**未来方向**：在 codegen 中为带 `onfocus` 的元素生成 `cx.on_focus(focus_handle, window, listener)` 订阅，并在元素 render 时关联 focus_handle。

---

#### C-2：Capture/Bubble 三阶段调度

**问题**：`EventPhase` 枚举（event_flow.rs:22-26）已定义但未使用，三阶段调度未实现。

**延迟原因**：GPUI 原生支持 `DispatchPhase::Capture/Bubble`，但 RML codegen 未生成 capture 阶段绑定。需设计 `onclick.capture={fn}` 语法 + codegen 路由。

**未来方向**：扩展 `EventHandler` 支持 `phase: DispatchPhase` 字段，codegen 生成 `.on_click_with_phase(cx.listener(...), DispatchPhase::Capture)`。

---

#### C-3：onsubmit/onload/onresize/onscroll 事件

**问题**：这四类事件在 GPUI 中无直接对应元素级方法。

**延迟原因**：
- `onsubmit`：需 form 元素抽象 + Enter 键拦截
- `onload`：RML 无异步加载生命周期，需设计
- `onresize`/`onscroll`：需 window 级订阅或 IntersectionObserver 类抽象

**未来方向**：按需逐个实现，优先级 onsubmit > onscroll > onresize > onload。

---

## 3. 依赖关系图

```
Phase A（并行）:
  A-1 ──→ A-2
  A-3（独立）

Phase B（部分串行）:
  B-1（独立）
  B-2（独立）
  B-3（依赖 B-2 的 model 机制理解，但无硬依赖）

Phase C（延迟）:
  C-1, C-2, C-3（独立延迟）
```

**推荐执行顺序**：
1. A-1 + A-3（并行，低风险清理）
2. A-2（依赖 A-1）
3. B-2（AST 变更，先做减少后续冲突）
4. B-1（命令绑定）
5. B-3（事件补全）

---

## 4. 风险评估

| 风险项 | 概率 | 影响 | 缓解措施 |
|---|---|---|---|
| A-1 空 impl 与用户手动 impl 冲突 | 低 | 中 | `has_manual_lifecycle_impl` 检测已存在，保留优先级 |
| B-1 命令对象借用冲突 | 中 | 中 | 采用 read-clone-then-execute 模式，参考现有 `gen_onclick_closure` |
| B-2 AST 结构变更引发回归 | 高 | 中 | 更新所有 match 臂，新增测试覆盖 converter/no-converter 两种路径 |
| B-3 oninput 与 model 双向绑定重复触发 | 中 | 高 | 确保触发顺序：model 同步 → oninput handler；或合并为单次更新 |
| B-3 stop_propagation 与 GPUI 调度器冲突 | 低 | 低 | `cx.stop_propagation()` 是 GPUI 原生 API，行为可预期 |

---

## 5. 验证准则

每个步骤完成后需满足：

1. **编译验证**：`cargo build --workspace` 无错误
2. **测试验证**：`cargo test --workspace` 全过，新增测试覆盖改动点
3. **回归验证**：现有 553 个测试无回归
4. **demo 验证**（针对 B 系列）：在 demo 中添加使用示例，手动验证功能生效
5. **文档更新**：在 `docs/09-architecture/observable-refactor-plan.md` 追加完成状态

---

## 6. 假设与决策

### 6.1 假设

- A-1 的空 impl 不会与未来可能引入的 ILifecycle 必选方法冲突（当前 trait 两个方法都有默认实现）
- B-1 的 `command` 属性不会与现有 `onclick` 属性冲突（两者同时存在时 command 优先生效，onclick 被忽略并发出 warning）
- B-2 的 `convert_back` 返回 `Option<Source>`，转换失败时写入 field_error 而非 panic
- B-3 的 oninput 事件在 model 双向绑定之后触发，用户可读取已同步的字段值

### 6.2 决策

| # | 决策点 | 选择 | 理由 |
|---|---|---|---|
| 1 | A-1 空 impl 是否带 `#[allow(dead_code)]` | 是 | 空 impl 会触发 dead_code warning |
| 2 | B-1 command 属性是否支持 `can_execute` 绑定 disabled | 否（本期） | 复杂度较高，延迟到 Phase C |
| 3 | B-2 convert_back 失败时的错误消息 | "转换失败" | 通用消息，未来可扩展为 converter 自定义 |
| 4 | B-3 oninput 与 model 同时存在时的触发顺序 | model 先，oninput 后 | 用户期望读取已同步值 |
| 5 | ObservableVec 是否完全删除 | 否（仅 deprecated） | 向后兼容，未来版本再删 |

---

## 7. 不在范围内

- 实际编写代码（本计划只产决策与步骤）
- GPUI 上游能力扩展（如元素级 focus 事件）
- 性能优化（如部分重渲）
- 热重载支持
- 运行时反应式系统（Proxy/反射）
