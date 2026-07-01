# Demo 单 Host 贡献架构重构计划

> **Status: Superseded** — 已完成。请参阅 [docs/09-architecture/contribution-system.md](../../docs/09-architecture/contribution-system.md)。本文档保留作历史参考，请勿按此文实现 `wire_host_changed` / 手动 `on_launch` 注册等已废弃 API。

## 概述

将 demo 从 3 个独立 host（`ACTIVITY_BAR`/`STATUS`/`CASE_TREE`）重构为**单 host + kind 分类**架构。MainWindow 作为 host 管理者，通过 `properties["kind"]`（`menu`/`activity`/`status`/`case`）分类贡献并维护 UI 绑定字段。`CaseActivityPanel` 封装案例树，cases 通过 `#[contribute]` 自注册。

## 当前状态分析

### 现有架构问题
- 3 个独立 host（`hosts.rs`）：`ACTIVITY_BAR`/`STATUS`/`CASE_TREE`，职责分散
- `bindings.rs` 重复映射 host→UI 数据，逻辑碎片化
- `features/` 目录 5 个文件（`case_tree.rs`/`navigation.rs`/`samples_panel.rs`/`status_text.rs`/`mod.rs`）承担注册逻辑，文件过多
- `cases/catalog.rs` 独立维护树构建逻辑
- `MainWindow` 持有 5 个 `Option<Entity<XxxCase>>` 字段 + 3 host 的 `on_changed` 回调，耦合重

### 关键约束
1. **`IContributionHost: Send + Sync`**：MainWindow 是 ViewModel（被 GPUI Entity 持有），无法同时作为 `Box<dyn IContributionHost>` 存入全局 registry。因此 MainWindow **不直接 impl `IContributionHost` trait**，而是通过 registry 中的 `ContributionHost` 实例 + `on_changed` 回调扮演 host 管理者角色——接收贡献（经 registry `register`）、维护 UI 字段（`on_changed` 中按 kind 分类刷新）、通过 add/remove 管理数据（`register`/`unregister`）。
2. **`#[contribute]` + `#[component]` 宏叠加**：两者都 parse `ItemStruct`。Rust 属性宏自下而上展开，`#[component]` 先展开为 `struct + impls + include!`（多 item），`#[contribute]` 需处理多 item 输入。
3. **`<Tree>` 标签硬编码**：codegen 查找 `self.case_tree_state` 字段（`tags.rs:295`），任何使用 `<Tree>` 的组件必须有此字段。
4. **`#[computed]` 无法访问 cx**：案例树数据需在 `on_loaded` / 回调中构建，不能走 computed。

## 宏改动

### `crates/macros/src/contribute.rs`

**1. 新增 `kind` 参数**：
- `ContributeArgs` 增加 `kind: Option<LitStr>`
- 解析分支 `Some("kind") => kind = Some(parse2(nv.value)?)`
- 生成代码增加 `.property("kind", #kind)`

**2. 新增 `parent_id` 参数**：
- `ContributeArgs` 增加 `parent_id: Option<LitStr>`
- 解析分支 `Some("parent_id") => parent_id = Some(parse2(nv.value)?)`
- 生成代码增加 `.parent_id(#parent_id)`（当存在时）

**3. 多 item 输入处理（支持与 `#[component]` 叠加）**：
- 当前：`syn::parse2::<ItemStruct>(input)` → 多 item 时失败
- 改为：`syn::parse2::<Vec<syn::Item>>(input)` 遍历找到 `Item::Struct`，提取 `struct_name`
- 输出：原样透传所有 item + 追加生成的 impl + 注册函数
- 兼容单 item（纯 `#[contribute]` 无 `#[component]`）和多 item（叠加）两种场景

生成代码模板（kind/parent_id 按需注入）：
```rust
#(original_items)*  // 透传 struct + impls + include!

impl rml_core::contribution::IContribution for #struct_name { /* id/name/description/icon */ }

impl rml_app::contribution::Registerable for #struct_name { /* into_entry */ }

pub fn #register_fn(cx: &mut gpui::App) {
    let contribution = Arc::new(#struct_name::default());
    let options = ContributionOptions::new()
        .visual_mode(#visual_mode)
        .placement(#placement)
        .property("kind", #kind)        // 新增
        .parent_id(#parent_id)           // 新增（可选）
        #order #group;
    cx.update_global::<ContributionRegistryGlobal, _>(|g, cx| {
        g.0.register(#host, contribution, options, cx);
    });
}
```

