# WPF 风格 Window / ModernWindow / RmlApplication 设计整合计划

> 本计划是对 `window-api-revision-and-phase1-completion-plan.md` 的修订与扩展。
> 修订起因：用户要求"完全按照 WPF 风格"设计 Window/ModernWindow/RmlApplication，并澄清架构依赖方向。
> 本计划聚焦于：架构解耦 + 声明式 API + 完成 Phase 1 剩余工作。

---

## 一、设计澄清 Design Clarifications

### 用户的多轮反馈（已整合）

1. **Window/ModernWindow 关系**：`.rml` 中可手动组装 TitleBar/StatusBar/Menu 达到 ModernWindow 效果，ModernWindow 是易用性封装（代码更少、更现代）
2. **窗口打开 API**：struct-based `Window::new(...).open::<V>(cx)`，不要自由函数 helper
3. **RmlApplication 主窗口类型**：希望像 WPF `StartupUri` 那样，Application 直接设置主窗口类型，不必每次写 `IAppLifecycle` 实现
4. **完全 WPF 风格双轨**：既支持 XAML 设计（`.rml` 中 `<ModernWindow>` 根标签），也支持代码设计（Rust 中 `Window::new()`）
5. **架构依赖澄清**：`gpui-component` 应被 `ui` crate 引用，不被 `app` crate 直接引用；需要 ModernWindow 开发时由用户引入 `ui` crate

### 本轮决策（用户已确认）

| 决策点 | 选择 |
|--------|------|
| Application 主窗口 API 风格 | **双支持**：声明式 `main_window::<V>()` + 命令式 `IAppLifecycle::on_launch` |
| 窗口配置来源 | **两者都支持**：链式 API（Phase 1）+ `.rml` 根元素属性（Phase 2） |
| 依赖方向 | `app` crate 不依赖 `rml_ui`；`ui` crate 依赖 `app` crate + `gpui-component` |

---

## 二、架构设计 Architecture

### 双层概念关系

```
┌─────────────────────────────────────────────────────────────┐
│ 用户应用 (demo)                                              │
│   use rml_ui::prelude::*;  // 启用 extension trait 方法       │
│   RmlApplication::new()                                     │
│       .main_window::<MyView>()  // 声明式（ui crate 提供）   │
│       .title("...").size(800, 600)                          │
│       .run();                                               │
└─────────────────────────────────────────────────────────────┘
        │依赖                          │依赖
        ▼                              ▼
┌──────────────────┐   依赖   ┌──────────────────────────────┐
│ crates/app       │ ◄────────│ crates/ui                    │
│ (无 UI 依赖)      │          │ (gpui-component 集成层)        │
│                  │          │                              │
│ • RmlApplication │          │ • RmlApplicationExt trait     │
│ • IAppLifecycle  │          │   - main_window::<V>()        │
│ • Window         │          │   - run_with_ui::<A>()        │
│ • ModernWindow   │          │ • WindowExt trait             │
│   (窗口配置对象)  │          │   - open::<V>(cx) + Root 包裹 │
│ • IRmlView bound │          │ • MainWindowBuilder<V>        │
└──────────────────┘          │ • ModernWindow RML 组件       │
        │                     │ • MenuItem / StatusBarItem    │
        │依赖                 │ • IWindowActions trait        │
        ▼                     └──────────────────────────────┘
┌──────────────────┐                   │依赖
│ crates/core      │ ◄─────────────────┘
│ • IRmlView trait  │
│ • ClickEvent 等   │
└──────────────────┘
```

### 层 1：窗口对象（crates/app，无 UI 依赖）

| 类型 | 职责 | API |
|------|------|-----|
| `RmlApplication` | 应用启动器（基础，不调用 `rml_ui::init`） | `RmlApplication::new().run::<A: IAppLifecycle>()` |
| `IAppLifecycle` | 应用生命周期 trait（命令式入口） | `on_launch` / `on_exit` / `on_activate` / `on_deactivate` |
| `Window` | 原生标题栏窗口配置对象 | `Window::new(title, w, h).into_modern()` |
| `ModernWindow` | 透明标题栏窗口配置对象 | `ModernWindow::new(title, w, h)` |

