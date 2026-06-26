# WPF 风格窗口 API — 最终实现计划

> 本计划取代之前的 `wpf-style-window-api-refined-plan.md`，基于已完成的 Steps 1-3/5 和探索结果，聚焦剩余工作。

## 一、概述

将 rust-gpui-rml 的窗口 API 打造为 WPF/MAUI 风格的框架体验：
- `IWindow` trait 提供窗口通用操作（open/show/close/state 等）的**默认实现**，真正"自管理"
- 内置 `Window`/`ModernWindow` 类型，开箱即用
- `#[window]` 宏作为用户创建窗口的主要方式，精简为只生成必要方法
- 全面清理过时文档（`#[view]`→`#[component]`、`IRmlView`→`IComponent` 等）

## 二、当前状态分析

### 已完成（Steps 1-3, 5）
| 文件 | 状态 | 说明 |
|------|------|------|
| `crates/core/src/component.rs` | ✅ | `IComponent` trait（合并自 `IRmlView`） |
| `crates/core/src/window.rs` | ✅ | `IWindow` trait，但 close/show/hide 等为**必需方法** |
| `crates/core/src/lib.rs` | ✅ | `pub mod window;`，无 `pub mod view;` |
| `crates/core/src/prelude.rs` | ✅ | 导出 `IWindow`/`IComponent`，无 `IRmlView` |
| `crates/macros/src/component.rs` | ✅ | `#[component]` 宏 |
| `crates/macros/src/window.rs` | ✅ | `#[window]` 宏，但生成所有方法（含 close/show 等） |
| `crates/macros/src/lib.rs` | ✅ | `mod component; mod window;`，无 `mod view;` |
| `crates/app/src/application.rs` | ✅ | `RmlApplication<W>` 类型状态，`main_window::<W>()` 内置 |
| `crates/ui/src/window/modern_window.rs` | ✅ | `ModernWindowShell`（已重命名） |
| `demo/src/counter.rml.rs` | ✅ | 使用 `#[window(...)]` |
| `demo/src/main.rs` | ✅ | `RmlApplication::new().main_window::<Counter>().run()` |

### 待完成
| 项目 | 说明 |
|------|------|
| IWindow 默认实现 | close/show/hide/activate/state 改为 trait 默认实现（基于 `handle()`） |
| `#[window]` 宏精简 | 移除 close/show/hide/activate/state 生成，依赖 trait 默认实现 |
| Step 4: 内置窗口类型 | `crates/ui/src/window/builtin_window.rs` — `Window` + `ModernWindow` |
| 源码文档清理 | `lifecycle.rs` 注释、`types.rs` 注释、`Cargo.toml` description |
| README 更新 | 各 crate README 中的 `#[view]`/`IRmlView`/`ModernWindow` 引用 |
| docs/** 更新 | ~100 处 `#[view]` → `#[component]` 等批量替换 |
| 全量验证 | `cargo build --workspace` + `cargo test` + `cargo run` |

## 三、实施步骤

### Phase 1: IWindow trait 添加默认实现

**文件**: `crates/core/src/window.rs`

**目标**: 将 `close()`/`show()`/`hide()`/`activate()`/`state()` 从必需方法改为**默认实现**，基于 `handle()` 调用 GPUI API。使 IWindow 真正"自管理"——用户只需实现 6 个核心方法即可获得完整窗口行为。

**改动**:
```rust
pub trait IWindow: IComponent {
    // === 必需：配置 ===
    fn title(&self) -> &str;
    fn width(&self) -> Pixels;
    fn height(&self) -> Pixels;

    // === 必需：句柄管理 ===
    fn handle(&self) -> Option<AnyWindowHandle>;
    fn set_handle(&mut self, handle: AnyWindowHandle);

    // === 必需：打开窗口（创建窗口实例） ===
    fn open(&mut self, cx: &mut App);

    // === 默认：窗口装饰 ===
    fn chrome(&self) -> WindowChrome { WindowChrome::Transparent }
    fn window_options(&self) -> WindowOptions { /* 现有默认实现保持不变 */ }

    // === 默认：窗口操作（基于 handle 自管理） ===
    fn close(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.remove_window();
            });
        }
    }

    fn show(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.activate_window();
            });
        }
    }

    fn hide(&mut self, cx: &mut App) {
        // GPUI 无法隐藏单个窗口，使用 minimize 作为最接近的替代
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.minimize_window();
            });
        }
    }

    fn activate(&mut self, cx: &mut App) {
        if let Some(handle) = self.handle() {
            let _ = handle.update(cx, |_view, window, _cx| {
                window.activate_window();
            });
        }
    }

    fn state(&self, cx: &mut App) -> WindowState {
        // GPUI 限制：无 is_minimized()，仅能检测 Maximized
        if let Some(handle) = self.handle() {
            if let Ok(maximized) = handle.update(cx, |_view, window, _cx| {
                window.is_maximized()
            }) {
                if maximized {
                    return WindowState::Maximized;
                }
            }
        }
        WindowState::Normal
    }
}
```

**设计理由**:
- WPF `Window` 类内置 Show/Close/Activate 行为，用户无需重写
- 集中窗口操作逻辑到 trait，消除 `#[window]` 宏中的重复代码
- 手动实现 `IWindow` 只需 6 个方法（title/width/height/handle/set_handle/open），而非 11 个

