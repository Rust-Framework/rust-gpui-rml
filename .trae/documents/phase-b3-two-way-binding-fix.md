# Phase B-3 双向绑定修复 + 宏 API 文档

## Summary

上一会话完成了 Phase B-3 双向绑定的代码骨架（scanner 字段类型提取、宏注入 `__rml_input_states`/`__rml_input_subscriptions`、`gen_model_input` 重写、`gen_input_state_impl` 生成），但 demo 编译失败，因为生成的代码调用了不存在的 gpui-component API：

1. `InputState::default()` 不存在 → 需 `InputState::new(window, cx)`
2. `cx.new_entity` 不在 `Context<Self>` 上 → 需 `cx.new`
3. `Input::new(...).clone()` 类型不匹配 → 需传引用 `&entity`
4. `Input` 没有 `.value()` 方法 → 正向绑定改用 `InputState::set_value`
5. `Input` 没有 `.placeholder()` 方法 → placeholder 在 `InputState::new()` 创建时设置
6. `Input` 没有 `.on_change()` 方法 → 反向绑定改用 `cx.subscribe` + `InputEvent::Change`

本计划修复这 6 个问题，让 demo 编译通过并真正实现 WPF 风格的双向绑定，然后补充宏 API 文档和用户指南。

## Current State Analysis

### 已完成且稳定（Phase B-2 + Phase 1 细粒度更新）

- `crates/engine/src/build/scanner.rs`：`StructMetadata.field_types: HashMap<String, String>` 已就绪
- `crates/engine/src/compiler/mod.rs`：`CodegenCtx.field_types` 已就绪
- `crates/macros/src/component.rs`：已注入 `__rml_input_states: HashMap<String, Entity<InputState>>` 和 `__rml_input_subscriptions: Vec<Subscription>`
- `crates/macros/src/lib.rs` + `command.rs`：`#[command(no_notify)]` 条件注入已就绪
- `crates/engine/src/compiler/codegen.rs`：`gen_observable_impl` 生成 `__rml_changed_fields()` 已就绪
- `docs/10-advanced/performance.md`：性能文档已重写为真实 GPUI 渲染模型

### 需要修复的文件

| 文件 | 问题 |
|------|------|
| `crates/engine/src/compiler/codegen.rs` | `gen_model_input`（L502-549）生成 `.value()`/`.on_change()`/`.placeholder()` 调用，均不存在于 Input；`gen_input_state_impl`（L869-893）生成 `InputState::default()` + `cx.new_entity`，均不存在 |
| `crates/ui/src/lib.rs` | 未 re-export `InputEvent`，生成代码无法引用 |
| `crates/macros/src/component.rs` | 缺少 `__rml_input_state_versions: HashMap<String, u64>` 字段（正向同步版本追踪所需） |
| `crates/engine/tests/codegen_two_way_binding_test.rs` | 8 个测试断言基于旧 API，需更新 |
| `demo/src/main_window.rml` | 已就绪（`<input model={name}>` + `<input model={age}>`），等待 codegen 修复 |

### 真实 gpui-component API（已确认）

通过阅读 `gpui-component` 源码确认：

**InputState**（`crates/ui/src/input/state.rs`）：
```rust
pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self  // L460
pub fn placeholder(self, placeholder: impl Into<SharedString>) -> Self  // L619 builder，消费 self
pub fn value(&self) -> SharedString  // L1168
pub fn set_value(&mut self, value: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>)  // L784，内部设 emit_events=false
impl EventEmitter<InputEvent> for InputState {}  // L454
```

**Input**（`crates/ui/src/input/input.rs`）：
```rust
pub fn new(state: &Entity<InputState>) -> Self  // L77，接收引用，内部 clone
pub fn disabled(self, disabled: bool) -> Self  // L150
// 无 .value() / .placeholder() / .on_change()
```

**InputEvent**（`state.rs` L122-128）：
```rust
pub enum InputEvent { Change, PressEnter { secondary, shift }, Focus, Blur }
```

**正确使用模式**（`gallery.rs` L27-35）：
```rust
let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("Search..."));
let _subscriptions = vec![cx.subscribe(&search_input, |this, _, e, cx| match e {
    InputEvent::Change => { /* ... */ cx.notify() }
    _ => {}
})];
// 使用: Input::new(&self.search_input)
```

## Proposed Changes

### 阶段 2-A：修复双向绑定 codegen（核心）

#### Step 1: re-export `InputEvent`

**文件**：`crates/ui/src/lib.rs` L52

