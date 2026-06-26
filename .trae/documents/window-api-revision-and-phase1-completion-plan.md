# 窗口 API 修订与 Phase 1 收尾 实施计划

> 本计划是对 `window-and-component-integration-plan.md` 的修订与收尾。
> 修订起因：用户两条澄清反馈 + `window.rs` 已实际重写为 struct-based API。
> 本计划聚焦于：同步修复下游不一致 + 完成 Phase 1 剩余工作。

---

## 一、修订背景 Revision Context

### 用户反馈

1. **Window/ModernWindow 关系澄清**（`/plan` 命令）：
   > `.rml` 中可以通过组装 TitleBar 等组件构造出 ModernWindow 的效果，ModernWindow 的提供是一种易用性封装，基于 ModernWindow 构建的 `.rml` 文件代码更少，更符合现代视觉效果应用设计的需求。

2. **窗口打开 API 形式**（system-reminder 反馈）：
   > 3.3 窗口打开 helper 做法不优雅，应该能够直接创建一个 Window 或 ModernWindow 对象，调用 open 方法，跟 WPF 一样操作。

### 现状与原计划的偏差

原计划 `window-and-component-integration-plan.md` §3.3 设想的窗口 helper 是**自由函数**：

```rust
// 原计划设想（已过时）
pub fn open_window<V>(cx: &mut App, title: ..., width: Pixels, height: Pixels) -> WindowHandle<Root>
pub fn open_modern_window<V>(cx: &mut App, title: ..., width: Pixels, height: Pixels) -> WindowHandle<Root>
```

但 `crates/app/src/window.rs` 已根据用户反馈 2 **实际重写为 struct-based API**：

```rust
// 实际实现（已落地）
pub struct Window { title, width, height, chrome: WindowChrome }
pub struct ModernWindow(Window);

impl Window {
    pub fn new(title, width, height) -> Self
    pub fn into_modern(self) -> ModernWindow
    pub fn open<V>(self, cx: &mut App) -> WindowHandle<...> where V: IRmlView + Render + Default
}
impl ModernWindow {
    pub fn new(title, width, height) -> Self
    pub fn open<V>(self, cx: &mut App) -> WindowHandle<...>
}
```

**偏差导致的问题**：
- `crates/app/src/lib.rs:28` 仍导出 `open_modern_window` / `open_window`（已不存在的符号）→ **编译失败**
- `demo/src/main.rs` 调用 `.title()` / `.size()` / `run::<todos::Todos>()` → **三重不兼容，编译失败**
- `application.rs` / `lifecycle.rs` 文档注释引用旧函数名 → 文档与实现不符

---

## 二、概念关系澄清 Concept Clarification

根据用户反馈 1，明确 `Window` 与 `ModernWindow` 的双层概念关系：

### 层 1：窗口对象（crates/app）—— WPF 风格窗口管理

| 类型 | crate | 职责 | API |
|------|-------|------|-----|
| `Window` | `rml_app` | 原生标题栏窗口对象 | `Window::new(title, w, h).open::<V>(cx)` |
| `ModernWindow` | `rml_app` | 透明标题栏窗口对象（`appears_transparent=true`） | `ModernWindow::new(title, w, h).open::<V>(cx)` |

**语义**：窗口对象负责 `WindowOptions` 配置（标题栏样式、尺寸、位置）+ 打开窗口 + 用 `Root` 包裹业务 view。类比 WPF `new Window().Show()`。

### 层 2：RML 组件（crates/ui）—— 视觉外壳封装

| 类型 | crate | 职责 | 用法 |
|------|-------|------|------|
| `TitleBar` / `StatusBar` / `Kbd` | `rml_ui` | 原子 chrome 组件 | `.rml` 中手动组装 |
| `ModernWindow` | `rml_ui` | 内置封装（TitleBar+Menu+StatusBar+Notification） | `.rml` 中作为根标签 `<ModernWindow>` |

**语义**：RML 组件负责窗口内部视觉布局。用户有**两种选择**：
- **易用性封装**：`<ModernWindow title="..." menu={...} status_bar={...}>` —— 代码少，现代视觉
- **手动组装**：`<div><TitleBar>...</TitleBar><Menu>...</Menu>...<StatusBar>...</StatusBar></div>` —— 灵活定制

