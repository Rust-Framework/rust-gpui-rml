# RML（Rust Markup Language）设计文档

> **为 GPUI 框架设计的 HTML 友好型声明式 UI 框架**  
> 核心理念：独立标记文件（`.rml`）+ 纯 Rust 业务逻辑（`.rml.rs`）  
> 对标 WPF XAML 的设计精髓，拥抱 HTML 的语法亲和力

## 一、设计哲学

### 1.1 背景：GPUI 的机遇与挑战

GPUI 是 Zed 编辑器团队开发的**混合即时与保留模式、GPU 加速**的 Rust UI 框架。它提供了高层级的声明式 UI 视图系统——所有 UI 从实现 `Render` trait 的 View 开始，框架在每帧调用 `render` 方法构建 UI 树。GPUI 拥有 Tailwind 风格的样式 API 和 GPU 加速渲染能力，已被证明能够支撑 Zed 这样大规模、高性能的桌面应用。

然而，GPUI 当前的 UI 开发模式存在显著痛点：

```rust
// 原生 GPUI 的 UI 构建方式 —— 命令式链式调用
div()
    .flex()
    .flex_col()
    .gap(px(16.0))
    .p(px(24.0))
    .bg(rgb(0xf5f5f5))
    .child(
        div()
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(Label::new("Hello World"))
    )
    .child(
        div()
            .flex()
            .gap(px(8.0))
            .child(
                Button::new("Click me")
                    .on_click(cx.listener(|this, _ev, cx| {
                        this.count += 1;
                        cx.notify();
                    }))
            )
    )
```

**核心问题**：

1. **UI 逻辑深度耦合**：UI 结构与业务逻辑、事件处理交织在同一个 Rust 文件中
2. **代码冗长**：链式调用的嵌套结构难以阅读和维护
3. **设计师无法参与**：UI 代码是 Rust 语法，非工程师无法理解和编辑
4. **缺乏标准化**：没有统一的 UI 标记语言，每个项目各自为政

### 1.2 设计目标

RML 旨在彻底解决上述问题，为 GPUI 带来工业化级别的 UI 开发体验：

| 目标            | 说明                                |
| ------------- | --------------------------------- |
| **关注点彻底分离**   | UI 结构（`.rml`）与业务逻辑（`.rml.rs`）完全独立 |
| **HTML 语法亲和** | 使用标准 HTML 标签和属性，Web 开发者零学习成本      |
| **WPF 级数据绑定** | 支持单向/双向绑定、值转换、命令系统                |
| **零运行时开销**    | 编译期将 `.rml` 转换为原生 GPUI 渲染代码       |
| **设计师友好**     | 纯标记语言，可使用任何 XML/HTML 编辑工具         |
| **热重载就绪**     | 独立文件为运行时监听和热更新提供天然基础              |

### 1.3 核心原则

RML 的设计遵循三条根本原则：

**① 声明式优于命令式** —— 描述“是什么 UI”，而非“如何构建 UI”

- HTML 的 `<div class="flex">` 比 `div().flex()` 更接近人类认知
- UI 结构一目了然，而非逐行推导

**② 逻辑与表现彻底分离** —— UI 文件不含业务逻辑，逻辑代码不含 UI 构造

- `.rml` 纯视图声明，`.rml.rs` 纯业务逻辑
- 设计师与工程师可并行工作，互不干扰

**③ HTML 语法优先** —— 降低 Web 开发者进入 Rust 桌面的门槛

- 标准 HTML 标签：`div`、`button`、`input`、`p`、`ul`、`li`
- 标准 HTML 属性：`class`、`id`、`style`、`placeholder`、`type`
- 标准事件模型：`onclick`、`oninput`、`onkeydown`

### 1.4 与现有方案的对比

| 维度       | 原生 GPUI       | gpui-rsx      | **RML（本方案）**     |
| -------- | ------------- | ------------- | ---------------- |
| UI 定义位置  | Rust 代码内      | Rust 代码内（宏）   | **独立 `.rml` 文件** |
| 语法风格     | 链式调用          | JSX-like      | **纯 HTML/XML**   |
| UI/逻辑分离  | 无             | 弱             | **强（文件级分离）**     |
| 设计师可参与   | 否             | 否             | **是**            |
| 热重载      | 否             | 否             | **是**            |
| 学习曲线     | 陡             | 中             | **平缓**           |
| 代码量      | 100%          | ~50%          | **~35%**         |
| IDE 工具支持 | Rust-analyzer | Rust-analyzer | **XML 工具链全支持**   |

