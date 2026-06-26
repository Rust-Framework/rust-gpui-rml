# 4.4 命令系统

> **本节目标**：完整掌握 RML 的命令系统——ICommand trait、`#[command]` 宏、命令参数、命令的启用条件。

## 4.4.1 命令的定义

命令（Command）是 ViewModel 中可被 UI 直接调用的方法。命令是 RML 事件处理的核心抽象：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}
```

```html
<button onclick={increment}>+1</button>
```

## 4.4.2 ICommand trait

RML 的命令系统基于 `ICommand` trait，借鉴自 WPF：

```rust
pub trait ICommand: Send + Sync {
    /// 命令是否可执行
    fn can_execute(&self) -> bool;

    /// 执行命令
    fn execute(&mut self, cx: &mut ViewContext<Self>);

    /// 命令可执行性变化时通知
    fn can_execute_changed(&self) -> Option<Subscription>;
}
```

`#[command]` 宏自动为标记的方法生成 `ICommand` 的实现，开发者通常不需要手动实现这个 trait。

## 4.4.3 `#[command]` 宏

`#[command]` 标记方法为命令，自动生成：

1. `ICommand` trait 的实现
2. 事件绑定的连接代码
3. 命令参数的解析代码

### 基础用法

```rust
#[command]
pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 业务逻辑
    cx.notify();
}
```

### 命令的签名

命令方法必须满足以下签名：

```rust
// 无参数命令
#[command]
pub fn method_name(&mut self, event: &EventType, cx: &mut ViewContext<Self>)

// 带参数命令
#[command]
pub fn method_name(&mut self, param: T, event: &EventType, cx: &mut ViewContext<Self>)

// 多参数命令
#[command]
pub fn method_name(&mut self, p1: T1, p2: T2, event: &EventType, cx: &mut ViewContext<Self>)
```

其中 `EventType` 是事件对象类型，如 `ClickEvent`、`ChangeEvent`、`KeyDownEvent` 等。

## 4.4.4 命令参数

### 无参数命令

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}
```

```html
<button onclick={increment}>+1</button>
```

### 单参数命令

```rust
#[command]
pub fn delete_item(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.items.retain(|i| i.id != id);
    cx.notify();
}
```

```html
<button onclick={delete_item, {item.id}}>删除</button>
```

### 多参数命令

```rust
#[command]
pub fn update_status(
    &mut self,
    id: u64,
    status: SharedString,
    _: &ClickEvent,
    cx: &mut ViewContext<Self>,
) {
    if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
        item.status = status;
        cx.notify();
    }
}
```

```html
<button onclick={update_status, {item.id}, 'completed'}>完成</button>
```

### 参数类型

命令参数可以是任意 `Send + Clone` 类型：

| 参数类型          | 在 `.rml` 中的写法                  |
| ------------- | ------------------------------ |
| `i32`、`u64` 等 | `onclick={fn, 42}`             |
| `SharedString` | `onclick={fn, 'hello'}` 或 `onclick={fn, {field}}` |
| `bool`        | `onclick={fn, true}`           |
| 自定义类型         | 需要实现 `FromStr` 或提供转换           |

## 4.4.5 命令的启用条件

### 用 `can_execute` 控制启用状态

```rust
#[command(can_execute = "can_increment")]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}

pub fn can_increment(&self) -> bool {
    self.count < 100  // 超过 100 时禁用
}
```

```html
<!-- 按钮在 can_increment 返回 false 时自动禁用 -->
<button onclick={increment}>+1</button>
```

### 用计算属性控制启用状态

也可以在 `.rml` 中用 `disabled` 属性：

```rust
#[computed]
pub fn can_submit(&self) -> bool {
    !self.user_name.is_empty() && !self.password.is_empty()
}
```

```html
<button disabled={!can_submit} onclick={submit}>提交</button>
```

### 两种方式的对比

| 方式                | 优点              | 缺点                  |
| ----------------- | --------------- | ------------------- |
| `can_execute`     | 命令与条件绑定，内聚      | 需要额外方法              |
| `disabled={...}`  | 灵活，可组合多个条件      | 条件与命令分离，可能不一致       |

💡 **最佳实践**：简单条件用 `disabled={}`，复杂业务规则用 `can_execute`。

## 4.4.6 命令与事件对象

命令方法接收事件对象作为参数，可以访问事件信息：

```rust
#[command]
pub fn on_click(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    println!("点击位置: {:?}", ev.position);
    self.count += 1;
    cx.notify();
}

#[command]
pub fn on_key_down(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    match ev.key {
        Key::Enter => self.submit(cx),
        Key::Escape => self.cancel(cx),
        _ => {}
    }
}
```

### 常用事件对象

| 事件对象           | 触发事件       | 常用属性                    |
| -------------- | ---------- | ----------------------- |
| `ClickEvent`   | `onclick`  | `position`、`button`     |
| `ChangeEvent`  | `onchange` | `value`                 |
| `InputEvent`   | `oninput`  | `value`                 |
| `KeyDownEvent` | `onkeydown`| `key`、`modifiers`       |
| `KeyUpEvent`   | `onkeyup`  | `key`、`modifiers`       |
| `MouseEvent`   | `onmouseenter` 等 | `position`、`buttons`    |
| `FocusEvent`   | `onfocus`、`onblur` | 无                       |

详见 [第 5 章 · 事件对象](../05-events/event-objects.md)。

## 4.4.7 命令的命名约定

### 用动词开头

```rust
// ✅ 动词开头，描述动作
#[command]
pub fn submit(&mut self, ...) { ... }

