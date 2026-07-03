# 3.4 计算属性

> **本节目标**：完整掌握 `#[computed]` 宏的依赖追踪、缓存机制、使用场景与最佳实践。

## 3.4.1 计算属性的定义

计算属性是基于其他字段自动计算的派生值。它有两个核心特性：

1. **自动依赖追踪**：自动追踪计算过程中访问的字段
2. **结果缓存**：依赖字段未变化时，不重新计算，直接返回缓存值

```rust
#[derive(IModel)]
#[component]
pub struct TodoViewModel {
    pub todos: Vec<TodoItem>,
}

impl TodoViewModel {
    #[computed]
    pub fn completed_count(&self) -> usize {
        // 自动追踪依赖：self.todos
        self.todos.iter().filter(|t| t.done).count()
    }
}
```

```html
<!-- 像访问字段一样访问计算属性 -->
<p>已完成: {completed_count}</p>
```

## 3.4.2 为什么需要计算属性

### 问题：在插值中写复杂表达式

```html
<!-- ❌ 每次重绘都重新计算 -->
<p>已完成: {todos.iter().filter(|t| t.done).count()}</p>
<p>待办: {todos.iter().filter(|t| !t.done).count()}</p>
<p>进度: {todos.iter().filter(|t| t.done).count() as f64 / todos.len() as f64 * 100.0}%</p>
```

三个插值都遍历 `todos`，重复计算。

### 解决：用计算属性缓存

```rust
#[computed]
pub fn completed_count(&self) -> usize {
    self.todos.iter().filter(|t| t.done).count()
}

#[computed]
pub fn pending_count(&self) -> usize {
    self.todos.len() - self.completed_count()
}

#[computed]
pub fn progress(&self) -> f64 {
    if self.todos.is_empty() {
        0.0
    } else {
        self.completed_count() as f64 / self.todos.len() as f64 * 100.0
    }
}
```

```html
<!-- ✅ 计算属性缓存，多个插值共享结果 -->
<p>已完成: {completed_count}</p>
<p>待办: {pending_count}</p>
<p>进度: {progress}%</p>
```

## 3.4.3 依赖追踪机制

`#[computed]` 在编译期分析方法体，自动识别访问的 `self` 字段：

```rust
#[computed]
pub fn full_name(&self) -> SharedString {
    // 访问了 self.first_name 和 self.last_name
    format!("{} {}", self.first_name, self.last_name).into()
}
```

编译期生成的代码（简化示意）：

```rust
pub fn full_name(&self) -> SharedString {
    // 检查缓存：如果 first_name 和 last_name 未变化，返回缓存
    if self.computed_cache.full_name.is_valid(&[&self.first_name, &self.last_name]) {
        return self.computed_cache.full_name.value.clone();
    }

    // 重新计算
    let result = format!("{} {}", self.first_name, self.last_name).into();

    // 更新缓存
    self.computed_cache.full_name.update(result.clone(), &[&self.first_name, &self.last_name]);

    result
}
```

### 依赖的类型

计算属性可以依赖：

- **字段**：`self.field_name`
- **其他计算属性**：`self.other_computed()`
- **字段方法**：`self.todos.len()`（依赖 `todos`）

```rust
#[computed]
pub fn summary(&self) -> SharedString {
    // 依赖 todos（通过 len()）和 completed_count（计算属性）
    format!("共 {} 项，已完成 {} 项", self.todos.len(), self.completed_count()).into()
}
```

## 3.4.4 计算属性的缓存

### 缓存的有效性

缓存基于依赖字段的"版本号"。每次 `cx.notify()` 后，相关字段的版本号递增，缓存失效。

```
字段版本号：
  todos: v3
  completed_count 缓存依赖 todos:v3

调用 cx.notify() 修改 todos：
  todos: v4
  completed_count 缓存失效（todos 版本变化）

下次访问 completed_count：
  重新计算，更新缓存为 todos:v4
```

### 缓存的生命周期

缓存与 ViewModel 实例绑定，ViewModel 销毁时缓存自动释放。

## 3.4.5 计算属性的使用场景

### 场景一：派生显示值

```rust
#[computed]
pub fn display_status(&self) -> SharedString {
    match self.status {
        Status::Loading => "加载中...".into(),
        Status::Success => "加载成功".into(),
        Status::Error(ref msg) => format!("错误: {}", msg).into(),
    }
}
```

```html
<p>{display_status}</p>
```

### 场景二：过滤列表

```rust
#[computed]
pub fn pending_todos(&self) -> Vec<&TodoItem> {
    self.todos.iter().filter(|t| !t.done).collect()
}
```

```html
<ul>
    <li each={todo in pending_todos} key={todo.id}>{todo.text}</li>
</ul>
```

### 场景三：格式化输出

```rust
#[computed]
pub fn formatted_price(&self) -> SharedString {
    format!("¥{:.2}", self.price).into()
}

#[computed]
pub fn formatted_date(&self) -> SharedString {
    self.timestamp.format("%Y-%m-%d %H:%M").to_string().into()
}
```

```html
<p>价格: {formatted_price}</p>
<p>时间: {formatted_date}</p>
```

### 场景四：条件判断

```rust
#[computed]
pub fn can_submit(&self) -> bool {
    !self.user_name.is_empty() && !self.password.is_empty() && !self.is_submitting
}
```

```html
<button disabled={!can_submit} onclick={submit}>提交</button>
```

### 场景五：聚合统计

```rust
#[computed]
pub fn total_price(&self) -> f64 {
    self.cart.iter().map(|item| item.price * item.quantity as f64).sum()
}

#[computed]
pub fn total_count(&self) -> usize {
    self.cart.iter().map(|item| item.quantity).sum()
}
```

