# 6.2 自定义组件

> **本节目标**：掌握用 `#[component]` 宏创建自定义组件，封装可复用的 UI 单元。

## 6.2.1 自定义组件的价值

自定义组件是大型应用的基础：

- **复用性**：一次编写，多处使用
- **封装性**：隐藏内部实现，暴露清晰接口
- **可维护性**：组件独立，修改不影响其他部分
- **可测试性**：组件可单独测试

## 6.2.2 创建自定义组件

### 步骤一：创建 `.rml` 模板

```html
<!-- components/counter.rml -->
<div class="counter">
    <button onclick={decrement} disabled={count <= min}>-</button>
    <span class="counter-value">{count}</span>
    <button onclick={increment} disabled={count >= max}>+</button>
</div>
```

### 步骤二：创建 `.rml.rs` 逻辑

```rust
// components/counter.rml.rs
use rml::prelude::*;

#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
    pub min: i32,
    pub max: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            count: 0,
            min: 0,
            max: 100,
        }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count < self.max {
            self.count += 1;
            cx.notify();
        }
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count > self.min {
            self.count -= 1;
            cx.notify();
        }
    }
}
```

### 步骤三：使用组件

```html
<!-- views/my_view.rml -->
<div>
    <h1>计数器示例</h1>
    <Counter count={initial_count} min={0} max={10} />
</div>
```

```rust
// views/my_view.rml.rs
use rml::prelude::*;
use crate::components::counter::Counter;

#[derive(IModel)]
#[component]
pub struct MyView {
    pub initial_count: i32,
}

impl MyView {
    pub fn new() -> Self {
        Self { initial_count: 5 }
    }
}
```

## 6.2.3 `#[component]` 宏

`#[component]` 宏标记一个结构体为 RML 组件：

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
    pub min: i32,
    pub max: i32,
}
```

### 与 `#[contribute]` 叠加

案例组件等需要向贡献点注册元数据时，**`#[contribute]` 写在 `#[component]` 之上**：

```rust
#[contribute(
    host = "demo.shell",
    id = "components.button",
    name = "case.button.title",
    kind = "case",
    parent_id = "cat.components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub button_clicks: i32,
}
```

`#[contribute]` 生成 `IContribution` + 每组件的 `__rml_register_*`；`build.rs` 扫描后生成统一 `register_rml_contributions(cx)`，在 `on_launch` 调用一次即可。`#[component]` 生成 RML 模板绑定与 codegen。两者互不冲突。

应用启动（`#[rml::main]` 已注入 `embed_contributions!`）：

```rust
crate::register_rml_contributions(cx);
```

纯元数据（无 struct、如分类节点）仍用 `contributions::register_case_categories` 等程序化 API。详见 [贡献点架构](../09-architecture/contribution-system.md)。

### `#[component]` 的参数

| 参数          | 类型     | 说明                  |
| ----------- | ------ | ------------------- |
| `template`  | 字符串    | 指定 `.rml` 模板文件路径    |
| `tag`       | 字符串    | 自定义标签名（默认为结构体名）     |
| `export`    | 布尔     | 是否导出为全局标签           |

### `template` 路径

路径相对于项目根目录的 `src` 目录：

```rust
// src/components/counter.rml
#[component]

// src/views/user/profile.rml
#[component]
```

## 6.2.4 组件的属性

组件的属性就是结构体的 `pub` 字段：

```rust
#[derive(IModel)]
#[component]
pub struct Button {
    pub text: SharedString,        // 输入属性
    pub variant: SharedString,     // 输入属性
    pub disabled: bool,            // 输入属性
    pub size: SharedString,        // 输入属性
}
```

### 在 `.rml` 中传递属性

```html
<Button
    text="提交"
    variant="primary"
    disabled={is_loading}
    size="large"
/>
```

### 属性的类型

| 类型              | 示例                              |
| --------------- | ------------------------------- |
| `SharedString`  | `text="提交"` 或 `text={value}`    |
| `i32`、`f64` 等   | `count={10}` 或 `count={value}`  |
| `bool`          | `disabled={true}` 或 `disabled`  |
| 枚举               | `variant="primary"`             |
| `Vec<T>`        | `items={my_items}`              |
| `Option<T>`     | `title={some_title}`            |

### 布尔属性的简写

```html
<!-- 完整写法 -->
<Button disabled={true}>提交</Button>

<!-- 简写 -->
<Button disabled>提交</Button>
```

## 6.2.5 组件的事件

