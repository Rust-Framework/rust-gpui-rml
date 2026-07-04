# 5.2 事件对象

> **本节目标**：完整掌握各类事件对象的属性和用法，在命令方法中正确读取事件信息。

## 5.2.1 事件对象的作用

事件对象携带事件发生时的上下文信息，如鼠标位置、按键代码、输入值等。命令方法通过事件对象参数访问这些信息：

```rust
#[command]
pub fn on_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    println!("点击位置: {:?}", ev.position);
    println!("点击按钮: {:?}", ev.button);
}
```

## 5.2.2 ClickEvent

点击事件对象，用于 `on-click`、`ondblclick`。

```rust
pub struct ClickEvent {
    pub position: Point<Pixels>,    // 点击位置（相对窗口）
    pub button: MouseButton,         // 鼠标按钮
    pub modifiers: Modifiers,        // 修饰键状态
    pub click_count: usize,          // 点击次数（用于检测双击）
}
```

### 用法

```rust
#[command]
pub fn on_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 检测双击
    if ev.click_count == 2 {
        self.open_item(cx);
        return;
    }

    // 检测右键
    if ev.button == MouseButton::Right {
        self.show_context_menu(ev.position, cx);
        return;
    }

    // 检测 Shift+点击
    if ev.modifiers.shift {
        self.select_range(cx);
    } else {
        self.select_single(cx);
    }
}
```

## 5.2.3 MouseEvent

鼠标事件对象，用于 `onmousedown`、`onmouseup`、`onmouseenter`、`onmouseleave`、`onmousemove`。

```rust
pub struct MouseEvent {
    pub position: Point<Pixels>,     // 鼠标位置
    pub button: MouseButton,          // 按下的按钮
    pub buttons: MouseButtons,        // 当前按下的所有按钮
    pub modifiers: Modifiers,         // 修饰键状态
}
```

### 用法

```rust
#[command]
pub fn on_mouse_move(&mut self, ev: &MouseEvent, cx: &mut ViewContext<Self>) {
    // 更新拖拽位置
    if self.is_dragging {
        self.drag_position = ev.position;
        cx.notify();
    }
}

#[command]
pub fn on_mouse_enter(&mut self, _: &MouseEvent, cx: &mut ViewContext<Self>) {
    self.is_hovered = true;
    cx.notify();
}

#[command]
pub fn on_mouse_leave(&mut self, _: &MouseEvent, cx: &mut ViewContext<Self>) {
    self.is_hovered = false;
    cx.notify();
}
```

## 5.2.4 WheelEvent

滚轮事件对象，用于 `onwheel`。

```rust
pub struct WheelEvent {
    pub position: Point<Pixels>,     // 鼠标位置
    pub delta: ScrollDelta,          // 滚动增量
    pub modifiers: Modifiers,        // 修饰键状态
}

pub enum ScrollDelta {
    Pixels(Point<Pixels>),           // 像素级滚动
    Lines(Point<f32>),               // 行级滚动
}
```

### 用法

```rust
#[command]
pub fn on_wheel(&mut self, ev: &WheelEvent, cx: &mut ViewContext<Self>) {
    if let ScrollDelta::Pixels(delta) = ev.delta {
        self.scroll_y += delta.y;
        cx.notify();
    }
}
```

## 5.2.5 KeyDownEvent / KeyUpEvent

键盘事件对象，用于 `onkeydown`、`onkeyup`。

```rust
pub struct KeyDownEvent {
    pub key: Key,                    // 按键
    pub modifiers: Modifiers,        // 修饰键状态
    pub is_repeat: bool,             // 是否为重复触发
}

pub struct KeyUpEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}
```

### Key 枚举

```rust
pub enum Key {
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Character(char),                 // 字符键
    // ... 更多按键
}
```

### 用法

```rust
#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    match ev.key {
        Key::Enter => self.submit(cx),
        Key::Escape => self.cancel(cx),
        Key::Tab => {
            if ev.modifiers.shift {
                self.focus_previous(cx);
            } else {
                self.focus_next(cx);
            }
        }
        Key::Character('a') if ev.modifiers.control => self.select_all(cx),
        Key::Character('s') if ev.modifiers.control => self.save(cx),
        _ => {}
    }
}
```

### Modifiers

```rust
pub struct Modifiers {
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
    pub command: bool,  // macOS 的 Command 键
}
```

## 5.2.6 InputEvent / ChangeEvent

输入事件对象，用于 `oninput`、`onchange`。

```rust
pub struct InputEvent {
    pub value: SharedString,         // 当前输入值
    pub old_value: SharedString,     // 之前的值
}

pub struct ChangeEvent {
    pub value: SharedString,         // 变化后的值
    pub old_value: SharedString,     // 变化前的值
}
```

### InputEvent vs ChangeEvent

