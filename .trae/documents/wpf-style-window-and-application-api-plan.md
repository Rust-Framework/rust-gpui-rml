# WPF 风格窗口与应用 API 规范化计划（修订版）

> **本计划取代之前的 `wpf-style-window-and-application-api-plan.md` 内容。**
>
> 修订起因：用户要求
> 1. `RmlApplication.main_window` 必须是 `IWindow` 类型的组件（必须是窗口，不是普通视图）
> 2. `RmlApplicationExt` 不应存在——`main_window` 是 `RmlApplication` 的内置功能
> 3. 定义抽象接口 `IComponent` 和 `IWindow`，参考 WPF/MAUI 设计理念
> 4. `IWindow` 自管理窗口通用操作（`open`/`show`/`close`/`state`），不通过扩展
> 5. 全面规范化：`#[view]` → `#[component]`，`IRmlView` → `IComponent`，新增 `#[window]`
> 6. 充分发挥 Rust + GPUI + gpui-component 的优秀特性

---

## 一、设计目标 Design Goals

### 核心 WPF 理念映射

| WPF 概念 | RML 对应 | 说明 |
|----------|---------|------|
| `FrameworkElement` | `IViewModel` | 基础元素契约（数据模型 + 生命周期） |
| `ContentControl` / `UserControl` | `IComponent` | 可复用组件（有模板 + 标签名，可在 `.rml` 中嵌套） |
| `Window` | `IWindow` | 窗口（有窗口配置 + 自管理 open/show/close/state） |
| `Application.MainWindow` | `RmlApplication.main_window::<W>()` | 内置主窗口设置 |
| `Application.StartupUri` | `main_window::<W>()` 声明式设置 | WPF 风格声明式 |
| `<Window x:Class="MyWindow">` | `#[window] struct MyWindow` | 代码+模板双轨 |

### 新的 Trait 层次（规范化后）

```
IModel (数据模型基础，纯数据，无 UI)
  └─ ILifecycle (生命周期回调：rml_on_loaded / rml_on_unloaded)
       └─ IViewModel: IModel + ILifecycle (ViewModel — 状态 + 命令)
            ├─ IComponent: IViewModel (组件 — rml_template() + rml_tag())
            │   └─ IWindow: IComponent (窗口 — 配置 + open/show/close/state)
            └─ (未来扩展点)
```

**关键变化**：
- `IRmlView` → **合并为 `IComponent`**（旧 `IComponent` 的 `rml_tag()` 合并入新 `IComponent`）
- `#[view]` → **`#[component]`**（旧 `#[component]` 的功能合并）
- 新增 **`IWindow: IComponent`**（窗口 IS 组件，有模板 + 窗口操作）
- 新增 **`#[window]`** 宏（类似 `#[component]` 但额外生成 `IWindow` 实现）

### 为什么 `IWindow: IComponent`？

在 WPF 中，`Window : ContentControl : Control : FrameworkElement`。窗口**本身就是组件**（可包含内容、有模板），只是额外拥有窗口生命周期操作。RML 沿用此设计：

- 窗口**有 `.rml` 模板**（定义窗口内容）——继承自 `IComponent`
- 窗口**有标签名**（理论上可在 `.rml` 中引用，虽不常用）——继承自 `IComponent`
- 窗口**有窗口操作**（open/show/close/state）——`IWindow` 新增

---

## 二、架构设计 Architecture

### 依赖方向（修订后）

```
┌──────────────────────────────────────────────────────────────┐
│ 用户应用 (demo)                                               │
│   use rml_ui::prelude::*;                                    │
│                                                              │
│   #[window]                                                  │
│   pub struct MainWindow { count: i32 }  // 自定义窗口        │
│                                                              │
│   fn main() {                                               │
│       RmlApplication::new()                                  │
│           .main_window::<MainWindow>()  // 内置 API          │
│           .run();                                            │
│   }                                                          │
└──────────────────────────────────────────────────────────────┘
        │依赖
        ▼
┌──────────────────────────────────────────────────┐
│ crates/app (应用启动器，不依赖 ui crate)          │
│                                                  │
│ • RmlApplication                                 │
│   - main_window::<W: IWindow>()  ← 内置方法     │
│   - run()                                        │
│ • IAppLifecycle                                  │
│                                                  │
│ 依赖：core (IWindow trait 定义在 core)            │
└──────────────────────────────────────────────────┘
        │依赖                          ▲依赖
        ▼                              │
┌──────────────────────────┐   依赖   ┌────────────────────────────┐
│ crates/core (核心层)     │ ◄────────│ crates/ui (UI 扩展层)      │
│                          │          │                            │
│ • IModel / ILifecycle     │          │ • 内置 Window / ModernWindow│
│ • IViewModel              │          │   (实现 IWindow)            │
│ • IComponent (合并)       │          │ • ModernWindow RML 组件    │
│ • IWindow (新增)          │          │ • MenuItem / StatusBarItem  │
│ • WindowChrome / State   │          │ • IWindowActions            │
│                          │          │ • init()                    │
│ 依赖：gpui               │          │ 依赖：core + app +          │
│                          │          │   gpui-component             │
└──────────────────────────┘          └────────────────────────────┘
```

