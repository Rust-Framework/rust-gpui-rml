# 2.3 属性系统

> **本节目标**：完整掌握 RML 的四类属性——标准 HTML 属性、数据绑定属性、事件绑定属性、指令属性。

## 2.3.1 四类属性总览

RML 的属性分为四类，每类有不同的语法和用途：

| 类别       | 语法示例                          | 用途           |
| -------- | ----------------------------- | ------------ |
| 标准 HTML 属性 | `class="..."`、`id="..."`      | 静态属性，直接透传    |
| 数据绑定属性   | `value={field}`、`class={dyn}` | 动态属性，绑定 ViewModel |
| 事件绑定属性   | `onclick={fn}`                | 事件处理         |
| 指令属性     | `if={cond}`、`each={...}`      | 控制渲染行为       |

## 2.3.2 标准 HTML 属性

标准 HTML 属性使用 `key="value"` 语法，值是字符串字面量，直接透传到 GPUI 元素。

### `class`：样式类名

```html
<div class="flex flex-col gap-4 p-6 bg-white rounded-lg shadow-md">
    内容
</div>
```

支持 Tailwind 风格的多类名，空格分隔。详见 [第 7 章 · Tailwind 互操作](../07-styling/tailwind-interop.md)。

### `id`：唯一标识

```html
<input id="username-input" type="text" />
```

`id` 主要用于 `ref` 引用的备选方案，以及在样式系统中作为选择器（未来支持）。

### `style`：行内样式

```html
<p style="color: blue; font-size: 16px;">内联样式</p>
```

行内样式支持 CSS 语法，编译期解析为 GPUI 样式 API 调用。

### 输入相关属性

```html
<input
    type="password"
    placeholder="密码"
    maxlength="20"
    disabled
    readonly
    autofocus
/>
```

| 属性           | 适用标签           | 说明           |
| ------------ | -------------- | ------------ |
| `type`       | `<input>`      | 输入类型         |
| `placeholder` | `<input>`、`<textarea>` | 占位符          |
| `maxlength`  | `<input>`、`<textarea>` | 最大长度         |
| `disabled`   | 所有交互元素         | 禁用           |
| `readonly`   | `<input>`、`<textarea>` | 只读           |
| `autofocus`  | `<input>`      | 自动聚焦         |
| `checked`    | `<input type="checkbox">` | 选中状态         |
| `value`      | `<input>`、`<textarea>` | 当前值          |
| `rows`       | `<textarea>`   | 行数           |

### 媒体相关属性

```html
<img src="/assets/avatar.png" alt="用户头像" />
<a href="https://example.com">链接</a>
```

## 2.3.3 数据绑定属性

数据绑定属性使用 `key={expression}` 语法，值是 Rust 表达式，编译期解析为 ViewModel 字段或方法。

### 文本绑定

```html
<!-- 单向绑定：显示 ViewModel 字段 -->
<p>欢迎, {user_name}</p>
<p>总计: {items.len()}</p>
```

### 属性绑定

```html
<!-- 动态 class -->
<div class={container_class}>动态类名</div>

<!-- 动态 value -->
<input value={user_name} />

<!-- 动态 disabled -->
<button disabled={is_loading}>提交</button>

<!-- 动态 checked -->
<input type="checkbox" checked={remember_me} />
```

### 表达式绑定

`{ }` 内部可以是任意 Rust 表达式：

```html
<p>{count + 1}</p>
<p>{if is_vip { "VIP" } else { "普通" }}</p>
<p>{items.iter().filter(|i| i.done).count()}</p>
```

⚠️ **注意**：表达式必须是**纯函数**，不能有副作用。复杂逻辑应放在 `#[computed]` 方法中。

详见 [第 3 章 · 数据绑定系统](../03-binding/INDEX.md)。

## 2.3.4 事件绑定属性

事件绑定属性使用 `on*={command}` 语法，绑定 ViewModel 的命令方法。

### 基础事件

```html
<button onclick={submit}>提交</button>
<input oninput={handle_input} />
<input onkeydown={handle_key} />
<input onkeyup={handle_key_up} />
<div onmouseenter={show_tooltip} onmouseleave={hide_tooltip}>
    悬停我
</div>
```

