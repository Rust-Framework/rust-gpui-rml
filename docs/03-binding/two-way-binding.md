# 3.3 双向绑定

> **本节目标**：完整掌握 `value={field}` 自动双向绑定机制——基于 `Entity<InputState>` / StateBridge 的数据流、循环防护、类型转换、适用字段类型与组件范围。

## 3.3.1 双向绑定的定义

双向绑定是 ViewModel 与 View 之间双向同步数据的绑定方式：

- ViewModel 字段变化 → View 更新（正向同步，VM→UI）
- View 用户输入 → ViewModel 字段更新（反向同步，UI→VM）

```html
<!-- 双向绑定：ViewModel ↔ View -->
<input value={user_name} />
```

### 自动推断原则

RML 遵循**自动推断双向绑定**原则：双向能力是属性的固有语义，框架自动识别。当 `value={field}` / `checked={field}` / `selected_index={field}` 绑定到可变字段时，框架自动启用双向同步。

### 三类双向绑定机制

| 机制 | 适用组件 | 绑定属性 | 反向同步方式 |
|------|---------|---------|-------------|
| **Stateless EventClick** | Checkbox / Switch / Radio / Rating / RadioGroup / Stepper | `checked` / `value` / `selected_index` | `on_click(&bool/&usize)` 事件注入回写 |
| **Stateful InputStateBridge** | `<input>` / `<textarea>` / `<Input>` / `<TextInput>` / `<NumberInput>` | `value` | `InputState` entity + `InputEvent::Change` 订阅 |
| **Stateful StateBridge** | `<Slider>` | `value` | State entity + 事件订阅（注册表驱动） |

### 实际实现机制

RML 的双向绑定基于 gpui-component 的 `InputState` / `SliderState` entity 与事件订阅模式：

- 每个 `value={field}` 绑定在首次 render 时**惰性创建**对应的 State Entity
- **正向同步**（VM→UI）：render 时对比字段版本号，若变化则调用 `State::set_value`
- **反向同步**（UI→VM）：`cx.subscribe` 订阅事件，闭包内回写字段 + `bump_version` + `cx.notify()`

这并非简单的 `value={field} + oninput={update_field}` 语法糖，而是基于 entity 生命周期管理的完整双向数据流。

## 3.3.2 双向绑定的数据流

### 正向同步（VM → UI）

```
#[command] 修改字段（如 self.count += 1）
    ↓
宏自动注入 __rml_bump_version("count")
    ↓
宏自动注入 cx.notify()
    ↓
下一次 render：调用 __rml_get_or_init_input_state("count", ...)
    ↓
对比 __rml_get_version("count") 与 __rml_state.input_state_versions["count"]
    ↓ 版本号不同
entity.update(cx, |state, cx| state.set_value(value, window, cx))
    ↓
InputState 更新显示值（emit_events=false，不触发 Change）
```

### 反向同步（UI → VM）

```
用户在输入框输入 "John"
    ↓
InputState 触发 InputEvent::Change
    ↓
cx.subscribe 的闭包被调用：
    let value = input_entity.read(cx).value();  // "John"
    match field {
        "name" => this.name = value.to_string();  // 回写字段
    }
    this.__rml_bump_version("name");              // 版本号 +1
    this.__rml_state.input_state_versions.insert("name", v);  // 标记已同步
    cx.notify();                                  // 触发重绘
    ↓
下一次 render：对比版本号相等 → 跳过 set_value（循环防护）
```

### 循环防护

双向绑定面临循环风险：VM→UI 触发 UI→VM，反之亦然。RML 通过两层防护避免循环：

1. **`set_value` 内部 `emit_events=false`**：正向同步调用 `InputState::set_value` 不会触发 `InputEvent::Change`，切断 VM→UI→VM 循环
2. **版本号标记**：反向闭包内 `bump_version` 后立即更新 `__rml_state.input_state_versions`，render 时版本号相等跳过 `set_value`，切断 UI→VM→UI 循环

