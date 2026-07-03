# 4.3 元素引用

> **本节目标**：掌握 `ref` 属性与 `ElementRef<T>` 的命令式访问能力，在需要直接操作 DOM 元素时游刃有余。

## 4.3.1 元素引用的定义

元素引用（Element Reference）让 ViewModel 可以直接访问 `.rml` 中的特定元素，进行命令式操作。

典型场景：

- 自动聚焦输入框
- 编程式触发按钮点击
- 读取输入框的当前值
- 控制元素的禁用/启用状态

```html
<!-- .rml 中用 ref 命名元素 -->
<input ref="username_input" model={user_name} />
<button ref="submit_btn" onclick={submit}>提交</button>
```

```rust
// .rml.rs 中用 ElementRef 访问
#[derive(IModel)]
#[component]
pub struct MyView {
    pub user_name: SharedString,

    #[element]
    pub username_input: ElementRef<Input>,

    #[element]
    pub submit_btn: ElementRef<Button>,
}

impl MyView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 自动聚焦输入框
        self.username_input.focus(cx);
    }
}
```

## 4.3.2 `ref` 属性

在 `.rml` 中用 `ref="name"` 为元素命名：

```html
<input ref="username_input" />
<div ref="container">...</div>
<button ref="submit_btn">提交</button>
```

### 命名约定

- 使用 `snake_case`
- 名称应描述元素的用途，而非类型
- ✅ `username_input`、`submit_btn`、`container`
- ❌ `input1`、`button`、`div`

### ref 的唯一性

同一视图内，每个 `ref` 名称必须唯一：

```html
<!-- ❌ 重复的 ref 名 -->
<input ref="input" />
<input ref="input" />
```

## 4.3.3 `ElementRef<T>` 类型

`ElementRef<T>` 是 RML 提供的元素引用类型，`T` 是被引用元素的类型：

```rust
use rml::prelude::*;

#[derive(IModel)]
#[component]
pub struct MyView {
    #[element]
    pub username_input: ElementRef<Input>,      // 引用 Input 组件

    #[element]
    pub submit_btn: ElementRef<Button>,         // 引用 Button 组件

    #[element]
    pub container: ElementRef<Div>,             // 引用 div 元素
}
```

### 支持的元素类型

| 元素类型          | 对应 `.rml` 标签           |
| ------------- | ---------------------- |
| `Div`         | `<div>`                |
| `Input`       | `<input>`              |
| `TextArea`    | `<textarea>`           |
| `Button`      | `<button>`             |
| `Label`       | `<label>`、`<p>`、`<h1>` |
| `Image`       | `<img>`                |
| `Checkbox`    | `<input type="checkbox">` |

## 4.3.4 `#[element]` 属性

`#[element]` 标记字段为元素引用，让 RML 编译器自动关联 `ref` 名称与字段名：

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    #[element]
    pub username_input: ElementRef<Input>,
    // 字段名 "username_input" 自动关联 .rml 中的 ref="username_input"
}
```

### 字段名与 ref 名的对应

默认情况下，字段名与 `ref` 名相同：

```rust
// 字段名 username_input ↔ ref="username_input"
#[element]
pub username_input: ElementRef<Input>,
```

```html
<input ref="username_input" />
```

### 显式指定 ref 名

如果需要字段名与 ref 名不同，可以显式指定：

```rust
#[element(ref = "user_input")]
pub username_input: ElementRef<Input>,
```

```html
<input ref="user_input" />
```

## 4.3.5 ElementRef 的初始化

`ElementRef` 实现了 `Default`，可以在构造函数中用 `Default::default()` 初始化：

```rust
impl MyView {
    pub fn new() -> Self {
        Self {
            user_name: SharedString::default(),
            username_input: ElementRef::default(),  // 初始为空
            submit_btn: ElementRef::default(),
        }
    }
}
```

视图加载时，RML Runtime 会自动填充 `ElementRef`，关联到实际的 UI 元素。

## 4.3.6 ElementRef 的常用方法

### `focus()`

让元素获得焦点：

```rust
self.username_input.focus(cx);
```

### `blur()`

让元素失去焦点：

```rust
self.username_input.blur(cx);
```

### `set_disabled()`

设置元素的禁用状态：

```rust
self.submit_btn.set_disabled(true, cx);
```

### `set_value()`

直接设置元素的值（绕过双向绑定）：

```rust
self.username_input.set_value("预设值".into(), cx);
```

### `value()`

读取元素的当前值：

```rust
let current_value = self.username_input.value();
```

### `is_visible() / set_visible()`

控制元素的可见性：

```rust
if !self.submit_btn.is_visible() {
    self.submit_btn.set_visible(true, cx);
}
```

## 4.3.7 使用场景

### 场景一：自动聚焦

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    // 视图加载后自动聚焦用户名输入框
    self.username_input.focus(cx);
}
```

### 场景二：编程式提交

```html
<input ref="search_input" model={search_text} onkeydown={on_key_down} />
```

```rust
#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    if ev.key == Key::Enter {
        // 回车键触发搜索
        self.perform_search(cx);
    }
}

fn perform_search(&mut self, cx: &mut ViewContext<Self>) {
    // ... 搜索逻辑
    // 搜索完成后清空输入框
    self.search_input.set_value(SharedString::default(), cx);
}
```

### 场景三：条件禁用

