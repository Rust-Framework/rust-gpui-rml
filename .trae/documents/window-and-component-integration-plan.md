# Window / ModernWindow / 组件完整集成 实施计划

> 本计划基于代码库实际探索编写，所有文件路径与 API 均来自真实代码。
> 命名遵循用户要求：**除非必须，不加 `Rml` 前缀**，避免过度产品化污染。

---

## 一、总览 Summary

### 目标
1. **`IAppLifecycle` 接口**：`RmlApplication::run::<A>()` 由 `A: IAppLifecycle` 控制窗口创建，不再直接绑定视图类型。
2. **Window（基础组件）**：暴露 `<TitleBar>` / `<StatusBar>` / `<Menu>` / `<Kbd>` 等原子标签到 RML，用户可在 `.rml` 中自行组装窗口外观。
3. **ModernWindow（封装组件）**：内置封装 TitleBar + Menu + StatusBar + Notification 的 `RenderOnce` 组件，用户通过 MVVM 数据绑定配置，无需编写基本布局。
4. **完整组件集成**：扩展 `component_lookup` 路由表至全部 gpui-component 组件，按组件提供属性映射，`crates/ui` 不再是纯 re-export。
5. **快捷键方案**：`<bind key="ctrl+s" onclick={save} />` RML 声明 → codegen 生成 Action 定义 + handler + bind_keys。

### 关键决策（无 Rml 前缀）
- 类型名：`ModernWindow` / `MenuItem` / `StatusBarItem` / `Menu` / `Kbd` / `IAppLifecycle` / `IWindowActions`
- Action 类型：`Save` / `Quit` / `OpenFile`（不带前缀，由 codegen 在独立模块生成）
- 保留 `RmlApplication`（应用启动器，类比 `Application`，不算"产品化污染"）

### 分阶段交付
| Phase | 内容 | 文件变更数 |
|-------|------|-----------|
| **Phase 1** | `IAppLifecycle` + Window 助手 + ModernWindow 内置封装 + 数据类型 + chrome 组件标签 | ~10 新建 + ~6 修改 |
| **Phase 2** | `<bind>` 快捷键 codegen + `<Kbd>` 显示组件 + Action 注册机制 | ~3 新建 + ~3 修改 |
| **Phase 3** | 扩展 `component_lookup` 至 40+ 组件 + 每组件属性映射 | ~2 修改 + ~1 新建 |
| **Phase 4** | List/Table/Dialog/Select 等有状态组件集成 + 多窗口 + 主题 | ~3 修改 + ~1 新建 |

---

## 二、当前状态分析 Current State Analysis

### 已有基础（来自代码探索）

**gpui-component v0.5.2**（git 依赖，`Cargo.toml:15`）提供：
- `TitleBar` — stateless `RenderOnce`，构造 `TitleBar::new()`，`TITLE_BAR_HEIGHT = px(34.)`，trait: `Styled + ParentElement`
- `StatusBar` — stateless `RenderOnce`，构造 `StatusBar::new()`，trait: `Styled + ParentElement`
- `Kbd` — stateless `RenderOnce`，构造 `Kbd::new(stroke: Keystroke)`，trait: `Styled`
- `Notification` — stateful entity，构造 `Notification::info/success/warning/error(msg)`
- `NotificationList` — stateful entity，构造 `NotificationList::new(window, cx)`
- `PopupMenu` — stateful entity，构造 `PopupMenu::build(window, cx, |m, win, cx| m.menu("Save", Box::new(Save)))`
- `WindowExt` trait — `window.push_notification(note, cx)` 入口
- `Root` — 包裹业务 view，提供 Dialog/Sheet/Notification 浮层支持
- 50+ 其他组件（Button/Input/List/Table/Dialog/Popover/Tooltip 等）

**crates/ui**（`crates/ui/src/lib.rs`）：
- 纯 re-export 门面，**零本地类型**
- 仅定义 `pub fn init(cx: &mut gpui::App)`
- re-export 24 个组件 + 6 个 trait

**crates/engine/src/tags.rs**：
- `BuiltinTag` 18 个原生标签
- `component_lookup` 仅 13 个 PascalCase 标签（11 Stateless + 2 Stateful）
- **缺失**：TitleBar/StatusBar/Kbd/ModernWindow/Avatar/Icon/Tooltip/Popover/Dialog/List/Table/Tab/Select 等

**crates/app/src/application.rs**：
- `RmlApplication::new().run::<R>()` where `R: IRmlView + Render + Default`
- 直接绑定视图类型，**无应用级生命周期管理**
- 窗口选项仅 `title` + `size` + 默认 `TitlebarOptions`

