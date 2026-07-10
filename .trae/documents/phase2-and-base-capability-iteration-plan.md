# Phase 2 表单组件 + 基础能力补齐迭代计划

## Summary

本计划承接 `phase2-form-inputs-execution-plan.md`，基于对 gpui-component 源码的实际验证，完成 B2-B7 共 6 个表单输入组件的声明式支持，并同步补齐 3 项基础能力缺口（非 input 双向绑定、焦点事件、once slot bug）。

**范围**：
- **Part A 基础能力补齐**：P1-1 非 input 组件 model 指令 / P1-2 焦点事件 / P0-3 once slot bug
- **Part B Phase 2 表单组件**：B2 Rating → B7 DatePicker（按复杂度递增）

**关键修正**（基于 gpui-component 源码验证）：
- B2 Rating 是 **Stateless**（非 Stateful），无 RatingState/RatingEvent，使用 `on_click(Fn(&usize,...))` 回调
- B3 NumberInput **复用 InputState**，有独立的 `NumberInputEvent::Step(StepAction)` 事件
- B8 Select / B9 ComboBox 是**泛型 delegate 组件**（`SelectState<D: SearchableListDelegate>`），复杂度远超原计划，延后至独立迭代

---

## Current State Analysis

### 已完成（经代码验证）

| 项 | 状态 | 证据 |
|----|------|------|
| P0-4 else-if 链式渲染 | ✅ | `ast.rs:91` `ElseIf` 变体 + `conditional_case.rml` 使用 else-if 链 |
| P0-1 用户组件事件支持 | ✅ | `user_component.rs:246-248` 处理 `Attribute::Event` + `event_fields` |
| P0-2 循环变量传参 | ✅ | `event.rs:176` `EventHandler::WithArgs` 支持 `on-click={command(arg)}` |
| Builtin CSS 完备性 | ✅ | mapper.rs 已补齐 align-self/content/font-style/text-decoration/border-x/y/圆角/flex-grow/shrink/basis/display block-grid/CSS Grid/aspect-ratio |
| Builtin 元素完备性 | ✅ | `<img>`/`<svg>`/`<anchored>`/`<deferred>` 均已注册 |
| overflow-x/y bug 修复 | ✅ | mapper.rs:162-172 使用 `overflow_x_hidden()`/`overflow_y(gpui::Overflow::Scroll)` |
| B1 Stepper | ✅ | tags.rs:501-507 + props_registry.rs:191-193 + compiler/components/stepper/ + demo/stepper_case.rml |

### 未开始（本计划范围）

| 项 | 现状 | 影响 |
|----|------|------|
| P0-3 once slot bug | `state.rs:144` `once_get_or_init(&mut self,...)` 未改 `&self` | once 指令在 slot 闭包内不可用 |
| P1-1 非 input model 绑定 | `binding.rs` 仅有 `gen_model_input` | Checkbox/Switch/Slider/Radio 无法用 `model={field}` |
| P1-2 焦点事件 | `event.rs:74` `on_focus`/`on_blur`/`on_submit` 返回 None | 表单组件无法响应焦点 |
| B2-B7 表单组件 | tags.rs/props_registry 无注册 | 6 个表单组件无声明式支持 |

### 延后（不在本计划范围）

| 项 | 原因 |
|----|------|
| B8 Select / B9 ComboBox | 泛型 delegate 组件（`SelectState<D: SearchableListDelegate>`），需设计 delegate 绑定机制，独立迭代 |
| P1-4 可复用模板片段 | 语法设计复杂度高，非表单场景阻塞项 |
| P2-2 作用域插槽表达式 | 增强型能力，非阻塞 |

---

## Proposed Changes

### Part A：基础能力补齐

#### A1. P0-3 once slot bug 修复

**问题**：`once_get_or_init` 签名为 `&mut self`，slot 闭包内仅有 `&self`（通过 `__rml_self_ref: &Self`），导致 once 指令在 slot 内编译失败。