> ℹ️ 上述数据流以 InputStateBridge（`<input>` / `<Input>` 等）为例。Stateful StateBridge（`<Slider>` 等）遵循相同模式，仅将 `__rml_get_or_init_input_state` 替换为 `__rml_get_or_init_<suffix>_state`，版本号存储于 `state_bridge_entities` 而非 `input_state_versions`。Stateless EventClick（Checkbox/Switch/Rating 等）无需 State entity，直接通过 `on_click` 事件回写字段。

## 3.3.3 适用标签与字段类型

### 当前支持的标签

RML 对小写原生标签和 PascalCase 组件均支持自动双向绑定，分属三类机制：

| 标签 | 机制 | 绑定属性 | 说明 |
|---|---|---|---|
| `<input>` | InputStateBridge | `value` | 文本输入框（基于 `InputState`） |
| `<textarea>` | InputStateBridge | `value` | 多行文本（基于 `InputState`） |
| `<Input>` | InputStateBridge | `value` | PascalCase 文本输入（复用 `InputState`） |
| `<TextInput>` | InputStateBridge | `value` | PascalCase 文本输入（复用 `InputState`） |
| `<NumberInput>` | InputStateBridge | `value` | 数字输入（复用 `InputState`） |
| `<Checkbox>` | Stateless EventClick | `checked` | 勾选框（`on_click(&bool)` 回写） |
| `<Switch>` | Stateless EventClick | `checked` | 开关（`on_click(&bool)` 回写） |
| `<Radio>` | Stateless EventClick | `checked` | 单选项（`on_click(&bool)` 回写） |
| `<Rating>` | Stateless EventClick | `value` | 评分（`on_click(&usize)` 回写） |
| `<RadioGroup>` | Stateless EventClick | `selected_index` | 单选组（`on_click(&usize)` 回写） |
| `<Stepper>` | Stateless EventClick | `value` | 步进器（`on_click(&usize)` 回写） |
| `<Slider>` | Stateful StateBridge | `value` | 滑块（基于 `SliderState`，注册表驱动） |

> ℹ️ **扩展机制**：新增 Stateful 表单组件时，在 `STATE_BRIDGE_REGISTRY` 注册 `StateBridgeSpec` 即可自动获得 `value={field}` 双向绑定能力，无需修改 codegen 主流程。Stateless 组件通过 `twoway.rs` 的事件注入模式扩展。

### 支持的字段类型

codegen 根据 `#[component]` 宏扫描的字段类型自动生成转换代码：

| 字段类型 | 正向转换（VM→UI） | 反向转换（UI→VM） | 校验失败行为 |
|---|---|---|---|
| `i32` / `u32` / `i64` / `u64` / `isize` / `usize` | `self.field.to_string().into()` | `match value.parse::<T>() { Ok(v) => 赋值, Err(_) => 设置错误 }` | 保留原值 + 红色边框 + tooltip"请输入有效的整数" |
| `f32` | `self.field.to_string().into()` | `match value.parse::<f32>() { ... }` | 保留原值 + 红色边框 + tooltip"请输入有效的数字" |
| `f64` | `self.field.to_string().into()` | `match value.parse::<f64>() { ... }` | 保留原值 + 红色边框 + tooltip"请输入有效的数字" |
| `bool` | `self.field.to_string().into()` | `!value.is_empty()` | 总是成功（无校验失败场景） |
| `String` / `SharedString` | `self.field.clone().into()` | `value.to_string()` | 总是成功（无校验失败场景） |

> 注：数字类型 parse 失败时（类型不匹配如 `"abc"`、类型溢出如 `"99999999999999999999"` 给 i32）**不覆盖原值**，仅设置校验错误状态，UI 显示红色边框 + tooltip。

### 转换器（Converter）

当字段类型与输入框显示格式不一致时，用 `value={field | Converter}` 声明转换器。转换器实现 `IConverter` trait，提供两个方向的自定义转换：

| 方向 | 调用 | 作用 |
|---|---|---|
| 正向（VM→UI） | `Converter.convert(&self.field)` | 字段值 → 显示串（如 `1500.0` → `"¥1500.00"`） |
| 反向（UI→VM） | `Converter.convert_back(&value)` | 输入串 → 字段值（如 `"¥1500.00"` → `Some(1500.0)`） |