## Demo 改动

### 删除文件
- `demo/src/shell/hosts.rs` — 单 host 无需多常量
- `demo/src/shell/bindings.rs` — 映射逻辑内聚到 `contributions.rs`
- `demo/src/features/` 整个目录（`mod.rs`/`case_tree.rs`/`navigation.rs`/`samples_panel.rs`/`status_text.rs`）— 注册逻辑分散，重构后内聚
- `demo/src/cases/catalog.rs` — 树构建移入 `CaseActivityPanel`

### 新增文件

#### `demo/src/shell/contributions.rs`
单 host 常量 + 映射辅助函数 + menu/status 贡献结构。

```rust
use std::sync::Arc;
use gpui::{App, BorrowAppContext, SharedString};
use rml_core::contribution::{IContribution, IContributionRegistry, ContributionOptions, VisualMode, VisualPlacement, ContributedEntry};
use rml_app::contribution::{ContributionRegistryGlobal, data_entry_dyn};
use rml_ui::{ActivityPanel, ActivityPanels, IconName, MenuItems, MenuItem, StatusBarAlign, StatusBarItem, StatusBarItems, TreeItem};

/// 单一 host_id —— MainWindow 管理的所有贡献
pub const SHELL_HOST: &str = "demo.shell";

/// kind 分类常量
pub const KIND_MENU: &str = "menu";
pub const KIND_ACTIVITY: &str = "activity";
pub const KIND_STATUS: &str = "status";
pub const KIND_CASE: &str = "case";

fn kind_of(entry: &ContributedEntry) -> Option<&str> {
    entry.options.properties.get("kind").map(|s| s.as_ref())
}

fn icon_from_name(name: &str) -> IconName { /* match BookOpen/Settings/Frame */ }

/// host → ActivityPanels（kind=activity）
pub fn build_activity_panels<C>(cx: &gpui::Context<C>, active_id: &str) -> ActivityPanels { ... }

/// host → StatusBarItems（kind=status）
pub fn build_status_items<C>(cx: &gpui::Context<C>) -> StatusBarItems { ... }

/// host → MenuItems（kind=menu），命令从 commands map 查找
pub fn build_menu_items<C>(cx: &gpui::Context<C>, commands: &std::collections::HashMap<String, Arc<dyn rml_core::command::ICommand>>) -> MenuItems { ... }

/// host → TreeItem 树（kind=case，按 parent_id 层级）
pub fn build_case_tree_items<C>(cx: &gpui::Context<C>) -> Vec<TreeItem> { ... }

/// 纯文本菜单贡献（id/name_key + kind=menu）
pub struct MenuEntryContribution { id: &'static str, name_key: &'static str }
impl IContribution for MenuEntryContribution { ... }
/// 在 MainWindow.on_loaded 中调用，注册菜单元数据
pub fn register_menu_entry(cx: &mut App, id: &'static str, name_key: &'static str, order: i32) { ... }

/// 纯文本状态贡献（id/name_key + kind=status）
pub struct StatusEntryContribution { id: &'static str, name_key: &'static str }
impl IContribution for StatusEntryContribution { ... }
pub fn register_status_entry(cx: &mut App, id: &'static str, name_key: &'static str, order: i32) { ... }
```

**设计要点**：
- `MenuEntryContribution` / `StatusEntryContribution` 是纯元数据贡献，不含命令。菜单命令由 MainWindow 在 `menu_commands: HashMap<String, Arc<dyn ICommand>>` 侧表维护，`build_menu_items` 按 id 查找挂接。
- `build_case_tree_items` 复用 `build_contribution_tree` 的 parent_id 层级算法，但先按 `kind=case` 过滤 entries（单 host 含全部 kind）。

