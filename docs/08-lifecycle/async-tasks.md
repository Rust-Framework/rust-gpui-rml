# 8.4 异步任务管理

> **本节目标**：掌握 RML 中异步任务的启动、取消、错误处理和生命周期管理。

## 8.4.1 异步任务的场景

RML 应用中常见的异步任务：

| 场景       | 示例                          |
| -------- | --------------------------- |
| 网络请求     | 获取数据、提交表单、上传文件              |
| 定时任务     | 心跳、轮询、倒计时                   |
| 文件操作     | 读写文件、解析大文件                  |
| 计算密集型任务  | 图像处理、数据分析                   |
| 动画       | 过渡动画、连续动画                   |

## 8.4.2 启动异步任务

### 基本用法

```rust
use rml::prelude::*;

#[derive(Model)]
#[view]
pub struct DataView {
    pub data: Vec<Item>,
    pub is_loading: bool,
}

impl DataView {
    #[command]
    pub fn refresh(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
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

### spawn 的参数

```rust
cx.spawn(|this, mut cx| async move {
    // this: WeakView<Self> - 弱引用，避免循环引用
    // cx: AsyncViewContext<Self> - 异步上下文

    // 异步操作
    let result = some_async_operation().await;

    // 更新视图
    let _ = this.update(&mut cx, |this, cx| {
        // 在这里可以访问 &mut Self 和 &mut ViewContext<Self>
        this.data = result;
        cx.notify();
    });
}).detach();
```

### detach vs 保留 Task

```rust
// detach：任务独立运行，无法取消
cx.spawn(|this, mut cx| async move {
    // ...
}).detach();

// 保留 Task：可以取消
let task = cx.spawn(|this, mut cx| async move {
    // ...
});
self.task = Some(task);  // 保存到字段

// 取消任务
if let Some(task) = self.task.take() {
    task.abort();
}
```

## 8.4.3 异步任务的取消

### 基本取消

```rust
#[derive(Model)]
#[view]
pub struct SearchView {
    pub search_text: SharedString,
    pub search_results: Vec<SearchResult>,
    search_task: Option<Task<()>>,
}

impl SearchView {
    #[command]
    pub fn on_search_input(&mut self, ev: &InputEvent, cx: &mut ViewContext<Self>) {
        self.search_text = ev.value.clone();
        cx.notify();

        // 取消之前的搜索任务
        if let Some(task) = self.search_task.take() {
            task.abort();
        }

        // 启动新的搜索任务
        let query = self.search_text.clone();
        self.search_task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(300))
                .await;

