# 4.1 ViewModel 结构

> **本节目标**：掌握 ViewModel 的标准结构——Model 派生、字段约定、构造函数、与 `.rml` 的关联。

## 4.1.1 ViewModel 的定义

ViewModel 是 `.rml.rs` 文件中的核心结构体，承担以下职责：

- 持有 UI 状态（响应式字段）
- 暴露命令方法（UI 可调用）
- 提供计算属性（派生值）
- 响应生命周期事件

```rust
// counter.rml.rs
use rml::prelude::*;

#[derive(IModel)]    // 1. 成为 GPUI Entity
#[component]             // 2. 标记为 RML 视图
pub struct Counter {
    pub count: i32,  // 3. 响应式状态
}

impl Counter {
    pub fn new() -> Self {  // 4. 构造函数
        Self { count: 0 }
    }

    #[command]  // 5. 命令方法
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}
```

## 4.1.2 `#[derive(IModel)]`

`#[derive(IModel)]` 让结构体成为 GPUI 管理的 Entity：

```rust
#[derive(IModel)]
pub struct Counter {
    pub count: i32,
}
```

派生后，结构体获得以下能力：

- **响应式状态**：字段变化时可通过 `cx.notify()` 触发 UI 更新
- **GPUI Entity**：可通过 `Entity<T>` 句柄被其他视图引用
- **ViewContext**：在命令方法中接收 `cx: &mut Context<Self>`

### Model 派生的要求

- 所有字段必须实现 `Default` 或在构造函数中初始化
- 字段类型必须是 `Send + 'static`（GPUI 的线程安全要求）

```rust
// ✅ 满足要求
#[derive(IModel)]
pub struct MyView {
    pub name: SharedString,  // SharedString: Send + 'static
    pub count: i32,          // i32: Send + 'static
    pub items: Vec<Item>,    // Vec<Item>: Send + 'static (if Item: Send)
}

// ❌ 不满足要求
#[derive(IModel)]
pub struct BadView {
    pub callback: Rc<dyn Fn()>,  // Rc 不是 Send
    pub borrowed: &'static str,  // 引用需要 'static
}
```

## 4.1.3 `#[component]` 属性

`#[component]` 标记结构体为 RML 视图，告诉编译器：

1. 这个 ViewModel 关联一个 `.rml` 文件
2. 编译器应为它生成 `Render` trait 的实现

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
}
```

### 文件关联规则

`#[component]` 默认按命名约定关联 `.rml` 文件：

```
src/views/counter.rml       ← UI 标记
src/views/counter.rml.rs    ← ViewModel（Counter 结构体）
```

文件名（不含扩展名）必须匹配。

### 显式指定模板

如果需要偏离命名约定，可以显式指定模板路径：

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
}
```

## 4.1.4 字段约定

### `pub` 字段：可绑定

UI 可以通过 `{field}` 绑定访问的字段：

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub user_name: SharedString,  // UI 可绑定 {user_name}
    pub count: i32,               // UI 可绑定 {count}
}
```

```html
<p>{user_name}</p>
<p>{count}</p>
```

### `private` 字段：内部状态

不需要在 UI 中暴露的字段可以保持 private：

```rust
#[derive(IModel)]
#[component]
pub struct TodoViewModel {
    pub todos: Vec<TodoItem>,  // pub：UI 需要遍历
    pub new_todo_text: SharedString,  // pub：UI 需要双向绑定

    next_id: u64,              // private：仅内部使用
    is_loading: bool,          // private：通过计算属性暴露
}
```

### 字段类型建议

| 用途          | 推荐类型                | 原因                  |
| ----------- | ------------------- | ------------------- |
| 文本          | `SharedString`      | GPUI 的字符串类型，零拷贝     |
| 数字          | `i32`、`u32`、`f64`   | 原生类型，性能好            |
| 布尔          | `bool`              | 原生类型                |
| 列表          | `Vec<T>`            | 标准集合                |
| 可空值         | `Option<T>`         | 标准可空类型              |
| 复杂数据        | 自定义 `Model` 结构体      | 嵌套数据结构              |
| 回调          | `Option<Arc<dyn Fn(...)>>` | 事件回调                |

## 4.1.5 构造函数

每个 ViewModel 应提供 `new()` 方法作为构造函数：

```rust
impl Counter {
    pub fn new() -> Self {
        Self {
            count: 0,
        }
    }
}
```

### 默认值初始化

```rust
impl MyView {
    pub fn new() -> Self {
        Self {
            user_name: SharedString::default(),
            count: 0,
            items: Vec::new(),
            is_loading: false,
            next_id: 1,
        }
    }
}
```

### 带参数的构造函数

```rust
impl UserView {
    pub fn new(user: User) -> Self {
        Self {
            user,
            is_editing: false,
        }
    }
}
```

### 在 `main.rs` 中启动

```rust
fn main() {
    RmlApplication::new()
        .main_window::<views::counter::Counter>()
        .run()
        .unwrap();
}
```

