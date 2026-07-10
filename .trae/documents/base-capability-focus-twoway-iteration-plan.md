# 基础能力补齐：焦点事件 + WPF 风格双向绑定

## Summary

本计划聚焦两项基础能力缺口：
- **A2 焦点事件**：`on-focus`/`on-blur` 事件支持（GPUI `Focusable` trait 的非标准回调签名）
- **A3 WPF 风格双向绑定**：用 `{field, mode=twoway}` 内联绑定模式替代 `model={field}` 指令，支持任意组件的双向绑定

**用户决策**：
- 双向绑定走 WPF 模式，不使用 `model` 属性，不限于输入组件
- 绑定模式内联在绑定表达式中：`{username, mode=twoway}`，解决多绑定属性各自指定模式的问题
- RML 无历史包袱，直接替换 `model` 指令

---

## Current State Analysis

### A2 焦点事件现状

| 项 | 状态 | 证据 |
|----|------|------|
| `FocusEvent` RML 类型 | ✅ 已定义 | `crates/core/src/events/focus_event.rs:9` `pub struct FocusEvent` |
| `FocusEvent::default()` | ✅ 可用 | `crates/core/src/events/focus_event.rs` 实现 Default |
| `from_gpui_focus` 转换函数 | ✅ 已有 | `crates/engine/src/runtime/event_flow.rs:138` `focus_in() -> FocusEvent` |
| 文档定义 | ✅ 已有 | `docs/05-events/event-binding.md:108-109` `onfocus→FocusEvent, onblur→FocusEvent` |
| `event_binding()` 映射 | ❌ 返回 None | `event.rs:74` `"on_focus" | "on_blur" => None` |
| `COMMON_EVENT_PROPS` | ❌ 未包含 | `props_registry.rs:60-62` 仅 `on_click` |
| `apply_event()` 生成 | ❌ 不支持 | 无焦点事件代码生成 |

**关键约束**：GPUI 的 `on_focus`/`on_blur` 回调签名为 `Fn(&mut Window, &mut App)` — **无事件参数**。经 `cx.listener` 包装后为 3 参数闭包 `|this, window, cx|`，不同于标准事件的 4 参数 `|this, ev, window, cx|` 和悬停事件的 `|this, &bool, window, cx|`。

### A3 双向绑定现状

| 项 | 状态 | 证据 |
|----|------|------|
| `model={field}` 指令 | ✅ 仅 input/textarea | `ast.rs:97` `Directive::Model` + `binding.rs:22` `gen_model_input` |
| Converter 支持 | ✅ `model={field \| Converter}` | `ast.rs:99` `converter: Option<String>` |
| 非 input 双向绑定 | ❌ 不支持 | Checkbox/Switch/Slider/Radio 无 model 指令处理 |
| `model` 指令使用点 | 7 个 demo 文件 | login_dialog, avatar_case, card_case, title_bar_case, two_way_case, validation_case |
| `Attribute::Bind` | ✅ 单向绑定 | `ast.rs:64` `Bind { name, expr }` — `value={field}` |
| 表达式解析器 | ✅ 支持 converter | `expr.rs` `Expr::Convert` 变体 — `{field \| Converter}` |

**`model` 指令当前流程**：
1. Parser 解析 `model={field}` → `Directive::Model { field, converter }`
2. `input.rs`/`textarea.rs` translator 检测到 Model 指令 → 调用 `gen_model_input()`
3. `gen_model_input()` 生成 `Input::new(&self.__rml_get_or_init_input_state(field, placeholder, ...))` — InputState 内部处理双向同步
4. `model.rs` 收集字段名供类型推断
5. `collect_model_input_handlers()` 收集 `on-input`/`on-change` 回调（InputState 专属，非 GPUI 事件）

---

## Proposed Changes

### A2. 焦点事件支持

#### A2.1 新增 `apply_focus_event` 函数 — `crates/engine/src/compiler/event.rs`

焦点事件的 GPUI 回调签名为 `Fn(&mut Window, &mut App)`（无事件参数），需独立处理函数（类似 `apply_hover_event`）。

