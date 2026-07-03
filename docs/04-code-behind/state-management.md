# 4.5 状态管理

> **本节目标**：完整掌握 RML 的状态管理——`cx.notify()` 的触发机制、Entity 模型、跨视图状态共享、异步状态更新。

## 4.5.1 状态的本质

在 RML 中，状态就是 ViewModel 的字段。状态管理就是：

1. **修改状态**：在命令方法中修改字段值
2. **通知变化**：调用 `cx.notify()` 告诉 GPUI 状态已变化
3. **触发重绘**：GPUI 重新调用 `Render::render`，UI 读取新值

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;       // 1. 修改状态
    cx.notify();           // 2. 通知变化 → 3. 触发重绘
}
```

## 4.5.2 `cx.notify()` 详解

### 通知的本质

`cx.notify()` 告诉 GPUI："这个 Entity 的状态变了，下次渲染时请重新调用 `render`"。

```
cx.notify()
    ↓
GPUI 标记 Entity 为 "dirty"
    ↓
下一帧时，GPUI 调用 Render::render
    ↓
RML 生成的代码重新读取所有绑定值
    ↓
GPUI diff 新旧渲染树，更新实际 UI
```

### 何时调用 `cx.notify()`

**规则**：每次修改 ViewModel 字段后，都必须调用 `cx.notify()`。

```rust
// ✅ 正确：修改后 notify
#[command]
pub fn update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}

// ❌ 错误：忘记 notify，UI 不会更新
#[command]
pub fn bad_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    // 缺少 cx.notify()
}
```

### 批量更新

多次 `cx.notify()` 会在同一帧内合并为一次重绘：

```rust
#[command]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.field1 = "new1".into();
    cx.notify();  // 不会立即重绘

    self.field2 = "new2".into();
    cx.notify();  // 不会立即重绘

    self.field3 = "new3".into();
    cx.notify();  // 函数返回后，下一帧统一重绘
}
```

💡 **最佳实践**：在命令方法末尾调用一次 `cx.notify()` 即可，无需每次修改字段都调用。

### 在异步任务中 notify

异步任务中修改状态必须通过 `this.update`：

```rust
cx.spawn(|this, mut cx| async move {
    let data = fetch_data().await;

    let _ = this.update(&mut cx, |this, cx| {
        this.data = data;
        cx.notify();  // 在 update 闭包内 notify
    });
}).detach();
```

⚠️ **注意**：不能在异步任务中直接捕获 `&mut self`，必须通过 `this.update` 获取 `&mut self`。详见 [第 8 章 · 状态生命周期](../08-lifecycle/state-lifecycle.md)。

## 4.5.3 Entity 模型

### Entity 是什么

ViewModel 通过 `#[derive(IModel)]` 成为 GPUI 管理的 Entity。Entity 是 GPUI 的状态管理单元：

- 每个 Entity 有唯一的 `EntityId`
- Entity 由 `App` 统一管理
- 通过 `Entity<T>` 句柄引用

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub count: i32,
}

// MyView 的实例是一个 Entity
// 通过 Entity<MyView> 句柄引用
```

### Entity 的创建

Entity 在 ViewModel 的 `new()` 方法中创建，由 `RmlApplication` 自动管理：

```rust
fn main() {
    RmlApplication::new()
        .main_window::<MyView>()  // 自动创建 MyView 的 Entity
        .run()
        .unwrap();
}
```

### Entity 的引用

通过 `Entity<T>` 句柄引用其他 ViewModel：

```rust
use gpui::Entity;

#[derive(IModel)]
#[component]
pub struct ParentView {
    pub child: Entity<ChildView>,
}
```

## 4.5.4 跨视图状态共享

### 方式一：父子引用

父视图持有子视图的 `Entity` 句柄：

```rust
#[derive(IModel)]
#[component]
pub struct ParentView {
    pub child: Entity<ChildView>,
    pub parent_count: i32,
}

impl ParentView {
    pub fn new(cx: &mut AppContext) -> Self {
        Self {
            child: cx.new_model(|_| ChildView::new()),
            parent_count: 0,
        }
    }