**修复方案**：将 `once_cache` 改为 `RefCell<HashMap<...>>` 实现内部可变性，`once_get_or_init` 签名改为 `&self`。

**文件清单**：

1. **`crates/ui/src/state.rs:144-157`** — 修改 `once_get_or_init` 签名：
   ```rust
   // before: pub fn once_get_or_init<T: ...>(&mut self, key, init) -> T
   // after:  pub fn once_get_or_init<T: ...>(&self, key, init) -> T
   pub fn once_get_or_init<T: 'static + Send + Sync + Clone>(
       &self,  // ← 关键改动
       key: &'static str,
       init: impl FnOnce() -> T,
   ) -> T {
       let cache = self.once_cache.borrow();
       if let Some(boxed) = cache.get(key) {
           if let Some(v) = boxed.downcast_ref::<T>() {
               return v.clone();
           }
       }
       drop(cache);
       let v = init();
       self.once_cache.borrow_mut().insert(key, Box::new(v.clone()));
       v
   }
   ```

2. **`crates/ui/src/state.rs`** — `once_cache` 字段类型改为 `RefCell<HashMap<&'static str, Box<dyn Any + Send + Sync>>>`
   ```rust
   // before: once_cache: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
   // after:  once_cache: RefCell<HashMap<&'static str, Box<dyn Any + Send + Sync>>>,
   ```

3. **`crates/engine/src/compiler/codegen/once.rs:69`** — 将 `self.__rml_state.once_get_or_init` 改为使用 `current_self_alias()`：
   ```rust
   // before: let {var} = self.__rml_state.once_get_or_init(...)
   // after:  let {var} = {alias}.__rml_state.once_get_or_init(...)
   ```
   其中 `alias = expr::current_self_alias().unwrap_or("self")`

4. **验证**：`once_case.rml` 在 slot 内使用 once 指令编译成功

---

#### A2. P1-2 焦点事件支持

**问题**：`event.rs:74` 中 `on_focus`/`on_blur`/`on_submit` 返回 `None`，表单组件无法响应焦点。

**修复方案**：GPUI 的 `FocusHandle` 提供 `on_focus`/`on_blur` 事件，通过 `Focusable` trait 的 `.on_focus(handler)`/`.on_blur(handler)` 方法绑定。RML codegen 需在 `event_binding()` 中添加这两个事件的映射。

**文件清单**：

1. **`crates/engine/src/compiler/event.rs:74`** — 修改 `event_binding()`：
   ```rust
   // before:
   "on_input" | "on_change" | "on_submit" | "on_focus" | "on_blur" | "on_load" | "on_resize" | "on_scroll" => None,
   // after:
   "on_submit" | "on_load" | "on_resize" | "on_scroll" => None,
   "on_focus" => Some(("FocusEvent", "on_focus", "")),
   "on_blur" => Some(("FocusEvent", "on_blur", "")),
   ```
   （`on_input`/`on_change` 已在 Input 专属 codegen 处理，保持 None）

2. **`crates/engine/src/compiler/event.rs`** — `apply_event()` 中 `on_focus`/`on_blur` 的代码生成：
   ```rust
   // 生成: .on_focus(cx.listener(move |this, _ev: &FocusEvent, _window, cx| { this.method(_ev, cx); }))
   ```
   注意：`on_focus`/`on_blur` 是 `Focusable` trait 方法，要求元素有 `FocusHandle`。GPUI 的 `div().id(...)` 自动实现 `Focusable`，因此对有 id 的元素生效。

3. **`crates/engine/src/compiler/props_registry.rs`** — 添加 `on_focus`/`on_blur` 到 `COMMON_EVENT_PROPS`：
   ```rust
   pub const COMMON_EVENT_PROPS: &[&str] = &["on_click", "on_focus", "on_blur"];
   ```

4. **验证**：新增 `focus_event_case.rml` demo，验证 `on-focus`/`on-blur` 事件触发