**crates/core/src/lifecycle.rs**：
- `ILifecycle` trait — 视图级生命周期（`rml_on_loaded` / `rml_on_unloaded`）
- 方法签名：`fn rml_on_loaded(&mut self, _cx: &mut Context<Self>)`
- **不接收 `&mut Window`** — 无法在生命周期回调中注册快捷键

**docs/ 目录**：
- 67 个文档文件，**零文档**描述窗口外壳 / 标题栏 / 状态栏 / 菜单栏 / 快捷键的产品形态
- 唯一相关：`docs/06-components/builtin-components.md:92` 列出 `<Notification>` 组件标签

---

## 三、Phase 1：IAppLifecycle + ModernWindow 内置封装

### 3.1 `IAppLifecycle` trait（新建）

**文件**：`crates/app/src/lifecycle.rs`（新建）

**职责**：应用级生命周期契约，类比 WPF `Application.OnStartup/OnExit`。

**设计**：
```rust
use gpui::App;

/// 应用级生命周期 trait
///
/// `RmlApplication::run::<A>()` 中 `A` 必须实现此 trait。
/// 类比 WPF 的 `Application` 类，由 App 负责打开主窗口，而非直接绑定视图类型。
pub trait IAppLifecycle: Sized + Send + 'static {
    /// 应用启动时调用（仅一次）
    ///
    /// 典型用途：打开主窗口、初始化全局状态、注册 app 级 Action。
    /// 在此处调用 `rml_app::open_window::<MyView>(cx, "My App", px(800.), px(600.))`。
    fn on_launch(&mut self, cx: &mut App);

    /// 应用退出前调用（仅一次）
    fn on_exit(&mut self, cx: &mut App) {}

    /// 应用被激活（前台）时调用
    fn on_activate(&mut self, cx: &mut App) {}

    /// 应用被停用（后台）时调用
    fn on_deactivate(&mut self, cx: &mut App) {}
}
```

**关联修改**：
- `crates/app/src/lib.rs`：添加 `pub mod lifecycle; pub use lifecycle::IAppLifecycle;`
- `crates/core/src/prelude.rs`：可选 re-export `IAppLifecycle`（但 app crate 不在 core 依赖中，保留在 `rml_app::prelude`）

### 3.2 `RmlApplication` 重构

**文件**：`crates/app/src/application.rs`（修改）

**变更**：`run` 方法签名从 `run::<R: IRmlView + Render + Default>` 改为 `run::<A: IAppLifecycle + Default>`。

**新实现**：
```rust
pub fn run<A>(self)
where
    A: IAppLifecycle + Default + 'static,
{
    let title = self.title;
    let size = Size { width: self.width, height: self.height };
    let _ = (title, size); // 保留兼容字段，但窗口创建权交给 App

    gpui_platform::application().run(move |cx: &mut App| {
        #[cfg(feature = "ui-components")]
        rml_ui::init(cx);

        let mut app = A::default();
        app.on_launch(cx);

        // 注册退出回调（GPUI 应用退出时触发）
        // 注：GPUI 当前没有显式的 on_exit 钩子，可在 on_release 中近似处理
        // 留作 Phase 4 完善，Phase 1 仅保证 on_launch 工作
    });
}
```

**向后兼容策略**：删除 `run::<R: IRmlView>` 签名（用户请求明确指出"不允许直接应用视图"）。现有 demo 需迁移到 `IAppLifecycle` 模式。

### 3.3 窗口打开 helper

**文件**：`crates/app/src/window.rs`（重写 stub）

**职责**：提供 `open_window::<V>()` 助手，封装 `WindowOptions` 构造 + `Root` 包裹。

```rust
use gpui::*;
use rml_core::view::IRmlView;

/// 打开一个窗口，以 `V` 为根视图
///
/// 自动构造 `WindowOptions`（title + size + 默认 titlebar），
/// 并在 feature `ui-components` 启用时用 `rml_ui::Root` 包裹，
/// 从而支持 Dialog/Sheet/Notification 浮层。
pub fn open_window<V>(
    cx: &mut App,
    title: impl Into<SharedString>,
    width: Pixels,
    height: Pixels,
) -> WindowHandle<Root>
where
    V: IRmlView + Render + Default + 'static,
{
    #[cfg(feature = "ui-components")]
    {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Default::default(),
                size: Size { width, height },
            })),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            ..Default::default()
        };
        cx.open_window(options, |window, cx| {
            let view = cx.new(|_cx| V::default());
            cx.new(|cx| rml_ui::Root::new(view, window, cx))
        })
        .expect("failed to open window")
    }

    #[cfg(not(feature = "ui-components"))]
    {
        // 退化路径：直接构造业务 view
        let options = WindowOptions { /* ... */ };
        cx.open_window(options, |_window, cx| {
            cx.new(|_cx| V::default())
        })
        .expect("failed to open window")
    }
}

/// 打开 ModernWindow 风格的窗口（titlebar 透明，由 ModernWindow 自绘标题栏）
///
/// 与 `open_window` 的区别：`TitlebarOptions.appears_transparent = true`，
/// 让 `TitleBar` 组件完全接管标题栏绘制。
pub fn open_modern_window<V>(
    cx: &mut App,
    title: impl Into<SharedString>,
    width: Pixels,
    height: Pixels,
) -> WindowHandle<Root>
where
    V: IRmlView + Render + Default + 'static,
{
    // 类似 open_window，但 titlebar appears_transparent = true
    // 并设置 traffic_light_position 为 TitleBar::title_bar_options() 的值
    // ...
}
```