**变更**：`input::{Input, InputState}` → `input::{Input, InputEvent, InputState}`

**原因**：生成代码需引用 `rml_ui::InputEvent::Change`，避免暴露 `gpui_component` 完整路径。

#### Step 2: 宏注入 `__rml_input_state_versions` 字段

**文件**：`crates/macros/src/component.rs` `inject_tracking_fields()` 末尾

**变更**：在 `__rml_input_subscriptions` 注入之后，追加：
```rust
let input_state_versions_field: Field = parse_quote! {
    #[allow(dead_code)]
    __rml_input_state_versions: std::collections::HashMap<String, u64>
};
named.named.push(input_state_versions_field);
```

**原因**：正向同步（VM→UI）需要记录每个字段上次同步到的版本号。当 `#[command]` 改变字段值并 bump_version 后，render 时对比版本号差异，决定是否调用 `InputState::set_value`。反向同步（UI→VM）在 subscribe 闭包内也会更新此版本号，标记"刚从 UI 同步过来"，避免 render 时冗余回写。

#### Step 3: 重写 `gen_input_state_impl`

**文件**：`crates/engine/src/compiler/codegen.rs` L853-893

**生成的方法签名**（新增 `placeholder` + `window` 参数）：
```rust
fn __rml_get_or_init_input_state(
    &mut self,
    field: &'static str,
    placeholder: Option<&'static str>,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<Self>,
) -> gpui::Entity<rml_ui::InputState>
```

**生成的方法体逻辑**：
1. **首次创建**：`cx.new(|cx| rml_ui::InputState::new(window, cx))`，可选 `.placeholder(p)` builder
2. **初始正向同步**：`entity.update(cx, |state, cx| state.set_value(initial_value, window, cx))`，其中 `initial_value` 由 match field 生成（`"name" => self.name.clone().into()`、`"age" => self.age.to_string().into()`）
3. **反向订阅**：`cx.subscribe(&entity, move |this, input_entity, event, cx| { match event { Change => { 反向赋值 + bump_version + 标记同步 + cx.notify() } _ => {} } })`，subscription push 到 `__rml_input_subscriptions`
4. **记录初始版本**：`self.__rml_input_state_versions.insert(field, self.__rml_get_version(field))`
5. **后续调用正向同步**：对比 `current_version` vs `last_synced`，若不同则 `entity.update(cx, |state, cx| state.set_value(value, window, cx))` 并更新 `last_synced`

**循环防护**：
- `set_value` 内部设 `emit_events = false`（已确认），不会触发 `InputEvent::Change`，无递归循环
- 反向闭包内 bump_version 后立即更新 `__rml_input_state_versions`，render 时版本号相等，跳过冗余 set_value
- `#[command]` 改字段 bump_version 但不更新 `__rml_input_state_versions`，render 时版本号不等，触发正向 set_value

**生成的 match arms**（由 `ctx.observable_fields` + `ctx.field_types` 驱动）：
```rust
// 初始值/正向同步 match
let initial_value: gpui::SharedString = match field {
    "name" => self.name.clone().into(),        // String → SharedString
    "age" => self.age.to_string().into(),       // i32 → String → SharedString
    _ => gpui::SharedString::default(),
};
// 反向赋值 match
match field {
    "name" => { this.name = value.to_string(); this.__rml_bump_version("name"); }
    "age" => { this.age = value.parse::<i32>().unwrap_or(0); this.__rml_bump_version("age"); }
    _ => {}
}
```

**新增辅助函数** `gen_field_value_expr(field, ty) -> String`：
- 数字类型：`self.{field}.to_string().into()`
- 其他（String/SharedString）：`self.{field}.clone().into()`

**新增辅助函数** `gen_field_assign_expr(field, ty) -> String`：
- 数字类型：`this.{field} = value.parse::<{ty}>().unwrap_or(0)`（i32/u32/i64/u64/isize/usize）、`.unwrap_or(0.0)`（f32/f64）
- bool：`this.{field} = !value.is_empty()`
- 其他：`this.{field} = value.to_string()`（统一用 to_string，避免 SharedString→String 的 Into 不确定性）

#### Step 4: 重写 `gen_model_input`

**文件**：`crates/engine/src/compiler/codegen.rs` L502-549

**生成的代码**（简化）：
```rust
rml_ui::Input::new(&self.__rml_get_or_init_input_state(
    "name",           // field name
    Some("姓名"),     // placeholder (Option<&'static str>)
    _window,          // render 签名中的 window 参数
    cx,               // Context<Self>
))
.disabled(false)      // 仅当 disabled 属性存在时生成
```

