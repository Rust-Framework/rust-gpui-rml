# 2.2 标签与控件映射

> **本节目标**：建立 HTML 标签到 GPUI 元素的完整映射表，理解每个标签的语义和底层实现。

## 2.2.1 完整映射表

| HTML 标签         | RML 语义   | 对应 GPUI 实现                                    |
| --------------- | -------- | --------------------------------------------- |
| `<div>`         | 通用容器/布局块 | `gpui::div()`                                 |
| `<span>`        | 内联文本容器   | `gpui::div().inline()`                        |
| `<p>`           | 段落文本     | `gpui::div().child(Label::new())`             |
| `<h1>` ~ `<h6>` | 标题       | `gpui::div().child(Label::new()).text_size()` |
| `<button>`      | 按钮       | `gpui_component::Button`                      |
| `<input>`       | 输入框      | `gpui_component::Input`                       |
| `<textarea>`    | 多行文本输入   | `gpui_component::TextArea`                    |
| `<ul>` / `<ol>` | 列表容器     | `gpui::div().flex().flex_col()`               |
| `<li>`          | 列表项      | `gpui::div()`                                 |
| `<img>`         | 图片       | `gpui_component::Image`                       |
| `<a>`           | 链接       | `gpui_component::Link`                        |
| `<label>`       | 标签       | `gpui::div().child(Label::new())`             |

## 2.2.2 容器类标签

### `<div>`：通用容器

最常用的标签，对应 GPUI 的 `div()`。用于布局分组、样式作用域、嵌套结构。

```html
<div class="card">
    <div class="card-header">
        <h2>标题</h2>
    </div>
    <div class="card-body">
        <p>内容</p>
    </div>
</div>
```

### `<span>`：内联容器

对应 `div().inline()`。用于行内文本片段的样式包装。

```html
<p>
    欢迎回来，<span class="username">{user_name}</span>！
    你有 <span class="badge">{unread_count}</span> 条未读消息。
</p>
```

## 2.2.3 文本类标签

### `<p>`：段落

对应 `div().child(Label::new(...))`。用于段落文本。

```html
<p class="intro">这是一段介绍文字。</p>
```

### `<h1>` ~ `<h6>`：标题

对应 `div().child(Label::new(...)).text_size(...)`。语义化标题层级。

```html
<h1>主标题</h1>
<h2>副标题</h2>
<h3>章节标题</h3>
```

### `<label>`：表单标签

对应 `div().child(Label::new(...))`。通常与 `<input>` 配合使用。

```html
<label for="username">用户名</label>
<input id="username" type="text" />
```

## 2.2.4 交互类标签

### `<button>`：按钮

对应 `gpui_component::Button`。RML 自动处理按钮的样式、状态、点击事件。

```html
<button class="btn primary" onclick={submit}>提交</button>
<button class="btn danger" onclick={delete_item} disabled={is_deleting}>
    删除
</button>
```

**支持的属性**：

| 属性          | 类型     | 说明           |
| ----------- | ------ | ------------ |
| `disabled`  | bool   | 是否禁用         |
| `onclick`   | 命令     | 点击事件         |
| `class`     | string | 样式类名         |
| `ref`       | string | 元素引用名        |

### `<input>`：输入框

对应 `gpui_component::Input`。支持多种 type。

```html
<!-- 文本输入 -->
<input type="text" model={user_name} placeholder="请输入用户名" />

<!-- 密码输入 -->
<input type="password" model={password} placeholder="密码" />

<!-- 复选框 -->
<input type="checkbox" checked={remember_me} onchange={toggle_remember} />

<!-- 数字输入 -->
<input type="number" model={age} min="0" max="150" />
```

**支持的 type**：

| type 值       | 用途       | 对应组件                       |
| ------------ | -------- | -------------------------- |
| `text`       | 单行文本（默认） | `Input`                    |
| `password`   | 密码       | `Input` with masked        |
| `number`     | 数字       | `Input` with number filter |
| `checkbox`   | 复选框      | `Checkbox`                 |
| `radio`      | 单选框      | `Radio`                    |
| `email`      | 邮箱       | `Input` with email filter  |

### `<textarea>`：多行文本

对应 `gpui_component::TextArea`。

```html
<textarea model={content} placeholder="请输入内容..." rows="5"></textarea>
```

## 2.2.5 列表类标签

### `<ul>` / `<ol>`：列表容器

对应 `div().flex().flex_col()`。`<ul>` 无序，`<ol>` 有序（自动添加序号）。

```html
<ul class="todo-list">
    <li each={todo in todos} key={todo.id}>
        {todo.text}
    </li>
</ul>

<ol class="ranking">
    <li each={user in top_users} key={user.id}>
        {user.name} - {user.score}
    </li>
</ol>
```

### `<li>`：列表项

对应 `div()`。必须配合 `<ul>` 或 `<ol>` 使用。

## 2.2.6 媒体类标签

### `<img>`：图片

对应 `gpui_component::Image`。

```html
<img src="/assets/avatar.png" alt="用户头像" class="avatar" />
```

**支持的属性**：

