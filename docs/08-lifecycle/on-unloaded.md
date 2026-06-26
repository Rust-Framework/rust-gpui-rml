# 8.3 on_unloaded 与清理

> **本节目标**：深入掌握 `#[on_unloaded]` 回调的使用场景和资源清理最佳实践。

## 8.3.1 on_unloaded 的触发时机

`#[on_unloaded]` 在视图被销毁前触发：

```
视图被销毁
    │
    ▼
#[on_unloaded]  ← 此时触发
    │
    ▼
从视图树移除
    │
    ▼
销毁 Entity
```

## 8.3.2 on_unloaded 的用途

### 用途一：取消异步任务

```rust
use std::time::Duration;

#[derive(Model)]
#[component]
pub struct DataView {
    pub data: Vec<Item>,
    load_task: Option<Task<()>>,
    refresh_timer: Option<Task<()>>,
}

impl DataView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.load_data(cx);
        self.start_refresh_timer(cx);
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 取消加载任务
        if let Some(task) = self.load_task.take() {
            task.abort();
        }

        // 取消定时器
        if let Some(timer) = self.refresh_timer.take() {
            timer.abort();
        }
    }

    fn load_data(&mut self, cx: &mut ViewContext<Self>) {
        self.load_task = Some(cx.spawn(|this, mut cx| async move {
            let data = fetch_data().await;
            let _ = this.update(&mut cx, |this, cx| {
                this.data = data;
                cx.notify();
            });
        }));
    }

    fn start_refresh_timer(&mut self, cx: &mut ViewContext<Self>) {
        self.refresh_timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    this.load_data(cx);
                });
            }
        }));
    }
}
```

### 用途二：取消订阅

```rust
#[derive(Model)]
#[component]
pub struct NotificationView {
    pub notifications: Vec<Notification>,
    subscription: Option<Subscription>,
}

impl NotificationView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        let service = cx.global::<Entity<NotificationService>>();
        self.subscription = Some(cx.subscribe(&service, |this, _, event, cx| {
            this.handle_notification(event, cx);
        }));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 取消订阅
        if let Some(sub) = self.subscription.take() {
            sub.unsubscribe();
        }
    }
}
```

### 用途三：保存状态

```rust
#[derive(Model)]
#[component]
pub struct EditorView {
    pub content: SharedString,
    pub cursor_position: usize,
}

impl EditorView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 从本地存储加载
        if let Some(saved) = cx.local_storage().get("editor_content") {
            self.content = saved.into();
            cx.notify();
        }
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
        // 保存到本地存储
        cx.local_storage().set("editor_content", &self.content);
        cx.local_storage().set("editor_cursor", &self.cursor_position.to_string());
    }
}
```

### 用途四：释放资源

```rust
#[derive(Model)]
#[component]
pub struct VideoPlayerView {
    pub video_url: SharedString,
    pub is_playing: bool,
    video_handle: Option<VideoHandle>,
}

impl VideoPlayerView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 加载视频
        self.video_handle = Some(VideoPlayer::load(&self.video_url, cx));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 释放视频资源
        if let Some(handle) = self.video_handle.take() {
            handle.release();
        }
    }
}
```

### 用途五：通知其他视图

```rust
#[derive(Model)]
#[component]
pub struct ChatView {
    pub is_active: bool,
    pub user_id: u64,
}

impl ChatView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.is_active = true;
        cx.notify();

        // 通知服务器用户上线
        let user_id = self.user_id;
        cx.spawn(|_, mut cx| async move {
            notify_user_online(user_id).await;
        }).detach();
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
        self.is_active = false;

        // 通知服务器用户下线
        let user_id = self.user_id;
        cx.spawn(|_, mut cx| async move {
            notify_user_offline(user_id).await;
        }).detach();
    }
}
```

## 8.3.3 on_unloaded 的最佳实践

### 1. 必须取消所有异步任务

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    // ✅ 取消所有任务
    if let Some(task) = self.load_task.take() {
        task.abort();
    }
    if let Some(task) = self.refresh_task.take() {
        task.abort();
    }
    if let Some(task) = self.upload_task.take() {
        task.abort();
    }
}
```

不取消会导致任务继续运行，访问已销毁的视图，引发 panic。

### 2. 必须取消所有订阅

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    // ✅ 取消所有订阅
    if let Some(sub) = self.notification_sub.take() {
        sub.unsubscribe();
    }
    if let Some(sub) = self.user_state_sub.take() {
        sub.unsubscribe();
    }
}
```

### 3. 保存重要状态

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    // ✅ 保存用户数据
    cx.local_storage().set("draft_content", &self.draft);
    cx.local_storage().set("scroll_position", &self.scroll_position.to_string());
}
```

### 4. 释放外部资源

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    // ✅ 释放外部资源
    if let Some(handle) = self.file_handle.take() {
        handle.close();
    }
    if let Some(connection) = self.db_connection.take() {
        connection.close();
    }
}
```

### 5. 避免在 on_unloaded 中启动新任务

```rust
// ❌ 不要在 on_unloaded 中启动新任务
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.spawn(|this, mut cx| async move {
        // 视图即将销毁，这个任务可能无法正常完成
        save_data().await;
    }).detach();
}

// ✅ 同步保存或提前保存
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.local_storage().set("data", &self.data);
}
```

## 8.3.4 资源清理清单

在 `#[on_unloaded]` 中需要清理的资源：