#### `demo/src/shell/case_activity_panel.rml.rs`
```rust
use rml::prelude::*;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::i18n::I18nState;
use rml_ui::TreeState;
use crate::shell::contributions;

#[contribute(
    host = "demo.shell",
    id = "samples",
    name = "shell.samples",
    icon = IconName::BookOpen,
    mode = Panel,
    kind = "activity",
    order = 0,
)]
#[component]
#[derive(Default)]
pub struct CaseActivityPanel {
    pub active: bool,  // 面板激活状态（由 MainWindow 同步）
}

impl ILifecycle for CaseActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 初始化案例树
        self.refresh_tree(cx);
        // 监听 registry 变化（贡献增减）+ i18n 变化（树标签刷新）
        cx.observe_global::<ContributionRegistryGlobal>(|this, cx| {
            this.refresh_tree(cx);
        }).detach();
        cx.observe_global::<I18nState>(|_this, cx| {
            cx.notify();
        }).detach();
    }
}

impl CaseActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = contributions::build_case_tree_items(cx);
        // case_tree_state 字段由 #[component] 注入的机制管理
        // 使用 self.case_tree_state（Option<Entity<TreeState>>）
        if let Some(state) = self.case_tree_state.as_ref() {
            state.update(cx, |s, cx| s.set_items(items, cx));
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.case_tree_state = Some(state);
        }
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &SharedString, cx: &mut Context<Self>) {
        // 转发到 MainWindow（经静态 weak ref）
        crate::shell::main_window::activate_case(item_id.to_string(), cx);
    }
}
```

**关键点**：
- `case_tree_state: Option<Entity<TreeState>>` 字段 —— 注意：`#[component]` 不自动注入此字段，需在结构体显式声明为 `pub` 字段（因 `<Tree>` codegen 查找 `self.case_tree_state`，`tags.rs:295`）。
- `observe_global::<ContributionRegistryGlobal>` 让面板自刷新树（新 case 注册时自动出现），无需 MainWindow 推送。
- `activate_case` 经 `main_window` 模块的静态 weak ref 转发（见下文）。

#### `demo/src/shell/case_activity_panel.rml`
```rml
<component>
    <div class="nav-tree">
        <Tree on_activate="on_case_activate" />
    </div>
</component>
```

### 修改文件

#### `demo/src/shell/mod.rs`
```rust
pub mod contributions;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;
#[path = "case_activity_panel.rml.rs"]
pub mod case_activity_panel;

pub use login_dialog::LoginDialog;
pub use main_window::MainWindow;
pub use case_activity_panel::CaseActivityPanel;
```
删除 `pub mod hosts;` 和 `pub mod bindings;`。

#### `demo/src/shell/main_window.rml.rs`
重构为单 host 架构：

