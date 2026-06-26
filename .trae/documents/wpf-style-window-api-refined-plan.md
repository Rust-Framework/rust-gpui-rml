# WPF 风格窗口与应用 API 实施计划（精炼版）

> 本计划基于已批准的 `wpf-style-window-and-application-api-plan.md`，结合用户最新澄清：
> - **IWindow 必须是窗口通用操作的内置抽象**（open/show/close/state 自管理，不通过扩展）
> - **参考 WPF/MAUI 的组件与窗体设计理念**
> - **充分发挥 Rust 优良特性**（类型状态模式、trait 继承、零成本抽象）

---

## 一、设计核心 Core Design

### 设计哲学

| 原则 | 实现 | WPF 等价物 |
|------|------|-----------|
| 窗口 IS 组件 | `IWindow: IComponent` | `Window : ContentControl` |
| 窗口自管理生命周期 | `IWindow::open/close/show/state` 内置方法 | `Window.Show()/Close()/Hide()` |
| 主窗口是内置功能 | `RmlApplication<W: IWindow>.main_window::<W>()` | `Application.MainWindow` |
| 类型状态构建器 | `RmlApplication<()>` → `RmlApplication<W>` | 编译期保证 `run()` 前设置主窗口 |
| 声明式启动 | `RmlApplication::new().main_window::<W>().run()` | `Application.StartupUri` |

### Trait 层次（已落地于 `crates/core`）

```
IModel (纯数据模型)
  └─ ILifecycle (生命周期回调)
       └─ IViewModel: IModel + ILifecycle (ViewModel — 状态 + 命令)
            └─ IComponent: IViewModel (组件 — rml_template() + rml_tag())
                 └─ IWindow: IComponent (窗口 — 配置 + open/show/close/state)
```

**关键：IWindow 的窗口操作是内置必需方法，不是扩展 trait 提供的。**

---

## 二、当前状态 Current State

### 已完成（Step 1 部分）

| 文件 | 状态 | 说明 |
|------|------|------|
| [component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/component.rs) | ✅ 已重写 | 合并 `IRmlView` + 旧 `IComponent` → 新 `IComponent`（含 `rml_template()` + `rml_tag()`） |
| [window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/window.rs) | ✅ 已创建 | `IWindow: IComponent` trait + `WindowChrome` + `WindowState`，使用 `AnyWindowHandle` 存储句柄，含 `open/show/close/hide/activate/state/handle/set_handle` 内置方法 |

### 待完成

| 文件 | 现状 | 目标 |
|------|------|------|
| [view.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/view.rs) | 仍存在旧 `IRmlView` | **删除**（已合并到 component.rs） |
| [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs) (core) | 有 `pub mod view;` | 移除 `view`，新增 `window` |
| [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) | 有 `pub use crate::view::IRmlView;` | 移除 IRmlView，新增 `IWindow/WindowChrome/WindowState` |
| [view.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/view.rs) | 生成 `impl IRmlView` | **重命名为 component.rs**，生成 `impl IComponent` |
| [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lib.rs) | 注册 `#[view]` + 旧 `#[component]` | 合并为新 `#[component]` + 新增 `#[window]` |
| [application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs) | 旧 `RmlApplication`（无泛型） | `RmlApplication<W: IWindow = ()>` + 内置 `main_window` |
| [window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/window.rs) (app) | 旧 `Window`/`ModernWindow` 配置对象 | **移到 ui crate** 并实现 `IWindow` |
| [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/lib.rs) | 有 ui 依赖 + window 模块 | 移除 ui 依赖 + window 模块 |
| [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/Cargo.toml) | 有 `rust-rml-ui` 可选依赖 + `ui-components` feature | 移除 ui 依赖与 feature |
| [mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/mod.rs) (ui) | 仅 ModernWindow RML 组件 | 新增 `builtin_window.rs`（内置 Window/ModernWindow IWindow 实现） |
| [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs) | 旧重导出 | 更新 + 导出内置 Window/ModernWindow（IWindow 实现） |
| [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/prelude.rs) | 旧 prelude | 更新 |
| [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/Cargo.toml) | 无 app 依赖 | 添加 `rust-rml-app` 依赖 |
| [counter.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/counter.rml.rs) | `#[view]` | `#[window]` |
| [todos.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/todos.rml.rs) | `#[view]` | `#[window]` |
| [main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs) | 用 `.title()/.size().run::<V>()`（不存在的方法） | `RmlApplication::new().main_window::<W>().run()` |

