# 6.1 内置组件

> **本节目标**：了解 RML 内置组件的来源——HTML 标签映射与 gpui-component 组件库。

## 6.1.1 内置组件的来源

RML 的内置组件来自两个层次：

```
┌─────────────────────────────────────────────┐
│           RML 内置组件                        │
│                                             │
│  ┌─────────────────┐  ┌─────────────────┐   │
│  │  HTML 标签映射   │  │ gpui-component  │   │
│  │  (基础元素)      │  │  (高级组件)      │   │
│  │                 │  │                 │   │
│  │  div, span, p   │  │  Button, Input  │   │
│  │  h1~h6, button  │  │  Checkbox, List │   │
│  │  input, ul, li  │  │  Modal, Tooltip │   │
│  └─────────────────┘  └─────────────────┘   │
└─────────────────────────────────────────────┘
```

## 6.1.2 HTML 标签映射

RML 把标准 HTML 标签映射到 GPUI 的基础元素：

| HTML 标签      | GPUI 元素          | 用途           |
| ------------- | ----------------- | ------------- |
| `<div>`       | `div()`           | 块级容器          |
| `<span>`      | `div()` (inline)  | 行内容器          |
| `<p>`         | `div()`           | 段落            |
| `<h1>`~`<h6>` | `div()` + 标题样式    | 标题            |
| `<button>`    | `Button::new()`   | 按钮            |
| `<input>`     | `Input::new()`    | 输入框           |
| `<textarea>`  | `Input::new()`    | 多行输入          |
| `<ul>`        | `div()`           | 无序列表          |
| `<ol>`        | `div()`           | 有序列表          |
| `<li>`        | `div()`           | 列表项           |
| `<img>`       | `img()`           | 图片            |
| `<a>`         | `div()`           | 链接            |
| `<label>`     | `div()`           | 标签            |

详见 [2.2 标签映射](../02-syntax/tags-mapping.md)。

## 6.1.3 gpui-component 组件库

RML 集成了 `gpui-component` 库，提供丰富的高级组件：

### 表单组件

| 组件名                | RML 写法                              | 用途           |
| ------------------ | ------------------------------------ | ------------ |
| `Button`           | `<Button>` 或 `<button>`              | 按钮           |
| `Input`            | `<Input>` 或 `<input>`                | 文本输入         |
| `Checkbox`         | `<Checkbox>` 或 `<input type="checkbox">` | 复选框          |
| `Switch`           | `<Switch>`                           | 开关           |
| `Radio`            | `<Radio>`                            | 单选框          |
| `Slider`           | `<Slider>`                           | 滑块           |
| `DatePicker`       | `<DatePicker>`                       | 日期选择器        |
| `ColorPicker`      | `<ColorPicker>`                      | 颜色选择器        |

### 导航组件

| 组件名           | RML 写法             | 用途           |
| ------------- | ------------------- | ------------ |
| `Navbar`      | `<Navbar>`          | 导航栏          |
| `Sidebar`     | `<Sidebar>`         | 侧边栏          |
| `Tabs`        | `<Tabs>`            | 标签页          |
| `Breadcrumb`  | `<Breadcrumb>`      | 面包屑          |
| `Pagination`  | `<Pagination>`      | 分页           |

### 数据展示组件

| 组件名           | RML 写法             | 用途           |
| ------------- | ------------------- | ------------ |
| `Table`       | `<Table>`           | 表格           |
| `List`        | `<List>`            | 列表           |
| `Card`        | `<Card>`            | 卡片           |
| `Tag`         | `<Tag>`             | 标签           |
| `Badge`       | `<Badge>`           | 徽章           |
| `Avatar`      | `<Avatar>`          | 头像           |
| `Tooltip`     | `<Tooltip>`         | 工具提示         |
| `Empty`       | `<Empty>`           | 空状态          |

### 反馈组件

| 组件名           | RML 写法             | 用途           |
| ------------- | ------------------- | ------------ |
| `Modal`       | `<Modal>`           | 模态框          |
| `Drawer`      | `<Drawer>`          | 抽屉           |
| `Notification`| `<Notification>`    | 通知           |
| `Message`     | `<Message>`         | 消息提示         |
| `Progress`    | `<Progress>`        | 进度条          |
| `Loading`     | `<Loading>`         | 加载           |

## 6.1.4 使用内置组件