### 3.4 ModernWindow 内置封装组件

**文件**：`crates/ui/src/window/modern_window.rs`（新建）

**设计要点**：
- `RenderOnce` 组件，内部组合 `TitleBar` + `Menu` + `StatusBar`
- 用户通过 builder 方法绑定数据：`.title()` / `.menu()` / `.status_bar()` / `.child()`
- 不需要在 `.rml` 中编写 `<TitleBar><Menu>...</Menu></TitleBar>` 布局
- 支持 `ParentElement`，用户的内容作为主区域子节点

**代码骨架**：
```rust
use gpui::*;
use gpui_component::{TitleBar, StatusBar, ParentElement as _, Styled as _, TITLE_BAR_HEIGHT};

use super::types::{MenuItem, StatusBarItem};
use super::menu_bar::render_menu_bar;

#[derive(IntoElement)]
pub struct ModernWindow {
    id: ElementId,
    title: Option<SharedString>,
    menu: Option<Vec<MenuItem>>,
    status_bar: Option<Vec<StatusBarItem>>,
    children: SmallVec<[AnyElement; 4]>,
}

impl ModernWindow {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            title: None,
            menu: None,
            status_bar: None,
            children: SmallVec::new(),
        }
    }

    /// 绑定标题栏内容（MVVM 数据绑定入口）
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// 绑定菜单数据（MVVM 数据绑定入口）
    /// ViewModel 持有 `Vec<MenuItem>` 字段，在 RML 中 `menu={self.menu_items}`
    pub fn menu(mut self, menu: Vec<MenuItem>) -> Self {
        self.menu = Some(menu);
        self
    }

    /// 绑定状态栏数据（MVVM 数据绑定入口）
    pub fn status_bar(mut self, items: Vec<StatusBarItem>) -> Self {
        self.status_bar = Some(items);
        self
    }
}

impl ParentElement for ModernWindow {
    fn extend(&mut self, child: AnyElement) {
        self.children.push(child);
    }
}

impl RenderOnce for ModernWindow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .flex()
            .flex_col()
            .size_full()
            // 顶部：TitleBar + 可选 Menu
            .child(
                TitleBar::new()
                    .when_some(self.title, |this, title| this.child(title))
                    .when_some(self.menu, |this, menu| {
                        this.child(render_menu_bar(&menu))
                    })
            )
            // 主内容区：用户的子节点
            .children(self.children)
            // 底部：可选 StatusBar
            .when_some(self.status_bar, |this, items| {
                this.child(render_status_bar(&items))
            })
    }
}

fn render_status_bar(items: &[StatusBarItem]) -> impl IntoElement {
    StatusBar::new().children(items.iter().map(|item| {
        div().child(item.label.clone())
            // .when_some(item.icon, |this, icon| this.child(Icon::new(icon)))
    }))
}
```

### 3.5 数据类型定义

**文件**：`crates/ui/src/window/types.rs`（新建）

**设计要点**：
- 纯数据结构，不含 GPUI 依赖（除了 `SharedString`）
- 闭包用 `Box<dyn Fn(&mut Window, &mut App) + 'static>`，匹配 GPUI 事件闭包模式
- 提供 `on_click_with` 助手方法，接收 `cx.listener` 风格的闭包

