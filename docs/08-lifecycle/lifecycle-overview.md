# 8.1 生命周期总览

> **本节目标**：了解 RML 视图和组件的生命周期阶段，以及各阶段的回调机制。

## 8.1.1 生命周期阶段

RML 视图和组件的生命周期分为四个阶段：

```
┌─────────────────────────────────────────────────────────────┐
│                     生命周期阶段                                │
│                                                             │
│  1. 创建（Creation）                                          │
│     └─ 结构体构造，字段初始化                                      │
│                                                             │
│  2. 加载（Loading）                                           │
│     └─ 视图首次渲染完成，可访问 DOM                                 │
│                                                             │
│  3. 更新（Updating）                                          │
│     └─ 状态变化触发重新渲染                                       │
│                                                             │
│  4. 卸载（Unmounting）                                        │
│     └─ 视图被销毁，释放资源                                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## 8.1.2 生命周期回调

RML 提供两个生命周期回调：

| 回调                | 触发时机           | 用途                  |
| ----------------- | -------------- | ------------------- |
| `#[on_loaded]`    | 视图首次渲染完成后      | 初始化数据、启动定时器、获取焦点    |
| `#[on_unloaded]`  | 视图被销毁前         | 清理资源、取消任务、保存状态      |

### 回调签名

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    // 视图加载完成
}

#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    // 视图即将卸载
}
```

## 8.1.3 生命周期流程

```
用户打开视图
    │
    ▼
┌─────────────────────────────────┐
│  1. 创建阶段                       │
│  - 调用 new() 构造函数              │
│  - 初始化字段                       │
│  - 注册到 GPUI Entity 系统         │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  2. 加载阶段                       │
│  - 编译 .rml 模板                 │
│  - 构建视图树                       │
│  - 首次渲染                        │
│  - 触发 #[on_loaded]             │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  3. 更新阶段（可重复）                 │
│  - 状态变化（cx.notify()）          │
│  - 重新渲染变化的部分                  │
│  - 触发 #[on_prop_change]       │
└─────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────┐
│  4. 卸载阶段                       │
│  - 触发 #[on_unloaded]           │
│  - 清理资源                        │
│  - 从视图树移除                     │
│  - 销毁 Entity                   │
└─────────────────────────────────┘
```

## 8.1.4 创建阶段

创建阶段通过构造函数完成：

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub data: Vec<Item>,
    pub is_loading: bool,

    // 私有字段（不参与数据绑定）
    timer: Option<Timer>,
    subscription: Option<Subscription>,
}

impl MyView {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            is_loading: false,
            timer: None,
            subscription: None,
        }
    }
}
```

### 创建阶段的限制

- 不能访问 `cx`（上下文）
- 不能启动异步任务
- 不能获取焦点
- 不能访问 DOM

这些操作需要在 `#[on_loaded]` 中完成。

## 8.1.5 加载阶段

加载阶段在视图首次渲染完成后触发 `#[on_loaded]`：

```rust
impl MyView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 1. 加载数据
        self.load_data(cx);

        // 2. 启动定时器
        self.start_timer(cx);

        // 3. 订阅全局状态
        self.subscribe_to_global_state(cx);

        // 4. 获取焦点
        self.focus_input(cx);
    }

    fn load_data(&mut self, cx: &mut ViewContext<Self>) {
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

    fn start_timer(&mut self, cx: &mut ViewContext<Self>) {
        self.timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(1))
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.tick(cx);
                });
            }
        }));
    }

    fn subscribe_to_global_state(&mut self, cx: &mut ViewContext<Self>) {
        self.subscription = Some(cx.subscribe(&self.global_state, |this, _, event, cx| {
            this.handle_global_event(event, cx);
        }));
    }

    fn focus_input(&mut self, cx: &mut ViewContext<Self>) {
        self.input_element.focus(cx);
    }
}
```

### 加载阶段能做什么

- 访问 `cx`（上下文）
- 启动异步任务
- 访问 DOM（通过 `ElementRef`）
- 获取焦点
- 订阅事件
- 加载数据

## 8.1.6 更新阶段

更新阶段由 `cx.notify()` 触发：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;       // 1. 修改状态
    cx.notify();           // 2. 通知变化 → 3. 重新渲染
}
```

### 属性变化回调

组件可以通过 `#[on_prop_change]` 监听属性变化：