### 方法名绑定

也可以用字符串形式绑定方法名：

```html
<button onclick="handle_click">方法名绑定</button>
```

### 带参数的事件

```html
<button onclick="delete_item, {item.id}">删除</button>
<button onclick="update_status, {item.id}, 'completed'">完成</button>
```

参数用逗号分隔，第一个是方法名，后续是参数表达式。

### 完整事件列表

| 事件属性          | 触发时机       | 事件对象            |
| ------------- | ---------- | --------------- |
| `onclick`     | 点击         | `ClickEvent`    |
| `oninput`     | 输入框值变化     | `InputEvent`   |
| `onchange`    | 值变化（失去焦点后） | `ChangeEvent`   |
| `onkeydown`   | 键盘按下       | `KeyDownEvent`  |
| `onkeyup`     | 键盘释放       | `KeyUpEvent`    |
| `onmouseenter` | 鼠标进入       | `MouseEvent`    |
| `onmouseleave` | 鼠标离开       | `MouseEvent`    |
| `onmousemove` | 鼠标移动       | `MouseEvent`    |
| `onfocus`     | 获得焦点       | `FocusEvent`    |
| `onblur`      | 失去焦点       | `FocusEvent`    |
| `onsubmit`    | 表单提交       | `SubmitEvent`   |

详见 [第 5 章 · 事件系统](../05-events/INDEX.md)。

## 2.3.5 指令属性

指令属性是 RML 的扩展能力，控制渲染行为。指令使用 `directive={expression}` 语法，**无任何前缀**。

### 完整指令列表

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

指令的详细用法见 [2.4 指令系统](./directives.md)。

## 2.3.6 属性的组合使用

四类属性可以同时出现在一个元素上：

```html
<button
    class="btn primary"              <!-- 标准 HTML 属性 -->
    disabled={is_submitting}         <!-- 数据绑定属性 -->
    onclick={submit}                 <!-- 事件绑定属性 -->
    if={show_submit_button}          <!-- 指令属性 -->
>
    提交
</button>
```

**属性顺序**：四类属性的书写顺序不影响行为，但建议遵循以下约定以提升可读性：

1. 标准 HTML 属性（`class`、`id`、`style`）
2. 数据绑定属性（`value={}`、`disabled={}`）
3. 事件绑定属性（`onclick={}`）
4. 指令属性（`if={}`、`each={}`）

## 2.3.7 布尔属性的简写

HTML 的布尔属性（如 `disabled`、`checked`、`readonly`）在 RML 中有两种写法：

```html
<!-- 静态布尔：仅写属性名表示 true -->
<input disabled />
<input checked />

<!-- 动态布尔：用绑定表达式 -->
<input disabled={is_loading} />
<input checked={is_checked} />
```

## 2.3.8 属性值的转义

属性值中的特殊字符需要转义：

```html
<!-- 字符串中的引号 -->
<p title="他说：&quot;你好&quot;">内容</p>

<!-- 也可以用单引号包裹双引号 -->
<p title='他说："你好"'>内容</p>
```

`{ }` 插值表达式中的字符串由 Rust 编译器处理，无需 HTML 转义：

```html
<p title={format!("用户：{}", name)}>内容</p>
```

## 2.3.9 自定义属性

对于自定义组件，可以定义任意属性作为组件的输入参数：

```html
<PrimaryButton
    label="保存"
    color="blue"
    size="large"
    onclick={save}
/>
```

自定义组件的属性定义见 [第 6 章 · 定义自定义组件](../06-components/define-component.md)。

## 2.3.10 小结

RML 的属性系统是四类属性的有序组合：

1. **标准 HTML 属性**：`class="..."`、`id="..."`、`style="..."`、`type="..."` 等
2. **数据绑定属性**：`value={field}`、`disabled={cond}` 等
3. **事件绑定属性**：`onclick={fn}`、`oninput={fn}` 等
4. **指令属性**：`if={}`、`each={}`、`model={}` 等

掌握这四类属性的语法和用途，你就能在 `.rml` 中表达任何 UI 的结构、样式、数据和行为。

下一节 → [2.4 指令系统](./directives.md)