### 命名冲突处理

当前 `rml_ui::ModernWindow`（RML 视觉外壳组件，`RenderOnce`，在 [modern_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs)）与即将从 app crate 迁入的 `ModernWindow`（IWindow 配置对象实现）冲突。

**决策**：将 RML 视觉外壳组件重命名为 `ModernWindowShell`，`ModernWindow` 名称让给 IWindow 实现。

---

## 三、依赖方向 Dependency Direction

```
demo ──► app (不依赖 ui)     app ──► core (IWindow 定义在 core)
  │                              │
  └──► ui (内置 Window IWindow)  ui ──► app + core + gpui-component
```

**关键**：`app` crate **不依赖** `ui` crate。`RmlApplication.main_window::<W: IWindow>()` 仅依赖 core 的 `IWindow` trait 约束。窗口打开逻辑（含 `rml_ui::Root` 包裹）由 `W` 的 `IWindow::open()` 实现负责（在 ui crate 或用户代码中）。

---

## 四、实施步骤 Implementation Steps

### Step 1：完成 Core trait 合并（收尾）

**目标**：删除旧 `view.rs`，更新模块声明与 prelude。

**文件变更**：
1. **删除** `crates/core/src/view.rs`
2. **修改** [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs)：
   - 移除 `pub mod view;`
   - 新增 `pub mod window;`
3. **修改** [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs)：
   - 移除 `pub use crate::view::IRmlView;`
   - 新增 `pub use crate::window::{IWindow, WindowChrome, WindowState};`

**验证**：`cargo build -p rust-rml-core` 通过。

### Step 2：宏更新 —— `#[view]` → `#[component]` + 新增 `#[window]`

**目标**：合并旧 `#[view]` 与 `#[component]` 为新 `#[component]`，新增 `#[window]` 宏。

**文件变更**：
1. **重命名** [view.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/view.rs) → `crates/macros/src/component.rs`：
   - `expand()` 生成 `impl IComponent`（含 `rml_template()` + `rml_tag()`），不再生成 `impl IRmlView`
   - 路径更新：`rml_core::view::IRmlView` → `rml_core::component::IComponent`
   - 移除 `is_component` 分支（所有 `#[component]` 都生成完整 `IComponent` impl）
2. **新建** `crates/macros/src/window.rs`：
   - `expand()` 在 `#[component]` 基础上额外生成 `impl IWindow`
   - 自动添加 `__rml_window_handle: Option<AnyWindowHandle>` 字段
   - 解析 `#[window(title = "...", width = N, height = N)]` 属性参数
   - 生成 `open()`/`close()`/`show()`/`hide()`/`activate()`/`state()`/`handle()`/`set_handle()` 实现
   - `open()` 实现中调用 `cx.open_window()` 创建窗口，用 `rml_ui::Root` 包裹业务 view
3. **修改** [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lib.rs)：
   - `mod view;` → `mod component;` + `mod window;`
   - `pub fn view` → 移除（合并到 `#[component]`）
   - `pub fn component` → 调用 `component::expand`
   - 新增 `pub fn window` → 调用 `window::expand`

**关键实现细节**：
- `#[window]` 宏依赖 `rml_ui` crate（用于 `Root` 包裹），因此宏 crate 需新增 `rust-rml-ui` 依赖
- 或：宏生成的 `open()` 代码通过 `extern crate` 引用 `rml_ui`（在用户 crate 中声明），宏 crate 不直接依赖 ui

