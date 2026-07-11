# gpui-component 深度审查迭代计划

## 概述

延续 v3 审查计划，本迭代完成剩余工作：A2 修复（props_registry 遗漏）、A3（SliderTranslator 声明式 min/max/step/default_value）、B2（事件 API 补全载荷类型）、B3（清理 demo 描述中的 codegen 内部术语），以及最终构建验证。

## 当前状态分析

### A1（Accordion open_indices 重命名）— 已完成 ✓

### A2（Tree StatefulWithDelegate 转换）— 代码修改完成，但存在关键遗漏

已完成：
- `tags.rs`：Tree 已改为 `StatefulWithDelegate { state_field: "tree_state", state_ctor: "move |_w, c| rml_ui::TreeState::new(c).items(__rml_delegate)", delegate_attr: "items" }`
- `translator/component/tree.rs`：完全重写，生成 `get_or_init_ref` + delegate 注入 + `Tree::new(Some(&__rml_entity))`
- `components/tree/gen.rs`：已删除
- `components/tree/mod.rs`：已更新（仅保留 setters）
- `tree_case.rml`：已重写（`ref="basic_tree" items={tree_items}`）
- `tree_case.rml.rs`：已重写（`ElementRef<TreeState>` + `Vec<TreeItem>`）

**关键遗漏（构建阻断）**：
- `props_registry.rs` L104-105：Tree 的 props 仍为 `["on_activate", "on_select"]`，缺少 `"items"`。注释仍写"Stateful 组件，数据由 TreeState Entity 提供，不支持 items 绑定"——与 A2 转换矛盾。
- `props_registry.rs` L412-415 测试：`assert!(!is_prop_registered("Tree", "items"))` 断言 Tree 不支持 items，与 A2 行为相反。
- `tree_case.rml` L4 描述：仍写"Stateful 组件"——应更新为用户视角描述（也属 B3 范畴）。

### A3（SliderTranslator）— 未开始

当前 Slider 用法反人类：min/max/step/default_value 需在 `on_loaded` 中手动链式调用 `SliderState::new().min(0.0).max(100.0).step(1.0).default_value(50.0)`，无法在 `.rml` 中声明式设置。

关键发现：
- Slider 在 `state_bridge.rs` 已注册（`value={field}` 双向绑定），但 `on_change` 事件**未在 `state_event.rs` 注册**——意味着 `<Slider on-change={handler} />`（无 value 绑定时）事件不会被订阅。
- `stateful.rs` L37 排除列表未包含 "Slider"。
- `mod.rs` 未注册 `slider` 模块。
- `props_registry.rs` 无 Slider 条目。

### B2（事件 API 补全载荷类型）— 未开始

以下 demo `.rml.rs` 文件的事件 API 行缺少载荷类型（仅写"事件"、"点击回调"等，未标注参数类型）：
- `avatar_case.rml.rs` L48：`("on-click", "事件", "点击回调")`
- `button_case.rml.rs` L49：`("on-click", "事件", "点击回调")`
- `counter_case.rml.rs` L42：`("on-click", "事件", "按钮点击回调")`
- `sidebar_case.rml.rs` L70：`("on-click", "事件", "点击事件回调")`
- `tag_case.rml.rs` L44：`("on-click", "事件", "点击回调")`

### B3（清理 demo 描述中的 codegen 内部术语）— 未开始

以下 `.rml` 文件描述包含 codegen 内部术语（`normalize_component_tag`、`Stateful 模式`、`Stateful 组件`、`StatelessWithItems`、`codegen` 等），违反"描述面向组件使用者而非框架实现者"原则：

| 文件 | 术语 |
|------|------|
| `calendar_case.rml` | `Stateful 模式` |
| `code_editor_case.rml` | `Stateful 模式` |
| `date_picker_case.rml` | `Stateful 模式` |
| `color_picker_case.rml` | `Stateful 模式` |
| `input_case.rml` | `Stateful 模式`、`codegen` |
| `tree_case.rml` | `Stateful 组件` |
| `slider_case.rml` | `Stateful 模式`（A3 重写后覆盖） |
| `stepper_case.rml` | `StatelessWithItems` |
| `otp_input_case.rml` | `Stateful 组件` |
| `number_input_case.rml` | `Stateful 模式` |
| `avatar_case.rml` | `normalize_component_tag` |
| `alert_case.rml` | `normalize_component_tag` |
| `hover_card_case.rml` | `normalize_component_tag` |
| `accordion_case.rml` | `normalize_component_tag`、`StatelessWithItems` |
| `table_case.rml` | `normalize_component_tag` |
| `sheet_case.rml` | `normalize_component_tag`、`codegen` |
| `popover_case.rml` | `normalize_component_tag` |
| `content_binding_case.rml` | `codegen` |
| `key_case.rml` | `codegen` |
| `user_component_event_case.rml` | `codegen` |

