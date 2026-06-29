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

### 声明式根节点配置

`#[window]` 不接受任何参数。窗口属性在 `.rml` 根节点上声明式配置：

```text
<!-- 默认：透明标题栏（现代风格） -->
<window title="My App" width="800" height="600">
    <!-- 子元素 -->
</window>

<!-- 原生标题栏（系统风格） -->
<modern_window title="My App" width="800" height="600">
    <!-- 子元素 -->
</modern_window>
```

属性说明：
- `title`：窗口标题（字符串）
- `width` / `height`：窗口尺寸（像素，整数）

若未指定属性，使用默认值：`title="RML Window"`, `width=800`, `height=600`。

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
    pub on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}
```

### 宏自动注入的字段

`#[component]`（和 `#[window]`）会为每个 `pub` 字段自动注入以下私有字段，用于实现响应式数据绑定与计算属性缓存：

| 注入字段 | 数量 | 用途 |
|---|---|---|
| `__rml_<field>_version: AtomicU64` | 每个 `pub` 字段一个 | 字段版本号计数器，`#[command]` 修改字段时自动 `fetch_add` |
| `__rml_computed_cache: ComputedCache` | 每结构体一个 | `#[computed]` 方法结果缓存，按方法名索引 |
| `__rml_input_states: HashMap<String, Entity<InputState>>` | 每结构体一个 | 双向绑定的 `InputState` entity 存储（按字段名索引） |
| `__rml_input_state_versions: HashMap<String, u64>` | 每结构体一个 | 双向绑定正向同步的版本号追踪 |

这些字段均为私有，不会进入 `IModel::rml_fields()`（其只收集 `pub` 字段）。所有注入字段均实现 `Default`，与 `#[derive(Default)]` 兼容。

> ⚠️ **注意**：`Subscription` 不会作为字段存储。`cx.subscribe` 返回的 `Subscription` 调用 `.detach()` 后随 entity 生命周期存活。这是因为 `Subscription` 内部含 `Box<dyn FnOnce() + 'static>`，不满足 `Send + Sync`，存储会导致视图类型无法满足 `open_window` 的 `Send + Sync` 约束。

详见 [第 6 章 · 组件系统](../06-components/INDEX.md)。

## 4.2.4 `#[command]`：标记命令

`#[command]` 标记方法为 UI 可调用的命令。**宏会自动注入字段版本号追踪与 `cx.notify()` 调用**，用户无需手写：

```rust
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
    // 宏自动注入：
    //   self.__rml_bump_version("count");
    //   cx.notify();
}
```

### 4.2.4.1 自动行为

`#[command]` 通过 `syn::visit::Visit` 遍历方法体，识别所有 `self.<field> = ...` 和 `self.<field> += ...` 等赋值/复合赋值操作，自动注入以下代码：

1. **字段修改后**：`self.__rml_bump_version("<field>");`（为每个被修改的 `pub` 字段注入一次）
2. **方法末尾**：`cx.notify();`（仅当方法返回 `()` 且存在 `&mut Context<Self>` 参数时）

**支持的复合赋值运算符**：`=`、`+=`、`-=`、`*=`、`/=`、`%=`、`&=`、`|=`、`^=`、`<<=`、`>>=`

```rust
#[command]
pub fn update_user(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.name = "Alice".into();        // 注入：bump_version("name")
    self.login_count += 1;              // 注入：bump_version("login_count")
    self.score *= 2;                   // 注入：bump_version("score")
    // 方法末尾自动注入：cx.notify();
}
```

> ⚠️ **限制**：字段修改检测基于 AST 模式匹配，不追踪借用的指针间接修改（如 `let p = &mut self.x; *p = 1;`）。若需此类模式，请手动调用 `self.__rml_bump_version("x")`。

### 4.2.4.2 `no_notify` 参数：禁用自动 notify

默认行为下，`#[command]` 会在方法末尾自动追加 `cx.notify()`。但在某些场景下，你可能不希望立即触发重绘：

```rust
// 默认：自动注入 bump_version + cx.notify()
#[command]
pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
}

// 禁用自动 notify（仍注入 bump_version）
#[command(no_notify)]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.a = 1;
    self.b = 2;
    // 用户在适当时机手动调用 cx.notify()
    if self.should_refresh {
        cx.notify();
    }
}
```

