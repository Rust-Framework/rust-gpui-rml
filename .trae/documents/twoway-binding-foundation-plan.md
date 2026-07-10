# PascalCase 组件自动双向绑定基础夯实计划

## Summary

本计划解决 RML 框架自动推断双向绑定原则的核心不一致问题：`<input value={field}>` 自动双向，但 `<Input value={field}>` / `<Checkbox checked={field}>` / `<Rating value={field}>` 等 PascalCase 表单组件仅单向。

**设计原则**：双向能力是属性的固有语义，框架自动识别，开发者无需声明 `mode=twoway`。

**目标**：建立框架级双向绑定注册表，使所有表单组件（现有 + 未来新增）的 `value={field}` / `checked={field}` / `selected_index={field}` 绑定自动双向。

**与 Phase 2 组件扩展的关系**：Phase 2 计划（`phase2-form-inputs-execution-plan.md` B1-B9）正在新增 9 个表单组件，其中 B3 NumberInput / B5 ColorPicker / B6 Calendar / B7 DatePicker / B8 Select / B9 ComboBox 尚未实现。本计划的双向绑定框架必须先于这些组件落地，确保新组件从第一天起就支持自动双向绑定。

---

## 1. Current State Analysis（现状分析）

### 1.1 双向绑定覆盖矩阵

| 组件 | 标签类型 | ComponentKind | 绑定属性 | 当前状态 | 机制 |
|------|---------|--------------|---------|---------|------|
| `<input>` | 小写 builtin | — | `value` | ✅ 双向 | `InputTranslator` → `gen_model_input()` → InputState 双向同步 |
| `<textarea>` | 小写 builtin | — | `value` | ✅ 双向 | `TextAreaTranslator` → `gen_model_input()` → InputState 双向同步 |
| `<Input>` | PascalCase | Stateful | `value` | ❌ 单向 | `StatefulComponentTranslator` → `.value(field.clone())` |
| `<TextInput>` | PascalCase | Stateful | `value` | ❌ 单向 | 同上 |
| `<Slider>` | PascalCase | Stateful | `value` | ❌ 单向 | 同上，无反向同步 |
| `<Checkbox>` | PascalCase | Stateless | `checked` | ❌ 单向 | `.selected(field)` 无回写 |
| `<Switch>` | PascalCase | Stateless | `checked` | ❌ 单向 | `.checked(field)` 无回写 |
| `<RadioGroup>` | PascalCase | Stateless | `selected_index` | ❌ 单向 | `.selected_index(field)` 无回写 |
| `<Rating>` | PascalCase | Stateless | `value` | ❌ 单向 | `.value(field)` 无回写 |
| `<Stepper>` | PascalCase | StatelessWithItems | `selected_index` | ❌ 单向 | `.selected_index(field)` 无回写 |

### 1.2 瓶颈定位