清理原则：
- `normalize_component_tag 统一为 X` → `X 标签等价`（或直接删除该句）
- `Stateful 模式` / `Stateful 组件` → 删除 codegen 分类，改为用户视角描述（如"通过 ref 引用状态"）
- `StatelessWithItems` → 删除或改为"子节点通过 builder 模式注入"
- `codegen` → `编译器` 或改为用户视角描述

---

## 实施步骤

### 步骤 1：A2 修复 — props_registry 一致性（关键）

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)

1. **L104-105**：更新 Tree 条目和注释
   - 旧：`("Tree", &["on_activate", "on_select"]),` + 注释"Stateful 组件，数据由 TreeState Entity 提供，不支持 items 绑定"
   - 新：`("Tree", &["items", "on_activate", "on_select"]),` + 注释"StatefulWithDelegate 组件，items 为委托数据绑定属性"

2. **L412-415**：修复测试断言
   - 旧：`assert!(!is_prop_registered("Tree", "items"));` + 注释"Tree 是 Stateful 组件..."
   - 新：`assert!(is_prop_registered("Tree", "items"));` + 注释"Tree 是 StatefulWithDelegate 组件，items 为委托数据绑定属性"

**验证**：`cargo test -p rust-rml-engine --lib props_registry`

### 步骤 2：A3 — SliderTranslator 声明式 min/max/step/default_value

#### 2a. 注册 Slider on_change 到 state_event.rs

**文件**：[state_event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/state_event.rs)

在 `STATE_EVENT_REGISTRY` 数组末尾添加 Slider 条目：
```rust
StateEventSpec {
    tag: "Slider",
    event_name: "on_change",
    event_type: "rml_ui::SliderEvent",
    event_variant: "Change",
    payload_binding: "value",
    call_template: "this.{method}((*value).clone(), cx)",
},
```

添加测试：`is_state_event_recognizes_slider` + `gen_subscribe_slider_change`

**原因**：当前 Slider 的 on_change 仅在 StateBridge（value={field}）路径下被订阅。无 value 绑定时 `<Slider on-change={handler} />` 事件不会被订阅——这是功能缺陷。

#### 2b. 创建 slider.rs translator

**新文件**：`crates/engine/src/compiler/translator/component/slider.rs`

仿照 `input.rs` 模式：
1. 若 `value={field}` 存在 → 委托 `gen_model_state_bridge`（StateBridge 路径）
2. 否则 → 构建 custom state_ctor 注入 min/max/step/default_value

```rust
const SKIP_ATTRS: &[&str] = &["min", "max", "step", "default_value"];
```

state_ctor 构建逻辑：
- 提取 `min`/`max`/`step`：Static 属性，解析为 f32（`"0"` → `0.0f32`）
- 提取 `default_value`：
  - Static `"50"` → `.default_value(50.0f32)`（单值滑块）
  - Bind `{range_default}` → `{ let __rml_default_value = (self.range_default).clone(); move |_w, _c| ...default_value(__rml_default_value) }`（范围滑块，字段类型 `(f32, f32)` 或 `SliderValue`）
- 构建链式 builder：`move |_w, _c| rml_ui::SliderState::new().min(0.0).max(100.0).step(1.0).default_value(50.0)`

调用 `gen_stateful_body`（处理 on_change 事件订阅，通过 state_event 注册）+ setter 循环（跳过 SKIP_ATTRS）。

辅助函数（复用 input.rs 模式）：
- `extract_static_f32(elem, name) -> Option<f32>`：从 Static 属性解析 f32
- `extract_static_string(elem, name) -> Option<String>`：复用 input.rs 的同名函数模式
- `build_slider_state_ctor(elem, min, max, step, default_value, loop_vars, computed) -> String`
- `attr_name(attr) -> &str`

#### 2c. 更新 stateful.rs 排除列表

**文件**：[stateful.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateful.rs) L37

旧：`if matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput" | "Input" | "TextInput")`
新：`if matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput" | "Input" | "TextInput" | "Slider")`

#### 2d. 更新 mod.rs 注册

**文件**：[mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/mod.rs)

1. 声明区添加：`pub mod slider;`（按字母序，在 separator 之后、stateful 之前）
2. `register_all` 中添加：`slider::register(registry);`（在 `stateful::register(registry);` 之前）

