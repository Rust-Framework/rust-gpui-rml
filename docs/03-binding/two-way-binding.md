# 3.3 双向绑定

> **本节目标**：完整掌握 `model` 指令的双向绑定机制——数据流方向、适用标签、字段要求、与命令的协作。

## 3.3.1 双向绑定的定义

双向绑定是 ViewModel 与 View 之间双向同步数据的绑定方式：

- ViewModel 字段变化 → View 更新（单向绑定）
- View 用户输入 → ViewModel 字段更新（反向绑定）

```html
<!-- 双向绑定：ViewModel ↔ View -->
<input model={user_name} />
```

等价于：

```html
<input
    value={user_name}
    oninput={update_user_name}
/>
```

其中 `update_user_name` 是 RML 自动生成的命令，把输入值赋给 `user_name` 字段。

## 3.3.2 双向绑定的数据流

```
用户输入 "John"
    ↓
View 触发 oninput 事件
    ↓
RML 自动生成的命令执行：
    self.user_name = "John".into();
    cx.notify();
    ↓
ViewModel.user_name 更新
    ↓
cx.notify() 触发重绘
    ↓
View 重新读取 {user_name} 显示
```

## 3.3.3 适用标签与字段类型

| 标签                       | 字段类型                | 说明           |
| ------------------------ | ------------------- | ------------ |
| `<input type="text">`    | `SharedString`      | 文本输入         |
| `<input type="password">` | `SharedString`      | 密码输入         |
| `<input type="number">`  | `i32`、`u32`、`f64` 等 | 数字输入         |
| `<input type="email">`   | `SharedString`      | 邮箱输入         |
| `<input type="checkbox">` | `bool`              | 复选框          |
| `<textarea>`             | `SharedString`      | 多行文本         |

## 3.3.4 基础用法

### 文本输入

```rust
#[derive(IModel)]
#[component]
pub struct LoginForm {
    pub username: SharedString,
    pub password: SharedString,
}
```

```html
<input model={username} placeholder="用户名" />
<input type="password" model={password} placeholder="密码" />
```

### 数字输入

```rust
#[derive(IModel)]
#[component]
pub struct Settings {
    pub age: i32,
    pub score: f64,
}
```

```html
<input type="number" model={age} min="0" max="150" />
<input type="number" model={score} step="0.1" />
```

### 复选框

```rust
#[derive(IModel)]
#[component]
pub struct Preferences {
    pub remember_me: bool,
    pub auto_save: bool,
}
```

```html
<label>
    <input type="checkbox" model={remember_me} />
    记住我
</label>
<label>
    <input type="checkbox" model={auto_save} />
    自动保存
</label>
```

### 多行文本

```rust
#[derive(IModel)]
#[component]
pub struct NoteEditor {
    pub content: SharedString,
}
```

```html
<textarea model={content} placeholder="请输入内容..." rows="10"></textarea>
```

## 3.3.5 双向绑定的字段要求

被 `model` 绑定的字段必须满足：

1. **`pub` 可见性**：RML 生成的命令需要访问该字段
2. **可赋值类型**：字段类型必须能从输入值赋值
3. **实现 `Default`**：通常需要默认值初始化

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub user_name: SharedString,  // ✅ pub，可赋值
    pub age: i32,                 // ✅ pub，可赋值
    pub remember_me: bool,        // ✅ pub，可赋值

    // ❌ 不满足要求的字段
    private_field: i32,           // 非 pub
    pub readonly_data: SharedString,  // 逻辑上不应被用户修改
}
```

## 3.3.6 双向绑定与事件处理的协作

`model` 处理数据同步，但你可能还需要在输入时执行额外逻辑。这时可以同时绑定 `oninput` 或 `onchange` 事件：

```html
<input
    model={search_text}
    oninput={on_search_input}
    placeholder="搜索..."
