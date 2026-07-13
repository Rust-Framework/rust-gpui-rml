# Select 组件双下拉框问题修复计划

## 1. 问题摘要

在 RML 案例页 `demo/src/cases/select_case.rml` 中，所有 `<Select>` 均位于 `<template slot="demo">` 内。点击 Select 触发下拉后，界面上同时出现两个下拉框（一个正常锚定、一个错位停留在左下角）。该问题仅出现在 slot 场景，非 slot 内直接使用 Select 无此现象。

## 2. 根因（已确认）

### 2.1 直接根因：`is_none()` 守卫检查了错误的 RmlState

`user_component.rs` 生成的 slot 注入守卫代码：
```rust
// self 是父视图（SelectCase），__rml_entity 是子组件（CaseDocPage）的 Entity
if self.__rml_state.slot("demo").is_none() {           // ← 检查 SelectCase 的 RmlState
    let __rml_slot_demo_value: SlotRenderer = Box::new(...);
    __rml_entity.update(cx, |this, _cx| {
        this.__rml_set_slot_demo(__rml_slot_demo_value); // ← 设置 CaseDocPage 的 RmlState
    });
}
```

- `self.__rml_state` 是**父视图 SelectCase** 的 RmlState。
- `__rml_entity.update(...)` 内的 `this.__rml_set_slot_demo(...)` 设置的是**子组件 CaseDocPage** 的 RmlState。
- 两个 RmlState 是**不同的实例**，`self.__rml_state.slot("demo")` 永远返回 `None`。

**后果**：每次 SelectCase 渲染（如切换尺寸触发重渲染），守卫永远为 true，重新创建 slot 闭包并覆盖 CaseDocPage 的 slot。旧 slot 闭包被丢弃时，其内部 Select 的 `deferred` 下拉框未被正确回收；新 slot 闭包引用同一个 `open == true` 的 SelectState，又产出一个新的 `deferred` 下拉框 → 双下拉框。

### 2.2 验证

生成代码 `select_case.rs` 确认：
- 第 53 行：`if self.__rml_state.slot("demo").is_none()` — SelectCase 的 RmlState
- 第 106 行：`__rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_demo(...); })` — CaseDocPage 的 RmlState

两者操作不同的 RmlState 实例，守卫失效。

3. **Select 渲染与 deferred 弹出层**：`gpui-component` `crates/ui/src/select.rs`
   - `Select::new` 给外层 div 设置了基于 state entity id 的稳定 id：`("select", state.entity_id())`（select.rs:595-602）。
   - `SelectState::render` 在 `self.state.open == true` 时产出 `deferred(anchored(...))` 弹出层（select.rs:547-585）。
   - **关键缺陷**：该 `deferred(...)` 调用本身没有显式 `.id(...)`，下拉框元素缺少独立稳定身份。
   - `set_open` 通过 `GlobalState::register_deferred_popover(&self.state.focus_handle)` 登记弹窗（select.rs:394-404）。

### 2.2 根因

问题由**两层身份缺失**叠加导致：

1. **外层身份缺失**：`user_component.rs` 生成的 slot 返回表达式原本没有稳定根 ID（生成的 `select_case.rs` 中 slot 根节点是 `gpui::div().flex().flex_col().gap(...)`，无 `.id(...)`）。父组件重渲染时，GPUI 在 reconciliation 阶段无法将跨帧的 slot 子树识别为同一棵树。

2. **内层身份缺失**：`SelectState::render` 内部的下拉框 `deferred(...)` 元素没有稳定 `ElementId`。`deferred` 元素在 GPUI 中处于全局延迟渲染层，当宿主元素树因 slot 闭包重新执行而被重建时，没有 ID 的 `deferred` 元素无法被 reconcile 为同一个实例。

结果：旧 slot 子树销毁时其 `deferred` 下拉框未能正确回收；新 slot 子树构建时同一个 `SelectState` 实体仍 `open == true`，于是又产出一个新的 `deferred` 下拉框。两个下拉框同时可见，形成“双下拉框”。

`user_component.rs:197-198` 的注释也印证了该现象：
> “每帧替换 SlotRenderer 会在 Select/Combobox 等 deferred 弹出层仍打开时丢弃旧闭包，遗留错位第二层菜单（左下角锚点失效）。”

### 2.3 与 ref/value 共存的关系（非直接根因，但需加固）

`Select` 属于 `StatefulWithDelegate`，支持两种用法：
- `ref="x"` + `items={...}`：由 `gen_stateful_with_delegate_body` 生成，State 实体存入 `RmlState.ref_entities`。
- `value={...}` + `items={...}`：由 `gen_model_delegate_state_bridge` 生成，State 实体存入 `RmlState.state_bridge_entities`。