**同名不冲突**：`rml_app::ModernWindow`（窗口对象）与 `rml_ui::ModernWindow`（RML 组件）用途不同，类比 WPF `Window` 类与 `<Window>` XAML 元素同名。用户在 Rust 代码中用窗口对象，在 `.rml` 中用 RML 组件标签。

---

## 三、当前状态分析 Current State

### 已完成（代码已落地）

| 文件 | 状态 | 关键 API |
|------|------|----------|
| `crates/app/src/lifecycle.rs` | ✅ 完成 | `IAppLifecycle` trait（`on_launch`/`on_exit`/`on_activate`/`on_deactivate`） |
| `crates/app/src/application.rs` | ✅ 完成 | `RmlApplication::run::<A: IAppLifecycle + Default>` |
| `crates/app/src/window.rs` | ✅ 完成（struct-based） | `Window` / `ModernWindow` + `.open::<V>(cx)` |

### 不一致（需修复）

| # | 位置 | 问题 | 影响 |
|---|------|------|------|
| 1 | `crates/app/src/lib.rs:28` | `pub use window::{open_modern_window, open_window}` 导出已不存在的符号 | 编译失败 |
| 2 | `crates/app/src/application.rs` 文档注释 | 引用 `open_modern_window::<MyView>(...)` 旧函数名 | 文档过时 |
| 3 | `crates/app/src/lifecycle.rs` 文档注释 | 引用 `open_modern_window::<MyView>(...)` 旧函数名 | 文档过时 |
| 4 | `demo/src/main.rs` | 调用 `.title()` / `.size()`（不存在）+ `run::<todos::Todos>()`（Todos 不满足 IAppLifecycle） | 编译失败 |

### Phase 1 剩余工作（未开始）

| # | 范围 | 说明 |
|---|------|------|
| 5 | `crates/ui/src/window/` 目录 | 5 个新文件：`mod.rs` / `modern_window.rs` / `types.rs` / `menu_bar.rs` / `actions.rs` |
| 6 | `crates/ui/src/lib.rs` + `prelude.rs` | re-export ModernWindow/MenuItem/StatusBarItem/IWindowActions/TitleBar/StatusBar/Kbd |
| 7 | `crates/ui/Cargo.toml` | 添加 `smallvec` 依赖 |
| 8 | `crates/engine/src/tags.rs` | `component_lookup` 新增 TitleBar/StatusBar/Kbd/ModernWindow 路由 |
| 9 | `crates/engine/src/compiler/component.rs` | ModernWindow 专用 setter（`menu`/`status_bar`/`title`） |
| 10 | `demo/src/main.rs` + `counter.rml` + `counter.rml.rs` | 迁移到 IAppLifecycle + ModernWindow 示例 |

---

## 四、实施步骤 Implementation Steps

### Step 1：修复 `crates/app/src/lib.rs` 导出

**文件**：`crates/app/src/lib.rs`

**变更**：将第 28 行的悬空导出更新为 struct 导出。

```rust
// 旧（编译失败）
pub use window::{open_modern_window, open_window};

// 新
pub use window::{Window, ModernWindow};
```

**验证**：`cargo build -p rust-rml-app` 通过（此 crate 内部编译）。

### Step 2：修复 `crates/app/src/application.rs` 文档注释

**文件**：`crates/app/src/application.rs`

**变更**：文档注释中的示例从旧函数调用改为新 struct API。

```rust
// 旧文档示例
// rml_app::open_modern_window::<MyView>(cx, "My App", px(800.), px(600.));

// 新文档示例
// rml_app::ModernWindow::new("My App", px(800.), px(600.)).open::<MyView>(cx);
```

### Step 3：修复 `crates/app/src/lifecycle.rs` 文档注释

**文件**：`crates/app/src/lifecycle.rs`

**变更**：同 Step 2，更新 `on_launch` 方法文档中的示例引用。

### Step 4：创建 `crates/ui/src/window/` 模块