**关键变化**：
- `Window` / `ModernWindow` **不再有 `open()` 方法**（移到 `ui` crate 的 `WindowExt` trait）
- `Window` 暴露 `pub fn build_options(&self) -> WindowOptions`（供 `ui` crate 调用）
- `RmlApplication::run::<A>()` **不调用 `rml_ui::init(cx)`**（由 `ui` crate 的 `run_with_ui` 处理）
- `app` crate 的 `Cargo.toml` **移除 `rust-rml-ui` 依赖**

### 层 2：UI 扩展（crates/ui，依赖 app + gpui-component）

| 类型 | 职责 | API |
|------|------|-----|
| `RmlApplicationExt` | 为 `RmlApplication` 添加声明式 API | `.main_window::<V>()` → `MainWindowBuilder<V>` |
| `MainWindowBuilder<V>` | 声明式主窗口构建器 | `.title()` / `.size()` / `.modern()` / `.run()` |
| `WindowExt` | 为 `Window`/`ModernWindow` 添加 `open()` 方法 | `.open::<V>(cx)`（用 Root 包裹） |
| `ModernWindow`（RML 组件） | 内置封装 TitleBar+Menu+StatusBar | `<ModernWindow title="..." menu={...}>` |
| `MenuItem` / `StatusBarItem` | MVVM 绑定数据类型 | `MenuItem::new("文件").submenu(...)` |
| `IWindowActions` | `&mut Window` 通知助手 trait | `window.notify_info("...", cx)` |

### 双入口使用模式

**模式 A：声明式（WPF StartupUri 风格，推荐）**
```rust
use rml_ui::prelude::*;  // 启用 RmlApplicationExt + MainWindowBuilder

fn main() {
    RmlApplication::new()
        .main_window::<counter::Counter>()
        .title("RML Counter Demo")
        .size(px(400.), px(500.))
        .modern(true)  // 默认 true，使用 ModernWindow（透明标题栏）
        .run();
}
```
框架自动：① 调用 `rml_ui::init(cx)` ② 创建 ModernWindow ③ 用 `Root` 包裹 `Counter` 视图 ④ 打开窗口

**模式 B：命令式（WPF OnStartup 重写风格，用于复杂场景）**
```rust
use rml_app::{IAppLifecycle, RmlApplication};
use rml_ui::prelude::*;  // 启用 WindowExt + init

struct MyApp;

impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut App) {
        rml_ui::init(cx);  // 用户手动调用 init
        ModernWindow::new("My App", px(800.), px(600.))
            .open::<MyView>(cx);  // WindowExt::open
    }
}

fn main() {
    RmlApplication::new().run::<MyApp>();
}
```

---

## 三、当前状态分析 Current State

### 已完成（代码已落地）

| 文件 | 状态 | 说明 |
|------|------|------|
| `crates/app/src/lifecycle.rs` | ✅ | `IAppLifecycle` trait |
| `crates/app/src/application.rs` | ✅ 但需重构 | `RmlApplication::run::<A>()` 调用 `rml_ui::init`（要移除） |
| `crates/app/src/window.rs` | ✅ 但需重构 | `Window`/`ModernWindow` 有 `open()` 方法（要移到 trait） |
| `crates/app/src/lib.rs` | ✅ 但需重构 | `extern crate rust_rml_ui`（要移除） |
| `crates/ui/src/window/` | ✅ | 5 个文件：types/actions/menu_bar/modern_window/mod |
| `crates/ui/src/lib.rs` + `prelude.rs` | ✅ 但需扩展 | re-export window 模块（要添加 Ext trait） |
| `crates/ui/Cargo.toml` | ✅ | smallvec 依赖 |

### 待重构（架构解耦）