```rust
#[command]
pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    if self.user_name.is_empty() {
        return;
    }

    // 禁用提交按钮，防止重复提交
    self.submit_btn.set_disabled(true, cx);
    cx.notify();

    // 异步提交
    cx.spawn(|this, mut cx| async move {
        let result = submit_to_server().await;
        let _ = this.update(&mut cx, |this, cx| {
            this.submit_btn.set_disabled(false, cx);
            this.handle_submit_result(result, cx);
        });
    }).detach();
}
```

### 场景四：滚动控制

```html
<div ref="message_list" class="message-list">
    <div each={msg in messages} key={msg.id}>{msg.text}</div>
</div>
```

```rust
#[command]
pub fn send_message(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // ... 添加新消息

    // 滚动到底部
    self.message_list.scroll_to_bottom(cx);
}
```

## 4.3.8 ElementRef 的生命周期

`ElementRef` 的生命周期与视图绑定：

```
1. ViewModel::new() → ElementRef::default()（空引用）
2. 视图首次渲染 → RML Runtime 填充 ElementRef
3. 用户交互 → 通过 ElementRef 操作元素
4. 视图卸载 → ElementRef 自动失效
```

⚠️ **注意**：在 `#[on_loaded]` 之前，`ElementRef` 是空的，不能调用其方法。`#[on_loaded]` 是最早可以安全使用 `ElementRef` 的时机。

## 4.3.9 ElementRef vs 数据绑定

| 特性     | ElementRef         | 数据绑定             |
| ------ | ------------------ | ---------------- |
| 范式     | 命令式                | 声明式              |
| 适用场景   | 编程式操作（聚焦、滚动、禁用）    | 数据显示与同步          |
| 性能     | 直接调用，无订阅开销         | 订阅 + diff        |
| 可读性    | 逻辑分散在命令方法中         | 集中在 `.rml` 中     |
| 维护性    | 较低，需追踪命令调用         | 较高，一目了然          |

### 选择建议

- **优先用数据绑定**：能用 `{field}` 或 `model={field}` 解决的，不要用 `ElementRef`
- **命令式操作用 ElementRef**：聚焦、滚动、编程式触发等场景
- **避免滥用**：不要用 `ElementRef` 替代数据绑定

```rust
// ❌ 滥用 ElementRef
self.username_input.set_value(self.user_name.clone(), cx);

// ✅ 用数据绑定
// .rml: <input value={user_name} />
```

## 4.3.10 完整示例：登录表单

```rust
// views/login.rml.rs
use rml::prelude::*;

#[derive(IModel)]
#[component]
pub struct LoginView {
    pub username: SharedString,
    pub password: SharedString,
    pub error_message: Option<SharedString>,
    pub is_submitting: bool,

    #[element]
    pub username_input: ElementRef<Input>,

    #[element]
    pub password_input: ElementRef<Input>,

    #[element]
    pub submit_btn: ElementRef<Button>,
}

impl LoginView {
    pub fn new() -> Self {
        Self {
            username: SharedString::default(),
            password: SharedString::default(),
            error_message: None,
            is_submitting: false,
            username_input: ElementRef::default(),
            password_input: ElementRef::default(),
            submit_btn: ElementRef::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 自动聚焦用户名输入框
        self.username_input.focus(cx);
    }

    #[computed]
    pub fn can_submit(&self) -> bool {
        !self.username.is_empty()
            && !self.password.is_empty()
            && !self.is_submitting
    }

    #[command]
    pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if !self.can_submit() {
            return;
        }

        self.is_submitting = true;
        self.error_message = None;
        self.submit_btn.set_disabled(true, cx);
        cx.notify();

        let username = self.username.clone();
        let password = self.password.clone();

        cx.spawn(|this, mut cx| async move {
            let result = authenticate(&username, &password).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.is_submitting = false;
                this.submit_btn.set_disabled(false, cx);

                match result {
                    Ok(_) => {
                        // 登录成功，跳转...
                    }
                    Err(e) => {
                        this.error_message = Some(e.to_string().into());
                        // 密码错误时聚焦密码框
                        this.password_input.focus(cx);
                        this.password_input.select_all(cx);
                    }
                }
                cx.notify();
            });
        }).detach();
    }

    #[command]
    pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        if ev.key == Key::Enter {
            if self.username.is_empty() {
                self.username_input.focus(cx);
            } else if self.password.is_empty() {
                self.password_input.focus(cx);
            } else {
                self.submit(&ClickEvent::default(), cx);
            }
        }
    }
}

async fn authenticate(username: &str, password: &str) -> Result<(), AuthError> {
    // 模拟异步认证
    Ok(())
}
```

```html
<!-- views/login.rml -->
<div class="login-form">
    <h1>登录</h1>

    <input
        ref="username_input"
        model={username}
        placeholder="用户名"
        onkeydown={on_key_down}
    />

    <input
        ref="password_input"
        type="password"
        model={password}
        placeholder="密码"
        onkeydown={on_key_down}
    />

    <p if={error_message.is_some()} class="error">{error_message}</p>

    <button
        ref="submit_btn"
        onclick={submit}
        disabled={!can_submit}
    >
        {if is_submitting { "登录中..." } else { "登录" }}
    </button>
</div>
```

## 4.3.11 小结

元素引用是 RML 的命令式逃生舱：

- **`ref` 属性**：在 `.rml` 中为元素命名
- **`ElementRef<T>`**：在 ViewModel 中持有元素引用
- **`#[element]`**：标记字段为元素引用
- **常用方法**：`focus()`、`blur()`、`set_disabled()`、`set_value()`

记住：**优先用数据绑定，命令式操作才用 ElementRef**。滥用 ElementRef 会破坏 RML 的声明式优势。

下一节 → [4.4 命令系统](./command-system.md)