## 二、架构总览

### 2.1 三层架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Presentation Layer                            │
│  ┌──────────────────┐    ┌──────────────────────────────────────┐   │
│  │   window.rml     │    │        window.rml.rs                 │   │
│  │   (UI 标记)      │───▶│   (Code-Behind 业务逻辑)            │   │
│  │   - 控件树       │    │   - 状态字段 (Model)                │   │
│  │   - 布局属性     │    │   - 事件处理器                      │   │
│  │   - 数据绑定     │    │   - 计算属性                        │   │
│  │   - 事件绑定     │    │   - 生命周期回调                    │   │
│  └──────────────────┘    └──────────────────────────────────────┘   │
│           │                              │                          │
│           ▼                              ▼                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              RML Compiler (build.rs / 过程宏)                │   │
│  │   .rml 解析 → AST 转换 → 语义验证 → GPUI 代码生成           │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Framework Layer                               │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐   │
│  │     GPUI      │  │gpui-component │  │    RML Runtime        │   │
│  │  (渲染引擎)   │  │  (组件库)     │  │   (绑定系统/热重载)   │   │
│  └───────────────┘  └───────────────┘  └───────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### 2.2 Workspace 结构

```
rml-framework/
├── Cargo.toml                          # Workspace 根配置
├── crates/
│   ├── core/                           # RML 框架核心
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # 核心 trait 导出
│   │       ├── view_model.rs           # ViewModel trait 定义
│   │       ├── binding.rs              # 绑定引擎
│   │       ├── command.rs              # 命令系统 (ICommand)
│   │       ├── converter.rs            # 值转换器 trait
│   │       └── lifecycle.rs            # 视图生命周期回调
│   │
│   ├── rml/                            # RML 解析引擎
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── parser/                 # 语法解析器
│   │       │   ├── mod.rs
│   │       │   ├── tokenizer.rs        # HTML/XML 词法分析
│   │       │   └── ast.rs              # 抽象语法树
│   │       ├── compiler/               # 编译器 (.rml → Rust)
│   │       │   ├── mod.rs
│   │       │   ├── codegen.rs          # 代码生成器
│   │       │   └── validator.rs        # 语义验证
│   │       └── runtime/                # 运行时支持
│   │           ├── mod.rs
│   │           └── watcher.rs           # 热重载文件监听
│   │
│   ├── macros/                         # 过程宏定义
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                  # 宏导出入口
│   │       ├── view.rs                 # #[view] 属性宏
│   │       ├── component.rs            # #[component] 组件宏
│   │       ├── command.rs              # #[command] 命令宏
│   │       ├── computed.rs             # #[computed] 计算属性宏
│   │       └── utils.rs                # 宏通用工具
│   │
│   └── app/                            # RML 应用框架
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs                  # 框架入口
│           ├── application.rs          # 应用启动器
│           ├── window.rs               # 窗口管理
│           └── resources.rs            # 资源加载
│
├── demo/                               # 示例项目
│   ├── Cargo.toml
│   ├── build.rs                        # 构建脚本
│   └── src/
│       ├── main.rs
│       ├── views/
│       │   ├── counter.rml
│       │   ├── counter.rml.rs
│       │   ├── todo.rml
│       │   └── todo.rml.rs
│       ├── components/
│       │   ├── button.rml
│       │   └── button.rml.rs
│       └── styles/
│           └── theme.rml
│
└── target/
    └── generated/                      # .rml 编译生成的 .rs 文件
        └── rml/
```

### 2.3 数据流架构（MVVM 模式）

RML 完全对标 WPF 的 MVVM 模式：

```
┌──────────┐    数据绑定     ┌──────────┐    渲染     ┌──────────┐
│  Model   │ ◀────────────▶ │ ViewModel │ ─────────▶ │   View   │
│ (数据)   │    (双向/单向)  │  (状态)   │   (GPUI)   │  (.rml)  │
└──────────┘                └──────────┘            └──────────┘
      │                            │                       │
      ▼                            ▼                       ▼
  业务逻辑                     命令/事件                用户交互
  (Rust)                      (Rust)                  (点击/输入)
```

