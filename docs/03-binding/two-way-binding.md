# 3.3 双向绑定

> **本节目标**：完整掌握 `model` 指令的双向绑定机制——基于 `Entity<InputState>` 的数据流、循环防护、类型转换、适用字段类型。

## 3.3.1 双向绑定的定义

双向绑定是 ViewModel 与 View 之间双向同步数据的绑定方式：

- ViewModel 字段变化 → View 更新（正向同步，VM→UI）
- View 用户输入 → ViewModel 字段更新（反向同步，UI→VM）

```html
<!-- 双向绑定：ViewModel ↔ View -->
<input model={user_name} />
```

### 实际实现机制

RML 的双向绑定基于 gpui-component 的 `InputState` entity 与 `InputEvent` 订阅模式：

- 每个 `<input model={field}>` 在首次 render 时**惰性创建**一个 `Entity<InputState>`
- **正向同步**（VM→UI）：render 时对比字段版本号，若变化则调用 `InputState::set_value`
- **反向同步**（UI→VM）：`cx.subscribe` 订阅 `InputEvent::Change`，闭包内回写字段 + `bump_version` + `cx.notify()`

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
对比 __rml_get_version("count") 与 __rml_input_state_versions["count"]
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
    this.__rml_input_state_versions.insert("name", v);  // 标记已同步
    cx.notify();                                  // 触发重绘
    ↓
下一次 render：对比版本号相等 → 跳过 set_value（循环防护）
```

### 循环防护

双向绑定面临循环风险：VM→UI 触发 UI→VM，反之亦然。RML 通过两层防护避免循环：

1. **`set_value` 内部 `emit_events=false`**：正向同步调用 `InputState::set_value` 不会触发 `InputEvent::Change`，切断 VM→UI→VM 循环
2. **版本号标记**：反向闭包内 `bump_version` 后立即更新 `__rml_input_state_versions`，render 时版本号相等跳过 `set_value`，切断 UI→VM→UI 循环

## 3.3.3 适用标签与字段类型

### 当前支持的标签

| 标签 | 说明 |
|---|---|
| `<input model={field}>` | 文本输入框（基于 `InputState`） |

> ⚠️ **未来支持**：`<textarea>`、`<input type="checkbox">`、`<input type="number">` 等特殊类型当前未实现 codegen，会回退为普通 `<input>` 文本输入。

### 支持的字段类型

codegen 根据 `#[component]` 宏扫描的字段类型自动生成转换代码：

| 字段类型 | 正向转换（VM→UI） | 反向转换（UI→VM） |
|---|---|---|
| `i32` / `u32` / `i64` / `u64` / `isize` / `usize` | `self.field.to_string().into()` | `value.parse::<T>().unwrap_or(0)` |
| `f32` | `self.field.to_string().into()` | `value.parse::<f32>().unwrap_or(0.0)` |
| `f64` | `self.field.to_string().into()` | `value.parse::<f64>().unwrap_or(0.0)` |
| `bool` | `self.field.to_string().into()` | `!value.is_empty()`（非空为 true） |
| `String` / `SharedString` | `self.field.clone().into()` | `value.to_string()` |

> 注：数字类型在输入非法字符时（如 `"abc"`）会兜底为 `0` / `0.0`，不会 panic。

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
<input model={username} placeholder="用户名" />
<input model={password} placeholder="密码" />
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
<input model={age} placeholder="年龄" />
<input model={score} placeholder="分数" />
```

用户输入 `"25"` 时，codegen 生成的反向闭包执行 `this.age = "25".parse::<i32>().unwrap_or(0)`，结果为 `25`。输入 `"abc"` 时兜底为 `0`。

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
<input model={name} placeholder="姓名" />
<input model={age} placeholder="年龄" />
<button onclick={increment}>+1</button>
<p>{profile_summary}</p>
```

## 3.3.5 双向绑定的字段要求

被 `model` 绑定的字段必须满足：

1. **`pub` 可见性**：codegen 生成的反向闭包需要访问 `this.field`
2. **类型可转换**：字段类型必须是上表列出的支持类型（`String`、`i32`、`f64` 等）
3. **`#[derive(Default)]` 或手动实现 `Default`**：用于 ViewModel 初始化（并非 `model` 机制本身的要求，而是整体框架惯例）

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

`model` 处理输入同步，`#[command]` 处理用户动作（如点击按钮）。两者独立工作：

```html
<input model={search_text} placeholder="搜索..." />
<button onclick={perform_search}>搜索</button>
```

```rust
#[command]
pub fn perform_search(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    // search_text 已经由 model 反向同步
    let query = self.search_text.clone();
    self.execute_query(&query, cx);
    // 宏自动注入 bump_version（若修改了字段）+ cx.notify()
}
```

> ⚠️ **避免在命令中重复修改 `model` 绑定的字段**：若 `#[command]` 修改了 `model` 绑定的字段（如 `self.name = ...`），会触发正向同步 `set_value`。虽然循环防护机制会避免无限循环，但可能导致用户输入被覆盖。

## 3.3.7 双向绑定的特殊场景

### 嵌套字段

`model` 不支持嵌套字段，需通过 `#[computed]` 派生或命令手动处理：

```html
<!-- ❌ 不支持嵌套字段 -->
<input model={user.profile.name} />

<!-- ✅ 通过 computed 派生为扁平字段 -->
```

