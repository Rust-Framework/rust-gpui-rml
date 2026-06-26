# 4.2 宏属性详解

> **本节目标**：完整掌握 RML 的全部宏属性——`#[window]`、`#[component]`、`#[command]`、`#[computed]`、`#[element]`、`#[on_loaded]`、`#[on_unloaded]`。

## 4.2.1 宏属性总览

| 宏属性              | 用途                         | 作用对象  |
| ---------------- | -------------------------- | ----- |
| `#[window]`      | 标记结构体为 RML 窗口的 Code-Behind | 结构体   |
| `#[component]`   | 标记结构体为自定义组件                | 结构体   |
| `#[command]`     | 标记方法为 UI 可调用的命令            | 方法    |
| `#[computed]`    | 标记为计算属性（依赖其他字段自动更新）        | 方法    |
| `#[on_loaded]`   | 视图加载完成后的回调                 | 方法    |
| `#[on_unloaded]` | 视图卸载前的清理回调                 | 方法    |
| `#[element]`     | 标记字段为 `ref` 引用的 UI 元素      | 字段    |

## 4.2.2 `#[window]`：标记窗口

`#[window]` 标记结构体为 RML 窗口的 ViewModel：

```rust
#[derive(IModel)]
#[window]
pub struct Counter {
    pub count: i32,
}
```

### 作用

- 关联 `.rml` 文件（默认按命名约定）
- 触发编译器生成 `Render` trait 实现
- 注册生命周期回调

### 参数

```rust
// 默认：按命名约定关联 counter.rml
#[window]

// 显式指定模板路径
#[window(template = "views/custom_counter.rml")]

// 指定生成的 Render 实现位置
#[window(generated_path = "OUT_DIR/views/counter.generated.rs")]
```

### 与 `#[component]` 的区别

| 特性     | `#[window]`         | `#[component]`           |
| ------ | ------------------- | ------------------------ |
| 用途     | 顶层窗口                | 可复用组件                    |
| 关联文件   | `.rml`              | `.rml`                   |
| 可被嵌套   | 否                   | 是                        |
| 插槽     | 不支持                 | 支持                       |
| 启动入口   | 可作为 `RmlApplication::main_window` 的入口 | 不能直接启动                   |

## 4.2.3 `#[component]`：标记组件

`#[component]` 标记结构体为自定义组件：

```rust
#[derive(IModel)]
#[component]
pub struct PrimaryButton {
    pub label: SharedString,
    pub on_click: Option<Arc<dyn Fn(&ClickEvent)>>,
}
```

详见 [第 6 章 · 组件系统](../06-components/INDEX.md)。

## 4.2.4 `#[command]`：标记命令

`#[command]` 标记方法为 UI 可调用的命令：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
    cx.notify();
}
```

### 命令的签名

命令方法必须满足以下签名之一：

```rust
// 无参数命令
#[command]
pub fn method_name(&mut self, _: &ClickEvent, cx: &mut Context<Self>)

// 带参数命令（参数在事件对象之前）
#[command]
pub fn method_name(&mut self, param: T, _: &ClickEvent, cx: &mut Context<Self>)

// 多参数命令
#[command]
pub fn method_name(&mut self, p1: T1, p2: T2, _: &ClickEvent, cx: &mut Context<Self>)
```

### 在 `.rml` 中调用

```html
<!-- 无参数 -->
<button onclick={increment}>+1</button>

<!-- 带参数 -->
<button onclick={delete_item, {item.id}}>删除</button>

<!-- 多参数 -->
<button onclick={update_status, {item.id}, 'completed'}>完成</button>
```

### 命令的命名约定

- 用**动词**开头：`increment`、`delete_item`、`update_status`
- 避免 `handle_` 前缀：`handle_click` → `submit`
- 避免名词：`button_click` → `submit`

## 4.2.5 `#[computed]`：标记计算属性

`#[computed]` 标记方法为计算属性，自动追踪依赖并缓存结果：

```rust
#[computed]
pub fn completed_count(&self) -> usize {
    self.todos.iter().filter(|t| t.done).count()
}
```

### 计算属性的签名

```rust
// 必须是 &self（只读）
#[computed]
pub fn method_name(&self) -> ReturnType

// 不能有参数
#[computed]
pub fn bad_computed(&self, x: i32) -> i32  // ❌
```

### 在 `.rml` 中访问

```html
<!-- 像字段一样访问，无需括号 -->
<p>{completed_count}</p>
```

详见 [第 3 章 · 计算属性](../03-binding/computed.md)。

## 4.2.6 `#[on_loaded]`：视图加载回调

