# 6.3 插槽与内容分发

> **本节目标**：掌握 `<slot>` 的用法，实现组件的内容分发，提升组件的灵活性。

## 6.3.1 插槽的概念

插槽（Slot）是组件内容分发的机制，允许父视图向子组件传递任意内容：

```
父视图                     子组件
┌──────────┐              ┌──────────────────┐
│ <Card>   │              │ <div class="card">│
│   <slot> │──内容传递──▶ │   <slot></slot>  │
│ </Card>  │              │ </div>            │
└──────────┘              └──────────────────┘
```

## 6.3.2 默认插槽

### 定义插槽

在组件模板中用 `<slot>` 定义内容占位符：

```html
<!-- components/card.rml -->
<div class="card">
    <div class="card-body">
        <slot></slot>
    </div>
</div>
```

### 使用插槽

父视图在组件标签内传递内容：

```html
<!-- views/my_view.rml -->
<Card>
    <h2>标题</h2>
    <p>这是卡片内容。</p>
    <button onclick={handle_click}>点击</button>
</Card>
```

### 渲染结果

```html
<div class="card">
    <div class="card-body">
        <h2>标题</h2>
        <p>这是卡片内容。</p>
        <button>点击</button>
    </div>
</div>
```

### 默认内容

插槽可以提供默认内容，当父视图未传递时显示：

```html
<!-- components/card.rml -->
<div class="card">
    <div class="card-body">
        <slot>暂无内容</slot>
    </div>
</div>
```

```html
<!-- 使用时未传递内容 -->
<Card></Card>

<!-- 渲染结果 -->
<div class="card">
    <div class="card-body">
        暂无内容
    </div>
</div>
```

## 6.3.3 具名插槽

当组件需要多个内容区域时，用具名插槽：

### 定义具名插槽

```html
<!-- components/card.rml -->
<div class="card">
    <div class="card-header">
        <slot name="header">默认标题</slot>
    </div>
    <div class="card-body">
        <slot></slot>
    </div>
    <div class="card-footer">
        <slot name="footer">默认页脚</slot>
    </div>
</div>
```

### 使用具名插槽

父视图用 `<template slot="...">` 指定内容到哪个插槽：

```html
<Card>
    <template slot="header">
        <h2>用户信息</h2>
    </template>

    <template>
        <p>姓名: 张三</p>
        <p>邮箱: zhangsan@example.com</p>
    </template>

    <template slot="footer">
        <button onclick={edit}>编辑</button>
        <button onclick={delete}>删除</button>
    </template>
</Card>
```

### 渲染结果

```html
<div class="card">
    <div class="card-header">
        <h2>用户信息</h2>
    </div>
    <div class="card-body">
        <p>姓名: 张三</p>
        <p>邮箱: zhangsan@example.com</p>
    </div>
    <div class="card-footer">
        <button>编辑</button>
        <button>删除</button>
    </div>
</div>
```

## 6.3.4 插槽的绑定上下文

插槽内容默认使用**父视图的绑定上下文**：

```html
<!-- 父视图 -->
<div>
    <Card>
        <template>
            <p>{user_name}</p>  <!-- 这里的 user_name 是父视图的字段 -->
        </template>
    </Card>
</div>
```

```rust
// 父视图
#[derive(Model)]
#[view]
pub struct MyView {
    pub user_name: SharedString,  // 父视图的字段
}
```

### 访问子组件的数据

如果需要在插槽中访问子组件的数据，用 `let` 指令：

```html
<!-- components/list.rml -->
<ul>
    <li each={item in items}>
        <slot let-item={item} let-index={index}></slot>
    </li>
</ul>
```

```rust
#[derive(Model)]
#[component(template = "components/list.rml")]
pub struct List {
    pub items: Vec<Item>,
}
```

```html
<!-- 父视图 -->
<List items={my_items}>
    <template let-item let-index>
        <span>{index}: {item.name}</span>
        <button onclick={delete_item, {item.id}}>删除</button>
    </template>
</List>
```

## 6.3.5 插槽的组合

插槽可以与数据绑定、事件绑定、指令组合使用：

### 与数据绑定组合

```html
<Card>
    <template slot="header">
        <h2>{user.name}</h2>
    </template>

    <template>
        <p>邮箱: {user.email}</p>
        <p>电话: {user.phone}</p>
    </template>
</Card>
```

### 与事件绑定组合

```html
<Card>
    <template slot="footer">
        <button onclick={handle_save}>保存</button>
        <button onclick={handle_cancel}>取消</button>
    </template>
</Card>
```

### 与指令组合

```html
<Card>
    <template>
        <div if={is_loading}>加载中...</div>
        <div if={!is_loading}>
            <ul>
                <li each={item in items} key={item.id}>
                    {item.name}
                </li>
            </ul>
        </div>
    </template>
</Card>
```

## 6.3.6 插槽的应用场景

### 场景一：通用卡片

```html
<!-- components/card.rml -->
<div class="card">
    <div class="card-header" if={has_header_slot}>
        <slot name="header"></slot>
    </div>
    <div class="card-body">
        <slot></slot>
    </div>
    <div class="card-footer" if={has_footer_slot}>
        <slot name="footer"></slot>
    </div>
</div>
```