---

#### A3. P1-1 非 input 组件 model 双向绑定

**问题**：`binding.rs` 仅有 `gen_model_input`，Checkbox/Switch/Slider/Radio 无法用 `model={field}` 实现双向绑定。

**修复方案**：为每个组件类型新增专属 model codegen 函数，生成组件构造 + `on_click`/`on_change` 回调写入 ViewModel 字段。

**组件 API 验证**（已确认）：
- **Checkbox**：`.checked(bool)` setter + `on_click(Fn(&ClickEvent,...))` — checked 值在 ViewModel 中维护，on_click 翻转
- **Switch**：`.checked(bool)` setter + `on_click(Fn(&ClickEvent,...))` — 同 Checkbox
- **Slider**：Stateful，`SliderState` + `SliderEvent::Change(f32)` — 通过 `cx.subscribe` 监听
- **Radio**：`.checked(bool)` setter + `on_click(Fn(&bool,...))` — bool 参数为新的选中状态
- **RadioGroup**：`.selected_index(Option<usize>)` + `on_click(Fn(&usize,...))` — usize 为新选中索引

**文件清单**：

1. **`crates/engine/src/compiler/codegen/binding.rs`** — 新增 4 个函数：
   ```rust
   /// Checkbox/Switch: model={bool_field} → .checked(self.field).on_click(cx.listener(...翻转...))
   pub(crate) fn gen_model_checkbox(elem, field, ctx) -> Result<GenResult, CodegenError>
   /// Radio: model={bool_field} → .checked(self.field).on_click(cx.listener(...翻转...))
   pub(crate) fn gen_model_radio(elem, field, ctx) -> Result<GenResult, CodegenError>
   /// RadioGroup: model={usize_field} → .selected_index(self.field).on_click(cx.listener(...写入...))
   pub(crate) fn gen_model_radio_group(elem, field, ctx) -> Result<GenResult, CodegenError>
   /// Slider: model={f32_field} → Slider::new(&state) + cx.subscribe(SliderEvent::Change)
   pub(crate) fn gen_model_slider(elem, field, ctx) -> Result<GenResult, CodegenError>
   ```

2. **`crates/engine/src/compiler/codegen/model.rs`** — 扩展字段收集逻辑，识别 Checkbox/Switch/Radio/Slider 的 model 字段

3. **`crates/engine/src/compiler/codegen/node.rs`** — 在 model 指令分发处添加新组件分支：
   ```rust
   match tag {
       "input" | "textarea" => gen_model_input(...),
       "Checkbox" => gen_model_checkbox(...),
       "Switch" => gen_model_checkbox(...),  // Switch 与 Checkbox 同模式
       "Radio" => gen_model_radio(...),
       "RadioGroup" => gen_model_radio_group(...),
       "Slider" => gen_model_slider(...),
       _ => error("model 指令不支持此组件"),
   }
   ```

4. **验证**：新增 `model_checkbox_case.rml` / `model_slider_case.rml` demo

---

### Part B：Phase 2 表单组件（B2-B7）

#### 通用交付物模板（每个组件 6 项）

| # | 交付物 | 文件 |
|---|--------|------|
| 1 | UI re-export | `crates/ui/src/components/<name>.rs` + `mod.rs` + `lib.rs` |
| 2 | Compiler codegen | `crates/engine/src/compiler/components/<name>/` （仅需专属 translator 时） |
| 3 | Translator | `crates/engine/src/compiler/translator/component/<name>.rs`（仅需专属 translator 时） |
| 4 | Tags 注册 | `crates/engine/src/tags.rs` `component_lookup()` |
| 5 | Props 注册 | `crates/engine/src/compiler/props_registry.rs` `COMPONENT_PROPS` |
| 6 | Demo case | `demo/src/cases/<name>_case.rml` + `.rml.rs` + `mod.rs` + i18n |

---

