# 8.5 资源管理

> **本节目标**：掌握 RML 应用中各类资源的管理——文件、网络、订阅、缓存的生命周期管理。

## 8.5.1 资源的类型

RML 应用中需要管理的资源：

| 资源类型        | 获取方式              | 释放方式              |
| ----------- | ------------------ | ------------------ |
| 文件句柄        | `File::open()`     | `file.close()`     |
| 网络连接        | `connect()`        | `conn.close()`     |
| 数据库连接       | `connect()`        | `conn.close()`     |
| 订阅          | `cx.subscribe()`   | `sub.unsubscribe()`|
| 定时器         | `cx.spawn()`       | `task.abort()`     |
| 缓存          | 自定义                | 手动清理               |
| WebSocket   | `connect()`        | `ws.close()`       |

## 8.5.2 资源管理的原则

### 1. 谁获取谁释放

```rust
#[derive(Model)]
#[component]
pub struct DataView {
    file_handle: Option<FileHandle>,  // 我获取的，我负责释放
}

impl DataView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.file_handle = Some(FileHandle::open("data.txt"));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(handle) = self.file_handle.take() {
            handle.close();  // 我释放
        }
    }
}
```

### 2. 及时释放

```rust
// ✅ 用完立即释放
pub fn process_file(&mut self, cx: &mut ViewContext<Self>) {
    let handle = FileHandle::open("data.txt");
    let data = handle.read_all();
    handle.close();  // 用完立即关闭

    self.data = data;
    cx.notify();
}

// ❌ 持有不释放
pub fn process_file(&mut self, cx: &mut ViewContext<Self>) {
    self.file_handle = Some(FileHandle::open("data.txt"));
    // 一直持有，直到视图卸载才释放
}
```

### 3. 错误时释放

```rust
pub fn load_data(&mut self, cx: &mut ViewContext<Self>) {
    let handle = FileHandle::open("data.txt")
        .expect("无法打开文件");

    let result = handle.read_all();

    // 无论成功失败都关闭
    handle.close();

    match result {
        Ok(data) => {
            self.data = data;
        }
        Err(e) => {
            self.error = Some(e.to_string().into());
        }
    }
    cx.notify();
}
```

## 8.5.3 文件资源管理

### 读取文件

```rust
use std::fs::File;
use std::io::Read;

#[derive(Model)]
#[component]
pub struct FileView {
    pub content: SharedString,
    pub is_loading: bool,
    pub error: Option<SharedString>,
}

impl FileView {
    #[command]
    pub fn open_file(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
        let path = ev.value.clone();

        self.is_loading = true;
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.content = content.into();
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
}
```

### 写入文件

```rust
#[command]
pub fn save_file(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    let path = self.file_path.clone();
    let content = self.content.clone();

    cx.spawn(|this, mut cx| async move {
        match tokio::fs::write(&path, content.as_bytes()).await {
            Ok(_) => {
                let _ = this.update(&mut cx, |this, cx| {
                    this.is_saved = true;
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
```

## 8.5.4 网络资源管理

### HTTP 请求

```rust
#[derive(Model)]
#[component]
pub struct ApiView {
    pub data: Option<ApiResponse>,
    pub is_loading: bool,
    pub error: Option<SharedString>,
}

impl ApiView {
    #[command]
    pub fn fetch_data(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        self.error = None;
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            match reqwest::get("https://api.example.com/data").await {
                Ok(response) => {
                    match response.json::<ApiResponse>().await {
                        Ok(data) => {
                            let _ = this.update(&mut cx, |this, cx| {
                                this.data = Some(data);
                                this.is_loading = false;
                                cx.notify();
                            });
                        }
                        Err(e) => {
                            let _ = this.update(&mut cx, |this, cx| {
                                this.error = Some(format!("解析失败: {}", e).into());
                                this.is_loading = false;
                                cx.notify();
                            });
                        }
                    }
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(format!("请求失败: {}", e).into());
                        this.is_loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
}
```

### WebSocket 连接

```rust
#[derive(Model)]
#[component]
pub struct ChatView {
    pub messages: Vec<Message>,
    pub is_connected: bool,
    websocket: Option<WebSocketConnection>,
}

impl ChatView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.connect_websocket(cx);
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 关闭 WebSocket 连接
        if let Some(ws) = self.websocket.take() {
            ws.close();
        }
    }

    fn connect_websocket(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            match WebSocketConnection::connect("wss://chat.example.com").await {
                Ok(ws) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.websocket = Some(ws);
                        this.is_connected = true;
                        cx.notify();
                    });
                }
                Err(_) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.is_connected = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
}
```

## 8.5.5 数据库连接管理

```rust
#[derive(Model)]
pub struct DatabaseManager {
    connection: Option<DbConnection>,
}

impl DatabaseManager {
    pub fn new() -> Self {
        Self {
            connection: None,
        }
    }

    pub fn connect(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            match DbConnection::connect("database.url").await {
                Ok(conn) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.connection = Some(conn);
                        cx.notify();
                    });
                }
                Err(e) => {
                    // 处理错误
                }
            }
        }).detach();
    }

    pub fn disconnect(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(conn) = self.connection.take() {
            conn.close();
            cx.notify();
        }
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(conn) = self.connection.take() {
            conn.close();
        }
    }
}
```