```rust
use gpui::{App, SharedString, Window};
use smallvec::SmallVec;

/// 菜单项数据（用于 MVVM 绑定）
#[derive(Clone)]
pub struct MenuItem {
    pub label: SharedString,
    pub on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub disabled: bool,
    pub checked: bool,
    pub children: SmallVec<[MenuItem; 4]>,  // 子菜单
    pub separator: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            disabled: false,
            checked: false,
            children: SmallVec::new(),
            separator: false,
        }
    }

    pub fn separator() -> Self {
        let mut item = Self::new("");
        item.separator = true;
        item
    }

    pub fn on_click(mut self, f: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    /// 助手：类似 `cx.listener` 的闭包绑定
    /// 用法：`MenuItem::new("Save").on_click_with(cx, |this, window, cx| this.save(window, cx))`
    pub fn on_click_with<T, F>(self, cx: &gpui::Context<T>, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T, &mut Window, &mut gpui::App) + 'static,
    {
        let weak = cx.weak_entity();
        self.on_click(Box::new(move |window, cx| {
            if let Some(this) = weak.upgrade() {
                this.update(cx, |this, cx| f(this, window, cx));
            }
        }))
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn submenu(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children.into();
        self
    }
}

/// 状态栏项数据
#[derive(Clone)]
pub struct StatusBarItem {
    pub label: SharedString,
    pub on_click: Option<Box<dyn Fn(&mut Window, &mut App) + 'static>>,
    pub icon: Option<SharedString>,  // 暂存图标名，后续映射到 IconName
}

impl StatusBarItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            icon: None,
        }
    }

    pub fn on_click_with<T, F>(self, cx: &gpui::Context<T>, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T, &mut Window, &mut gpui::App) + 'static,
    {
        let weak = cx.weak_entity();
        StatusBarItem {
            on_click: Some(Box::new(move |window, cx| {
                if let Some(this) = weak.upgrade() {
                    this.update(cx, |this, cx| f(this, window, cx));
                }
            })),
            ..self
        }
    }
}
```

### 3.6 Menu Bar 渲染器

**文件**：`crates/ui/src/window/menu_bar.rs`（新建）

**职责**：将 `Vec<MenuItem>` 数据渲染为 `PopupMenu` 风格的水平菜单栏。

**实现思路**：
- 顶层菜单项横向排列（File / Edit / View / Help）
- 点击展开下拉 `PopupMenu`
- 子菜单递归渲染

```rust
use gpui::*;
use gpui_component::{PopupMenu, PopupMenuItem};

use super::types::MenuItem;

/// 渲染水平菜单栏
pub fn render_menu_bar(items: &[MenuItem]) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .children(items.iter().enumerate().map(|(i, item)| {
            render_top_menu_item(i, item)
        }))
}

fn render_top_menu_item(idx: usize, item: &MenuItem) -> impl IntoElement {
    // 顶层项：点击展开 PopupMenu
    // 使用 button + dropdown_menu 或直接构造 PopupMenu
    // ...
}
```

**注**：Phase 1 先实现基础菜单（顶层点击 + 下拉列表），子菜单与图标留 Phase 4。

### 3.7 IWindowActions 助手 trait

**文件**：`crates/ui/src/window/actions.rs`（新建）

**职责**：提供 `show_notification` / `show_dialog` 等便捷方法，供 ViewModel 调用。

```rust
use gpui::{App, Window, SharedString};
use gpui_component::{Notification, NotificationType, WindowExt};

/// 窗口操作助手 trait
///
/// 为 `&mut Window` 提供便捷的消息通知 / 对话框 API。
/// 自动通过 `WindowExt` 路由到 `Root` 管理的 `NotificationList`。
pub trait IWindowActions {
    /// 显示一条通知（右下角，类似 VSCode）
    fn show_notification(&mut self, message: impl Into<SharedString>, kind: NotificationKind, cx: &mut App);

    /// 显示信息通知
    fn notify_info(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Info, cx);
    }

    /// 显示成功通知
    fn notify_success(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Success, cx);
    }

    /// 显示错误通知
    fn notify_error(&mut self, message: impl Into<SharedString>, cx: &mut App) {
        self.show_notification(message, NotificationKind::Error, cx);
    }
}

#[derive(Clone, Copy)]
pub enum NotificationKind {
    Info,
    Success,
    Warning,
    Error,
}

impl IWindowActions for Window {
    fn show_notification(&mut self, message: impl Into<SharedString>, kind: NotificationKind, cx: &mut App) {
        let note = match kind {
            NotificationKind::Info => Notification::info(message),
            NotificationKind::Success => Notification::success(message),
            NotificationKind::Warning => Notification::warning(message),
            NotificationKind::Error => Notification::error(message),
        };
        self.push_notification(note, cx);
    }
}
```

### 3.8 crates/ui 模块整合

**文件**：`crates/ui/src/window/mod.rs`（新建）

```rust
pub mod actions;
pub mod menu_bar;
pub mod modern_window;
pub mod types;

pub use actions::{IWindowActions, NotificationKind};
pub use modern_window::ModernWindow;
pub use types::{MenuItem, StatusBarItem};
```

**文件**：`crates/ui/src/lib.rs`（修改）

在现有 re-export 基础上添加：
```rust
pub mod window;
pub use window::{ModernWindow, MenuItem, StatusBarItem, IWindowActions, NotificationKind};

// 新增 re-export（用于 RML 标签映射）
pub use gpui_component::{TitleBar, StatusBar, Kbd};
```

**文件**：`crates/ui/src/prelude.rs`（修改）

添加：
```rust
pub use crate::window::{ModernWindow, MenuItem, StatusBarItem, IWindowActions, NotificationKind};
pub use crate::{TitleBar, StatusBar, Kbd};
```

