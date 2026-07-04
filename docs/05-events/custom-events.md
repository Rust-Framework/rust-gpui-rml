# 5.4 自定义事件

> **本节目标**：掌握自定义组件如何声明和触发事件，让组件可被父视图监听和响应。

## 5.4.1 自定义事件的定义

自定义事件是组件向父视图通信的机制。当组件内部发生某些事情时（如用户操作、状态变化），可以通过自定义事件通知父视图。

```
组件内部触发事件 → 事件冒泡到父视图 → 父视图处理事件
```

## 5.4.2 声明自定义事件

在组件中用 `Option<Arc<dyn Fn(...)>>` 字段声明事件回调：

```rust
#[derive(IModel)]
#[component]
pub struct SearchBox {
    pub query: SharedString,
    pub on_search: Option<Arc<dyn Fn(&SearchEvent)>>,
    pub on_clear: Option<Arc<dyn Fn(&ClickEvent)>>,
}
```

### 事件回调的签名

```rust
// 无参数事件
Option<Arc<dyn Fn()>>

// 带事件对象的事件
Option<Arc<dyn Fn(&MyEvent)>>

// 带多个参数的事件
Option<Arc<dyn Fn(SharedString, &MyEvent)>>
```

## 5.4.3 定义事件对象

自定义事件可以携带自定义的事件对象：

```rust
#[derive(Clone)]
pub struct SearchEvent {
    pub query: SharedString,
    pub timestamp: chrono::DateTime<chrono::Local>,
}
```

### 事件对象的设计

| 字段类型          | 用途              |
| ------------- | --------------- |
| `SharedString` | 文本数据            |
| `u64`、`i32` 等 | 数字数据            |
| `bool`        | 布尔标志            |
| 自定义枚举          | 状态、类型           |
| 时间戳            | 事件发生时间          |

## 5.4.4 触发事件

在组件内部通过回调触发事件：

```rust
impl SearchBox {
    pub fn new() -> Self {
        Self {
            query: SharedString::default(),
            on_search: None,
            on_clear: None,
        }
    }

    #[command]
    pub fn perform_search(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if let Some(callback) = &self.on_search {
            callback(&SearchEvent {
                query: self.query.clone(),
                timestamp: chrono::Local::now(),
            });
        }
    }

    #[command]
    pub fn clear(&mut self, ev: &ClickEvent, cx: &mut Context<Self>) {
        self.query = SharedString::default();
        cx.notify();

        if let Some(callback) = &self.on_clear {
            callback(ev);
        }
    }
}
```

## 5.4.5 在父视图中监听事件

父视图通过 `on_*` 属性监听子组件的事件：

```html
<div class="search-panel">
    <SearchBox
        on_search={handle_search}
        on_clear={handle_clear}
    />
    <p if={search_results.is_empty()}>暂无结果</p>
    <ul>
        <li each={result in search_results} key={result.id}>
            {result.title}
        </li>
    </ul>
</div>
```

```rust
#[derive(IModel)]
#[component]
pub struct SearchPanel {
    pub search_results: Vec<SearchResult>,
}

impl SearchPanel {
    #[command]
    pub fn handle_search(&mut self, ev: &SearchEvent, cx: &mut Context<Self>) {
        // ev.query 是搜索框的输入值
        self.perform_search(&ev.query, cx);
    }

    #[command]
    pub fn handle_clear(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.search_results.clear();
        cx.notify();
    }

    fn perform_search(&mut self, query: &str, cx: &mut Context<Self>) {
        // 执行搜索...
        cx.notify();
    }
}
```

## 5.4.6 事件命名约定

自定义事件的命名遵循 `on_*` 前缀：

| 事件字段          | 在 `.rml` 中的属性   | 触发时机         |
| ------------- | ---------------- | ------------ |
| `on_search`   | `on_search={...}` | 执行搜索         |
| `on_clear`    | `on_clear={...}`  | 清空内容         |
| `on_change`   | `on_change={...}` | 值变化          |
| `on_select`   | `on_select={...}` | 选择项          |
| `on_submit`   | `on_submit={...}` | 提交           |

### 命名规则