**关键依赖变化**：
- `crates/app` **不再依赖** `crates/ui`（移除 `rust-rml-ui` 可选依赖 + `ui-components` feature）
- `crates/ui` **新增依赖** `crates/app`（用于内置 Window/ModernWindow 实现）
- `crates/core` 新增 `window` 模块（`IWindow` trait 定义）

### 双入口使用模式

**模式 A：声明式（WPF StartupUri 风格，推荐）**

```rust
use rml_app::RmlApplication;
use rml_ui::prelude::*;  // 启用 #[window] 宏 + 内置组件

#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub hovered: bool,
}

fn main() {
    RmlApplication::new()
        .main_window::<MainWindow>()  // 内置方法，无需 Ext trait
        .run();
}
```

框架自动：
1. 调用 `rml_ui::init(cx)` 初始化 gpui-component
2. 创建 `MainWindow::default()` 实例
3. 调用 `IWindow::open()` 打开窗口
4. 渲染 `MainWindow.rml` 模板

**模式 B：命令式（WPF OnStartup 重写风格）**

```rust
use rml_app::{IAppLifecycle, RmlApplication};
use rml_ui::prelude::*;

struct MyApp;

impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut gpui::App) {
        rml_ui::init(cx);
        let mut window = MainWindow::default();
        window.open(cx);  // IWindow::open
    }
}

fn main() {
    RmlApplication::new().run::<MyApp>();
}
```

---

## 三、核心 Trait 设计 Core Trait Design

### 3.1 `IComponent`（合并 `IRmlView` + 旧 `IComponent`）

**文件**：`crates/core/src/component.rs`（合并自 `view.rs` + `component.rs`）

```rust
//! IComponent trait —— RML 组件基础契约
//!
//! 所有可在 .rml 中使用的 UI 类型均实现此 trait。
//! #[component] 宏自动实现。

use crate::view_model::IViewModel;

/// RML 组件基础 trait。
///
/// 组件拥有：
/// - `.rml` 模板路径（定义视觉结构）
/// - 标签名（PascalCase，用于 .rml 中的 `<MyComponent>` 引用）
///
/// 由 `#[component]` 宏自动实现。
pub trait IComponent: IViewModel {
    /// 关联的 `.rml` 模板路径（相对于 `src` 目录）
    fn rml_template() -> &'static str;

    /// 组件标签名（PascalCase），用于 `.rml` 中的 `<MyComponent>`。
    /// 默认返回结构体名，由 `#[component]` 宏生成。
    fn rml_tag() -> &'static str;
}
```

### 3.2 `IWindow`（新增）

**文件**：`crates/core/src/window.rs`（新建）

```rust
//! IWindow trait —— 窗口抽象接口
//!
//! 参考 WPF Window 类设计：
//! - 窗口 IS 组件（继承 IComponent，有模板和标签）
//! - 窗口有配置属性（title / width / height / chrome）
//! - 窗口自管理生命周期操作（open / show / close / state）
//!
//! 由 `#[window]` 宏自动实现，也可手动 impl。

use gpui::{App, Pixels, Point, Size, TitlebarOptions, WindowBounds, WindowHandle, WindowOptions, px};
use crate::component::IComponent;

/// 窗口标题栏样式（WPF: Window.WindowStyle）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowChrome {
    /// 系统原生标题栏
    Native,
    /// 透明标题栏（现代风格，由 TitleBar 组件自绘）
    #[default]
    Transparent,
}

/// 窗口状态（WPF: WindowState）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowState {
    #[default]
    Normal,
    Minimized,
    Maximized,
}

/// 窗口抽象接口（WPF Window 类等价物）
///
/// 窗口是一种特殊组件，可作为顶层 OS 窗口打开。
/// 窗口自管理其窗口句柄（WindowHandle），无需扩展 trait。
///
/// 通过 `#[window]` 宏自动实现，或手动 impl。
///
/// # 示例
///
/// ```rust,ignore
/// #[window]
/// #[derive(Default)]
/// pub struct MainWindow {
///     count: i32,
/// }
///
/// fn main() {
///     RmlApplication::new()
///         .main_window::<MainWindow>()
///         .run();
/// }
/// ```
pub trait IWindow: IComponent {
    // ── 配置属性（WPF: Window.Title / Width / Height / WindowStyle）──

    /// 窗口标题
    fn title(&self) -> &str;

    /// 窗口宽度
    fn width(&self) -> Pixels;

    /// 窗口高度
    fn height(&self) -> Pixels;

    /// 标题栏样式（默认透明/现代风格）
    fn chrome(&self) -> WindowChrome {
        WindowChrome::Transparent
    }