**验证**：`cargo build -p rust-rml-macros` 通过。

### Step 3：App crate 重构 —— 内置 `main_window` + 移除 UI 依赖

**目标**：`RmlApplication<W: IWindow = ()>` 泛型化，内置 `main_window` 方法，移除对 ui crate 的依赖。

**文件变更**：
1. **重写** [application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs)：
   ```rust
   pub struct RmlApplication<W: IWindow = ()> {
       _window: PhantomData<W>,
   }

   impl RmlApplication<()> {
       pub fn new() -> Self { Self { _window: PhantomData } }

       /// 命令式启动（WPF OnStartup 重写风格）
       pub fn run<A: IAppLifecycle + Default + 'static>(self) {
           gpui_platform::application().run(move |cx: &mut App| {
               let mut app = A::default();
               app.on_launch(cx);
           });
       }
   }

   impl<W: IWindow + Default + 'static> RmlApplication<W> {
       /// 声明式设置主窗口类型（WPF StartupUri 风格，内置方法）
       pub fn main_window<NewW: IWindow + Default + 'static>(self) -> RmlApplication<NewW> {
           RmlApplication { _window: PhantomData }
       }

       /// 启动应用并打开主窗口
       pub fn run(self) {
           gpui_platform::application().run(move |cx: &mut App| {
               let mut window = W::default();
               window.open(cx);
           });
       }
   }
   ```
2. **删除/清空** [window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/window.rs)（Window/ModernWindow 移到 ui crate）
3. **修改** [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/lib.rs)：
   - 移除 `#[cfg(feature = "ui-components")] extern crate rust_rml_ui as rml_ui;`
   - 移除 `pub mod window;`
   - 移除 `pub use window::{Window, ModernWindow};`
4. **修改** [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/Cargo.toml)：
   - 移除 `rust-rml-ui = { workspace = true, optional = true }`
   - 移除 `[features]` 段（`default = ["ui-components"]` / `ui-components = [...]`）
   - 新增 `rust-rml-core` 依赖（已有）

**验证**：`cargo build -p rust-rml-app` 通过（不依赖 ui crate）。

### Step 4：UI crate 扩展 —— 内置 Window/ModernWindow 实现 IWindow

**目标**：将 app crate 中的 Window/ModernWindow 迁移到 ui crate，实现 `IWindow` trait，作为开箱即用的窗口基类。

**文件变更**：
1. **修改** [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/Cargo.toml)：
   - 新增 `rust-rml-app = { workspace = true }` 依赖
2. **新建** `crates/ui/src/window/builtin_window.rs`：
   - `Window` 结构体（原生标题栏窗口）实现 `IWindow`
   - `ModernWindow` 结构体（透明标题栏窗口）实现 `IWindow`
   - 字段：`title`/`width`/`height`/`chrome`/`__rml_window_handle: Option<AnyWindowHandle>`
   - `open()` 实现：构建 `WindowOptions`，调用 `cx.open_window()`，用 `rml_ui::Root` 包裹业务 view
   - 注意：这些是"配置对象"风格的窗口（类似 WPF `Window` 基类），不绑定特定 view 类型
3. **修改** [mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/mod.rs)：
   - 新增 `pub mod builtin_window;`
   - 新增 `pub use builtin_window::{Window, ModernWindow};`
   - 重命名现有 `ModernWindow`（RML 外壳）为 `ModernWindowShell`：`pub use modern_window::ModernWindow as ModernWindowShell;`
4. **修改** [modern_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs)：
   - 将 `ModernWindow` 重命名为 `ModernWindowShell`（避免与 builtin_window 的 `ModernWindow` 冲突）
5. **修改** [lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)：
   - 更新重导出：`ModernWindow` → `ModernWindowShell`，新增 `Window`/`ModernWindow`（IWindow 实现）