| 属性     | 类型     | 说明              |
| ------ | ------ | --------------- |
| `src`  | string | 图片路径（本地或 URL）   |
| `alt`  | string | 替代文本            |
| `class` | string | 样式类名            |

### `<a>`：链接

对应 `gpui_component::Link`。

```html
<a href="https://example.com" onclick={open_link}>访问网站</a>
```

## 2.2.7 自闭合标签

HTML 中自闭合的标签在 RML 中也支持自闭合写法：

```html
<input type="text" />
<img src="avatar.png" />
<br />
```

⚠️ **注意**：`<br />` 换行标签在 RML 中映射为 `div().h(px(0.0))`，仅用于文本换行场景。

## 2.2.8 标签的 GPUI 代码生成

每个 HTML 标签在编译期会生成对应的 GPUI 代码。例如：

```html
<!-- 输入 -->
<div class="card">
    <h1>标题</h1>
    <button onclick={submit}>提交</button>
</div>
```

```rust
// 生成的 GPUI 代码（简化示意）
gpui::div()
    .class("card")
    .child(
        gpui::div()
            .text_size(28.0)
            .child(gpui::Label::new("标题"))
    )
    .child(
        gpui_component::Button::new("提交")
            .on_click(cx.listener(|this, ev, cx| this.submit(ev, cx)))
    )
```

💡 **设计要点**：生成的代码与手写代码完全等价，没有任何运行时开销。你可以用 `cargo rml-expand` 命令查看生成的完整代码，详见 [第 10 章 · 调试技巧](../10-advanced/debugging.md)。

## 2.2.9 扩展组件 kebab-case 与小写别名规范

扩展轨组件（gpui-component 路由表、`compiler/menu/` codegen）**推荐**在 RML 中使用 **小写或 kebab-case**，引擎通过 `normalize_component_tag()` 映射为 PascalCase，通过 `canonical_tag()` 额外处理小写别名：

| RML 标签（推荐） | 规范化后 | 说明 |
|------------------|----------|------|
| `context-menu` | `ContextMenu` | kebab-case |
| `dropdown-menu` | `DropdownMenu` | kebab-case |
| `menu-bar` | `MenuBar` | kebab-case |
| `menu-item` | `MenuItem` | kebab-case |
| `menu-separator` | `MenuSeparator` | kebab-case |
| `app-menu-bar` | `AppMenuBar` | kebab-case |
| `button`（扩展轨） | `Button` | 小写 |
| `accordion` | `Accordion` | 小写别名（非 kebab） |
| `item` | `AccordionItem` | 仅 `<accordion>` 内上下文敏感短标签 |
| `descriptions` | `DescriptionList` | 小写别名（非 kebab） |
| `description` | `DescriptionItem` | 仅 `<descriptions>` 内上下文敏感短标签 |
| `separator` | `DescriptionSeparator` | 仅 `<descriptions>` 内上下文敏感短标签；PascalCase `<Separator>` 是独立组件 |

规则：

1. 连字符分段，每段首字母大写后拼接：`foo-bar-baz` → `FooBarBaz`
2. 已是 PascalCase 的标签原样匹配（向后兼容 `<Button>`）
3. snake_case 特殊标签（`menu`、`status_bar`）不参与 kebab 转换，单独注册
4. `component_lookup_resolved()` 先查原始标签，再查规范化结果
5. `canonical_tag()` 在 `normalize_component_tag` 基础上额外处理小写别名：`accordion` → `Accordion`、`item` → `AccordionItem`、`descriptions` → `DescriptionList`、`description` → `DescriptionItem`、`separator` → `DescriptionSeparator`。供 `props_registry` 属性查询使用，避免在 `COMPONENT_PROPS` 中重复登记
6. `<item>` 短标签仅在 `<accordion>` / `<Accordion>` 父容器内被识别为 `AccordionItem`（由 `is_item_builder_tag` 判断）；顶层使用 `<item>` 报 "unknown tag" 错误。同理，`<description>` / `<separator>` 仅在 `<descriptions>` / `<DescriptionList>` 父容器内被识别

菜单子项仅 `menu-item` 与 `menu-separator`（菜单内小写 `separator` 为分隔线别名）；描述列表内小写 `<separator />` 映射到 `DescriptionSeparator`（调用容器 `.separator()`），与独立组件 `<Separator>`（PascalCase）区分。

## 2.2.10 自定义组件

```html
<!-- HTML 标准标签：小写 -->
<div>、<button>、<input>

<!-- 自定义组件：PascalCase -->
<PrimaryButton label="保存" onclick={save} />
<Card>
    <div slot="header">头部</div>
    <p>内容</p>
</Card>
```

自定义组件的详细用法见 [第 6 章 · 组件系统](../06-components/INDEX.md)。

## 2.2.11 小结

RML 的标签系统是 HTML 标签到 GPUI 元素的**一一映射**：

- 容器类：`div`、`span`
- 文本类：`p`、`h1~h6`、`label`
- 交互类：`button`、`input`、`textarea`
- 列表类：`ul`、`ol`、`li`
- 媒体类：`img`、`a`
- 自定义组件：PascalCase 命名

掌握这张映射表，你就能在 `.rml` 中表达任何 UI 结构。

下一节 → [2.3 属性系统](./attributes.md)