- 字段名用 `on_` 前缀 + 动词
- 用 `snake_case`
- 描述事件的发生时机，而非处理方式

```rust
// ✅ 好的命名
pub on_search: Option<Arc<dyn Fn(&SearchEvent)>>,
pub on_item_selected: Option<Arc<dyn Fn(u64)>>,

// ❌ 不好的命名
pub callback: Option<Arc<dyn Fn(&SearchEvent)>>,  // 太泛化
pub handle_search: Option<Arc<dyn Fn(&SearchEvent)>>,  // handle_ 前缀冗余
```

## 5.4.7 事件的参数传递

### 无参数事件

```rust
pub on_close: Option<Arc<dyn Fn()>>,
```

```html
<Dialog on_close={handle_close} />
```

```rust
#[command]
pub fn handle_close(&mut self, _: &CloseEvent, cx: &mut Context<Self>) {
    self.is_dialog_open = false;
    cx.notify();
}
```

### 带参数事件

```rust
pub on_select: Option<Arc<dyn Fn(u64, &SelectEvent)>>,
```

```html
<Dropdown on_select={handle_select} />
```

```rust
#[command]
pub fn handle_select(&mut self, id: u64, ev: &SelectEvent, cx: &mut Context<Self>) {
    self.selected_id = Some(id);
    cx.notify();
}
```

### 带事件对象的事件

```rust
pub on_change: Option<Arc<dyn Fn(&ChangeEvent)>>,
```

```html
<Input on_change={handle_change} />
```

## 5.4.8 事件的冒泡

自定义事件默认会冒泡，可以通过 `stop_propagation()` 阻止：

```html
<div on-click={on_outer_click}>
    <SearchBox on_search={on_search} />
</div>
```

```rust
// SearchBox 内部
#[command]
pub fn perform_search(&mut self, ev: &ClickEvent, cx: &mut Context<Self>) {
    // 触发 on_search 事件
    if let Some(callback) = &self.on_search {
        callback(&SearchEvent { /* ... */ });
    }

    // 阻止冒泡，不触发外部 div 的 on-click
    ev.stop_propagation();
}
```

## 5.4.9 完整示例：自定义对话框组件

```rust
// components/dialog.rml.rs
use rml::prelude::*;

#[derive(Clone)]
pub struct DialogCloseEvent {
    pub reason: CloseReason,
}

#[derive(Clone)]
pub enum CloseReason {
    UserClosed,
    ConfirmClicked,
    CancelClicked,
}

#[derive(IModel)]
#[component]
pub struct Dialog {
    pub title: SharedString,
    pub is_open: bool,
    pub on_close: Option<Arc<dyn Fn(&DialogCloseEvent)>>,
    pub on_confirm: Option<Arc<dyn Fn(&ClickEvent)>>,
    pub on_cancel: Option<Arc<dyn Fn(&ClickEvent)>>,
}

impl Dialog {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            is_open: false,
            on_close: None,
            on_confirm: None,
            on_cancel: None,
        }
    }

    pub fn open(&mut self, cx: &mut Context<Self>) {
        self.is_open = true;
        cx.notify();
    }

    pub fn close(&mut self, reason: CloseReason, cx: &mut Context<Self>) {
        self.is_open = false;
        cx.notify();

        if let Some(callback) = &self.on_close {
            callback(&DialogCloseEvent { reason });
        }
    }

    #[command]
    pub fn on_confirm_click(&mut self, ev: &ClickEvent, cx: &mut Context<Self>) {
        if let Some(callback) = &self.on_confirm {
            callback(ev);
        }
        self.close(CloseReason::ConfirmClicked, cx);
    }

    #[command]
    pub fn on_cancel_click(&mut self, ev: &ClickEvent, cx: &mut Context<Self>) {
        if let Some(callback) = &self.on_cancel {
            callback(ev);
        }
        self.close(CloseReason::CancelClicked, cx);
    }

    #[command]
    pub fn on_overlay_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.close(CloseReason::UserClosed, cx);
    }

    #[command]
    pub fn on_content_click(&mut self, ev: &ClickEvent, _cx: &mut Context<Self>) {
        ev.stop_propagation();  // 阻止冒泡，不关闭对话框
    }
}
```