```html
<p>共 {total_count} 件商品，总计 {total_price}</p>
```

## 3.4.6 计算属性的规则

### 规则一：必须是只读方法

计算属性不能修改 `self`：

```rust
#[computed]
pub fn bad_computed(&mut self) -> i32 {  // ❌ 不能是 &mut self
    self.count += 1;
    self.count
}

#[computed]
pub fn good_computed(&self) -> i32 {  // ✅ 必须是 &self
    self.count + 1
}
```

### 规则二：不能有参数

```rust
#[computed]
pub fn with_param(&self, x: i32) -> i32 {  // ❌ 不能有参数
    self.count + x
}
```

如果需要参数化计算，应该写成普通方法，在插值中调用：

```html
<p>{calculate_with(5)}</p>
```

但这样会失去缓存。更好的做法是为每个常用参数组合定义计算属性：

```rust
#[computed]
pub fn count_plus_5(&self) -> i32 {
    self.count + 5
}
```

### 规则三：返回值必须可克隆

计算属性的返回值会被缓存，因此必须实现 `Clone`：

```rust
#[computed]
pub fn items_ref(&self) -> &Vec<Item> {  // ❌ 返回引用无法缓存
    &self.items
}

#[computed]
pub fn items_count(&self) -> usize {  // ✅ usize 实现 Clone
    self.items.len()
}
```

## 3.4.7 计算属性的依赖链

计算属性可以依赖其他计算属性，形成依赖链：

```rust
#[computed]
pub fn completed_count(&self) -> usize {
    self.todos.iter().filter(|t| t.done).count()
}

#[computed]
pub fn pending_count(&self) -> usize {
    self.todos.len() - self.completed_count()  // 依赖 completed_count
}

#[computed]
pub fn progress_percent(&self) -> f64 {
    if self.todos.is_empty() {
        0.0
    } else {
        self.completed_count() as f64 / self.todos.len() as f64 * 100.0
    }
}
```

依赖链的缓存传播：

```
todos 变化
    ↓
completed_count 缓存失效
    ↓
pending_count 缓存失效（依赖 completed_count）
    ↓
progress_percent 缓存失效（依赖 completed_count）
```

## 3.4.8 计算属性 vs 普通方法

| 特性     | 计算属性 `#[computed]` | 普通方法           |
| ------ | ------------------- | --------------- |
| 缓存     | ✅ 自动缓存              | ❌ 每次调用都执行       |
| 依赖追踪   | ✅ 自动                | ❌ 无             |
| 参数     | ❌ 无参数               | ✅ 可以有参数         |
| 修改 self | ❌ 只读                | ✅ 可以修改（需 `&mut self`） |
| 调用方式   | `{name}`（像字段）       | `{name(args)}`（像方法） |

### 选择建议

- **纯计算、依赖明确**：用 `#[computed]`
- **需要参数**：用普通方法
- **需要修改状态**：用 `#[command]`
- **简单的一次性计算**：直接在插值中写表达式

## 3.4.9 调试计算属性

### 查看依赖

用 `cargo rml-expand` 查看生成的代码，可以看到计算属性的依赖列表：

```bash
cargo rml-expand views::todo
```

输出会包含：

```rust
// #[computed] completed_count
// dependencies: [todos]
pub fn completed_count(&self) -> usize {
    self.todos.iter().filter(|t| t.done).count()
}
```

### 验证缓存

在计算属性中加日志，验证是否被缓存：

```rust
#[computed]
pub fn expensive_calc(&self) -> f64 {
    log::info!("expensive_calc 被调用");
    // ... 复杂计算
}
```

如果日志只在依赖变化时打印，说明缓存生效。

## 3.4.10 计算属性的最佳实践

### 1. 保持计算属性纯函数

```rust
// ✅ 纯函数：只读 self，无副作用
#[computed]
pub fn display_name(&self) -> SharedString {
    format!("{} ({})", self.name, self.id).into()
}

// ❌ 有副作用：修改了其他状态
#[computed]
pub fn bad_computed(&self) -> i32 {
    self.cache.hit_count += 1;  // 副作用！
    self.count + 1
}
```

### 2. 避免过深的依赖链

```rust
// ❌ 依赖链过深，缓存失效传播成本高
#[computed] pub fn a(&self) -> i32 { self.x + 1 }
#[computed] pub fn b(&self) -> i32 { self.a() + 1 }
#[computed] pub fn c(&self) -> i32 { self.b() + 1 }
#[computed] pub fn d(&self) -> i32 { self.c() + 1 }

// ✅ 直接计算，减少中间层
#[computed]
pub fn d(&self) -> i32 {
    self.x + 4
}
```

### 3. 计算属性命名用名词

```rust
// ✅ 名词：像字段
#[computed] pub fn completed_count(&self) -> usize { ... }
#[computed] pub fn display_name(&self) -> SharedString { ... }

// ❌ 动词：像方法
#[computed] pub fn calculate_count(&self) -> usize { ... }
#[computed] pub fn get_name(&self) -> SharedString { ... }
```

## 3.4.11 小结

计算属性是 RML 绑定系统的核心能力：

- **自动依赖追踪**：编译期分析方法体，识别依赖字段
- **结果缓存**：依赖未变化时直接返回缓存
- **像字段访问**：在 `.rml` 中用 `{name}` 访问，无需括号
- **只读纯函数**：不修改 `self`，无副作用

最佳实践：**任何在 `.rml` 中出现的复杂表达式，都应提取为计算属性**。这既提升性能（缓存），又提升可读性（命名）。

下一节 → [3.5 值转换器](./converter.md)