按原计划 §3.4-3.8 执行，创建 5 个文件。

#### 4.1 `crates/ui/src/window/types.rs`

纯数据结构，闭包捕获 `WeakEntity<T>`。按原计划 §3.5 实现：

- `MenuItem`：`label` / `on_click: Option<Box<dyn Fn(&mut Window, &mut App)>>` / `disabled` / `checked` / `children: SmallVec<[MenuItem; 4]>` / `separator`
  - `MenuItem::new(label)` / `::separator()` / `.on_click(f)` / `.on_click_with(cx, f)` / `.disabled(b)` / `.checked(b)` / `.submenu(vec)`
- `StatusBarItem`：`label` / `on_click` / `icon: Option<SharedString>`
  - `StatusBarItem::new(label)` / `.on_click_with(cx, f)`

#### 4.2 `crates/ui/src/window/actions.rs`

`IWindowActions` trait + `NotificationKind` enum，为 `&mut Window` 提供便捷通知 API。按原计划 §3.7 实现。

#### 4.3 `crates/ui/src/window/menu_bar.rs`

`render_menu_bar(&[MenuItem]) -> impl IntoElement`，水平菜单栏渲染器。按原计划 §3.6 实现，Phase 1 先支持顶层点击 + 下拉列表。

#### 4.4 `crates/ui/src/window/modern_window.rs`

`ModernWindow` RenderOnce 组件，组合 TitleBar + Menu + StatusBar。按原计划 §3.4 实现：

- 字段：`id` / `title: Option<SharedString>` / `menu: Option<Vec<MenuItem>>` / `status_bar: Option<Vec<StatusBarItem>>` / `children: SmallVec<[AnyElement; 4]>`
- Builder：`.title()` / `.menu()` / `.status_bar()`
- 实现 `ParentElement` + `RenderOnce`
- `RenderOnce::render`：`div().flex().flex_col().size_full()` → TitleBar（含可选 Menu）→ children → 可选 StatusBar

#### 4.5 `crates/ui/src/window/mod.rs`

```rust
pub mod actions;
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use modern_window::ModernWindow;
pub use types::{MenuItem, StatusBarItem};
```

### Step 5：集成 `crates/ui/src/lib.rs` + `prelude.rs`

**文件**：`crates/ui/src/lib.rs`

新增：
```rust
pub mod window;
pub use window::{ModernWindow, MenuItem, StatusBarItem, IWindowActions, NotificationKind};
pub use gpui_component::{TitleBar, StatusBar, Kbd};
```

**文件**：`crates/ui/src/prelude.rs`

新增对应 re-export。

### Step 6：添加 `smallvec` 依赖

**文件**：`crates/ui/Cargo.toml`

```toml
[dependencies]
smallvec = "1"
```

### Step 7：扩展 `crates/engine/src/tags.rs` 路由

**文件**：`crates/engine/src/tags.rs`

在 `component_lookup` 中新增 4 个路由（按原计划 §3.9）：

```rust
"TitleBar" => Stateless, "rml_ui::TitleBar"
"StatusBar" => Stateless, "rml_ui::StatusBar"
"Kbd" => Stateless, "rml_ui::Kbd"
"ModernWindow" => Stateless, "rml_ui::ModernWindow"
```

### Step 8：扩展 `crates/engine/src/compiler/component.rs` setter

**文件**：`crates/engine/src/compiler/component.rs`

在 `component_bind_setter` 中为 ModernWindow 添加专用 setter（按原计划 §3.10）：

```rust
// ModernWindow 专用 setter（tag == "ModernWindow"）
"menu" => Some(format!(".menu({})", value_expr)),
"status_bar" => Some(format!(".status_bar({})", value_expr)),
```

`title` 属性复用现有 string setter 逻辑。

### Step 9：迁移 `demo/src/main.rs`

**文件**：`demo/src/main.rs`

**变更**：从旧的 `.title().size().run::<Todos>()` 模式迁移到 `IAppLifecycle + ModernWindow::new().open()` 模式。