#### B2. Rating（Stateless）— 最简单，验证 Stateless + on_click 模式

**gpui-component API**（已验证 `rating.rs`）：
- 构造：`Rating::new(id: impl Into<ElementId>)` — Stateless
- 无 RatingState / 无 EventEmitter（使用内部 `window.use_keyed_state`）
- setter：`.value(usize)`、`.max(usize)`、`.disabled(bool)`、`.color(Hsla)`、`.on_click(Fn(&usize, &mut Window, &mut App))`
- trait：`Sizable`、`Styled`、`Disableable`

**ComponentKind**：`Stateless`（无专属 translator，复用 `StatelessComponentTranslator`）

**属性映射**：
- `value={n}` → `.value(self.n)`（bind）
- `max="5"` → `.max(5usize)`（static）
- `disabled` → `.disabled(true)`（通用 static）
- `on-click={on_rate}` → `.on_click(cx.listener(move |this, val: &usize, _, cx| { this.on_rate(*val, cx); }))`（event，Fn(&usize,...) 同 Pagination）

**文件清单**：
1. 创建 `crates/ui/src/components/rating.rs`：`pub use gpui_component::rating::Rating;`
2. 修改 `crates/ui/src/components/mod.rs`：添加 `pub mod rating;` + `pub use rating::Rating;`
3. 修改 `crates/ui/src/lib.rs`：`pub use components::{..., Rating, ...};`
4. 修改 `crates/engine/src/tags.rs`：`"Rating" | "rating"` → `Stateless`, container: false
5. 修改 `crates/engine/src/compiler/props_registry.rs`：`("Rating", &["value", "max", "on_click"])`
6. 修改 `crates/engine/src/compiler/setters.rs`：
   - static_setter: `"max"` → `.max(Nusize)`（Rating 专属）
   - event_setter: `"on_click"` 分支添加 `"Rating"` 到 `Fn(&usize,...)` 组件列表
7. 创建 `demo/src/cases/rating_case.rml` + `.rml.rs` + 注册 + i18n

**无需**：专属 translator、专属 codegen 模块（Stateless 由泛化 translator 处理）

---

#### B3. NumberInput（Stateful，复用 InputState）— 验证状态复用 + 事件 downcast

**gpui-component API**（已验证 `input/number_input.rs`）：
- 构造：`NumberInput::new(&Entity<InputState>)` — **复用 InputState**（同 Input/CodeEditor）
- 事件：`NumberInputEvent::Step(StepAction)` — 需 `cx.subscribe` + downcast
- setter：`.placeholder(SharedString)`、`.prefix(AnyElement)`、`.suffix(AnyElement)`、`.appearance(bool)`
- trait：`Styled`、`Disableable`

**ComponentKind**：`Stateful { state_field: "input_state", state_ctor: "|w, c| rml_ui::InputState::new(w, c)" }`（同 Input）

**关键差异**：codegen 需生成 `cx.subscribe(&entity, move |_, event, cx| { if let Ok(e) = event.downcast_ref::<NumberInputEvent>() {...} })` 处理 NumberInputEvent

**文件清单**：
1. 创建 `crates/ui/src/components/number_input.rs`：`pub use gpui_component::input::number_input::{NumberInput, NumberInputEvent};`
2. 修改 `crates/ui/src/components/mod.rs` + `lib.rs`
3. 修改 `crates/engine/src/tags.rs`：`"NumberInput" | "number-input"` → `Stateful`
4. 修改 `crates/engine/src/compiler/props_registry.rs`：`("NumberInput", &["placeholder", "prefix", "suffix", "appearance", "on_change"])`
5. 创建 `crates/engine/src/compiler/components/number_input/` — 专属 codegen（处理 NumberInputEvent downcast）
6. 创建 `crates/engine/src/compiler/translator/component/number_input.rs` — 专属 translator
7. 创建 `demo/src/cases/number_input_case.rml` + `.rml.rs` + 注册 + i18n

