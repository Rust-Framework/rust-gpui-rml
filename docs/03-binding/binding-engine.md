# 3.6 绑定引擎原理

> **本节目标**：深入理解 RML 绑定引擎的编译期路径解析与运行时订阅机制，掌握性能优化的底层依据。

## 3.6.1 绑定引擎的两阶段

RML 绑定引擎分为两个阶段：

```
┌─────────────────────────────────────────────────────────────────┐
│                    编译期（build.rs）                            │
│                                                                 │
│  1. 解析 .rml 文件为 AST                                         │
│  2. 识别 {field} 插值和绑定属性                                    │
│  3. 检查 field 是否存在于 ViewModel                               │
│  4. 检查类型是否可渲染                                             │
│  5. 生成订阅代码和 Render 实现                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    运行时（RML Runtime）                         │
│                                                                 │
│  1. 视图加载时，建立字段与 UI 元素的订阅关系                        │
│  2. cx.notify() 触发时，通知所有订阅者                             │
│  3. 订阅者重新读取字段值，更新 UI                                  │
│  4. 视图卸载时，自动清理订阅                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 3.6.2 编译期：路径解析

### AST 解析

RML 编译器把 `.rml` 解析为 AST（抽象语法树）：

```
div(class="counter")
├── h1
│   └── text("计数: ")
│   └── interpolation(field="count")  ← 识别为绑定
└── button(onclick="increment")       ← 识别为事件绑定
    └── text("+1")
```

### 绑定路径检查

编译器检查每个 `{field}` 中的 `field` 是否存在于 ViewModel：

```rust
// ViewModel 定义
#[derive(IModel)]
#[component]
pub struct Counter {
    pub count: i32,
}
```

```html
<!-- .rml -->
<p>{count}</p>      <!-- ✅ count 存在 -->
<p>{conut}</p>      <!-- ❌ 拼写错误，编译失败 -->
<p>{missing}</p>    <!-- ❌ 字段不存在，编译失败 -->
```

### 类型检查

编译器检查绑定表达式的返回类型是否可渲染：

```html
<p>{count}</p>              <!-- ✅ i32 实现 Display -->
<p>{user_name}</p>          <!-- ✅ SharedString 可渲染 -->
<p>{todos}</p>              <!-- ❌ Vec<TodoItem> 不可渲染 -->
<p>{todos.len()}</p>        <!-- ✅ usize 实现 Display -->
```

### 生成订阅代码

编译器为每个绑定生成订阅代码：

```rust
// 由 RML 编译器生成（简化示意）
impl Render for Counter {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let count = self.count;  // 读取绑定值

        gpui::div()
            .child(
                gpui::div()
                    .child(gpui::Label::new(format!("计数: {}", count)))
            )
            .child(
                gpui_component::Button::new("+1")
                    .on_click(cx.listener(|this, _, cx| {
                        this.increment(&ClickEvent::default(), cx);
                    }))
            )
    }
}
```

## 3.6.3 运行时：订阅机制

### 订阅的建立

视图首次渲染时，RML Runtime 为每个绑定建立订阅：

```
ViewModel: Counter { count: 0 }
    │
    ├── 订阅者 1: <p>{count}</p>
    ├── 订阅者 2: <p>{count + 1}</p>
    └── 订阅者 3: <button disabled={count == 0}>
```

### 订阅的触发

当 `cx.notify()` 被调用时：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();  // ← 触发所有订阅者
}
```

GPUI 会重新调用 `Render::render`，RML 生成的代码重新读取所有绑定值。

### 订阅的粒度

RML 的订阅粒度是**整个 ViewModel**，而非单个字段。这意味着：

- 任何 `cx.notify()` 都会触发整个视图重新渲染
- 但 GPUI 的 diff 算法会跳过未变化的元素

💡 **设计要点**：虽然订阅粒度是 ViewModel 级别，但 GPUI 的 diff 算法保证了实际 DOM 操作的最小化。对于大多数应用，这种粒度已经足够高效。

### 细粒度优化的未来

RML 路线图包含细粒度订阅优化：

- 编译期分析每个绑定依赖的字段
- 只在依赖字段变化时触发对应绑定的更新
- 进一步减少不必要的重绘

## 3.6.4 计算属性的缓存机制

`#[computed]` 的缓存机制是绑定引擎的核心优化：

### 缓存的结构

```rust
// 编译期生成的缓存结构（简化示意）
struct CounterCache {
    // 计算属性缓存
    completed_count: Option<(usize, Vec<u64>)>,  // (值, 依赖版本号)
    pending_count: Option<(usize, Vec<u64>)>,
    progress: Option<(f64, Vec<u64>)>,
}
```

### 缓存的检查