`RmlApplication` 会自动调用 `Counter::new()` 创建初始视图。

## 4.1.6 ViewModel 与 Model 的区别

RML 借鉴 WPF 的 MVVM 模式，区分 Model 和 ViewModel：

### Model：纯数据

```rust
#[derive(IModel)]
pub struct TodoItem {
    pub id: u64,
    pub text: SharedString,
    pub done: bool,
}
```

- 不标注 `#[component]`
- 不含命令方法
- 不含计算属性
- 可被多个 ViewModel 共享

### ViewModel：视图状态

```rust
#[derive(IModel)]
#[component]
pub struct TodoViewModel {
    pub todos: Vec<TodoItem>,
    pub new_todo_text: SharedString,
}

impl TodoViewModel {
    #[command]
    pub fn add_todo(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        // ...
    }
}
```

- 标注 `#[component]`
- 包含命令方法
- 包含计算属性
- 与特定 `.rml` 文件关联

## 4.1.7 嵌套 Model

ViewModel 可以包含 Model 作为字段：

```rust
#[derive(IModel)]
pub struct User {
    pub name: SharedString,
    pub email: SharedString,
    pub profile: UserProfile,
}

#[derive(IModel)]
pub struct UserProfile {
    pub avatar: SharedString,
    pub bio: SharedString,
}

#[derive(IModel)]
#[component]
pub struct UserView {
    pub user: User,  // 嵌套 Model
    pub is_editing: bool,
}
```

```html
<div>
    <p>姓名: {user.name}</p>
    <p>邮箱: {user.email}</p>
    <p>头像: <img src={user.profile.avatar} /></p>
    <p>简介: {user.profile.bio}</p>
</div>
```

## 4.1.8 ViewModel 的生命周期

```
1. RmlApplication::main_window::<MyView>().run()
   ↓
2. MyView::new() 被调用，创建 ViewModel 实例
   ↓
3. GPUI 创建 Entity，注册到 App
   ↓
4. 视图首次渲染，调用 Render::render
   ↓
5. #[on_loaded] 回调被调用
   ↓
6. 用户交互，命令方法被调用，cx.notify() 触发重绘
   ↓
7. 视图卸载，#[on_unloaded] 回调被调用
   ↓
8. Entity 被销毁，资源释放
```

详见 [第 8 章 · 视图生命周期](../08-lifecycle/view-lifecycle.md)。

## 4.1.9 完整示例

```rust
// views/user_list.rml.rs
use rml::prelude::*;

#[derive(IModel)]
pub struct User {
    pub id: u64,
    pub name: SharedString,
    pub email: SharedString,
    pub is_active: bool,
}

#[derive(IModel)]
#[component]
pub struct UserListViewModel {
    pub users: Vec<User>,
    pub search_text: SharedString,
    pub selected_user_id: Option<u64>,

    next_id: u64,
    is_loading: bool,
}

impl UserListViewModel {
    pub fn new() -> Self {
        Self {
            users: Vec::new(),
            search_text: SharedString::default(),
            selected_user_id: None,
            next_id: 1,
            is_loading: false,
        }
    }

    #[computed]
    pub fn filtered_users(&self) -> Vec<&User> {
        if self.search_text.is_empty() {
            self.users.iter().collect()
        } else {
            self.users
                .iter()
                .filter(|u| u.name.contains(self.search_text.as_ref()))
                .collect()
        }
    }

    #[computed]
    pub fn active_count(&self) -> usize {
        self.users.iter().filter(|u| u.is_active).count()
    }

    #[command]
    pub fn add_user(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_loading = true;
        cx.notify();

        // 模拟异步加载
        let new_user = User {
            id: self.next_id,
            name: format!("用户 {}", self.next_id).into(),
            email: format!("user{}@example.com", self.next_id).into(),
            is_active: true,
        };
        self.users.push(new_user);
        self.next_id += 1;
        self.is_loading = false;
        cx.notify();
    }

    #[command]
    pub fn select_user(&mut self, id: u64, _: &ClickEvent, cx: &mut Context<Self>) {
        self.selected_user_id = Some(id);
        cx.notify();
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 加载初始数据
        self.add_user(&ClickEvent::default(), cx);
        self.add_user(&ClickEvent::default(), cx);
    }
}
```

## 4.1.10 小结

ViewModel 的标准结构：

1. **`#[derive(IModel)]`**：成为 GPUI Entity
2. **`#[component]`**：标记为 RML 视图，关联 `.rml` 文件
3. **`pub` 字段**：UI 可绑定的响应式状态
4. **`private` 字段**：内部状态，不暴露给 UI
5. **`new()` 方法**：构造函数
6. **`#[command]` 方法**：UI 可调用的命令
7. **`#[computed]` 方法**：派生值，自动缓存

掌握这个结构，你就掌握了 `.rml.rs` 文件的骨架。

下一节 → [4.2 宏属性详解](./macros.md)