框架内置转换器（`rml::prelude::*`）：`Currency`（f64↔`¥#.##`）、`Percent`、`UpperCase`/`LowerCase`、`Trim`、`BoolToYesNo`。

```rust
#[derive(Default)]
#[component]
pub struct OrderView {
    pub price: f64,  // VM 中存原始数值
}
```

```html
<!-- 正向显示 ¥1500.00，反向解析 ¥1500.00 → 1500.0 -->
<input value={price | Currency} placeholder="输入 ¥1500.00" />
```

codegen 生成（简化）：

```rust
// 正向（初始值 + 版本号变化时）
"price" => Currency.convert(&self.price).into(),  // 而非 to_string()

// 反向（InputEvent::Change）
match Currency.convert_back(&value.to_string()) {
    Some(v) => { this.price = v; bump_version("price"); }  // 解析成功
    None    => { __rml_state.field_errors["price"] = Some("转换失败".into()); }  // 解析失败
}
```

> ⚠️ `convert_back` 返回 `None` 时**不覆盖原值**，设置 `"转换失败"` 错误状态，UI 显示红色边框 + tooltip。无 converter 的字段仍走裸 `parse` 路径。

## 3.3.4 基础用法

### 文本输入

```rust
#[derive(Default)]
#[component]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}
```

```html
<input value={username} placeholder="用户名" />
<input value={password} placeholder="密码" />
```

### 数字输入

```rust
#[derive(Default)]
#[component]
pub struct Settings {
    pub age: i32,
    pub score: f64,
}
```

```html
<input value={age} placeholder="年龄" />
<input value={score} placeholder="分数" />
```

用户输入 `"25"` 时，codegen 生成的反向闭包执行 `match "25".parse::<i32>() { Ok(v) => this.age = v, Err(_) => 设置错误状态 }`，结果为 `25`。输入 `"abc"` 时 parse 失败，**保留原值**，仅设置 `__rml_state.field_errors["age"] = Some("请输入有效的整数")`，UI 显示红色边框 + tooltip。

### PascalCase 组件

PascalCase 表单组件同样自动双向绑定，无需额外声明：

```html
<!-- Checkbox / Switch：checked={field} 自动双向 -->
<Checkbox checked={agree} label="同意条款" />
<Switch checked={notifications} />

<!-- Rating：value={field} 自动双向（usize 字段） -->
<Rating value={score} max="5" />

<!-- Slider：value={field} 自动双向（f32 字段，StateBridge 机制） -->
<Slider value={volume} />

<!-- Input：value={field} 自动双向（复用 InputState） -->
<Input value={username} placeholder="用户名" />
```

### 完整示例（来自 demo）

```rust
// crates/demo/src/main_window.rml.rs
#[derive(Default)]
#[window]
pub struct MainWindow {
    pub name: String,
    pub age: i32,
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn profile_summary(&self) -> String {
        format!("姓名: {}, 年龄: {}, 计数: {}", self.name, self.age, self.count)
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        // 宏自动注入：bump_version("count") + cx.notify()
    }
}
```

```html
<!-- crates/demo/src/main_window.rml -->
<input value={name} placeholder="姓名" />
<input value={age} placeholder="年龄" />
<button on-click={increment}>+1</button>
<p>{profile_summary}</p>
```

## 3.3.5 双向绑定的字段要求

被 `value={field}` / `checked={field}` / `selected_index={field}` 绑定的字段必须满足：

1. **`pub` 可见性**：codegen 生成的反向闭包需要访问 `this.field`
2. **类型可转换**：字段类型必须是上表列出的支持类型（`String`、`i32`、`f64`、`bool`、`usize`、`f32` 等）
3. **`#[derive(Default)]` 或手动实现 `Default`**：用于 ViewModel 初始化（并非绑定机制本身的要求，而是整体框架惯例）

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    pub user_name: String,   // ✅ pub + 支持的类型
    pub age: i32,            // ✅ pub + 支持的类型

    // ❌ 不满足要求的字段
    private_field: i32,     // 非 pub，反向闭包无法访问
    pub data: Vec<String>,   // 不支持的类型（codegen 会生成 to_string() 但运行时无意义）
}
```

## 3.3.6 与命令的协作

`value={field}` 处理输入同步，`#[command]` 处理用户动作（如点击按钮）。两者独立工作：