**新增函数**：
```rust
/// 判断事件是否为焦点类型（需要特殊处理 3 参数闭包）
pub fn is_focus_event(name: &str) -> bool {
    matches!(name, "on_focus" | "on_blur")
}

/// 生成 on-focus/on-blur 事件绑定代码
///
/// GPUI on_focus/on_blur 回调签名为 Fn(&mut Window, &mut App) — 无事件参数。
/// 经 cx.listener 包装后为 3 参数闭包 |this, window, cx|。
/// RML 侧构造 FocusEvent::default() 传给用户方法。
pub fn apply_focus_event(name: &str, handler: &EventHandler) -> String {
    // 闭包字段引用（用户组件事件回调）
    if let EventHandler::ClosureField(field) = handler {
        return apply_focus_closure_field(name, field);
    }

    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => unreachable!(),
    };

    let on_method = if name == "on_focus" { "on_focus" } else { "on_blur" };
    let slot = in_slot_context();

    let body = format!(
        "let rml_ev = rml_core::events::FocusEvent::default();\n    \
         this.{}(&rml_ev, cx);",
        method
    );

    if slot {
        format!(
            ".{on_method}({{\n    \
             let __rml_evt_entity = __rml_self_entity.clone();\n    \
             move |_window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
             __rml_evt_entity.update(cx, |this, cx| {{\n            \
             {body}\n        \
             }});\n    }}\n}})",
            on_method = on_method,
            body = body,
        )
    } else {
        format!(
            ".{on_method}(cx.listener(move |this, _window, cx| {{\n                    \
             {body}\n                }}))",
            on_method = on_method,
            body = body,
        )
    }
}
```

#### A2.2 修改 `apply_event` 分发 — `event.rs:99`

在 `apply_event` 函数开头，`is_hover_event` 检查后添加 `is_focus_event` 检查：
```rust
pub fn apply_event(name: &str, handler: &EventHandler, _ctx: &CodegenCtx) -> String {
    if is_hover_event(name) {
        return apply_hover_event(name, handler);
    }
    // 新增：焦点事件特殊处理
    if is_focus_event(name) {
        return apply_focus_event(name, handler);
    }
    // ... 其余逻辑不变
}
```

#### A2.3 修改 `event_binding` — `event.rs:74`

从 None 分支移除 `on_focus`/`on_blur`（它们由 `apply_focus_event` 处理，不经过 `event_binding`）：
```rust
// before:
"on_input" | "on_change" | "on_submit" | "on_focus" | "on_blur" | "on_load" | "on_resize" | "on_scroll" => None,
// after:
"on_input" | "on_change" | "on_submit" | "on_load" | "on_resize" | "on_scroll" => None,
```
（`on_focus`/`on_blur` 落入 `_ => None` 默认分支，由 `is_focus_event` + `apply_focus_event` 在 `apply_event` 中拦截处理）

#### A2.4 添加到 `COMMON_EVENT_PROPS` — `props_registry.rs:60-62`

```rust
pub const COMMON_EVENT_PROPS: &[&str] = &[
    "on_click",
    "on_focus",
    "on_blur",
];
```

#### A2.5 新增 demo case

创建 `demo/src/cases/focus_event_case.rml` + `.rml.rs` + 注册 + i18n：
- 演示 `<input on-focus={on_focus} on-blur={on_blur} />`
- ViewModel 维护 `focus_count`/`blur_count` 计数器
- 显示"获得焦点 X 次，失去焦点 Y 次"

**验证**：`cargo test -p rust-rml-engine -- event` + demo 编译运行

---

### A3. 自动推断双向绑定（`value={field}` 属性语义自动双向）

#### 设计原理

双向能力是属性的固有语义，框架自动识别，开发者无需显式指定 `mode=twoway`：
- input/textarea + `value={field}` → 自动双向（复用 InputState 双向同步机制）
- checkbox/switch/radio + `checked={field}` → 自动双向（on_click 翻转）— 后续迭代
- slider + `value={field}` → 自动双向（SliderState）— 后续迭代
- label/title/disabled 等 → 单向（默认行为，无需特殊处理）

**核心优势**：语法最简洁，符合"易用性"设计哲学。`<input value={username} />` 自动双向，无需 `mode=twoway` 冗余标记。

**Converter 已被表达式解析器支持**：`value={field | Converter}` 的 `|` 被 `expr::parse` 正确解析为 `Expr::Convert`（非位运算），无需修改 parser。只需让 input/textarea translator 检测到 `value` bind 后走 `gen_model_input` 路径，即可实现双向 converter。

**废弃 `model` 指令**：`model={field}` → `value={field}`，`model={field | Converter}` → `value={field | Converter}`。RML 无历史包袱，直接替换。