### 场景二：模态对话框

```html
<!-- components/modal.rml -->
<div if={is_open} class="modal-overlay" onclick={on_overlay_click}>
    <div class="modal-content" onclick={on_content_click}>
        <div class="modal-header">
            <h2>{title}</h2>
            <button onclick={on_close}>✕</button>
        </div>
        <div class="modal-body">
            <slot></slot>
        </div>
        <div class="modal-footer">
            <slot name="footer">
                <button onclick={on_close}>关闭</button>
            </slot>
        </div>
    </div>
</div>
```

### 场景三：列表项

```html
<!-- components/list.rml -->
<ul class="list">
    <li each={item in items} key={item.id}>
        <slot let-item={item}></slot>
    </li>
</ul>
```

### 场景四：表单字段

```html
<!-- components/form_field.rml -->
<div class="form-field">
    <label if={has_label}>
        <slot name="label"></slot>
    </label>
    <div class="form-control">
        <slot></slot>
    </div>
    <div class="form-error" if={has_error}>
        <slot name="error"></slot>
    </div>
    <div class="form-hint" if={has_hint}>
        <slot name="hint"></slot>
    </div>
</div>
```

## 6.3.7 完整示例：可配置的对话框

```html
<!-- components/dialog.rml -->
<div if={is_open} class="dialog-overlay" onclick={on_overlay_click}>
    <div class="dialog" onclick={on_content_click}>
        <div class="dialog-header">
            <slot name="header">
                <h2>{title}</h2>
            </slot>
            <button if={closable} onclick={on_close} class="dialog-close">✕</button>
        </div>

        <div class="dialog-body">
            <slot></slot>
        </div>

        <div class="dialog-footer" if={has_footer}>
            <slot name="footer">
                <button onclick={on_close}>关闭</button>
            </slot>
        </div>
    </div>
</div>
```

```rust
// components/dialog.rml.rs
use rml::prelude::*;

#[derive(Model)]
#[component(template = "components/dialog.rml")]
pub struct Dialog {
    pub title: SharedString,
    pub is_open: bool,
    pub closable: bool,
    pub has_footer: bool,

    pub on_close: Option<Arc<dyn Fn()>>,
}

impl Dialog {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            is_open: false,
            closable: true,
            has_footer: true,
            on_close: None,
        }
    }

    pub fn open(&mut self, cx: &mut ViewContext<Self>) {
        self.is_open = true;
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut ViewContext<Self>) {
        self.is_open = false;
        cx.notify();

        if let Some(callback) = &self.on_close {
            callback();
        }
    }

    #[command]
    pub fn on_close_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.close(cx);
    }

    #[command]
    pub fn on_overlay_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.closable {
            self.close(cx);
        }
    }

    #[command]
    pub fn on_content_click(&mut self, ev: &ClickEvent, _cx: &mut ViewContext<Self>) {
        ev.stop_propagation();
    }
}
```

### 使用对话框

```html
<!-- views/user_view.rml -->
<div>
    <button onclick={show_delete_dialog}>删除用户</button>

    <Dialog
        title="确认删除"
        on_close={handle_dialog_close}
    >
        <template>
            <p>确定要删除用户 {user_name} 吗？</p>
            <p class="warning">此操作不可撤销！</p>
        </template>

        <template slot="footer">
            <button onclick={handle_cancel}>取消</button>
            <button onclick={handle_confirm} class="danger">确认删除</button>
        </template>
    </Dialog>
</div>
```

## 6.3.8 插槽的注意事项

### 1. 默认内容的覆盖

```html
<!-- 组件定义 -->
<slot>默认内容</slot>

<!-- 父视图传递内容 -->
<Card>
    <template>自定义内容</template>
</Card>

<!-- 渲染结果：显示"自定义内容"，默认内容被覆盖 -->
```

### 2. 插槽的嵌套

插槽内容可以包含其他组件：

```html
<Card>
    <template>
        <Avatar src={user.avatar} />
        <UserName name={user.name} />
    </template>
</Card>
```

### 3. 插槽与 `each` 的配合

```html
<List items={users}>
    <template let-item>
        <div class="user-item">
            <span>{item.name}</span>
            <button onclick={delete_user, {item.id}}>删除</button>
        </div>
    </template>
</List>
```

### 4. 插槽的事件上下文

插槽中的事件绑定到父视图的命令：

```html
<!-- 父视图 -->
<Card>
    <template slot="footer">
        <button onclick={handle_save}>保存</button>  <!-- handle_save 是父视图的命令 -->
    </template>
</Card>
```

## 6.3.9 小结

插槽是组件内容分发的核心机制：

- **默认插槽**：`<slot></slot>`，接收父视图的默认内容
- **具名插槽**：`<slot name="...">`，接收指定名称的内容
- **默认内容**：`<slot>默认内容</slot>`，未传递时显示
- **作用域插槽**：`<slot let-item={item}>`，向父视图暴露数据
- **绑定上下文**：插槽内容使用父视图的上下文

掌握插槽，你就能设计出高度灵活、可复用的组件。

下一节 → [6.4 组件属性](./component-props.md)