```html
<input value={search_text} placeholder="搜索..." />
<button on-click={perform_search}>搜索</button>
```

```rust
#[command]
pub fn perform_search(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    // search_text 已经由双向绑定反向同步
    let query = self.search_text.clone();
    self.execute_query(&query, cx);
    // 宏自动注入 bump_version（若修改了字段）+ cx.notify()
}
```

> ⚠️ **避免在命令中重复修改双向绑定的字段**：若 `#[command]` 修改了双向绑定的字段（如 `self.name = ...`），会触发正向同步 `set_value`。虽然循环防护机制会避免无限循环，但可能导致用户输入被覆盖。

### `oninput` / `onchange` 与双向绑定协作

`<input value={field}>` 可同时声明 `oninput={fn}` / `onchange={fn}`。handler 在反向同步**之后**、`cx.notify()` **之前**调用，可读取已同步的最新字段值：

```html
<input value={name} oninput={on_name_input} />
<input value={age} onchange={on_age_change} />
```

```rust
#[command]
pub fn on_name_input(&mut self, _ev: &InputEvent, cx: &mut Context<Self>) {
    // name 字段已被双向绑定反向同步，self.name 是最新值
    self.input_count += 1;
    cx.notify();
}

#[command]
pub fn on_age_change(&mut self, _ev: &ChangeEvent, cx: &mut Context<Self>) {
    // onchange 在失焦/回车时触发，与 oninput 的逐键触发互补
    self.change_count += 1;
    cx.notify();
}
```

| 事件 | 触发时机 | 与双向绑定的关系 |
|---|---|---|
| `oninput` | 每次输入（逐键） | 已反向同步，可读最新字段 |
| `onchange` | 失焦 / 回车（值提交） | 已反向同步，适合提交类逻辑 |

> `oninput`/`onchange` handler 签名为 `(&InputEvent, &mut Context<Self>)` / `(&ChangeEvent, &mut Context<Self>)`，与独立 `<input>`（无双向绑定）上的事件绑定一致。

## 3.3.7 双向绑定的特殊场景

### 嵌套字段

双向绑定不支持嵌套字段，需通过 `#[computed]` 派生或命令手动处理：

```html
<!-- ❌ 不支持嵌套字段 -->
<input value={user.profile.name} />

<!-- ✅ 通过 computed 派生为扁平字段 -->
```

```rust
#[computed]
pub fn display_name(&self) -> String {
    self.user.profile.name.clone()
}
```

### 自定义组件的双向绑定

PascalCase 表单组件的自动双向绑定分属三类机制（详见 3.3.1）：

- **Stateless EventClick**：`Checkbox` / `Switch` / `Radio` / `Rating` / `RadioGroup` / `Stepper` — 通过 `twoway.rs` 注入 `on_click` 事件回写字段
- **Stateful InputStateBridge**：`<Input>` / `<TextInput>` / `<NumberInput>` — 复用 `InputState` entity 双向同步
- **Stateful StateBridge**：`<Slider>` — 通过 `STATE_BRIDGE_REGISTRY` 注册表驱动

**扩展 Stateful 组件**：新增 Stateful 表单组件时，在 `state_bridge.rs` 的 `STATE_BRIDGE_REGISTRY` 中注册 `StateBridgeSpec`：

```rust
// crates/engine/src/compiler/state_bridge.rs
pub static STATE_BRIDGE_REGISTRY: &[StateBridgeSpec] = &[
    StateBridgeSpec {
        tag: "Slider",
        bind_property: "value",
        bridge_key: "slider",
        state_type: "rml_ui::SliderState",
        state_ctor: "rml_ui::SliderState::default()",
        state_method_suffix: "slider",
        // ... 事件匹配、值提取/设置模板
    },
    // 新增组件在此注册
];
```