当前 `ref` 存在时直接走 ref 路径，`value` 绑定会被静默忽略。若用户同时写 `ref` 和 `value`，语义冲突会导致调试困难，需在编译期报错。

## 3. 修复方案

### 3.1 P0：为 slot 内容根节点生成稳定 ID

**目标**：让 GPUI 能在父组件重渲染时识别 slot 子树的同一性，为 reconcile 提供外层锚点。

**修改文件**：`crates/engine/src/compiler/translator/user_component.rs:200-210`

**修改内容**：
具名 slot 与 default slot 的返回表达式统一外包一层带 ID 的 `div()`：
```rust
(gpui::div()
    .id("__rml_slot_<slot_name>")
    .child({slot_code_replaced}))
.into_any_element()
```
其中 `<slot_name>` 在 codegen 阶段拼接为 `"demo"` / `"default"` 等字符串。

**注意**：当前 `user_component.rs` 源码中已存在类似的 `.id({slot_id:?})` 代码，但最新生成的 `select_case.rs` 中仍未出现该 ID，说明生成代码尚未重新产出或代码未实际生效。本步骤需先确认源码逻辑正确，再重新 build 验证生成结果。

**理由**：
- 无论 slot 内容是单节点还是多节点，最终都返回一个带稳定 ID 的根元素。
- ID 以 `"__rml_slot_<slot_name>"` 构成，slot 名在组件内唯一，能保证同一组件实例内同一 slot 的根节点身份稳定。
- 使用 `&'static str` 可直接 `Into<ElementId>`，避免元组类型不匹配问题。

### 3.2 P0 Fallback：为 gpui-component 的 deferred 下拉框分配稳定 ID

**目标**：`Deferred` 元素不支持 `.id()` setter（`Element::id()` 是 getter），改为用带稳定 ID 的 `div()` 包裹 `deferred`，为 GPUI reconciliation 提供锚点。

**修改文件**：
- `third_party/gpui-component/crates/ui/src/select.rs:547-591`
- `third_party/gpui-component/crates/ui/src/combobox.rs:661-681`

**修改内容**：
将 `SelectState::render` 中原来的：
```rust
.when(self.state.open, |this| {
    this.child(
        deferred(
            anchored().snap_to_window_with_margin(px(8.)).child(...)
        )
        .with_priority(1),
    )
})
```
改为：
```rust
.when(self.state.open, |this| {
    this.child(
        div()
            .id(("select_popup", cx.entity_id()))
            .child(
                deferred(
                    anchored().snap_to_window_with_margin(px(8.)).child(...)
                )
                .with_priority(1),
            ),
    )
})
```

Combobox 同理用 `div().id(("combobox_popup", cx.entity_id())).child(deferred(...))`。

**集成方式**：
- 将 `gpui-component` 对应版本复制到项目 `third_party/gpui-component` 目录；
- 修改 `crates/ui/Cargo.toml` / `crates/macros/Cargo.toml` / `crates/assets/Cargo.toml` 内联 workspace 依赖（去除 `workspace = true` 继承），避免根 workspace 依赖冲突；
- 在根 `Cargo.toml` 添加 `[patch."https://github.com/longbridge/gpui-component.git"]` 覆盖 gpui-component 来源。

**验证结果**：
- `cargo check -p rust-rml-demo` 通过
- `cargo test -p rust-rml-engine --lib` 1343 passed
- `cargo build -p rust-rml-demo` 成功

### 3.3 P1：禁止 `ref` 与 `value` 绑定同时用于 Select/Combobox

**目标**：在编译期拒绝语义冲突的写法，避免用户误以为双向绑定生效。

**修改文件**：`crates/engine/src/compiler/translator/component/stateful.rs:64-79`

**修改内容**：在提取 `ref_name` 后增加检查（当前源码已存在，需确认其覆盖所有 `StatefulWithDelegate` 组件并补充单测）：
```rust
if let Some(spec) = lookup_state_bridge_for_tag(canonical.as_str()) {
    if ref_name.is_some()
        && elem.attributes.iter().any(|attr| {
            matches!(attr, Attribute::Bind { name, .. } if name == spec.bind_property)
        })
    {
        return Err(CodegenError {
            message: format!(
                "<{}> cannot use both 'ref' and '{}' binding; use 'ref' with on-change or '{}' without ref",
                tag, spec.bind_property, spec.bind_property
            ),
            span: Some(elem.span),
        });
    }
}
```

### 3.4 P1：运行时诊断（可选，用于验证）

**目标**：在修复前后观察 slot 闭包执行频率与 deferred 弹窗注册情况。

**方案 A**：在 `user_component.rs` 生成的 slot 闭包首行临时插入 `println!`，观察父组件重渲染时闭包是否被反复调用。