## 8.5.6 订阅管理

### 订阅全局状态

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

        self.subscription = Some(cx.subscribe(
            &service,
            |this, _, event, cx| {
                this.handle_notification(event, cx);
            },
        ));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(sub) = self.subscription.take() {
            sub.unsubscribe();
        }
    }

    fn handle_notification(&mut self, event: &NotificationEvent, cx: &mut ViewContext<Self>) {
        self.notifications.push(event.notification.clone());
        cx.notify();
    }
}
```

### 订阅多个源

```rust
#[derive(Model)]
#[component]
pub struct DashboardView {
    pub user_updates: Vec<UserUpdate>,
    pub system_alerts: Vec<SystemAlert>,
    user_subscription: Option<Subscription>,
    system_subscription: Option<Subscription>,
}

impl DashboardView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        let user_service = cx.global::<Entity<UserService>>();
        let system_service = cx.global::<Entity<SystemService>>();

        self.user_subscription = Some(cx.subscribe(
            &user_service,
            |this, _, event, cx| {
                this.user_updates.push(event.clone());
                cx.notify();
            },
        ));

        self.system_subscription = Some(cx.subscribe(
            &system_service,
            |this, _, event, cx| {
                this.system_alerts.push(event.clone());
                cx.notify();
            },
        ));
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(sub) = self.user_subscription.take() {
            sub.unsubscribe();
        }
        if let Some(sub) = self.system_subscription.take() {
            sub.unsubscribe();
        }
    }
}
```

## 8.5.7 缓存管理

### 简单缓存

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Model)]
pub struct Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    entries: HashMap<K, CacheEntry<V>>,
    ttl: Duration,
}

struct CacheEntry<V> {
    value: V,
    created_at: Instant,
}

impl<K, V> Cache<K, V>
where
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            ttl,
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.entries.get(key).and_then(|entry| {
            if entry.created_at.elapsed() < self.ttl {
                Some(entry.value.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: K, value: V) {
        self.entries.insert(
            key,
            CacheEntry {
                value,
                created_at: Instant::now(),
            },
        );
    }

    pub fn cleanup_expired(&mut self) {
        self.entries.retain(|_, entry| entry.created_at.elapsed() < self.ttl);
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}
```

### 在视图中使用缓存

```rust
#[derive(Model)]
#[component]
pub struct CachedDataView {
    pub data: Option<Data>,
    pub is_loading: bool,
    cache: Entity<Cache<String, Data>>,
}

impl CachedDataView {
    pub fn new(cache: Entity<Cache<String, Data>>) -> Self {
        Self {
            data: None,
            is_loading: false,
            cache,
        }
    }

    #[command]
    pub fn load_data(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        let cache_key = "data_key".to_string();

        // 先查缓存
        let cached = self.cache.read(cx).get(&cache_key);
        if let Some(data) = cached {
            self.data = Some(data);
            cx.notify();
            return;
        }

        // 缓存未命中，从服务器加载
        self.is_loading = true;
        cx.notify();

        let cache = self.cache.clone();
        cx.spawn(|this, mut cx| async move {
            match fetch_data_from_api().await {
                Ok(data) => {
                    // 写入缓存
                    cache.update(&mut cx, |cache, cx| {
                        cache.set(cache_key, data.clone());
                        cx.notify();
                    });

                    let _ = this.update(&mut cx, |this, cx| {
                        this.data = Some(data);
                        this.is_loading = false;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.is_loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
}
```

## 8.5.8 资源管理的最佳实践

### 1. 使用 RAII

```rust
// ✅ RAII：资源在 Drop 时自动释放
pub struct FileHandle {
    file: Option<File>,
}

impl FileHandle {
    pub fn open(path: &str) -> std::io::Result<Self> {
        Ok(Self {
            file: Some(File::open(path)?),
        })
    }

    pub fn read_all(&mut self) -> std::io::Result<String> {
        let mut content = String::new();
        self.file.as_mut().unwrap().read_to_string(&mut content)?;
        Ok(content)
    }
}

impl Drop for FileHandle {
    fn drop(&mut self) {
        // 自动关闭文件
        if let Some(file) = self.file.take() {
            drop(file);
        }
    }
}
```

### 2. 集中管理资源

```rust
#[derive(Model)]
pub struct ResourceManager {
    connections: HashMap<String, Connection>,
    file_handles: HashMap<String, FileHandle>,
    subscriptions: Vec<Subscription>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            file_handles: HashMap::new(),
            subscriptions: Vec::new(),
        }
    }

    pub fn get_connection(&mut self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    pub fn add_connection(&mut self, name: String, conn: Connection) {
        self.connections.insert(name, conn);
    }

    pub fn close_all(&mut self) {
        for (_, conn) in self.connections.drain() {
            conn.close();
        }
        for (_, handle) in self.file_handles.drain() {
            // handle 在 Drop 时自动关闭
        }
        for sub in self.subscriptions.drain() {
            sub.unsubscribe();
        }
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        self.close_all();
    }
}
```