#### 2e. 更新 props_registry.rs

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)

在 COMPONENT_PROPS 中添加：
```rust
// Slider 专用（on_change 通过 state_event 订阅 SliderEvent::Change）
("Slider", &["on_change"]),
```

注：min/max/step/default_value 由 SliderTranslator 注入 state_ctor，不参与 setter 分发，不需要注册（同 Input 的 placeholder/default_value/masked 模式）。

#### 2f. 重写 slider_case.rml

**文件**：[slider_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/slider_case.rml)

```html
<Slider ref="slider_state" min="0" max="100" step="1" default-value="50" />
<Slider ref="disabled_state" min="0" max="100" default-value="30" disabled={true} />
<Slider ref="range_state" min="0" max="100" step="5" default-value={range_default} on-change={on_range_change} />
```

描述更新：移除"Stateful 模式"、"on_loaded 中初始化"等 codegen 内部术语，改为用户视角。

#### 2g. 重写 slider_case.rml.rs

**文件**：[slider_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/slider_case.rml.rs)

字段变更：
- `slider_state: ElementRef<SliderState>`（替换 `Option<Entity<SliderState>>`）
- `disabled_state: ElementRef<SliderState>`
- `range_state: ElementRef<SliderState>`
- `range_default: (f32, f32)` — 新增，范围滑块默认值

on_loaded 变更：
- 移除手动 `cx.new(|_cx| SliderState::new().min(0.0)...)` 初始化
- 仅设置 `self.range_default = (20.0, 80.0);`
- 更新 API 表格

**验证**：`cargo build -p rust-rml-engine && cargo build -p rust-rml-demo`

### 步骤 3：B2 — 事件 API 补全载荷类型

逐文件修复 5 个 demo `.rml.rs` 的事件 API 行，补充载荷类型：

| 文件 | 行 | 旧描述 | 新描述 |
|------|----|--------|--------|
| `avatar_case.rml.rs` | L48 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| `button_case.rml.rs` | L49 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| `counter_case.rml.rs` | L42 | `"按钮点击回调"` | `"按钮点击回调（参数：&ClickEvent）"` |
| `sidebar_case.rml.rs` | L70 | `"点击事件回调"` | `"点击事件回调（参数：&ClickEvent）"` |
| `tag_case.rml.rs` | L44 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |

实施时再做一次全量 grep 确认无遗漏。

### 步骤 4：B3 — 清理 demo 描述中的 codegen 内部术语

逐文件清理 20 个 `.rml` 文件的描述文本。替换规则：

| 术语 | 替换为 |
|------|--------|
| `normalize_component_tag 统一为 X` | `X 标签等价` |
| `Stateful 模式` | 删除或改为用户视角（如"通过 ref 引用状态"） |
| `Stateful 组件` | 删除或改为用户视角 |
| `StatelessWithItems` | 删除或改为"子节点通过 builder 模式注入" |
| `codegen` | `编译器` 或改为用户视角描述 |

每个文件的描述改为面向组件使用者的语言，不包含框架实现细节。

### 步骤 5：最终验证

```bash
cargo build -p rust-rml-engine
cargo test -p rust-rml-engine --lib
cargo build -p rust-rml-demo
```

---

## 假设与决策

1. **SliderTranslator + StateBridge 交互**：当 `value={field}` 存在时，委托 StateBridge（使用默认 `SliderState::new()` state_ctor）。此时 min/max/step/default_value 不支持声明式设置——这是可接受的限制，demo 不需要 value 绑定 + min/max/step 组合。未来可通过扩展 StateBridge API 支持自定义 state_ctor。

2. **Slider on_change 载荷类型**：用户方法接收 `SliderValue`（`Single(f32)` 或 `Range(f32, f32)`），与 state_bridge.rs 的 event_match 一致。用户可在方法内 match 处理。

3. **default_value 声明式语法**：
   - 单值：`default-value="50"`（Static，解析为 f32）
   - 范围：`default-value={range_default}`（Bind，字段类型 `(f32, f32)`）
   - 不支持 Static 元组字符串（如 `default-value="(20, 80)"`）——解析复杂且 Bind 更灵活

4. **B3 描述清理原则**：描述面向组件使用者，不包含 codegen 分类（Stateful/Stateless/StatefulWithDelegate）、内部函数名（normalize_component_tag）、实现术语（codegen/state_field）。保留用户可见的概念（ref、items、builder 模式、事件订阅）。

5. **无兼容性设计**：Slider 的旧用法（手动 on_loaded 初始化）在 demo 中直接替换为新声明式用法，不保留兼容路径。