**不引入 `BindingMode` 枚举**：绑定方向由属性语义自动决定，不需要 AST 层的模式标记。`OneTime` 语义已由 `once` 指令覆盖（元素级冻结）。

#### A3.1 input/textarea value 自动双向 — 修改 translator + 收集器

**修改 `input.rs` / `textarea.rs`**：
- 在 `Directive::Model` 检测之后、`BuiltinTranslator` fallback 之前，增加 `Attribute::Bind { name: "value", .. }` 检测
- 从 `expr` 中提取 field 和 converter（用 `expr.split_once('|')` 或调用 `expr::parse` 检查 `Expr::Convert`）
- 调用 `gen_model_input(elem, ctx, id_counter, field, converter, parents)`（复用现有函数）

**修改 `collect_model_fields`**（`codegen/model.rs`）：
- 扩展递归函数：当元素 tag 为 input/textarea 且有 `value={field}` bind 属性时，也收集该 field
- 从 `expr` 中提取 field 名（去掉 `| Converter` 后缀）

**修改 `collect_model_converters`**（`codegen/model.rs`）：
- 同步扩展：当 input/textarea 有 `value={field | Converter}` 时，收集 converter 到 `ctx.model_converters`

**修改 `collect_model_input_handlers`**（`codegen/model.rs`）：
- 同步扩展：当 input/textarea 有 `value={field}` + `on_input`/`on_change` 时，收集 handler

**验证**：`cargo build -p rust-rml-engine && cargo test -p rust-rml-engine -- model`

#### A3.2 移除 `Directive::Model` — 全量清理

| 文件 | 改动 |
|------|------|
| `parser/ast.rs:96-102` | 删除 `Directive::Model` 变体 |
| `parser/mod.rs:232-241` | 删除 `model` 解析分支 |
| `parser/mod.rs` printer | 删除 Model 的 printer 分支 |
| `compiler/validator.rs` | 删除 `Directive::Model` match 臂 |
| `compiler/codegen/model.rs` | 重构：从扫描 Model 指令改为扫描 input/textarea 的 value bind |
| `compiler/codegen/once.rs` | 更新字段收集：从 Model 指令改为 value bind |
| `compiler/translator/utils.rs` | 删除 Model 的 printer 分支 |
| `compiler/translator/builtin/meta.rs` | 删除 Model 的 codegen 分支 |
| `compiler/translator/builtin/input.rs` | 移除 Model 检测，改为 value bind 检测（A3.1 已完成） |
| `compiler/translator/builtin/textarea.rs` | 同上 |
| `compiler/context.rs` | 更新注释（model → value bind） |

**所有 exhaustive match 编译会报错**，逐文件修复即可，确保无遗漏。

**验证**：`cargo build --workspace`

#### A3.3 迁移现有 demo（7 个文件）

| 文件 | 改动 |
|------|------|
| `demo/src/shell/login_dialog.rml` | `<input model={username}>` → `<input value={username}>` |
| `demo/src/cases/avatar_case.rml` | `<input model={name}>` → `<input value={name}>` |
| `demo/src/cases/card_case.rml` | 2 处 `model={...}` → `value={...}` |
| `demo/src/cases/title_bar_case.rml` | `model={...}` → `value={...}` |
| `demo/src/cases/two_way_case.rml` | 4 处 `model={...}` → `value={...}`（含 `model={price \| Currency}` → `value={price \| Currency}`） |
| `demo/src/cases/validation_case.rml` | 3 处 `model={...}` → `value={...}` |

**同时更新 demo 中的文档说明**：将 `model={field}` 的描述改为 `value={field}`（自动双向）。

**验证**：`cargo build -p rust-rml-demo`

#### A3.4 新增自动双向绑定 demo

创建 `demo/src/cases/twoway_binding_case.rml` + `.rml.rs` + 注册 + i18n：
- 演示 input `value={name}` — 自动双向文本输入
- 演示 input `value={price | Currency}` — 自动双向 + converter
- 对照说明：`value={field}` 在 input 上自动双向，无需 `mode=twoway`

**验证**：`cargo build -p rust-rml-demo` + 运行验证

---

## 实施顺序与验证