| # | 位置 | 问题 | 解决 |
|---|------|------|------|
| 1 | `crates/app/Cargo.toml` | `rust-rml-ui` 可选依赖 | 移除（app 不依赖 ui） |
| 2 | `crates/app/src/lib.rs:19` | `extern crate rust_rml_ui` | 移除 |
| 3 | `crates/app/src/application.rs:61` | `rml_ui::init(cx)` 调用 | 移除（移到 ui crate） |
| 4 | `crates/app/src/window.rs:116-149` | `Window::open()` 实现（依赖 `rml_ui::Root`） | 移到 ui crate 的 `WindowExt` trait |
| 5 | `crates/app/src/window.rs:178-193` | `ModernWindow::open()` 实现 | 同上 |
| 6 | `crates/ui/Cargo.toml` | 未依赖 `rust-rml-app` | 添加依赖 |

### 待新增（声明式 API）

| # | 文件 | 内容 |
|---|------|------|
| 7 | `crates/ui/src/window/ext.rs` | `RmlApplicationExt` + `WindowExt` trait |
| 8 | `crates/ui/src/window/main_window_builder.rs` | `MainWindowBuilder<V>` 声明式构建器 |

### Phase 1 剩余工作（未开始）

| # | 范围 | 说明 |
|---|------|------|
| 9 | `crates/engine/src/tags.rs` | `component_lookup` 新增 TitleBar/StatusBar/ModernWindow 路由 + `StatelessNoId` 变体 |
| 10 | `crates/engine/src/compiler/component.rs` | `component_bind_setter` 添加 `tag` 参数 + ModernWindow 专用 setter |
| 11 | `demo/src/main.rs` | 迁移到声明式 `main_window::<V>().run()` |
| 12 | `demo/src/counter.rml` + `.rml.rs` | 根元素改为 `<ModernWindow>` + menu/status 绑定 |

---

## 四、实施步骤 Implementation Steps

### Step 1：`app` crate 解耦 —— 移除 UI 依赖

**文件**：`crates/app/Cargo.toml`
```toml
# 移除
# [dependencies]
# rust-rml-ui = { workspace = true, optional = true }
# [features]
# ui-components = ["dep:rust-rml-ui"]
```
保留 `default = []`（空 feature），或直接移除 features 段。

**文件**：`crates/app/src/lib.rs`
```rust
// 移除
// #[cfg(feature = "ui-components")]
// extern crate rust_rml_ui as rml_ui;
```
同时移除模块文档中关于 `ui-components` feature 的描述。

**文件**：`crates/app/src/application.rs`
```rust
pub fn run<A>(self)
where A: IAppLifecycle + Default + 'static,
{
    gpui_platform::application().run(move |cx: &mut App| {
        // 移除：rml_ui::init(cx);
        // 改由用户在 on_launch 中调用，或由 ui crate 的 run_with_ui 处理
        let mut app = A::default();
        app.on_launch(cx);
    });
}
```

**文件**：`crates/app/src/window.rs`
```rust
// 保留 Window / ModernWindow struct + new() + into_modern() + build_options()
// 移除所有 impl Window { pub fn open<V>(...) } 块（包括 cfg-gated 的两个版本）
// 暴露 build_options 为 pub：
pub fn build_options(&self) -> WindowOptions { ... }
```

**验证**：`cargo build -p rust-rml-app` 通过（不依赖 rml_ui）。

### Step 2：`ui` crate 添加 `app` 依赖

**文件**：`crates/ui/Cargo.toml`
```toml
[dependencies]
rust-rml-app = { workspace = true }  # 新增
# 其他现有依赖...
```

### Step 3：创建 `WindowExt` trait

**文件**：`crates/ui/src/window/ext.rs`（新建）

```rust
//! WindowExt —— 为 rml_app::Window / ModernWindow 添加 open() 方法
//!
//! 通过 `rml_ui::Root` 包裹业务 view，支持 Dialog/Sheet/Notification 等浮层。

use gpui::{App, AppContext, Render, WindowHandle};
use rml_app::{ModernWindow, Window};
use rml_core::view::IRmlView;
use crate::Root;

pub trait WindowExt: Sized {
    /// 打开窗口，以 `V` 为根视图
    ///
    /// 自动用 `rml_ui::Root` 包裹业务 view，支持 Dialog/Sheet/Notification 等浮层。
    fn open<V>(self, cx: &mut App) -> WindowHandle<Root>
    where V: IRmlView + Render + Default + 'static;
}

impl WindowExt for Window {
    fn open<V>(self, cx: &mut App) -> WindowHandle<Root>
    where V: IRmlView + Render + Default + 'static,
    {
        let options = self.build_options();
        cx.open_window(options, |window, cx| {
            let view = cx.new(|_cx| V::default());
            cx.new(|cx| Root::new(view, window, cx))
        }).expect("failed to open window")
    }
}

impl WindowExt for ModernWindow {
    fn open<V>(self, cx: &mut App) -> WindowHandle<Root>
    where V: IRmlView + Render + Default + 'static,
    {
        // ModernWindow 内部就是 Window，委托实现
        // 需要在 app crate 暴露 ModernWindow.0 或添加 into_inner()
        self.into_inner().open::<V>(cx)
    }
}
```