            let results = search(&query).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.search_results = results;
                cx.notify();
            });
        }));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 视图卸载时取消任务
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
    }
}
```

### 取消正在运行的请求

```rust
impl SearchView {
    #[command]
    pub fn cancel_search(&mut self, _: &ClickEvent, _cx: &mut ViewContext<Self>) {
        if let Some(task) = self.search_task.take() {
            task.abort();
            // 任务被取消，不会执行后续的 update
        }
    }
}
```

## 8.4.4 错误处理

### Result 处理

```rust
cx.spawn(|this, mut cx| async move {
    match fetch_data().await {
        Ok(data) => {
            let _ = this.update(&mut cx, |this, cx| {
                this.data = data;
                this.error = None;
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
```

### 多个错误处理

```rust
cx.spawn(|this, mut cx| async move {
    let user_result = fetch_user().await;
    let posts_result = fetch_posts().await;

    let _ = this.update(&mut cx, |this, cx| {
        match (user_result, posts_result) {
            (Ok(user), Ok(posts)) => {
                this.user = Some(user);
                this.posts = posts;
                this.error = None;
            }
            (Ok(user), Err(e)) => {
                this.user = Some(user);
                this.error = Some(format!("加载帖子失败: {}", e).into());
            }
            (Err(e), _) => {
                this.error = Some(format!("加载用户失败: {}", e).into());
            }
        }
        this.is_loading = false;
        cx.notify();
    });
}).detach();
```

### 重试机制

```rust
async fn fetch_with_retry<T, F, Fut>(max_retries: usize, f: F) -> Result<T, String>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, String>>,
{
    let mut retries = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                retries += 1;
                if retries >= max_retries {
                    return Err(e);
                }
                tokio::time::sleep(std::time::Duration::from_secs(retries as u64)).await;
            }
        }
    }
}

// 使用
cx.spawn(|this, mut cx| async move {
    let result = fetch_with_retry(3, || async {
        fetch_data().await.map_err(|e| e.to_string())
    }).await;

    let _ = this.update(&mut cx, |this, cx| {
        match result {
            Ok(data) => {
                this.data = data;
                this.error = None;
            }
            Err(e) => {
                this.error = Some(e.into());
            }
        }
        cx.notify();
    });
}).detach();
```

## 8.4.5 并发任务

### 并行执行

```rust
cx.spawn(|this, mut cx| async move {
    // 并行执行多个任务
    let (user, posts, friends) = tokio::join!(
        fetch_user(),
        fetch_posts(),
        fetch_friends()
    );

    let _ = this.update(&mut cx, |this, cx| {
        this.user = user.ok();
        this.posts = posts.unwrap_or_default();
        this.friends = friends.unwrap_or_default();
        this.is_loading = false;
        cx.notify();
    });
}).detach();
```

### 竞速执行

```rust
cx.spawn(|this, mut cx| async move {
    // 从多个数据源获取，用最快的那个
    let result = tokio::select! {
        result = fetch_from_cache() => result,
        result = fetch_from_api() => result,
    };

    let _ = this.update(&mut cx, |this, cx| {
        this.data = result.ok();
        cx.notify();
    });
}).detach();
```

### 顺序执行

```rust
cx.spawn(|this, mut cx| async move {
    // 顺序执行
    let user = fetch_user().await?;
    let posts = fetch_posts(user.id).await?;
    let comments = fetch_comments(user.id).await?;

    let _ = this.update(&mut cx, |this, cx| {
        this.user = Some(user);
        this.posts = posts;
        this.comments = comments;
        cx.notify();
    });
    Ok(())
}).detach();
```

## 8.4.6 定时任务

### 一次性定时

```rust
use std::time::Duration;

cx.spawn(|this, mut cx| async move {
    cx.background_executor()
        .timer(Duration::from_secs(5))
        .await;

    let _ = this.update(&mut cx, |this, cx| {
        this.show_notification = false;
        cx.notify();
    });
}).detach();
```

### 循环定时

```rust
use std::time::Duration;

#[derive(Model)]
#[view]
pub struct ClockView {
    pub current_time: SharedString,
    timer: Option<Task<()>>,
}

impl ClockView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(1))
                    .await;

                let _ = this.update(&mut cx, |this, cx| {
                    this.current_time = chrono::Local::now()
                        .format("%H:%M:%S")
                        .to_string()
                        .into();
                    cx.notify();
                });
            }
        }));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
    }
}
```

### 条件循环

```rust
self.timer = Some(cx.spawn(|this, mut cx| async move {
    let mut count = 0;
    while count < 10 {
        cx.background_executor()
            .timer(Duration::from_secs(1))
            .await;

        let should_continue = this.update(&mut cx, |this, cx| {
            this.count = count;
            cx.notify();
            this.should_continue
        }).unwrap_or(false);

        if !should_continue {
            break;
        }

        count += 1;
    }
}));
```

## 8.4.7 进度跟踪

```rust
#[derive(Model)]
#[view]
pub struct UploadView {
    pub upload_progress: f64,
    pub is_uploading: bool,
    upload_task: Option<Task<()>>,
}

impl UploadView {
    #[command]
    pub fn start_upload(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.is_uploading = true;
        self.upload_progress = 0.0;
        cx.notify();

        self.upload_task = Some(cx.spawn(|this, mut cx| async move {
            let total = 100;
            for i in 1..=total {
                // 模拟上传
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;

                let _ = this.update(&mut cx, |this, cx| {
                    this.upload_progress = (i as f64 / total as f64) * 100.0;
                    cx.notify();
                });
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.is_uploading = false;
                cx.notify();
            });
        }));
    }

    #[command]
    pub fn cancel_upload(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(task) = self.upload_task.take() {
            task.abort();
        }
        self.is_uploading = false;
        self.upload_progress = 0.0;
        cx.notify();
    }
}
```

## 8.4.8 异步任务的注意事项

### 1. 弱引用避免循环

```rust
cx.spawn(|this, mut cx| async move {
    // this 是 WeakView<Self>，不会阻止视图被销毁
    // 如果视图已销毁，this.update() 会返回 Err
    let _ = this.update(&mut cx, |this, cx| {
        // ...
    });
}).detach();
```

### 2. 检查 update 返回值

```rust
cx.spawn(|this, mut cx| async move {
    let data = fetch_data().await;

    let result = this.update(&mut cx, |this, cx| {
        this.data = data;
        cx.notify();
    });

    if result.is_err() {
        // 视图已被销毁，无法更新
        println!("视图已销毁，无法更新");
    }
}).detach();
```

### 3. 避免在异步任务中持有锁

```rust
// ❌ 在异步任务中持有锁可能导致死锁
cx.spawn(|this, mut cx| async move {
    let _guard = self.mutex.lock().unwrap();  // 持有锁
    let data = fetch_data().await;  // 异步操作时仍持有锁
    // ...
}).detach();