### 3.9 标签路由表扩展

**文件**：`crates/engine/src/tags.rs`（修改）

在 `component_lookup` 中新增：
```rust
"TitleBar" => Some(ComponentTag {
    ctor_path: "rml_ui::TitleBar",
    kind: ComponentKind::Stateless,
}),
"StatusBar" => Some(ComponentTag {
    ctor_path: "rml_ui::StatusBar",
    kind: ComponentKind::Stateless,
}),
"Kbd" => Some(ComponentTag {
    ctor_path: "rml_ui::Kbd",
    kind: ComponentKind::Stateless,
}),
"ModernWindow" => Some(ComponentTag {
    ctor_path: "rml_ui::ModernWindow",
    kind: ComponentKind::Stateless,
}),
```

### 3.10 组件属性 setter 扩展

**文件**：`crates/engine/src/compiler/component.rs`（修改）

在 `component_static_setter` / `component_bind_setter` 中添加 ModernWindow 专用 setter：

```rust
// ModernWindow 专用 setter
"menu" => Some(format!(".menu({})", value_expr)),  // bind: menu={self.menu_items}
"status_bar" => Some(format!(".status_bar({})", value_expr)),  // bind: status_bar={self.status_items}
```

对于 `title` 属性，复用现有的 string setter 逻辑。

### 3.11 Demo 迁移

**文件**：`demo/src/main.rs`（修改）

```rust
fn main() {
    RmlApplication::new()
        .run::<MyApp>();
}

struct MyApp;

impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut App) {
        rml_app::open_modern_window::<Counter>(cx, "RML Counter Demo", px(400.), px(500.));
    }
}
```

**文件**：`demo/src/counter.rml.rs`（修改）

在 ViewModel 中添加菜单与状态栏数据：
```rust
pub struct Counter {
    count: i32,
    hovered: bool,
    menu_items: Vec<MenuItem>,
    status_items: Vec<StatusBarItem>,
}

impl Default for Counter {
    fn default() -> Self {
        Self {
            count: 0,
            hovered: false,
            menu_items: Vec::new(),
            status_items: Vec::new(),
        }
    }
}

impl Counter {
    fn build_menu(&mut self, cx: &mut Context<Self>) {
        self.menu_items = vec![
            MenuItem::new("文件")
                .submenu(vec![
                    MenuItem::new("重置").on_click_with(cx, |this, _w, cx| this.reset(cx)),
                    MenuItem::separator(),
                    MenuItem::new("退出").on_click_with(cx, |this, _w, cx| this.quit(cx)),
                ]),
            MenuItem::new("帮助")
                .submenu(vec![
                    MenuItem::new("关于").on_click_with(cx, |this, _w, cx| this.about(cx)),
                ]),
        ];
        self.status_items = vec![
            StatusBarItem::new("就绪"),
            StatusBarItem::new(format!("计数: {}", self.count)),
        ];
    }
}
```

**文件**：`demo/src/counter.rml`（修改）

```html
<ModernWindow title="RML 计数器" menu={menu_items} status_bar={status_items}>
    <div class="counter">
        <h1 ref="title">计数器</h1>
        <p class="count">{count}</p>
        <!-- ... 原有内容 ... -->
    </div>
</ModernWindow>
```

---

## 四、Phase 2：快捷键 RML 声明 + Kbd 显示

### 4.1 `<bind>` 元素解析

**文件**：`crates/engine/src/parser/ast.rs`（修改）

`<bind>` 不需要新的 AST 节点类型 — 它是一个普通 `Element`，在 codegen 阶段特殊处理。

**文件**：`crates/engine/src/compiler/codegen.rs`（修改）

在 `gen_node` 中识别 `<bind>` 元素，收集到 `CodegenCtx.bindings`：

```rust
if elem.tag == "bind" {
    // 收集绑定信息，不生成渲染代码
    let key = elem.attributes.iter().find_map(|a| match a {
        Attribute::Static { name: "key", value } => Some(value.clone()),
        _ => None,
    });
    let action_name = elem.attributes.iter().find_map(|a| match a {
        Attribute::Event { name: "onclick", handler } => Some(handler.method_name()),
        _ => None,
    });
    ctx.bindings.push(BindInfo { key, action_name });
    return Ok(("".into(), false));  // bind 不生成渲染代码
}
```

### 4.2 Action 类型生成

**文件**：`crates/engine/src/compiler/shortcut.rs`（新建）

**职责**：为每个 `<bind>` 生成 Action 类型 + 注册代码。

