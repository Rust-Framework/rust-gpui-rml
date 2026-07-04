# Menu/StatusBar MVVM ViewModel 落地计划（剩余部分 D-H + 验证）

## 摘要

承接前序已完成的工作（Parts A/B/C/I：框架侧 codegen `each` 支持 + `MenuBar`/`StatusBar` 纯容器化 + `items` 绑定路径清理），本计划完成**业务侧剩余工作**：创建 `MenuViewModel`/`StatusViewModel`、重构 `MainWindow`（`RelayCommand` 字段 + 类型化集合）、`StatusReady` 实现 `IVisualContribution`、更新 RML 模板、删除 `shell_chrome.rs` 与 `menu_shell_contribs.rs`。

## 当前状态分析

### 已完成（Parts A/B/C/I）

* **A1** [menu\_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs)：`gen_menu_bar` 支持 `<menu-item each={m in menus}>`，生成 `self.menus.iter().map(|m| ...)` + `MenuBar::new(...).children(...)`。`gen_menu_bar_button_for_item` 处理 loop\_var 上下文的 label/command/嵌套 submenu。

* **A2/A3/A4** [item.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/item.rs)：`gen_command_closure` 支持 loop\_var（生成 `let __rml_cmd = {access}.clone(); if let Some(cmd) = &__rml_cmd { cmd.can_execute(...); cmd.execute(...); }`）；`icon` 支持 bind；`gen_popup_menu_body` 支持嵌套 `each`。

* **B** [menu.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs)：`MenuBar` 为纯 `ParentElement` 容器，`IMenuItem`/`MenuItem`/`build_popup_menu_from_items` 已删除。

* **C** [status\_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs)：`StatusBar` 为纯 `ParentElement` 容器，`IStatusBarItem`/`StatusBarItem` 已删除，`StatusBarAlign` 保留。

* **I1** [props\_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)：`MenuBar`/`menu`/`StatusBar` 的 `items` 注册已移除，测试验证 NOT registered。

* **I2** [setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/setters.rs)：`bind_setter` 简化为始终返回 `None`。

* **I4** [node.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs)：`<component>` 支持 `each` 指令，生成 `{iter_expr}.iter().map(|{item}| {code})` + `is_iter=true`。

### 未完成（本计划范围）

demo crate 当前**无法编译**——`main_window.rml.rs` 与 `shell_chrome.rs` 仍 import 已删除的 `IMenuItem`/`IStatusBarItem`/`MenuItem`/`StatusBarItem`。

* **Part D**：创建 `MenuViewModel` + `StatusViewModel`

* **Part E**：重构 `MainWindow`——`RelayCommand` 字段 + `Vec<MenuViewModel>`/`Vec<StatusViewModel>` 集合 + `build_menu_tree()` 方法

* **Part F**：`StatusReady` 实现 `IVisualContribution`

* **Part G**：更新 `main_window.rml` 模板——`each` 指令 + `<component>` 迭代

* **Part H**：删除 `shell_chrome.rs` + `menu_shell_contribs.rs`，更新 `mod.rs`

## 关键设计决策

### 决策 1：菜单使用 `RelayCommand` 字段，不使用贡献注册

**原因**：

* `Arc<dyn IContribution>` 无法 downcast 为 `Arc<dyn ICommand>`（trait object 不可逆）

* codegen（Part A2 已实现）在 loop\_var 上下文生成 `cmd.can_execute()` / `cmd.execute()` 直接调用，要求字段类型为 `Option<Arc<dyn ICommand>>`

* `RelayCommand` 实现 `ICommand`，可作为 `Arc<dyn ICommand>` 持有

* 消除 323 行 `menu_shell_contribs.rs`（11 个 struct + `impl IContribution` + `impl ICommand` + `with_main_window` 样板），替换为 7 个 `RelayCommand::new` 一行初始化