**移除的调用**：
- `.value(...)` — 正向绑定由 `__rml_get_or_init_input_state` 内部处理
- `.on_change(...)` — 反向绑定由 `__rml_get_or_init_input_state` 内部 subscribe 处理
- `.placeholder(...)` — placeholder 作为参数传入 helper，在 `InputState::new()` 创建时设置

**保留的属性处理**：
- `disabled` → `.disabled(true/false)`（Input 有此方法）
- `placeholder` → 提取为 `Some("...")` 传入 helper
- 其他静态属性 → 按现有逻辑处理

#### Step 5: 删除 `gen_value_forward` 和 `gen_value_reverse`

**文件**：`crates/engine/src/compiler/codegen.rs` L555-580

**原因**：这两个函数生成 `.value(self.field.clone())` 和 `this.field = state.value().into()` 表达式，已被 Step 3 的 `gen_field_value_expr` 和 `gen_field_assign_expr` 替代。删除避免死代码。

### 阶段 2-B：更新测试 + 验证

#### Step 6: 更新 `codegen_two_way_binding_test.rs`

**文件**：`crates/engine/tests/codegen_two_way_binding_test.rs`

**更新内容**：
1. `gen_model_input_uses_get_or_init_input_state` — 保留，断言 `__rml_get_or_init_input_state` 调用 + `&self.` 引用传递
2. `gen_model_input_generates_type_conversion_for_i32` — 更新断言：`parse::<i32>().unwrap_or(0)` 仍存在（在 `gen_input_state_impl` 生成的 match 中）
3. `gen_model_input_generates_into_for_string` — 更新：`state.value().into()` → `value.to_string()`（闭包参数名变化 + 使用 to_string）
4. `gen_model_input_includes_bump_version_and_notify` — 保留，断言 `__rml_bump_version` + `cx.notify()`
5. `gen_input_state_impl_generates_helper_method` — 更新：方法签名包含 `window: &mut gpui::Window` 和 `placeholder: Option<&'static str>`
6. `gen_model_input_supports_multiple_inputs` — 保留
7. `gen_model_input_preserves_placeholder_attribute` — 更新：placeholder 不再作为 `.placeholder("...")` 链式调用，而是作为 `Some("...")` 参数传入 helper
8. `gen_model_input_floating_point_types` — 更新：`parse::<f64>().unwrap_or(0.0)` 仍存在

**新增测试**：
- `gen_input_state_impl_includes_subscribe` — 验证生成代码包含 `cx.subscribe` + `InputEvent::Change`
- `gen_input_state_impl_includes_set_value` — 验证生成代码包含 `set_value` 正向同步
- `gen_input_state_impl_includes_version_tracking` — 验证生成代码包含 `__rml_input_state_versions` 对比逻辑

#### Step 7: 修复其他测试辅助函数

**文件**：
- `crates/engine/tests/codegen_observable_test.rs` — `make_ctx()` 已有 `field_types: HashMap::new()`，无需改
- `crates/engine/src/compiler/event.rs` 测试辅助 — 已有 `field_types: HashMap::new()`，无需改
- `crates/engine/src/compiler/component.rs` 测试辅助 — 同上

#### Step 8: 验证 demo 编译 + 运行

```bash
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

验证点：
- demo 编译通过（无 6 个 API 不匹配错误）
- 两个 input 框显示，placeholder 正确
- 在姓名 input 中输入文字 → `profile_summary` 实时更新
- 在年龄 input 中输入数字 → `profile_summary` 实时更新
- 输入非法字符到年龄框 → 兜底为 0（不崩溃）

#### Step 9: 全工作区测试

```bash
cargo test --workspace
```

预期：所有现有测试 + 更新的 8 个双向绑定测试 + 3 个新增测试通过。

### 阶段 3：宏 API 文档 + 用户指南

#### Step 10: 更新 `docs/04-code-behind/macros.md`

**文件**：`docs/04-code-behind/macros.md`（已存在）

**新增/更新内容**：
- `#[window]` 宏：注入 `__rml_window_handle` + `__rml_input_states` + `__rml_input_subscriptions` + `__rml_input_state_versions` 字段（含 `Default` 兼容性说明）
- `#[component]` 宏：同上
- `#[command]` 宏：`no_notify` 参数语义 + 自动 `bump_version` + `cx.notify()` 注入条件（返回类型为 `()` + 有 `&mut Context<Self>` 参数）
- `#[computed]` 宏：方法重命名为 `__rml_computed_<name>` + 生成版本缓存包装 + 依赖追踪
- 宏注入字段完整清单表（字段名、类型、用途、用户不可直接访问）