```rust
#[computed]
pub fn display_name(&self) -> String {
    self.user.profile.name.clone()
}
```

### 列表中的双向绑定

`model` 在列表项中使用时，每个 item 需要独立的字段。当前 codegen 按字段名索引 `InputState`，列表项会共享同一个 `InputState`，导致冲突。

```html
<!-- ❌ 列表项的 model 会冲突 -->
<li each={user in users}>
    <input model={user.name} />
</li>
```

**解决方案**：列表项使用 `value={}` + `oninput={command, {index}}` 手动同步：

```html
<li each={index, user in users} key={user.id}>
    <input value={user.name} oninput={update_name, {index}} />
</li>
```

```rust
#[command]
pub fn update_name(&mut self, index: usize, ev: &InputEvent, cx: &mut Context<Self>) {
    self.users[index].name = ev.value.to_string();
}
```

### 自定义组件的双向绑定

当前 codegen 仅支持内置 `<input>` 标签的双向绑定。自定义组件（如 `<Slider>`、`<DatePicker>`）的双向绑定将在未来版本支持。

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

反向闭包内 `bump_version` 后立即更新 `__rml_input_state_versions`：

```rust
// codegen 生成的反向闭包（简化）
cx.subscribe(&entity, move |this, input_entity, event, cx| {
    match event {
        InputEvent::Change => {
            let value = input_entity.read(cx).value();
            this.name = value.to_string();              // 回写字段
            this.__rml_bump_version("name");             // 版本号 +1
            let v = this.__rml_get_version("name");
            this.__rml_input_state_versions.insert("name".to_string(), v);  // 标记已同步
            cx.notify();                                 // 触发重绘
        }
        _ => {}
    }
}).detach();
```

下一次 render 时，`__rml_get_or_init_input_state` 对比版本号：

```rust
let current_version = self.__rml_get_version("name");           // 反向闭包 bump 后的版本号
let last_synced = self.__rml_input_state_versions.get("name").copied().unwrap_or(0);
// current_version == last_synced（反向闭包已标记）
// → 跳过 set_value，切断 UI→VM→UI 循环
```

### 防护失效的场景

若用户在 `#[command]` 中手动修改 `model` 绑定的字段：

```rust
#[command]
pub fn uppercase_name(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.name = self.name.to_uppercase();  // 修改了 model 绑定的字段
    // 宏自动注入：bump_version("name")，但未更新 __rml_input_state_versions
}
```

此时 `current_version > last_synced`，render 时触发 `set_value` 将输入框值更新为大写。这是预期行为（用户主动修改），循环防护仍有效（`set_value` 不触发 Change）。

## 3.3.9 双向绑定的性能

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

## 3.3.10 常见陷阱

### 陷阱一：忘记 `pub`

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    user_name: String,  // ❌ 非 pub，反向闭包无法访问 this.user_name
}
```

codegen 会生成 `this.user_name = value.to_string()`，但 `user_name` 非 `pub` 导致编译错误（实际上 codegen 在 `impl MyView` 块内，可以访问私有字段，但 `IModel::rml_fields()` 不会收集非 pub 字段，版本号追踪字段也不会注入，导致 `__rml_bump_version("user_name")` 编译失败）。

### 陷阱二：在命令中修改 model 绑定的字段

```rust
#[command]
pub fn on_input(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
    // ❌ 与 model 反向闭包冲突
    self.user_name = ev.value.to_uppercase().into();
}
```

若需要在输入时转换值，应该用 `value={}` + `oninput` + 命令的方式，而不是 `model`。或者用 `#[computed]` 派生显示值：

```rust
#[computed]
pub fn display_name(&self) -> String {
    self.user_name.to_uppercase()
}
```

### 陷阱三：列表中使用 model

```html
<!-- ❌ 列表项的 model 会冲突（共享同一个 InputState） -->
<li each={user in users}>
    <input model={user.name} />
</li>
```

列表项的 `model` 需通过 `value={}` + `oninput={command, {index}}` 手动处理，详见 3.3.7。

### 陷阱四：不支持的字段类型

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    pub data: Vec<String>,  // ❌ codegen 生成 self.data.clone().into()，运行时无意义
    pub timestamp: u64,     // ✅ 支持的整数类型
}
```

若字段类型不在支持列表中（见 3.3.3），codegen 仍会生成代码（编译通过），但运行时行为未定义。请仅使用支持的字段类型。

## 3.3.11 小结

双向绑定是表单输入的核心机制：

- **语法**：`<input model={field} placeholder="..." />`
- **机制**：基于 `Entity<InputState>` + `cx.subscribe(InputEvent::Change)` + 版本号追踪
- **正向同步**：render 时对比版本号，变化则 `set_value`
- **反向同步**：`InputEvent::Change` 触发订阅闭包，回写字段 + `bump_version` + `notify`
- **循环防护**：`set_value` 内部 `emit_events=false` + 版本号标记双层防护
- **字段要求**：`pub` + 支持的类型（`String`、`i32`、`f64` 等）

记住：`model` 是基于 `InputState` entity 的完整双向数据流，而非简单的语法糖。需要更细粒度控制时，回退到 `value={}` + `oninput` + 命令的手动方式。

下一节 → [3.4 计算属性](./computed.md)