#[command]
pub fn delete_item(&mut self, ...) { ... }

#[command]
pub fn toggle_complete(&mut self, ...) { ... }

// ❌ 名词开头，描述状态
#[command]
pub fn button_click(&mut self, ...) { ... }  // 应为 submit 或 click_submit

// ❌ handle_ 前缀，冗余
#[command]
pub fn handle_click(&mut self, ...) { ... }  // 应为 submit
```

### 命名风格

- 用 `snake_case`
- 避免缩写：`delete_item` 而非 `del_item`
- 避免泛化：`submit` 而非 `click`、`action`

## 4.4.8 命令的复用

命令可以在多个事件中复用：

```html
<!-- 同一个命令绑定到不同事件 -->
<button onclick={submit}>提交</button>
<input onkeydown={on_enter_key} />

<!-- 也可以直接绑定到回车键 -->
<input onkeydown={submit_on_enter} />
```

```rust
#[command]
pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    // 提交逻辑
}

#[command]
pub fn submit_on_enter(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
    if ev.key == Key::Enter {
        self.submit(&ClickEvent::default(), cx);
    }
}
```

## 4.4.9 命令与异步操作

命令可以启动异步任务：

```rust
#[command]
pub fn load_data(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.is_loading = true;
    cx.notify();

    cx.spawn(|this, mut cx| async move {
        let data = fetch_data_from_server().await;

        let _ = this.update(&mut cx, |this, cx| {
            this.data = data;
            this.is_loading = false;
            cx.notify();
        });
    }).detach();
}
```

⚠️ **注意**：异步任务中修改 ViewModel 必须通过 `this.update`，不能直接捕获 `&mut self`。详见 [第 8 章 · 状态生命周期](../08-lifecycle/state-lifecycle.md)。

## 4.4.10 命令的测试

命令方法可以独立测试，无需 UI：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment() {
        let mut view = MyView::new();
        view.count = 5;

        // 直接调用命令方法（需要模拟 cx）
        // view.increment(&ClickEvent::default(), &mut cx);

        assert_eq!(view.count, 6);
    }

    #[test]
    fn test_can_increment() {
        let mut view = MyView::new();
        view.count = 100;
        assert!(!view.can_increment());

        view.count = 99;
        assert!(view.can_increment());
    }
}
```

详见 [第 9 章 · 可测试性设计](../09-architecture/testability.md)。

## 4.4.11 完整示例：购物车

```rust
#[derive(Model)]
pub struct CartItem {
    pub id: u64,
    pub name: SharedString,
    pub price: f64,
    pub quantity: u32,
}

#[derive(Model)]
#[component]
pub struct CartView {
    pub items: Vec<CartItem>,
    pub is_checking_out: bool,

    next_id: u64,
}

impl CartView {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            is_checking_out: false,
            next_id: 1,
        }
    }

    #[computed]
    pub fn total_price(&self) -> f64 {
        self.items.iter().map(|i| i.price * i.quantity as f64).sum()
    }

    #[computed]
    pub fn total_count(&self) -> u32 {
        self.items.iter().map(|i| i.quantity).sum()
    }

    #[computed]
    pub fn can_checkout(&self) -> bool {
        !self.items.is_empty() && !self.is_checking_out
    }

    #[command]
    pub fn add_item(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.items.push(CartItem {
            id: self.next_id,
            name: format!("商品 {}", self.next_id).into(),
            price: 99.0,
            quantity: 1,
        });
        self.next_id += 1;
        cx.notify();
    }

    #[command]
    pub fn increase_quantity(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            item.quantity += 1;
            cx.notify();
        }
    }

    #[command]
    pub fn decrease_quantity(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
            if item.quantity > 1 {
                item.quantity -= 1;
            } else {
                // 数量为 0 时移除
                self.items.retain(|i| i.id != id);
            }
            cx.notify();
        }
    }

    #[command]
    pub fn remove_item(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.items.retain(|i| i.id != id);
        cx.notify();
    }

    #[command]
    pub fn checkout(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if !self.can_checkout() {
            return;
        }

        self.is_checking_out = true;
        cx.notify();

        let total = self.total_price();

        cx.spawn(|this, mut cx| async move {
            let result = process_payment(total).await;

            let _ = this.update(&mut cx, |this, cx| {
                this.is_checking_out = false;
                match result {
                    Ok(_) => {
                        this.items.clear();
                        // 显示成功提示...
                    }
                    Err(e) => {
                        // 显示错误提示...
                        log::error!("结账失败: {}", e);
                    }
                }
                cx.notify();
            });
        }).detach();
    }
}

async fn process_payment(amount: f64) -> Result<(), String> {
    // 模拟支付
    Ok(())
}
```

## 4.4.12 小结

命令系统是 RML 事件处理的核心抽象：

- **`#[command]`**：标记方法为 UI 可调用的命令
- **命令参数**：支持无参、单参、多参
- **`can_execute`**：控制命令的启用状态
- **事件对象**：命令方法接收事件对象，可访问事件信息
- **异步支持**：命令可以启动异步任务

掌握命令系统，你就能把任何业务逻辑暴露给 UI 调用。

下一节 → [4.5 状态管理](./state-management.md)
