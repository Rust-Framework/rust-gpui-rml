# 2.6 完整示例：Todo 应用

> **本节目标**：用一个完整的 Todo 应用串起本章所有语法点——标签映射、属性系统、指令系统、插值表达式。

## 2.6.1 应用需求

实现一个待办清单应用，功能包括：

- 输入新任务并添加
- 显示任务列表，支持勾选完成
- 显示统计信息（总计、已完成、待办）
- 删除任务
- 空状态提示
- 回车键快速添加

## 2.6.2 完整的 `.rml` 文件

```html
<!-- todo.rml —— 完整的待办清单应用 -->
<div class="todo-app">
    <!-- 应用标题 -->
    <h1 class="app-title">📋 待办清单</h1>

    <!-- 输入区域 -->
    <div class="input-area">
        <input
            class="todo-input"
            type="text"
            placeholder="输入新任务..."
            value={new_todo_text}
            onkeydown="on_enter_key"
        />
        <button class="btn-add" on-click={add_todo}>添加</button>
    </div>

    <!-- 统计信息 -->
    <div class="stats">
        <span>总计: {todos.len()}</span>
        <span>已完成: {completed_count}</span>
        <span>待办: {pending_count}</span>
    </div>

    <!-- 任务列表 -->
    <ul class="todo-list">
        <li each={todo in todos} key={todo.id} class="todo-item">
            <input
                type="checkbox"
                checked={todo.done}
                onchange={toggle_todo, {todo.id}}
            />
            <span class={todo.done ? "done" : ""}>
                {todo.text}
            </span>
            <button class="btn-delete" on-click={delete_todo, {todo.id}}>
                ✕
            </button>
        </li>
    </ul>

    <!-- 空状态 -->
    <div if={todos.is_empty()}>
        <p class="empty-hint">🎉 暂无任务，添加一条吧！</p>
    </div>
</div>
```

## 2.6.3 语法点逐行解析

### 标签映射

```html
<h1>           <!-- → div().text_size(28.0).child(Label::new(...)) -->
<div>          <!-- → div() -->
<input>        <!-- → gpui_component::Input -->
<button>       <!-- → gpui_component::Button -->
<ul>           <!-- → div().flex().flex_col() -->
<li>           <!-- → div() -->
<span>         <!-- → div().inline() -->
<p>            <!-- → div().child(Label::new(...)) -->
```

### 标准属性

```html
class="todo-app"              <!-- 样式类名 -->
type="text"                   <!-- 输入类型 -->
placeholder="输入新任务..."     <!-- 占位符 -->
type="checkbox"               <!-- 复选框类型 -->
```

### 数据绑定属性

```html
value={new_todo_text}         <!-- 双向绑定到 new_todo_text 字段 -->
checked={todo.done}           <!-- 单向绑定到 todo.done -->
class={todo.done ? "done" : ""}  <!-- 动态 class -->
```

### 事件绑定属性

```html
on-click={add_todo}                          <!-- 无参数命令 -->
on-click={toggle_todo, {todo.id}}            <!-- 带参数命令 -->
on-click={delete_todo, {todo.id}}            <!-- 带参数命令 -->
onkeydown="on_enter_key"                    <!-- 方法名绑定 -->
onchange={toggle_todo, {todo.id}}           <!-- change 事件 -->
```

### 指令属性

```html
if={todos.is_empty()}         <!-- 条件渲染：空状态 -->
each={todo in todos}          <!-- 列表渲染：遍历 todos -->
key={todo.id}                 <!-- 列表唯一标识 -->
```

### 插值表达式

```html
{todos.len()}                 <!-- 字段方法调用 -->
{completed_count}             <!-- 计算属性 -->
{pending_count}               <!-- 计算属性 -->
{todo.text}                   <!-- 列表项字段 -->
{todo.id}                     <!-- 列表项字段（用于事件参数） -->
```

## 2.6.4 对应的 `.rml.rs` 文件