- **Model**：纯 Rust 数据结构，不含 UI 逻辑
- **ViewModel**：实现 `Model` trait 的 GPUI Entity，持有状态并响应命令
- **View**：`.rml` 文件定义的 UI 结构，通过数据绑定消费 ViewModel 状态
- **Code-Behind**（`.rml.rs`）：天然承担 ViewModel 角色

## 三、HTML 友好型 RML 语法规范

### 3.1 设计原则：让 HTML 开发者感到亲切

RML 语法遵循 **“HTML 优先，增强属性”** 的策略：

- ✅ 使用标准 HTML 标签：`div`、`button`、`input`、`span`、`p`、`ul`、`li`、`img`、`a`
- ✅ 使用标准 HTML 属性：`class`、`id`、`style`、`placeholder`、`type`、`src`、`href`
- ✅ 使用标准的 `{ }` 插值表达式（类似 React/Vue）
- ✅ 通过极简指令扩展能力：`if`、`each`、`model`（**无任何框架前缀**）

### 3.2 基础示例：计数器应用

```html
<!-- counter.rml —— 完全像在写 HTML -->
<div class="counter-container">
    <h1 class="counter-title">⚡ RML 计数器</h1>

    <div class="counter-display">
        <span class="counter-value">{count}</span>
        <span class="counter-status" if={count > 10}>
            🚀 超过十啦！
        </span>
    </div>

    <div class="counter-buttons">
        <button class="btn primary" onclick={increment}>➕ 增加</button>
        <button class="btn danger" onclick={decrement} if={count > 0}>➖ 减少</button>
        <button class="btn secondary" onclick={reset}>↺ 重置</button>
    </div>

    <div class="counter-history" if={!history.is_empty()}>
        <p>操作记录：</p>
        <ul>
            <li each={entry in history} key={entry.id}>
                第 {entry.id} 次: {entry.operation} → {entry.result}
            </li>
        </ul>
    </div>
</div>
```

### 3.3 标签与控件映射

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

### 3.4 属性系统

#### 3.4.1 标准 HTML 属性（直接透传）

```html
<!-- 类名（支持 Tailwind 风格，直接映射到 GPUI 样式） -->
<div class="flex flex-col gap-4 p-6 bg-white rounded-lg shadow-md">

<!-- ID（用于 Code-Behind 引用） -->
<input id="username-input" type="text" placeholder="请输入用户名">

<!-- 行内样式 -->
<p style="color: blue; font-size: 16px;">内联样式</p>

<!-- 标准输入属性 -->
<input type="password" placeholder="密码" maxlength="20" disabled>
```

#### 3.4.2 数据绑定（`{ }` 插值）

```html
<!-- 单向绑定：显示数据 -->
<p>欢迎, {user_name}</p>

<!-- 属性绑定 -->
<div class={container_class}>动态类名</div>
<input value={user_name}>

<!-- 双向绑定（model 指令） -->
<input model={user_name} placeholder="输入姓名">
<!-- 等价于：value={user_name} + oninput={update_user_name} -->
```

#### 3.4.3 事件绑定（标准 `on*` 事件）

```html
<!-- 点击事件 -->
<button onclick={submit}>提交</button>

<!-- 直接调用 Code-Behind 方法 -->
<button onclick="handle_click">方法名绑定</button>

<!-- 带参数的事件 -->
<button onclick="delete_item, {item.id}">删除</button>

<!-- 键盘事件 -->
<input onkeydown="on_enter_key" onkeyup="validate_input">

<!-- 鼠标事件 -->
<div onmouseenter="show_tooltip" onmouseleave="hide_tooltip">
   悬停我
</div>
```

#### 3.4.4 极简指令（无任何框架前缀）

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

### 3.5 完整示例：Todo 应用

```html
<!-- todo.rml —— 完整的待办清单应用 -->
<div class="todo-app">
    <!-- 应用标题 -->
    <h1 class="app-title">📋 待办清单</h1>

    <!-- 输入区域 -->
    <div class="input-area">
        <input 
            class="todo-input"
            type="text"
            placeholder="输入新任务..."
            model={new_todo_text}
            onkeydown="on_enter_key"
        />
        <button class="btn-add" onclick={add_todo}>添加</button>
    </div>

    <!-- 统计信息 -->
    <div class="stats">
        <span>总计: {todos.len()}</span>
        <span>已完成: {completed_count}</span>
        <span>待办: {pending_count}</span>
    </div>

    <!-- 任务列表 -->
    <ul class="todo-list">
        <li each={todo in todos} key={todo.id} class="todo-item">
            <input 
                type="checkbox" 
                checked={todo.done}
                onchange={toggle_todo, {todo.id}}
            />
            <span class={todo.done ? "done" : ""}>
                {todo.text}
            </span>
            <button class="btn-delete" onclick={delete_todo, {todo.id}}>
                ✕
            </button>
        </li>
    </ul>

    <!-- 空状态 -->
    <div if={todos.is_empty()}>
        <p class="empty-hint">🎉 暂无任务，添加一条吧！</p>
    </div>
</div>
```