组件可以通过 `Option<Arc<dyn Fn(...)>>` 字段声明事件回调：

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
    pub min: i32,
    pub max: i32,

    pub on_change: Option<Arc<dyn Fn(i32)>>,        // 值变化事件
    pub on_reach_min: Option<Arc<dyn Fn()>>,        // 达到最小值事件
    pub on_reach_max: Option<Arc<dyn Fn()>>,        // 达到最大值事件
}
```

### 触发事件

```rust
impl Counter {
    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count < self.max {
            self.count += 1;
            cx.notify();

            // 触发 on_change 事件
            if let Some(callback) = &self.on_change {
                callback(self.count);
            }

            // 触发 on_reach_max 事件
            if self.count == self.max {
                if let Some(callback) = &self.on_reach_max {
                    callback();
                }
            }
        }
    }
}
```

### 监听事件

```html
<Counter
    count={initial_count}
    min={0}
    max={10}
    on_change={handle_count_change}
    on_reach_max={handle_reach_max}
/>
```

```rust
#[command]
pub fn handle_count_change(&mut self, new_count: i32, cx: &mut Context<Self>) {
    println!("计数变化: {}", new_count);
}

#[command]
pub fn handle_reach_max(&mut self, cx: &mut Context<Self>) {
    println!("达到最大值！");
}
```

详见 [5.4 自定义事件](../05-events/custom-events.md)。

## 6.2.6 组件的双向绑定

组件可以通过 `model` 指令实现双向绑定：

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
    pub min: i32,
    pub max: i32,
}
```

### 在父视图中双向绑定

```html
<Counter model={my_count} min={0} max={10} />
```

### 实现双向绑定

组件需要实现 `TwoWayBinding` trait：

```rust
impl TwoWayBinding for Counter {
    type Value = i32;

    fn get_value(&self) -> Self::Value {
        self.count
    }

    fn set_value(&mut self, value: Self::Value, cx: &mut Context<Self>) {
        self.count = value;
        cx.notify();
    }
}
```

## 6.2.7 组件的插槽

组件可以通过 `<slot>` 接收父视图传递的内容：

```html
<!-- components/card.rml -->
<div class="card">
    <div class="card-header">
        <slot name="header">默认标题</slot>
    </div>
    <div class="card-body">
        <slot></slot>
    </div>
    <div class="card-footer">
        <slot name="footer">默认页脚</slot>
    </div>
</div>
```

### 使用插槽

```html
<Card>
    <template slot="header">
        <h2>用户信息</h2>
    </template>

    <template>
        <p>姓名: {user.name}</p>
        <p>邮箱: {user.email}</p>
    </template>

    <template slot="footer">
        <button onclick={edit_user}>编辑</button>
    </template>
</Card>
```

详见 [6.3 插槽与内容分发](./slots.md)。

## 6.2.8 组件的生命周期

组件支持与视图相同的生命周期回调：

```rust
#[derive(IModel)]
#[component]
pub struct DataLoader {
    pub data: Vec<Item>,
    pub is_loading: bool,
}

impl DataLoader {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        self.load_data(cx);
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut Context<Self>) {
        // 清理资源
    }

    fn load_data(&mut self, cx: &mut Context<Self>) {
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
}
```

详见 [第 8 章 生命周期管理](../08-lifecycle/INDEX.md)。

## 6.2.9 组件的嵌套

组件可以嵌套使用：

```html
<!-- components/user_card.rml -->
<div class="user-card">
    <Avatar src={user.avatar} />
    <div class="user-info">
        <h3>{user.name}</h3>
        <p>{user.email}</p>
    </div>
    <Button onclick={edit_user} variant="ghost">编辑</Button>
</div>
```

### 组件树

```
UserCard
├── Avatar
├── div
│   ├── h3
│   └── p
└── Button
```

## 6.2.10 完整示例：搜索框组件

```rust
// components/search_box.rml.rs
use std::time::Duration;
use rml::prelude::*;

#[derive(Clone)]
pub struct SearchEvent {
    pub query: SharedString,
}

#[derive(IModel)]
#[component]
pub struct SearchBox {
    pub query: SharedString,
    pub placeholder: SharedString,
    pub is_searching: bool,

    pub on_search: Option<Arc<dyn Fn(&SearchEvent)>>,
    pub on_clear: Option<Arc<dyn Fn()>>,

    debounce_task: Option<Task<()>>,
}

impl SearchBox {
    pub fn new() -> Self {
        Self {
            query: SharedString::default(),
            placeholder: "搜索...".into(),
            is_searching: false,
            on_search: None,
            on_clear: None,
            debounce_task: None,
        }
    }

    #[command]
    pub fn on_input(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
        self.query = ev.value.clone();
        cx.notify();

        // 取消之前的防抖任务
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }

        if self.query.is_empty() {
            return;
        }

        // 启动防抖搜索
        let query = self.query.clone();
        self.debounce_task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;

            let _ = this.update(&mut cx, |this, _cx| {
                if let Some(callback) = &this.on_search {
                    callback(&SearchEvent { query: query.clone() });
                }
            });
        }));
    }

    #[command]
    pub fn on_clear_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }

        self.query = SharedString::default();
        cx.notify();

        if let Some(callback) = &self.on_clear {
            callback();
        }
    }
}
```