```rust
use std::sync::Arc;
use std::sync::Mutex;
use gpui::{BorrowAppContext, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, MenuItems, StatusBarItems, TabItem};

use crate::cases::{self, ButtonCase, CounterCase, I18nCase, OpenTab, TwoWayCase, WelcomeCase};
use crate::shell::contributions::{self, SHELL_HOST, KIND_MENU};
use crate::shell::CaseActivityPanel;

// 静态 weak ref 桥接 CaseActivityPanel → MainWindow（案例激活）
static MAIN_WINDOW_WEAK: Mutex<Option<gpui::WeakEntity<MainWindow>>> = Mutex::new(None);

pub fn activate_case(case_id: String, app: &mut gpui::App) {
    if let Ok(guard) = MAIN_WINDOW_WEAK.lock() {
        if let Some(weak) = guard.as_ref() {
            if let Some(entity) = weak.upgrade() {
                entity.update(app, |main, cx| main.open_case(case_id, cx));
            }
        }
    }
}

#[window]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    active_panel_id: String,
    activity_panels: ActivityPanels,
    status_items: StatusBarItems,
    menu_items: MenuItems,
    menu_commands: std::collections::HashMap<String, Arc<dyn ICommand>>,
    i18n_version: u32,
    welcome_case: Option<Entity<WelcomeCase>>,
    counter_case: Option<Entity<CounterCase>>,
    two_way_case: Option<Entity<TwoWayCase>>,
    button_case: Option<Entity<ButtonCase>>,
    i18n_case: Option<Entity<I18nCase>>,
    theme_cmd: Option<Arc<dyn ICommand>>,
    lang_cmd: Option<Arc<dyn ICommand>>,
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 1. 注册静态 weak ref（供 CaseActivityPanel 回调）
        if let Ok(mut guard) = MAIN_WINDOW_WEAK.lock() {
            *guard = Some(cx.weak_entity());
        }

        // 2. 初始化默认 Tab
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab { id: "welcome".into(), title: cx.t("shell.welcome").to_string() });
            self.selected_tab = 0;
            self.active_case_id = "welcome".into();
        }
        self.show_chrome = true;
        if self.active_panel_id.is_empty() { self.active_panel_id = "samples".into(); }

        // 3. 初始化 case 实体
        self.welcome_case.get_or_insert_with(|| cx.new(|_| WelcomeCase::default()));
        self.counter_case.get_or_insert_with(|| cx.new(|_| CounterCase::default()));
        self.two_way_case.get_or_insert_with(|| cx.new(|_| TwoWayCase::default()));
        self.button_case.get_or_insert_with(|| cx.new(|_| ButtonCase::default()));
        self.i18n_case.get_or_insert_with(|| cx.new(|_| I18nCase::default()));

        // 4. 创建菜单命令 + 注册菜单贡献（kind=menu）
        self.theme_cmd = Some(Arc::new(RelayCommand::new(cx, |this, cx| this.apply_toggle_theme(cx))));
        self.lang_cmd = Some(Arc::new(RelayCommand::new(cx, |this, cx| this.apply_switch_en(cx))));
        self.menu_commands.insert("menu.theme_toggle".into(), self.theme_cmd.clone().unwrap());
        self.menu_commands.insert("menu.lang_en".into(), self.lang_cmd.clone().unwrap());
        contributions::register_menu_entry(cx, "menu.theme_toggle", "menu.theme_toggle", 0);
        contributions::register_menu_entry(cx, "menu.lang_en", "menu.lang_en", 1);

        // 5. 绑定 host on_changed → 刷新 UI 字段
        Self::wire_host_changed(cx);
        self.refresh_bindings(cx);

        // 6. 登录对话框
        window.defer(cx, |window, cx| { super::LoginDialog::default().open(window, cx); });
    }
}

impl MainWindow {
    fn wire_host_changed(cx: &mut Context<Self>) {
        let weak = cx.weak_entity();
        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.set_host_on_changed(SHELL_HOST, Box::new(move |app| {
                if let Some(entity) = weak.upgrade() {
                    entity.update(app, |main, cx| main.refresh_bindings(cx));
                }
            }));
        });
    }

    fn refresh_bindings(&mut self, cx: &mut Context<Self>) {
        self.activity_panels = contributions::build_activity_panels(cx, &self.active_panel_id);
        self.status_items = contributions::build_status_items(cx);
        self.menu_items = contributions::build_menu_items(cx, &self.menu_commands);
        self.i18n_version = self.i18n_version.wrapping_add(1);
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> { /* 同现有 */ }

    #[command]
    pub fn on_chrome_toggle(&mut self, cx: &mut Context<Self>) { self.show_chrome = !self.show_chrome; }

    #[command]
    pub fn on_panel_change(&mut self, id: &SharedString, cx: &mut Context<Self>) {
        let new_id = id.to_string();
        self.active_panel_id = if self.active_panel_id == new_id { String::new() } else { new_id };
        self.refresh_bindings(cx);
    }

    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) { /* 同现有，用 cases::case_title_key */ }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) { /* 同现有 */ }

    fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) { /* 同现有 */ }
    fn apply_switch_en(&mut self, cx: &mut Context<Self>) { /* 同现有，末尾 refresh_bindings + rebuild menu */ }
}
```

**删除内容**：
- `case_tree_state` 字段（移入 `CaseActivityPanel`）
- `navigation` 导入 + `navigation::set_case_activate_handler` / `navigation::activate_case` 调用
- `wire_contribution_sync` 中 3 host 循环 → 单 host `wire_host_changed`
- `refresh_shell_bindings` → `refresh_bindings`（含 menu_items 刷新）
- `rebuild_menu_items`（改由 `refresh_bindings` 统一从 host 构建）

#### `demo/src/shell/main_window.rml`
```rml
<tab_window title="RML Showcase" width="1100" height="720" startup="CenterScreen"
    icon={IconName::Frame} tabs={tab_bar_items} selected_tab={selected_tab}
    on_tab_click="on_tab_click" show_chrome={show_chrome} on_chrome_toggle="on_chrome_toggle">

    <slot_left>
        <ActivityBar panels={activity_panels} on_panel_change="on_panel_change">
            <div if={active_panel_id == "samples"} class="nav-tree">
                <CaseActivityPanel />
            </div>
        </ActivityBar>
    </slot_left>

    <slot_menu>
        <menu items={menu_items} />
    </slot_menu>

    <slot_footer>
        <status_bar items={status_items} />
    </slot_footer>

    <div class="case-host">
        <div if={active_case_id == "welcome" || active_case_id == ""}><WelcomeCase /></div>
        <div if={active_case_id == "binding.counter"}><CounterCase /></div>
        <div if={active_case_id == "binding.two-way"}><TwoWayCase /></div>
        <div if={active_case_id == "components.button"}><ButtonCase /></div>
        <div if={active_case_id == "i18n.basic"}><I18nCase /></div>
    </div>

</tab_window>
```
唯一变化：`<Tree on_activate="on_case_activate" />` → `<CaseActivityPanel />`。