## 四、Code-Behind（`.rml.rs`）

### 4.1 基础结构

```rust
// todo.rml.rs
use rml::prelude::*;

#[derive(Model)]
pub struct TodoItem {
    pub id: u64,
    pub text: SharedString,
    pub done: bool,
}

#[derive(Model)]
#[view]  // 极简宏：标记为 RML 视图
pub struct TodoViewModel {
    pub new_todo_text: SharedString,
    pub todos: Vec<TodoItem>,
    next_id: u64,
}

impl TodoViewModel {
    pub fn new() -> Self {
        Self {
            new_todo_text: SharedString::default(),
            todos: Vec::new(),
            next_id: 1,
        }
    }

    // 计算属性：自动追踪依赖，UI 自动更新
    #[computed]
    pub fn completed_count(&self) -> usize {
        self.todos.iter().filter(|t| t.done).count()
    }

    #[computed]
    pub fn pending_count(&self) -> usize {
        self.todos.len() - self.completed_count()
    }

    // 命令方法：UI 可直接调用
    #[command]
    pub fn add_todo(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.new_todo_text.is_empty() {
            return;
        }
        self.todos.push(TodoItem {
            id: self.next_id,
            text: self.new_todo_text.clone(),
            done: false,
        });
        self.next_id += 1;
        self.new_todo_text = SharedString::default();
        cx.notify();  // 触发 UI 重绘
    }

    #[command]
    pub fn toggle_todo(&mut self, id: u64, _: &ChangeEvent, cx: &mut ViewContext<Self>) {
        if let Some(todo) = self.todos.iter_mut().find(|t| t.id == id) {
            todo.done = !todo.done;
            cx.notify();
        }
    }

    #[command]
    pub fn delete_todo(&mut self, id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.todos.retain(|t| t.id != id);
        cx.notify();
    }

    // 键盘事件处理方法
    pub fn on_enter_key(&mut self, ev: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        if ev.key == Key::Enter {
            self.add_todo(&ClickEvent::default(), cx);
        }
    }
}
```

### 4.2 宏属性说明

| 宏属性              | 用途                         |
| ---------------- | -------------------------- |
| `#[view]`        | 标记结构体为 RML 视图的 Code-Behind |
| `#[component]`   | 标记结构体为自定义组件                |
| `#[command]`     | 标记方法为 UI 可调用的命令            |
| `#[computed]`    | 标记为计算属性（依赖其他字段自动更新）        |
| `#[on_loaded]`   | 视图加载完成后的回调                 |
| `#[on_unloaded]` | 视图卸载前的清理回调                 |
| `#[element]`     | 标记字段为 `ref` 引用的 UI 元素      |

### 4.3 元素引用（`ref`）

```html
<!-- 在 .rml 中 -->
<input ref="username_input" model={user_name} />
<button ref="submit_btn" onclick={submit}>提交</button>
```

```rust
// 在 .rml.rs 中
#[derive(Model)]
#[view]
pub struct MyView {
    #[element]
    pub username_input: ElementRef<Input>,
    #[element]
    pub submit_btn: ElementRef<Button>,
}

impl MyView {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 自动聚焦输入框
        self.username_input.focus(cx);
    }
}
```

### 4.4 生命周期回调

```rust
impl MyView {
    /// 视图加载完成时调用
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.load_initial_data(cx);
    }

    /// 视图即将卸载时调用
    #[on_unloaded]
    pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
        // 清理资源、取消订阅等
    }
}
```

## 五、组件系统

### 5.1 自定义组件

```html
<!-- components/primary_button.rml -->
<button class="btn-primary" onclick={on_click}>
    <span class="btn-content">
        <slot name="icon"></slot>
        <slot>{label}</slot>
    </span>
</button>
```

