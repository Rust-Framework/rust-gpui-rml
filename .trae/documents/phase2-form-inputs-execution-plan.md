# Phase 2 表单输入组件执行计划

## Summary

本计划是 `p0-cleanup-and-phase2-form-inputs-plan.md` Part B 的执行落地文档。Part A（P0 设计对齐收尾）已完成，本计划聚焦于 9 个 Phase 2 表单输入组件的声明式支持实现。

**范围**：B1 Stepper → B9 ComboBox，按复杂度递增实施。每个组件完成 6 项交付物，紧跟编译 + 测试验证。

**关键修正**：B1 Stepper 的 `direction` 属性映射到 `.vertical()` setter（而非 `horizontal(id)`/`vertical(id)` 关联函数），已通过 gpui-component 源码验证。

---

## Current State Analysis

### 已完成（Part A）

| 项 | 状态 | 证据 |
|----|------|------|
| VisualEntityCache 文档对齐 | ✅ | `entity_cache.rs:1-12` 注释已更新为"视觉贡献 Entity 生命周期管理" |
| 架构文档补充 | ✅ | `docs/09-architecture/contribution-system.md` 已添加设计说明段落 |
| project_memory 硬约束更新 | ✅ | 区分"贡献注册缓存"（不需要）与"视觉 Entity 生命周期管理"（必要） |
| else-if 链式渲染 | ✅ | AST/Parser/Codegen/Validator/Printer/LSP/Demo/文档/测试 9 步全部落地 |

### 未开始（Part B）

- `tags.rs` `component_lookup()` 无 Stepper/Rating/NumberInput/OtpInput/ColorPicker/Calendar/DatePicker/Select/ComboBox 条目
- `props_registry.rs` `COMPONENT_PROPS` 无上述组件的专用属性
- `crates/ui/src/components/` 无上述组件的 re-export 文件
- `crates/engine/src/compiler/components/` 无上述组件的 codegen 模块
- `crates/engine/src/compiler/translator/component/` 无上述组件的 translator
- `demo/src/cases/` 无上述组件的 demo case

### 架构关键点（已验证）

1. **StatelessWithItems 组件**（如 Stepper）：`StatelessComponentTranslator::matches` 仅匹配 `Stateless`/`StatelessNoId`（`stateless.rs:36-41`），因此 `StatelessWithItems` 组件**必须**有专属 translator
2. **Stateful 组件**（NumberInput/OtpInput/ColorPicker/Calendar/DatePicker/Select/ComboBox）：`StatefulComponentTranslator` 泛化处理所有 `Stateful` 组件（`stateful.rs:32-41`），但排除 Tree/CodeEditor（有专属 translator）。若新组件无特殊构造需求，可复用泛化 translator；若有特殊需求（如 OtpInput 的 length 参数注入、NumberInput 的事件 downcast），需专属 translator。注：Rating 实际为 Stateless（[tags.rs:510-514](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L510-L514)），不走 Stateful 路径
3. **Codegen 路由**：translator 注册顺序决定优先级——专属 translator 的 `matches()` 优先于泛化 `StatefulComponentTranslator`/`StatelessComponentTranslator`

---

## Proposed Changes

### 通用交付物模板（每个组件 6 项）

| # | 交付物 | 文件 | 参考实现 |
|---|--------|------|---------|
| 1 | UI re-export | `crates/ui/src/components/<name>.rs` + `mod.rs` 注册 | `spinner.rs` |
| 2 | Compiler codegen | `crates/engine/src/compiler/components/<name>/mod.rs` + `gen.rs` + `mod.rs` 注册 | `tab_bar/gen.rs` |
| 3 | Translator | `crates/engine/src/compiler/translator/component/<name>.rs` + `mod.rs` 注册 | `accordion.rs` |
| 4 | Tags 注册 | `crates/engine/src/tags.rs` `component_lookup()` | 现有条目 |
| 5 | Props 注册 | `crates/engine/src/compiler/props_registry.rs` `COMPONENT_PROPS` | 现有条目 |
| 6 | Demo case | `demo/src/cases/<name>_case.rml` + `.rml.rs` + `mod.rs` 注册 + i18n | 现有 case |