    // ── 窗口选项构建（默认实现）──

    /// 从配置构建 GPUI WindowOptions
    fn window_options(&self) -> WindowOptions {
        let titlebar = match self.chrome() {
            WindowChrome::Native => TitlebarOptions {
                title: Some(self.title().into()),
                appears_transparent: false,
                traffic_light_position: None,
            },
            WindowChrome::Transparent => TitlebarOptions {
                title: Some(self.title().into()),
                appears_transparent: true,
                traffic_light_position: Some(Point::new(px(9.), px(9.))),
            },
        };

        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                origin: Default::default(),
                size: Size {
                    width: self.width(),
                    height: self.height(),
                },
            })),
            titlebar: Some(titlebar),
            ..Default::default()
        }
    }

    // ── 生命周期操作（WPF: Window.Show / Close / Activate）──

    /// 打开窗口（WPF: Window.Show()）
    ///
    /// 创建 OS 窗口并显示。窗口句柄存储在实例内部。
    fn open(&mut self, cx: &mut App);

    /// 关闭窗口（WPF: Window.Close()）
    fn close(&mut self, cx: &mut App);

    /// 显示窗口（若已隐藏）
    fn show(&mut self, cx: &mut App);

    /// 隐藏窗口（WPF: Window.Hide()）
    fn hide(&mut self, cx: &mut App);

    /// 激活窗口（置于前台）
    fn activate(&mut self, cx: &mut App);

    // ── 状态查询（WPF: Window.WindowState）──

    /// 获取窗口状态
    fn state(&self, cx: &App) -> WindowState;

    // ── 句柄访问（供高级用途）──

    /// 获取窗口句柄（未打开时返回 None）
    fn handle(&self) -> Option<WindowHandle<()>>;

    /// 设置窗口句柄（由 `open()` 内部调用）
    fn set_handle(&mut self, handle: WindowHandle<()>);
}
```

### 3.3 `RmlApplication`（重构）

**文件**：`crates/app/src/application.rs`

```rust
//! RmlApplication —— 应用启动器
//!
//! 参考 WPF Application 类设计：
//! - `main_window::<W: IWindow>()` 设置主窗口类型（内置功能，非扩展）
//! - `run()` 启动应用并打开主窗口

use std::marker::PhantomData;
use gpui::App;
use rml_core::window::IWindow;

use crate::lifecycle::IAppLifecycle;

/// RML 应用启动器
///
/// 内置主窗口设置，无需扩展 trait。
/// 类比 WPF Application + StartupUri。
pub struct RmlApplication<W: IWindow = ()> {
    _window: PhantomData<W>,
}

impl RmlApplication<()> {
    pub fn new() -> Self {
        Self { _window: PhantomData }
    }

    /// 命令式启动（WPF OnStartup 重写风格）
    ///
    /// `A: IAppLifecycle` 负责窗口创建。
    pub fn run<A>(self)
    where
        A: IAppLifecycle + Default + 'static,
    {
        gpui_platform::application().run(move |cx: &mut App| {
            let mut app = A::default();
            app.on_launch(cx);
        });
    }
}

impl<W: IWindow + Default + 'static> RmlApplication<W> {
    /// 声明式设置主窗口类型（WPF StartupUri 风格）
    ///
    /// ```rust,ignore
    /// RmlApplication::new()
    ///     .main_window::<MainWindow>()
    ///     .run();
    /// ```
    pub fn main_window<NewW: IWindow + Default + 'static>(self) -> RmlApplication<NewW> {
        RmlApplication { _window: PhantomData }
    }

    /// 启动应用并打开主窗口
    ///
    /// 框架自动：
    /// 1. 调用 `rml_ui::init(cx)` 初始化 gpui-component（若可用）
    /// 2. 创建 `W::default()` 实例
    /// 3. 调用 `IWindow::open()` 打开主窗口
    pub fn run(self) {
        gpui_platform::application().run(move |cx: &mut App| {
            // 初始化由窗口类型自身的 open() 实现负责
            // （IWindow::open 实现可选择是否调用 rml_ui::init）
            let mut window = W::default();
            window.open(cx);
        });
    }
}

impl Default for RmlApplication<()> {
    fn default() -> Self { Self::new() }
}
```

**关键设计点**：
- `RmlApplication` 使用泛型 `W: IWindow` 追踪主窗口类型
- `main_window::<W>()` 是**内置方法**，返回 `RmlApplication<W>`（类型状态模式）
- `run()` 行为取决于 `W`：
  - `W = ()`：需要 `IAppLifecycle` 命令式启动
  - `W = SomeWindow`：自动调用 `W::default().open(cx)` 声明式启动
- **无需 `RmlApplicationExt` 扩展 trait**
- **无需 `MainWindowBuilder<V>`**（配置由 `IWindow` trait 方法提供）

---

## 四、宏设计 Macro Design

### 4.1 `#[component]`（合并旧 `#[view]` + `#[component]`）