```html
<!-- components/dialog.rml -->
<div if={is_open} class="dialog-overlay" on-click={on_overlay_click}>
    <div class="dialog-content" on-click={on_content_click}>
        <h2 class="dialog-title">{title}</h2>
        <div class="dialog-body">
            <slot></slot>
        </div>
        <div class="dialog-footer">
            <button on-click={on_cancel_click}>取消</button>
            <button on-click={on_confirm_click}>确认</button>
        </div>
    </div>
</div>
```

### 使用对话框

```html
<!-- views/user_view.rml -->
<div>
    <button on-click={show_delete_dialog}>删除用户</button>

    <Dialog
        title="确认删除"
        on_close={handle_dialog_close}
        on_confirm={handle_delete_confirm}
        on_cancel={handle_delete_cancel}
    >
        <p>确定要删除用户 {user_name} 吗？此操作不可撤销。</p>
    </Dialog>
</div>
```

```rust
// views/user_view.rml.rs
#[derive(IModel)]
#[component]
pub struct UserView {
    pub user_name: SharedString,
    pub delete_dialog: Entity<Dialog>,
}

impl UserView {
    pub fn new(cx: &mut AppContext) -> Self {
        Self {
            user_name: "张三".into(),
            delete_dialog: cx.new_model(|_| Dialog::new("确认删除")),
        }
    }

    #[command]
    pub fn show_delete_dialog(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.delete_dialog.update(cx, |dialog, cx| dialog.open(cx));
    }

    #[command]
    pub fn handle_delete_confirm(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        // 执行删除
        self.delete_user(cx);
    }

    #[command]
    pub fn handle_delete_cancel(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        // 取消，无需额外处理
    }

    #[command]
    pub fn handle_dialog_close(&mut self, ev: &DialogCloseEvent, cx: &mut Context<Self>) {
        match ev.reason {
            CloseReason::ConfirmClicked => {
                // 已在 handle_delete_confirm 处理
            }
            CloseReason::CancelClicked => {
                // 用户取消
            }
            CloseReason::UserClosed => {
                // 用户点击遮罩关闭
            }
        }
    }

    fn delete_user(&mut self, cx: &mut Context<Self>) {
        // 删除逻辑...
        cx.notify();
    }
}
```

## 5.4.10 自定义事件的注意事项

### 1. 事件回调是 `Option`

事件回调字段必须是 `Option`，因为父视图可能不监听：

```rust
// ✅ Option 类型
pub on_search: Option<Arc<dyn Fn(&SearchEvent)>>,

// ❌ 非 Option，父视图必须监听
pub on_search: Arc<dyn Fn(&SearchEvent)>,
```

### 2. 触发前检查 `Some`

```rust
// ✅ 检查后再调用
if let Some(callback) = &self.on_search {
    callback(&event);
}

// ❌ 直接调用，可能 panic
(self.on_search.as_ref().unwrap())(&event);
```

### 3. 事件对象实现 `Clone`

事件对象需要实现 `Clone`，因为可能被多次传递：

```rust
#[derive(Clone)]  // ← 必须
pub struct SearchEvent {
    pub query: SharedString,
}
```

### 4. 避免事件循环

```rust
// ❌ 事件循环：A 触发 B，B 触发 A
impl ComponentA {
    fn on_event(&mut self) {
        self.trigger_b();  // 触发 B
    }
}

impl ComponentB {
    fn on_event(&mut self) {
        self.trigger_a();  // 触发 A → 死循环
    }
}
```

## 5.4.11 小结

自定义事件是组件通信的核心机制：

- **声明**：用 `Option<Arc<dyn Fn(...)>>` 字段
- **触发**：在组件内部检查 `Some` 后调用回调
- **监听**：父视图用 `on_*` 属性绑定命令
- **冒泡**：自定义事件默认冒泡，可 `stop_propagation`
- **命名**：`on_` 前缀 + 动词，`snake_case`

掌握自定义事件，你就能设计出可复用、可组合的组件。

下一节 → [5.5 防抖与节流](./debounce-throttle.md)
