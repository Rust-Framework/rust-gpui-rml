# 5.5 防抖与节流

> **本节目标**：掌握高频事件的性能优化策略——防抖（Debounce）与节流（Throttle）。

## 5.5.1 为什么需要防抖与节流

某些事件会高频触发，如果每次都执行重计算或网络请求，会导致性能问题：

| 事件           | 触发频率           | 潜在问题           |
| ------------ | -------------- | -------------- |
| `oninput`    | 每次按键          | 频繁搜索请求         |
| `onmousemove` | 鼠标移动时持续触发     | 频繁重绘           |
| `onwheel`    | 滚轮滚动时持续触发     | 频繁滚动计算         |
| `onresize`   | 窗口大小变化时持续触发   | 频繁布局计算         |

防抖与节流是两种常用的优化策略：

| 策略   | 行为                  | 适用场景           |
| ---- | ------------------- | -------------- |
| 防抖   | 延迟执行，期间再次触发则重新计时    | 搜索输入、窗口大小变化    |
| 节流   | 固定频率执行，期间触发被忽略      | 鼠标移动、滚动        |

## 5.5.2 防抖（Debounce）

防抖的核心思想：**事件触发后延迟执行，如果在延迟期间再次触发，则重新计时**。

```
事件触发 ──── 等待 300ms ──── 执行
                                ↑
                    期间再次触发 → 重新等待 300ms
```

### 适用场景

- 搜索框输入：用户停止输入后再搜索
- 窗口大小变化：用户停止调整后再重新布局
- 表单验证：用户停止输入后再验证

### 实现

```rust
use std::time::Duration;
use gpui::Timer;

#[derive(IModel)]
#[component]
pub struct SearchView {
    pub search_text: SharedString,
    pub search_results: Vec<SearchResult>,
    pub is_searching: bool,

    debounce_task: Option<Task<()>>,
}

impl SearchView {
    #[command]
    pub fn on_search_input(&mut self, ev: &InputEvent, cx: &mut ViewContext<Self>) {
        self.search_text = ev.value.clone();
        cx.notify();

        // 取消之前的防抖任务
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }

        // 启动新的防抖任务
        let query = self.search_text.clone();
        self.debounce_task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor().timer(Duration::from_millis(300)).await;

            let _ = this.update(&mut cx, |this, cx| {
                this.perform_search(&query, cx);
            });
        }));
    }

    fn perform_search(&mut self, query: &str, cx: &mut ViewContext<Self>) {
        if query.is_empty() {
            self.search_results.clear();
            cx.notify();
            return;
        }

        self.is_searching = true;
        cx.notify();

        let query = query.to_string();
        cx.spawn(|this, mut cx| async move {
            let results = fetch_search_results(&query).await;

            let _ = this.update(&mut cx, |this, cx| {
                this.search_results = results;
                this.is_searching = false;
                cx.notify();
            });
        }).detach();
    }
}
```

### 防抖的效果

```
用户输入 "hello"
    ↓
触发 oninput("h")
    启动防抖任务（300ms 后执行）
    ↓
触发 oninput("he")（100ms 后）
    取消之前的任务
    启动新的防抖任务（300ms 后执行）
    ↓
触发 oninput("hel")（100ms 后）
    取消之前的任务
    启动新的防抖任务（300ms 后执行）
    ↓
... 持续输入 ...
    ↓
用户停止输入
    ↓
300ms 后，最后一次防抖任务执行
    执行搜索 "hello"
```

## 5.5.3 节流（Throttle）

节流的核心思想：**固定频率执行，期间触发被忽略**。

```
事件触发 ──── 立即执行 ──── 100ms 内忽略 ──── 100ms 后可再次执行
```

### 适用场景

- 鼠标移动：限制重绘频率
- 滚动事件：限制滚动计算频率
- 拖拽事件：限制位置更新频率

### 实现

