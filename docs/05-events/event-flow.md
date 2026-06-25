# 5.3 事件流

> **本节目标**：掌握 RML 的事件流机制——冒泡、捕获、阻止默认行为、阻止冒泡。

## 5.3.1 事件流的三个阶段

RML 借鉴 DOM 事件模型，事件流分为三个阶段：

```
1. 捕获阶段（Capture）：从根元素向目标元素传播
2. 目标阶段（Target）：事件到达目标元素
3. 冒泡阶段（Bubble）：从目标元素向根元素传播
```

```
┌─────────────────────────────────────┐
│           根元素 <div>              │  ← 捕获开始
│  ┌─────────────────────────────┐    │
│  │      父元素 <div>           │    │  ← 捕获
│  │  ┌─────────────────────┐    │    │
│  │  │   目标 <button>     │    │    │  ← 目标（点击的元素）
│  │  └─────────────────────┘    │    │  ← 冒泡开始
│  └─────────────────────────────┘    │  ← 冒泡
└─────────────────────────────────────┘  ← 冒泡结束
```

## 5.3.2 冒泡机制

默认情况下，RML 事件在冒泡阶段触发。事件从目标元素开始，逐级向上传播到根元素：

```html
<div onclick={on_outer_click}>
    <div onclick={on_middle_click}>
        <button onclick={on_inner_click}>点击我</button>
    </div>
</div>
```

点击 `<button>` 时，事件触发顺序：

1. `on_inner_click`（目标元素）
2. `on_middle_click`（父元素）
3. `on_outer_click`（祖父元素）

### 冒泡的用途

冒泡允许父元素统一处理子元素的事件：

```html
<ul onclick={on_list_click}>
    <li each={item in items} key={item.id} data-id={item.id}>
        {item.text}
    </li>
</ul>
```

```rust
#[command]
pub fn on_list_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 通过事件目标判断点击了哪个 <li>
    if let Some(item_id) = ev.target_data("id") {
        self.select_item(item_id.parse().unwrap(), cx);
    }
}
```

## 5.3.3 阻止冒泡

用 `stop_propagation()` 阻止事件继续冒泡：

```html
<div onclick={on_outer_click}>
    <button onclick={on_inner_click}>点击我</button>
</div>
```

```rust
#[command]
pub fn on_inner_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    ev.stop_propagation();  // 阻止冒泡，on_outer_click 不会触发

    self.handle_button_click(cx);
}

#[command]
pub fn on_outer_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 不会触发，因为内部阻止了冒泡
    self.handle_outer_click(cx);
}
```

### 阻止冒泡的场景

- 模态对话框内部点击不应关闭对话框
- 下拉菜单内部点击不应触发外部点击
- 嵌套列表的子项点击不应触发父项点击

```html
<div onclick={close_modal} class="modal-overlay">
    <div onclick={on_modal_content_click} class="modal-content">
        <!-- 点击这里不应关闭对话框 -->
        <p>对话框内容</p>
    </div>
</div>
```

```rust
#[command]
pub fn on_modal_content_click(&mut self, ev: &ClickEvent, _cx: &mut ViewContext<Self>) {
    ev.stop_propagation();  // 阻止冒泡，不关闭对话框
}

#[command]
pub fn close_modal(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.is_modal_open = false;
    cx.notify();
}
```

## 5.3.4 阻止默认行为

某些事件有默认行为，可以用 `prevent_default()` 阻止：

```rust
#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    // Ctrl+S 默认会触发浏览器保存，阻止它
    if ev.modifiers.control && ev.key == Key::Character('s') {
        ev.prevent_default();
        self.save(cx);
    }
}
```

### 有默认行为的事件

| 事件           | 默认行为           | 阻止后的效果       |
| ------------ | -------------- | ------------- |
| `onclick`（链接） | 导航到 href       | 不导航           |
| `onkeydown`（Tab） | 切换焦点           | 不切换焦点         |
| `onsubmit`   | 提交表单           | 不提交，由 JS 处理   |
| `onwheel`    | 滚动             | 不滚动           |

## 5.3.5 事件委托

事件委托是冒泡机制的经典应用：把多个子元素的事件统一交给父元素处理。

### 传统方式：每个子元素绑定事件

```html
<ul>
    <li each={item in items} key={item.id} onclick={select_item, {item.id}}>
        {item.text}
    </li>
</ul>
```