**[setters.rs:278-281](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs#L278-L281)** — `component_bind_setter` 的 `value` 分支：

```rust
"value" => {
    let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
    Some(format!(".value({}.clone())", rust_expr))  // 单向，无反向同步
}
```

**根因**：`component_bind_setter` 是纯属性→setter 映射函数，无法感知组件上下文，也无法注入事件回调。双向绑定需要「属性绑定 + 事件回调」协同，超出了单函数的职责范围。

### 1.3 既有可复用机制

| 机制 | 位置 | 适用范围 |
|------|------|---------|
| InputState 双向同步 | [codegen/binding.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs) `gen_model_input()` + [observable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs) `gen_input_state_impl()` | Input/TextInput（复用 InputState） |
| 字段收集器 | [codegen/model.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/model.rs) `collect_model_fields/converters/handlers` | 扫描 `value={field}` on input/textarea |
| 事件特殊签名 | [setters.rs:424-566](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs#L424-L566) `component_event_setter` | Checkbox/Switch `&bool`、Rating/RadioGroup/Stepper `&usize` |
| Stateless translator | [stateless.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateless.rs) | 属性循环 + setter 调用 |
| Stateful translator | [stateful.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateful.rs) `gen_stateful_body()` | Entity 初始化 + 事件订阅 |

### 1.4 Phase 2 组件扩展对齐分析

[phase2-form-inputs-execution-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/phase2-form-inputs-execution-plan.md) 的 9 个组件：

| 组件 | Phase 2 状态 | ComponentKind | 需要双向绑定的属性 | 本计划覆盖 |
|------|-------------|--------------|------------------|-----------|
| B1 Stepper | ✅ 已实现 | StatelessWithItems | `selected_index` | Phase C1 |
| B2 Rating | ✅ 已实现 | Stateless | `value` | Phase C1 |
| B3 NumberInput | ❌ 未实现 | Stateful (InputState) | `value` | Phase C2 |
| B4 OtpInput | ✅ 已实现 | Stateful (OtpState) | — (OTP 无字段绑定) | 不涉及 |
| B5 ColorPicker | ❌ 未实现 | Stateful (ColorPickerState) | `value` | Phase C4 |
| B6 Calendar | ❌ 未实现 | Stateful (CalendarState) | `value` | Phase C4 |
| B7 DatePicker | ❌ 未实现 | Stateful (DatePickerState) | `value` | Phase C4 |
| B8 Select | ❌ 未实现 | Stateful (SelectState) | `value` | Phase C4 |
| B9 ComboBox | ❌ 未实现 | Stateful (ComboboxState) | `value` | Phase C4 |

**关键发现**：
1. B2 Rating 在 Phase 2 计划中被描述为 Stateful（RatingState），但实际实现为 Stateless（[tags.rs:510-514](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L510-L514)）。Phase 2 计划需修正此描述。
2. B3 NumberInput 复用 InputState，可直接复用本计划 Phase C3 的 Input/TextInput 双向绑定机制。
3. B5-B9 的 Stateful 组件各自有独立的 State Entity 和事件类型，需要框架提供通用的 Stateful 双向绑定机制。

---

## 2. Architecture Design（架构设计）

### 2.1 双向绑定分类

PascalCase 表单组件按反向同步机制分为两类：

**Category 1: Stateless 事件注入型**（Checkbox/Switch/Radio/Rating/RadioGroup/Stepper）

- 无 State Entity，值通过 `.checked()` / `.value()` / `.selected_index()` 传入
- 反向同步：在 `on_click` 事件回调中直接回写 ViewModel 字段
- 事件载荷类型已知：`&bool`（Checkbox/Switch/Radio）、`&usize`（Rating/RadioGroup/Stepper）
- 不需要字段收集器（无版本追踪、无 InputState）

**Category 2: Stateful State 桥接型**（Input/TextInput/Slider/NumberInput/ColorPicker/DatePicker/Select/ComboBox）

- 有 State Entity（InputState/SliderState/...），通过 `Type::new(&entity)` 构造
- 反向同步：订阅 State Entity 的事件 → 回写 ViewModel 字段
- 正向同步：检查字段版本 → 更新 State Entity 值
- 需要字段收集器（追踪版本、类型、converter）

### 2.2 TwoWayBindingRegistry 设计

```rust
/// 双向绑定规格 —— 描述 (组件, 属性) 的双向绑定方式
pub struct TwoWayBindingSpec {
    /// 绑定属性名（"value" / "checked" / "selected_index"）
    pub bind_property: &'static str,
    /// 反向同步方式
    pub kind: TwoWayBindingKind,
}

pub enum TwoWayBindingKind {
    /// Stateless 事件注入：on_click 回调直接回写字段
    /// payload_extractor 将事件载荷转换为字段赋值代码
    EventClick {
        payload_type: PayloadType,
    },
    /// Stateful State 桥接：复用 InputState 双向同步机制
    /// 用于 Input/TextInput/NumberInput（均基于 InputState）
    InputStateBridge,
    /// Stateful State 桥接：通用 State Entity 订阅
    /// 用于 Slider/ColorPicker/DatePicker/Select/ComboBox（各有独立 State）
    StateBridge {
        state_field: &'static str,
        event_variant: &'static str,
        value_extractor: &'static str,
    },
}

pub enum PayloadType {
    Bool,   // Checkbox/Switch/Radio: &bool → this.field = *checked
    Usize,  // Rating/RadioGroup/Stepper: &usize → this.field = *value
}

/// 双向绑定注册表
pub static TWOWAY_BINDING_REGISTRY: &[(&str, TwoWayBindingSpec)] = &[
    // Category 1: Stateless 事件注入型
    ("Checkbox",      TwoWayBindingSpec { bind_property: "checked",        kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Bool } }),
    ("Switch",        TwoWayBindingSpec { bind_property: "checked",        kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Bool } }),
    ("Radio",         TwoWayBindingSpec { bind_property: "checked",        kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Bool } }),
    ("Rating",        TwoWayBindingSpec { bind_property: "value",          kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Usize } }),
    ("RadioGroup",    TwoWayBindingSpec { bind_property: "selected_index", kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Usize } }),
    ("Stepper",       TwoWayBindingSpec { bind_property: "selected_index", kind: TwoWayBindingKind::EventClick { payload_type: PayloadType::Usize } }),
    // Category 2: Stateful State 桥接型
    ("Input",         TwoWayBindingSpec { bind_property: "value",          kind: TwoWayBindingKind::InputStateBridge }),
    ("TextInput",     TwoWayBindingSpec { bind_property: "value",          kind: TwoWayBindingKind::InputStateBridge }),
    ("NumberInput",   TwoWayBindingSpec { bind_property: "value",          kind: TwoWayBindingKind::InputStateBridge }),
    ("Slider",        TwoWayBindingSpec { bind_property: "value",          kind: TwoWayBindingKind::StateBridge { state_field: "slider_state", event_variant: "SliderEvent", value_extractor: "state.value()" } }),
    // Phase 2 后续组件实施时注册：
    // ("ColorPicker",  TwoWayBindingSpec { bind_property: "value", kind: TwoWayBindingKind::StateBridge { ... } }),
    // ("Select",       TwoWayBindingSpec { bind_property: "value", kind: TwoWayBindingKind::StateBridge { ... } }),
    // ...
];
```

### 2.3 代码生成流程

#### Category 1: Stateless 事件注入型

**修改 `stateless.rs` 的 `gen_stateless_body()`**：

```
1. 预扫描元素属性，检测双向绑定对：
   - 查询 TWOWAY_BINDING_REGISTRY[(canonical_tag, bind_property)]
   - 提取 field 名（从 bind 表达式）
   - 记录用户是否同时声明了 on_click 事件

2. 正向 setter（已有逻辑不变）：
   - checked={field} → .selected(self.field) / .checked(self.field)
   - value={field} → .value(self.field)
   - selected_index={field} → .selected_index(self.field)

3. 反向 on_click 注入（新增）：
   - 如果检测到双向绑定，生成合并的 on_click 回调：
     a. 自动回写：this.field = *payload; this.__rml_bump_version("field");
     b. 用户回调（如有）：this.user_handler(payload, cx);
     c. cx.notify();
   - 跳过属性循环中 on_click 的正常 setter 生成（已被合并回调替代）
```

**生成代码示例**：

```rml
<!-- 用户写法 -->
<Checkbox checked={agree} on-click={on_agree_change} />
```

```rust
// 框架生成
Checkbox::new(("rml_el", 0usize))
    .selected(self.agree.clone())  // 正向
    .on_click(cx.listener(move |this, checked: &bool, _window, cx| {
        // 反向自动回写
        this.agree = *checked;
        this.__rml_bump_version("agree");
        // 用户回调
        this.on_agree_change(checked, cx);
        cx.notify();
    }))
```

#### Category 2: Stateful State 桥接型 — InputStateBridge

**修改 `stateful.rs` 的 `gen_stateful_body()`**：

```
1. 预扫描：检测 value={field} 绑定 + 组件为 Input/TextInput/NumberInput
2. 如果检测到 InputStateBridge 双向绑定：
   - 跳过标准 Stateful 构造路径
   - 调用 gen_model_input()（复用现有函数）
   - 生成 rml_ui::Input::new(&self.__rml_get_or_init_input_state("field", placeholder, _window, cx))
3. 如果未检测到双向绑定（ref 模式或无 value 绑定）：
   - 走标准 Stateful 构造路径（不变）
```

**字段收集器扩展**：修改 `collect_model_fields/converters/handlers` 的 `is_input_or_textarea()` 判断，扩展为也包含 PascalCase 的 Input/TextInput/NumberInput：

```rust
fn supports_twoway_value(tag: &str) -> bool {
    matches!(tag, "input" | "textarea" | "Input" | "TextInput" | "NumberInput")
}
```

#### Category 2: Stateful State 桥接型 — StateBridge（通用）

**用于 Slider 及 Phase 2 后续 Stateful 组件**：

```
1. 预扫描：检测 value={field} 绑定 + 组件在 StateBridge 注册表中
2. 生成通用 State 桥接代码：
   a. 构造 State Entity（复用 get_or_init_ref 或字段引用）
   b. 订阅 State 事件 → 回写字段
   c. 正向同步（版本检查 → 更新 State 值）
3. 需要扩展 gen_input_state_impl 为泛化的 gen_state_impl，支持多种 State 类型
```

**Slider 生成代码示例**：

```rml
<!-- 用户写法 -->
<Slider value={volume} />
```

```rust
// 框架生成
rml_ui::Slider::new(&self.__rml_get_or_init_slider_state("volume", _window, cx))
```

```rust
// __rml_get_or_init_slider_state 实现（类似 __rml_get_or_init_input_state）
fn __rml_get_or_init_slider_state(&mut self, field: &'static str, ...) -> Entity<SliderState> {
    if !self.__rml_state.slider_states.contains_key(field) {
        let entity = cx.new(|cx| SliderState::new());
        // 正向：设置初始值
        let initial = match field { "volume" => self.volume as f32, _ => 0.0 };
        entity.update(cx, |state, cx| state.set_value(initial, cx));
        // 反向：订阅事件
        cx.subscribe(&entity, move |this, state_entity, event, cx| {
            match event {
                SliderEvent::Change => {
                    let value = state_entity.read(cx).value();
                    match field {
                        "volume" => { this.volume = value as i32; this.__rml_bump_version("volume"); }
                        _ => {}
                    }
                    cx.notify();
                }
                _ => {}
            }
        }).detach();
        self.__rml_state.slider_states.insert(field.to_string(), entity);
    }
    // 正向同步：版本检查
    ...
    self.__rml_state.slider_states.get(field).unwrap().clone()
}
```

---

## 3. Proposed Changes（分阶段详细规划）

### Phase C1: Stateless 事件注入型双向绑定（Checkbox/Switch/Radio/Rating/RadioGroup/Stepper）

**影响范围**：6 个已实现的 Stateless 表单组件

#### C1.1 创建双向绑定注册表 — 新建 `crates/engine/src/compiler/twoway.rs`

```rust
//! 双向绑定注册表 —— 描述 (组件, 属性) 的双向绑定方式
//!
//! 自动推断原则：属性具备双向能力则自动双向，开发者无需声明 mode=twoway。
//! - Checkbox/Switch/Radio + checked={field} → 自动双向（on_click &bool 回写）
//! - Rating + value={field} → 自动双向（on_click &usize 回写）
//! - RadioGroup/Stepper + selected_index={field} → 自动双向（on_click &usize 回写）

pub enum TwoWayBindingKind { ... }
pub enum PayloadType { Bool, Usize }
pub struct TwoWayBindingSpec { ... }
pub static TWOWAY_BINDING_REGISTRY: &[(&str, TwoWayBindingSpec)] = &[ ... ];

/// 查询组件的指定属性是否支持双向绑定
pub fn lookup_twoway_binding(tag: &str, bind_property: &str) -> Option<&'static TwoWayBindingSpec>;

/// 从 bind 表达式中提取 field 名（复用 model.rs 的 extract_field_converter）
pub fn extract_bind_field(expr: &str) -> String;
```

#### C1.2 修改 `stateless.rs` — 注入 on_click 反向回写

**修改 `gen_stateless_body()`**：

1. 在属性循环前，预扫描双向绑定：
   ```rust
   let twoway = lookup_twoway_binding(&canonical, bind_property);
   let twoway_field = twoway.map(|spec| extract_bind_field(expr));
   let user_on_click = elem.attributes.iter().find_map(|attr| {
       if let Attribute::Event { name, handler, .. } = attr {
           (name == "on_click").then(|| handler)
       } else { None }
   });
   ```

2. 在属性循环中，如果 `on_click` 已被双向绑定接管，跳过正常 event setter

3. 属性循环后，如果有双向绑定，生成合并的 on_click 回调：
   ```rust
   if let (Some(spec), Some(field)) = (twoway, twoway_field) {
       let on_click_code = gen_twoway_on_click(&canonical, &field, spec, user_on_click);
       code.push_str(&on_click_code);
   }
   ```

**新增函数 `gen_twoway_on_click()`**（在 `twoway.rs` 中）：

```rust
/// 生成双向绑定的 on_click 回调代码
///
/// 合并自动回写 + 用户回调（如有）
fn gen_twoway_on_click(
    tag: &str,
    field: &str,
    spec: &TwoWayBindingSpec,
    user_handler: Option<&EventHandler>,
) -> String {
    let (payload_type, payload_var) = match &spec.kind {
        TwoWayBindingKind::EventClick { payload_type } => (
            payload_type,
            match payload_type {
                PayloadType::Bool => "checked",
                PayloadType::Usize => "value",
            }
        ),
        _ => unreachable!(),
    };

    let payload_ty = match payload_type {
        PayloadType::Bool => "bool",
        PayloadType::Usize => "usize",
    };

    // 自动回写代码
    let auto_sync = match payload_type {
        PayloadType::Bool => format!(
            "this.{field} = *{payload_var};\n    this.__rml_bump_version({field:?});",
            field = field, payload_var = payload_var
        ),
        PayloadType::Usize => format!(
            "this.{field} = *{payload_var};\n    this.__rml_bump_version({field:?});",
            field = field, payload_var = payload_var
        ),
    };

    // 用户回调（如有）
    let user_call = if let Some(handler) = user_handler {
        let method = match handler {
            EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
            EventHandler::WithArgs(m, _) => m,
            EventHandler::ClosureField(_) => return String::new(), // 闭包字段不支持合并
        };
        format!("\n    this.{}({}, cx);", method, payload_var)
    } else {
        String::new()
    };

    format!(
        ".on_click(cx.listener(move |this, {payload_var}: &{payload_ty}, _window, cx| {{\n    \
         {auto_sync}{user_call}\n    \
         cx.notify();\n}}))",
        payload_var = payload_var,
        payload_ty = payload_ty,
        auto_sync = auto_sync,
        user_call = user_call,
    )
}
```

#### C1.3 Stepper 专属 translator 对齐

Stepper 使用专属 translator（`stepper/gen.rs`），不走 `stateless.rs`。需要在 `gen_stepper()` 中添加同样的双向绑定检测和 on_click 注入逻辑。

**修改 `crates/engine/src/compiler/components/stepper/gen.rs`**：

在属性处理循环前添加双向绑定预扫描，在循环后注入 on_click 回调。复用 `twoway.rs` 中的 `gen_twoway_on_click()` 函数。

#### C1.4 RadioGroup 专属 translator 对齐

RadioGroup 也使用专属 translator（`radio_group.rs`），同样需要添加双向绑定支持。复用 `twoway.rs` 函数。

#### C1.5 单元测试

新建 `crates/engine/tests/codegen_twoway_pascal_test.rs`：

| 测试 | 验证点 |
|------|--------|
| `checkbox_checked_twoway_generates_on_click` | `<Checkbox checked={agree} />` 生成 `.selected(self.agree)` + `.on_click(...)` 回写 |
| `switch_checked_twoway_generates_on_click` | `<Switch checked={enabled} />` 生成 `.checked(self.enabled)` + `.on_click(...)` 回写 |
| `rating_value_twoway_generates_on_click` | `<Rating value={score} />` 生成 `.value(self.score)` + `.on_click(...)` 回写 |
| `radiogroup_selected_index_twoway` | `<RadioGroup selected_index={idx} />` 生成双向绑定 |
| `stepper_selected_index_twoway` | `<Stepper selected_index={step} />` 生成双向绑定 |
| `checkbox_twoway_with_user_handler` | `<Checkbox checked={agree} on-click={on_change} />` 生成合并回调（自动回写 + 用户回调） |
| `checkbox_without_twoway_no_on_click` | `<Checkbox label="同意" />` 不生成 on_click（无双向绑定） |
| `checkbox_twoway_closure_field_skips_merge` | 闭包字段 handler 时不合并（降级为仅自动回写） |

**验证**：`cargo test -p rust-rml-engine -- twoway_pascal`

---

### Phase C2: Stateful InputStateBridge 双向绑定（Input/TextInput）

**影响范围**：2 个已实现的 Stateful 表单组件 + Phase 2 B3 NumberInput

#### C2.1 修改 `stateful.rs` — 检测 value={field} 走 InputState 路径

**修改 `gen_stateful_body()`**：

在函数开头添加双向绑定检测：

```rust
// 检测 InputStateBridge 双向绑定：Input/TextInput/NumberInput + value={field}
let twoway_value = if matches!(canonical.as_str(), "Input" | "TextInput" | "NumberInput") {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name, expr, .. } = attr {
            (name == "value").then(|| expr.clone())
        } else { None }
    })
} else {
    None
};

if let Some(expr) = twoway_value {
    // 走 gen_model_input 路径（复用 lowercase <input> 的双向同步机制）
    let (field, _) = extract_field_converter(&expr);
    let code = gen_model_input(elem, ctx, id_counter, field, parents)?;
    // ... 应用 CSS 样式和其他非 value 属性的 setter
    return Ok(code);
}
// 否则走标准 Stateful 构造路径（不变）
```

**注意**：`gen_model_input` 生成的是 `rml_ui::Input::new(...)`，对于 TextInput/NumberInput 需要调整 `ctor_path`。方案：给 `gen_model_input` 添加 `ctor_path` 参数，或新增 `gen_model_stateful_input()` 变体。

#### C2.2 扩展字段收集器 — 修改 `codegen/model.rs`

```rust
// before:
fn is_input_or_textarea(tag: &str) -> bool {
    tag == "input" || tag == "textarea"
}

// after:
fn supports_twoway_value(tag: &str) -> bool {
    matches!(tag, "input" | "textarea" | "Input" | "TextInput" | "NumberInput")
}
```

同步修改 `collect_model_fields`、`collect_model_converters`、`collect_model_input_handlers` 三个函数中的 `is_input_or_textarea` 调用。

#### C2.3 单元测试

| 测试 | 验证点 |
|------|--------|
| `pascal_input_value_twoway` | `<Input value={name} />` 生成 `__rml_get_or_init_input_state` 调用 |
| `pascal_textinput_value_twoway` | `<TextInput value={email} />` 同上 |
| `pascal_input_ref_no_twoway` | `<Input ref="my_input" />` 走标准 Stateful 路径（不触发双向） |
| `pascal_input_value_with_converter` | `<Input value={price \| Currency} />` 生成 converter 双向绑定 |
| `pascal_input_value_with_on_change` | `<Input value={name} on-change={on_name_change} />` 合并 handler |

**验证**：`cargo test -p rust-rml-engine -- pascal_input`

---

### Phase C3: Stateful StateBridge 双向绑定（Slider）

**影响范围**：1 个已实现的 Stateful 表单组件

#### C3.1 调研 SliderState 事件系统

实施前需确认：
- `SliderState` 是否实现 `EventEmitter`？事件类型是什么？
- `SliderState::value()` / `set_value()` 方法签名
- Slider 的 `on_change` 事件签名（如有）

#### C3.2 新增 `gen_slider_state_impl()` — 修改 `codegen/observable.rs`

参考 `gen_input_state_impl()` 的模式，为 Slider 生成 `__rml_get_or_init_slider_state` 方法：

```rust
pub(super) fn gen_slider_state_impl(ctx: &CodegenCtx) -> String {
    // 收集 Slider 双向绑定字段
    let slider_fields: Vec<String> = ctx.slider_fields.clone();
    // ... 生成 __rml_get_or_init_slider_state 方法
    // 正向：field → SliderState 初始值
    // 反向：订阅 SliderEvent → 回写字段
}
```

#### C3.3 扩展 ViewModel state — 修改 `codegen/mod.rs`

在 `__rml_state` 中添加 `slider_states: HashMap<String, Entity<SliderState>>` 和 `slider_state_versions: HashMap<String, u64>`。

#### C3.4 修改 `stateful.rs` — Slider 检测 StateBridge

在 `gen_stateful_body()` 中添加 Slider 双向绑定检测，走 `gen_model_slider()` 路径。

#### C3.5 扩展字段收集器

新增 `collect_slider_fields()` 函数，扫描 `<Slider value={field}>` 绑定。

#### C3.6 单元测试

| 测试 | 验证点 |
|------|--------|
| `slider_value_twoway` | `<Slider value={volume} />` 生成 `__rml_get_or_init_slider_state` 调用 |
| `slider_value_with_ref` | `<Slider ref="my_slider" />` 走标准 Stateful 路径 |

**验证**：`cargo test -p rust-rml-engine -- slider_twoway`

---

### Phase C4: 框架通用化 + Phase 2 对齐

**影响范围**：Phase 2 后续组件（B5 ColorPicker / B6 Calendar / B7 DatePicker / B8 Select / B9 ComboBox）

#### C4.1 通用 StateBridge 机制

将 C3 的 Slider StateBridge 机制通用化，支持任意 Stateful 组件：

```rust
pub(super) fn gen_state_bridge_impl(
    ctx: &CodegenCtx,
    state_type: &str,         // "SliderState" / "SelectState" / ...
    event_enum: &str,         // "SliderEvent" / "SelectEvent" / ...
    event_variant: &str,      // "Change" / "Select" / ...
    value_method: &str,       // "value()" / "selected_index()" / ...
    fields: &[String],        // 双向绑定字段列表
    field_types: &HashMap<String, String>,
) -> String
```

每个 Stateful 组件注册时声明其 StateBridge 规格，codegen 统一生成 `__rml_get_or_init_<state>_state` 方法。

#### C4.2 Phase 2 组件实施时的双向绑定清单

当 Phase 2 B5-B9 组件实施时，需在 `TWOWAY_BINDING_REGISTRY` 中注册：

| 组件 | bind_property | kind | 实施前提 |
|------|--------------|------|---------|
| ColorPicker | `value` | StateBridge { state_field: "color_picker_state", event_variant: "ColorPickerEvent::Change", value_extractor: "state.color()" } | 确认 ColorPickerState 事件系统 |
| Calendar | `value` | StateBridge { state_field: "calendar_state", event_variant: "CalendarEvent::Change", value_extractor: "state.value()" } | 确认 CalendarState 事件系统 |
| DatePicker | `value` | StateBridge { state_field: "date_picker_state", event_variant: "DatePickerEvent::Change", value_extractor: "state.value()" } | 确认 DatePickerState 事件系统 |
| Select | `value` | StateBridge { state_field: "select_state", event_variant: "SelectEvent::Change", value_extractor: "state.value()" } | 确认 SelectState 事件系统 |
| ComboBox | `value` | StateBridge { state_field: "combobox_state", event_variant: "ComboboxEvent::Change", value_extractor: "state.value()" } | 确认 ComboboxState 事件系统 |
| NumberInput | `value` | InputStateBridge | 复用 Input/TextInput 机制（B3 使用 InputState） |

#### C4.3 修正 Phase 2 计划文档

修正 `phase2-form-inputs-execution-plan.md` 中 B2 Rating 的描述：
- **错误描述**：B2 Rating 被描述为 Stateful（RatingState）
- **实际状态**：Rating 实现为 Stateless（[tags.rs:510-514](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L510-L514)）
- **修正**：更新 Phase 2 计划中 B2 的 ComponentKind 为 Stateless，并注明双向绑定走 C1 事件注入路径

修正 `gpui-component-advanced-full-coverage-plan.md` 中 A.2 Rating 的描述：
- 同样修正为 Stateless
- 注明双向绑定由 C1 机制覆盖

---

### Phase C5: Demo 验证 + 文档更新

#### C5.1 扩展 `two_way_case.rml`

在现有 `two_way_case` demo 中添加 PascalCase 组件的双向绑定演示：

```rml
<!-- Stateless 事件注入型 -->
<Checkbox checked={agree} on-change={on_agree_change} />
<Switch checked={notifications} />
<Rating value={score} max="10" />
<RadioGroup selected_index={radio_idx} layout="vertical">
  <Radio>选项A</Radio>
  <Radio>选项B</Radio>
</RadioGroup>
<Stepper selected_index={step_idx}>
  <step-item>步骤一</step-item>
  <step-item>步骤二</step-item>
</Stepper>

<!-- Stateful InputStateBridge -->
<Input value={username} placeholder="用户名" />
<TextInput value={email} placeholder="邮箱" />

<!-- Stateful StateBridge -->
<Slider value={volume} />
```

#### C5.2 文档更新

更新以下文档，补充 PascalCase 双向绑定说明：
- `docs/04-data-binding/two-way-binding.md`（如存在）
- `docs/06-components/reference/input.md`
- `docs/06-components/reference/checkbox.md`
- `docs/06-components/reference/rating.md`

#### C5.3 project_memory 更新

在 project_memory.md 中添加硬约束：
- "PascalCase 表单组件的 `value={field}` / `checked={field}` / `selected_index={field}` 绑定自动双向，与小写 `<input>` 行为一致"
- "新增 Stateful 表单组件时，必须在 TWOWAY_BINDING_REGISTRY 中注册双向绑定规格"

---

## 4. 实施顺序与验证

### 4.1 实施顺序

| 步骤 | Phase | 内容 | 依赖 | 验证 |
|------|-------|------|------|------|
| 1 | C1.1 | 创建 `twoway.rs` 注册表 | 无 | `cargo build -p rust-rml-engine` |
| 2 | C1.2 | 修改 `stateless.rs` 注入 on_click | C1.1 | `cargo test -- twoway_pascal` |
| 3 | C1.3 | Stepper translator 对齐 | C1.1 | `cargo test -- stepper` |
| 4 | C1.4 | RadioGroup translator 对齐 | C1.1 | `cargo test -- radio_group` |
| 5 | C1.5 | C1 单元测试 | C1.1-C1.4 | 全量通过 |
| 6 | C2.1 | `stateful.rs` InputStateBridge | C1.1 | `cargo test -- pascal_input` |
| 7 | C2.2 | 扩展字段收集器 | C2.1 | 现有 input 测试不破坏 |
| 8 | C2.3 | C2 单元测试 | C2.1-C2.2 | 全量通过 |
| 9 | C3 | Slider StateBridge | C2 | `cargo test -- slider_twoway` |
| 10 | C4.1 | 通用 StateBridge 机制 | C3 | 编译通过 |
| 11 | C5 | Demo + 文档 | C1-C4 | demo 运行正常 |

### 4.2 每阶段验证清单

- [ ] `cargo build --workspace` 编译通过
- [ ] `cargo test -p rust-rml-engine` 全量测试通过（无回归）
- [ ] 新增的双向绑定测试全部通过
- [ ] 现有 demo 不受影响（`cargo run -p rust-rml-demo` 正常启动）
- [ ] `props_registry_complete` 测试通过（属性注册一致性）
- [ ] 无 `unnecessary_parens` 等 clippy 警告

### 4.3 回归风险点

| 风险 | 缓解措施 |
|------|---------|
| Stateless 组件无双向绑定时误注入 on_click | 预扫描仅在 `TWOWAY_BINDING_REGISTRY` 命中时注入 |
| Input/TextInput ref 模式被误判为双向 | 仅 `value={field}` bind 属性触发，`ref` 指令不触发 |
| 用户 on_click 与自动回写冲突 | 合并回调：自动回写 → 用户回调 → cx.notify() |
| 字段收集器扩展影响现有 input/textarea | `supports_twoway_value` 仅新增标签，不改原有逻辑 |
| SliderState 事件系统与预期不符 | C3 实施前先调研 gpui-component 源码确认 |

---

## 5. Assumptions & Decisions

### 5.1 设计决策

| 决策项 | 选择 | 依据 |
|-------|------|------|
| 双向绑定触发条件 | `value={field}` / `checked={field}` / `selected_index={field}` bind 属性 | 自动推断原则，无需 mode=twoway |
| Stateless 反向同步方式 | on_click 事件注入 | 组件已有 on_click 特殊签名（&bool/&usize） |
| Stateful Input 反向同步方式 | 复用 InputState 机制 | InputState 已有成熟的版本追踪 + 双向同步 |
| Stateful Slider 反向同步方式 | 新增 SliderState 桥接 | SliderState 有独立事件系统 |
| 用户 on_click + 自动回写 | 合并为单个 on_click 回调 | 避免双重事件订阅，自动回写先于用户回调 |
| 闭包字段 handler | 不合并，降级为仅自动回写 | ClosureField 无法提取方法名 |
| TWOWAY_BINDING_REGISTRY 位置 | `crates/engine/src/compiler/twoway.rs` | 独立模块，单一信源 |

### 5.2 关键假设

1. **gpui-component 组件事件签名稳定**：Checkbox/Switch 的 `on_click(&bool)`、Rating/RadioGroup/Stepper 的 `on_click(&usize)` 签名不会变化
2. **SliderState 实现 EventEmitter**：需 C3 实施前确认，若未实现则需走命令式 on_change 回调
3. **InputState 机制可复用于 TextInput/NumberInput**：TextInput 本质是 Input 的别名（[tags.rs:317-324](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L317-L324) `ctor_path` 均为 `rml_ui::Input`），NumberInput 也复用 InputState

### 5.3 铁律遵循

- 一个 rs 文件 = 一个职责（`twoway.rs` 仅含注册表和查询函数）
- 无 `rml_` 前缀（除框架内部 `__rml_*`）
- 禁止兼容性设计（双向绑定是新能力，不涉及旧 API 迁移）
- 属性命名禁止下划线
- `mod.rs` 仅 re-export

---

## 6. 与 Phase 2 组件扩展的协调

### 6.1 时间线

```
Phase C1 (Stateless 双向) ──┐
                             ├──> Phase C2 (InputStateBridge) ──> Phase C3 (SliderBridge) ──> Phase C4 (通用化)
                             │
Phase 2 B1-B2 (已完成) ──────┘
                             
Phase 2 B3 NumberInput <── 依赖 C2（InputStateBridge）
Phase 2 B5 ColorPicker <── 依赖 C4（通用 StateBridge）
Phase 2 B6 Calendar <── 依赖 C4
Phase 2 B7 DatePicker <── 依赖 C4
Phase 2 B8 Select <── 依赖 C4
Phase 2 B9 ComboBox <── 依赖 C4
```

### 6.2 对齐检查清单

- [ ] Phase 2 B2 Rating 描述修正（Stateful → Stateless）
- [ ] Phase 2 B3 NumberInput 实施时复用 C2 InputStateBridge
- [ ] Phase 2 B5-B9 实施时在 TWOWAY_BINDING_REGISTRY 注册
- [ ] Phase 2 全覆盖计划 A.2 Rating 描述修正
- [ ] 每个新 Stateful 组件实施前确认其 State 事件系统