注册后，codegen 自动为该组件生成 `__rml_get_or_init_<suffix>_state` 方法，无需修改主流程。

## 3.3.8 循环防护机制详解

双向绑定的核心难点是避免数据流循环。RML 采用两层防护：

### 防护一：`set_value` 不触发 Change 事件

gpui-component 的 `InputState::set_value(value, window, cx)` 内部设 `emit_events = false`：

```rust
// InputState 源码（简化）
pub fn set_value(&mut self, value: impl Into<SharedString>, window: &mut Window, cx: &mut Context<Self>) {
    self.value = value.into();
    self.emit_events = false;  // 不触发 InputEvent::Change
    cx.notify();
}
```

这意味着正向同步（VM→UI）调用 `set_value` 后，**不会**触发反向闭包，切断 `VM→UI→VM` 循环。

### 防护二：版本号标记

反向闭包内 `bump_version` 后立即更新 `__rml_state.input_state_versions`：

```rust
// codegen 生成的反向闭包（简化）
cx.subscribe(&entity, move |this, input_entity, event, cx| {
    match event {
        InputEvent::Change => {
            let value = input_entity.read(cx).value();
            this.name = value.to_string();              // 回写字段
            this.__rml_bump_version("name");             // 版本号 +1
            let v = this.__rml_get_version("name");
            this.__rml_state.input_state_versions.insert("name".to_string(), v);  // 标记已同步
            cx.notify();                                 // 触发重绘
        }
        _ => {}
    }
}).detach();
```

下一次 render 时，`__rml_get_or_init_input_state` 对比版本号：

```rust
let current_version = self.__rml_get_version("name");           // 反向闭包 bump 后的版本号
let last_synced = self.__rml_state.input_state_versions.get("name").copied().unwrap_or(0);
// current_version == last_synced（反向闭包已标记）
// → 跳过 set_value，切断 UI→VM→UI 循环
```

### 防护失效的场景

若用户在 `#[command]` 中手动修改双向绑定的字段：

```rust
#[command]
pub fn uppercase_name(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.name = self.name.to_uppercase();  // 修改了双向绑定的字段
    // 宏自动注入：bump_version("name")，但未更新 __rml_state.input_state_versions
}
```

此时 `current_version > last_synced`，render 时触发 `set_value` 将输入框值更新为大写。这是预期行为（用户主动修改），循环防护仍有效（`set_value` 不触发 Change）。

### 校验失败时的循环防护

当用户输入非法值（如 `"abc"` 给 `i32` 字段）时：
- 反向闭包内 `parse` 失败，进入 `Err(_)` 分支
- **不调用** `__rml_bump_version`（字段值未变）
- **不调用** `cx.notify()`（但闭包末尾仍有 notify，触发一次重绘以显示错误 UI）
- 设置 `__rml_state.field_errors[field] = Some("请输入有效的整数")`
- 下一次 render：版本号未变（`current_version == last_synced`），跳过 `set_value`
- render 时检查 `__rml_state.field_errors`，发现 `Some`，包裹红色边框 + tooltip

## 3.3.9 校验失败 UI

当用户输入非法值时，RML 自动显示校验失败的视觉反馈：

### UI 表现

- **红色边框**：通过 `Styled` trait 直接对 Input 调用 `.border_color(gpui::rgb(0xff0000))`，覆盖 Input 自身的主题色边框（input.rs 内部 `border_color(theme_color)` → `refine_style(&self.style)` 顺序保证用户样式覆盖主题色）。校验通过时恢复正常主题色。
- **tooltip 气泡**：Input 被包裹在 `div().id(...).tooltip(...)` 中，hover 时显示错误提示（如"请输入有效的整数"），不占用布局空间。wrapper div 仅承载 tooltip，不再附加边框。

### 错误状态生命周期