**文件**：`crates/macros/src/component.rs`（由 `view.rs` 重命名）

生成的 impl 链：
```rust
impl IModel for #struct_name { ... }
impl ILifecycle for #struct_name { ... }
impl IViewModel for #struct_name {}
impl IComponent for #struct_name {
    fn rml_template() -> &'static str { #template_path }
    fn rml_tag() -> &'static str { #struct_name_str }
}
// + include!("OUT_DIR/rml_generated/<snake>.rs")
```

**变化**：
- 旧 `#[view]` → `#[component]`
- 旧 `#[component]`（额外生成 `IComponent`）→ 合并入新 `#[component]`
- 所有 `#[component]` 结构体自动获得 `rml_tag()`（从结构体名派生）

### 4.2 `#[window]`（新增）

**文件**：`crates/macros/src/window.rs`（新建）

```rust
//! #[window] 宏 —— 自动实现 IWindow trait
//!
//! 在 #[component] 基础上额外生成：
//! 1. 窗口句柄字段（__rml_window_handle: Option<WindowHandle<()>>）
//! 2. IWindow trait 实现
//! 3. 默认的 open/close/show/state 实现
```

宏展开效果：
```rust
// 输入
#[window(title = "My App", width = 800, height = 600)]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

// 展开（概念性）
pub struct MainWindow {
    pub count: i32,
    __rml_window_handle: Option<WindowHandle<()>>,  // 自动添加
}

impl IModel for MainWindow { ... }
impl ILifecycle for MainWindow { ... }
impl IViewModel for MainWindow {}
impl IComponent for MainWindow {
    fn rml_template() -> &'static str { "main_window.rml" }
    fn rml_tag() -> &'static str { "MainWindow" }
}
impl IWindow for MainWindow {
    fn title(&self) -> &str { "My App" }
    fn width(&self) -> Pixels { px(800.) }
    fn height(&self) -> Pixels { px(600.) }
    fn chrome(&self) -> WindowChrome { WindowChrome::Transparent }

    fn open(&mut self, cx: &mut App) {
        let options = self.window_options();
        let handle = cx.open_window(options, |window, cx| {
            let view = cx.new(|_cx| Self::default());
            // 用 rml_ui::Root 包裹（若可用）
            cx.new(|cx| rml_ui::Root::new(view, window, cx))
        }).expect("failed to open window");
        self.set_handle(handle);
    }

    fn close(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            // GPUI 窗口关闭逻辑
        }
    }

    // show/hide/activate/state 的默认实现...
    fn handle(&self) -> Option<WindowHandle<()>> { self.__rml_window_handle }
    fn set_handle(&mut self, handle: WindowHandle<()>) { self.__rml_window_handle = Some(handle); }
}
// + include!("OUT_DIR/rml_generated/main_window.rs")
```

**窗口配置方式**（两种，参考 WPF）：
1. **属性参数**：`#[window(title = "...", width = 800, height = 600)]`
2. **方法重写**：手动 `impl IWindow` 重写 `title()` / `width()` / `height()`

### 4.3 宏注册

**文件**：`crates/macros/src/lib.rs`

```rust
// 变化：
// - pub fn view(...) → pub fn component(...)  （重命名）
// - pub fn component(...) → 移除（合并）
// - 新增 pub fn window(...)

#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args, input)
}

#[proc_macro_attribute]
pub fn window(args: TokenStream, input: TokenStream) -> TokenStream {
    window::expand(args, input)
}
```

---

## 五、当前状态分析 Current State

### 命名冲突分析（关键）

当前代码库中存在 **`IComponent` 命名冲突**：

| 当前名称 | 当前位置 | 用途 | 处理 |
|----------|---------|------|------|
| `IRmlView` trait | `crates/core/src/view.rs:14` | 视图标记（有 `rml_template()`） | **重命名为 `IComponent`** |
| `IComponent` trait | `crates/core/src/component.rs:15` | 组件标记（有 `rml_tag()`，继承 `IRmlView`） | **合并入新 `IComponent`** |
| `#[view]` 宏 | `crates/macros/src/lib.rs:37` | 视图宏 | **重命名为 `#[component]`** |
| `#[component]` 宏 | `crates/macros/src/lib.rs:43` | 组件宏（`#[view]` + `IComponent`） | **合并入新 `#[component]`** |

**合并策略**：
- 新 `IComponent` = 旧 `IRmlView` + 旧 `IComponent`（同时有 `rml_template()` 和 `rml_tag()`）
- 新 `#[component]` = 旧 `#[view]` + 旧 `#[component]`（生成所有 impl）
- 旧 `#[component]` 在代码库中 **零实际使用**（仅定义和文档，无结构体标注），合并无破坏

### 已有文件清单