### 3. 资源池

```rust
pub struct ConnectionPool {
    connections: Vec<Connection>,
    max_size: usize,
}

impl ConnectionPool {
    pub fn new(max_size: usize) -> Self {
        Self {
            connections: Vec::new(),
            max_size,
        }
    }

    pub fn get(&mut self) -> Option<Connection> {
        self.connections.pop().or_else(|| {
            if self.connections.len() < self.max_size {
                Some(Connection::new())
            } else {
                None
            }
        })
    }

    pub fn return_connection(&mut self, conn: Connection) {
        if self.connections.len() < self.max_size {
            self.connections.push(conn);
        } else {
            conn.close();
        }
    }
}
```

## 8.5.9 完整示例：图片查看器

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rml::prelude::*;

#[derive(Model)]
pub struct ImageCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
}

struct CacheEntry {
    image: ImageData,
    last_used: Instant,
}

impl ImageCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
        }
    }

    pub fn get(&mut self, url: &str) -> Option<ImageData> {
        if let Some(entry) = self.entries.get_mut(url) {
            entry.last_used = Instant::now();
            Some(entry.image.clone())
        } else {
            None
        }
    }

    pub fn set(&mut self, url: String, image: ImageData) {
        if self.entries.len() >= self.max_size {
            self.evict_lru();
        }
        self.entries.insert(url, CacheEntry {
            image,
            last_used: Instant::now(),
        });
    }

    fn evict_lru(&mut self) {
        if let Some((lru_url, _)) = self.entries
            .iter()
            .min_by_key(|(_, entry)| entry.last_used)
            .map(|(k, _)| (k.clone(), ()))
        {
            self.entries.remove(&lru_url);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Model)]
#[component]
pub struct ImageViewer {
    pub current_image: Option<ImageData>,
    pub is_loading: bool,
    pub error: Option<SharedString>,
    pub image_url: SharedString,

    cache: Entity<ImageCache>,
    load_task: Option<Task<()>>,
}

impl ImageViewer {
    pub fn new(cache: Entity<ImageCache>) -> Self {
        Self {
            current_image: None,
            is_loading: false,
            error: None,
            image_url: SharedString::default(),
            cache,
            load_task: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        if !self.image_url.is_empty() {
            self.load_image(cx);
        }
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        if let Some(task) = self.load_task.take() {
            task.abort();
        }
    }

    fn load_image(&mut self, cx: &mut ViewContext<Self>) {
        // 取消之前的加载任务
        if let Some(task) = self.load_task.take() {
            task.abort();
        }

        let url = self.image_url.to_string();

        // 先查缓存
        let cached = self.cache.update(cx, |cache, cx| {
            cache.get(&url)
        });

        if let Some(image) = cached {
            self.current_image = Some(image);
            self.is_loading = false;
            cx.notify();
            return;
        }

        // 缓存未命中，从网络加载
        self.is_loading = true;
        self.error = None;
        cx.notify();

        let cache = self.cache.clone();
        self.load_task = Some(cx.spawn(|this, mut cx| async move {
            match fetch_image(&url).await {
                Ok(image) => {
                    // 写入缓存
                    cache.update(&mut cx, |cache, cx| {
                        cache.set(url.clone(), image.clone());
                        cx.notify();
                    });

                    let _ = this.update(&mut cx, |this, cx| {
                        this.current_image = Some(image);
                        this.is_loading = false;
                        this.error = None;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.is_loading = false;
                        this.error = Some(e.to_string().into());
                        cx.notify();
                    });
                }
            }
        }));
    }

    #[command]
    pub fn change_image(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
        self.image_url = ev.value.clone();
        self.load_image(cx);
    }

    #[command]
    pub fn reload(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.load_image(cx);
    }

    #[command]
    pub fn clear_cache(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.cache.update(cx, |cache, cx| {
            cache.clear();
            cx.notify();
        });
    }
}

async fn fetch_image(url: &str) -> Result<ImageData, String> {
    // 模拟图片下载
    Ok(ImageData::default())
}

#[derive(Model, Clone, Default)]
pub struct ImageData {
    pub data: Vec<u8>,
}
```

## 8.5.10 小结

RML 的资源管理：

- **原则**：谁获取谁释放、及时释放、错误时释放
- **文件**：用 `tokio::fs` 异步读写
- **网络**：HTTP 请求、WebSocket 连接
- **数据库**：连接池管理
- **订阅**：`cx.subscribe()` + `sub.unsubscribe()`
- **缓存**：LRU 缓存，定期清理
- **最佳实践**：RAII、集中管理、资源池

下一章 → [第 9 章 · 架构与最佳实践](../09-architecture/INDEX.md)
