# 2.4 指令系统

> **本节目标**：完整掌握 RML 的 10 个指令——`if`、`else`、`each`、`key`、`model`、`show`、`once`、`html`、`ref`、`slot`。这是 RML 区别于 HTML 的核心能力。

## 2.4.1 指令总览

| 指令      | 用途              | 示例                                    |
| ------- | --------------- | ------------------------------------- |
| `if`    | 条件渲染            | `<div if={is_visible}>内容</div>`       |
| `else`  | 条件分支            | `<div else>备选内容</div>`                |
| `each`  | 列表渲染            | `<li each={item in items}>`           |
| `key`   | 列表唯一标识（配合 each） | `<li key={item.id}>`                  |
| `model` | 双向绑定            | `<input model={user_name}>`           |
| `show`  | 显示/隐藏（CSS 控制）   | `<div show={is_active}>`              |
| `once`  | 仅首次渲染           | `<span once>初始化: {init_value}</span>` |
| `html`  | 渲染 HTML 字符串     | `<div html={raw_content}>`            |
| `ref`   | 获取元素引用          | `<div ref="container">`               |
| `slot`  | 组件插槽            | `<my-component><div slot="header">`   |

## 2.4.2 `if` / `else`：条件渲染

`if` 指令根据表达式真假决定是否渲染元素。`else` 必须紧跟在 `if` 元素之后。

### 基础用法

```html
<div if={is_logged_in}>
    欢迎回来，{user_name}！
</div>
<div else>
    请先登录。
</div>
```

### 多分支条件

RML 不支持 `else if`，需要用嵌套 `if` 实现：

```html
<div if={status == "loading"}>
    加载中...
</div>
<div else>
    <div if={status == "success"}>
        加载成功：{data}
    </div>
    <div else>
        加载失败：{error}
    </div>
</div>
```

### `if` 与 `each` 的组合

`if` 可以用在 `each` 内部，过滤列表项：

```html
<ul>
    <li each={todo in todos} key={todo.id}>
        <span if={todo.done} class="done">{todo.text}</span>
        <span else>{todo.text}</span>
    </li>
</ul>
```

⚠️ **注意**：`if` 不能直接过滤 `each` 的迭代。如果需要过滤列表，应在 `#[computed]` 中处理：

```rust
#[computed]
pub fn pending_todos(&self) -> Vec<&TodoItem> {
    self.todos.iter().filter(|t| !t.done).collect()
}
```

```html
<li each={todo in pending_todos} key={todo.id}>...</li>
```

### `if` vs `show`

| 指令     | 行为                  | 适用场景           |
| ------ | ------------------- | -------------- |
| `if`   | 条件为假时**不创建**元素      | 切换成本高、初始隐藏的元素  |
| `show` | 条件为假时**创建但隐藏**元素（CSS） | 频繁切换、需要保留状态的元素 |

```html
<!-- if：条件为假时元素不存在 -->
<div if={show_detail}>详细信息</div>

<!-- show：条件为假时元素存在但不可见 -->
<div show={show_detail}>详细信息</div>
```

💡 **最佳实践**：默认用 `if`。只有在需要频繁切换且元素内部状态需要保留时（如表单输入），才用 `show`。

## 2.4.3 `each` / `key`：列表渲染

`each` 指令用于遍历列表，`key` 指令提供唯一标识以优化渲染。

### 基础用法

```html
<ul>
    <li each={todo in todos} key={todo.id}>
        {todo.text}
    </li>
</ul>
```

### 索引访问

`each` 支持 `index, item` 语法获取索引：

```html
<ol>
    <li each={index, user in users} key={user.id}>
        第 {index + 1} 名：{user.name}
    </li>
</ol>
```

### 嵌套列表

```html
<div each={category in categories} key={category.id}>
    <h2>{category.name}</h2>
    <ul>
        <li each={item in category.items} key={item.id}>
            {item.name}
        </li>
    </ul>
</div>
```

### `key` 的重要性

`key` 是 RML 优化列表渲染的关键。它让框架能够：

- 识别哪些项是新增、删除、移动的
- 复用已有 DOM 节点，避免重建
- 保持元素内部状态（如输入框焦点）

```html
<!-- ✅ 正确：使用稳定的唯一 ID -->
<li each={todo in todos} key={todo.id}>

<!-- ❌ 错误：使用索引作为 key，列表变化时会导致状态错乱 -->
<li each={index, todo in todos} key={index}>
```

⚠️ **注意**：`key` 的值必须是**稳定且唯一**的。不要用数组索引作为 key，除非列表永远不会重排。

### 空列表处理

```html
<ul if={!todos.is_empty()}>
    <li each={todo in todos} key={todo.id}>{todo.text}</li>
</ul>
<div else>
    <p>🎉 暂无任务</p>
</div>
```

## 2.4.4 `model`：双向绑定

`model` 指令实现双向数据绑定，等价于 `value={field}` + `oninput={update_field}`。

### 基础用法

```html
<input model={user_name} placeholder="输入姓名" />
```