```rust
// components/primary_button.rml.rs
use rml::prelude::*;

#[derive(Model)]
#[component(template = "components/primary_button.rml")]
pub struct PrimaryButton {
    pub label: SharedString,
    pub on_click: Option<Arc<dyn Fn(&ClickEvent)>>,
}

impl PrimaryButton {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { 
            label: label.into(), 
            on_click: None 
        }
    }
}
```

### 5.2 组件使用

```html
<!-- 像使用 HTML 标签一样使用自定义组件 -->
<div>
    <PrimaryButton label="保存" on_click={save_command} />
    <PrimaryButton label="删除" on_click={delete_command} />

    <!-- 带插槽的组件 -->
    <Card>
        <div slot="header">自定义头部</div>
        <p>卡片主体内容</p>
        <div slot="footer">自定义底部</div>
    </Card>
</div>
```

## 六、样式与主题

### 6.1 样式系统

RML 支持 WPF 风格的样式系统：

```html
<!-- styles.rml -->
<ResourceDictionary>
    <!-- 具名样式 -->
    <Style key="Heading1" target="TextBlock">
        <Setter property="FontSize" value="28" />
        <Setter property="FontWeight" value="Bold" />
        <Setter property="Foreground" value="text-primary" />
    </Style>

    <!-- 隐式样式（自动应用于所有目标类型） -->
    <Style target="Button">
        <Setter property="Padding" value="8, 12" />
        <Setter property="CornerRadius" value="4" />
    </Style>

    <!-- 样式继承 -->
    <Style key="DangerButton" target="Button" based_on="Button">
        <Setter property="Background" value="red-500" />
        <Setter property="Foreground" value="white" />
    </Style>
</ResourceDictionary>
```

### 6.2 使用样式

```html
<!-- 应用样式 -->
<button style="{StaticResource DangerButton}">删除</button>
<TextBlock style="{StaticResource Heading1}">标题</TextBlock>
```

## 七、构建流程

### 7.1 Build.rs 集成

```rust
// demo/build.rs
use rml_compiler::RmlBuild;

fn main() {
    RmlBuild::new()
        .input_dir("src/views")
        .input_dir("src/components")
        .output_dir(std::env::var("OUT_DIR").unwrap())
        .with_watch(true)   // 开发时监听变化
        .compile()
        .unwrap();

    println!("cargo:rerun-if-changed=src/views");
    println!("cargo:rerun-if-changed=src/components");
}
```

### 7.2 编译流程

```
┌─────────────────────────────────────────────────────────────────┐
│                    编译时流程 (cargo build)                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. build.rs 执行                                              │
│     ├── 扫描 src/**/*.rml 文件                                 │
│     ├── 解析 RML 为 AST                                        │
│     ├── 验证语法和类型（绑定路径检查）                         │
│     ├── 生成对应的 .rml.generated.rs 文件                      │
│     └── 输出到 OUT_DIR/                                        │
│                                                                 │
│  2. Rust 编译器执行                                            │
│     ├── 编译 .rml.rs（手写逻辑）                               │
│     ├── include! 生成的 .rml.generated.rs（UI 渲染代码）       │
│     └── 链接为最终二进制                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 7.3 生成的代码示例

```rust
// 由 RML 编译器自动生成 —— 开发者无需手动维护
// OUT_DIR/views/todo.generated.rs

impl RmlView for TodoViewModel {
    const VIEW_PATH: &'static str = "views/todo.rml";
}

impl Render for TodoViewModel {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // 从 .rml 生成的 GPUI 渲染代码
        gpui::div()
            .class("todo-app")
            .child(
                gpui::div()
                    .class("input-area")
                    .child(
                        gpui_component::Input::new()
                            .placeholder("输入新任务...")
                            .value(self.new_todo_text.clone())
                            .on_change(cx.listener(|this, value, cx| {
                                this.new_todo_text = value;
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, ev, cx| {
                                if ev.key == Key::Enter {
                                    this.add_todo(&ClickEvent::default(), cx);
                                }
                            }))
                    )
                    .child(
                        gpui_component::Button::new("添加")
                            .on_click(cx.listener(|this, ev, cx| {
                                this.add_todo(ev, cx);
                            }))
                    )
            )
            // ... 统计、列表、条件渲染等全部自动生成
    }
}
```

## 八、热重载

### 8.1 设计原理

由于 `.rml` 是独立的外部文件，RML 可以在开发模式下监听文件变更：

```
┌─────────────────────────────────────────────────────────────────┐
│                      热重载工作流                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. 开发者修改 .rml 文件                                       │
│     ↓                                                          │
│  2. 文件监听器检测到变更                                       │
│     ↓                                                          │
│  3. 增量编译：仅重新编译变更的 .rml                            │
│     ↓                                                          │
│  4. 生成新的 Rust 代码                                         │
│     ↓                                                          │
│  5. 通过 GPUI 的 Entity 机制触发热重绘                         │
│     ↓                                                          │
│  6. UI 实时更新，无需重新编译 Rust 代码                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 8.2 启用热重载