```
用户输入 "abc"（i32 字段）
    ↓
反向闭包 parse 失败 → __rml_state.field_errors["age"] = Some("请输入有效的整数")
    ↓
cx.notify() → render → 检查 __rml_state.field_errors → 显示红色边框 + tooltip
    ↓
用户输入 "25"（有效值）
    ↓
反向闭包 parse 成功 → __rml_state.field_errors["age"] = None + bump_version + cx.notify()
    ↓
render → 检查 __rml_state.field_errors → None → 直接返回 Input（无边框）
    ↓
正向同步（版本号变化）→ set_value("25") → __rml_state.field_errors["age"] = None（冗余清除）
```

### 正向同步清除错误

当 `#[command]` 修改双向绑定的字段后，正向同步 `set_value` 会自动清除错误状态：

```rust
#[command]
pub fn reset_age(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.age = 0;  // 代码设置的值视为有效
    // 宏自动注入：bump_version("age") + cx.notify()
}
```

render 时版本号变化 → `set_value("0")` → `__rml_state.field_errors["age"] = None` → 红色边框消失。

### 默认错误消息

| 字段类型 | 错误消息 |
|---|---|
| 整数（i32/u32/i64/u64/isize/usize） | `请输入有效的整数` |
| 浮点（f32/f64） | `请输入有效的数字` |

> 业务范围校验通过 `#[validate]` 宏（C# Attribute 风格）实现，不污染 RML 声明语法。详见 [3.3.10 自定义校验规则](#3310-自定义校验规则)。

## 3.3.10 自定义校验规则

`#[validate]` 属性为双向绑定字段声明校验规则，在 parse 成功后、赋值前执行校验链。任一规则失败 → 设置错误状态（不赋值、不 `bump_version`），全部通过 → 赋值 + 清除错误 + `bump_version`。

### 声明方式

`#[validate]` 是字段级属性，放在 `pub` 字段上：

```rust
#[window]
#[derive(Default)]
pub struct Form {
    #[validate(range(min = 0, max = 150))]
    pub age: i32,

    #[validate(required, length(min = 3, max = 20))]
    pub name: String,

    #[validate(regex = r"^\w+@\w+\.\w+$", message = "邮箱格式错误")]
    pub email: String,
}
```

### 规则与字段类型匹配

| 规则 | 适用类型 | 生成代码 | 默认消息 |
|---|---|---|---|
| `required` | `String` | `__rml_value.is_empty()` | 此项为必填 |
| `length(min, max)` | `String` | `__rml_value.len() < min \|\| .len() > max` | 长度必须在 min-max 之间 |
| `range(min, max)` | 数字类型 | `v < min \|\| v > max` | 值必须在 min-max 之间 |
| `regex = "..."` | `String` | `rml::regex::Regex::new(...).is_match(...)` | 格式不正确 |
| `custom = "fn"` | 数字/String | `Self::fn(&value)` → `Option<SharedString>` | 由函数返回 |

- `min`/`max` 任一可省略（仅校验单边）
- `bool` 类型忽略所有校验规则
- 数字类型忽略 `required`/`length`/`regex`；String 类型忽略 `range`

### 自定义校验函数

`custom = "fn_name"` 引用 `impl ViewName` 块内的函数，签名为 `fn(&str) -> Option<SharedString>`：

```rust
impl Form {
    fn validate_phone(value: &str) -> Option<SharedString> {
        if value.len() != 11 || !value.chars().all(|c| c.is_ascii_digit()) {
            Some("手机号必须为 11 位数字".into())
        } else {
            None
        }
    }
}

#[derive(Default)]
#[window]
pub struct Form {
    #[validate(custom = "validate_phone")]
    pub phone: String,
}
```

返回 `Some(消息)` 表示校验失败，`None` 表示通过。数字类型的 `custom` 函数接收原始字符串（`value.as_ref()`），而非解析后的数字。

### 错误消息覆盖

`message = "..."` 全局覆盖所有失败分支的默认消息：

```rust
// 各规则使用默认消息
#[validate(range(min = 0, max = 150))]
pub age: i32,  // 失败显示 "值必须在 0-150 之间"

// 统一覆盖
#[validate(range(min = 0, max = 150), message = "年龄不合法")]
pub age: i32,  // 失败显示 "年龄不合法"
```