6. **修改** [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/prelude.rs)：
   - 更新重导出列表

**验证**：`cargo build -p rust-rml-ui` 通过。

### Step 5：Demo 迁移

**目标**：demo 改用 `#[window]` 宏与新 `main_window` API。

**文件变更**：
1. **修改** [counter.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/counter.rml.rs)：
   ```rust
   use rml::prelude::*;
   use rml_ui::prelude::*;  // 启用 #[window] 宏

   #[window(title = "RML Counter Demo", width = 400, height = 500)]
   #[derive(Default)]
   pub struct Counter {
       pub count: i32,
       pub hovered: bool,
   }
   // ... impl Counter 不变
   ```
2. **修改** [todos.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/todos.rml.rs)：类似改为 `#[window]`
3. **修改** [main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs)：
   ```rust
   use rml_app::RmlApplication;
   use rml_ui::prelude::*;

   #[path = "counter.rml.rs"]
   mod counter;

   fn main() {
       RmlApplication::new()
           .main_window::<counter::Counter>()  // 内置方法
           .run();
   }
   ```
4. **修改** [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/demo/Cargo.toml)：
   - `rust-rml-app` 移除 `features = ["ui-components"]`（feature 已删除）

**验证**：`cargo build -p rust-rml-demo` + `cargo run -p rust-rml-demo` 通过。

### Step 6：全量验证

```bash
cargo build -p rust-rml-core
cargo build -p rust-rml-macros
cargo build -p rust-rml-app
cargo build -p rust-rml-ui
cargo build -p rust-rml-engine
cargo build -p rust-rml-demo
cargo build --workspace
cargo test --workspace
cargo run -p rust-rml-demo
```

---

## 五、假设与决策

### 关键决策
1. **`IWindow: IComponent`**：窗口 IS 组件（WPF `Window : ContentControl` 等价），窗口操作是 IWindow 的内置必需方法，不通过扩展 trait
2. **`AnyWindowHandle` 存储通用窗口句柄**：因 `WindowHandle<()>` 要求 `(): Render` 不可行，改用 `AnyWindowHandle` 类型擦除
3. **`main_window` 是 `RmlApplication` 内置方法**：泛型类型状态模式 `RmlApplication<W: IWindow>`，编译期保证 `run()` 前设置主窗口
4. **`app` crate 不依赖 `ui` crate**：`IWindow` 定义在 core，`IWindow::open()` 实现由具体类型（ui crate 或用户代码）提供
5. **`rml_ui::ModernWindow`（RML 外壳）重命名为 `ModernWindowShell`**：释放 `ModernWindow` 名称给 IWindow 实现
6. **`#[window]` 宏依赖 `rml_ui`**：宏生成的 `open()` 代码引用 `rml_ui::Root`，需确保用户 crate 声明 `extern crate rust_rml_ui`

### 待实现时验证的细节
1. GPUI `AnyWindowHandle` 的 `close/minimize/show/hide` 具体 API（参考 gpui crate 源码）
2. `#[window]` 宏如何处理 `rml_ui::Root` 引用（宏 crate 依赖 ui 还是用户 crate 声明 extern）
3. 内置 `Window`/`ModernWindow` 是否需要关联 view 类型（泛型 `Window<V>` 或纯配置对象）

---

## 六、执行顺序

1. Step 1：Core trait 合并收尾 → `cargo build -p rust-rml-core`
2. Step 2：宏更新 → `cargo build -p rust-rml-macros`
3. Step 3：App crate 重构 → `cargo build -p rust-rml-app`
4. Step 4：UI crate 扩展 → `cargo build -p rust-rml-ui`
5. Step 5：Demo 迁移 → `cargo build -p rust-rml-demo`
6. Step 6：全量验证 → `cargo build --workspace` + `cargo test` + `cargo run`

每步完成后立即验证编译，避免错误累积。
