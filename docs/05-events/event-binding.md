# 5.1 事件绑定

> **本节目标**：完整掌握 `on*` 事件属性的全部写法——命令绑定、方法名绑定、带参数绑定。

## 5.1.1 事件绑定的语法

RML 使用标准 HTML 的 `on*` 属性绑定事件：

```html
<button on-click={submit}>提交</button>
<input oninput={handle_input} />
<div onmouseenter={show_tooltip}>悬停我</div>
```

## 5.1.2 三种绑定方式

### 方式一：命令绑定（推荐）

直接绑定 `#[command]` 标记的方法：

```rust
#[command]
pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 提交逻辑
    cx.notify();
}
```

```html
<button on-click={submit}>提交</button>
```

**优点**：

- 编译期检查命令是否存在
- 类型安全，参数类型在编译期验证
- 与 `#[command]` 宏紧密集成

### 方式二：方法名绑定

用字符串形式绑定方法名：

```rust
// 不需要 #[command] 标记
pub fn handle_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 处理逻辑
    cx.notify();
}
```

```html
<button on-click="handle_click">提交</button>
```

**特点**：

- 方法不需要 `#[command]` 标记
- 字符串形式，编译期不检查
- 适用于内部辅助方法

⚠️ **注意**：方法名绑定的方法签名必须与命令方法相同（接收事件对象和 cx）。

### 方式三：带参数绑定

传递额外参数给命令：

```rust
#[command]
pub fn delete_item(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.items.retain(|i| i.id != id);
    cx.notify();
}
```

```html
<button on-click={delete_item, {item.id}}>删除</button>
```

## 5.1.3 完整事件列表

### 鼠标事件

| 事件属性          | 触发时机       | 事件对象            |
| ------------- | ---------- | --------------- |
| `on-click`     | 点击         | `ClickEvent`    |
| `ondblclick`  | 双击         | `ClickEvent`    |
| `onmousedown` | 鼠标按下       | `MouseEvent`    |
| `onmouseup`   | 鼠标释放       | `MouseEvent`    |
| `onmouseenter` | 鼠标进入元素     | `MouseEvent`    |
| `onmouseleave` | 鼠标离开元素     | `MouseEvent`    |
| `onmousemove` | 鼠标在元素上移动   | `MouseEvent`    |
| `onwheel`     | 滚轮滚动       | `WheelEvent`    |

### 键盘事件

| 事件属性        | 触发时机   | 事件对象            |
| ----------- | ------ | --------------- |
| `onkeydown` | 键盘按下   | `KeyDownEvent`  |
| `onkeyup`   | 键盘释放   | `KeyUpEvent`    |

### 表单事件

| 事件属性       | 触发时机           | 事件对象           |
| ---------- | -------------- | -------------- |
| `oninput`  | 输入框值实时变化       | `InputEvent`   |
| `onchange` | 值变化（失去焦点后）     | `ChangeEvent`  |
| `onsubmit` | 表单提交           | `SubmitEvent`  |
| `onfocus`  | 获得焦点           | `FocusEvent`   |
| `onblur`   | 失去焦点           | `FocusEvent`   |

### 其他事件

| 事件属性           | 触发时机       | 事件对象           |
| -------------- | ---------- | -------------- |
| `onload`       | 元素加载完成     | `LoadEvent`    |
| `onresize`     | 窗口大小变化     | `ResizeEvent`  |
| `onscroll`     | 滚动         | `ScrollEvent`  |

## 5.1.4 事件绑定的示例

### 点击事件

```html
<button on-click={increment}>+1</button>
<button on-click={decrement}>-1</button>
<button on-click={reset}>重置</button>
```

### 输入事件

```html
<input
    value={search_text}
    oninput={on_search_input}
    placeholder="搜索..."
/>
```

### 键盘事件

```html
<input
    value={new_todo}
    onkeydown={on_enter_key}
    placeholder="按回车添加"
/>
```

### 鼠标悬停

```html
<div
    onmouseenter={show_tooltip}
    onmouseleave={hide_tooltip}
    class="tooltip-target"
>
    悬停查看详情
</div>
```

### 焦点事件

```html
<input
    onfocus={on_input_focus}
    onblur={on_input_blur}
    placeholder="点击聚焦"
/>
```

## 5.1.5 带参数的事件绑定

### 单参数

```html
<button on-click={delete_item, {item.id}}>删除</button>
<button on-click={edit_item, {item.id}}>编辑</button>
<button on-click={toggle_status, {item.id}}>切换状态</button>
```

### 多参数

```html
<button on-click={update_status, {item.id}, 'completed'}>标记完成</button>
<button on-click={update_status, {item.id}, 'pending'}>标记待办</button>
<button on-click={move_item, {item.id}, {target_index}}>移动</button>
```