    #[command]
    pub fn increment_both(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.parent_count += 1;
        self.child.update(cx, |child, cx| {
            child.count += 1;
            cx.notify();
        });
        cx.notify();
    }
}
```

### 方式二：全局状态

用 GPUI 的 `Global` 机制共享全局状态：

```rust
use gpui::Global;

#[derive(Clone)]
pub struct AppTheme {
    pub primary_color: Hsla,
    pub background: Hsla,
}

impl Global for AppTheme {}

// 在 App 中注册全局状态
fn main() {
    RmlApplication::new()
        .with_global(AppTheme {
            primary_color: rgb(0x1890ff).into(),
            background: rgb(0xffffff).into(),
        })
        .main_window::<MyView>()
        .run()
        .unwrap();
}
```

```rust
// 在 ViewModel 中访问全局状态
#[derive(IModel)]
#[component]
pub struct MyView {
    pub theme: AppTheme,
}

impl MyView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.theme = cx.global::<AppTheme>().clone();

        // 订阅全局状态变化
        cx.observe_global::<AppTheme>(|this, cx| {
            this.theme = cx.global::<AppTheme>().clone();
            cx.notify();
        }).detach();
    }
}
```

### 方式三：上下文（Context）

用 GPUI 的 `Context` 机制在视图树中传递数据：

```rust
// 父视图设置上下文
#[command]
pub fn set_user_context(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    cx.set_global(UserContext {
        current_user: self.current_user.clone(),
    });
}

// 子视图读取上下文
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    if let Some(user_ctx) = cx.try_global::<UserContext>() {
        self.current_user = user_ctx.current_user.clone();
    }
}
```

## 4.5.5 状态更新的模式

### 模式一：同步更新

最简单的模式，直接修改字段并 notify：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}
```

### 模式二：异步更新

用于网络请求、文件 IO 等异步操作：

```rust
#[command]
pub fn load_data(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.is_loading = true;
    cx.notify();

    cx.spawn(|this, mut cx| async move {
        let data = fetch_data().await;

        let _ = this.update(&mut cx, |this, cx| {
            this.data = data;
            this.is_loading = false;
            cx.notify();
        });
    }).detach();
}
```

### 模式三：批量更新

一次性修改多个相关字段：

```rust
#[command]
pub fn refresh(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.is_loading = true;
    self.error = None;
    cx.notify();

    cx.spawn(|this, mut cx| async move {
        let result = fetch_all_data().await;

        let _ = this.update(&mut cx, |this, cx| {
            match result {
                Ok((users, posts, comments)) => {
                    this.users = users;
                    this.posts = posts;
                    this.comments = comments;
                    this.is_loading = false;
                    this.last_refresh = chrono::Local::now();
                }
                Err(e) => {
                    this.error = Some(e.to_string().into());
                    this.is_loading = false;
                }
            }
            cx.notify();
        });
    }).detach();
}
```

### 模式四：条件更新

根据条件决定是否更新：

```rust
#[command]
pub fn update_if_valid(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    if self.is_valid() {
        self.apply_changes();
        cx.notify();
    }
}
```

## 4.5.6 状态的持久化

### 保存到本地存储

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    if let Err(e) = self.save_to_storage() {
        log::error!("保存状态失败: {}", e);
    }
}

fn save_to_storage(&self) -> Result<(), StorageError> {
    let serialized = serde_json::to_string(&self.todos)?;
    std::fs::write("todos.json", serialized)?;
    Ok(())
}
```

### 从本地存储加载

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    if let Err(e) = self.load_from_storage() {
        log::warn!("加载状态失败: {}", e);
    }
    cx.notify();
}

fn load_from_storage(&mut self) -> Result<(), StorageError> {
    if let Ok(content) = std::fs::read_to_string("todos.json") {
        self.todos = serde_json::from_str(&content)?;
    }
    Ok(())
}
```

## 4.5.7 状态的订阅

### 订阅其他 Entity

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    // 订阅子视图的变化
    cx.observe(&self.child, |this, _child, cx| {
        // 子视图变化时更新父视图
        this.child_count = _child.count;
        cx.notify();
    }).detach();
}
```

### 订阅全局状态

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.observe_global::<AppTheme>(|this, cx| {
        this.theme = cx.global::<AppTheme>().clone();
        cx.notify();
    }).detach();
}
```