---

### B1. Stepper（StatelessWithItems）— 验证 items builder 模式

**gpui-component API**（已验证，`stepper/stepper.rs` + `stepper/item.rs`）：
- `Stepper::new(id: impl Into<ElementId>)` — 构造器
- `.vertical()` — 设置垂直布局（默认水平，`.layout(Axis::Vertical)` 的快捷方法）
- `.selected_index(usize)` — 设置选中步骤
- `.disabled(bool)` — 禁用
- `.text_center(bool)` — 文本居中
- `.item(StepperItem)` — 添加步骤项（**非 ParentElement**，自定义方法）
- `.on_click(F)` where `F: Fn(&usize, &mut Window, &mut App)` — 点击回调
- 实现 `Sizable`（`.with_size(Size)`）

**StepperItem API**：
- `StepperItem::new()` — 无参构造
- `.icon(impl Into<Icon>)` — 设置图标
- `.disabled(bool)` — 禁用
- 实现 `ParentElement`（`.child()` 接收子元素）

**实现细节**：

| 项 | 内容 |
|----|------|
| ComponentKind | `StatelessWithItems` |
| ctor_path | `rml_ui::Stepper` |
| container | false |
| 标签别名 | `"Stepper"` / `"stepper"` |
| 子项标签 | `"StepperItem"` / `"step-item"` |
| 专属 translator | **是**（StatelessWithItems 不被 StatelessComponentTranslator 处理） |
| codegen 参考 | `tab_bar/gen.rs`（`.child(TabItem::new()...)` 模式适配为 `.item(StepperItem::new()...)`） |

**属性映射**：
- `direction="vertical"` → `.vertical()`（`direction="horizontal"` 或缺省 → 无调用）
- `selected-index="2"` → `.selected_index(2usize)`
- `disabled` → `.disabled(true)`
- `text-center` → `.text_center(true)`
- `on-click={on_step_click}` → `.on_click(cx.listener(move \|_, idx: &usize, _, cx\| this.on_step_click(idx, cx)))`

**文件清单**：
1. 创建 `crates/ui/src/components/stepper.rs`：`pub use gpui_component::stepper::{Stepper, StepperItem};`
2. 修改 `crates/ui/src/components/mod.rs`：添加 `pub mod stepper;` + `pub use stepper::{Stepper, StepperItem};`
3. 创建 `crates/engine/src/compiler/components/stepper/mod.rs`：`pub mod gen;`
4. 创建 `crates/engine/src/compiler/components/stepper/gen.rs`：`gen_stepper()` 函数 + 单元测试
5. 修改 `crates/engine/src/compiler/components/mod.rs`：添加 `pub mod stepper;`
6. 创建 `crates/engine/src/compiler/translator/component/stepper.rs`：`StepperTranslator` 薄包装
7. 修改 `crates/engine/src/compiler/translator/component/mod.rs`：添加 `pub mod stepper;` + `stepper::register(registry);`
8. 修改 `crates/engine/src/tags.rs`：
   - `component_lookup()` 添加 `"Stepper" | "stepper"` → `StatelessWithItems`
   - `is_item_builder_tag()` 添加 `"StepperItem" | "step-item"`
9. 修改 `crates/engine/src/compiler/props_registry.rs`：添加 `("Stepper", &["selected_index", "direction", "text_center", "on_click"])` + `("StepperItem", &["icon"])`
10. 创建 `demo/src/cases/stepper_case.rml` + `stepper_case.rml.rs` + 注册 + i18n

**codegen 核心逻辑**（`gen.rs`）：
```
1. 构造器：rml_ui::Stepper::new(id) 或 rml_ui::Stepper::new("rml_ref:name")
2. CSS class 样式
3. 属性 setter：
   - direction="vertical" → .vertical()
   - selected_index → .selected_index(N)
   - disabled → .disabled(true)
   - text_center → .text_center(true)
   - on_click → .on_click(cx.listener(...))
4. 子节点：.item(rml_ui::StepperItem::new().icon(...).child(...))
```