```html
<!-- components/search_box.rml -->
<div class="search-box">
    <input
        model={query}
        oninput={on_input}
        placeholder={placeholder}
        class="search-input"
    />
    <button
        if={!query.is_empty()}
        onclick={on_clear_click}
        class="clear-btn"
    >
        ✕
    </button>
    <div if={is_searching} class="loading">搜索中...</div>
</div>
```

### 使用搜索框

```html
<!-- views/user_list.rml -->
<div class="user-list-view">
    <h1>用户列表</h1>

    <SearchBox
        placeholder="搜索用户..."
        on_search={handle_search}
        on_clear={handle_clear}
    />

    <ul>
        <li each={user in filtered_users} key={user.id}>
            {user.name} ({user.email})
        </li>
    </ul>
</div>
```

```rust
// views/user_list.rml.rs
use rml::prelude::*;
use crate::components::search_box::{SearchBox, SearchEvent};

#[derive(IModel)]
#[component]
pub struct UserListView {
    pub users: Vec<User>,
    pub search_query: SharedString,
    pub filtered_users: Vec<User>,
}

impl UserListView {
    pub fn new(users: Vec<User>) -> Self {
        Self {
            users: users.clone(),
            search_query: SharedString::default(),
            filtered_users: users,
        }
    }

    #[command]
    pub fn handle_search(&mut self, ev: &SearchEvent, cx: &mut Context<Self>) {
        self.search_query = ev.query.clone();
        self.filter_users(cx);
    }

    #[command]
    pub fn handle_clear(&mut self, cx: &mut Context<Self>) {
        self.search_query = SharedString::default();
        self.filtered_users = self.users.clone();
        cx.notify();
    }

    fn filter_users(&mut self, cx: &mut Context<Self>) {
        let query = self.search_query.to_lowercase();
        self.filtered_users = self
            .users
            .iter()
            .filter(|u| {
                u.name.to_lowercase().contains(&query)
                    || u.email.to_lowercase().contains(&query)
            })
            .cloned()
            .collect();
        cx.notify();
    }
}
```

## 6.2.11 组件设计的最佳实践

### 1. 单一职责

每个组件只做一件事：

```rust
// ✅ 单一职责
pub struct Avatar { ... }       // 只显示头像
pub struct UserName { ... }     // 只显示用户名
pub struct UserCard { ... }     // 组合 Avatar 和 UserName

// ❌ 职责混乱
pub struct UserAvatarAndNameAndActions { ... }
```

### 2. 通过属性配置，而非继承

```rust
// ✅ 通过属性配置
#[component]
pub struct Button {
    pub variant: SharedString,  // primary, secondary, danger
    pub size: SharedString,     // small, medium, large
    pub disabled: bool,
}

// ❌ 通过继承
pub struct PrimaryButton : Button { ... }
pub struct LargeButton : Button { ... }
```

### 3. 事件向上，属性向下

```
父视图 --属性--> 子组件
父视图 <--事件-- 子组件
```

```rust
// ✅ 属性向下，事件向上
pub struct SearchBox {
    pub placeholder: SharedString,                    // 属性向下
    pub on_search: Option<Arc<dyn Fn(&SearchEvent)>>, // 事件向上
}

// ❌ 子组件直接修改父视图状态
pub struct SearchBox {
    parent: Entity<ParentView>,  // 强耦合
}
```

### 4. 合理的默认值

```rust
impl Button {
    pub fn new() -> Self {
        Self {
            text: SharedString::default(),
            variant: "primary".into(),    // 默认主要按钮
            size: "medium".into(),         // 默认中等大小
            disabled: false,
        }
    }
}
```

### 5. 清晰的接口文档

```rust
/// 按钮组件
///
/// # 属性
/// - `text`: 按钮文本
/// - `variant`: 样式变体（"primary" | "secondary" | "danger" | "ghost"）
/// - `size`: 尺寸（"small" | "medium" | "large"）
/// - `disabled`: 是否禁用
///
/// # 事件
/// - `on_click`: 点击事件
///
/// # 示例
/// ```html
/// <Button text="提交" variant="primary" size="large" on_click={submit} />
/// ```
#[derive(IModel)]
#[component]
pub struct Button {
    pub text: SharedString,
    pub variant: SharedString,
    pub size: SharedString,
    pub disabled: bool,
    pub on_click: Option<Arc<dyn Fn(&ClickEvent)>>,
}
```

## 6.2.12 小结

自定义组件是 RML 的核心复用机制：

- **创建**：`#[component]` + `.rml` 模板
- **属性**：结构体的 `pub` 字段
- **事件**：`Option<Arc<dyn Fn(...)>>` 字段
- **双向绑定**：实现 `TwoWayBinding` trait
- **插槽**：`<slot>` 接收父视图内容
- **生命周期**：`#[on_loaded]`、`#[on_unloaded]`

掌握自定义组件，你就能构建可复用、可组合的 UI 库。

下一节 → [6.3 插槽与内容分发](./slots.md)