等价于：

```html
<input
    value={user_name}
    oninput={update_user_name}
/>
```

其中 `update_user_name` 是 RML 自动生成的命令，把输入值赋给 `user_name` 字段。

### 适用标签

`model` 主要用于表单元素：

| 标签          | 绑定字段类型        |
| ----------- | ------------- |
| `<input>`   | `SharedString`、`i32`、`f64` 等 |
| `<textarea>` | `SharedString` |
| `<input type="checkbox">` | `bool`        |
| `<input type="number">` | `i32`、`f64`   |

### 双向绑定的字段要求

被 `model` 绑定的字段必须是 `pub` 且类型可赋值：

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub user_name: SharedString,  // ✅ 可以双向绑定
    pub age: i32,                 // ✅ 可以双向绑定
    pub remember_me: bool,        // ✅ 可以双向绑定
}
```

详见 [第 3 章 · 双向绑定](../03-binding/two-way-binding.md)。

## 2.4.5 `show`：显示/隐藏

`show` 指令通过 CSS 控制元素的显示/隐藏，元素始终存在于 DOM 中。

```html
<div show={is_loading} class="loading-spinner">
    加载中...
</div>
```

等价于设置 `style="display: {is_loading ? 'flex' : 'none'}"`。

### `show` 与 `if` 的选择

| 场景                  | 推荐指令   |
| ------------------- | ------ |
| 切换频率低，初始可能隐藏        | `if`   |
| 频繁切换，元素内部有状态需要保留    | `show` |
| 条件为假时元素不应占用内存       | `if`   |
| 切换时需要保持滚动位置、焦点等状态   | `show` |

## 2.4.6 `once`：仅首次渲染

`once` 指令让元素只在首次渲染时计算绑定值，之后不再更新。

```html
<div once>
    初始化时间: {chrono::Local::now().format("%H:%M:%S")}
</div>
```

### 适用场景

- 显示初始化时的快照值（如启动时间、版本号）
- 静态内容，避免不必要的重渲染
- 性能优化：减少绑定订阅

⚠️ **注意**：`once` 内的绑定值在首次渲染后不再变化，即使 ViewModel 字段更新。

## 2.4.7 `html`：渲染 HTML 字符串

`html` 指令用于渲染包含 HTML 标签的字符串。

```html
<div html={raw_content}></div>
```

其中 `raw_content` 是 `SharedString`，内容会被解析为 HTML 并渲染。

### 安全提示

⚠️ **注意**：`html` 指令会直接渲染原始 HTML，**不会进行 XSS 过滤**。仅用于渲染可信内容（如本地生成的富文本）。对于用户输入，必须先进行转义。

```rust
// ❌ 危险：直接渲染用户输入
#[computed]
pub fn user_bio_html(&self) -> SharedString {
    self.user.bio.clone()  // 可能包含恶意脚本
}

// ✅ 安全：转义后再渲染
#[computed]
pub fn user_bio_text(&self) -> SharedString {
    self.user.bio.replace('<', "&lt;").replace('>', "&gt;").into()
}
```

## 2.4.8 `ref`：元素引用

`ref` 指令为元素指定一个引用名，在 `.rml.rs` 中可以通过 `#[element]` 字段访问。

```html
<input ref="username_input" model={user_name} />
<button ref="submit_btn" onclick={submit}>提交</button>
```

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub user_name: SharedString,
    #[element]
    pub username_input: ElementRef<Input>,
    #[element]
    pub submit_btn: ElementRef<Button>,
}

impl MyView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 自动聚焦输入框
        self.username_input.focus(cx);
    }
}
```

详见 [第 4 章 · 元素引用](../04-code-behind/element-ref.md)。

## 2.4.9 `slot`：组件插槽

`slot` 指令用于自定义组件的内容分发。详见 [第 6 章 · 插槽机制](../06-components/slots.md)。

```html
<Card>
    <div slot="header">自定义头部</div>
    <p>卡片主体内容</p>
    <div slot="footer">自定义底部</div>
</Card>
```

## 2.4.10 指令的组合

多个指令可以同时用在同一个元素上：

```html
<li
    each={todo in todos}
    key={todo.id}
    if={!todo.done}
    class={todo.priority == "high" ? "urgent" : ""}
    onclick={toggle_todo, {todo.id}}
>
    {todo.text}
</li>
```

**指令执行顺序**：

1. `each` —— 决定元素是否重复
2. `if` —— 决定每次迭代是否渲染
3. `key` —— 为每次迭代提供唯一标识
4. 其他指令和属性 —— 应用于最终元素

## 2.4.11 小结

RML 的 10 个指令是 HTML 之上的扩展能力：

- **条件**：`if`、`else`、`show`
- **循环**：`each`、`key`
- **绑定**：`model`
- **优化**：`once`
- **内容**：`html`、`slot`
- **引用**：`ref`

记住这些指令的语义和适用场景，你就能在 `.rml` 中表达任何动态 UI 需求。

下一节 → [2.5 插值表达式](./interpolation.md)