#### Step 11: 重写 `docs/03-binding/two-way-binding.md`

**文件**：`docs/03-binding/two-way-binding.md`（已存在）

**重写内容**：
- 双向绑定语法：`<input model={field_name} placeholder="..." />`
- 工作原理（三阶段）：
  1. 初始正向同步：首次 render 时从字段值初始化 InputState
  2. 反向同步（UI→VM）：`InputEvent::Change` → 字段赋值 + bump_version + notify
  3. 正向同步（VM→UI）：render 时版本号对比，`#[command]` 改字段后自动 set_value
- 支持的字段类型表（i32/u32/i64/u64/f32/f64/usize/isize/bool/String/SharedString）+ 转换规则
- 循环防护机制：`set_value` 内部 `emit_events=false` + 版本号标记
- 完整示例（引用 demo 的 name/age 绑定）
- 限制说明：placeholder 仅支持静态字符串（不支持绑定表达式）

#### Step 12: 更新 `docs/10-advanced/performance.md`

**文件**：`docs/10-advanced/performance.md`（Phase 1 已重写）

**补充内容**：
- 双向绑定的性能特性：InputState entity 复用（HashMap 惰性初始化）、subscription 长期持有、正向同步仅在版本号变化时触发
- 细粒度更新与双向绑定的协作：`#[command]` bump_version → computed 属性缓存失效 → 仅重算依赖字段

## Assumptions & Decisions

1. **正向同步策略**：在 `__rml_get_or_init_input_state` 内部于 render 时对比版本号，而非在 `#[command]` 宏中注入 `set_value` 调用。原因：command 上下文无 `window` 参数（`set_value` 需要），render 上下文有。
2. **placeholder 限制**：仅支持静态字符串（RML `placeholder="..."` 属性），不支持绑定表达式（`placeholder={some_field}`）。原因：`InputState::placeholder()` 是 builder 方法，仅在 entity 创建时可用，若支持动态 placeholder 需在每次 render 比较并重建 entity，复杂度不匹配收益。
3. **反向闭包类型转换**：String 字段统一用 `value.to_string()`（而非 `value.into()`），避免 `SharedString → String` 的 `Into` 实现不确定性。
4. **match arms 覆盖所有 pub 字段**：`gen_input_state_impl` 为所有 `observable_fields` 生成 match arms，未绑定的字段为死代码（`#[allow(dead_code)]` 已覆盖）。简化 codegen 逻辑，无需追踪哪些字段实际被 `<input model={...}>` 绑定。
5. **不删除 `gen_value_forward`/`gen_value_reverse` 的测试**：这两个函数被删除后，相关测试断言移入 `gen_input_state_impl` 的测试中验证生成代码内容。
6. **文档语言**：保持中文（与现有 docs 一致），代码示例用英文注释。

## Verification Steps

1. `cargo build -p rust-rml-demo` — 编译通过，无 API 不匹配错误
2. `cargo test --workspace` — 所有测试通过（现有 192+ 单元测试 + 7 集成测试 + 8 更新 + 3 新增双向绑定测试）
3. `cargo run -p rust-rml-demo` — demo 启动，两个 input 框可交互，`profile_summary` 实时更新
4. 手动验证循环防护：在 input 中快速输入，确认无卡顿/死循环
5. 手动验证正向同步：若有 `#[command]` 修改 `name` 字段，input 框内容应同步更新（demo 暂无此场景，但 codegen 已支持）

## File Change Summary

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/ui/src/lib.rs` | 编辑 | re-export `InputEvent` |
| `crates/macros/src/component.rs` | 编辑 | 注入 `__rml_input_state_versions` 字段 |
| `crates/engine/src/compiler/codegen.rs` | 编辑 | 重写 `gen_model_input` + `gen_input_state_impl`，删除 `gen_value_forward`/`gen_value_reverse`，新增 `gen_field_value_expr`/`gen_field_assign_expr` |
| `crates/engine/tests/codegen_two_way_binding_test.rs` | 编辑 | 更新 8 个测试断言 + 新增 3 个测试 |
| `docs/04-code-behind/macros.md` | 编辑 | 宏 API 参考文档 |
| `docs/03-binding/two-way-binding.md` | 编辑 | 双向绑定用户指南 |
| `docs/10-advanced/performance.md` | 编辑 | 补充双向绑定性能特性 |