```rust
#[derive(Model)]
#[component(template = "components/data_view.rml")]
pub struct DataView {
    pub data_id: u64,
    pub data: Option<Data>,
}

impl DataView {
    #[on_prop_change(data_id)]
    pub fn on_data_id_change(&mut self, cx: &mut ViewContext<Self>) {
        self.load_data(self.data_id, cx);
    }
}
```

## 8.1.7 卸载阶段

卸载阶段在视图被销毁前触发 `#[on_unloaded]`：

```rust
impl MyView {
    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 1. 取消定时器
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }

        // 2. 取消订阅
        if let Some(subscription) = self.subscription.take() {
            subscription.unsubscribe();
        }

        // 3. 保存状态
        self.save_state();
    }

    fn save_state(&self) {
        // 保存到本地存储
    }
}
```

### 卸载阶段必须做的事

- 取消异步任务
- 取消订阅
- 释放资源（文件句柄、网络连接）
- 保存状态

## 8.1.8 生命周期的触发时机

### 视图的创建和卸载

```rust
// 创建视图
let view = cx.new_model(|_| MyView::new());

// 视图首次渲染 → 触发 #[on_loaded]

// ... 用户操作 ...

// 视图被销毁 → 触发 #[on_unloaded]
```

### 条件渲染的生命周期

```html
<div if={is_visible}>
    <MyView />
</div>
```

- `is_visible` 从 `false` 变 `true`：创建视图，触发 `#[on_loaded]`
- `is_visible` 从 `true` 变 `false`：销毁视图，触发 `#[on_unloaded]`

### 列表渲染的生命周期

```html
<ul>
    <li each={item in items} key={item.id}>
        <MyView data={item} />
    </li>
</ul>
```

- 新增项：创建新视图，触发 `#[on_loaded]`
- 删除项：销毁视图，触发 `#[on_unloaded]`
- 移动项：不触发（通过 `key` 复用）

## 8.1.9 生命周期的注意事项

### 1. `#[on_loaded]` 只触发一次

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    println!("视图加载完成");  // 只打印一次
}
```

### 2. `#[on_unloaded]` 在销毁前触发

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    // 此时还能访问 cx，可以做清理工作
    println!("视图即将卸载");
}
```

### 3. 异步任务必须在卸载时取消

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    // 不取消会导致任务继续运行，访问已销毁的视图
    if let Some(task) = self.async_task.take() {
        task.abort();
    }
}
```

### 4. 避免在 `#[on_loaded]` 中做重计算

```rust
// ❌ 阻塞 UI
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    let data = heavy_computation();  // 阻塞
    self.data = data;
    cx.notify();
}

// ✅ 异步执行
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.spawn(|this, mut cx| async move {
        let data = heavy_computation().await;  // 不阻塞
        let _ = this.update(&mut cx, |this, cx| {
            this.data = data;
            cx.notify();
        });
    }).detach();
}
```

## 8.1.10 生命周期与状态管理

### 状态持久化

```rust
#[derive(Model)]
#[component]
pub struct EditorView {
    pub content: SharedString,
}

impl EditorView {
    pub fn new() -> Self {
        Self {
            content: SharedString::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 从本地存储加载内容
        if let Some(saved) = cx.local_storage().get("editor_content") {
            self.content = saved.into();
            cx.notify();
        }
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
        // 保存内容到本地存储
        cx.local_storage().set("editor_content", &self.content);
    }
}
```

### 跨视图状态同步

```rust
#[derive(Model)]
#[component]
pub struct ParentView {
    pub shared_state: Entity<SharedState>,
    pub child_view: Entity<ChildView>,
}

impl ParentView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 订阅共享状态
        cx.subscribe(&self.shared_state, |this, state, event, cx| {
            // 同步状态到子视图
            this.child_view.update(cx, |child, cx| {
                child.sync_state(event, cx);
            });
        }).detach();
    }
}
```

## 8.1.11 小结

RML 的生命周期管理：

- **四个阶段**：创建 → 加载 → 更新 → 卸载
- **两个回调**：`#[on_loaded]`、`#[on_unloaded]`
- **创建阶段**：构造函数初始化字段
- **加载阶段**：首次渲染完成，可访问 DOM 和 cx
- **更新阶段**：`cx.notify()` 触发重新渲染
- **卸载阶段**：清理资源，取消任务

下一节 → [8.2 on_loaded 与初始化](./on-loaded.md)
