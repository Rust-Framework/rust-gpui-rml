# 2.5 插值表达式

> **本节目标**：完整掌握 `{ }` 插值的语法、上下文、表达式边界与常见用法。

## 2.5.1 插值的语法

`{ }` 是 RML 唯一的动态语法。它可以在两个位置使用：

1. **文本位置**：在元素内容中嵌入动态值
2. **属性值位置**：在属性值中嵌入动态值

```html
<!-- 文本插值 -->
<p>欢迎, {user_name}</p>

<!-- 属性插值 -->
<div class={container_class}>动态类名</div>
```

## 2.5.2 插值的上下文

`{ }` 内部的表达式在 ViewModel 的上下文中求值，可以访问：

- ViewModel 的所有 `pub` 字段
- ViewModel 的 `#[computed]` 方法
- Rust 标准库函数和类型

```html
<!-- 访问字段 -->
<p>{user_name}</p>
<p>{count}</p>

<!-- 调用计算属性 -->
<p>{display_message}</p>
<p>{formatted_date}</p>

<!-- 表达式 -->
<p>{count + 1}</p>
<p>{items.len()}</p>
<p>{if is_vip { "VIP" } else { "普通" }}</p>
```

## 2.5.3 表达式的类型

`{ }` 内部可以是任意 Rust 表达式，但结果类型必须可渲染：

### 可渲染类型

| 类型              | 渲染方式       |
| --------------- | ---------- |
| `SharedString`  | 直接显示       |
| `String`        | 直接显示       |
| `&str`          | 直接显示       |
| `i32`、`u32`、`i64`、`u64` | 转为字符串显示    |
| `f32`、`f64`     | 转为字符串显示    |
| `bool`          | 显示 "true" / "false" |
| 任何实现 `Display` 的类型 | 转为字符串显示    |

### 示例

```html
<!-- 字符串 -->
<p>{user_name}</p>
<p>{format!("Hello, {}", name)}</p>

<!-- 数字 -->
<p>{count}</p>
<p>{price * quantity}</p>

<!-- 布尔 -->
<p>{is_logged_in}</p>

<!-- 表达式 -->
<p>{items.iter().filter(|i| i.done).count()}</p>
<p>{if score >= 90 { "A" } else if score >= 80 { "B" } else { "C" }}</p>
```

## 2.5.4 表达式的边界

`{ }` 内部**不支持**以下内容：

### 不支持语句

```html
<!-- ❌ 不能写 let 语句 -->
<p>{ let x = 5; x }</p>
```

### 不支持控制流

```html
<!-- ❌ 不能写 for 循环 -->
<p>{ for i in 0..10 { i } }</p>

<!-- ❌ 不能写 while 循环 -->
<p>{ while cond { do_something() } }</p>
```

### 不支持函数定义

```html
<!-- ❌ 不能定义函数 -->
<p>{ fn helper() -> i32 { 42 } helper() }</p>
```

### 替代方案

复杂逻辑应放在 `#[computed]` 方法中：

```rust
// counter.rml.rs
#[computed]
pub fn display_grade(&self) -> SharedString {
    let grade = if self.score >= 90 {
        "A"
    } else if self.score >= 80 {
        "B"
    } else {
        "C"
    };
    grade.into()
}
```

```html
<!-- counter.rml -->
<p>等级：{display_grade}</p>
```

## 2.5.5 文本插值的混合

`{ }` 可以与静态文本混合使用：

```html
<p>欢迎, {user_name}！你有 {unread_count} 条未读消息。</p>
<p>总价：¥{total_price}（含税 ¥{tax}）</p>
```

多个插值表达式可以在同一行，用静态文本分隔。

## 2.5.6 属性插值

`{ }` 可以作为属性值，动态绑定 ViewModel 数据：

```html
<!-- 动态 class -->
<div class={container_class}>...</div>

<!-- 动态 value -->
<input value={user_name} />

<!-- 动态 disabled -->
<button disabled={is_loading}>提交</button>

<!-- 动态 src -->
<img src={avatar_url} />
```

### 字符串与插值的混合

属性值也可以混合静态字符串和插值：

```html
<div class="card {theme_class}">...</div>
<img src="/assets/{avatar_name}.png" />
```

⚠️ **注意**：混合写法在编译期会拼接为 `format!` 调用。

## 2.5.7 转义

如果需要在文本中显示 `{` 或 `}` 字符本身，需要转义：

```html
<p>使用 {'{'} 和 {'}'} 包裹表达式</p>
```

或者用 `{{` 和 `}}`（借鉴 Rust 的 format! 语法）：

```html
<p>显示字面量 {'{'}value{'}'}</p>
```

## 2.5.8 插值与指令的区别

| 特性   | 插值 `{ }`     | 指令 `if={}` 等 |
| ---- | ------------ | ------------- |
| 用途   | 嵌入动态值        | 控制渲染行为        |
| 位置   | 文本或属性值       | 属性名           |
| 返回值  | 可渲染类型        | 通常为 `bool` 或迭代器 |
| 副作用  | 无（纯表达式）      | 无             |

## 2.5.9 性能考量

每个 `{ }` 插值会在编译期生成一个绑定订阅。当 ViewModel 字段变化时，只有依赖该字段的插值会重新计算。

```html
<!-- 这两个插值独立订阅 count 字段 -->
<p>当前值：{count}</p>
<p>两倍值：{count * 2}</p>
```

💡 **最佳实践**：避免在插值中写复杂表达式，把计算移到 `#[computed]` 方法中。这样计算结果会被缓存，多个插值共享同一个计算结果。

```rust
// ✅ 推荐：计算属性
#[computed]
pub fn double_count(&self) -> i32 {
    self.count * 2
}
```

```html
<p>当前值：{count}</p>
<p>两倍值：{double_count}</p>
```

## 2.5.10 小结

`{ }` 插值是 RML 唯一的动态语法，用于把 ViewModel 数据嵌入 UI：

- **位置**：文本或属性值
- **内容**：任意 Rust 表达式
- **类型**：实现 `Display` 的类型
- **限制**：不支持语句、控制流、函数定义
- **优化**：复杂逻辑用 `#[computed]` 缓存

掌握插值表达式，你就掌握了 `.rml` 文件的全部动态能力。

下一节 → [2.6 完整示例：Todo 应用](./complete-example.md)