### Phase 2: 精简 `#[window]` 宏

**文件**: `crates/macros/src/window.rs`

**目标**: 移除 `close()`/`show()`/`hide()`/`activate()`/`state()` 的生成代码，依赖 Phase 1 的 trait 默认实现。

**改动**: 在 `gen_impl_iwindow()` 函数中：
- **保留生成**: `title()`、`width()`、`height()`、`window_options()`、`open()`、`handle()`、`set_handle()`
- **移除生成**: `close()`、`show()`、`hide()`、`activate()`、`state()`（由 trait 默认实现提供）

**注意**: `open()` 仍然由宏生成，因为其逻辑特定于 RML 组件（创建 `Self::default()` 视图 + `rml_ui::Root` 包装）。

### Phase 3: 内置 Window/ModernWindow（Step 4）

**新建文件**: `crates/ui/src/window/builtin_window.rs`

**目标**: 提供开箱即用的 `IWindow` 实现，WPF `Window` 类风格的直接可用窗口类型。

#### 3.1 `Window` — 基础窗口

```rust
use gpui::*;
use rml_core::prelude::*;
use rml_core::{IComponent, IModel, ILifecycle, IViewModel, IWindow, WindowChrome, WindowState};

/// 基础窗口 — 无装饰，仅承载内容视图。
///
/// 类比 WPF `Window` 类：可直用，也可作为更复杂窗口的基础。
pub struct Window {
    title: SharedString,
    width: Pixels,
    height: Pixels,
    content: Option<AnyView>,
    handle: Option<AnyWindowHandle>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "RML Window".into(),
            width: px(800.),
            height: px(600.),
            content: None,
            handle: None,
        }
    }
}

impl Window {
    pub fn new() -> Self { Self::default() }
    pub fn title(mut self, title: impl Into<SharedString>) -> Self { self.title = title.into(); self }
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self { self.width = w; self.height = h; self }
    pub fn content(mut self, content: impl Into<AnyView>) -> Self { self.content = Some(content.into()); self }
}

// 手动实现 trait 层级（ui crate 无 RML 构建过程，不能用 #[component] 宏）
impl IModel for Window {}
impl ILifecycle for Window {}
impl IViewModel for Window {}
impl IComponent for Window {
    fn rml_template() -> &'static str { "" }  // 无 RML 模板
    fn rml_tag() -> &'static str { "Window" }
}

impl Render for Window {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // 渲染内容视图，无内容时显示占位符
        if let Some(content) = self.content.clone() {
            content.into_element()
        } else {
            div().child("Window content not set")
        }
    }
}

impl IWindow for Window {
    fn title(&self) -> &str { &self.title }
    fn width(&self) -> Pixels { self.width }
    fn height(&self) -> Pixels { self.height }

    fn open(&mut self, cx: &mut App) {
        let options = self.window_options();
        let title = self.title.clone();
        let content = self.content.clone();
        let handle = cx.open_window(options, |window, cx| {
            let view = cx.new(|_| Self { title, content, ..Default::default() });
            cx.new(|cx| rml_ui::Root::new(view, window, cx))
        }).expect("failed to open window");
        self.handle = Some(handle.into());
    }

    fn handle(&self) -> Option<AnyWindowHandle> { self.handle }
    fn set_handle(&mut self, h: AnyWindowHandle) { self.handle = Some(h); }
    // close/show/hide/activate/state — 使用 trait 默认实现
}
```