**方案 B**：临时在 `gpui-component` 的 `SelectState::set_open` 中增加日志，检测同一 `focus_handle` 在未 `unregister` 前被重复注册。

本计划以 **方案 A** 作为验证手段，不纳入正式代码变更。

## 4. 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `crates/engine/src/compiler/translator/user_component.rs` | 修改 | slot 闭包返回表达式外包带 ID 的 `div()`（`__rml_slot_<name>`） |
| `crates/engine/src/compiler/translator/component/stateful.rs` | 已完成 | `ref` + `value` 共存编译期检查（第 64-79 行） |
| `third_party/gpui-component/crates/ui/src/select.rs` | 修改 | `div().id(("select_popup", entity_id)).child(deferred(...))` 包裹 |
| `third_party/gpui-component/crates/ui/src/combobox.rs` | 修改 | `div().id(("combobox_popup", entity_id)).child(deferred(...))` 包裹 |
| `third_party/gpui-component/crates/ui/Cargo.toml` | 修改 | 内联 workspace 依赖，去除 `workspace = true` |
| `third_party/gpui-component/crates/macros/Cargo.toml` | 修改 | 内联 `edition`，去除 `[lints]` workspace 继承 |
| `third_party/gpui-component/crates/assets/Cargo.toml` | 修改 | 内联 workspace 依赖，去除 `workspace = true` |
| 根 `Cargo.toml` | 修改 | 添加 `[patch."https://github.com/longbridge/gpui-component.git"]` |
| `demo/src/cases/select_case.rml` | 无需修改 | 当前写法合法，用于验证 |

## 5. 验证步骤与执行状态

### 5.1 已完成的验证

| 步骤 | 状态 | 结果 |
|------|------|------|
| 源码审查：`user_component.rs` 生成 slot 根 ID | 完成 | 具名 slot 与 default slot 均生成 `gpui::div().id("__rml_slot_<name>").child(...)` |
| 源码审查：`stateful.rs` ref + value 检查 | 完成 | 第 64-79 行已存在，覆盖 Select/Combobox |
| 单元测试：`translator::user_component::tests` | 通过 | 24 个测试全部通过，含 `test_named_slot_root_has_stable_id` / `test_default_slot_root_has_stable_id` |
| 单元测试：`translator::component::stateful::tests` | 通过 | 3 个测试全部通过，覆盖 ref+value 冲突与合法 ref 模式 |
| 全量引擎测试 | 通过 | `cargo test -p rust-rml-engine --lib`：1343 passed; 0 failed |
| Demo 构建 | 通过 | `cargo build -p rust-rml-demo` 成功 |
| 生成代码审查 | 通过 | `select_case.rs` 第 54 行出现 `gpui::div().id("__rml_slot_demo").child(...)` |

### 5.2 待完成的运行时验证

以下步骤需在 GUI 可运行环境下执行：

1. 运行 demo，进入 Select 案例页。
2. 依次点击 5 个 Select，确认每次只出现一个下拉框。
3. 打开下拉后点击空白处或按 ESC 关闭，确认无残留下拉框。
4. 循环切换尺寸（触发父组件重渲染）后再点击 Select，确认无双重下拉框。
5. **若通过，则 3.2（gpui-component patch）不需要实施。**
6. 若仍出现双下拉框，实施 3.2 的 gpui-component patch 后重复上述步骤。

### 5.3 编译期检查验证（可选）

临时在 `select_case.rml` 写一个同时带 `ref` 和 `value` 的 Select：
```rml
<Select ref="bad_select" items={basic_items} value={bound_fruit} />
```
build 应失败，并输出明确错误信息：`<Select> cannot use both 'ref' and 'value' binding; ...`
验证后移除该临时代码。

## 6. 假设与决策

- **假设 1**：GPUI 的 deferred 元素 reconcile 失败由 slot 根节点 ID 缺失和/或 deferred 元素自身 ID 缺失共同导致。外层稳定 ID 可能足以让 GPUI 识别同一 slot 子树并正确回收其 deferred 弹出层。
- **假设 2**：`"__rml_slot_<slot_name>"` 作为根 ID 在同一组件实例内足够唯一。不同组件实例拥有独立渲染树，不会冲突。
- **决策 1**：始终用 `div().id(...).child(...)` 包裹 slot 内容，而不尝试给单节点 slot 的已有元素加 ID，避免处理文本节点、非 `ElementId` 组件等复杂情况。
- **决策 2**：优先不修改 gpui-component 源码；仅在 slot 根节点 ID 修复验证失败时，通过本地 patch 为 gpui-component 的 Select/Combobox deferred 元素补稳定 ID。
- **决策 3**：`ref` + `value` 编译期检查覆盖所有 `StatefulWithDelegate` 组件（当前为 Select/Combobox），而非仅 Select。