| 事件       | 触发时机           | 用途           |
| ---------- | -------------- | ------------ |
| `oninput`  | 每次输入（实时）      | 实时搜索、字符计数    |
| `onchange` | 失去焦点后          | 提交、验证       |

### 用法

```rust
#[command]
pub fn on_input(&mut self, ev: &InputEvent, cx: &mut ViewContext<Self>) {
    // 实时搜索
    self.search_query = ev.value.clone();
    self.perform_search(cx);
}

#[command]
pub fn on_change(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
    // 验证邮箱格式
    if !ev.value.contains('@') {
        self.error = Some("请输入有效的邮箱".into());
    } else {
        self.email = ev.value.clone();
        self.error = None;
    }
    cx.notify();
}
```

## 5.2.7 FocusEvent

焦点事件对象，用于 `onfocus`、`onblur`。

```rust
pub struct FocusEvent {
    pub target: EntityId,            // 获得或失去焦点的元素
}
```

### 用法

```rust
#[command]
pub fn on_focus(&mut self, _: &FocusEvent, cx: &mut ViewContext<Self>) {
    self.is_focused = true;
    self.show_placeholder = false;
    cx.notify();
}

#[command]
pub fn on_blur(&mut self, _: &FocusEvent, cx: &mut ViewContext<Self>) {
    self.is_focused = false;
    self.validate_input(cx);
    cx.notify();
}
```

## 5.2.8 SubmitEvent

表单提交事件对象，用于 `onsubmit`。

```rust
pub struct SubmitEvent {
    pub form_data: HashMap<SharedString, SharedString>,  // 表单数据
}
```

### 用法

```html
<form onsubmit={handle_submit}>
    <input name="username" model={username} />
    <input name="password" type="password" model={password} />
    <button type="submit">登录</button>
</form>
```

```rust
#[command]
pub fn handle_submit(&mut self, ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
    let username = ev.form_data.get("username").cloned().unwrap_or_default();
    let password = ev.form_data.get("password").cloned().unwrap_or_default();

    self.authenticate(&username, &password, cx);
}
```

## 5.2.9 事件对象的通用方法

所有事件对象都实现了以下 trait：

```rust
pub trait Event {
    /// 阻止默认行为
    fn prevent_default(&self);

    /// 阻止事件冒泡
    fn stop_propagation(&self);

    /// 是否已阻止默认行为
    fn is_default_prevented(&self) -> bool;

    /// 是否已阻止冒泡
    fn is_propagation_stopped(&self) -> bool;
}
```

详见 [5.3 事件流](./event-flow.md)。

## 5.2.10 完整示例：键盘快捷键

```rust
#[derive(IModel)]
#[component]
pub struct EditorView {
    pub content: SharedString,
    pub is_saved: bool,
    pub cursor_position: usize,
}

impl EditorView {
    #[command]
    pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        // Ctrl+S: 保存
        if ev.modifiers.control && ev.key == Key::Character('s') {
            ev.prevent_default();
            self.save(cx);
            return;
        }

        // Ctrl+C: 复制
        if ev.modifiers.control && ev.key == Key::Character('c') {
            self.copy_selection(cx);
            return;
        }

        // Ctrl+V: 粘贴
        if ev.modifiers.control && ev.key == Key::Character('v') {
            self.paste(cx);
            return;
        }

        // Ctrl+Z: 撤销
        if ev.modifiers.control && ev.key == Key::Character('z') {
            self.undo(cx);
            return;
        }

        // Ctrl+Shift+Z: 重做
        if ev.modifiers.control && ev.modifiers.shift && ev.key == Key::Character('z') {
            self.redo(cx);
            return;
        }

        // Escape: 退出编辑
        if ev.key == Key::Escape {
            self.exit_editing(cx);
            return;
        }
    }

    fn save(&mut self, cx: &mut ViewContext<Self>) {
        // 保存逻辑
        self.is_saved = true;
        cx.notify();
    }

    // ... 其他方法
}
```

## 5.2.11 小结

RML 的事件对象提供类型化的载荷访问：

| 事件对象           | 用途              | 关键属性                    |
| -------------- | --------------- | ----------------------- |
| `ClickEvent`   | 点击              | `position`、`button`、`modifiers` |
| `MouseEvent`   | 鼠标移动/悬停         | `position`、`button`     |
| `WheelEvent`   | 滚轮              | `delta`                 |
| `KeyDownEvent` | 键盘按下            | `key`、`modifiers`       |
| `KeyUpEvent`   | 键盘释放            | `key`、`modifiers`       |
| `InputEvent`   | 实时输入            | `value`、`old_value`     |
| `ChangeEvent`  | 值变化             | `value`、`old_value`     |
| `FocusEvent`   | 焦点              | `target`                |
| `SubmitEvent`  | 表单提交            | `form_data`             |

掌握这些事件对象，你就能在命令方法中获取完整的用户交互信息。

下一节 → [5.3 事件流](./event-flow.md)
