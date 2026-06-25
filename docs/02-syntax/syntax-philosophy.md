# 2.1 语法哲学

> **核心策略**：HTML 优先，增强属性。让 HTML 开发者感到亲切，让 Rust 开发者感到简洁。

## 2.1.1 三条语法原则

RML 的语法设计遵循三条原则，它们是所有语法决策的判据：

### 原则一：使用标准 HTML 标签

RML 不发明新标签。所有结构标签都是 HTML 开发者熟悉的：

```html
<div>、<span>、<p>、<h1>~<h6>、<button>、<input>、<textarea>、
<ul>、<ol>、<li>、<img>、<a>、<label>
```

这意味着：

- ✅ 任何 HTML 编辑器都能高亮 `.rml` 文件
- ✅ 任何 XML 工具都能格式化、lint `.rml` 文件
- ✅ Web 开发者打开 `.rml` 第一秒就能读懂结构

### 原则二：使用标准 HTML 属性

RML 不发明新属性名。所有通用属性都是 HTML 标准：

```html
class、id、style、placeholder、type、src、href、
value、checked、disabled、maxlength
```

### 原则三：极简指令，无框架前缀

RML 通过极简指令扩展能力，**无任何框架前缀**：

```html
<!-- ✅ RML：无前缀，简洁 -->
<div if={is_visible}>内容</div>
<li each={item in items} key={item.id}>

<!-- ❌ 反例：如果加了前缀，就违背了 HTML 优先原则 -->
<div r:if={is_visible}>内容</div>
<li v-for="item in items" :key="item.id">
```

## 2.1.2 与 Vue / React 语法的对比

| 能力     | Vue           | React          | **RML**                  |
| ------ | ------------- | -------------- | ------------------------ |
| 条件渲染   | `v-if`        | `{cond && <div/>}` | **`if={cond}`**          |
| 列表渲染   | `v-for`       | `{items.map(...)}` | **`each={item in items}`** |
| 双向绑定   | `v-model`     | `value + onChange` | **`model={field}`**      |
| 显示/隐藏  | `v-show`      | `style={{display}}` | **`show={cond}`**        |
| 事件绑定   | `@click`      | `onClick`      | **`onclick={fn}`**       |
| 插值     | `{{ var }}`   | `{var}`        | **`{var}`**              |
| 框架前缀   | `v-`、`@`、`:`  | 无              | **无**                    |

💡 **设计要点**：RML 选择**无前缀**路线，是因为它的文件是独立的 `.rml`，不会与 HTML 混淆。前缀在 Vue 中是为了与原生 HTML 属性区分，在 RML 中没有必要。

## 2.1.3 增强属性：`{ }` 插值

RML 在 HTML 之上只增加一种语法：`{ }` 插值表达式。它借鉴自 React/Vue，用于把 ViewModel 数据嵌入到 UI 中。

```html
<!-- 文本插值 -->
<p>欢迎, {user_name}</p>

<!-- 属性插值 -->
<div class={container_class}>动态类名</div>
<input value={user_name}>

<!-- 表达式插值 -->
<p>总计: {items.len()}</p>
<p>{if is_vip { "VIP" } else { "普通" }}</p>
```

`{ }` 内部是 Rust 表达式，编译期会进行类型检查。详见 [2.5 插值表达式](./interpolation.md)。

## 2.1.4 语法边界：什么不能做

为了保持 HTML 优先的简洁性，RML 明确**不支持**以下写法：

```html
<!-- ❌ 不支持在 .rml 中写 Rust 语句 -->
<div>
    { let x = 5; x }
</div>

<!-- ❌ 不支持在 .rml 中定义函数 -->
<div>
    { fn helper() -> i32 { 42 } helper() }
</div>

<!-- ❌ 不支持在 .rml 中写复杂的逻辑控制流 -->
<div>
    { for i in 0..10 { /* 不能在插值里循环 */ } }
</div>
```

这些限制是**有意为之**：

- `.rml` 是声明文件，不是脚本文件
- 复杂逻辑应该放在 `.rml.rs` 的 `#[computed]` 方法中
- 这强制保持了"逻辑与表现分离"的原则

如果需要在 UI 中展示复杂计算结果，正确做法是：

```rust
// counter.rml.rs
#[computed]
pub fn display_message(&self) -> SharedString {
    if self.count > 10 {
        "🚀 超过十啦！".into()
    } else {
        "继续加油".into()
    }
}
```

```html
<!-- counter.rml -->
<span>{display_message}</span>
```

## 2.1.5 注释

RML 支持 HTML 风格注释：

```html
<!-- 这是一个注释，不会渲染到 UI -->
<div>内容</div>

<!--
    多行注释
    可以跨越多行
-->
```

⚠️ **注意**：注释不会被渲染到最终 UI，但会保留在 `.rml` 文件中作为文档。RML 编译器会忽略注释内容。

## 2.1.6 文件结构约定

一个 `.rml` 文件的标准结构：

```html
<!-- 1. 文件头注释（可选）：说明视图用途 -->
<!-- counter.rml —— 计数器视图 -->

<!-- 2. 单一根元素（必需） -->
<div class="counter-container">

    <!-- 3. 子元素 -->
    <h1>{title}</h1>

    <!-- 4. 绑定与指令 -->
    <button onclick={increment}>+</button>

</div>
```

**关键规则**：

- 每个 `.rml` 文件**必须有且仅有一个根元素**
- 根元素可以是任意 HTML 标签或自定义组件
- 文件名与 `.rml.rs` 中的 ViewModel 通过 `#[view]` 宏自动关联

## 2.1.7 小结

RML 的语法哲学可以浓缩为：

> **"HTML 标签 + HTML 属性 + `{ }` 插值 + 极简指令，无任何框架前缀。"**

这种设计让 `.rml` 文件：

- 对设计师：像 HTML 一样亲切
- 对前端工程师：像 React/Vue 一样熟悉
- 对 Rust 工程师：像声明式宏一样简洁
- 对工具链：像 XML 一样可处理

下一节 → [2.2 标签与控件映射](./tags-mapping.md)
