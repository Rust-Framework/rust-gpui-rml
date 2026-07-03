# 11.2 案例研究：从零到一的 Todo 应用

> **本节目标**：用一个中等复杂度的 Todo 应用串起全书知识点，演示完整的项目结构与开发流程。

## 11.2.1 需求分析

我们要构建一个 Todo 应用，功能包括：

- 添加 / 删除 / 编辑待办
- 标记完成 / 取消完成
- 按状态筛选（全部 / 未完成 / 已完成）
- 本地持久化（重启不丢失）
- 统计剩余数量

涉及的知识点：

- MVVM 三层
- 双向绑定与计算属性
- 事件系统
- 生命周期与持久化
- 组件封装
- 样式与主题

## 11.2.2 项目结构

```
todo-app/
├── Cargo.toml
├── build.rs
├── src/
│   ├── main.rs
│   ├── app.rml
│   ├── app.rml.rs
│   ├── models/
│   │   ├── todo.rs
│   │   └── mod.rs
│   ├── services/
│   │   ├── todo_store.rs
│   │   └── mod.rs
│   ├── views/
│   │   └── todo_list/
│   │       ├── todo_list.rml
│   │       ├── todo_list.rml.rs
│   │       └── mod.rs
│   ├── components/
│   │   ├── todo_item/
│   │   │   ├── todo_item.rml
│   │   │   ├── todo_item.rml.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   └── styles/
│       ├── theme.rmlcss
│       └── mod.rs
└── tests/
    └── todo_flow.rs
```

## 11.2.3 Model 层

```rust
// src/models/todo.rs
#[derive(Model, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Todo {
    pub id: u64,
    pub title: SharedString,
    pub completed: bool,
    pub created_at: i64,
}

#[derive(Model, Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub enum TodoFilter {
    All,
    Active,
    Completed,
}

impl TodoFilter {
    pub fn matches(&self, todo: &Todo) -> bool {
        match self {
            TodoFilter::All => true,
            TodoFilter::Active => !todo.completed,
            TodoFilter::Completed => todo.completed,
        }
    }
}
```

Model 是纯数据，无 GPUI 依赖，可独立单测。

## 11.2.4 Service 层

```rust
// src/services/todo_store.rs
use std::path::PathBuf;

pub struct TodoStore { path: PathBuf }

impl TodoStore {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self { path: app_data_dir.join("todos.json") }
    }

    pub async fn load(&self) -> Result<Vec<Todo>> {
        match tokio::fs::read(&self.path).await {
            Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(vec![]),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save(&self, todos: &[Todo]) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(todos)?;
        tokio::fs::write(&self.path, bytes).await?;
        Ok(())
    }
}
```

Service 只做 I/O，返回 `Result`，错误由 ViewModel 处理。

## 11.2.5 ViewModel 层

```rust
// src/views/todo_list/todo_list.rml.rs
use crate::models::*;
use crate::services::TodoStore;

#[derive(IModel)]
pub struct TodoListViewModel {
    pub todos: Vec<Todo>,
    pub filter: TodoFilter,
    pub new_todo_title: SharedString,
    pub is_loading: bool,
    pub error: Option<SharedString>,
    store: Arc<TodoStore>,
    save_task: Option<Task<()>>,
}

impl TodoListViewModel {
    pub fn new(store: Arc<TodoStore>) -> Self {
        Self {
            todos: vec![],
            filter: TodoFilter::All,
            new_todo_title: "".into(),
            is_loading: false,
            error: None,
            store,
            save_task: None,
        }
    }

    // 计算属性：根据筛选条件过滤
    #[computed]
    pub fn visible_todos(&self) -> Vec<Todo> {
        self.todos.iter()
            .filter(|t| self.filter.matches(t))
            .cloned()
            .collect()
    }

    #[computed]
    pub fn remaining_count(&self) -> usize {
        self.todos.iter().filter(|t| !t.completed).count()
    }

    #[computed]
    pub fn can_add(&self) -> bool {
        !self.new_todo_title.trim().is_empty()
    }

    // 命令：添加
    #[command]
    pub fn add_todo(&mut self, _ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
        let title = self.new_todo_title.trim().to_string();
        if title.is_empty() { return; }
        let todo = Todo {
            id: next_id(),
            title: title.into(),
            completed: false,
            created_at: now_ts(),
        };
        self.todos.push(todo);
        self.new_todo_title = "".into();
        cx.notify();
        self.schedule_save(cx);
    }

    // 命令：切换完成
    #[command]
    pub fn toggle(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        let id = ev.data::<u64>();
        if let Some(t) = self.todos.iter_mut().find(|t| t.id == id) {
            t.completed = !t.completed;
            cx.notify();
            self.schedule_save(cx);
        }
    }

    // 命令：删除
    #[command]
    pub fn remove(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        let id = ev.data::<u64>();
        self.todos.retain(|t| t.id != id);
        cx.notify();
        self.schedule_save(cx);
    }

    // 命令：切换筛选
    #[command]
    pub fn set_filter(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.filter = ev.data::<TodoFilter>();
        cx.notify();
    }

    // 命令：清空已完成
    #[command]
    pub fn clear_completed(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.todos.retain(|t| !t.completed);
        cx.notify();
        self.schedule_save(cx);
    }

    // 生命周期：加载持久化数据
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        cx.notify();
        let store = self.store.clone();
        cx.spawn(|this, mut cx| async move {
            let result = store.load().await;
            let _ = this.update(&mut cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(todos) => { this.todos = todos; this.error = None; }
                    Err(e) => { this.error = Some(e.to_string().into()); }
                }
                cx.notify();
            });
        }).detach();
    }

    // 防抖保存：避免每次修改都写盘
    fn schedule_save(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(t) = self.save_task.take() { t.abort(); }
        let store = self.store.clone();
        let todos = self.todos.clone();
        self.save_task = Some(cx.spawn(|_this, mut cx| async move {
            cx.background_executor().timer(Duration::from_millis(500)).await;
            if let Err(e) = store.save(&todos).await {
                log::error!("保存失败: {e}");
            }
        }));
    }
}
```