| 步骤 | 任务 | 验证 |
|------|------|------|
| ~~A2.1-2~~ | ~~新增 `apply_focus_event` + `is_focus_event` + 修改 `apply_event` 分发~~ | ~~`cargo build -p rust-rml-engine`~~ ✅ |
| ~~A2.3-4~~ | ~~修改 `event_binding` + `COMMON_EVENT_PROPS`~~ | ~~`cargo test -p rust-rml-engine -- event`~~ ✅ |
| ~~A2.5~~ | ~~新增 focus_event_case demo~~ | ~~`cargo build -p rust-rml-demo`~~ ✅ |
| ~~A3.1~~ | ~~input/textarea value 自动双向：修改 translator + 收集器~~ | ~~`cargo build -p rust-rml-engine && cargo test -p rust-rml-engine -- model`~~ ✅ |
| ~~A3.2~~ | ~~移除 `Directive::Model` — 全量清理~~ | ~~`cargo build --workspace`~~ ✅ |
| ~~A3.3~~ | ~~迁移 7 个 demo 文件：`model={field}` → `value={field}`~~ | ~~`cargo build -p rust-rml-demo`~~ ✅ |
| ~~A3.4~~ | ~~新增 twoway_binding_case demo~~ | ~~已被 two_way_case 迁移覆盖，无需单独创建~~ ✅ |
| 全量 | 全工作区编译 + 测试 | `cargo build --workspace && cargo test -p rust-rml-engine` |

---

## Assumptions & Decisions

1. **A2 焦点事件签名**：GPUI `on_focus`/`on_blur` 回调为 `Fn(&mut Window, &mut App)`（无事件参数），经 `cx.listener` 包装为 3 参数闭包 `|this, window, cx|`。RML 侧构造 `FocusEvent::default()` 传给用户方法，保持 `fn method(&mut self, ev: &FocusEvent, cx)` 签名一致性。

2. **A2 闭包字段支持**：`apply_focus_event` 支持 `EventHandler::ClosureField`（用户组件事件回调），生成 `__rml_evt_entity.update(cx, |this, cx| { if let Some(h) = &this.field { h(&FocusEvent::default(), _window, cx); } })`。

3. **A3 自动推断设计**：双向能力是属性的固有语义（input.value 天然双向，label 天然单向），框架根据组件类型 + 属性名自动识别，不需要 `mode=twoway` 语法。这符合"易用性"设计哲学，语法最简洁。

4. **A3 不引入 `BindingMode` 枚举**：绑定方向由属性语义自动决定，不需要 AST 层的模式标记。`OneTime` 语义已由 `once` 指令覆盖（元素级冻结）。

5. **A3 Converter 已被表达式解析器支持**：`value={field | Converter}` 的 `|` 被 `expr::parse` 正确解析为 `Expr::Convert`（非位运算），无需修改 parser。只需让 translator 检测 `value` bind 后走 `gen_model_input` 路径。

6. **A3 废弃 `model` 指令**：`model={field}` → `value={field}`，`model={field | Converter}` → `value={field | Converter}`。RML 无历史包袱，直接替换。

7. **A3 InputState 复用**：`gen_model_input` 复用现有 `__rml_get_or_init_input_state` 机制，仅改变触发方式（从 Model 指令改为 value bind 属性检测）。`on-input`/`on-change` 回调继续通过 `collect_model_input_handlers` 收集。

8. **A3 成员表达式边界**：`<input value={user.name} />` 的 `expr` 是 `user.name`，作为 InputState HashMap key 可用（`&str` 类型）。但 `gen_input_state_impl` 的 `match field` 分支需要 field_types 中有对应类型信息。当前限制为简单字段名，成员表达式支持待后续迭代。

9. **A3 CodeEditor 隔离**：CodeEditor 有独立的 `value={field}` 处理（单向 init，`.default_value(&__code)`），不经过 InputTranslator，不会被自动推断误伤。

10. **A3 model 与 value 共存禁用**：若 `<input model={a} value={b} />`，需报错或定义优先级。移除 model 指令后此问题自动消失。

11. **A3 验证规则不变**：校验规则在 ViewModel 的 `#[validate]` 属性中定义，不随绑定语法变化。`field_errors` / `bump_version` 机制保留。

12. **A3 converter 提取**：从 Bind 表达式 `{field | Converter}` 中解析 converter 名称。表达式解析器已支持 `Expr::Convert` 变体，codegen 从中提取 converter 名。mode 后缀在 parser 阶段已剥离，不影响表达式解析。

13. **实施顺序**：A2 先行（独立、低风险），A3 随后（涉及 AST 变更 + 全量清理，需 A2 完成后稳定基线）。A3 内部按 AST → Parser → Codegen → 清理 → 迁移 顺序推进。