`#[on_loaded]` 标记视图加载完成后调用的方法：

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    // 加载初始数据
    self.load_initial_data(cx);
}
```

### 调用时机

```
1. ViewModel::new() 创建实例
2. GPUI 创建 Entity
3. 视图首次渲染
4. #[on_loaded] 被调用  ← 这里
5. 用户交互...
```

### 典型用途

- 加载初始数据
- 启动定时器
- 建立网络连接
- 订阅外部事件源

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
    // 加载本地存储
    self.load_from_storage(cx);

    // 订阅全局事件
    cx.observe_global::<AppTheme>(&mut |this, cx| {
        this.theme = cx.global::<AppTheme>().clone();
        cx.notify();
    }).detach();

    // 启动自动保存定时器
    cx.spawn(|this, mut cx| async move {
        loop {
            cx.background_executor().timer(Duration::from_secs(60)).await;
            let _ = this.update(&mut cx, |this, cx| this.auto_save(cx));
        }
    }).detach();
}
```

## 4.2.7 `#[on_unloaded]`：视图卸载回调

`#[on_unloaded]` 标记视图卸载前调用的方法：

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut Context<Self>) {
    // 清理资源
    self.save_to_storage();
    self.cancel_pending_requests();
}
```

### 调用时机

```
1. 用户交互...
2. 视图即将卸载
3. #[on_unloaded] 被调用  ← 这里
4. Entity 被销毁
```

### 典型用途

- 保存状态到本地存储
- 取消未完成的异步任务
- 关闭文件句柄、网络连接
- 取消订阅

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut Context<Self>) {
    // 保存状态
    if let Err(e) = self.save_state() {
        log::error!("保存状态失败: {}", e);
    }

    // 取消异步任务
    if let Some(handle) = self.pending_task.take() {
        handle.abort();
    }
}
```

## 4.2.8 `#[element]`：标记元素引用

`#[element]` 标记字段为 `ref` 引用的 UI 元素：

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub user_name: SharedString,

    #[element]
    pub username_input: ElementRef<Input>,

    #[element]
    pub submit_btn: ElementRef<Button>,
}
```

```html
<!-- .rml 中用 ref 关联 -->
<input ref="username_input" model={user_name} />
<button ref="submit_btn" onclick={submit}>提交</button>
```

详见 [4.3 元素引用](./element-ref.md)。

## 4.2.9 宏属性的组合

多个宏属性可以组合使用：

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub count: i32,
    pub user_name: SharedString,

    #[element]
    pub username_input: ElementRef<Input>,
}

impl MyView {
    pub fn new() -> Self {
        Self {
            count: 0,
            user_name: SharedString::default(),
            username_input: ElementRef::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut Context<Self>) {
        self.username_input.focus(cx);
    }

    #[command]
    pub fn submit(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.user_name.is_empty() {
            return;
        }
        self.count += 1;
        cx.notify();
    }

    #[computed]
    pub fn display_count(&self) -> SharedString {
        format!("提交次数: {}", self.count).into()
    }

    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut Context<Self>) {
        log::info!("视图卸载，最终计数: {}", self.count);
    }
}
```

## 4.2.10 宏属性的常见错误

### 错误一：忘记 `#[derive(IModel)]`

```rust
// ❌ 缺少 Model 派生
#[component]
pub struct MyView {
    pub count: i32,
}
```

### 错误二：`#[command]` 方法签名错误

```rust
// ❌ 缺少 cx 参数
#[command]
pub fn bad_command(&mut self) {
    self.count += 1;
}

// ✅ 正确签名
#[command]
pub fn good_command(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
    cx.notify();
}
```

### 错误三：`#[computed]` 用了 `&mut self`

```rust
// ❌ 计算属性不能修改 self
#[computed]
pub fn bad_computed(&mut self) -> i32 {
    self.count += 1;
    self.count
}

// ✅ 必须是 &self
#[computed]
pub fn good_computed(&self) -> i32 {
    self.count + 1
}
```

### 错误四：`#[element]` 字段类型错误

```rust
// ❌ 不是 ElementRef
#[element]
pub username_input: Input,

// ✅ 必须是 ElementRef<T>
#[element]
pub username_input: ElementRef<Input>,
```

## 4.2.11 小结

RML 的 7 个宏属性是 `.rml.rs` 的核心工具：

- **结构体级**：`#[window]`、`#[component]`
- **方法级**：`#[command]`、`#[computed]`、`#[on_loaded]`、`#[on_unloaded]`
- **字段级**：`#[element]`

记住每个宏的作用和签名要求，你就能写出规范的 ViewModel。

下一节 → [4.3 元素引用](./element-ref.md)