### 校验链执行顺序

多个规则按声明顺序执行，形成 if-else if 链：

```rust
#[validate(required, length(min = 3, max = 20))]
pub name: String,
```

生成代码（简化）：

```rust
let __rml_value = value.to_string();
if __rml_value.is_empty() {
    this.__rml_state.field_errors.insert("name", Some("此项为必填".into()));      // required 先执行
} else if __rml_value.len() < 3 || __rml_value.len() > 20 {
    this.__rml_state.field_errors.insert("name", Some("长度必须在 3-20 之间".into())); // length 后执行
} else {
    this.name = __rml_value;
    this.__rml_state.field_errors.insert("name", None);  // 清除错误
    this.__rml_bump_version("name");
}
```

### 完整示例

```rust
#[window]
#[derive(Default)]
pub struct RegistrationForm {
    #[validate(required, length(min = 2, max = 30))]
    pub username: String,

    #[validate(range(min = 18, max = 150))]
    pub age: i32,

    #[validate(regex = r"^\w+@\w+\.\w+$")]
    pub email: String,

    #[validate(custom = "validate_phone", message = "手机号格式错误")]
    pub phone: String,
}
```

```html
<input value={username} placeholder="用户名（2-30 字符）" />
<input value={age} placeholder="年龄（18-150）" />
<input value={email} placeholder="邮箱" />
<input value={phone} placeholder="手机号" />
```

### IValidate 接口式校验

当内置规则无法表达复杂校验逻辑时，可通过 `IValidate` trait 自定义校验器，用 `#[validate(MyValidator)]` 引用。与规则式（range/length/required/regex/custom）+ `message` 互斥。

#### 声明方式

`MyValidator` 必须实现 `IValidate` + `Default`：

```rust
use rml::prelude::*;  // 引入 IValidate + ValidResult

#[derive(Default)]
struct EmailValidator;

impl IValidate for EmailValidator {
    fn valid(&self, value: &str) -> ValidResult {
        if value.contains('@') && value.contains('.') {
            ValidResult::Pass
        } else {
            ValidResult::Fail("邮箱格式错误".into())
        }
    }
}

#[window]
#[derive(Default)]
pub struct Form {
    #[validate(EmailValidator)]
    pub email: String,
}
```

`IValidate` 提供三个方法（均有默认实现）：
- `valid(&self, value: &str) -> ValidResult`：简单校验
- `valid_with_view(&self, value: &str, view: &dyn Any) -> ValidResult`：带视图上下文（默认委托给 `valid`）
- `message(&self, result: &ValidResult) -> Option<SharedString>`：结果→消息转换

#### 跨字段校验（Context 注入）

重写 `valid_with_view`，通过 `view.downcast_ref::<MyView>()` 访问视图的其他字段。codegen 自动将 `&self` 作为 `&dyn Any` 注入：

```rust
#[derive(Default)]
struct PasswordConfirmValidator;

impl IValidate for PasswordConfirmValidator {
    fn valid_with_view(&self, value: &str, view: &dyn std::any::Any) -> ValidResult {
        if let Some(form) = view.downcast_ref::<RegistrationForm>() {
            if value != form.password {
                return ValidResult::Fail("两次输入的密码不一致".into());
            }
        }
        ValidResult::Pass
    }
}

#[window]
#[derive(Default)]
pub struct RegistrationForm {
    pub password: String,
    #[validate(PasswordConfirmValidator)]
    pub password_confirm: String,
}
```

#### codegen 行为

以 `email: String` + `#[validate(EmailValidator)]` 为例：

```rust
{
    let __rml_value = value.to_string();
    let __rml_validator = EmailValidator::default();
    let __rml_result = __rml_validator.valid_with_view(&__rml_value, this as &dyn std::any::Any);
    if let Some(__rml_err_msg) = __rml_validator.message(&__rml_result) {
        this.__rml_state.field_errors.insert("email".to_string(), Some(__rml_err_msg));
    } else {
        this.email = __rml_value;
        this.__rml_state.field_errors.insert("email".to_string(), None);
        this.__rml_bump_version("email");
    }
}
```

