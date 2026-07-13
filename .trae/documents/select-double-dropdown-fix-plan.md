# Select 组件双下拉框问题修复计划

## 1. 问题摘要

在 RML 案例页 `demo/src/cases/select_case.rml` 中，所有 `<Select>` 均位于 `<template slot="demo">` 内。点击 Select 触发下拉后，界面上同时出现两个下拉框（一个正常锚定、一个错位停留在左下角）。该问题仅出现在 slot 场景，非 slot 内直接使用 Select 无此现象。

## 2. 代码层面根因分析

### 2.1 关键代码路径

1. **slot 闭包生成位置**：`crates/engine/src/compiler/translator/user_component.rs:178-209`
   - 生成代码形如：
     ```rust
     if self.__rml_state.slot("demo").is_none() {
         let __rml_slot_demo_value: SlotRenderer = Box::new({
             let __rml_self_entity = __rml_self_entity.clone();
             move |_scope, _window, _app| -> gpui::AnyElement {
                 __rml_self_entity.update(_app, |this, cx| {
                     let __rml_self_ref: &Self = this;
                     (gpui::div().flex().flex_col().gap(...).child(...Select::new(...))...)
                         .into_any_element()
                 })
             }
         });
         __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_demo(__rml_slot_demo_value); });
     }
     ```
   - `SlotRenderer` 对象仅注入一次（`is_none()` 守卫），但**闭包体在父组件（CaseDocPage）每次重渲染时都会重新执行**。`rml_core::slot.rs:104` 的注释明确说明：`SlotRenderer` "每次调用生成新的 `AnyElement`"。

2. **slot 渲染位置**：子组件（CaseDocPage）渲染 `<slot name="demo" />` 时调用上述闭包，闭包内重新构造 `Select::new(&entity)` 等 `RenderOnce` 元素。

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

**目标**：如果仅修复 slot 根节点 ID 后仍出现双下拉框，则从根因层面为 `deferred` 元素本身提供稳定身份。

**修改文件**：
- `gpui-component` `crates/ui/src/select.rs:549` 附近
- `gpui-component` `crates/ui/src/combobox.rs:664` 附近（Combobox 与 Select 同构，存在相同风险）

**修改内容**：
将 `SelectState::render` 中：
```rust
this.child(
    deferred(
        anchored().snap_to_window_with_margin(px(8.)).child(...)
    )
    .with_priority(1),
)
```
改为：
```rust
this.child(
    deferred(
        anchored().snap_to_window_with_margin(px(8.)).child(...)
    )
    .id(("select_popup", cx.entity_id()))
    .with_priority(1),
)
```

Combobox 同理添加 `.id(("combobox_popup", cx.entity_id()))`。

**集成方式**：
- 将 `gpui-component` 对应版本复制到项目 `third_party/gpui-component` 目录；
- 在根 `Cargo.toml` 添加 `[patch.crates-io]` 覆盖 gpui-component 来源：
  ```toml
  [patch.crates-io]
  gpui-component = { path = "third_party/gpui-component/crates/ui" }
  gpui-component-assets = { path = "third_party/gpui-component/crates/assets" }
  ```
- 修改后 `cargo update -p gpui-component` 并重新 build。

**决策**：先实施 3.1 并验证；若 3.1 单独即可修复，则暂缓 3.2，仅作为记录。若 3.1 不足，必须实施 3.2。

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
| `crates/engine/src/compiler/translator/user_component.rs` | 修改 | 确保 slot 闭包返回表达式外包带 ID 的 `div()`；同步更新单测 |
| `crates/engine/src/compiler/translator/component/stateful.rs` | 修改/验证 | 确认 `ref` 与 `value` 绑定共存的编译期错误已存在并覆盖 Select/Combobox |
| `demo/src/cases/select_case.rml` | 无需修改 | 当前写法合法，用于验证 |
| `target/debug/build/rust-rml-demo-*/out/rml_generated/select_case.rs` | 重新生成 | build 后自动生成，用于检查生成代码是否带稳定 ID |
| `third_party/gpui-component/...`（条件性） | 新增/修改 | 若 3.1 不足，复制并 patch gpui-component，为 Select/Combobox 的 deferred 加 id |
| 根 `Cargo.toml`（条件性） | 修改 | 若实施 3.2，添加 `[patch.crates-io]` 覆盖 gpui-component |

## 5. 验证步骤

1. **源码审查**
   - 确认 `user_component.rs` 中具名 slot 与 default slot 均生成 `gpui::div().id("__rml_slot_xxx").child(...)`。
   - 确认 `stateful.rs` 中 `ref` + `value` 检查逻辑存在。

2. **编译检查**
   ```powershell
   cargo check -p rust-rml-engine
   cargo test -p rust-rml-engine --lib translator::user_component
   ```

3. **生成代码审查**
   - build demo 后打开 `target/debug/build/rust-rml-demo-*/out/rml_generated/select_case.rs`。
   - 确认 slot 闭包返回表达式形如：
     ```rust
     (gpui::div().id("__rml_slot_demo").child(...)).into_any_element()
     ```

4. **运行时验证（3.1 是否足够）**
   - 运行 demo，进入 Select 案例页。
   - 依次点击 5 个 Select，确认每次只出现一个下拉框。
   - 打开下拉后点击空白处或按 ESC 关闭，确认无残留下拉框。
   - 循环切换尺寸（触发父组件重渲染）后再点击 Select，确认无双重下拉框。
   - **若通过，则 3.2 不需要实施。**

5. **运行时验证（3.2 Fallback）**
   - 若 4 中出现双重下拉框，实施 3.2 的 gpui-component patch。
   - 重复 4 的验证步骤，确认问题消失。

6. **编译期检查验证**
   - 临时在 `select_case.rml` 写一个同时带 `ref` 和 `value` 的 Select：
     ```rml
     <Select ref="bad_select" items={basic_items} value={bound_fruit} />
     ```
   - build 应失败，并输出明确错误信息：`<Select> cannot use both 'ref' and 'value' binding; ...`
   - 验证后移除该临时代码。

7. **回归测试**
   ```powershell
   cargo test -p rust-rml-engine --lib
   cargo build -p rust-rml-demo
   ```

## 6. 假设与决策

- **假设 1**：GPUI 的 deferred 元素 reconcile 失败由 slot 根节点 ID 缺失和/或 deferred 元素自身 ID 缺失共同导致。外层稳定 ID 可能足以让 GPUI 识别同一 slot 子树并正确回收其 deferred 弹出层。
- **假设 2**：`"__rml_slot_<slot_name>"` 作为根 ID 在同一组件实例内足够唯一。不同组件实例拥有独立渲染树，不会冲突。
- **决策 1**：始终用 `div().id(...).child(...)` 包裹 slot 内容，而不尝试给单节点 slot 的已有元素加 ID，避免处理文本节点、非 `ElementId` 组件等复杂情况。
- **决策 2**：优先不修改 gpui-component 源码；仅在 slot 根节点 ID 修复验证失败时，通过本地 patch 为 gpui-component 的 Select/Combobox deferred 元素补稳定 ID。
- **决策 3**：`ref` + `value` 编译期检查覆盖所有 `StatefulWithDelegate` 组件（当前为 Select/Combobox），而非仅 Select。