| 文件 | 状态 | 变更类型 |
|------|------|---------|
| `crates/core/src/view.rs` | `IRmlView` 定义 | **合并到 `component.rs` 后删除** |
| `crates/core/src/component.rs` | 旧 `IComponent` 定义 | **重写为合并后的 `IComponent`** |
| `crates/core/src/view_model.rs` | `IViewModel` 定义 | 不变 |
| `crates/core/src/lifecycle.rs` | `ILifecycle` 定义 | 不变 |
| `crates/core/src/model.rs` | `IModel` 定义 | 不变 |
| `crates/core/src/lib.rs` | 模块声明 | **更新模块声明** |
| `crates/core/src/prelude.rs` | 重导出 | **更新重导出** |
| `crates/macros/src/view.rs` | `#[view]`/`#[component]` 实现 | **重命名为 `component.rs`** + 更新 |
| `crates/macros/src/lib.rs` | 宏注册 | **更新注册** |
| `crates/app/src/application.rs` | `RmlApplication` | **重构** |
| `crates/app/src/window.rs` | `Window`/`ModernWindow` 配置对象 | **移到 `crates/ui`** |
| `crates/app/src/lib.rs` | 模块声明 | **更新** |
| `crates/app/Cargo.toml` | 依赖 | **移除 ui 依赖** |
| `crates/ui/src/window/mod.rs` | 窗口组件模块 | **扩展** |
| `crates/ui/src/window/modern_window.rs` | RML ModernWindow 组件 | 不变 |
| `crates/ui/src/window/actions.rs` | `IWindowActions` trait | 不变 |
| `crates/ui/src/lib.rs` | 重导出 | **更新** |
| `crates/ui/src/prelude.rs` | prelude | **更新** |
| `crates/ui/Cargo.toml` | 依赖 | **添加 app 依赖** |
| `crates/engine/**` | 编译器 | **零影响**（不引用任何 trait） |
| `demo/src/counter.rml.rs` | `#[view]` 使用 | **改为 `#[window]`** |
| `demo/src/todos.rml.rs` | `#[view]` 使用 | **改为 `#[window]`** |
| `demo/src/main.rs` | 入口 | **迁移到新 API** |

### 影响范围统计