#### 3.2 `ModernWindow` — 现代窗口（带 chrome）

```rust
/// 现代窗口 — 使用 ModernWindowShell 提供 TitleBar/MenuBar/StatusBar。
///
/// 类比 WPF `NavigationWindow` 或带 chrome 的窗口。
pub struct ModernWindow {
    title: SharedString,
    width: Pixels,
    height: Pixels,
    content: Option<AnyView>,
    menu: Vec<MenuItem>,
    status_bar: Vec<StatusBarItem>,
    handle: Option<AnyWindowHandle>,
}

impl Default for ModernWindow {
    fn default() -> Self {
        Self {
            title: "RML Application".into(),
            width: px(1024.),
            height: px(768.),
            content: None,
            menu: Vec::new(),
            status_bar: Vec::new(),
            handle: None,
        }
    }
}

impl ModernWindow {
    pub fn new() -> Self { Self::default() }
    pub fn title(mut self, title: impl Into<SharedString>) -> Self { self.title = title.into(); self }
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self { self.width = w; self.height = h; self }
    pub fn content(mut self, content: impl Into<AnyView>) -> Self { self.content = Some(content.into()); self }
    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self { self.menu = menu; self }
    pub fn status_bar(mut self, items: Vec<StatusBarItem>) -> Self { self.status_bar = items; self }
}

impl IModel for ModernWindow {}
impl ILifecycle for ModernWindow {}
impl IViewModel for ModernWindow {}
impl IComponent for ModernWindow {
    fn rml_template() -> &'static str { "" }
    fn rml_tag() -> &'static str { "ModernWindow" }
}

impl Render for ModernWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let shell = ModernWindowShell::new(self.content.clone().unwrap_or_default())
            .title(self.title.clone());
        // 注：menu/status_bar 的绑定需根据 ModernWindowShell API 调整
        shell
    }
}

impl IWindow for ModernWindow {
    fn title(&self) -> &str { &self.title }
    fn width(&self) -> Pixels { self.width }
    fn height(&self) -> Pixels { self.height }
    fn chrome(&self) -> WindowChrome { WindowChrome::Native }

    fn open(&mut self, cx: &mut App) {
        let options = self.window_options();
        let title = self.title.clone();
        let content = self.content.clone();
        let handle = cx.open_window(options, |window, cx| {
            let view = cx.new(|_| Self { title, content, ..Default::default() });
            cx.new(|cx| rml_ui::Root::new(view, window, cx))
        }).expect("failed to open window");
        self.handle = Some(handle.into());
    }

    fn handle(&self) -> Option<AnyWindowHandle> { self.handle }
    fn set_handle(&mut self, h: AnyWindowHandle) { self.handle = Some(h); }
}
```

#### 3.3 更新 ui crate 导出

**修改**: `crates/ui/src/window/mod.rs`
```rust
mod builtin_window;
pub use builtin_window::{Window, ModernWindow};
```

**修改**: `crates/ui/src/lib.rs`
```rust
pub use window::{Window, ModernWindow, ModernWindowShell, /* ... */};
```

**修改**: `crates/ui/src/prelude.rs`
```rust
pub use window::{Window, ModernWindow, ModernWindowShell, /* ... */};
```

### Phase 4: 清理源码文档

| 文件 | 改动 |
|------|------|
| `crates/app/src/lifecycle.rs` | 更新 doc 注释：移除 `rml_app::ModernWindow::new(...).open::<MyView>(cx)` 引用，改为 `IWindow::open()` 模式 |
| `crates/ui/src/window/types.rs:4` | `<ModernWindow>` → `<ModernWindowShell>` |
| `crates/macros/Cargo.toml` | description 中 `#[view]` → `#[component]`/`#[window]` |
| `crates/core/src/command.rs:27` | 注释中 `#[view]` → `#[component]` |

### Phase 5: 更新各 crate README

| 文件 | 改动 |
|------|------|
| `crates/core/README.md` | trait 表中 `IRmlView` → `IComponent`，添加 `IWindow`，更新 `#[view]` → `#[component]` |
| `crates/macros/README.md` | `#[view]` → `#[component]`，添加 `#[window]` 宏说明 |
| `crates/app/README.md` | 更新 `RmlApplication` API 示例（`main_window::<W>().run()`），移除 `RmlApplicationExt` |
| `crates/ui/README.md` | `ModernWindow` → `ModernWindowShell`，添加内置 `Window`/`ModernWindow` 说明 |
| `demo/README.md` | 更新示例代码为 `#[window]` + `main_window::<Counter>().run()` |