**影响**：菜单不再经贡献系统注册。"host 接收贡献当场转化 ViewModel" 模式适用于 **cases / status / activities**，菜单改由 `MainWindow::build_menu_tree()` 手工构建树 + `RelayCommand` 字段绑定。

### 决策 2：菜单树整体硬编码，submenu root 不保留为贡献

**原因**：

* 用户要求"大幅度精简代码" + "消除 shell\_chrome 之类的多余实现"

* submenu root 标签经 `t_static()` 直接获取 i18n，无需经贡献 `name()` 间接获取

* `build_menu_tree()` 在 `on_loaded` 中一次性构建，locale 切换时 `t_static()` 自动刷新

### 决策 3：`MenuViewModel.command` 类型为 `Option<Arc<dyn ICommand>>`

对齐用户原案 `contribution -> ICommand`。叶子节点持有 `RelayCommand`（`Arc<dyn ICommand>`），submenu root 为 `None`。codegen 生成的 `command={c.command}` 绑定闭包直接调用 `can_execute`/`execute`。

### 决策 4：`StatusViewModel` 经贡献系统构建

status 项需 `IVisualContribution::render()` 返回富内容（`AnyElement`），保留贡献模式。`from_contribution()` 过滤 `kind="status"` 槽位 + `as_visual()` 校验。

### 决策 5：`ContribEntry` 类型别名迁移至 `main_window.rml.rs`

`shell_chrome.rs` 删除后，`pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions)` 移入 `main_window.rml.rs`（`MainWindow` 内部使用）。

## 实施步骤

### Part D：创建 MenuViewModel / StatusViewModel

#### D1：`demo/src/shell/menu_view_model.rs`（新建）

```rust
//! 菜单视图模型 —— 手工构建的类型化树结构。
//!
//! 供 MainWindow.menus 集合持有，RML <menu-item each={m in menus}> 直接消费。
//! 菜单不经贡献系统注册（消除 menu_shell_contribs.rs 样板），
//! 叶子节点的 command 字段持有 MainWindow 的 RelayCommand。

use std::sync::Arc;
use gpui::SharedString;
use rml_core::command::ICommand;

#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub label: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
    /// 叶子节点持有 RelayCommand（Arc<dyn ICommand>）；submenu root 为 None
    pub command: Option<Arc<dyn ICommand>>,
    /// 子菜单（按 order 排序）
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// submenu root 构造（无命令）
    pub fn root(id: &str, label: SharedString, order: i32) -> Self {
        Self {
            id: id.into(),
            label,
            group: None,
            order,
            command: None,
            children: Vec::new(),
        }
    }

    /// 叶子节点构造（带命令）
    pub fn leaf(id: &str, label: SharedString, order: i32, command: Arc<dyn ICommand>) -> Self {
        Self {
            id: id.into(),
            label,
            group: None,
            order,
            command: Some(command),
            children: Vec::new(),
        }
    }

    pub fn child(mut self, child: MenuViewModel) -> Self {
        self.children.push(child);
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}
```

#### D2：`demo/src/shell/status_view_model.rs`（新建）