**注意**：`ModernWindow(Window)` 是 tuple struct，需要在 `app` crate 中暴露 `pub fn into_inner(self) -> Window` 或将字段设为 `pub`。

**文件**：`crates/app/src/window.rs` 补充：
```rust
impl ModernWindow {
    pub fn into_inner(self) -> Window {
        self.0
    }
}
```

### Step 4：创建 `RmlApplicationExt` + `MainWindowBuilder`

**文件**：`crates/ui/src/window/ext.rs`（续）

```rust
use rml_app::{IAppLifecycle, RmlApplication};
use std::marker::PhantomData;

/// 为 RmlApplication 添加声明式主窗口 API
pub trait RmlApplicationExt: Sized {
    /// 声明主窗口视图类型（WPF StartupUri 风格）
    ///
    /// ```rust,ignore
    /// RmlApplication::new()
    ///     .main_window::<MyView>()
    ///     .title("...").size(px(800.), px(600.))
    ///     .run();
    /// ```
    fn main_window<V>(self) -> MainWindowBuilder<V>
    where V: IRmlView + Render + Default + 'static;
}

impl RmlApplicationExt for RmlApplication {
    fn main_window<V>(self) -> MainWindowBuilder<V>
    where V: IRmlView + Render + Default + 'static,
    {
        MainWindowBuilder {
            title: "RML Application".into(),
            width: gpui::px(800.),
            height: gpui::px(600.),
            modern: true,
            _view: PhantomData,
        }
    }
}

/// 声明式主窗口构建器
pub struct MainWindowBuilder<V> {
    title: gpui::SharedString,
    width: gpui::Pixels,
    height: gpui::Pixels,
    modern: bool,
    _view: PhantomData<V>,
}

impl<V> MainWindowBuilder<V>
where V: IRmlView + Render + Default + 'static,
{
    pub fn title(mut self, title: impl Into<gpui::SharedString>) -> Self {
        self.title = title.into(); self
    }
    pub fn size(mut self, w: gpui::Pixels, h: gpui::Pixels) -> Self {
        self.width = w; self.height = h; self
    }
    pub fn modern(mut self, modern: bool) -> Self {
        self.modern = modern; self
    }

    /// 启动应用 —— 自动初始化 gpui-component + 创建窗口 + 打开视图
    pub fn run(self) {
        // 内部 App：在 on_launch 中执行所有初始化
        struct MainWindowApp {
            config: WindowConfig,
        }
        impl IAppLifecycle for MainWindowApp {
            fn on_launch(&mut self, cx: &mut App) {
                crate::init(cx);  // rml_ui::init
                if self.config.modern {
                    ModernWindow::new(&self.config.title, self.config.width, self.config.height)
                        .open::<V>(cx);
                } else {
                    Window::new(&self.config.title, self.config.width, self.config.height)
                        .open::<V>(cx);
                }
            }
        }
        RmlApplication::new().run::<MainWindowApp>();
    }
}