```rust
use rml_app::{IAppLifecycle, RmlApplication, ModernWindow};
use gpui::{App, px};

fn main() {
    RmlApplication::new().run::<DemoApp>();
}

#[derive(Default)]
struct DemoApp;

impl IAppLifecycle for DemoApp {
    fn on_launch(&mut self, cx: &mut App) {
        ModernWindow::new("RML Counter Demo", px(400.), px(500.))
            .open::<counter::Counter>(cx);
    }
}
```

**注**：主窗口展示 Counter（作为 ModernWindow 示例）。Todos 视图保留但暂不展示（或作为 Phase 4 多窗口示例）。

### Step 10：迁移 `demo/src/counter.rml` + `counter.rml.rs`

**文件**：`demo/src/counter.rml`

根元素改为 `<ModernWindow>`：

```html
<ModernWindow title="RML 计数器" menu={menu_items} status_bar={status_items}>
    <div class="counter">
        <h1 ref="title">计数器</h1>
        <p class="count">{count}</p>
        <!-- 原有 Button/ref/once/converter 内容保留 -->
    </div>
</ModernWindow>
```

**文件**：`demo/src/counter.rml.rs`

ViewModel 新增 `menu_items: Vec<MenuItem>` / `status_items: Vec<StatusBarItem>` 字段 + `build_menu` 方法（按原计划 §3.11）。

### Step 11：验证

1. `cargo build -p rust-rml-app` — app crate 编译通过
2. `cargo build -p rust-rml-ui` — ui crate 编译通过（ModernWindow 组件 + 数据类型）
3. `cargo build -p rust-rml-engine` — engine crate 编译通过（tags.rs 路由 + component.rs setter）
4. `cargo test -p rust-rml-engine` — 现有 180+ 测试不回归
5. `cargo build -p rust-rml-demo` — demo 编译通过
6. `cargo run -p rust-rml-demo` — demo 启动，显示 ModernWindow（标题栏 + 菜单 + 状态栏 + 业务内容）

---

## 五、验证步骤 Verification

### 编译验证
```bash
cargo build --workspace
```
预期：全 workspace 编译通过，无悬空符号错误。

### 单元测试验证
```bash
cargo test -p rust-rml-engine
```
预期：180+ 现有测试通过 + 新增 ModernWindow setter 测试（如有）通过。

### 运行验证
```bash
cargo run -p rust-rml-demo
```
预期：
- 窗口打开，标题栏显示 "RML 计数器"
- 菜单栏显示 "文件" / "帮助" 顶层项
- 状态栏显示 "就绪" / "计数: 0"
- 业务区显示计数器内容
- 点击菜单项 → 触发对应命令（如"重置"）
- 命令执行后状态栏更新

### 文档一致性验证
- `crates/app/src/lib.rs` 导出 `Window` / `ModernWindow`（不再有 `open_window` / `open_modern_window`）
- `application.rs` / `lifecycle.rs` 文档示例使用新 API

---

## 六、假设与决策 Assumptions & Decisions

### 假设
1. `window.rs` struct-based API 是最终设计，不再回退到自由函数 helper
2. `rml_app::ModernWindow`（窗口对象）与 `rml_ui::ModernWindow`（RML 组件）同名不冲突，因为用途不同（Rust 代码 vs .rml 标签）
3. `#[view]` 宏生成的视图（Counter/Todos）自动实现 `IRmlView + Render + Default`，满足 `.open::<V>()` 约束
4. `gpui-component` 的 `TitleBar` / `StatusBar` / `Kbd` API 稳定，可直接 re-export

### 决策
1. **窗口对象 API**：`Window::new(title, w, h).open::<V>(cx)` —— WPF 风格，响应用户反馈 2
2. **ModernWindow 双层概念**：
   - `rml_app::ModernWindow` = 窗口对象（透明标题栏配置 + open）
   - `rml_ui::ModernWindow` = RML 组件（TitleBar+Menu+StatusBar 封装）
   - 用户可选：`<ModernWindow>` 封装 或 `<TitleBar>`+`<StatusBar>` 手动组装 —— 响应用户反馈 1
