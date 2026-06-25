# 9.2 MVVM 模式实践

> **本节目标**：把 Model / ViewModel / View 三层的协作契约讲透，建立可复用的 MVVM 落地范式。

## 9.2.1 MVVM 在 RML 中的映射

RML 的 MVVM 不是口号，而是被编译器和绑定引擎强制约束的运行时契约：

```
┌──────────────────────────────────────────────────────────────┐
│                      RML 的 MVVM 数据流                          │
│                                                              │
│   ┌─────────┐   不可变引用    ┌──────────────┐   绑定     ┌──────┐
│   │  Model  │ ─────────────▶ │  ViewModel   │ ────────▶ │ View │
│   │ 纯数据   │               │ 持状态 + 命令  │           │ .rml │
│   └─────────┘               └──────────────┘           └──────┘
│         ▲                         ▲                       │    │
│         │  反序列化                  │  cx.notify()          │    │
│         │                         │                       ▼    │
│   ┌──────────┐              ┌──────────────┐       ┌──────────┐
│   │ Service  │ ──结果回写──▶ │  ViewModel   │ ◀──── │  用户事件 │
│   │ I/O 层   │              │  改状态       │  命令  │  点击/输入 │
│   └──────────┘              └──────────────┘       └──────────┘
└──────────────────────────────────────────────────────────────┘
```

| 层          | RML 中的体现                                       | 是否依赖 GPUI          |
| ---------- | ----------------------------------------------- | ------------------- |
| **Model**  | `#[derive(Model)]` 的纯数据结构体                      | 否（可独立编译、单测）         |
| **ViewModel** | `#[derive(Model)]` + `#[command]` + `#[computed]` | 是（持有 `ViewContext` 时的方法签名） |
| **View**   | `.rml` 模板 + 编译期生成的 `Render` 实现                  | 是（由框架生成，开发者不手写）     |

## 9.2.2 Model 层契约

### 不可变优先

Model 应当被设计为**值语义**：状态变化通过“替换整个 Model”完成，而非原地修改字段。

```rust
// ✅ 推荐：Model 是 Clone 的纯数据
#[derive(Model, Clone, Debug)]
pub struct TodoItem {
    pub id: u64,
    pub title: SharedString,
    pub completed: bool,
}

// 修改时整体替换
let updated = TodoItem { completed: true, ..item.clone() };
```

### 无 GPUI 依赖

Model crate 应当能脱离 GPUI 独立编译。这样它可被 CLI、测试、服务端复用。

```rust
// ❌ 反例：Model 里出现 gpui 类型
#[derive(Model)]
pub struct BadModel {
    pub cx: ViewContext<Self>, // 编译错误：Model 不能持有 context
    pub color: gpui::Rgb,      // 应改用领域类型（如 hex 字符串）
}
```

### 提供纯函数式派生

Model 可以提供只读计算方法，但不持有状态：

```rust
impl TodoItem {
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        self.due_date.map(|d| d < now).unwrap_or(false)
    }
}
```

## 9.2.3 ViewModel 层契约

### 持有 Model，不暴露内部

ViewModel 持有 Model，但对外只暴露**视图需要的字段**。这层翻译是 ViewModel 的核心价值。

```rust
#[derive(Model)]
pub struct TodoListViewModel {
    // 内部持有完整 Model
    items: Vec<TodoItem>,
    filter: TodoFilter,
    // 视图本地状态
    is_loading: bool,
    error: Option<SharedString>,
}

impl TodoListViewModel {
    // ✅ 计算属性：把 Model + 视图状态翻译成视图可消费的形态
    #[computed]
    pub fn visible_items(&self) -> Vec<TodoItem> {
        self.items.iter()
            .filter(|i| self.filter.matches(i))
            .cloned()
            .collect()
    }

    #[computed]
    pub fn remaining_count(&self) -> usize {
        self.items.iter().filter(|i| !i.completed).count()
    }

    // ✅ 命令：唯一允许修改状态的地方
    #[command]
    pub fn toggle(&mut self, id: u64, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.completed = !item.completed;
            cx.notify();
        }
    }
}
```

### 命令的契约

`#[command]` 方法必须遵守：

1. **签名固定**：`fn(&mut self, ev: &Event, cx: &mut ViewContext<Self>)` 或 `fn(&mut self, cx: &mut ViewContext<Self>)`
2. **只改状态**：不返回值、不直接操作 DOM、不调用其他视图的命令
3. **必须 notify**：任何状态变化后调用 `cx.notify()`，否则 View 不刷新
4. **可重入**：同一命令可能被快速连续触发，状态机要能正确处理

### 异步命令的模式

异步命令遵循“立即改状态 → spawn 任务 → 完成后回写”三段式：