### 基础 HTML 标签

```html
<div class="container">
    <h1>标题</h1>
    <p>段落内容</p>
    <button onclick={handle_click}>点击</button>
</div>
```

### 高级组件（PascalCase）

```html
<div>
    <Button variant="primary" onclick={submit}>提交</Button>
    <Button variant="danger" onclick={delete_item}>删除</Button>

    <Input model={username} placeholder="用户名" />

    <Checkbox model={remember_me}>记住我</Checkbox>

    <Modal if={show_modal} on_close={close_modal}>
        <p>对话框内容</p>
    </Modal>
</div>
```

### 组件属性

内置组件支持标准 HTML 属性和组件特有属性：

```html
<!-- 标准属性 -->
<button class="btn" disabled={is_loading} onclick={submit}>
    提交
</button>

<!-- 组件特有属性 -->
<Button variant="primary" size="large" loading={is_loading}>
    提交
</Button>

<Input
    model={search_text}
    placeholder="搜索..."
    prefix_icon="search"
    suffix_icon={has_value ? "clear" : ""}
/>
```

## 6.1.5 内置组件的样式

### 通过 class 属性

```html
<button class="btn primary">主要按钮</button>
<button class="btn danger">危险按钮</button>
```

```css
/* styles.css */
.btn {
    padding: 8px 16px;
    border-radius: 4px;
    border: none;
    cursor: pointer;
}

.btn.primary {
    background-color: #007bff;
    color: white;
}

.btn.danger {
    background-color: #dc3545;
    color: white;
}
```

### 通过 style 属性

```html
<div style="background: red; padding: 10px;">
    红色背景
</div>
```

### 通过 variant 属性（高级组件）

```html
<Button variant="primary">主要</Button>
<Button variant="secondary">次要</Button>
<Button variant="danger">危险</Button>
<Button variant="ghost">幽灵</Button>

<Input size="small">小输入框</Input>
<Input size="medium">中等输入框</Input>
<Input size="large">大输入框</Input>
```

## 6.1.6 内置组件的事件

内置组件支持标准事件属性：

```html
<button onclick={handle_click}>点击</button>
<input oninput={handle_input} onchange={handle_change} />
<div onmouseenter={handle_enter} onmouseleave={handle_leave}>
    悬停我
</div>
```

高级组件还支持自定义事件：

```html
<Modal on_close={handle_close}>...</Modal>
<Tabs on_change={handle_tab_change}>...</Tabs>
<Pagination on_change={handle_page_change} />
```

详见 [5.4 自定义事件](../05-events/custom-events.md)。

## 6.1.7 内置组件的响应式

内置组件完全支持数据绑定：

```html
<div>
    <p>当前计数: {count}</p>
    <Button onclick={increment} disabled={count >= 10}>
        增加
    </Button>

    <Input model={search_text} placeholder="搜索..." />
    <p if={!search_text.is_empty()}>正在搜索: {search_text}</p>

    <ul>
        <li each={item in items} key={item.id}>
            {item.name}
        </li>
    </ul>
</div>
```

## 6.1.8 内置组件的列表渲染

```html
<Table data={users}>
    <Column title="ID" field="id" />
    <Column title="姓名" field="name" />
    <Column title="邮箱" field="email" />
</Table>

<List data={items}>
    <template>
        <div class="item">
            <h3>{item.title}</h3>
            <p>{item.description}</p>
        </div>
    </template>
</List>
```

## 6.1.9 内置组件 vs 自定义组件

| 维度     | 内置组件           | 自定义组件           |
| ------ | -------------- | --------------- |
| 来源     | 框架提供           | 开发者编写           |
| 命名     | HTML 标签或 PascalCase | PascalCase     |
| 模板     | 内置             | `.rml` 文件       |
| 可定制性   | 通过属性和样式        | 完全可定制           |
| 适用场景   | 通用 UI          | 业务特定 UI         |

## 6.1.10 小结

RML 的内置组件来自两个层次：

- **HTML 标签映射**：`div`、`span`、`button`、`input` 等基础元素
- **gpui-component 库**：`Button`、`Input`、`Modal`、`Table` 等高级组件

内置组件完全支持数据绑定、事件绑定、列表渲染等 RML 特性，是构建应用的基础。

下一节 → [6.2 自定义组件](./custom-components.md)