### 事件委托：父元素统一处理

```html
<ul onclick={on_list_click}>
    <li each={item in items} key={item.id} data-id={item.id}>
        {item.text}
    </li>
</ul>
```

```rust
#[command]
pub fn on_list_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 通过 data-id 属性识别点击的项
    if let Some(id_str) = ev.target_attribute("data-id") {
        let id: u64 = id_str.parse().unwrap();
        self.select_item(id, cx);
    }
}
```

### 事件委托的优点

| 优点     | 说明                          |
| ------ | --------------------------- |
| 减少绑定数  | 只在父元素绑定一次，子元素无需单独绑定         |
| 动态友好   | 新增子元素自动被父元素处理，无需重新绑定        |
| 性能优化   | 减少订阅数量，降低内存占用               |

⚠️ **注意**：事件委托适用于点击、键盘等冒泡事件，不适用于 `onmouseenter`、`onmouseleave` 等不冒泡的事件。

## 5.3.6 事件流的控制

### 完整示例

```html
<div onclick={on_root_click} class="root">
    <div onclick={on_middle_click} class="middle">
        <button onclick={on_button_click} class="button">
            点击我
        </button>
    </div>
</div>
```

```rust
#[command]
pub fn on_button_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    println!("1. 按钮点击");
    // ev.stop_propagation();  // 取消注释则阻止冒泡
    // ev.prevent_default();   // 取消注释则阻止默认行为
    self.handle_button(cx);
}

#[command]
pub fn on_middle_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    println!("2. 中间元素点击");
    self.handle_middle(cx);
}

#[command]
pub fn on_root_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    println!("3. 根元素点击");
    self.handle_root(cx);
}
```

### 控制选项

| 调用                       | 效果                  |
| ------------------------ | ------------------- |
| 无                        | 事件正常冒泡，依次触发三个处理函数   |
| `ev.stop_propagation()`  | 只触发 `on_button_click` |
| `ev.prevent_default()`   | 阻止默认行为，但事件仍冒泡       |
| 两者都调用                    | 既阻止默认行为，又阻止冒泡       |

## 5.3.7 自定义事件与冒泡

自定义组件的事件也会冒泡，可以通过 `stop_propagation()` 控制：

```html
<div onclick={on_outer_click}>
    <SearchBox on_search={on_search} />
</div>
```

```rust
// SearchBox 组件内部
#[command]
pub fn on_search_internal(&mut self, ev: &SearchEvent, cx: &mut ViewContext<Self>) {
    // 处理搜索...
    ev.stop_propagation();  // 阻止冒泡到外部 div
}
```

详见 [5.4 自定义事件](./custom-events.md)。

## 5.3.8 事件流的最佳实践

### 1. 谨慎阻止冒泡

```rust
// ❌ 滥用 stop_propagation
#[command]
pub fn on_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    ev.stop_propagation();  // 总是阻止，可能影响外部逻辑
    self.handle(cx);
}

// ✅ 只在必要时阻止
#[command]
pub fn on_modal_content_click(&mut self, ev: &ClickEvent, _cx: &mut ViewContext<Self>) {
    ev.stop_propagation();  // 模态框内部点击，明确需要阻止
}
```

### 2. 优先用事件委托

```html
<!-- ❌ 每个子元素都绑定 -->
<li each={item in items} onclick={select, {item.id}}>

<!-- ✅ 父元素事件委托 -->
<ul onclick={on_list_click}>
    <li each={item in items} data-id={item.id}>
```

### 3. 阻止默认行为要明确

```rust
// ❌ 不清楚为什么阻止
#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    ev.prevent_default();
    self.handle(cx);
}

// ✅ 明确阻止原因
#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    if ev.modifiers.control && ev.key == Key::Character('s') {
        ev.prevent_default();  // 阻止浏览器保存，使用应用内保存
        self.save(cx);
    }
}
```

## 5.3.9 小结

RML 的事件流机制：

- **三个阶段**：捕获 → 目标 → 冒泡
- **默认冒泡**：事件从目标元素向上传播
- **`stop_propagation()`**：阻止事件继续冒泡
- **`prevent_default()`**：阻止事件的默认行为
- **事件委托**：父元素统一处理子元素事件

掌握事件流，你就能精确控制事件的传播路径，实现复杂的交互逻辑。

下一节 → [5.4 自定义事件](./custom-events.md)