/>
```

```rust
#[command]
pub fn on_search_input(&mut self, _: &InputEvent, cx: &mut Context<Self>) {
    // model 已经更新了 search_text，这里可以执行额外逻辑
    self.perform_search(cx);
}
```

⚠️ **注意**：`model` 的事件处理在 `oninput` 之前执行，确保命令方法能读到最新值。

## 3.3.7 双向绑定与命令的对比

| 场景          | 推荐方式              | 原因                  |
| ----------- | ----------------- | ------------------- |
| 简单字段同步      | `model={field}`   | 自动同步，无需手写命令         |
| 输入时需要执行逻辑   | `model` + `oninput` | 同步 + 逻辑             |
| 输入值需要转换     | `value={}` + `oninput` + 命令 | 手动控制数据流             |
| 输入值需要验证     | `model` + `onblur` 验证 | 同步 + 失焦验证           |

### 示例：输入验证

```html
<input
    model={email}
    onblur={validate_email}
    placeholder="邮箱"
/>
<span if={email_error.is_some()} class="error">
    {email_error}
</span>
```

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub email: SharedString,
    pub email_error: Option<SharedString>,
}

impl MyView {
    #[command]
    pub fn validate_email(&mut self, _: &FocusEvent, cx: &mut Context<Self>) {
        if self.email.is_empty() || !self.email.contains('@') {
            self.email_error = Some("请输入有效的邮箱地址".into());
        } else {
            self.email_error = None;
        }
        cx.notify();
    }
}
```

## 3.3.8 双向绑定的特殊场景

### 自定义组件的双向绑定

自定义组件也可以支持 `model`，需要在组件中声明可绑定的属性：

```rust
#[derive(IModel)]
#[component]
pub struct Slider {
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub on_change: Option<Arc<dyn Fn(&ChangeEvent)>>,
}
```

```html
<Slider model={volume} min="0" max="100" />
```

详见 [第 6 章 · 组件通信](../06-components/component-communication.md)。

### 嵌套字段的双向绑定

`model` 不支持嵌套字段，需要通过命令手动处理：

```html
<!-- ❌ 不支持嵌套字段 -->
<input model={user.profile.name} />

<!-- ✅ 用命令手动同步 -->
<input value={user.profile.name} oninput={update_user_name} />
```

```rust
#[command]
pub fn update_user_name(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
    self.user.profile.name = ev.value.clone();
    cx.notify();
}
```

## 3.3.9 双向绑定的性能

双向绑定比单向绑定多一个事件订阅：

- 单向绑定：1 个订阅（字段 → UI）
- 双向绑定：2 个订阅（字段 → UI + UI → 字段）

但实际开销很小，因为：

- 事件订阅是轻量的
- 只有用户实际输入时才触发反向同步
- `cx.notify()` 会批量合并

## 3.3.10 双向绑定的常见陷阱

### 陷阱一：忘记 `pub`

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    user_name: SharedString,  // ❌ 非 pub，model 无法访问
}
```

### 陷阱二：在命令中重复修改字段

```rust
#[command]
pub fn on_input(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
    // model 已经更新了 user_name，这里又改一次会导致冲突
    self.user_name = ev.value.to_uppercase().into();
    cx.notify();
}
```

如果需要在输入时转换值，应该用 `value={}` + `oninput` + 命令的方式，而不是 `model`。

### 陷阱三：在循环中使用 `model`

```html
<!-- ❌ 列表项的 model 会冲突 -->
<li each={user in users}>
    <input model={user.name} />  <!-- user 是只读引用 -->
</li>
```

列表项的 `model` 需要特殊处理，通常通过事件参数传递索引：

```html
<li each={index, user in users} key={user.id}>
    <input
        value={user.name}
        oninput={update_user_name, {index}}
    />
</li>
```

```rust
#[command]
pub fn update_user_name(&mut self, index: usize, ev: &InputEvent, cx: &mut Context<Self>) {
    self.users[index].name = ev.value.clone();
    cx.notify();
}
```

## 3.3.11 小结

双向绑定是表单输入的核心机制：

- **语法**：`model={field}`
- **数据流**：ViewModel ↔ View
- **适用标签**：`<input>`、`<textarea>`
- **字段要求**：`pub`、可赋值
- **协作**：可与 `oninput`、`onblur` 等事件配合

记住：`model` 是"自动同步"的语法糖，等价于 `value={field}` + `oninput={update_field}`。需要更细粒度控制时，回退到手动方式。

下一节 → [3.4 计算属性](./computed.md)