| 范围 | IRmlView 引用 | IComponent 引用 | #[view] 使用 | #[component] 使用 |
|------|-------------|---------------|------------|-----------------|
| 源代码（load-bearing） | ~8 处 | ~5 处 | 2 处（demo） | 0 处 |
| 文档（docs/**） | ~5 处 | ~5 处 | ~100 处 | ~15 处 |
| 计划文档（.trae/documents/**） | ~30 处 | ~10 处 | — | ~15 处 |

**引擎 crate 零影响**：`crates/engine/**` 完全不引用 `IRmlView`/`IComponent`/`IViewModel`，编译器生成 `impl Render` 而非 trait impl。`tags.rs` 中的 "component" 词汇指 gpui-component 组件，与本重命名无关。

---

## 六、实施步骤 Implementation Steps

### Step 1：Core trait 合并 —— `IRmlView` + `IComponent` → 新 `IComponent`

**文件变更**：

1. **`crates/core/src/component.rs`**（重写，合并 `view.rs` + `component.rs`）：
```rust
//! IComponent trait —— RML 组件基础契约
use crate::view_model::IViewModel;

/// RML 组件基础 trait（合并旧 IRmlView + IComponent）
pub trait IComponent: IViewModel {
    fn rml_template() -> &'static str;
    fn rml_tag() -> &'static str;
}
```

2. **`crates/core/src/window.rs`**（新建）：
   - `WindowChrome` 枚举
   - `WindowState` 枚举
   - `IWindow: IComponent` trait（如上 §3.2 设计）

3. **`crates/core/src/view.rs`** → **删除**（内容合并到 `component.rs`）

4. **`crates/core/src/lib.rs`**：
```rust
pub mod component;  // 已有，内容更新
pub mod window;     // 新增
// pub mod view;    // 删除
```

5. **`crates/core/src/prelude.rs`**：
```rust
pub use crate::component::IComponent;       // 更新
pub use crate::window::{IWindow, WindowChrome, WindowState};  // 新增
// pub use crate::view::IRmlView;  // 删除
```

**验证**：`cargo build -p rust-rml-core` 通过。

### Step 2：宏更新 —— `#[view]` → `#[component]` + 新增 `#[window]`

**文件变更**：

1. **`crates/macros/src/component.rs`**（由 `view.rs` 重命名）：
   - `expand()` 函数生成 `impl IComponent`（而非 `impl IRmlView`）
   - 所有 `#[component]` 自动包含 `rml_tag()` 生成
   - 更新 `rml_core::view::IRmlView` → `rml_core::component::IComponent` 路径
   - 移除 `is_component` 分支（所有 component 都有 `rml_tag()`）

2. **`crates/macros/src/window.rs`**（新建）：
   - `expand()` 函数在 `#[component]` 基础上额外生成 `IWindow` 实现
   - 自动添加 `__rml_window_handle` 字段
   - 解析 `#[window(title = "...", width = N, height = N)]` 属性参数
   - 生成 `open()`/`close()`/`show()`/`hide()`/`state()`/`handle()`/`set_handle()` 实现
   - `open()` 实现中调用 `rml_ui::Root::new()` 包裹视图（引用 `rml_ui` crate）

3. **`crates/macros/src/lib.rs`**：
```rust
// 重命名：pub fn view → pub fn component
// 移除：pub fn component（旧的，合并）
// 新增：pub fn window

mod component;  // 重命名自 view
mod window;     // 新增

#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args, input)
}

#[proc_macro_attribute]
pub fn window(args: TokenStream, input: TokenStream) -> TokenStream {
    window::expand(args, input)
}
```

**验证**：`cargo build -p rust-rml-macros` 通过。

### Step 3：App crate 重构 —— 内置 `main_window` + 移除 UI 依赖

**文件变更**：

1. **`crates/app/src/application.rs`**（重写）：
   - `RmlApplication<W: IWindow = ()>` 泛型化
   - 内置 `main_window::<W: IWindow>()` 方法
   - `run()` 两种重载：命令式（`W = ()`）+ 声明式（`W = SomeWindow`）
   - 移除 `rml_ui::init(cx)` 调用（由 `IWindow::open()` 实现负责）

2. **`crates/app/src/window.rs`** → **清空或删除**：
   - `Window`/`ModernWindow` 配置对象移到 `crates/ui`
   - 移除 `extern crate rust_rml_ui`
   - 移除 `ui-components` feature 相关代码

3. **`crates/app/src/lib.rs`**：
```rust
extern crate rust_rml_core as rml_core;
// 移除：#[cfg(feature = "ui-components")] extern crate rust_rml_ui as rml_ui;

pub mod application;
pub mod lifecycle;
pub mod resources;
// 移除：pub mod window;

pub use application::RmlApplication;
pub use lifecycle::IAppLifecycle;
// 移除：pub use window::{Window, ModernWindow};
```

4. **`crates/app/Cargo.toml`**：
```toml
[dependencies]
rust-rml-core = { workspace = true }
gpui = { workspace = true }
gpui_platform = { workspace = true }
# 移除：rust-rml-ui = { workspace = true, optional = true }
# 移除：[features] default = ["ui-components"] / ui-components = [...]
```

**验证**：`cargo build -p rust-rml-app` 通过（不依赖 ui crate）。

### Step 4：UI crate 扩展 —— 内置 Window/ModernWindow 实现 IWindow

**文件变更**：

1. **`crates/ui/Cargo.toml`**：
```toml
[dependencies]
rust-rml-core = { workspace = true }
rust-rml-app = { workspace = true }  # 新增
gpui = { workspace = true }
gpui-component = { workspace = true }
smallvec = "1"
```

2. **`crates/ui/src/window/builtin_window.rs`**（新建，由 `crates/app/src/window.rs` 迁移）：
   - `Window` 结构体实现 `IWindow`（原生标题栏窗口）
   - `ModernWindow` 结构体实现 `IWindow`（透明标题栏窗口）
   - 保留配置字段（title/width/height/chrome）+ 窗口句柄
   - `open()` 实现使用 `rml_ui::Root` 包裹
   - 这些是"开箱即用"的窗口基类（类似 WPF `Window` 基类）

3. **`crates/ui/src/window/mod.rs`**：
```rust
pub mod actions;
pub mod builtin_window;    // 新增（由 app crate 迁移）
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use builtin_window::{Window, ModernWindow};  // 新增
pub use modern_window::ModernWindow as ModernWindowShell;  // 重命名避免冲突
pub use types::{MenuItem, StatusBarItem};
```

> **命名澄清**：`rml_ui::ModernWindow`（RML 视觉外壳组件，`RenderOnce`）重命名为 `ModernWindowShell`，避免与 `rml_ui::window::ModernWindow`（IWindow 实现，配置对象）冲突。

4. **`crates/ui/src/lib.rs`** + **`crates/ui/src/prelude.rs`**：
   - 更新重导出
   - 添加 `Window`/`ModernWindow`（IWindow 实现）

**验证**：`cargo build -p rust-rml-ui` 通过。

### Step 5：Demo 迁移

**文件变更**：

1. **`demo/src/counter.rml.rs`** → **改为 `#[window]`**：
```rust
use rml::prelude::*;
use rml_ui::prelude::*;

#[window(title = "RML Counter Demo", width = 400, height = 500)]
#[derive(Default)]
pub struct Counter {
    pub count: i32,
    pub hovered: bool,
}

impl Counter {
    #[computed]
    pub fn double_count(&self) -> i32 { self.count * 2 }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
    // ...
}
```

2. **`demo/src/todos.rml.rs`** → **改为 `#[window]`**（类似）

3. **`demo/src/main.rs`**：
```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

use rml_app::RmlApplication;
use rml_ui::prelude::*;  // 启用 #[window] 宏

#[path = "counter.rml.rs"]
mod counter;

fn main() {
    RmlApplication::new()
        .main_window::<counter::Counter>()  // 内置方法
        .run();
}
```

**验证**：`cargo build -p rust-rml-demo` + `cargo run -p rust-rml-demo` 通过。

### Step 6：全量验证

```bash
# 编译验证
cargo build -p rust-rml-core      # trait 合并
cargo build -p rust-rml-macros     # 宏更新
cargo build -p rust-rml-app        # app 重构（不依赖 ui）
cargo build -p rust-rml-ui         # ui 扩展（内置 Window IWindow）
cargo build -p rust-rml-engine     # 引擎（零影响，应直接通过）
cargo build -p rust-rml-demo       # demo 迁移

# 全 workspace
cargo build --workspace

# 测试
cargo test -p rust-rml-engine     # 180+ 测试
cargo test -p rust-rml-core       # 24+ 测试

# 运行
cargo run -p rust-rml-demo
```

---

## 七、假设与决策 Assumptions & Decisions

### 假设
1. `#[view]` 宏生成的视图（如 `Counter`）自动实现 `IComponent`，满足 `IWindow: IComponent` 约束
2. GPUI 的 `WindowHandle` 提供 `close`/`minimize`/`maximize` 等窗口操作 API
3. `gpui-component` 的 `Root` / `TitleBar` / `StatusBar` API 稳定
4. `WindowHandle<()>` 可用于存储通用窗口句柄（或使用 `AnyWindowHandle`）

### 决策
1. **`IWindow: IComponent`**：窗口 IS 组件（WPF `Window : ContentControl` 等价），有模板 + 窗口操作
2. **`IRmlView` → `IComponent` 合并**：旧 `IComponent` 的 `rml_tag()` 合并入新 `IComponent`，所有组件都有标签名
3. **`#[view]` → `#[component]` 合并**：旧 `#[component]` 功能合入新 `#[component]`，所有组件都生成 `IComponent` impl
4. **`main_window` 是 `RmlApplication` 内置方法**：使用泛型类型状态模式 `RmlApplication<W: IWindow>`，无需扩展 trait
5. **`IWindow::open()` 是必需方法**：由 `#[window]` 宏生成实现（使用 `rml_ui::Root` 包裹），不提供默认实现（因 core crate 不依赖 ui crate）
6. **内置 `Window`/`ModernWindow` 移到 `crates/ui`**：实现 `IWindow`，作为开箱即用的窗口基类
7. **`rml_ui::ModernWindow`（RML 外壳组件）重命名为 `ModernWindowShell`**：避免与 `rml_ui::window::ModernWindow`（IWindow 实现）命名冲突
8. **`app` crate 不依赖 `ui` crate**：`RmlApplication.main_window::<W: IWindow>()` 仅依赖 core 的 `IWindow` trait 约束，窗口打开逻辑由 `W` 的 `IWindow::open()` 实现负责
9. **文档更新范围**：源代码内文档（crate README、doc comments）必须更新；`docs/**` 用户文档批量更新；`.trae/documents/**` 计划文档作为历史归档，仅更新本文件
10. **引擎 crate 零影响**：`crates/engine/**` 不引用任何 trait，重命名不影响编译器

### 待实现时确定的细节
1. `WindowHandle<()>` 是否可行，或需用 `AnyWindowHandle` 存储通用句柄
2. GPUI 窗口的 `close`/`show`/`hide`/`state` 具体 API 调用方式
3. `#[window]` 宏如何处理 `rml_ui::Root` 的可选依赖（若用户不依赖 `rml_ui`）
4. 内置 `Window`/`ModernWindow` 的默认 `.rml` 模板内容

---

## 八、文件变更清单 File Change List

### Core crate（crates/core）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/core/src/component.rs` | **重写** | 合并 `IRmlView` + `IComponent` → 新 `IComponent` |
| `crates/core/src/window.rs` | **新建** | `IWindow` trait + `WindowChrome` + `WindowState` |
| `crates/core/src/view.rs` | **删除** | 内容合并到 `component.rs` |
| `crates/core/src/lib.rs` | 修改 | 移除 `mod view`，新增 `mod window` |
| `crates/core/src/prelude.rs` | 修改 | 更新重导出 |
| `crates/core/README.md` | 修改 | 更新 trait 清单 |

### Macros crate（crates/macros）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/macros/src/component.rs` | **重命名+重写** | 由 `view.rs` 重命名，生成 `IComponent` impl |
| `crates/macros/src/window.rs` | **新建** | `#[window]` 宏实现 |
| `crates/macros/src/lib.rs` | 修改 | 重命名 `view` → `component`，新增 `window` |
| `crates/macros/README.md` | 修改 | 更新宏清单 |

### App crate（crates/app）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/app/src/application.rs` | **重写** | `RmlApplication<W: IWindow>` 泛型化 + 内置 `main_window` |
| `crates/app/src/window.rs` | **清空/删除** | `Window`/`ModernWindow` 移到 ui crate |
| `crates/app/src/lib.rs` | 修改 | 移除 ui 依赖 + window 模块 |
| `crates/app/Cargo.toml` | 修改 | 移除 `rust-rml-ui` 依赖 + `ui-components` feature |
| `crates/app/README.md` | 修改 | 更新 API 说明 |

### UI crate（crates/ui）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/ui/src/window/builtin_window.rs` | **新建** | 内置 `Window`/`ModernWindow` 实现 `IWindow` |
| `crates/ui/src/window/mod.rs` | 修改 | 新增模块 + 重命名 `ModernWindow` → `ModernWindowShell` |
| `crates/ui/src/window/modern_window.rs` | 修改 | 重命名 `ModernWindow` → `ModernWindowShell` |
| `crates/ui/src/lib.rs` | 修改 | 更新重导出 |
| `crates/ui/src/prelude.rs` | 修改 | 更新 prelude |
| `crates/ui/Cargo.toml` | 修改 | 添加 `rust-rml-app` 依赖 |

### Demo

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `demo/src/counter.rml.rs` | 修改 | `#[view]` → `#[window]` |
| `demo/src/todos.rml.rs` | 修改 | `#[view]` → `#[window]` |
| `demo/src/main.rs` | 修改 | 使用 `main_window::<W>().run()` 新 API |

### 零影响

| 范围 | 说明 |
|------|------|
| `crates/engine/**` | 编译器不引用任何 trait，零影响 |
| `crates/macros/src/command.rs` | 不涉及 |
| `crates/macros/src/computed.rs` | 不涉及 |
| `crates/macros/src/derive_model.rs` | 不涉及 |
| `crates/macros/src/lifecycle.rs` | 不涉及 |

### 文档更新（按优先级）

| 优先级 | 范围 | 说明 |
|--------|------|------|
| P0 | `crates/*/README.md` | 各 crate README 中的 trait/宏清单 |
| P0 | `crates/core/src/prelude.rs` doc | prelude 文档 |
| P1 | `docs/04-code-behind/macros.md` | 宏参考文档（`#[view]` → `#[component]`，新增 `#[window]`） |
| P1 | `docs/06-components/custom-components.md` | 自定义组件文档 |
| P1 | `docs/04-code-behind/viewmodel-structure.md` | ViewModel 结构文档 |
| P2 | `docs/**` 其余 | 批量替换 `#[view]` → `#[component]` |
| P2 | `README.md`（根） | 更新快速开始 + 宏表 |
| P3 | `.trae/documents/**` | 历史计划文档，作为归档不更新（本文件除外） |