**实施时需确认**：NumberInputEvent 的完整变体列表（Step 之外是否有 Change）、StepAction 的变体

---

#### B4. OtpInput（Stateful, OtpState）— 验证构造器参数注入

**gpui-component API**（已验证 `input/otp_input.rs`）：
- 构造：`OtpState::new(length: usize, window, cx)` + `OtpInput::new(&Entity<OtpState>)`
- 事件：`InputEvent::Change`（复用 InputEvent）
- setter：`.groups(usize)`、`.mask(bool)`、`.default_value(SharedString)`
- trait：`Styled`

**ComponentKind**：`Stateful { state_field: "otp_state", state_ctor: "|w, c| rml_ui::OtpState::new(6, w, c)" }`（默认 length=6）

**关键差异**：`length` 属性需注入 `state_ctor` 闭包：`|w, c| rml_ui::OtpState::new({length}, w, c)`

**文件清单**：
1. 创建 `crates/ui/src/components/otp_input.rs`：`pub use gpui_component::input::otp_input::{OtpInput, OtpState};`
2. 修改 `crates/ui/src/components/mod.rs` + `lib.rs`
3. 修改 `crates/engine/src/tags.rs`：`"OtpInput" | "otp-input"` → `Stateful`
4. 修改 `crates/engine/src/compiler/props_registry.rs`：`("OtpInput", &["length", "groups", "mask", "default_value", "on_change"])`
5. 创建 `crates/engine/src/compiler/components/otp_input/` — 专属 codegen（length 注入 state_ctor）
6. 创建 `crates/engine/src/compiler/translator/component/otp_input.rs` — 专属 translator
7. 创建 `demo/src/cases/otp_input_case.rml` + `.rml.rs` + 注册 + i18n

---

#### B5. ColorPicker（Stateful, ColorPickerState）— 标准 Stateful

**gpui-component API**（已验证 `color_picker.rs`）：
- 构造：`ColorPickerState::new(window, cx)` + `ColorPicker::new(&Entity<ColorPickerState>)`
- 事件：`ColorPickerEvent::Change(Option<Hsla>)` — 需 `cx.subscribe`
- setter：`.placeholder(SharedString)`、`.default_value(Hsla)`、`.disabled(bool)`
- trait：`Styled`、`Disableable`

**ComponentKind**：`Stateful { state_field: "color_picker_state", state_ctor: "|w, c| rml_ui::ColorPickerState::new(w, c)" }`

**文件清单**：
1. 创建 `crates/ui/src/components/color_picker.rs`：`pub use gpui_component::color_picker::{ColorPicker, ColorPickerState, ColorPickerEvent};`
2. 修改 `crates/ui/src/components/mod.rs` + `lib.rs`
3. 修改 `crates/engine/src/tags.rs`：`"ColorPicker" | "color-picker"` → `Stateful`
4. 修改 `crates/engine/src/compiler/props_registry.rs`：`("ColorPicker", &["placeholder", "default_value", "on_change"])`
5. 创建 `demo/src/cases/color_picker_case.rml` + `.rml.rs` + 注册 + i18n

**无需**：专属 translator（标准 Stateful，复用 `StatefulComponentTranslator`）——但需在 `input/event.rs` 或专属 codegen 中处理 `ColorPickerEvent::Change` 订阅

---

#### B6. Calendar（Stateful, CalendarState）— 跨模块 re-export

**gpui-component API**（已验证 `time/calendar.rs`）：
- 构造：`CalendarState::new(window, cx)` + `Calendar::new(&Entity<CalendarState>)`
- 事件：`CalendarEvent::Selected(Date)` — 需 `cx.subscribe`
- setter：`.year_range(...)`（待确认签名）
- trait：`Styled`

**ComponentKind**：`Stateful { state_field: "calendar_state", state_ctor: "|w, c| rml_ui::CalendarState::new(w, c)" }`

