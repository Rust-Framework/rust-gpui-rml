# 开发者指南

> **本节目标**：从项目搭建到 MVVM + 贡献点 + 自定义组件，走通 RML 应用的完整开发流程。示例均来自 `demo/` 可运行代码。

## 1. 项目搭建

### 1.1 依赖与 build.rs

`demo/Cargo.toml` 引入 `rust-rml-engine`（别名 `rml`）、`rust-rml-app`、`rust-rml-ui`、`gpui`、`gpui-component`。

`demo/build.rs`：

```rust
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .assets("assets", true)
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

- `.scan_dir("src")` — 扫描 `.rml` 与同名 `.rml.rs`
- `.assets("assets", true)` — CSS/i18n 嵌入二进制

### 1.2 应用入口

`demo/src/main.rs`：

```rust
#[rml::main]
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<shell::MainWindow>()
        .run::<app::Startup>();
}
```

`#[rml::main]` 注入资源初始化；`RmlApplication` 管理主窗口生命周期。

### 1.3 文件三件套

| 文件 | 职责 |
|------|------|
| `*.rml` | 声明式 UI（根节点 + 布局 + 绑定） |
| `*.rml.rs` | ViewModel：字段、`#[command]`、`#[computed]`、生命周期 |
| `main.rs` | 应用启动 |

## 2. 窗口 + RML + Code-Behind

### 2.1 主窗口 RML

`demo/src/shell/main_window.rml` 使用 `<tab_window>` 根，声明插槽与内容：

```html
<tab_window title="RML Showcase" width="1100" height="720" ...>
    <slot_left>...</slot_left>
    <slot_menu><menu items={menu_items} /></slot_menu>
    <slot_footer><status_bar items={status_items} /></slot_footer>
    <div class="case-host">...</div>
</tab_window>
```

根节点类型决定 codegen 输出，见 [window-roots.md](../06-components/reference/window-roots.md)。

### 2.2 Code-Behind 结构

```rust
#[window]
#[derive(Default)]
pub struct MainWindow {
    // 普通字段 — RML {field} 绑定
    active_panel_id: String,
    show_chrome: bool,
    // MVVM 控件数据
    activity_panels: Vec<Arc<dyn IActivityPanel>>,
    menu_items: Vec<Arc<dyn IMenuItem>>,
    status_items: Vec<Arc<dyn IStatusBarItem>>,
    // Stateful 子组件
    case_activity_panel: Option<Entity<CaseActivityPanel>>,
    // 子组件 Entity
    welcome_case: Option<Entity<WelcomeCase>>,
}
```

`#[window]` 注入 `__rml_window_handle` 等窗口字段；`#[component]` 用于可复用片段（如 `ButtonCase`）。

## 3. MVVM：字段、计算属性、命令

### 3.1 字段与单向绑定

RML `{count}` 读取 ViewModel 字段。字段变更后需 `cx.notify()`（`#[command]` 宏通常自动处理）。

### 3.2 `#[computed]`

派生 UI 数据，RML 中当字段使用：

```rust
#[computed]
pub fn tab_bar_items(&self) -> Vec<TabItem> {
    self.open_tabs.iter().map(|t| TabItem::new(t.title.as_str())).collect()
}
```

```html
<tab_window tabs={tab_bar_items} selected_tab={selected_tab} ...>
```

### 3.3 `#[command]`

事件处理函数，RML `onclick={on_click}` 或 `on_panel_change="on_panel_change"`：

```rust
#[command]
pub fn on_panel_change(&mut self, id: &SharedString, cx: &mut Context<Self>) {
    let new_id = id.to_string();
    if self.active_panel_id == new_id {
        self.active_panel_id = String::new();
    } else {
        self.active_panel_id = new_id;
    }
    self.refresh_shell_bindings(cx);
}
```

### 3.4 双向绑定（input）

ViewModel 普通字段 + RML `model` 指令：

```html
<input model={name} placeholder={t("demo.name_placeholder")} />
```

```rust
pub name: String,
pub age: String,

#[computed]
pub fn profile_summary(&self) -> String {
    format!("{} / {}", self.name, self.age)
}
```

见 `demo/src/cases/two_way_case.rml`。

## 4. 贡献点 → MVVM 绑定

Demo 通过 `shell_chrome::map_shell_chrome` 将贡献条目映射为 Shell 控件数据。详见 [贡献点架构](../09-architecture/contribution-system.md)。