| 资源类型        | 清理方式              | 示例                    |
| ----------- | ------------------ | --------------------- |
| 异步任务        | `task.abort()`     | `Task<()>`            |
| 订阅          | `sub.unsubscribe()`| `Subscription`        |
| 定时器         | `timer.abort()`    | `Task<()>`            |
| 文件句柄        | `handle.close()`   | `FileHandle`          |
| 网络连接        | `conn.close()`     | `Connection`          |
| 数据库连接       | `conn.close()`     | `DbConnection`        |
| 本地存储        | `storage.set(...)` | 保存状态                  |

## 8.3.5 完整示例：聊天应用

```rust
use std::time::Duration;
use rml::prelude::*;

#[derive(Model)]
#[component]
pub struct ChatView {
    pub messages: Vec<Message>,
    pub input_text: SharedString,
    pub is_connected: bool,
    pub online_users: Vec<User>,

    #[element]
    pub message_list: ElementRef<Div>,
    #[element]
    pub input_field: ElementRef<Input>,

    websocket: Option<WebSocketConnection>,
    refresh_task: Option<Task<()>>,
    message_subscription: Option<Subscription>,
}

impl ChatView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            input_text: SharedString::default(),
            is_connected: false,
            online_users: Vec::new(),
            message_list: ElementRef::default(),
            input_field: ElementRef::default(),
            websocket: None,
            refresh_task: None,
            message_subscription: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 1. 连接 WebSocket
        self.connect_websocket(cx);

        // 2. 加载历史消息
        self.load_history(cx);

        // 3. 启动心跳定时器
        self.start_heartbeat(cx);

        // 4. 订阅全局通知
        self.subscribe_notifications(cx);

        // 5. 聚焦输入框
        self.input_field.focus(cx);
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
        // 1. 保存草稿
        cx.local_storage().set("chat_draft", &self.input_text);

        // 2. 保存滚动位置
        let scroll_pos = self.message_list.scroll_position();
        cx.local_storage().set("chat_scroll", &scroll_pos.to_string());

        // 3. 取消心跳定时器
        if let Some(task) = self.refresh_task.take() {
            task.abort();
        }

        // 4. 取消订阅
        if let Some(sub) = self.message_subscription.take() {
            sub.unsubscribe();
        }

        // 5. 关闭 WebSocket 连接
        if let Some(ws) = self.websocket.take() {
            ws.close();
        }

        // 6. 通知服务器用户下线
        // 注意：这里用同步方式，因为视图即将销毁
        // 实际应用中可能需要提前处理
    }

    fn connect_websocket(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            match WebSocketConnection::connect("wss://chat.example.com").await {
                Ok(ws) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.websocket = Some(ws.clone());
                        this.is_connected = true;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.is_connected = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    fn load_history(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            let messages = fetch_message_history().await.unwrap_or_default();
            let _ = this.update(&mut cx, |this, cx| {
                this.messages = messages;
                cx.notify();
            });
        }).detach();
    }

    fn start_heartbeat(&mut self, cx: &mut ViewContext<Self>) {
        self.refresh_task = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(30))
                    .await;
                let _ = this.update(&mut cx, |this, cx| {
                    if let Some(ws) = &this.websocket {
                        ws.send_heartbeat();
                    }
                });
            }
        }));
    }

    fn subscribe_notifications(&mut self, cx: &mut ViewContext<Self>) {
        let notification_service = cx.global::<Entity<NotificationService>>();
        self.message_subscription = Some(cx.subscribe(
            &notification_service,
            |this, _, event, cx| {
                this.handle_notification(event, cx);
            },
        ));
    }

    fn handle_notification(&mut self, event: &NotificationEvent, cx: &mut ViewContext<Self>) {
        // 处理通知
        cx.notify();
    }

    #[command]
    pub fn send_message(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.input_text.is_empty() {
            return;
        }

        let message = Message {
            id: generate_id(),
            text: self.input_text.clone(),
            timestamp: chrono::Local::now(),
        };

        self.messages.push(message.clone());
        self.input_text = SharedString::default();
        cx.notify();

        // 发送到服务器
        if let Some(ws) = &self.websocket {
            ws.send_message(&message);
        }
    }
}
```

## 8.3.6 on_unloaded 的注意事项

### 1. 只触发一次

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    println!("视图卸载");  // 只打印一次
}
```

### 2. 此时还能访问 cx

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    // ✅ 可以访问 cx
    cx.local_storage().set("data", &self.data);
}
```

### 3. 不能启动新的异步任务

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, cx: &mut ViewContext<Self>) {
    // ❌ 视图即将销毁，新任务无法正常完成
    cx.spawn(|this, mut cx| async move {
        save_data().await;
    }).detach();
}
```

### 4. 条件渲染会触发

```html
<div if={show_view}>
    <MyView />
</div>
```

`show_view` 从 `true` 变 `false` 时，`MyView` 的 `#[on_unloaded]` 会被触发。

## 8.3.7 小结

`#[on_unloaded]` 是资源清理的核心回调：

- **触发时机**：视图被销毁前
- **用途**：取消任务、取消订阅、保存状态、释放资源
- **最佳实践**：必须清理所有资源，避免内存泄漏
- **限制**：不能启动新的异步任务
- **与条件渲染**：`if` 控制视图的销毁

下一节 → [8.4 异步任务管理](./async-tasks.md)