**UI re-export 路径**：`pub use gpui_component::time::calendar::{Calendar, CalendarState, CalendarEvent};`

**文件清单**：
1. 创建 `crates/ui/src/components/calendar.rs`：跨模块 re-export
2. 修改 `crates/ui/src/components/mod.rs` + `lib.rs`
3. 修改 `crates/engine/src/tags.rs`：`"Calendar" | "calendar"` → `Stateful`
4. 修改 `crates/engine/src/compiler/props_registry.rs`：`("Calendar", &["on_select"])`（on_select 映射 CalendarEvent::Selected）
5. 创建 `demo/src/cases/calendar_case.rml` + `.rml.rs` + 注册 + i18n

---

#### B7. DatePicker（Stateful, DatePickerState）— 依赖 Calendar

**gpui-component API**（已验证 `time/date_picker.rs`）：
- 构造：`DatePickerState::new(window, cx)` + `DatePicker::new(&Entity<DatePickerState>)`
- 另有 `DatePickerState::range(window, cx)` 构造范围选择模式
- 事件：`DatePickerEvent::Change(Date)` — 需 `cx.subscribe`
- setter：`.placeholder(SharedString)`、`.cleanable(bool)`、`.default_value(Date)`
- trait：`Styled`

**ComponentKind**：`Stateful { state_field: "date_picker_state", state_ctor: "|w, c| rml_ui::DatePickerState::new(w, c)" }`

**UI re-export 路径**：`pub use gpui_component::time::date_picker::{DatePicker, DatePickerState, DatePickerEvent};`

**文件清单**：
1. 创建 `crates/ui/src/components/date_picker.rs`：跨模块 re-export
2. 修改 `crates/ui/src/components/mod.rs` + `lib.rs`
3. 修改 `crates/engine/src/tags.rs`：`"DatePicker" | "date-picker"` → `Stateful`
4. 修改 `crates/engine/src/compiler/props_registry.rs`：`("DatePicker", &["placeholder", "cleanable", "default_value", "on_change"])`
5. 创建 `demo/src/cases/date_picker_case.rml` + `.rml.rs` + 注册 + i18n

---

## 实施顺序与验证

### 阶段 1：基础能力补齐（A1 → A2 → A3）

| 步骤 | 任务 | 验证命令 |
|------|------|---------|
| A1 | once slot bug 修复 | `cargo test -p rust-rml-engine -- once` + once_case slot 内验证 |
| A2 | 焦点事件支持 | `cargo test -p rust-rml-engine -- event` + focus_event_case demo |
| A3 | 非 input model 绑定 | `cargo test -p rust-rml-engine -- model` + model_checkbox_case demo |

### 阶段 2：Phase 2 表单组件（B2 → B7）

| 步骤 | 组件 | ComponentKind | 验证命令 |
|------|------|---------------|---------|
| B2 | Rating | Stateless | `cargo build --workspace && cargo test -p rust-rml-engine -- rating` |
| B3 | NumberInput | Stateful（复用 InputState） | `cargo build --workspace && cargo test -p rust-rml-engine -- number_input` |
| B4 | OtpInput | Stateful（length 注入） | `cargo build --workspace && cargo test -p rust-rml-engine -- otp_input` |
| B5 | ColorPicker | Stateful（标准） | `cargo build --workspace && cargo test -p rust-rml-engine -- color_picker` |
| B6 | Calendar | Stateful（标准） | `cargo build --workspace && cargo test -p rust-rml-engine -- calendar` |
| B7 | DatePicker | Stateful（标准） | `cargo build --workspace && cargo test -p rust-rml-engine -- date_picker` |

### 每个组件的验证清单

1. `cargo build -p rust-rml-ui` — UI re-export 编译通过
2. `cargo build -p rust-rml-engine` — codegen + translator 编译通过
3. `cargo test -p rust-rml-engine -- <component>` — 单元测试通过
4. `cargo test -p rust-rml-engine --test props_registry_complete` — props 注册表一致性
5. `cargo build -p rust-rml-demo` — demo 编译通过
6. 运行 demo — 组件 case 正常渲染与交互

