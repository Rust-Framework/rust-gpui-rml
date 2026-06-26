# 3.2 单向绑定

> **本节目标**：完整掌握单向绑定的全部用法——文本绑定、属性绑定、表达式绑定，以及单向绑定的性能特性。

## 3.2.1 单向绑定的定义

单向绑定是数据从 ViewModel 流向 View 的绑定方式。ViewModel 字段变化时，View 自动更新；View 不会修改 ViewModel 字段。

```html
<!-- 单向绑定：ViewModel → View -->
<p>{user_name}</p>
```

## 3.2.2 文本绑定

最常见的单向绑定形式，把 ViewModel 字段值显示为文本。

### 基础用法

```html
<p>欢迎, {user_name}</p>
<p>当前计数: {count}</p>
<p>完成进度: {completed_count}/{total_count}</p>
```

### 字段方法调用

```html
<p>任务总数: {todos.len()}</p>
<p>是否为空: {todos.is_empty()}</p>
<p>第一个任务: {todos.first().map(|t| t.text.as_ref()).unwrap_or("无")}</p>
```

### 嵌套字段

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub user: User,
}

#[derive(Model)]
pub struct User {
    pub profile: UserProfile,
}

#[derive(Model)]
pub struct UserProfile {
    pub name: SharedString,
    pub email: SharedString,
}
```

```html
<p>姓名: {user.profile.name}</p>
<p>邮箱: {user.profile.email}</p>
```

## 3.2.3 属性绑定

把 ViewModel 字段值绑定到元素的属性。

### class 绑定

```html
<!-- 静态 class -->
<div class="card">...</div>

<!-- 动态 class：整个 class 来自 ViewModel -->
<div class={container_class}>...</div>

<!-- 混合 class：静态 + 动态 -->
<div class="card {theme_class}">...</div>

<!-- 条件 class：三元表达式 -->
<div class={is_active ? "active" : "inactive"}>...</div>
```

### value 绑定

```html
<input value={user_name} />
<input value={format!("¥{:.2}", price)} />
```

⚠️ **注意**：`value={field}` 是单向绑定，输入框的值会显示 `field`，但用户输入不会更新 `field`。要实现输入同步，需要用 `model={field}` 双向绑定。

### disabled / checked 绑定

```html
<button disabled={is_loading}>提交</button>
<input type="checkbox" checked={remember_me} />
<input type="text" readonly={is_read_only} />
```

### src / href 绑定

```html
<img src={avatar_url} alt="头像" />
<a href={detail_url}>查看详情</a>
```

## 3.2.4 表达式绑定

`{ }` 内部可以是任意 Rust 表达式，只要结果是可渲染类型。

### 算术表达式

```html
<p>总价: {price * quantity}</p>
<p>折扣后: {price * quantity * (1.0 - discount)}</p>
<p>平均: {total / count}</p>
```

### 逻辑表达式

```html
<p>{is_vip ? "VIP 用户" : "普通用户"}</p>
<p>{count > 10 ? "很多" : "不多"}</p>
```

### 方法调用

```html
<p>{todos.iter().filter(|t| t.done).count()}</p>
<p>{users.iter().map(|u| u.score).sum::<i32>()}</p>
<p>{format!("{} ({})", user.name, user.id)}</p>
```

### if-else 表达式

```html
<p>{
    if score >= 90 { "优秀" }
    else if score >= 80 { "良好" }
    else if score >= 60 { "及格" }
    else { "不及格" }
}</p>
```

⚠️ **注意**：复杂表达式应放在 `#[computed]` 方法中，避免在 `.rml` 中写过多逻辑。详见 [3.4 计算属性](./computed.md)。

## 3.2.5 列表项绑定

在 `each` 指令内，绑定路径是列表项本身：

```html
<ul>
    <li each={todo in todos} key={todo.id}>
        <span>{todo.text}</span>
        <span>{todo.done ? "✓" : "○"}</span>
    </li>
</ul>
```

`todo` 是迭代变量，可以访问其所有 `pub` 字段。