```rust
// main.rs
fn main() {
    RmlApplication::new()
        .with_hot_reload(vec!["src/views".into(), "src/components".into()])
        .run::<TodoViewModel>()
        .unwrap();
}
```

## 九、与 GPUI 核心概念的映射

### 9.1 Entity 模型

GPUI 中所有状态都由 `App` 统一管理，通过 `Entity` 句柄访问。RML 的 Code-Behind 结构体天然对应 GPUI 的 `Model` trait：

```rust
#[derive(Model)]  // 使结构体成为 GPUI Entity
#[view]
pub struct MyView {
    pub count: u32,  // 自动成为响应式状态
}

// 状态变更时调用 cx.notify() 触发重绘
```

### 9.2 Render Trait

GPUI 的 `Render` trait 是 UI 渲染的核心。RML 编译器自动生成 `Render::render` 方法的实现，开发者无需手写：

```rust
// 开发者无需手写 —— RML 编译器自动生成
impl Render for MyView {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        // 从 .rml 生成的代码
    }
}
```

### 9.3 元素系统

GPUI 的 `div` 元素是核心构建块，类似 HTML 的 `<div>` 但针对 Rust 的所有权模型和 GPU 渲染优化。RML 的每个 HTML 标签最终都映射为 GPUI 的元素组合。

## 十、开发者体验

### 10.1 从零开始的完整示例

**步骤 1：定义 UI（`views/counter.rml`）**

```html
<div class="app">
    <h1>计数: {count}</h1>
    <button onclick={increment}>+1</button>
    <button onclick={decrement} if={count > 0}>-1</button>
</div>
```

**步骤 2：实现逻辑（`views/counter.rml.rs`）**

```rust
use rml::prelude::*;

#[derive(Model)]
#[view]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count += 1;
        cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.count > 0 {
            self.count -= 1;
            cx.notify();
        }
    }
}
```

**步骤 3：启动应用（`main.rs`）**

```rust
use rml::app::RmlApplication;

mod views;

fn main() {
    RmlApplication::new()
        .run::<views::counter::Counter>()
        .unwrap();
}
```

**就这么简单。** 无需手写任何 GPUI 链式调用，UI 与逻辑完美分离。

### 10.2 代码量对比

| 任务         | 原生 GPUI | gpui-rsx | **RML**   |
| ---------- | ------- | -------- | --------- |
| 计数器 UI     | ~30 行   | ~15 行    | **~8 行**  |
| Todo 应用 UI | ~80 行   | ~40 行    | **~25 行** |
| UI/逻辑分离    | ❌       | ❌        | **✅**     |
| 设计师可编辑     | ❌       | ❌        | **✅**     |

## 十一、价值分析

### 11.1 开发效率提升

| 指标      | 原生 GPUI       | gpui-rsx      | **RML**        |
| ------- | ------------- | ------------- | -------------- |
| UI 代码量  | 100%          | ~50%          | **~35%**       |
| UI/逻辑分离 | 无             | 弱             | **强（文件级）**     |
| 设计师可参与  | 否             | 否             | **是**          |
| 热重载     | 否             | 否             | **是**          |
| 学习曲线    | 陡             | 中             | **平缓**         |
| IDE 支持  | Rust-analyzer | Rust-analyzer | **XML 工具链全支持** |

### 11.2 团队协作优势

- **UI/UX 设计师**：直接编辑 `.rml` 文件，使用熟悉的 HTML 语法
- **Rust 后端工程师**：专注 `.rml.rs` 中的业务逻辑和状态管理
- **全栈开发者**：可独立完成完整功能，分工更加清晰
- **代码审查**：UI 变更（`.rml`）与逻辑变更（`.rml.rs`）分离，审查更聚焦

### 11.3 长期价值

