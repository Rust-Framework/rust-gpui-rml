# 8.2 on_loaded 与初始化

> **本节目标**：深入掌握 `#[on_loaded]` 回调的使用场景和最佳实践。

## 8.2.1 on_loaded 的触发时机

`#[on_loaded]` 在视图首次渲染完成后触发：

```
构造函数 new()
    │
    ▼
编译 .rml 模板
    │
    ▼
构建视图树
    │
    ▼
首次渲染
    │
    ▼
#[on_loaded]  ← 此时触发
```

## 8.2.2 on_loaded 的用途

### 用途一：加载数据

```rust
#[derive(IModel)]
#[component]
pub struct UserListView {
    pub users: Vec<User>,
    pub is_loading: bool,
    pub error: Option<SharedString>,
}

impl UserListView {
    pub fn new() -> Self {
        Self {
            users: Vec::new(),
            is_loading: false,
            error: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        self.fetch_users(cx);
    }

    fn fetch_users(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.error = None;
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            match fetch_users_from_api().await {
                Ok(users) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.users = users;
                        this.is_loading = false;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string().into());
                        this.is_loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
}
```

### 用途二：启动定时器

```rust
use std::time::Duration;

#[derive(IModel)]
#[component]
pub struct ClockView {
    pub current_time: SharedString,
    timer: Option<Task<()>>,
}

impl ClockView {
    pub fn new() -> Self {
        Self {
            current_time: SharedString::default(),
            timer: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        self.update_time(cx);
        self.start_timer(cx);
    }

    fn start_timer(&mut self, cx: &mut Context<Self>) {
        self.timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(1))
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.update_time(cx);
                });
            }
        }));
    }

    fn update_time(&mut self, cx: &mut Context<Self>) {
        self.current_time = chrono::Local::now().format("%H:%M:%S").to_string().into();
        cx.notify();
    }
}
```

### 用途三：订阅事件

```rust
#[derive(IModel)]
#[component]
pub struct NotificationView {
    pub notifications: Vec<Notification>,
    subscription: Option<Subscription>,
}

impl NotificationView {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            subscription: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 订阅全局通知服务
        let notification_service = cx.global::<Entity<NotificationService>>();

        self.subscription = Some(cx.subscribe(
            &notification_service,
            |this, _, event, cx| {
                this.handle_notification(event, cx);
            },
        ));
    }

    fn handle_notification(&mut self, event: &NotificationEvent, cx: &mut Context<Self>) {
        self.notifications.push(event.notification.clone());
        cx.notify();
    }
}
```

### 用途四：获取焦点

```rust
#[derive(IModel)]
#[component]
pub struct SearchView {
    pub query: SharedString,

    #[element]
    pub search_input: ElementRef<Input>,
}

impl SearchView {
    pub fn new() -> Self {
        Self {
            query: SharedString::default(),
            search_input: ElementRef::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 自动聚焦搜索框
        self.search_input.focus(cx);
    }
}
```

```html
<!-- views/search.rml -->
<div class="search-view">
    <input
        ref="search_input"
        value={query}
        placeholder="搜索..."
    />
</div>
```

### 用途五：初始化第三方库

```rust
#[derive(IModel)]
#[component]
pub struct ChartView {
    pub data: Vec<f64>,
    #[element]
    pub chart_container: ElementRef<Div>,
}

impl ChartView {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            chart_container: ElementRef::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 初始化图表
        self.init_chart(cx);
        self.load_data(cx);
    }

    fn init_chart(&mut self, cx: &mut Context<Self>) {
        // 获取图表容器，初始化图表库
        // self.chart_container...
    }

    fn load_data(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            let data = fetch_chart_data().await;
            let _ = this.update(&mut cx, |this, cx| {
                this.data = data;
                this.update_chart(cx);
            });
        }).detach();
    }

    fn update_chart(&mut self, cx: &mut Context<Self>) {
        // 更新图表数据
        cx.notify();
    }
}
```

## 8.2.3 on_loaded 的最佳实践

### 1. 分离关注点

```rust
// ✅ 分离初始化逻辑
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    self.load_initial_data(cx);
    self.setup_subscriptions(cx);
    self.start_background_tasks(cx);
    self.focus_initial_element(cx);
}

fn load_initial_data(&mut self, cx: &mut Context<Self>) {
    // 只负责加载数据
}

fn setup_subscriptions(&mut self, cx: &mut Context<Self>) {
    // 只负责设置订阅
}

// ❌ 所有逻辑堆在一起
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    // 加载数据
    self.is_loading = true;
    cx.notify();
    cx.spawn(|this, mut cx| async move {
        // ...
    }).detach();

    // 设置订阅
    cx.subscribe(&self.global_state, |this, _, event, cx| {
        // ...
    }).detach();

    // 启动定时器
    cx.spawn(|this, mut cx| async move {
        // ...
    }).detach();

    // 获取焦点
    self.input.focus(cx);
}
```

### 2. 异步操作用 spawn

```rust
// ✅ 异步操作用 spawn
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    cx.spawn(|this, mut cx| async move {
        let data = fetch_data().await;
        let _ = this.update(&mut cx, |this, cx| {
            this.data = data;
            cx.notify();
        });
    }).detach();
}

// ❌ 阻塞 UI
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    let data = fetch_data_blocking();  // 阻塞 UI 线程
    self.data = data;
    cx.notify();
}
```

### 3. 错误处理

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    self.load_data(cx);
}