---

## 九、执行顺序 Execution Order

1. **Step 1**：Core trait 合并 → `cargo build -p rust-rml-core` 通过
2. **Step 2**：宏更新 → `cargo build -p rust-rml-macros` 通过
3. **Step 3**：App crate 重构 → `cargo build -p rust-rml-app` 通过
4. **Step 4**：UI crate 扩展 → `cargo build -p rust-rml-ui` 通过
5. **Step 5**：Demo 迁移 → `cargo build -p rust-rml-demo` 通过
6. **Step 6**：全量验证 → `cargo build --workspace` + `cargo test` + `cargo run`

每个步骤完成后立即验证编译，避免错误累积。

---

## 十、与原计划的关系 Relationship to Original Plans

### 本计划的变化（相比之前版本）

| 维度 | 之前版本 | 本版本 |
|------|---------|--------|
| 主窗口 API | `RmlApplicationExt` 扩展 trait（ui crate） | `RmlApplication` 内置 `main_window::<W>()` |
| 主窗口类型约束 | `V: IRmlView + Render + Default` | `W: IWindow + Default`（必须是窗口） |
| 窗口操作 | `WindowExt` trait 扩展 | `IWindow` trait 内置（open/show/close/state） |
| `IRmlView` | 保留 | **合并为 `IComponent`** |
| `#[view]` | 保留 | **重命名为 `#[component]`** |
| `IComponent` | 保留（继承 `IRmlView`） | **合并**（同时有 `rml_template` + `rml_tag`） |
| `#[component]` | 保留（额外生成 `IComponent`） | **合并**（所有 component 都生成 `IComponent`） |
| `IWindow` | 不存在 | **新增**（`IWindow: IComponent`） |
| `#[window]` | 不存在 | **新增** |
| 窗口配置 | `MainWindowBuilder<V>` | `IWindow` trait 方法（title/width/height/chrome） |

### 保持不变

- **架构解耦方向**：`app` crate 不依赖 `ui` crate
- **引擎零影响**：`crates/engine/**` 不受影响
- **Phase 2+ 计划**：`<bind>` codegen、`<Kbd>` 组件、Action 注册等后续阶段保持不变