struct WindowConfig {
    title: gpui::SharedString,
    width: gpui::Pixels,
    height: gpui::Pixels,
}
```

### Step 5：集成 `ext` 模块到 `ui` crate

**文件**：`crates/ui/src/window/mod.rs`
```rust
pub mod actions;
pub mod ext;          // 新增
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use ext::{MainWindowBuilder, RmlApplicationExt, WindowExt};  // 新增
pub use modern_window::ModernWindow;
pub use types::{MenuItem, StatusBarItem};
```

**文件**：`crates/ui/src/lib.rs`
```rust
pub use window::{MainWindowBuilder, RmlApplicationExt, WindowExt, /* 现有... */};
```

**文件**：`crates/ui/src/prelude.rs`
```rust
pub use crate::window::{RmlApplicationExt, WindowExt, /* 现有... */};
```

**验证**：
```bash
cargo build -p rust-rml-app   # app crate 不依赖 ui，编译通过
cargo build -p rust-rml-ui     # ui crate 依赖 app，编译通过
```

### Step 6：扩展 `tags.rs` 路由表

**文件**：`crates/engine/src/tags.rs`

在 `ComponentKind` 枚举中添加 `StatelessNoId` 变体：
```rust
pub enum ComponentKind {
    Stateless,           // Type::new(id)
    StatelessNoId,       // Type::new()  —— 新增，用于 TitleBar/StatusBar/ModernWindow
    Stateful { state_field: &'static str },
}
```

在 `component_lookup` 中添加 3 个路由（Kbd 留 Phase 2）：
```rust
"TitleBar" => Some(ComponentTag { ctor_path: "rml_ui::TitleBar", kind: ComponentKind::StatelessNoId }),
"StatusBar" => Some(ComponentTag { ctor_path: "rml_ui::StatusBar", kind: ComponentKind::StatelessNoId }),
"ModernWindow" => Some(ComponentTag { ctor_path: "rml_ui::ModernWindow", kind: ComponentKind::StatelessNoId }),
```

### Step 7：扩展 `component.rs` 处理 `StatelessNoId` + ModernWindow setter

**文件**：`crates/engine/src/compiler/component.rs`

7.1 在 `gen_component` 的构造器 match 中添加：
```rust
tags::ComponentKind::StatelessNoId => {
    format!("{}::new()", component.ctor_path)
}
```

7.2 修改 `component_bind_setter` 签名，添加 `tag: &str` 参数：
```rust
pub fn component_bind_setter(
    name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str],
    tag: &str,  // 新增
) -> Option<String> {
    // ... 现有表达式解析逻辑 ...
    match name {
        "value" => ...,
        "disabled" => ...,
        // ModernWindow 专用 setter
        "menu" if tag == "ModernWindow" => Some(format!(".menu({})", rust_expr)),
        "status_bar" if tag == "ModernWindow" => Some(format!(".status_bar({})", rust_expr)),
        "title" if tag == "ModernWindow" => Some(format!(".title({})", rust_expr)),
        // 现有 label 等...
    }
}
```

7.3 在 `gen_component` 中调用 `component_bind_setter` 时传入 `tag`：
```rust
if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, tag) {
```

7.4 更新所有测试调用点（添加 `tag` 参数）。

**验证**：`cargo build -p rust-rml-engine` + `cargo test -p rust-rml-engine`（180+ 测试通过）。

### Step 8：迁移 demo `main.rs`

**文件**：`demo/src/main.rs`
```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

use rml_app::RmlApplication;
use rml_ui::prelude::*;  // 启用 RmlApplicationExt

#[path = "counter.rml.rs"]
mod counter;

fn main() {
    RmlApplication::new()
        .main_window::<counter::Counter>()
        .title("RML Counter Demo")
        .size(gpui::px(400.), gpui::px(500.))
        .run();
}
```

**注**：移除 `todos` 模块引用（暂不展示，Phase 4 多窗口示例再启用）。

### Step 9：迁移 demo `counter.rml` + `counter.rml.rs`

**文件**：`demo/src/counter.rml`

根元素改为 `<ModernWindow>`：
```html
<ModernWindow title="RML 计数器" menu={menu_items} status_bar={status_items}>
    <div class="counter">
        <h1 ref="title">计数器</h1>
        <p class="count">{count}</p>
        <p class="next">下一个：{count + 1}</p>
        <p class="double">两倍（计算属性）：{double_count}</p>
        <p class="positive">是否为正：{count > 0}</p>
        <p class="hover">悬停状态：{hovered | BoolToYesNo}</p>
        <p once>版本: v0.1.0 (固定不变)</p>
        <div class="buttons" onhover={on_hover_change}>
            <Button ref="dec_btn" label="-" onclick={decrement} />
            <Button ref="inc_btn" label="+" primary="" onclick={increment} />
        </div>
    </div>
</ModernWindow>
```

**文件**：`demo/src/counter.rml.rs`

新增 `menu_items` / `status_items` 字段 + 构建方法：
```rust
use rml::prelude::*;
use rml_ui::{MenuItem, StatusBarItem};

#[derive(Default)]
#[view]
pub struct Counter {
    pub count: i32,
    pub hovered: bool,
    pub menu_items: Vec<MenuItem>,
    pub status_items: Vec<StatusBarItem>,
}

impl Counter {
    #[computed]
    pub fn double_count(&self) -> i32 { self.count * 2 }

    fn build_menu() -> Vec<MenuItem> {
        vec![
            MenuItem::new("文件").submenu(vec![
                MenuItem::new("重置").on_click_with(/* cx */, |this, _w, _cx| {
                    this.count = 0;
                }),
                MenuItem::separator(),
                MenuItem::new("退出"),
            ]),
            MenuItem::new("帮助").submenu(vec![
                MenuItem::new("关于"),
            ]),
        ]
    }

    fn build_status(count: i32) -> Vec<StatusBarItem> {
        vec![
            StatusBarItem::new("就绪"),
            StatusBarItem::new(format!("计数: {}", count)),
        ]
    }

    // 现有 #[command] 方法保留...
}
```

**注意**：`menu_items` / `status_items` 的初始化需要在 view 创建时完成。具体实现细节（如在 `Default` 实现中填充，或通过 `#[view]` 宏的初始化钩子）在实现时确定。

### Step 10：全量验证

```bash
# 1. 编译验证
cargo build -p rust-rml-app    # app crate 无 UI 依赖
cargo build -p rust-rml-ui      # ui crate 含 Ext trait
cargo build -p rust-rml-engine   # tags.rs + component.rs 扩展
cargo build -p rust-rml-demo     # demo 迁移完成

# 2. 全 workspace 编译
cargo build --workspace

# 3. 单元测试
cargo test -p rust-rml-engine   # 180+ 现有测试 + 新增 StatelessNoId/ModernWindow setter 测试
cargo test -p rust-rml-core     # 24+ 测试

# 4. 运行验证
cargo run -p rust-rml-demo
```

**运行预期**：
- 窗口打开，标题栏显示 "RML 计数器"
- 菜单栏显示 "文件" / "帮助" 顶层项
- 状态栏显示 "就绪" / "计数: 0"
- 业务区显示计数器内容（+/- 按钮、computed 属性、converter 等）
- 点击 "重置" 菜单 → 计数归零 + 状态栏更新

---

## 五、文件变更清单 File Change List

### 修改（app crate 解耦）

| 文件 | 变更 |
|------|------|
| `crates/app/Cargo.toml` | 移除 `rust-rml-ui` 可选依赖 + `ui-components` feature |
| `crates/app/src/lib.rs` | 移除 `extern crate rust_rml_ui` + feature 文档 |
| `crates/app/src/application.rs` | `run()` 中移除 `rml_ui::init(cx)` 调用 |
| `crates/app/src/window.rs` | 移除 `Window::open()` / `ModernWindow::open()` 方法；`build_options()` 改为 `pub`；`ModernWindow` 添加 `into_inner()` |

### 新建（ui crate 扩展 trait）

| 文件 | 职责 |
|------|------|
| `crates/ui/src/window/ext.rs` | `RmlApplicationExt` + `WindowExt` + `MainWindowBuilder<V>` |

### 修改（ui crate 集成）

| 文件 | 变更 |
|------|------|
| `crates/ui/Cargo.toml` | 添加 `rust-rml-app` 依赖 |
| `crates/ui/src/window/mod.rs` | 添加 `pub mod ext;` + re-export |
| `crates/ui/src/lib.rs` | re-export `RmlApplicationExt` / `WindowExt` / `MainWindowBuilder` |
| `crates/ui/src/prelude.rs` | 添加 Ext trait 到 prelude |

### 修改（engine crate 路由 + setter）

| 文件 | 变更 |
|------|------|
| `crates/engine/src/tags.rs` | `ComponentKind` 添加 `StatelessNoId` + 3 个路由 |
| `crates/engine/src/compiler/component.rs` | `gen_component` 处理 `StatelessNoId`；`component_bind_setter` 添加 `tag` 参数 + ModernWindow setter；更新测试 |

### 迁移（demo）

| 文件 | 变更 |
|------|------|
| `demo/src/main.rs` | 迁移到声明式 `main_window::<V>().run()` |
| `demo/src/counter.rml` | 根元素改为 `<ModernWindow>` + menu/status_bar 绑定 |
| `demo/src/counter.rml.rs` | 新增 menu_items/status_items 字段 + build 方法 |

---

## 六、假设与决策 Assumptions & Decisions

### 假设
1. `#[view]` 宏生成的视图（Counter）自动实现 `IRmlView + Render + Default`，满足 `main_window::<V>()` 约束
2. `gpui-component` 的 `TitleBar` / `StatusBar` / `Root` API 稳定
3. `MenuItem::on_click_with` 的闭包捕获 `WeakEntity<T>` 模式可工作（已在 types.rs 中实现）

### 决策
1. **app crate 完全无 UI 依赖**：移除 `rust-rml-ui` 可选依赖，`RmlApplication::run()` 不调用 `rml_ui::init`
2. **声明式 API 在 ui crate**：通过 `RmlApplicationExt` extension trait 实现，用户 `use rml_ui::prelude::*` 启用
3. **命令式 API 需用户手动 init**：`IAppLifecycle::on_launch` 中用户自己调用 `rml_ui::init(cx)`
4. **Phase 1 仅链式 API 配置窗口**：`.rml` 根元素属性（`<ModernWindow title="..." width="...">`）留 Phase 2
5. **Kbd 留 Phase 2**：`Kbd::new(Keystroke)` 签名特殊，需单独处理
6. **demo 移除 todos 模块**：简化为单窗口 Counter 示例，Todos 留 Phase 4 多窗口示例

---

## 七、执行顺序 Execution Order

1. **Step 1-2**：app crate 解耦 + ui crate 添加 app 依赖 → `cargo build -p rust-rml-app` 通过
2. **Step 3-5**：创建 WindowExt + RmlApplicationExt + MainWindowBuilder + 集成 → `cargo build -p rust-rml-ui` 通过
3. **Step 6-7**：tags.rs 路由 + component.rs setter → `cargo build -p rust-rml-engine` + `cargo test` 通过
4. **Step 8-9**：demo 迁移 → `cargo build -p rust-rml-demo` 通过
5. **Step 10**：全量验证 → `cargo build --workspace` + `cargo run -p rust-rml-demo`

每个步骤完成后立即验证编译，避免错误累积。

---

## 八、与原计划的关系 Relationship to Original Plans

### 与 `window-api-revision-and-phase1-completion-plan.md` 的关系

- **Step 1-6（已完成）**：保持现状（app crate 修复 + ui crate window 模块创建）
- **Step 7-8（原 tags.rs + component.rs）**：本计划 Step 6-7，扩展为 `StatelessNoId` + ModernWindow setter
- **Step 9-10（原 demo 迁移）**：本计划 Step 8-9，迁移到声明式 API
- **Step 11（原验证）**：本计划 Step 10

### 新增内容（本计划独有）

- **架构解耦**：app crate 移除 UI 依赖（原计划未涉及）
- **声明式 API**：`RmlApplicationExt` + `MainWindowBuilder`（原计划只有命令式）
- **`StatelessNoId` 变体**：处理 TitleBar/StatusBar/ModernWindow 的无参 `new()`（原计划用 `Stateless`，不准确）

### 后续 Phase（保持不变）

- **Phase 2**：`<bind>` 快捷键 codegen + `<Kbd>` 显示组件 + Action 注册 + `.rml` 根元素属性配置
- **Phase 3**：扩展 `component_lookup` 至 40+ 组件 + 每组件属性映射
- **Phase 4**：List/Table/Dialog/Select 等有状态组件 + 多窗口 + 主题