fn load_data(&mut self, cx: &mut Context<Self>) {
    self.is_loading = true;
    cx.notify();

    cx.spawn(|this, mut cx| async move {
        match fetch_data().await {
            Ok(data) => {
                let _ = this.update(&mut cx, |this, cx| {
                    this.data = data;
                    this.is_loading = false;
                    this.error = None;
                    cx.notify();
                });
            }
            Err(e) => {
                let _ = this.update(&mut cx, |this, cx| {
                    this.error = Some(e.to_string().into());
                    this.is_loading = false;
                    cx.notify();
                });
            }
        }
    }).detach();
}
```

### 4. 避免重复加载

```rust
#[derive(IModel)]
#[component]
pub struct DataView {
    pub data: Vec<Item>,
    pub is_loaded: bool,
}

impl DataView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        if !self.is_loaded {
            self.load_data(cx);
            self.is_loaded = true;
        }
    }
}
```

## 8.2.4 on_loaded 与条件渲染

条件渲染会触发 `#[on_loaded]`：

```html
<div if={show_detail}>
    <DetailView data={selected_item} />
</div>
```

```rust
#[derive(IModel)]
#[component]
pub struct DetailView {
    pub data: Item,
}

impl DetailView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        println!("详情视图加载");
        self.load_related_data(cx);
    }
}
```

- `show_detail` 从 `false` 变 `true`：创建 `DetailView`，触发 `#[on_loaded]`
- `show_detail` 从 `true` 变 `false`：销毁 `DetailView`，触发 `#[on_unloaded]`

## 8.2.5 on_loaded 与列表渲染

列表新增项会触发 `#[on_loaded]`：

```html
<ul>
    <li each={item in items} key={item.id}>
        <ItemView item={item} />
    </li>
</ul>
```

```rust
#[derive(IModel)]
#[component]
pub struct ItemView {
    pub item: Item,
}

impl ItemView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 每个新项都会触发
        self.load_thumbnail(cx);
    }
}
```

## 8.2.6 完整示例：用户资料页

```rust
use std::time::Duration;
use rml::prelude::*;

#[derive(IModel)]
#[component]
pub struct UserProfileView {
    pub user_id: u64,
    pub user: Option<User>,
    pub posts: Vec<Post>,
    pub is_loading: bool,
    pub error: Option<SharedString>,

    #[element]
    pub avatar_input: ElementRef<Input>,

    refresh_timer: Option<Task<()>>,
}

impl UserProfileView {
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            user: None,
            posts: Vec::new(),
            is_loading: false,
            error: None,
            avatar_input: ElementRef::default(),
            refresh_timer: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        // 1. 加载用户数据
        self.load_user(cx);

        // 2. 加载用户帖子
        self.load_posts(cx);

        // 3. 启动定时刷新
        self.start_refresh_timer(cx);
    }

    fn load_user(&mut self, cx: &mut Context<Self>) {
        self.is_loading = true;
        cx.notify();

        let user_id = self.user_id;
        cx.spawn(|this, mut cx| async move {
            match fetch_user(user_id).await {
                Ok(user) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.user = Some(user);
                        this.is_loading = false;
                        this.error = None;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string().into());
                        this.is_loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    fn load_posts(&mut self, cx: &mut Context<Self>) {
        let user_id = self.user_id;
        cx.spawn(|this, mut cx| async move {
            match fetch_user_posts(user_id).await {
                Ok(posts) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.posts = posts;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string().into());
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    fn start_refresh_timer(&mut self, cx: &mut Context<Self>) {
        self.refresh_timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.refresh_data(cx);
                });
            }
        }));
    }

    fn refresh_data(&mut self, cx: &mut Context<Self>) {
        self.load_user(cx);
        self.load_posts(cx);
    }

    #[command]
    pub fn on_avatar_change(&mut self, ev: &ChangeEvent, cx: &mut Context<Self>) {
        if let Some(user) = &mut self.user {
            user.avatar_url = ev.value.clone();
            cx.notify();

            // 上传头像
            let user_id = self.user_id;
            let avatar_url = ev.value.clone();
            cx.spawn(|this, mut cx| async move {
                if let Err(e) = update_avatar(user_id, &avatar_url).await {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string().into());
                        cx.notify();
                    });
                }
            }).detach();
        }
    }
}
```

```html
<!-- views/user_profile.rml -->
<div class="user-profile">
    <div if={is_loading} class="loading">加载中...</div>

    <div if={error.is_some()} class="error">{error}</div>

    <div if={!is_loading && error.is_none() && user.is_some()}>
        <div class="user-header">
            <img src={user.avatar_url} class="user-avatar" />
            <div class="user-info">
                <h1>{user.name}</h1>
                <p>{user.email}</p>
                <p>{user.bio}</p>
            </div>
        </div>

        <div class="user-posts">
            <h2>帖子</h2>
            <ul>
                <li each={post in posts} key={post.id} class="post-item">
                    <h3>{post.title}</h3>
                    <p>{post.content}</p>
                    <span class="post-date">{post.created_at}</span>
                </li>
            </ul>
        </div>
    </div>
</div>
```

## 8.2.7 小结

`#[on_loaded]` 是视图初始化的核心回调：

- **触发时机**：视图首次渲染完成后
- **用途**：加载数据、启动定时器、订阅事件、获取焦点
- **最佳实践**：分离关注点、异步操作、错误处理、避免重复加载
- **与条件渲染**：`if` 控制视图的创建和销毁
- **与列表渲染**：新增项触发 `#[on_loaded]`

下一节 → [8.3 on_unloaded 与清理](./on-unloaded.md)