### 参数类型

```html
<!-- 数字 -->
<button on-click={set_priority, {item.id}, 1}>高优先级</button>

<!-- 字符串 -->
<button on-click={set_category, {item.id}, 'work'}>工作</button>

<!-- 布尔 -->
<button on-click={set_flag, {item.id}, true}>启用</button>

<!-- 字段引用 -->
<button on-click={copy_item, {source.id}, {target.id}}>复制</button>
```

## 5.1.6 事件绑定的常见模式

### 模式一：单一命令

最简单的模式，一个事件绑定一个命令：

```html
<button on-click={submit}>提交</button>
```

### 模式二：条件命令

根据条件决定是否执行：

```rust
#[command]
pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    if !self.can_submit() {
        return;
    }
    // 提交逻辑
}
```

```html
<button on-click={submit} disabled={!can_submit}>提交</button>
```

### 模式三：链式命令

一个事件触发多个命令：

```rust
#[command]
pub fn on_enter_key(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    if ev.key == Key::Enter {
        self.validate_input(cx);
        self.submit(&ClickEvent::default(), cx);
        self.clear_input(cx);
    }
}
```

```html
<input onkeydown={on_enter_key} value={input_text} />
```

### 模式四：事件转发

父视图处理子组件的事件：

```html
<SearchBox on_search={handle_search} placeholder="搜索..." />
```

```rust
#[command]
pub fn handle_search(&mut self, query: SharedString, _: &SearchEvent, cx: &mut ViewContext<Self>) {
    self.search_query = query;
    self.perform_search(cx);
}
```

## 5.1.7 事件绑定的注意事项

### 命令必须存在

```html
<!-- ❌ 命令不存在，编译错误 -->
<button on-click={non_existent}>点击</button>
```

### 参数类型必须匹配

```rust
#[command]
pub fn delete_item(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // id 类型是 u64
}
```

```html
<!-- ❌ 参数类型不匹配 -->
<button on-click={delete_item, 'string_id'}>删除</button>

<!-- ✅ 参数类型匹配 -->
<button on-click={delete_item, {item.id}}>删除</button>
```

### 事件对象类型必须匹配

```rust
// 命令声明的事件对象类型
#[command]
pub fn on_click(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) { ... }
```

```html
<!-- ✅ on-click 对应 ClickEvent -->
<button on-click={on_click}>点击</button>

<!-- ❌ onkeydown 对应 KeyDownEvent，不匹配 -->
<button onkeydown={on_click}>点击</button>
```

## 5.1.8 事件绑定的最佳实践

### 1. 用动词命名命令

```rust
// ✅ 动词开头
#[command]
pub fn submit(&mut self, ...) { ... }

#[command]
pub fn delete_item(&mut self, ...) { ... }

// ❌ 名词或事件名
#[command]
pub fn button_click(&mut self, ...) { ... }  // 应为 submit
#[command]
pub fn on_click(&mut self, ...) { ... }      // 应为 submit
```

### 2. 命令与事件语义对应

```html
<!-- ✅ 语义对应 -->
<button on-click={submit}>提交</button>           <!-- 点击 = 提交 -->
<input onkeydown={on_enter_key} />              <!-- 键盘 = 回车处理 -->

<!-- ❌ 语义混乱 -->
<button on-click={on_input_change}>提交</button>  <!-- 点击 ≠ 输入变化 -->
```

### 3. 复杂逻辑用辅助方法

```rust
#[command]
pub fn on_enter_key(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    if ev.key == Key::Enter {
        self.submit_form(cx);  // 调用辅助方法
    }
}

// 辅助方法，不标注 #[command]
fn submit_form(&mut self, cx: &mut ViewContext<Self>) {
    if self.validate() {
        self.save_data(cx);
        self.show_success(cx);
    }
}
```

### 4. 避免在事件中做重计算

```rust
// ❌ 每次点击都重计算
#[command]
pub fn on_sort(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.items.sort_by(|a, b| {
        // 复杂排序逻辑...
    });
    cx.notify();
}

// ✅ 用计算属性缓存
#[computed]
pub fn sorted_items(&self) -> Vec<&Item> {
    let mut items: Vec<_> = self.items.iter().collect();
    items.sort_by(|a, b| {
        // 复杂排序逻辑...
    });
    items
}
```

## 5.1.9 小结

RML 的事件绑定提供三种方式：

- **命令绑定**：`on-click={command}`（推荐）
- **方法名绑定**：`on-click="method_name"`
- **带参数绑定**：`on-click={command, {param}}`

掌握完整的事件列表和绑定方式，你就能处理任何用户交互。

下一节 → [5.2 事件对象](./event-objects.md)