```rust
//! 状态栏视图模型 —— 解包 (IVisualContribution, ContributionOptions) 为类型化结构。
//!
//! 供 MainWindow.status 集合持有，RML <component each={s in status} content={s.render(_window, cx)} /> 直接消费。

use std::sync::Arc;
use gpui::SharedString;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};
use rml_ui::StatusBarAlign;

/// 贡献条目类型别名（从 shell_chrome.rs 迁入）
pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);

#[derive(Clone)]
pub struct StatusViewModel {
    pub id: SharedString,
    pub align: StatusBarAlign,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
}

impl StatusViewModel {
    /// 从贡献条目构造；非 status 槽位或非视觉贡献返回 None。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("status") {
            return None;
        }
        c.as_visual()?;
        let align = match opts.properties.get("align").map(|s| s.as_ref()) {
            Some("right") => StatusBarAlign::Right,
            Some("center") => StatusBarAlign::Center,
            _ => StatusBarAlign::Left,
        };
        Some(Self {
            id: c.id().into(),
            align,
            order: opts.order,
            contribution: c,
        })
    }

    /// 渲染状态栏项（委托给底层 IVisualContribution）。
    pub fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        self.contribution
            .as_visual()
            .expect("StatusViewModel requires IVisualContribution")
            .render(window, cx)
    }
}

/// 从贡献条目列表构建 StatusViewModel 列表（按 order 排序）。
pub fn build_status_view_models(entries: &[ContribEntry]) -> Vec<StatusViewModel> {
    let mut items: Vec<StatusViewModel> = entries
        .iter()
        .filter_map(|(c, o)| StatusViewModel::from_contribution(c.clone(), o.clone()))
        .collect();
    items.sort_by_key(|s| s.order);
    items
}
```

***

### Part E：重构 MainWindow