⚠️ **注意**：订阅返回的 `Subscription` 必须 `.detach()` 或在字段中保持，否则订阅会被立即取消。详见 [第 8 章 · 订阅与取消](../08-lifecycle/subscriptions.md)。

## 4.5.8 状态管理的最佳实践

### 1. 单一职责

每个 ViewModel 只管理一个视图的状态：

```rust
// ✅ 单一职责
#[derive(IModel)]
#[component]
pub struct UserListView {
    pub users: Vec<User>,
    pub selected_user: Option<u64>,
}

#[derive(IModel)]
#[component]
pub struct UserDetailView {
    pub user: User,
    pub is_editing: bool,
}

// ❌ 上帝 ViewModel
#[derive(IModel)]
#[component]
pub struct MegaView {
    pub users: Vec<User>,
    pub selected_user: Option<u64>,
    pub user_detail: User,
    pub is_editing: bool,
    pub posts: Vec<Post>,
    pub comments: Vec<Comment>,
    // ... 过多职责
}
```

### 2. 状态最小化

只保存必要的状态，能计算的用 `#[computed]`：

```rust
// ✅ 最小状态 + 计算属性
#[derive(IModel)]
#[component]
pub struct CartView {
    pub items: Vec<CartItem>,  // 唯一状态
}

impl CartView {
    #[computed]
    pub fn total_price(&self) -> f64 {
        self.items.iter().map(|i| i.price * i.quantity as f64).sum()
    }

    #[computed]
    pub fn total_count(&self) -> u32 {
        self.items.iter().map(|i| i.quantity).sum()
    }
}

// ❌ 冗余状态
#[derive(IModel)]
#[component]
pub struct BadCartView {
    pub items: Vec<CartItem>,
    pub total_price: f64,    // 冗余，可计算
    pub total_count: u32,    // 冗余，可计算
}
```

### 3. 不可变优先

尽量用不可变字段，通过命令方法修改：

```rust
// ✅ 字段 pub 但通过命令修改
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count += 1;
        cx.notify();
    }
}
```

### 4. 避免深度嵌套

```rust
// ❌ 深度嵌套，难以维护
#[derive(IModel)]
#[component]
pub struct DeepView {
    pub data: Outer,
}

pub struct Outer { pub middle: Middle }
pub struct Middle { pub inner: Inner }
pub struct Inner { pub value: i32 }

// .rml: {data.middle.inner.value}

// ✅ 扁平化结构
#[derive(IModel)]
#[component]
pub struct FlatView {
    pub value: i32,
}
```

## 4.5.9 状态管理反模式

### 反模式一：忘记 notify

```rust
// ❌ UI 不更新
#[command]
pub fn bad_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    // 忘记 cx.notify()
}
```

### 反模式二：在异步中直接修改 self

```rust
// ❌ 编译错误或运行时错误
#[command]
pub fn bad_async(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    cx.spawn(|this, mut cx| async move {
        // 不能直接捕获 &mut self
        self.count += 1;  // 编译错误
    }).detach();
}

// ✅ 通过 this.update
#[command]
pub fn good_async(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    cx.spawn(|this, mut cx| async move {
        let _ = this.update(&mut cx, |this, cx| {
            this.count += 1;
            cx.notify();
        });
    }).detach();
}
```

### 反模式三：过度使用全局状态

```rust
// ❌ 所有状态都放全局
#[derive(Global)]
pub struct GlobalState {
    pub user: User,
    pub cart: Vec<CartItem>,
    pub theme: AppTheme,
    pub settings: Settings,
    // ... 过多状态
}

// ✅ 全局只放真正共享的状态
#[derive(Global)]
pub struct AppTheme { ... }  // 只放主题

// 其他状态放在各自的 ViewModel 中
```

## 4.5.10 小结

RML 的状态管理核心：

- **状态即字段**：ViewModel 的 `pub` 字段就是状态
- **`cx.notify()`**：修改状态后必须调用，触发重绘
- **Entity 模型**：ViewModel 是 GPUI Entity，可被引用
- **跨视图共享**：父子引用、全局状态、上下文三种方式
- **异步更新**：通过 `this.update` 在异步任务中修改状态

最佳实践：单一职责、状态最小化、不可变优先、避免深度嵌套。

下一章 → [第 5 章 · 事件系统](../05-events/INDEX.md)