### 4.1 数据流

```
#[contribute] register → ContributionRegistry → map_shell_chrome（demo）→ RML 绑定
```

### 4.2 ActivityBar / StatusBar / Menu

MainWindow 在 `refresh_bindings` 中调用 `crate::shell::shell_chrome::map_shell_chrome`：

```rust
let bindings = map_shell_chrome(MainWindow::ID, cx, &self.menu_commands);
self.activity_panels = bindings.activity_panels;
self.status_items = bindings.status_items;
self.menu_items = bindings.menu_items;
```

RML：`<ActivityBar panels={activity_panels} />`、`<status_bar items={status_items} />`、`<menu-bar items={menu_items} />`。

### 4.3 Tree（案例树）

`CaseActivityPanel` 使用 `map_case_tree_items(MainWindow::ID, cx)`，并通过 `subscribe_host_changes` 自动刷新（非 `observe_global`）。

### 4.4 案例激活

`register_case_activation` + `activate_case`（`demo/shell/case_activation.rs`）；案例内容路由由 `CaseHost` 承担。

## 5. 自定义 `#[component]` 工作流

### 5.1 案例组件：`#[contribute]` + `#[component]`

Demo 案例除 UI 逻辑外，还向案例树注册元数据。在 struct 上**先写 `#[contribute]`，再写 `#[component]`**（宏展开顺序要求 contribute 在外层）：

```rust
#[contribute(
    host = "demo.shell",
    id = "components.menu.context",
    name = "case.menu.context.title",  // 用 case.* 标题 key，不用 tree.*
    kind = "case",
    group = "menu",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct MenuContextCase {
    pub last_action: String,
}
```

贡献由 `RmlApplication::bootstrap_runtime` 自动注册，**无需**在 `on_launch` 手动调用。详见 [贡献点架构](../09-architecture/contribution-system.md)。

菜单相关 Demo 案例：

| 结构体 | id | order |
|--------|-----|-------|
| `MenuContextCase` | `components.menu.context` | 16 |
| `MenuDropdownCase` | `components.menu.dropdown` | 17 |
| `MenuEditorCase` | `components.menu.editor` | 18 |
| `MenuFeaturesCase` | `components.menu.features` | 19 |
| `MenuCustomCase` | `components.menu.custom` | 20 |

### 5.2 普通子组件

1. 创建 `feature.rml`（根为 `<component>`）+ `feature.rml.rs`
2. 在 struct 上标注 `#[component]`（及需要的 `#[command]`）
3. 父窗口 RML 中 `<Feature />` 引用
4. 父 ViewModel 在 `on_loaded` 中 `cx.new(|_| Feature::default())` 并持有 `Option<Entity<Feature>>`

子组件案例：`demo/src/cases/button_case.rml` 被 `main_window.rml` 引用为 `<ButtonCase />`。

对话框：`login_dialog.rml` 根为 `<dialog>`，从父窗口 `LoginDialog::default().open(window, cx)` 打开。

## 6. 生命周期要点

### 6.1 `on_loaded`

集中初始化 Stateful 组件与子 Entity：

```rust
impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Shell 绑定：首次 render 时 __rml_attach_contribution_bindings（需 bindings =）
        self.case_host.get_or_insert_with(|| cx.new(|_| CaseHost::default()));
        register_case_activation(/* demo/shell/case_activation */);
    }
}
```

### 6.2 常见陷阱

| 问题 | 原因 | 解决 |
|------|------|------|
| CaseHost / 子组件 panic | 未 `cx.new` 子 Entity | `on_loaded` 中 `get_or_insert_with` |
| Shell 数据不刷新 | 未使用 `#[contributehost]` | 标注 host 窗口；框架自动 subscribe |
| `<Input model>` 编译失败 | `model` 只能用于小写 `input` | 改用 `<input model={...}>` |
| 用了未注册标签 | 如 `<Modal>` | 查 [reference/INDEX.md](../06-components/reference/INDEX.md) |

完整清单见 [避坑清单](../11-cookbook/pitfall-checklist.md)。

## 7. 推荐阅读路径

1. [快速开始](./quick-start.md) — 最小计数器
2. 本文 — 端到端流程
3. [组件参考](../06-components/reference/INDEX.md) — 查标签 API
4. [MVVM 实践](../09-architecture/mvvm-practice.md) — 架构细节
5. `demo/src/` — 可运行参考实现