```rust
// todo.rml.rs
use rml::prelude::*;

#[derive(IModel)]
pub struct TodoItem {
    pub id: u64,
    pub text: SharedString,
    pub done: bool,
}

#[derive(IModel)]
#[component]
pub struct TodoViewModel {
    pub new_todo_text: SharedString,
    pub todos: Vec<TodoItem>,
    next_id: u64,
}

impl TodoViewModel {
    pub fn new() -> Self {
        Self {
            new_todo_text: SharedString::default(),
            todos: Vec::new(),
            next_id: 1,
        }
    }

    // 计算属性：自动追踪依赖，UI 自动更新
    #[computed]
    pub fn completed_count(&self) -> usize {
        self.todos.iter().filter(|t| t.done).count()
    }

    #[computed]
    pub fn pending_count(&self) -> usize {
        self.todos.len() - self.completed_count()
    }

    // 命令方法：UI 可直接调用
    #[command]
    pub fn add_todo(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.new_todo_text.is_empty() {
            return;
        }
        self.todos.push(TodoItem {
            id: self.next_id,
            text: self.new_todo_text.clone(),
            done: false,
        });
        self.next_id += 1;
        self.new_todo_text = SharedString::default();
        cx.notify();
    }

    #[command]
    pub fn toggle_todo(&mut self, id: u64, _: &ChangeEvent, cx: &mut ViewContext<Self>) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == id) {
            todo.done = !todo.done;
            cx.notify();
        }
    }

    #[command]
    pub fn delete_todo(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.todos.retain(|t| t.id != id);
        cx.notify();
    }

    // 键盘事件处理方法
    pub fn on_enter_key(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        if ev.key == Key::Enter {
            self.add_todo(&ClickEvent::default(), cx);
        }
    }
}
```

## 2.6.5 数据流分析

以"添加任务"为例，追踪完整的数据流：

```
1. 用户在输入框输入"买牛奶"
   ↓
2. value={new_todo_text} 双向绑定
   ↓
3. ViewModel.new_todo_text = "买牛奶"
   ↓
4. 用户按回车键
   ↓
5. onkeydown="on_enter_key" 触发
   ↓
6. on_enter_key 方法检查 Enter 键，调用 add_todo
   ↓
7. add_todo 命令：
   - 检查 new_todo_text 非空
   - 创建 TodoItem，加入 todos
   - 清空 new_todo_text
   - 调用 cx.notify()
   ↓
8. UI 重新渲染：
   - {todos.len()} 更新为 1
   - {completed_count} 更新为 0
   - {pending_count} 更新为 1
   - each={todo in todos} 渲染新列表项
   - 输入框清空（model 双向绑定）
   - if={todos.is_empty()} 不再渲染空状态
```

## 2.6.6 关键设计点

### 1. `next_id` 字段不需要 `pub`

```rust
pub struct TodoViewModel {
    pub new_todo_text: SharedString,  // pub：UI 需要绑定
    pub todos: Vec<TodoItem>,         // pub：UI 需要遍历
    next_id: u64,                     // private：仅内部使用
}
```

不需要在 UI 中访问的字段可以保持 private，遵循最小暴露原则。

### 2. 计算属性自动缓存

```rust
#[computed]
pub fn completed_count(&self) -> usize {
    self.todos.iter().filter(|t| t.done).count()
}
```

`#[computed]` 会自动追踪 `self.todos` 的变化，只有 `todos` 变化时才重新计算。UI 中的 `{completed_count}` 和 `{pending_count}` 共享同一个计算结果。

### 3. 事件参数的传递

```html
<button on-click={delete_todo, {todo.id}}>
```

`{todo.id}` 在编译期被解析为表达式，运行时作为参数传递给 `delete_todo` 命令。

### 4. 双向绑定与命令的协作

```html
<input value={new_todo_text} onkeydown="on_enter_key" />
```

`value={field}` 自动双向绑定处理输入值的同步，`onkeydown` 处理特殊按键（回车提交）。两者协作完成"输入 + 回车提交"的交互模式。

## 2.6.7 扩展练习

尝试为这个 Todo 应用添加以下功能，巩固本章所学：

1. **优先级**：为每个任务添加 `priority` 字段（high/medium/low），用不同颜色显示
2. **过滤**：添加三个按钮（全部/待办/已完成），切换显示哪些任务
3. **编辑**：双击任务进入编辑模式，修改任务文本
4. **批量操作**：添加"全部完成"、"清除已完成"按钮
5. **持久化**：在 `#[on_loaded]` 中加载本地存储，在 `#[on_unloaded]` 中保存

每个扩展练习都会用到本章的多个语法点，是巩固学习的好方法。

## 2.6.8 小结

这个 Todo 应用展示了 RML 的全部核心语法：

- **标签映射**：8 种 HTML 标签的使用
- **标准属性**：`class`、`type`、`placeholder`
- **数据绑定**：`model`、`checked`、动态 `class`
- **事件绑定**：无参命令、带参命令、方法名绑定
- **指令**：`if`、`each`、`key`
- **插值**：字段访问、方法调用、计算属性、三元表达式

掌握这个示例，你就掌握了 `.rml` 文件的全部表达能力。后续章节会在此基础上深入数据绑定、事件系统、组件封装等高级主题。

下一章 → [第 3 章 · 数据绑定系统](../03-binding/INDEX.md)