// ✅ 在 update 闭包中访问数据
cx.spawn(|this, mut cx| async move {
    let data = fetch_data().await;
    let _ = this.update(&mut cx, |this, cx| {
        // 在这里安全地访问 self
        this.data = data;
        cx.notify();
    });
}).detach();
```

### 4. 取消时清理资源

```rust
cx.spawn(|this, mut cx| async move {
    let temp_file = create_temp_file().await;

    // 使用 temp_file...

    // 如果任务被取消，这里不会执行
    // 需要在 cancel 时手动清理
}).detach();
```

## 8.4.9 完整示例：数据加载器

```rust
use std::time::Duration;
use rml::prelude::*;

#[derive(Model)]
#[view]
pub struct DataLoader {
    pub data: Vec<Item>,
    pub is_loading: bool,
    pub error: Option<SharedString>,
    pub last_updated: Option<SharedString>,
    pub auto_refresh: bool,

    load_task: Option<Task<()>>,
    refresh_timer: Option<Task<()>>,
}

impl DataLoader {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            is_loading: false,
            error: None,
            last_updated: None,
            auto_refresh: false,
            load_task: None,
            refresh_timer: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.load_data(cx);
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(task) = self.load_task.take() {
            task.abort();
        }
        if let Some(task) = self.refresh_timer.take() {
            task.abort();
        }
    }

    fn load_data(&mut self, cx: &mut ViewContext<Self>) {
        // 取消之前的加载任务
        if let Some(task) = self.load_task.take() {
            task.abort();
        }

        self.is_loading = true;
        self.error = None;
        cx.notify();

        self.load_task = Some(cx.spawn(|this, mut cx| async move {
            match fetch_data_with_retry(3).await {
                Ok(data) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.data = data;
                        this.is_loading = false;
                        this.error = None;
                        this.last_updated = Some(
                            chrono::Local::now()
                                .format("%Y-%m-%d %H:%M:%S")
                                .to_string()
                                .into()
                        );
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.is_loading = false;
                        this.error = Some(e.into());
                        cx.notify();
                    });
                }
            }
        }));
    }

    fn start_auto_refresh(&mut self, cx: &mut ViewContext<Self>) {
        self.refresh_timer = Some(cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(60))
                    .await;

                let should_refresh = this.update(&mut cx, |this, _cx| {
                    this.auto_refresh
                }).unwrap_or(false);

                if !should_refresh {
                    break;
                }

                let _ = this.update(&mut cx, |this, cx| {
                    this.load_data(cx);
                });
            }
        }));
    }

    #[command]
    pub fn refresh(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.load_data(cx);
    }

    #[command]
    pub fn toggle_auto_refresh(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.auto_refresh = !self.auto_refresh;

        if self.auto_refresh {
            self.start_auto_refresh(cx);
        } else {
            if let Some(task) = self.refresh_timer.take() {
                task.abort();
            }
        }

        cx.notify();
    }
}

async fn fetch_data_with_retry(max_retries: usize) -> Result<Vec<Item>, String> {
    let mut retries = 0;
    loop {
        match fetch_data().await {
            Ok(data) => return Ok(data),
            Err(e) => {
                retries += 1;
                if retries >= max_retries {
                    return Err(format!("加载失败，已重试 {} 次: {}", max_retries, e));
                }
                tokio::time::sleep(Duration::from_secs(retries as u64)).await;
            }
        }
    }
}

async fn fetch_data() -> Result<Vec<Item>, String> {
    // 模拟网络请求
    Ok(Vec::new())
}
```

## 8.4.10 小结

RML 的异步任务管理：

- **启动**：`cx.spawn()` 启动异步任务
- **取消**：保留 `Task`，调用 `task.abort()`
- **错误处理**：用 `Result` 处理错误
- **并发**：`tokio::join!` 并行，`tokio::select!` 竞速
- **定时**：`cx.background_executor().timer()`
- **生命周期**：在 `#[on_unloaded]` 中取消所有任务

下一节 → [8.5 资源管理](./resource-management.md)