#### `demo/src/cases/*.rml.rs`（5 个案例文件）
每个案例叠加 `#[contribute]` 注册为 `kind=case`，按 `parent_id` 挂接树层级：

**`welcome_case.rml.rs`**（无贡献，默认 Tab，不计入树）—— 保持不变。

**`counter_case.rml.rs`**：
```rust
#[contribute(
    host = "demo.shell", id = "binding.counter", name = "case.counter.title",
    kind = "case", parent_id = "cat.binding", order = 1,
)]
#[component]
#[derive(Default)]
pub struct CounterCase { pub count: i32 }
// ...（其余不变）
```

**`two_way_case.rml.rs`**：
```rust
#[contribute(host = "demo.shell", id = "binding.two-way", name = "case.two_way.title", kind = "case", parent_id = "cat.binding", order = 2)]
#[component] ...
```

**`button_case.rml.rs`**：
```rust
#[contribute(host = "demo.shell", id = "components.button", name = "case.button.title", kind = "case", parent_id = "cat.components", order = 11)]
#[component] ...
```

**`i18n_case.rml.rs`**：
```rust
#[contribute(host = "demo.shell", id = "i18n.basic", name = "case.i18n.title", kind = "case", parent_id = "cat.i18n", order = 21)]
#[component] ...
```

**分类节点**（`cat.binding`/`cat.components`/`cat.i18n`）：这些是纯树分类，无对应组件。在 `contributions.rs` 中程序化注册（kind=case, parent_id=None）：
```rust
pub fn register_case_categories(cx: &mut App) {
    register_case_node(cx, "cat.binding", "tree.cat.binding", None, 0);
    register_case_node(cx, "cat.components", "tree.cat.components", None, 10);
    register_case_node(cx, "cat.i18n", "tree.cat.i18n", None, 20);
}
```

#### `demo/src/cases/mod.rs`
```rust
pub mod catalog;  // 保留 case_title_key + OpenTab（轻量）
#[path = "welcome_case.rml.rs"] pub mod welcome_case;
#[path = "counter_case.rml.rs"] pub mod counter_case;
#[path = "two_way_case.rml.rs"] pub mod two_way_case;
#[path = "button_case.rml.rs"] pub mod button_case;
#[path = "i18n_case.rml.rs"] pub mod i18n_case;

pub use catalog::{case_title_key, OpenTab};
pub use button_case::ButtonCase;
pub use counter_case::CounterCase;
pub use i18n_case::I18nCase;
pub use two_way_case::TwoWayCase;
pub use welcome_case::WelcomeCase;
```
`catalog.rs` 精简：仅保留 `OpenTab` + `case_title_key`（删除树构建函数，移入 `contributions.rs`）。

#### `demo/src/cases/catalog.rs`
```rust
use gpui::AppContext;
pub struct OpenTab { pub id: String, pub title: String }
pub fn case_title_key(id: &str) -> &'static str { /* 同现有 match */ }
```

#### `demo/src/app.rs`
```rust
use gpui::App;
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use gpui::BorrowAppContext;
use rml_app::contribution::ContributionRegistryGlobal;
use crate::shell::contributions::{self, SHELL_HOST};
use crate::cases::{counter_case, two_way_case, button_case, i18n_case};

#[derive(Default)]
pub struct Startup;
impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
        // 预创建单 host
        cx.update_global::<ContributionRegistryGlobal, _>(|g, _| { g.0.ensure_host(SHELL_HOST); });
        // 注册分类节点（纯元数据，无组件）
        contributions::register_case_categories(cx);
        // 注册状态贡献（kind=status）
        contributions::register_status_entry(cx, "status.ready", "shell.status_ready", 0);
        // 注册案例组件（kind=case，含组件）
        counter_case::__rml_register_countercase(cx);
        two_way_case::__rml_register_twowaycase(cx);
        button_case::__rml_register_buttoncase(cx);
        i18n_case::__rml_register_i18ncase(cx);
        // CaseActivityPanel（kind=activity）由 MainWindow.on_loaded 注册
        //   —— 或在此注册（需确保 __rml_register_* 可独立调用）
        crate::shell::case_activity_panel::__rml_register_caseactivitypanel(cx);
        // 菜单贡献（kind=menu）由 MainWindow.on_loaded 注册（需命令上下文）
    }
}
```