**单元测试**（参考 `accordion/gen.rs` 测试模式）：
- `gen_stepper_minimal` — 最小构造
- `gen_stepper_with_vertical` — direction 属性
- `gen_stepper_with_selected_index` — selected_index 属性
- `gen_stepper_with_step_item` — StepperItem 子节点
- `gen_stepper_with_on_click` — 事件绑定
- `gen_stepper_with_ref_uses_stable_id` — ref 指令
- `gen_stepper_rejects_non_item_child` — 拒绝非 StepperItem 子节点
- `gen_stepper_with_sizable` — size 属性

**验证**：`cargo build -p rust-rml-ui && cargo build -p rust-rml-engine && cargo test -p rust-rml-engine -- stepper`

---

### B2. Rating（Stateless）— 验证 Stateless + on_click &usize 双向绑定

**gpui-component 来源**：`gpui_component::rating::{Rating, RatingEvent}`

> **修正说明**：原计划描述 Rating 为 Stateful（RatingState），实际实现为 Stateless（[tags.rs:510-514](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L510-L514)）。Rating 无 State Entity，通过 `on_click(&usize)` 事件回写。双向绑定由 Phase C1 事件注入机制覆盖。

| 项 | 内容 |
|----|------|
| 构造 | `Rating::new(("rml_el", id))` |
| ComponentKind | `Stateless` |
| 属性 | `value`（bind，双向）、`max`（usize）、`allow_half`（bool）、`disabled`（bool） |
| 事件 | `on_click(&usize)` — 双向绑定通过 C1 EventClick { payload_type: Usize } 注入 |
| 专属 translator | **否**（标准 Stateless，复用 `StatelessComponentTranslator`） |
| container | false |
| 双向绑定 | ✅ Phase C1 已覆盖：`<Rating value={score} />` 自动双向 |

**文件清单**：
1. 创建 `crates/ui/src/components/rating.rs`：`pub use gpui_component::rating::{Rating, RatingEvent};`
2. 修改 `crates/ui/src/components/mod.rs`
3. 修改 `crates/engine/src/tags.rs`：添加 `"Rating"` → `Stateless`（已实现）
4. 修改 `crates/engine/src/compiler/props_registry.rs`：添加 `("Rating", &["value", "max", "allow_half", "disabled"])`
5. 修改 `crates/engine/src/compiler/setters.rs`（如需）：添加 Rating 专属 setter（`max`/`allow_half`）
6. 创建 `demo/src/cases/rating_case.rml` + `.rml.rs` + 注册 + i18n

---

### B3. NumberInput（Stateful, 复用 InputState）— 验证状态复用