### 全量验证（全部完成后）

1. `cargo build --workspace` — 全工作区编译通过
2. `cargo test -p rust-rml-engine` — 全部引擎测试通过
3. `cargo run -p rust-rml-demo` — 所有新 case 可正常访问
4. 无 `rml_` 前缀标识符（除框架内部 `__rml_*`）

---

## Assumptions & Decisions

1. **B2 Rating 是 Stateless**：原 `phase2-form-inputs-execution-plan.md` 假设 Rating 是 Stateful（RatingState/RatingEvent），但 gpui-component 源码验证显示 Rating 是 Stateless，使用内部 `window.use_keyed_state`，无 RatingState/RatingEvent。本计划采用 Stateless 模式。

2. **B3 NumberInput 复用 InputState**：NumberInput 构造器为 `NumberInput::new(&Entity<InputState>)`，与 Input/CodeEditor 共享 InputState。ComponentKind 配置与 Input 一致，但需专属 codegen 处理 `NumberInputEvent::Step` 事件 downcast。

3. **B4 OtpInput 需 length 参数注入**：`OtpState::new(length, window, cx)` 的 length 参数需从 `.rml` 属性提取为 usize 字面量，注入 `state_ctor` 闭包。默认 length=6。

4. **B5/B6/B7 是标准 Stateful**：ColorPicker/Calendar/DatePicker 均为标准 Stateful 组件（`State::new(window, cx)` + `Component::new(&Entity<State>)`），可复用 `StatefulComponentTranslator`。但各自有独立的事件类型（ColorPickerEvent/CalendarEvent/DatePickerEvent），需在事件订阅 codegen 中处理。

5. **B8/B9 延后**：Select 和 ComboBox 是泛型 delegate 组件（`SelectState<D: SearchableListDelegate>`），需用户在 .rml.rs 中定义 delegate struct。这要求设计 delegate 绑定机制（类似 Table 的 delegate 模式），复杂度高，延后至独立迭代。

6. **A3 非 input model 绑定范围**：本计划覆盖 Checkbox/Switch/Radio/RadioGroup/Slider 共 5 个组件的 model 指令支持。Checkbox/Switch/Radio 使用 `on_click` 回调翻转 bool 字段；Slider 使用 `cx.subscribe` 监听 `SliderEvent::Change`。

7. **A2 焦点事件实现方式**：GPUI 的 `Focusable` trait 提供 `.on_focus(handler)`/`.on_blur(handler)` 方法，要求元素有 `FocusHandle`（即有 `.id(...)`）。RML codegen 生成 `.on_focus(cx.listener(move |this, _ev: &FocusEvent, _window, cx| { this.method(_ev, cx); }))`。`on_focus`/`on_blur` 添加到 `COMMON_EVENT_PROPS`，对所有有 id 的元素生效。

8. **A1 once 修复采用 RefCell 方案**：将 `RmlState.once_cache` 从 `HashMap` 改为 `RefCell<HashMap>`，使 `once_get_or_init` 只需 `&self`。同时 codegen 改用 `current_self_alias()` 替代硬编码 `self`，确保 slot 闭包内使用 `__rml_self_ref` 别名。

9. **属性名规范**：所有 .rml 属性使用 kebab-case（如 `default-value`、`on-change`），parser normalize 为 snake_case 后匹配 setter。tags.rs / props_registry.rs 中登记的属性名均为 snake_case。

10. **事件 downcast 模式**：Stateful 组件的 EventEmitter 事件统一通过 `cx.subscribe(&entity, move |_, event, cx| { if let Ok(e) = event.downcast_ref::<XxxEvent>() {...} })` 处理。参考现有 Input 的 `on_change` codegen 模式。