### Phase 6: 更新 docs/** 用户指南

**范围**: `docs/` 目录下所有 `.md` 文件

**批量替换**:
1. `#[view(` → `#[component(` 或 `#[window(`（根据上下文）
2. `#[view]` → `#[component]` 或 `#[window]`
3. `IRmlView` → `IComponent`
4. `rml_view` → `rml_component`（模块路径）
5. `use rml::prelude::*;` 示例中确保 `#[window]`/`#[component]` 可用
6. 添加 `#[window]` 宏的使用教程
7. 添加 `IWindow` trait 文档
8. 添加内置 `Window`/`ModernWindow` 使用文档

**注意**: 需逐个检查上下文，区分是组件还是窗口的 `#[view]`。

### Phase 7: 全量验证

```bash
# 1. 工作区编译
cargo build --workspace

# 2. 工作区测试
cargo test --workspace

# 3. Demo 运行
cargo run -p rust-rml-demo
```

**预期验证点**:
- `IWindow` trait 默认实现编译通过
- `#[window]` 宏精简后编译通过（依赖 trait 默认实现）
- 内置 `Window`/`ModernWindow` 编译通过
- Demo 正常启动并显示 Counter 窗口

## 四、假设与决策

### 决策 1: IWindow 默认实现
- **选择**: close/show/hide/activate/state 使用 trait 默认实现
- **理由**: WPF `Window` 类内置这些行为；集中逻辑到 trait 消除宏中重复；手动实现 IWindow 更简单
- **影响**: `#[window]` 宏精简，内置窗口类型复用默认实现

### 决策 2: 内置窗口类型实现方式
- **选择**: 手动实现所有 trait（`IModel`/`ILifecycle`/`IViewModel`/`IComponent`/`Render`/`IWindow`）
- **理由**: ui crate 无 RML 构建过程（`build.rs`），不能用 `#[component]` 宏（会生成 `include!` RML codegen）
- **影响**: 代码量较多，但清晰直接

### 决策 3: 内置窗口的 `rml_template()` 返回空字符串
- **选择**: `fn rml_template() -> &'static str { "" }`
- **理由**: 内置窗口是程序化容器，无 RML 模板；`Render` 手动实现
- **影响**: `IComponent` 的 `rml_template()` 对内置窗口无意义，但 trait 要求实现

### 决策 4: `open()` 仍为必需方法
- **选择**: `open()` 不提供默认实现
- **理由**: `open()` 需要创建 `Self::default()` 视图并包装在 `Root` 中，逻辑特定于具体窗口类型；内置窗口和宏生成窗口的 `open()` 行为不同
- **影响**: 每个 IWindow 实现需自定义 `open()`

### 决策 5: `state()` 的 GPUI 限制
- **选择**: 默认实现只能检测 Normal/Maximized，无法检测 Minimized
- **理由**: GPUI 无 `is_minimized()` API
- **影响**: `WindowState::Minimized` 在默认实现中不可达；用户可覆盖 `state()` 实现自定义检测

### 决策 6: ui crate 不依赖 app crate
- **选择**: `crates/ui/Cargo.toml` 不添加 `rust-rml-app` 依赖
- **理由**: `IWindow` trait 在 `core` crate（ui 已依赖）；`RmlApplication` 在 `app` crate（不需要反向依赖）
- **影响**: 依赖方向保持 `app → core`，`ui → core`，`app` 不依赖 `ui`

## 五、验证清单

- [ ] Phase 1: `crates/core/src/window.rs` — IWindow trait 默认实现编译通过
- [ ] Phase 2: `crates/macros/src/window.rs` — 精简后的 `#[window]` 宏编译通过
- [ ] Phase 3: `crates/ui/src/window/builtin_window.rs` — `Window`/`ModernWindow` 编译通过
- [ ] Phase 4: 源码文档无过时引用
- [ ] Phase 5: 各 crate README 更新
- [ ] Phase 6: docs/** 无 `#[view]`/`IRmlView` 残留
- [ ] Phase 7: `cargo build --workspace` 通过
- [ ] Phase 7: `cargo test --workspace` 通过
- [ ] Phase 7: `cargo run -p rust-rml-demo` 正常启动