**gpui-component 来源**：`gpui_component::input::number_input::{NumberInput, NumberInputEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `NumberInput::new(&Entity<InputState>)` — 复用 InputState |
| ComponentKind | `Stateful { state_field: "input_state", state_ctor: "\|w, c\| InputState::new(w, c)" }` |
| 属性 | 同 Input + `precision`、`step`、`min`、`max` |
| 事件 | `NumberInputEvent`（需在 subscribe 回调中区分 NumberInputEvent vs InputEvent） |
| 专属 translator | **可能**（若 NumberInputEvent 需 downcast 区分） |
| container | false |

**关键差异**：codegen 需生成 `cx.subscribe(&entity, move |_, event, cx| { if let Ok(e) = event.downcast_ref::<NumberInputEvent>() {...} })` 处理 NumberInputEvent。

**实施时需读 `number_input.rs` 源码确认**：NumberInputEvent 的具体变体、precision/step/min/max 的 setter 方法签名。

---

### B4. OtpInput（Stateful, OtpState）— 验证构造器参数注入

**gpui-component 来源**：`gpui_component::input::otp_input::{OtpInput, OtpState}`

| 项 | 内容 |
|----|------|
| 构造 | `OtpState::new(length, w, cx)` + `OtpInput::new(&Entity<OtpState>)` |
| ComponentKind | `Stateful { state_field: "otp_state", state_ctor: "\|w, c\| OtpState::new(6, w, c)" }`（默认 length=6） |
| 属性 | `length`（构造器参数）、`default_value`、`mask`（bool） |
| 事件 | `InputEvent` |
| 专属 translator | **是**（length 需注入 state_ctor 闭包） |
| container | false |

**特殊处理**：`length` 属性需在 codegen 时提取为 usize 字面量，注入 `state_ctor` 闭包：`|w, c| OtpState::new({length}, w, c)`。

**实施时需读 `otp_input.rs` 源码确认**：OtpState::new 签名、mask/default_value setter。

---

### B5. ColorPicker（Stateful, ColorPickerState）— 标准 Stateful

**gpui-component 来源**：`gpui_component::color_picker::{ColorPicker, ColorPickerState, ColorPickerEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `ColorPickerState::new()` + `ColorPicker::new(&Entity<ColorPickerState>)` |
| ComponentKind | `Stateful { state_field: "color_picker_state", state_ctor: "\|_w, _c\| ColorPickerState::new()" }` |
| 属性 | `default_value`（Hsla）、`placeholder` |
| 事件 | `ColorPickerEvent` |
| 专属 translator | **否**（标准 Stateful）——除非 ColorPickerEvent 需特殊处理 |
| container | false |

---

### B6. Calendar（Stateful, CalendarState）— 跨模块 re-export

**gpui-component 来源**：`gpui_component::time::calendar::{Calendar, CalendarState, CalendarEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `CalendarState::new()` + `Calendar::new(&Entity<CalendarState>)` |
| ComponentKind | `Stateful { state_field: "calendar_state", state_ctor: "\|_w, _c\| CalendarState::new()" }` |
| 属性 | `year_range`（暂不支持高级 matcher） |
| 事件 | `CalendarEvent` |
| UI re-export | `pub use gpui_component::time::calendar::{Calendar, CalendarState, CalendarEvent};` |
| container | false |

---

### B7. DatePicker（Stateful, DatePickerState）— 依赖 Calendar

**gpui-component 来源**：`gpui_component::time::date_picker::{DatePicker, DatePickerState}`

| 项 | 内容 |
|----|------|
| 构造 | `DatePickerState::new()` + `DatePicker::new(&Entity<DatePickerState>)` |
| ComponentKind | `Stateful { state_field: "date_picker_state", state_ctor: "\|_w, _c\| DatePickerState::new()" }` |
| 属性 | `placeholder`、`cleanable`（bool）、`default_value` |
| 事件 | DatePicker 事件（位于 time 模块） |
| UI re-export | `pub use gpui_component::time::date_picker::{DatePicker, DatePickerState};` |
| container | false |

---

### B8. Select（Stateful, SelectState）— 验证 items 绑定

**gpui-component 来源**：`gpui_component::select::{Select, SelectState, SelectEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `SelectState::new()` + `Select::new(&Entity<SelectState>)` |
| ComponentKind | `Stateful { state_field: "select_state", state_ctor: "\|_w, _c\| SelectState::new()" }` |
| 属性 | `placeholder`、`menu_width`、`menu_max_h`、`icon`、`value`（bind）、`items`（bind） |
| 事件 | `SelectEvent`、`DismissEvent` |
| 专属 translator | **可能**（items 绑定需生成 `.items(vec)` 调用） |
| container | false |

**复杂度中高**：需处理选项数据绑定。方案：通过 `items={vec}` bind 传入 `Vec<SelectItem>`，codegen 生成 `.items(vec)` 调用。

**实施时需读 `select.rs` 源码确认**：SelectState::new 签名、items/value setter、SelectItem 类型。

---

### B9. ComboBox（Stateful, ComboboxState）— 复用 Select 模式

**gpui-component 来源**：`gpui_component::combobox::{Combobox, ComboboxState, ComboboxEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `ComboboxState::new()` + `Combobox::new(&Entity<ComboboxState>)` |
| ComponentKind | `Stateful { state_field: "combobox_state", state_ctor: "\|_w, _c\| ComboboxState::new()" }` |
| 属性 | 同 Select + `search_placeholder` |
| 事件 | `ComboboxEvent`、`DismissEvent` |
| container | false |

---

## 实施顺序与验证

按复杂度递增，每个组件完成后立即验证：

| 步骤 | 组件 | 验证命令 |
|------|------|---------|
| 1 | B1 Stepper | `cargo build --workspace && cargo test -p rust-rml-engine -- stepper` |
| 2 | B2 Rating | `cargo build --workspace && cargo test -p rust-rml-engine -- rating` |
| 3 | B3 NumberInput | `cargo build --workspace && cargo test -p rust-rml-engine -- number_input` |
| 4 | B4 OtpInput | `cargo build --workspace && cargo test -p rust-rml-engine -- otp_input` |
| 5 | B5 ColorPicker | `cargo build --workspace && cargo test -p rust-rml-engine -- color_picker` |
| 6 | B6 Calendar | `cargo build --workspace && cargo test -p rust-rml-engine -- calendar` |
| 7 | B7 DatePicker | `cargo build --workspace && cargo test -p rust-rml-engine -- date_picker` |
| 8 | B8 Select | `cargo build --workspace && cargo test -p rust-rml-engine -- select` |
| 9 | B9 ComboBox | `cargo build --workspace && cargo test -p rust-rml-engine -- combobox` |

### 每个组件的验证清单

1. `cargo build -p rust-rml-ui` — UI re-export 编译通过
2. `cargo build -p rust-rml-engine` — codegen + translator 编译通过
3. `cargo test -p rust-rml-engine -- <component>` — codegen 单元测试通过
4. `cargo test -p rust-rml-engine --test props_registry_complete` — props 注册表完整性测试通过
5. `cargo build -p rust-rml-demo` — demo 编译通过
6. 运行 demo — 组件 case 正常渲染与交互

### 全量验证（全部完成后）

1. `cargo build --workspace` — 全工作区编译通过
2. `cargo test -p rust-rml-engine` — 全部引擎测试通过
3. `cargo run -p rust-rml-demo` — 所有新 case 在 demo 中可正常访问
4. `crates/engine/src/compiler/components/` 与 `crates/ui/src/components/` 仅做 re-export/codegen，无业务代码
5. 每个组件独占一个 rs 文件/目录
6. 无 `rml_` 前缀标识符（除框架内部 `__rml_*`）

---

## Assumptions & Decisions

1. **B1 Stepper direction 映射修正**：原计划假设 `direction` 映射到 `horizontal(id)`/`vertical(id)` 关联函数（参考 RadioGroup），但 gpui-component 源码验证显示 Stepper 使用 `Stepper::new(id)` + `.vertical()` setter 模式。本计划采用 `.vertical()` setter。

2. **B2-B9 API 待实施时验证**：本计划基于 gpui-component 通用模式推断各组件 API。每个组件实施时**必须先读 gpui-component 源码**确认：
   - State::new() 签名（是否需要 window/cx 参数）
   - EventEmitter 事件类型（决定是否需 cx.subscribe 订阅）
   - setter 方法名与签名
   - 是否实现 Sizable/Styled trait

3. **专属 translator 判断标准**：
   - StatelessWithItems 组件**必须**有专属 translator（StatelessComponentTranslator 不处理此 kind）
   - Stateful 组件默认复用 `StatefulComponentTranslator`，仅在以下情况需专属 translator：
     - 构造器参数需从属性注入（如 OtpInput 的 length）
     - 事件需 downcast 区分（如 NumberInput 的 NumberInputEvent vs InputEvent）
     - 需特殊子节点处理（如 Select 的 items 绑定）

4. **Select/ComboBox 选项绑定方案**：通过 `items={vec}` bind 传入选项数据，不使用 `<option>` 子标签（避免 items builder 复杂度）。若后续需声明式选项，可扩展。

5. **Tooltip 独立组件**：本迭代不纳入，延后至 Phase 3。

6. **每个组件需附带 codegen 单元测试**：覆盖构造器选择、关键 setter、事件绑定、ref/id 增量（参考 `accordion/gen.rs` / `tab_bar/gen.rs` 测试模式）。

7. **属性名规范**：所有 .rml 属性使用 kebab-case（如 `selected-index`、`on-click`），parser normalize 为 snake_case 后匹配 setter。tags.rs / props_registry.rs 中登记的属性名均为 snake_case。