```rust
pub struct BindInfo {
    pub key: String,         // "ctrl+s"
    pub action_name: String, // "save" → PascalCase → "Save"
}

/// 生成 Action 类型定义
/// 在生成代码顶部插入：
/// gpui::actions!(rml_auto_counter, [Save]);
pub fn gen_action_definitions(view_name: &str, bindings: &[BindInfo]) -> String {
    let actions: Vec<String> = bindings.iter()
        .map(|b| to_pascal_case(&b.action_name))
        .collect();
    if actions.is_empty() { return String::new(); }
    format!("gpui::actions!(rml_auto_{}, [{}]);", view_name.to_lowercase(), actions.join(", "))
}

/// 生成 Action 注册 + key binding 代码
/// 插入到 Render::render 方法顶部：
pub fn gen_action_registration(bindings: &[BindInfo]) -> String {
    let mut code = String::new();
    for b in bindings {
        let action = to_pascal_case(&b.action_name);
        code.push_str(&format!(
            "window.bind_keys([gpui::KeyBinding::new({:?}, rml_auto_{}::{}{}, None)]);\n",
            b.key, view_name_lower, action, "::default()"
        ));
        code.push_str(&format!(
            "cx.on_action(|&rml_auto_{}::{}, window, cx| {{ self.{}(window, cx); }});\n",
            view_name_lower, action, b.action_name
        ));
    }
    code
}
```

**关键决策**：Action 注册放在 `Render::render` 顶部，用实例级 `OnceCell<bool>` 去重避免重复绑定。

### 4.3 实例级注册去重

**文件**：`crates/engine/src/compiler/codegen.rs`（修改）

在生成的 `Render` impl 中添加去重逻辑：

```rust
impl gpui::Render for Counter {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        // Auto-generated: action registration (once per instance)
        if !self.__actions_registered {
            window.bind_keys([gpui::KeyBinding::new("ctrl+s", rml_auto_counter::Save::default(), None)]);
            cx.on_action(|&rml_auto_counter::Save, window, cx| { self.save(window, cx); });
            self.__actions_registered = true;
        }
        // ... rest of render
    }
}
```

**字段添加**：`#[view]` 宏生成的 struct 需添加 `__actions_registered: bool` 字段（默认 `false`）。

### 4.4 `<Kbd>` 显示组件

**文件**：`crates/engine/src/compiler/component.rs`（修改）

在 `component_static_setter` 中为 `Kbd` 添加属性映射：

```rust
// Kbd 组件专用：binding="ctrl+s" → Kbd::new(Keystroke::parse("ctrl+s").unwrap())
"binding" if tag == "Kbd" => Some(format!(
    ".binding(gpui::Keystroke::parse({:?}).unwrap())",
    value
)),
```

**注**：`Kbd::new` 接收 `Keystroke`，需要 codegen 解析字符串。Phase 2 先支持字面量字符串，Phase 4 支持表达式绑定。

### 4.5 Demo 快捷键示例

**文件**：`demo/src/counter.rml`（修改）

```html
<ModernWindow title="RML 计数器" menu={menu_items} status_bar={status_items}>
    <bind key="ctrl+=" onclick={increment} />
    <bind key="ctrl+-" onclick={decrement} />
    <bind key="ctrl+r" onclick={reset} />
    
    <div class="counter">
        <!-- ... -->
        <p>快捷键：<Kbd binding="ctrl+=" /> 增加，<Kbd binding="ctrl+-" /> 减少</p>
    </div>
</ModernWindow>
```

---

## 五、Phase 3：完整组件集成

### 5.1 扩展 component_lookup 路由表

**文件**：`crates/engine/src/tags.rs`（修改）

新增映射（共 27 个新标签，加上现有 13 个 = 40）：

```rust
// Stateless 组件（无状态，直接 ::new(id)）
"Avatar" => Stateless, "AvatarGroup" => Stateless,
"Icon" => Stateless,
"Skeleton" => Stateless, "Spinner" => Stateless,
"Tooltip" => Stateless,  // 注：gpui-component Tooltip 是 entity，需特殊处理
"Popover" => Stateless,
"Tab" => Stateless, "TabBar" => Stateless,
"Accordion" => Stateless, "AccordionItem" => Stateless,
"Alert" => Stateless, "Breadcrumb" => Stateless, "BreadcrumbItem" => Stateless,
"Link" => Stateless, "Pagination" => Stateless, "Rating" => Stateless,
"GroupBox" => Stateless, "DescriptionList" => Stateless,
"Stepper" => Stateless, "HoverCard" => Stateless,
"Progress" => Stateless, "ProgressCircle" => Stateless,  // 已有

// Stateful 组件（需要 &self.<field> state entity）
"Dialog" => Stateful { state_field: "dialog_state" },
"List" => Stateful { state_field: "list_state" },
"Table" => Stateful { state_field: "table_state" },
"DataTable" => Stateful { state_field: "table_state" },
"Select" => Stateful { state_field: "select_state" },
"Combobox" => Stateful { state_field: "combobox_state" },
"Radio" => Stateless, "RadioGroup" => Stateful { state_field: "radio_state" },
"ColorPicker" => Stateful { state_field: "color_picker_state" },
"Tree" => Stateful { state_field: "tree_state" },
"NumberInput" => Stateful { state_field: "input_state" },  // 复用 input_state
"OtpInput" => Stateful { state_field: "otp_state" },
"Sidebar" => Stateful { state_field: "sidebar_state" },
"Dock" => Stateful { state_field: "dock_state" },
```