```rust
use std::time::{Duration, Instant};

#[derive(IModel)]
#[component]
pub struct DragView {
    pub drag_position: Point<Pixels>,
    pub is_dragging: bool,

    last_throttle_time: Option<Instant>,
}

impl DragView {
    #[command]
    pub fn on_mouse_move(&mut self, ev: &MouseEvent, cx: &mut ViewContext<Self>) {
        if !self.is_dragging {
            return;
        }

        // 节流：每 16ms（约 60fps）最多执行一次
        let now = Instant::now();
        let should_execute = self
            .last_throttle_time
            .map(|last| now.duration_since(last) >= Duration::from_millis(16))
            .unwrap_or(true);

        if should_execute {
            self.drag_position = ev.position;
            self.last_throttle_time = Some(now);
            cx.notify();
        }
    }

    #[command]
    pub fn on_mouse_down(&mut self, ev: &MouseEvent, cx: &mut ViewContext<Self>) {
        self.is_dragging = true;
        self.drag_position = ev.position;
        self.last_throttle_time = None;
        cx.notify();
    }

    #[command]
    pub fn on_mouse_up(&mut self, _: &MouseEvent, cx: &mut ViewContext<Self>) {
        self.is_dragging = false;
        self.last_throttle_time = None;
        cx.notify();
    }
}
```

### 节流的效果

```
鼠标移动事件持续触发（每 1ms 一次）
    ↓
0ms:  执行，更新位置
1ms:  忽略
2ms:  忽略
...
15ms: 忽略
16ms: 执行，更新位置
17ms: 忽略
...
32ms: 执行，更新位置
...
```

## 5.5.4 防抖 vs 节流的选择

| 场景              | 推荐策略 | 原因                  |
| --------------- | ---- | ------------------- |
| 搜索框输入           | 防抖   | 用户停止输入后再搜索，减少请求     |
| 窗口大小变化          | 防抖   | 用户停止调整后再布局          |
| 表单字段验证          | 防抖   | 用户停止输入后再验证          |
| 鼠标移动（拖拽）        | 节流   | 保持流畅响应，限制频率         |
| 滚动事件            | 节流   | 限制滚动计算频率            |
| 按钮防连点           | 节流   | 防止快速多次触发            |

### 决策树

```
是否需要"停止后才执行"？
    ├── 是 → 防抖
    └── 否 → 是否需要"固定频率执行"？
              ├── 是 → 节流
              └── 否 → 无需优化
```

## 5.5.5 封装防抖/节流工具

### 防抖工具

```rust
use std::time::Duration;

pub struct Debounce {
    delay: Duration,
    task: Option<Task<()>>,
}

impl Debounce {
    pub fn new(delay: Duration) -> Self {
        Self {
            delay,
            task: None,
        }
    }

    pub fn trigger<F>(&mut self, cx: &mut ViewContext<impl Model>, f: F)
    where
        F: FnOnce(&mut ViewContext<impl Model>) + Send + 'static,
    {
        if let Some(task) = self.task.take() {
            task.abort();
        }

        let delay = self.delay;
        self.task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor().timer(delay).await;
            let _ = this.update(&mut cx, |_, cx| f(cx));
        }));
    }
}
```

### 使用防抖工具

```rust
#[derive(IModel)]
#[component]
pub struct SearchView {
    pub search_text: SharedString,
    search_debounce: Debounce,
}

impl SearchView {
    pub fn new() -> Self {
        Self {
            search_text: SharedString::default(),
            search_debounce: Debounce::new(Duration::from_millis(300)),
        }
    }

    #[command]
    pub fn on_search_input(&mut self, ev: &InputEvent, cx: &mut ViewContext<Self>) {
        self.search_text = ev.value.clone();
        cx.notify();

        let query = self.search_text.clone();
        self.search_debounce.trigger(cx, move |cx| {
            // 这里的 this 是 SearchView
            // this.perform_search(&query, cx);
        });
    }
}
```

## 5.5.6 完整示例：搜索框