```rust
#[command]
pub fn refresh(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 1. 立即改状态：给用户即时反馈
    self.is_loading = true;
    self.error = None;
    cx.notify();

    // 2. spawn 异步任务
    cx.spawn(|this, mut cx| async move {
        let result = todo_service::fetch_all(&cx).await;
        // 3. 完成后回写
        let _ = this.update(&mut cx, |this, cx| {
            this.is_loading = false;
            match result {
                Ok(items) => { this.items = items; this.error = None; }
                Err(e) => { this.error = Some(e.to_string().into()); }
            }
            cx.notify();
        });
    }).detach();
}
```

## 9.2.4 View 层契约

### View 是 ViewModel 的投影

`.rml` 模板中只能出现：

- ViewModel 的 `pub` 字段
- ViewModel 的 `#[computed]` 方法
- ViewModel 的 `#[command]` 方法名

模板**不应**直接访问 Service、全局变量、或 ViewModel 的私有字段。

### View 无状态

`.rml` 模板不持有状态。所有状态在 ViewModel 中，模板只是状态的函数：

```
View(state) = UI
```

这是热重载可行的根本原因——状态在 ViewModel，模板可随时替换。

### View 无副作用

模板中的事件绑定只是“声明”：告诉框架“点击这个按钮时调用 `login` 命令”。实际副作用发生在命令里，不在模板里。

## 9.2.5 三层协作的完整示例

需求：一个搜索框，输入时防抖 300ms，调用 API，显示结果列表。

**Model**：

```rust
#[derive(Model, Clone, Debug)]
pub struct SearchResult {
    pub id: u64,
    pub title: SharedString,
    pub url: SharedString,
}
```

**Service**（不属于 MVVM，但 ViewModel 依赖它）：

```rust
pub async fn search(query: &str, cx: &mut AsyncApp) -> Result<Vec<SearchResult>> { ... }
```

**ViewModel**：

```rust
#[derive(Model)]
pub struct SearchViewModel {
    pub query: SharedString,
    pub results: Vec<SearchResult>,
    pub is_searching: bool,
    pub error: Option<SharedString>,
    #[element] input: ElementRef,
}

impl SearchViewModel {
    #[computed]
    pub fn has_results(&self) -> bool {
        !self.is_searching && !self.results.is_empty()
    }

    #[command]
    pub fn on_input(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
        self.query = ev.value.clone();
        self.is_searching = true;
        cx.notify();
        // 防抖：每次输入取消上一次任务
        if let Some(t) = self.debounce_task.take() { t.abort(); }
        let query = self.query.clone();
        self.debounce_task = Some(cx.spawn(|this, mut cx| async move {
            cx.background_executor().timer(Duration::from_millis(300)).await;
            let result = search_service::search(&query, &mut cx).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.is_searching = false;
                match result {
                    Ok(r) => { this.results = r; this.error = None; }
                    Err(e) => { this.error = Some(e.to_string().into()); }
                }
                cx.notify();
            });
        }));
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.input.focus(cx);
    }
}
```

**View（.rml）**：

```html
<div class="search">
  <input ref="input" r:model="query" on:input="on_input" placeholder="搜索…" />
  <p r:if="is_searching">搜索中…</p>
  <p r:if="error" class="error">{error}</p>
  <ul r:if="has_results">
    <li r:each="results" r:key="id">
      <a href="{url}">{title}</a>
    </li>
  </ul>
</div>
```

三层各司其职：Model 可被服务端复用、ViewModel 可在无 UI 环境下单测、View 可被设计师独立修改。

## 9.2.6 MVVM 的常见误用

| 误用                          | 正确做法                          |
| --------------------------- | ----------------------------- |
| ViewModel 直接调用 `div().child(...)` | 永远不构造 GPUI 树，交给 `.rml`        |
| Model 持有 `ViewContext`      | Model 不依赖 GPUI               |
| View 里写 `{fetch_data()}`    | 数据获取放命令，模板只读状态                |
| 一个 ViewModel 服务多个视图         | 每个视图一个 ViewModel；共享状态用 Context |
| 命令方法返回 `bool` 表示成功          | 命令无返回值；通过状态字段反馈               |
| 在 `#[computed]` 里修改状态       | 计算属性必须纯函数，只读                  |

## 9.2.7 何时偏离 MVVM

MVVM 是默认范式，但下列场景可适度偏离：

- **一次性脚本工具**：直接用 GPUI 链式调用更快，不必上 RML
- **极简静态界面**：没有状态、没有交互，可省略 ViewModel，只用 `.rml` + 静态数据
- **性能热点**：绑定有微小开销，帧率敏感的动画层可降级为命令式 GPUI

偏离时请用注释说明理由，避免被后续维护者误以为是规范。

下一节 → [9.3 SOLID 原则在 RML 中的落地](./solid-principles.md)