### 5.2 组件元信息表

**文件**：`crates/engine/src/compiler/component_meta.rs`（新建）

**职责**：按组件声明支持的属性、事件、特有 setter。

```rust
pub struct ComponentMeta {
    pub tag: &'static str,
    pub supports_label: bool,
    pub supports_placeholder: bool,
    pub supports_tooltip: bool,
    pub supports_disabled: bool,
    pub supports_size: bool,
    pub supports_variant: bool,
    pub custom_setters: &'static [(&'static str, &'static str)],  // (attr_name, setter_template)
    pub custom_events: &'static [(&'static str, &'static str)],   // (event_name, method)
}

static COMPONENT_META: &[ComponentMeta] = &[
    ComponentMeta {
        tag: "Button",
        supports_label: true,
        supports_placeholder: false,
        supports_tooltip: true,
        supports_disabled: true,
        supports_size: true,
        supports_variant: true,
        custom_setters: &[("icon", ".icon({})")],
        custom_events: &[("onclick", ".on_click")],
    },
    ComponentMeta {
        tag: "Input",
        supports_label: false,
        supports_placeholder: true,
        supports_tooltip: false,
        supports_disabled: true,
        supports_size: true,
        supports_variant: false,
        custom_setters: &[],
        custom_events: &[("onclick", ".on_click"), ("onchange", ".on_change")],
    },
    // ... 40+ 组件
];

pub fn lookup_meta(tag: &str) -> Option<&'static ComponentMeta> {
    COMPONENT_META.iter().find(|m| m.tag == tag)
}
```

### 5.3 component.rs 重构

**文件**：`crates/engine/src/compiler/component.rs`（修改）

`gen_component` 改为先查 `ComponentMeta`，按元信息生成 setter：

```rust
pub fn gen_component(elem: &Element, ctx: &CodegenCtx, ...) -> Result<String, CodegenError> {
    let tag = elem.component_tag();
    let meta = component_meta::lookup_meta(&tag);
    // ... 按 meta 生成代码
}
```

---

## 六、Phase 4：高级组件 + 多窗口

### 6.1 复杂组件集成
- `List` / `DataTable` — 数据驱动渲染，`each` 指令与 state delegate 协作
- `Dialog` — 通过 `WindowExt::open_dialog` 触发，RML 中声明 `<Dialog>` 模板
- `Select` / `Combobox` — 选项数据绑定
- `Tooltip` — `tooltip="text"` 属性映射到 hover tooltip
- `Dock` / `Sidebar` — 多面板布局

### 6.2 多窗口
**文件**：`crates/app/src/window.rs`（扩展）

- `open_window::<V>()` 返回 `WindowHandle<R>`
- 窗口间通信 via `cx.app_global()` 或 event bus
- 窗口关闭回调

### 6.3 主题集成
- CSS 子集与 gpui-component 主题协作
- `cx.theme()` 注入

---

## 七、验证步骤 Verification

### Phase 1 验证
1. `cargo build -p rust-rml-app` — IAppLifecycle trait 编译通过
2. `cargo build -p rust-rml-ui` — ModernWindow + 数据类型编译通过
3. `cargo build -p rust-rml-engine` — 标签路由表扩展编译通过
4. `cargo test -p rust-rml-engine` — 现有 180+ 测试不回归
5. `cargo run -p rust-rml-demo` — demo 启动，显示 ModernWindow（标题栏 + 菜单 + 状态栏）
6. 菜单点击 → 触发 ViewModel 命令（如"重置"）
7. ViewModel 调用 `window.notify_info("已重置", cx)` → 右下角通知出现

### Phase 2 验证
1. `cargo build -p rust-rml-engine` — shortcut codegen 编译通过
2. `cargo test -p rust-rml-engine` — 新增 bind codegen 测试通过
3. demo 中按 `Ctrl+=` → 计数器 +1
4. demo 中按 `Ctrl+-` → 计数器 -1
5. `<Kbd binding="ctrl+=" />` 显示为样式化按键标签

### Phase 3 验证
1. `component_lookup` 返回 40+ 组件
2. 新增单元测试：每个新组件的 codegen 输出正确
3. demo 中使用 `<Tooltip>` / `<Popover>` / `<Avatar>` 等组件

---

## 八、假设与决策 Assumptions & Decisions