## 11.2.6 View 层

```html
<!-- src/views/todo_list/todo_list.rml -->
<div class="todo-app">
  <header>
    <h1>待办</h1>
    <p r:if="is_loading">加载中…</p>
    <p r:if="error" class="error">{error}</p>
  </header>

  <form on:submit="add_todo" class="add-form">
    <input r:model="new_todo_title" placeholder="添加待办…" />
    <button type="submit" r:attr:disabled="!can_add">添加</button>
  </form>

  <div class="filters">
    <button r:each="filters" r:key="{$index}"
            r:class:active="filter == $item"
            on:click="set_filter"
            data-value="{$item}">
      {label}
    </button>
  </div>

  <ul class="todo-list">
    <li r:each="visible_todos" r:key="id">
      <TodoItem todo="{$item}" on:toggle="toggle" on:remove="remove" />
    </li>
  </ul>

  <footer r:if="!todos.is_empty()">
    <span>{remaining_count} 项剩余</span>
    <button r:if="remaining_count < todos.len()" on:click="clear_completed">
      清除已完成
    </button>
  </footer>
</div>
```

## 11.2.7 组件封装

```rust
// src/components/todo_item/todo_item.rml.rs
#[derive(IModel)]
pub struct TodoItem {
    pub todo: Todo,
    on_toggle: Option<Command>,
    on_remove: Option<Command>,
}
```

```html
<!-- src/components/todo_item/todo_item.rml -->
<div class="todo-item" r:class:completed="todo.completed">
  <input type="checkbox" r:checked="todo.completed" on:change="on_toggle" />
  <span class="title">{todo.title}</span>
  <button class="remove" on:click="on_remove">✕</button>
</div>
```

组件只接受 props + 触发事件，不直接操作父状态。

## 11.2.8 样式与主题

```css
/* src/styles/theme.rmlcss */
:root {
  --color-primary: #4f46e5;
  --color-danger: #ef4444;
  --color-bg: #ffffff;
  --color-text: #1f2937;
  --color-muted: #6b7280;
  --space-1: 4px;
  --space-2: 8px;
  --space-4: 16px;
}

.todo-app {
  max-width: 480px;
  margin: 0 auto;
  padding: var(--space-4);
  background: var(--color-bg);
  color: var(--color-text);
}

.todo-item.completed .title {
  text-decoration: line-through;
  color: var(--color-muted);
}
```

## 11.2.9 测试

### Model 单测

```rust
#[test]
fn filter_active_excludes_completed() {
    let todo = Todo { completed: true, ..Default::default() };
    assert!(!TodoFilter::Active.matches(&todo));
    assert!(TodoFilter::Completed.matches(&todo));
    assert!(TodoFilter::All.matches(&todo));
}
```

### ViewModel 单测

```rust
#[test]
fn remaining_count_excludes_completed() {
    let vm = TodoListViewModel::new(mock_store());
    let _ = vm.todos = vec![
        Todo { completed: false, ..Default::default() },
        Todo { completed: true, ..Default::default() },
        Todo { completed: false, ..Default::default() },
    ];
    assert_eq!(vm.remaining_count(), 2);
}

#[test]
fn visible_todos_respects_filter() {
    let mut vm = TodoListViewModel::new(mock_store());
    vm.todos = vec![
        Todo { id: 1, completed: false, ..Default::default() },
        Todo { id: 2, completed: true, ..Default::default() },
    ];
    vm.filter = TodoFilter::Active;
    assert_eq!(vm.visible_todos().len(), 1);
    assert_eq!(vm.visible_todos()[0].id, 1);
}
```

### 集成测试

```rust
#[test]
fn add_persists_to_store() {
    let store = Arc::new(TodoStore::new(tempdir()));
    let mut cx = TestContext::new();
    let mut vm = TodoListViewModel::new(store.clone());
    vm.new_todo_title = "买菜".into();
    vm.add_todo(&SubmitEvent::default(), &mut cx.with_view(&mut vm));
    cx.run_until_parked();

    let loaded = store.load().blocking_recv().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].title, "买菜");
}
```

## 11.2.10 开发流程回顾

1. **设计 Model**：纯数据，可序列化
2. **设计 Service**：I/O 隔离，返回 Result
3. **设计 ViewModel**：状态 + 命令 + 计算属性
4. **写模板**：纯结构 + 绑定
5. **抽组件**：复用片段封装
6. **加样式**：主题变量 + 组件样式
7. **写测试**：Model 单测 → ViewModel 单测 → 集成测试
8. **热重载调试**：改模板实时看效果
9. **性能检查**：列表 key、notify 频率

每一步都对应前十章的知识点，案例把散落的知识串成了完整能力。

下一节 → [11.3 迁移指南](./migration-guide.md)