### 索引绑定

```html
<ol>
    <li each={index, user in users} key={user.id}>
        第 {index + 1} 名：{user.name}
    </li>
</ol>
```

## 3.2.6 单向绑定的更新机制

单向绑定的更新依赖 `cx.notify()`：

```rust
#[command]
pub fn update_name(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.user_name = "新名字".into();
    cx.notify();  // ← 触发所有依赖 user_name 的绑定更新
}
```

### 更新的粒度

RML 的绑定是**细粒度**的：只有依赖变化字段的 UI 元素会重新渲染。

```html
<!-- 这两个绑定独立订阅不同字段 -->
<p>{user_name}</p>           <!-- 订阅 user_name -->
<p>{count}</p>               <!-- 订阅 count -->

<!-- 修改 user_name 只会更新第一个 <p> -->
```

### 批量更新

同一帧内的多次 `cx.notify()` 会被合并为一次重绘：

```rust
#[command]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.user_name = "新名字".into();
    cx.notify();  // 不会立即重绘

    self.count += 1;
    cx.notify();  // 不会立即重绘

    // 函数返回后，GPUI 在下一帧统一重绘
}
```

## 3.2.7 单向绑定的性能优化

### 1. 用计算属性替代复杂表达式

```html
<!-- ❌ 每次重绘都重新计算 -->
<p>{items.iter().filter(|i| i.done).count()}</p>

<!-- ✅ 计算属性缓存结果 -->
<p>{completed_count}</p>
```

### 2. 用 `once` 避免不必要的订阅

```html
<!-- 只在首次渲染时计算，之后不再更新 -->
<span once>启动时间: {start_time}</span>
```

### 3. 避免在插值中创建临时对象

```html
<!-- ❌ 每次重绘创建新的 String -->
<p>{format!("Hello, {}", name)}</p>

<!-- ✅ 用计算属性返回 SharedString -->
<p>{greeting}</p>
```

```rust
#[computed]
pub fn greeting(&self) -> SharedString {
    format!("Hello, {}", self.name).into()
}
```

## 3.2.8 单向绑定的限制

### 不能在 View 中修改 ViewModel

```html
<!-- ❌ 单向绑定是只读的 -->
<input value={user_name} />  <!-- 用户输入不会更新 user_name -->
```

要实现双向同步，必须用 `model` 指令：

```html
<!-- ✅ 双向绑定 -->
<input model={user_name} />
```

### 不能在插值中调用命令

```html
<!-- ❌ 插值是纯表达式，不能有副作用 -->
<p>{submit()}</p>  <!-- 编译错误 -->
```

## 3.2.9 调试单向绑定

### 检查绑定路径

如果绑定不显示值，检查：

1. 字段是否 `pub`
2. 字段名是否拼写正确
3. 字段类型是否可渲染

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub user_name: SharedString,  // ✅ pub
    private_field: i32,           // ❌ 不能绑定
}
```

### 检查 notify 调用

如果绑定值不更新，检查：

1. 修改字段后是否调用了 `cx.notify()`
2. `cx.notify()` 是否在正确的 `ViewContext` 上调用

```rust
#[command]
pub fn update(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();  // ← 必须调用
}
```

### 用 `cargo rml-expand` 查看生成代码

```bash
cargo rml-expand views::counter
```

这会输出 `.rml` 生成的 Rust 代码，可以检查绑定是否正确生成。详见 [第 10 章 · 调试技巧](../10-advanced/debugging.md)。

## 3.2.10 小结

单向绑定是 RML 最常用的绑定形式：

- **文本绑定**：`{field}` 显示字段值
- **属性绑定**：`class={}`、`value={}`、`disabled={}` 等
- **表达式绑定**：`{field + 1}`、`{field.method()}` 等
- **列表项绑定**：在 `each` 内访问迭代变量字段

记住：单向绑定是只读的，ViewModel 变化时 View 更新，但 View 不能修改 ViewModel。要实现双向同步，用 `model` 指令。

下一节 → [3.3 双向绑定](./two-way-binding.md)