### 假设
1. **gpui-component API 稳定**：v0.5.2 的 `TitleBar` / `StatusBar` / `Kbd` / `Notification` / `PopupMenu` API 不会发生破坏性变更
2. **GPUI `cx.on_action` 幂等**：重复调用会替换 handler，不会累积
3. **`window.bind_keys` 可重复调用**：会累积绑定，需要实例级去重
4. **`crates/ui` 依赖 `smallvec`**：需在 `Cargo.toml` 添加（用于 `MenuItem.children`）

### 决策
1. **ModernWindow 是 Stateless RenderOnce**：不持有 state，数据通过 builder 方法注入
2. **MenuItem 用闭包而非命令名**：闭包捕获 `WeakEntity<T>`，符合 GPUI 事件模式；不引入字符串命令派发的间接层
3. **Action 注册在 `render()` 中**：用实例级 `__actions_registered: bool` 去重，避免修改 `ILifecycle` trait 签名（向后兼容）
4. **`<bind>` 不生成渲染输出**：仅作为 codegen 时的指令收集，不出现在渲染树中
5. **`open_window` vs `open_modern_window`**：前者用默认 titlebar，后者用透明 titlebar + `TitleBar` 组件自绘
6. **命名无 Rml 前缀**：所有新类型（`ModernWindow` / `MenuItem` / `IAppLifecycle` 等）均不加前缀；保留 `RmlApplication`（类比 `Application`）

---

## 九、文件变更清单 File Change List

### Phase 1（新建）
| 文件 | 职责 |
|------|------|
| `crates/app/src/lifecycle.rs` | `IAppLifecycle` trait |
| `crates/ui/src/window/mod.rs` | window 模块根 |
| `crates/ui/src/window/modern_window.rs` | `ModernWindow` 组件 |
| `crates/ui/src/window/types.rs` | `MenuItem` / `StatusBarItem` 数据类型 |
| `crates/ui/src/window/menu_bar.rs` | 菜单栏渲染器 |
| `crates/ui/src/window/actions.rs` | `IWindowActions` trait |

### Phase 1（修改）
| 文件 | 变更 |
|------|------|
| `crates/app/src/lib.rs` | 添加 `pub mod lifecycle; pub use lifecycle::IAppLifecycle;` |
| `crates/app/src/application.rs` | `run::<A: IAppLifecycle>` 签名重构 |
| `crates/app/src/window.rs` | 添加 `open_window` / `open_modern_window` helpers |
| `crates/ui/src/lib.rs` | 添加 `pub mod window;` + re-exports |
| `crates/ui/src/prelude.rs` | 添加 ModernWindow/MenuItem 等 |
| `crates/ui/Cargo.toml` | 添加 `smallvec` 依赖 |
| `crates/engine/src/tags.rs` | 添加 TitleBar/StatusBar/Kbd/ModernWindow 路由 |
| `crates/engine/src/compiler/component.rs` | 添加 ModernWindow 专用 setter |
| `demo/src/main.rs` | 迁移到 IAppLifecycle 模式 |
| `demo/src/counter.rml` | 使用 `<ModernWindow>` 根标签 |
| `demo/src/counter.rml.rs` | 添加 menu_items/status_items 字段 |

### Phase 2（新建）
| 文件 | 职责 |
|------|------|
| `crates/engine/src/compiler/shortcut.rs` | `<bind>` codegen |
| `crates/engine/src/compiler/component_meta.rs` | 组件元信息表 |

### Phase 2（修改）
| 文件 | 变更 |
|------|------|
| `crates/engine/src/compiler/codegen.rs` | `<bind>` 收集 + action 注册注入 |
| `crates/engine/src/compiler/component.rs` | Kbd 专用 setter |
| `crates/engine/src/compiler/mod.rs` | 注册 shortcut 模块 |

### Phase 3（修改）
| 文件 | 变更 |
|------|------|
| `crates/engine/src/tags.rs` | 扩展至 40+ 组件路由 |
| `crates/engine/src/compiler/component.rs` | 按 ComponentMeta 生成 setter |

---

## 十、执行顺序 Execution Order

1. **Phase 1.1** — `crates/app/src/lifecycle.rs`（IAppLifecycle trait）
2. **Phase 1.2** — `crates/app/src/application.rs` 重构 + `crates/app/src/window.rs` 助手
3. **Phase 1.3** — `crates/ui/src/window/` 全部 5 个文件
4. **Phase 1.4** — `crates/ui/src/lib.rs` + `prelude.rs` 整合
5. **Phase 1.5** — `crates/engine/src/tags.rs` 路由扩展 + `component.rs` setter
6. **Phase 1.6** — demo 迁移（main.rs + counter.rml + counter.rml.rs）
7. **Phase 1.7** — 验证：cargo build + cargo test + cargo run

每个子步骤完成后立即验证编译，避免错误累积。