```rust
#[computed]
pub fn completed_count(&self) -> usize {
    // 检查缓存
    let current_versions = vec![self.todos.version()];
    if let Some((cached_value, cached_versions)) = &self.cache.completed_count {
        if cached_versions == &current_versions {
            return *cached_value;  // 缓存命中
        }
    }

    // 缓存未命中，重新计算
    let result = self.todos.iter().filter(|t| t.done).count();

    // 更新缓存
    self.cache.completed_count = Some((result, current_versions));

    result
}
```

### 缓存的失效

`cx.notify()` 后，所有字段的版本号递增，相关缓存自动失效：

```
todos 版本: v3 → v4
    ↓
completed_count 缓存依赖 todos:v3，失效
    ↓
pending_count 缓存依赖 completed_count，传递失效
    ↓
progress 缓存依赖 completed_count，传递失效
```

## 3.6.5 双向绑定的实现

`model` 指令在编译期生成两个方向的代码：

### 正向：ViewModel → UI

```rust
// 生成的代码（简化）
let value = self.user_name.clone();
input.value(value)
```

### 反向：UI → ViewModel

```rust
// 生成的代码（简化）
input.on_change(cx.listener(|this, value, cx| {
    this.user_name = value;
    cx.notify();
}))
```

两个方向的代码组合，实现双向同步。

## 3.6.6 性能特性总结

| 操作              | 开销                | 优化建议              |
| --------------- | ----------------- | ----------------- |
| 编译期路径检查         | 一次性，编译时           | 无需优化              |
| 视图首次渲染          | 建立订阅，O(绑定数)       | 减少不必要的绑定          |
| `cx.notify()`   | 触发重绘，O(视图大小)      | 批量更新，避免频繁 notify  |
| 计算属性缓存命中        | O(1)，版本号比较        | 用计算属性替代复杂表达式      |
| 计算属性缓存未命中       | O(计算复杂度)          | 避免过深的依赖链          |
| 双向绑定事件          | O(1)，字段赋值 + notify | 避免在 oninput 中做重计算 |

## 3.6.7 性能优化策略

### 策略一：减少 `cx.notify()` 调用

```rust
// ❌ 频繁 notify
#[command]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    for i in 0..100 {
        self.items.push(i);
        cx.notify();  // 100 次 notify
    }
}

// ✅ 批量更新后一次 notify
#[command]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    for i in 0..100 {
        self.items.push(i);
    }
    cx.notify();  // 1 次 notify
}
```

### 策略二：用计算属性替代复杂插值

```html
<!-- ❌ 每次重绘都遍历 -->
<p>{items.iter().filter(|i| i.active).count()}</p>

<!-- ✅ 计算属性缓存 -->
<p>{active_count}</p>
```

### 策略三：用 `once` 避免不必要的订阅

```html
<!-- 只渲染一次，不订阅后续变化 -->
<span once>版本: {version}</span>
```

### 策略四：拆分大视图

```rust
// ❌ 单一巨型 ViewModel，任何 notify 都触发全量重绘
#[derive(IModel)]
#[component]
pub struct MegaView {
    pub header_data: HeaderData,
    pub sidebar_data: SidebarData,
    pub content_data: ContentData,
    pub footer_data: FooterData,
}

// ✅ 拆分为多个子视图，各自独立 notify
pub struct AppView {
    pub header: Entity<HeaderView>,
    pub sidebar: Entity<SidebarView>,
    pub content: Entity<ContentView>,
    pub footer: Entity<FooterView>,
}
```

## 3.6.8 调试绑定引擎

### 查看生成的代码

```bash
cargo rml-expand views::counter
```

输出 `.rml` 生成的完整 Rust 代码，可以检查：

- 绑定是否正确生成
- 计算属性的依赖是否正确识别
- 事件处理是否正确连接

### 绑定追踪日志

在 debug 模式下，RML Runtime 会输出绑定追踪日志：

```bash
RML_LOG=bindings cargo run
```

输出示例：

```
[bindings] Counter::render() called
[bindings]   reading field: count (version: 3)
[bindings]   computed: completed_count (cache hit)
[bindings]   computed: pending_count (cache miss, recalculating)
[bindings] Counter::render() completed in 0.2ms
```

### 性能分析

```bash
RML_PROFILE=1 cargo run
```

输出每个视图的渲染时间、绑定数、缓存命中率等指标。详见 [第 10 章 · 性能优化](../10-advanced/performance.md)。

## 3.6.9 小结

RML 绑定引擎的核心机制：

- **编译期**：路径解析、类型检查、代码生成
- **运行时**：订阅建立、notify 触发、缓存检查
- **计算属性**：自动依赖追踪、版本号缓存
- **双向绑定**：正向读取 + 反向赋值

理解这些机制，你就能：

- 预测绑定的性能表现
- 诊断"UI 不更新"等常见问题
- 编写高性能的 RML 应用

下一章 → [第 4 章 · Code-Behind 业务逻辑](../04-code-behind/INDEX.md)