```rust
use std::time::Duration;
use rml::prelude::*;

#[derive(IModel)]
#[component]
pub struct SearchView {
    pub search_text: SharedString,
    pub search_results: Vec<SearchResult>,
    pub is_searching: bool,
    pub search_error: Option<SharedString>,

    debounce_task: Option<Task<()>>,
}

impl SearchView {
    pub fn new() -> Self {
        Self {
            search_text: SharedString::default(),
            search_results: Vec::new(),
            is_searching: false,
            search_error: None,
            debounce_task: None,
        }
    }

    #[command]
    pub fn on_search_input(&mut self, ev: &InputEvent, cx: &mut ViewContext<Self>) {
        self.search_text = ev.value.clone();
        self.search_error = None;
        cx.notify();

        // 取消之前的防抖任务
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }

        // 空查询立即清空结果
        if self.search_text.is_empty() {
            self.search_results.clear();
            cx.notify();
            return;
        }

        // 启动防抖搜索
        let query = self.search_text.clone();
        self.debounce_task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;

            let _ = this.update(&mut cx, |this, cx| {
                this.perform_search(&query, cx);
            });
        }));
    }

    fn perform_search(&mut self, query: &str, cx: &mut ViewContext<Self>) {
        self.is_searching = true;
        cx.notify();

        let query = query.to_string();
        cx.spawn(|this, mut cx| async move {
            match fetch_search_results(&query).await {
                Ok(results) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.search_results = results;
                        this.is_searching = false;
                        this.search_error = None;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.search_error = Some(e.to_string().into());
                        this.is_searching = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }

    #[command]
    pub fn clear_search(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(task) = self.debounce_task.take() {
            task.abort();
        }
        self.search_text = SharedString::default();
        self.search_results.clear();
        self.search_error = None;
        self.is_searching = false;
        cx.notify();
    }
}

async fn fetch_search_results(query: &str) -> Result<Vec<SearchResult>, String> {
    // 模拟异步搜索
    Ok(Vec::new())
}

#[derive(IModel)]
pub struct SearchResult {
    pub id: u64,
    pub title: SharedString,
}
```

```html
<!-- views/search.rml -->
<div class="search-view">
    <div class="search-input-area">
        <input
            value={search_text}
            oninput={on_search_input}
            placeholder="搜索..."
        />
        <button if={!search_text.is_empty()} on-click={clear_search}>✕</button>
    </div>

    <div if={is_searching} class="loading">
        搜索中...
    </div>

    <div if={search_error.is_some()} class="error">
        {search_error}
    </div>

    <ul if={!is_searching && search_error.is_none()} class="results">
        <li each={result in search_results} key={result.id}>
            {result.title}
        </li>
    </ul>

    <div if={!is_searching && search_results.is_empty() && !search_text.is_empty()} class="empty">
        未找到相关结果
    </div>
</div>
```

## 5.5.7 防抖与节流的注意事项

### 1. 异步任务的取消

防抖必须在每次触发时取消之前的任务，否则会有多个任务同时执行：

```rust
// ✅ 取消之前的任务
if let Some(task) = self.debounce_task.take() {
    task.abort();
}

// ❌ 不取消，多个任务会同时执行
self.debounce_task = Some(cx.spawn(...));
```

### 2. 组件卸载时清理

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    // 取消未完成的防抖任务
    if let Some(task) = self.debounce_task.take() {
        task.abort();
    }
}
```

### 3. 避免过度防抖

```rust
// ❌ 防抖时间过长，影响响应性
Debounce::new(Duration::from_secs(2))

// ✅ 合理的防抖时间
Debounce::new(Duration::from_millis(300))  // 搜索
Debounce::new(Duration::from_millis(100))  // 验证
```

### 4. 节流的初始执行

节流的第一次触发应该立即执行，避免延迟感：

```rust
let should_execute = self
    .last_throttle_time
    .map(|last| now.duration_since(last) >= Duration::from_millis(16))
    .unwrap_or(true);  // 第一次立即执行
```

## 5.5.8 小结

防抖与节流是高频事件的核心优化策略：

| 策略   | 行为              | 适用场景           |
| ---- | --------------- | -------------- |
| 防抖   | 延迟执行，期间触发重新计时   | 搜索、验证、窗口调整     |
| 节流   | 固定频率执行，期间触发忽略   | 鼠标移动、滚动、拖拽     |

实现要点：

- 防抖：用 `Task` + `timer`，每次触发取消之前的任务
- 节流：用 `Instant` 记录上次执行时间，判断是否超过间隔
- 组件卸载时清理未完成的任务

下一章 → [第 6 章 · 组件系统](../06-components/INDEX.md)