**使用 `no_notify` 的场景**：
- 批量更新多个字段后，由用户决定 notify 时机
- 异步任务中连续修改字段，避免每次修改都触发重绘
- 返回类型非 `()` 时（宏不会为非 `()` 返回类型注入 notify，此时无需 `no_notify`）

### 4.2.4.3 命令的签名

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

### 4.2.5.1 版本追踪机制

`#[computed]` 的缓存基于字段版本号追踪：

1. **字段版本号**：每个 `pub` 字段注入 `__rml_<field>_version: AtomicU64`，`#[command]` 修改字段时自动 `fetch_add`
2. **依赖版本和**：`#[computed]` 方法调用时计算依赖字段的版本号之和（`__rml_computed_deps_version`），若与缓存时的版本号之和不同则重算
3. **缓存存储**：结果存入 `__rml_computed_cache: ComputedCache`（`Mutex<HashMap<String, (u64, Box<dyn Any>)>>`），按方法名索引

```
#[computed] pub fn summary(&self) -> String {
    format!("{} / {}", self.name, self.count)
}

// 等价于：
pub fn summary(&self) -> String {
    let dep_version = self.__rml_get_version("name") + self.__rml_get_version("count");
    self.__rml_computed_cache.get_or_compute::<String>(
        "summary",
        dep_version,
        || format!("{} / {}", self.name, self.count)
    )
}
```

### 4.2.5.2 依赖自动追踪

`build.rs` 中的 `scan_computed_methods()` 会扫描 `.rml.rs` 文件中所有 `#[computed]` 方法的方法体，识别 `self.<field>` 访问，自动建立依赖关系：

- 扫描 `self.<ident>` 模式的字段访问（包括 `self.count`、`self.name` 等）
- 支持 `format!`/`println!`/`vec!` 等宏参数内的字段访问
- 依赖信息存入 `CodegenCtx.computed_deps`，codegen 生成 `__rml_computed_deps_version` 方法

### 4.2.5.3 ComputedCache 实现

`ComputedCache` 使用 `Mutex<HashMap<String, (u64, Box<dyn Any>)>>` 存储：

- **key**：方法名（如 `"summary"`）
- **value**：`(dep_version_sum, Box<dyn Any>)`，`dep_version_sum` 是依赖字段的版本号之和
- `get_or_compute::<T: Clone + 'static>(&self, name, version, f)`：若版本号匹配则返回缓存，否则调用 `f` 计算并缓存

> ⚠️ **`unsafe impl Send + Sync`**：`ComputedCache` 内部 `Box<dyn Any>` 存储的类型可能不满足 `Send`（如 `Vec<MenuItem>` 含 `Rc<dyn Fn>`），但 `#[computed]` 仅在 render 线程调用，通过 `Mutex` 保证同步安全。`crates/core/src/lib.rs` 使用 `#![deny(unsafe_code)]` 而非 `#![forbid(unsafe_code)]` 以允许此处的局部 `#[allow(unsafe_code)]`。

### 4.2.5.4 计算属性的签名

```rust
// 必须是 &self（只读）
#[computed]
pub fn method_name(&self) -> ReturnType

// 不能有参数
#[computed]
pub fn bad_computed(&self, x: i32) -> i32  // ❌

// 不能用 &mut self
#[computed]
pub fn bad_computed(&mut self) -> i32  // ❌
```

**返回类型要求**：必须实现 `Clone + 'static`。常见支持类型：`String`、`SharedString`、`usize`、`i32`、`bool`、`Vec<T>` 等。

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
        // 宏自动注入：self.__rml_bump_version("count"); cx.notify();
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
// ❌ 缺少 cx 参数（宏无法注入 notify）
#[command]
pub fn bad_command(&mut self) {
    self.count += 1;
}

// ✅ 正确签名（宏自动注入 bump_version + notify）
#[command]
pub fn good_command(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
}
```

> 注：若 `#[command]` 方法缺少 `&mut Context<Self>` 参数，宏仍会注入 `bump_version`（因为不依赖 `cx`），但不会注入 `cx.notify()`。此时需手动调用 `cx.notify()` 触发重绘。

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