**注意**：`#[contribute]` 生成的 `__rml_register_*` 函数名是结构体名小写。`CaseActivityPanel` → `__rml_register_caseactivitypanel`。需验证 `format_ident!` 的 `to_lowercase()` 产出符合预期。

## 数据流

### 注册时序
```
app.on_launch
  ├─ ensure_host("demo.shell")
  ├─ register_case_categories（cat.* 纯元数据）
  ├─ register_status_entry（kind=status）
  ├─ cases.__rml_register_*（kind=case，含 parent_id）
  └─ CaseActivityPanel.__rml_register_*（kind=activity）

MainWindow.on_loaded
  ├─ 注册菜单贡献（kind=menu，命令存入 menu_commands 侧表）
  ├─ wire_host_changed（设置 on_changed 回调）
  └─ refresh_bindings（首次刷新 UI 字段）
      ├─ build_activity_panels（过滤 kind=activity）
      ├─ build_status_items（过滤 kind=status）
      └─ build_menu_items（过滤 kind=menu + 挂接命令）

CaseActivityPanel.on_loaded
  ├─ refresh_tree（过滤 kind=case，按 parent_id 构建树）
  └─ observe_global::<ContributionRegistryGlobal>（自刷新树）
```

### 案例激活流
```
用户点击树叶子节点
  → Tree on_activate
  → CaseActivityPanel.on_case_activate
  → main_window::activate_case(id, cx)（静态 weak ref）
  → MainWindow.open_case(id, cx)
  → 更新 open_tabs + selected_tab + active_case_id
  → RML if 指令切换 <XxxCase /> 渲染
```

## 假设与决策

1. **MainWindow 不直接 impl `IContributionHost`**：因 `IContributionHost: Send + Sync` 且 registry 持有 `Box<dyn IContributionHost>`，ViewModel 无法同时被 Entity 和 registry 持有。改由 registry 内的 `ContributionHost` 存储 entries，MainWindow 经 `on_changed` 回调扮演 host 管理者。满足用户「接收贡献 + add/remove 维护数据 + 维护 UI 字段」的实质要求。

2. **菜单命令侧表**：菜单贡献是纯元数据（不含命令），命令由 MainWindow 在 `menu_commands: HashMap` 维护，`build_menu_items` 按 id 查找挂接。避免扩展 `IContribution` trait。

3. **案例渲染保持硬编码**：MainWindow 保留 5 个 `Option<Entity<XxxCase>>` 字段 + RML `if` 指令切换。不走 `IVisualContribution` 动态渲染（需更多宏+框架改动，违背「简洁」要求）。`#[contribute]` 仅注册案例树元数据。

4. **CaseActivityPanel 自刷新树**：经 `cx.observe_global::<ContributionRegistryGlobal>` 监听 registry 变化，无需 MainWindow 推送。避免 `on_changed` 单回调竞争。

5. **案例激活桥接用静态 weak ref**：`CaseActivityPanel` 由 RML 实例化，MainWindow 无其 Entity 句柄。静态 `Mutex<Option<WeakEntity<MainWindow>>>` 是最简桥接（同现有 `navigation.rs` 模式，内聚到 `main_window.rml.rs`）。

6. **`case_tree_state` 字段**：`CaseActivityPanel` 显式声明 `pub case_tree_state: Option<Entity<TreeState>>`（`<Tree>` codegen 查找此字段名，`tags.rs:295`）。

## 验证步骤

1. **宏改动验证**：
   - `cargo build -p rust-rml-macros` 编译通过
   - `#[contribute]` + `#[component]` 叠加不报错
   - `kind`/`parent_id` 参数正确生成 `.property("kind", ...)` / `.parent_id(...)`

2. **Demo 编译**：
   - `cargo build --workspace` 通过
   - 无 `unused import` / `dead code` 告警（清理删除文件的所有引用）

3. **运行时验证**：
   - 窗口启动显示菜单栏（主题切换 / 语言切换可点击）
   - 状态栏显示 "就绪" 文本
   - 活动栏「示例」图标可点击切换面板
   - 案例树显示 3 分类 + 4 案例节点（counter/two-way/button/i18n）
   - 点击案例叶子节点 → 打开对应 Tab → 显示案例内容
   - 语言切换 → 菜单/状态/树标签刷新
   - 主题切换 → 明暗切换

4. **测试**：
   - `cargo test --workspace` 全部通过（288 现有测试无回归）