数字字段（如 `age: i32`）外层包裹 `match value.parse::<i32>()`，parse 失败仍走默认类型错误消息。

详见 [4.2.9 #[validate] 宏](../04-code-behind/macros.md#ivalidate-接口式校验) 的 IValidate 接口式校验章节。

## 3.3.11 双向绑定的性能

双向绑定比单向绑定多以下开销：

| 操作 | 时机 | 开销 |
|---|---|---|
| `Entity<InputState>` 创建 | 首次 render | 一次性，后续复用 |
| `cx.subscribe` + `.detach()` | 首次 render | 一次性，订阅随 entity 存活 |
| 版本号对比 | 每次 render | `AtomicU64::load` + 比较，O(1) |
| `set_value` 调用 | 仅版本号变化时 | 字符串拷贝 + GPUI 内部更新 |
| 反向闭包执行 | 用户每次输入 | 字段回写 + `HashMap::insert` + `cx.notify()` |

**关键优化**：
- 正向同步仅在版本号变化时调用 `set_value`，避免冗余更新
- 反向同步的 `cx.notify()` 会批量合并，多次输入只触发一次 render
- `Subscription.detach()` 不占用结构体字段，避免 `Vec<Subscription>` 的内存开销

## 3.3.12 常见陷阱

### 陷阱一：忘记 `pub`

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    user_name: String,  // ❌ 非 pub，反向闭包无法访问 this.user_name
}
```

codegen 会生成 `this.user_name = value.to_string()`，但 `user_name` 非 `pub` 导致编译错误（实际上 codegen 在 `impl MyView` 块内，可以访问私有字段，但 `IModel::rml_fields()` 不会收集非 pub 字段，版本号追踪字段也不会注入，导致 `__rml_bump_version("user_name")` 编译失败）。

### 陷阱二：在命令中修改双向绑定的字段

```rust
#[command]
pub fn on_input(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
    // ❌ 与反向闭包冲突
    self.user_name = ev.value.to_uppercase().into();
}
```

若需要在输入时转换值，应该用单向 `value={expr}` + `oninput` + 命令的方式（不触发自动双向绑定），或者用 `#[computed]` 派生显示值：

```rust
#[computed]
pub fn display_name(&self) -> String {
    self.user_name.to_uppercase()
}
```

### 陷阱三：不支持的字段类型

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    pub data: Vec<String>,  // ❌ codegen 生成 self.data.clone().into()，运行时无意义
    pub timestamp: u64,     // ✅ 支持的整数类型
}
```

若字段类型不在支持列表中（见 3.3.3），codegen 仍会生成代码（编译通过），但运行时行为未定义。请仅使用支持的字段类型。

## 3.3.13 小结

双向绑定是表单输入的核心机制：

- **语法**：`<input value={field} placeholder="..." />`、`<Checkbox checked={field} />`、`<Slider value={field} />`
- **自动推断**：`value={field}` / `checked={field}` / `selected_index={field}` 绑定到可变字段时自动启用双向同步
- **三类机制**：Stateless EventClick（Checkbox/Switch/Rating 等）、Stateful InputStateBridge（input/Input/TextInput）、Stateful StateBridge（Slider）
- **正向同步**：render 时对比版本号，变化则 `set_value`
- **反向同步**：事件触发订阅闭包，回写字段 + `bump_version` + `notify`
- **循环防护**：`set_value` 内部 `emit_events=false` + 版本号标记双层防护
- **字段要求**：`pub` + 支持的类型（`String`、`i32`、`f64`、`bool`、`usize`、`f32` 等）
- **扩展性**：新增 Stateful 组件在 `STATE_BRIDGE_REGISTRY` 注册即可获得双向绑定能力

记住：双向绑定是基于 `InputState` / `StateBridge` entity 的完整双向数据流，而非简单的语法糖。需要更细粒度控制时，回退到单向 `value={expr}` + `oninput` + 命令的手动方式。

下一节 → [3.4 计算属性](./computed.md)