1. **设计系统沉淀**：`.rml` 文件成为可复用的设计资产
2. **工具链生态**：可利用现有的 XML/HTML 工具（格式化、Lint、预览）
3. **跨平台潜力**：未来可将 RML 编译到其他后端（如 Web、SwiftUI）
4. **低代码/无代码**：RML 的声明式特性为可视化设计器奠定基础
5. **招聘优势**：Web 开发者可快速上手 Rust 桌面开发

## 十二、挑战与应对策略

| 挑战             | 应对策略                                   |
| -------------- | -------------------------------------- |
| **GPUI 版本不稳定** | 锁定 GPUI 的 git rev；RML 编译器版本与 GPUI 版本绑定 |
| **编译时间增加**     | 使用 `build.rs` 增量编译；仅重新生成变更的 `.rml` 文件  |
| **错误信息可读性**    | RML 编译器输出带行号、列号的详细错误信息                 |
| **调试复杂性**      | 提供 `cargo rml-expand` 命令查看生成的 Rust 代码  |
| **IDE 支持**     | 开发 RML 的 VS Code 插件，提供语法高亮和自动补全        |
| **学习成本**       | 完善的文档、示例项目和迁移指南                        |

## 十三、实施路线图

### Phase 1：基础架构（6-8 周）

- [ ] 建立 Workspace 结构（5 个 crates）
- [ ] `core`：定义 `RmlView`、`BindingContext` 等基础 trait
- [ ] `rml`：实现基于 `quick-xml` 的 HTML 友好解析器
- [ ] `macros`：实现 `#[view]` 属性宏框架
- [ ] `app`：实现 `RmlApplication` 启动器
- [ ] 支持基础标签：`div`、`p`、`span`、`button`、`input`
- [ ] 支持基础属性：`class`、`id`、`style`、`value`
- [ ] 支持 `{ }` 单向数据绑定
- [ ] `build.rs` 集成

### Phase 2：核心功能（6-8 周）

- [ ] 事件绑定：`onclick`、`oninput`、`onkeydown` 等
- [ ] 双向绑定：`model` 指令
- [ ] 条件渲染：`if` / `else`
- [ ] 列表渲染：`each` / `key`
- [ ] 计算属性：`#[computed]`
- [ ] 命令系统：`#[command]`

### Phase 3：高级特性（6-8 周）

- [ ] 自定义组件系统（`#[component]`）
- [ ] 插槽（`slot`）
- [ ] 样式系统（`ResourceDictionary`）
- [ ] 值转换器（`Converter` trait）
- [ ] 生命周期回调（`#[on_loaded]`、`#[on_unloaded]`）
- [ ] 元素引用（`ref` / `#[element]`）

### Phase 4：生态与工具（持续）

- [ ] VS Code 插件：语法高亮、自动补全、错误提示
- [ ] 热重载：文件监听 + 增量编译
- [ ] 性能优化：增量渲染
- [ ] 文档站点 + API 文档
- [ ] 示例应用集合

## 十四、总结

RML 框架的核心价值在于 **“HTML 的语法亲和力 + WPF 的设计理念 + GPUI 的原生性能”** 三者完美融合：

| 维度     | RML 的优势                                 |
| ------ | --------------------------------------- |
| **语法** | 纯 HTML 标签 + 标准事件模型，Web 开发者零学习成本         |
| **架构** | WPF 级 UI/逻辑分离，Markup + Code-Behind 各司其职 |
| **性能** | 编译期生成原生 GPUI 代码，零运行时开销                  |
| **生态** | 5 个独立 crates 模块化设计，可单独复用或替换             |
| **扩展** | 自定义组件 + 极简指令系统，满足任意复杂 UI 需求             |
| **体验** | 热重载、IDE 支持、设计师可参与                       |

通过这套方案，Rust + GPUI 桌面开发将获得：

1. **工业化 UI 开发模式**：与 WPF、Vue、React 等成熟框架对齐开发体验
2. **团队协作效率质变**：设计师直接产出 `.rml`，工程师专注 `.rml.rs`
3. **极低的迁移成本**：前端/移动端开发者可快速转向 Rust 桌面开发
4. **长期可维护性**：UI 标记与业务逻辑清晰分离，代码库整洁有序

RML 不仅是一个技术方案，更是为 Rust 桌面开发生态建立了一套**标准化、工程化、设计师友好**的 UI 开发体系。🚀