**文件**: [main\_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

#### E1：更新 import

移除：`IMenuItem` / `IStatusBarItem` / `StatusBarItem` / `build_menu_tree` / `ContribEntry`（from shell\_chrome）

新增：

```rust
use rml_core::command::{ICommand, RelayCommand};
use rml_core::i18n::t_static;
use crate::shell::menu_view_model::MenuViewModel;
use crate::shell::status_view_model::{build_status_view_models, ContribEntry, StatusViewModel};
```

#### E2：更新 struct 字段

```rust
#[window]
#[contributehost(id = "demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    pub cases: Vec<CaseViewModel>,
    pub menus: Vec<MenuViewModel>,
    pub status: Vec<StatusViewModel>,
    activities: Vec<Arc<dyn IActivityPanel>>,

    // RelayCommand 字段（WPF MVVM 模式，7 个叶子命令）
    open_welcome_command: Arc<dyn ICommand>,
    open_button_case_command: Arc<dyn ICommand>,
    open_menu_dropdown_case_command: Arc<dyn ICommand>,
    open_features_case_command: Arc<dyn ICommand>,
    toggle_theme_command: Arc<dyn ICommand>,
    switch_en_command: Arc<dyn ICommand>,
    exit_command: Arc<dyn ICommand>,

    // Tab 状态
    open_tabs: Vec<Arc<dyn IValue>>,
    selected_tab: usize,
    show_chrome: bool,
    slot_left_size: gpui::Pixels,

    // 框架仪式
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    entries: std::sync::RwLock<Vec<ContribEntry>>,
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
    manager: Option<Arc<DemoWorkbenchManager>>,
    lsp_client: Option<Arc<LspClient>>,
}
```

**注意**：`Arc<dyn ICommand>` 字段配合 `#[derive(Default)]`——`RelayCommand` 实现了 `Default`（no-op），`Arc::<RelayCommand>::default()` 可作为初始值。`on_loaded` 中替换为真实命令。

但 `Arc<dyn ICommand>` 的 `Default` 需要 `Arc<dyn ICommand>: Default`。由于 `RelayCommand: Default`，`Arc::new(RelayCommand::default())` 可作为 `Arc<dyn ICommand>`。但 `#[derive(Default)]` 会尝试 `Arc::<dyn ICommand>::default()`，这需要 `dyn ICommand: Default`，不成立。

**修正**：移除 `#[derive(Default)]`，手写 `impl Default for MainWindow`：

```rust
impl Default for MainWindow {
    fn default() -> Self {
        let default_cmd: Arc<dyn ICommand> = Arc::new(RelayCommand::default());
        Self {
            cases: Vec::new(),
            menus: Vec::new(),
            status: Vec::new(),
            activities: Vec::new(),
            open_welcome_command: default_cmd.clone(),
            open_button_case_command: default_cmd.clone(),
            open_menu_dropdown_case_command: default_cmd.clone(),
            open_features_case_command: default_cmd.clone(),
            toggle_theme_command: default_cmd.clone(),
            switch_en_command: default_cmd.clone(),
            exit_command: default_cmd,
            open_tabs: Vec::new(),
            selected_tab: 0,
            show_chrome: false,
            slot_left_size: gpui::px(260.),
            activity_bar: None,
            entries: std::sync::RwLock::new(Vec::new()),
            host_rx: None,
            manager: None,
            lsp_client: None,
        }
    }
}
```

#### E3：更新 `on_loaded`——初始化 RelayCommand + 构建菜单树

在 `project_entries()` 之后添加 RelayCommand 初始化 + `build_menu_tree()`：

```rust
impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 1. 注册 host + drain
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }

        // 2. 初始化 RelayCommand 字段（WPF MVVM 模式）
        self.open_welcome_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("welcome".to_string(), cx);
        }));
        self.open_button_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.button".to_string(), cx);
        }));
        self.open_menu_dropdown_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.menu.dropdown".to_string(), cx);
        }));
        self.open_features_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.open_case("components.menu.features".to_string(), cx);
        }));
        self.toggle_theme_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_toggle_theme(cx);
        }));
        self.switch_en_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.apply_switch_en(cx);
        }));
        self.exit_command = Arc::new(RelayCommand::action(|cx| cx.quit()));

        // 3. 投影到类型化集合（cases/status/activities 经贡献；menus 手工构建）
        self.project_entries();

        // 4. MainWindowRef 单例
        let shell_weak = cx.weak_entity();
        cx.set_service(Arc::new(MainWindowRef(shell_weak)));

        // 5-8. LSP / manager / welcome tab / ActivityBar（保持不变）
        // ...
    }
}
```

#### E4：更新 `project_entries` + 新增 `build_menu_tree`

```rust
impl MainWindow {
    fn project_entries(&mut self) {
        let entries = self.entries.read().unwrap();
        self.cases = entries
            .iter()
            .filter_map(|(c, o)| CaseViewModel::from_contribution(c.clone(), o.clone()))
            .collect();
        self.status = build_status_view_models(&entries);
        self.activities = entries
            .iter()
            .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
            .filter_map(|(c, _)| {
                VisualActivityPanel::new(c.clone()).map(|p| Arc::new(p) as Arc<dyn IActivityPanel>)
            })
            .collect();
        // menus 不经贡献系统，由 build_menu_tree 手工构建
        self.menus = self.build_menu_tree();
    }

    /// 手工构建菜单树（消除 menu_shell_contribs.rs + shell_chrome.rs）。
    /// 标签经 t_static() 获取 i18n；命令绑定到 RelayCommand 字段。
    fn build_menu_tree(&self) -> Vec<MenuViewModel> {
        vec![
            MenuViewModel::root("menu.file", t_static("menu.file"), 0)
                .child(MenuViewModel::leaf(
                    "menu.file.new",
                    t_static("menu.file_new"),
                    0,
                    self.open_welcome_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.file.open",
                    t_static("menu.file_open"),
                    1,
                    self.open_button_case_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.file.exit",
                    t_static("menu.file_exit"),
                    2,
                    self.exit_command.clone(),
                )),
            MenuViewModel::root("menu.view", t_static("menu.view"), 10)
                .child(MenuViewModel::leaf(
                    "menu.theme_toggle",
                    t_static("menu.theme_toggle"),
                    0,
                    self.toggle_theme_command.clone(),
                ))
                .child(MenuViewModel::leaf(
                    "menu.lang_en",
                    t_static("menu.lang_en"),
                    1,
                    self.switch_en_command.clone(),
                )),
            MenuViewModel::root("menu.help", t_static("menu.help"), 20)
                .child(MenuViewModel::root("menu.help.docs", t_static("case.menu.help_center"), 0)
                    .child(MenuViewModel::leaf(
                        "menu.help.guide",
                        t_static("case.menu.nested"),
                        0,
                        self.open_menu_dropdown_case_command.clone(),
                    ))
                    .child(MenuViewModel::leaf(
                        "menu.help.about",
                        t_static("menu.help_about"),
                        1,
                        self.open_welcome_command.clone(),
                    )))
                .child(MenuViewModel::root("menu.help.cases", t_static("case.menu.features.group"), 1)
                    .child(MenuViewModel::leaf(
                        "menu.open_features",
                        t_static("case.menu.features.title"),
                        0,
                        self.open_features_case_command.clone(),
                    ))),
        ]
    }
}
```

#### E5：删除 `project_chrome` / `build_status_items`

* 删除 `fn project_chrome(&mut self)`（157-173 行区间）

* 删除 `fn build_status_items(...)`（175-191 行区间）

#### E6：更新 `apply_switch_en`

```rust
pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
    cx.set_i18n("en-US");
    // 刷新 menus（t_static 自动读取新 locale）+ status（名称经贡献 name() 刷新）
    self.menus = self.build_menu_tree();
    self.status = {
        let entries = self.entries.read().unwrap();
        build_status_view_models(&entries)
    };
    cx.notify();
}
```

***

### Part F：StatusReady 实现 IVisualContribution

**文件**: [status\_bar\_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml.rs) L42-54

```rust
use gpui::{AnyElement, SharedString};
use rml::prelude::*;
use rml_core::contribution::IVisualContribution;
use rml_core::i18n::t_static;

// ... StatusBarCase 保持不变 ...

#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.status_ready").into() }
}

impl IVisualContribution for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        gpui::div()
            .text_xs()
            .child(t_static("shell.status_ready"))
            .into_any_element()
    }
}
```

**注意**：确认 `IVisualContribution` trait 的 `render` 签名为 `fn render(&self, &mut Window, &mut App) -> AnyElement`（对齐 project\_memory 约束：`IVisualContribution::render` 直接接收 `&mut Window, &mut App`，不经 `RenderContext` 包装）。

***

### Part G：更新 main\_window\.rml 模板

**文件**: [main\_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

```rml
<tab-window
    title="RML Showcase"
    width="1100"
    height="720"
    startup="CenterScreen"
    icon={IconName::Frame}
    tabs={tab_bar_items}
    selected-index={selected_tab}
    on-tab-click="on_tab_click"
    show-chrome={show_chrome}
    on-chrome-toggle="on_chrome_toggle"
    left-size={slot_left_size}>

    <template slot="left">
        <ActivityBar ref="activity_bar" />
    </template>

    <template slot="menu">
        <menu-bar>
            <menu-item each={m in menus} label={m.label}>
                <menu-item each={c in m.children} label={c.label} command={c.command} />
            </menu-item>
        </menu-bar>
    </template>

    <template slot="title">
        <Button label="Docs" ghost="" />
    </template>

    <template slot="bottom">
        <div>Output panel — drag the top edge to resize</div>
    </template>

    <template slot="footer">
        <status-bar>
            <component each={s in status} content={s.render(_window, cx)} />
        </status-bar>
    </template>

    <component content={self.active_view(_window, cx)} />

</tab-window>
```

**codegen 行为验证**：

* `<menu-item each={m in menus} label={m.label}>` → `self.menus.iter().map(|m| menu_bar_button(...m.label...).dropdown_menu(...))`，`is_iter=true` → `MenuBar::children(...)`

* 嵌套 `<menu-item each={c in m.children} label={c.label} command={c.command} />` → 在 dropdown\_menu 闭包内 `for c in m.children.iter() { menu = menu.item(...).on_click(...) }`

* `command={c.command}` → `let __rml_cmd = c.command.clone(); if let Some(cmd) = &__rml_cmd { cmd.can_execute(...); cmd.execute(...); }`（`c.command` 为 `Option<Arc<dyn ICommand>>`）

* `<component each={s in status} content={s.render(_window, cx)} />` → `self.status.iter().map(|s| s.render(_window, cx))`，`is_iter=true` → `StatusBar::children(...)`

**嵌套 each 的 iterable 识别**：`m.children` 以 loop\_var `m` 开头 → codegen 生成 `m.children.iter()`（不是 `self.m.children.iter()`），正确捕获外层 loop\_var。

***

### Part H：删除 shell\_chrome.rs + menu\_shell\_contribs.rs，更新 mod.rs

#### H1：删除文件

* 删除 [shell\_chrome.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_chrome.rs)

* 删除 [menu\_shell\_contribs.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs)

`#[contribute]` 宏生成的 `#[ctor::ctor]` 注册函数随文件删除自动失效，对应菜单贡献不再注册（符合设计——菜单改用 RelayCommand）。

#### H2：更新 `demo/src/shell/mod.rs`

```rust
pub mod case_view_model;
pub mod menu_view_model;
pub mod status_view_model;
pub mod workbench;
#[path = "activity_panel.rml.rs"]
pub mod activity_panel;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use main_window::{MainWindow, MainWindowRef};
```

***

## 假设与验证

### 假设

1. `IVisualContribution::render` 签名为 `fn render(&self, &mut Window, &mut App) -> AnyElement`（对齐 project\_memory）
2. `t_static()` 返回 `SharedString`，`MenuViewModel::label` / `root()` / `leaf()` 接受 `SharedString`
3. `RelayCommand::new(cx, |this, cx| ...)` 闭包签名 `Fn(&mut MainWindow, &mut Context<MainWindow>)`——`this` 方法需为 `pub` 或同 crate 可见
4. `#[contribute]` 宏经 `#[ctor::ctor]` 自动注册，删除文件即取消注册
5. `Context<MainWindow>` 可 deref 为 `&mut App`，`s.render(_window, cx)` 在 RML render 作用域内可编译

### 验证步骤

1. `cargo build -p rust-rml-engine` —— 确认 engine crate 编译（Parts A/I 已完成，应通过）
2. `cargo build -p rust-rml-ui` —— 确认 ui crate 编译（Parts B/C 已完成，应通过）
3. `cargo build -p rust-rml-demo` —— **核心验证**：确认 ViewModel + MainWindow + RML 模板编译通过
4. `cargo test -p rust-rml-engine` —— 确认 props\_registry 一致性测试通过
5. 运行 demo 手动验证：

   * 菜单栏显示 File / View / Help 三组

   * 点击 File → New 打开 welcome tab

   * 点击 File → Open 打开 button case

   * 点击 File → Exit 退出应用

   * 点击 View → Theme Toggle 切换主题

   * 点击 View → Lang EN 切换语言（菜单标签刷新）

   * Help → Docs → Guide 打开 dropdown case

   * Help → Docs → About 打开 welcome

   * Help → Cases → Features 打开 features case

   * 状态栏显示 StatusReady 文本

### 风险点

1. **`Arc<dyn ICommand>`** **字段 +** **`#[derive(Default)]`**：`dyn ICommand` 无 `Default`，需手写 `impl Default`。已在 E2 修正。
2. **嵌套** **`each`** **的 loop\_var 捕获**：`<menu-item each={c in m.children}>` 在 dropdown\_menu 闭包内，闭包为 `move`，外层 `m` 会被捕获。codegen（Part A4）生成 `for c in m.children.iter()`——需确认 `m` 在闭包作用域可见。
3. **`s.render(_window, cx)`** **借用**：`self.status.iter()` 借用 `&self.status`，`cx` 为 `&mut Context<Self>`——两者不冲突（不同借用目标）。
4. **`StatusBarAlign`** **导出**：`StatusViewModel` 引用 `rml_ui::StatusBarAlign`——已在 [prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/prelude.rs) 导出。