3. **demo 迁移**：主窗口展示 Counter（ModernWindow 示例），Todos 保留但暂不展示
4. **不锁定 gpui-component rev**：保持现状（git 依赖无 rev），避免引入额外变更。若后续发现漂移问题，再单独处理

---

## 七、文件变更清单 File Change List

### 修改（4 文件）
| 文件 | 变更 |
|------|------|
| `crates/app/src/lib.rs` | 导出从 `open_modern_window, open_window` 改为 `Window, ModernWindow` |
| `crates/app/src/application.rs` | 文档注释更新为新 API |
| `crates/app/src/lifecycle.rs` | 文档注释更新为新 API |
| `crates/ui/Cargo.toml` | 添加 `smallvec` 依赖 |

### 新建（5 文件）
| 文件 | 职责 |
|------|------|
| `crates/ui/src/window/mod.rs` | window 模块根 + re-export |
| `crates/ui/src/window/modern_window.rs` | `ModernWindow` RenderOnce 组件 |
| `crates/ui/src/window/types.rs` | `MenuItem` / `StatusBarItem` 数据类型 |
| `crates/ui/src/window/menu_bar.rs` | 菜单栏渲染器 |
| `crates/ui/src/window/actions.rs` | `IWindowActions` trait + `NotificationKind` |

### 修改（集成）
| 文件 | 变更 |
|------|------|
| `crates/ui/src/lib.rs` | `pub mod window;` + re-export ModernWindow/MenuItem/StatusBarItem/IWindowActions/TitleBar/StatusBar/Kbd |
| `crates/ui/src/prelude.rs` | 对应 re-export |
| `crates/engine/src/tags.rs` | `component_lookup` 新增 4 个路由 |
| `crates/engine/src/compiler/component.rs` | ModernWindow 专用 setter（`menu`/`status_bar`） |

### 迁移（demo）
| 文件 | 变更 |
|------|------|
| `demo/src/main.rs` | 迁移到 `IAppLifecycle + ModernWindow::new().open()` |
| `demo/src/counter.rml` | 根元素改为 `<ModernWindow>` + menu/status_bar 绑定 |
| `demo/src/counter.rml.rs` | 新增 menu_items/status_items 字段 + build_menu 方法 |

---

## 八、执行顺序 Execution Order

1. **Step 1-3**：修复 app crate（lib.rs 导出 + 文档注释）→ `cargo build -p rust-rml-app` 通过
2. **Step 4-6**：创建 crates/ui/src/window/ 模块 + 集成 lib.rs/prelude + smallvec 依赖 → `cargo build -p rust-rml-ui` 通过
3. **Step 7-8**：扩展 tags.rs 路由 + component.rs setter → `cargo build -p rust-rml-engine` + `cargo test -p rust-rml-engine` 通过
4. **Step 9-10**：迁移 demo → `cargo build -p rust-rml-demo` 通过
5. **Step 11**：全量验证 → `cargo build --workspace` + `cargo run -p rust-rml-demo`

每个步骤完成后立即验证编译，避免错误累积。

---

## 九、与原计划的关系 Relationship to Original Plan

本计划是对 `window-and-component-integration-plan.md` 的**修订与收尾**，仅修改：

- **§3.3 窗口打开 helper**：从自由函数 `open_window::<V>(cx, ...)` 修订为 struct API `Window::new(...).open::<V>(cx)`
- **§3.11 demo 迁移**：从 `open_modern_window::<Counter>(cx, ...)` 修订为 `ModernWindow::new(...).open::<Counter>(cx)`
- **新增**：明确 Window/ModernWindow 双层概念关系（窗口对象 vs RML 组件）

原计划的以下部分**保持不变**：
- Phase 1 的 ModernWindow RML 组件设计（§3.4）
- 数据类型定义（§3.5）
- Menu Bar 渲染器（§3.6）
- IWindowActions trait（§3.7）
- 标签路由表扩展（§3.9）
- 组件 setter 扩展（§3.10）
- Phase 2（快捷键 + Kbd）、Phase 3（40+ 组件）、Phase 4（高级组件 + 多窗口）的全部设计

Phase 2-4 将在本计划完成后按原计划继续执行。
