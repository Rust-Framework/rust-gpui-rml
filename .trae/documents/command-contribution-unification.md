# 命令贡献统一（ICommand : IContribution）实施计划

## Summary

将 `ICommand` 统一为 `IContribution` 的子 trait——命令本身即是贡献点。`#[contribute]` 宏不再自动生成 `impl IContribution`，改为编译期校验目标类型已实现 `IContribution`。宏参数精简为固定 5 项 + flag + 任意扩展属性。封装 `CallContext` 作为命令方法参数，提供 `Window`/`App` 能力。菜单叶子项手写 `impl ICommand`，消除 `MainWindow.menu_commands` 侧表。

## Current State Analysis

### 现状问题

1. **`ICommand` 与 `IContribution` 割裂**：`ICommand: Send + Sync + 'static`，`IContribution: Send + Sync + Any`，两者无继承关系。菜单贡献是纯元数据（`IContribution`），命令逻辑侧表在 `MainWindow.menu_commands: HashMap<String, Arc<dyn ICommand>>` 中维护，贡献与行为分离。

2. **`#[contribute]` 宏过度生成**：自动生成 `impl IContribution`（含 `name()`/`description()`/`icon()`），用户无法动态实现这些方法。宏参数过多（`name`/`description`/`icon`/`kind`/`slot`/`placement`/`mode`），职责不清。

3. **命令参数弱类型**：`ICommand::execute(&self, parameter: &dyn Any, cx: &mut App)`——`parameter` 几乎不用，`cx` 只是 `App` 无 `Window` 访问能力。

### 关键文件清单

| 文件 | 角色 |
|---|---|
| `crates/core/src/command.rs` | `ICommand` trait + `RelayCommand` |
| `crates/core/src/contribution.rs` | `IContribution`/`IContributionHost`/`IContributionRegistry`/`ContributionOptions` |
| `crates/core/src/prelude.rs` | 核心 prelude 导出 |
| `crates/app/src/contribution/host_handle.rs` | `EntityHostHandle` + `HostOp` + `drain_host_ops` |
| `crates/app/src/contribution/registry.rs` | `ContributionRegistry` 实现 |
| `crates/macros/src/contribute.rs` | `#[contribute]` 宏展开 |
| `crates/ui/src/components/menu.rs` | `MenuItem`/`MenuBar` + `on_click` 调 `cmd.execute` |
| `demo/src/shell/shell_chrome.rs` | `map_menu_items(entries, commands)` 投影层 |
| `demo/src/shell/main_window.rml.rs` | `MainWindow` host，持有 `menu_commands` HashMap |
| `demo/src/shell/menu_shell_contribs.rs` | 14 个菜单贡献 struct（5 个 submenu root + 8 个 leaf + 1 个 `MenuHelpDocs`） |
| `demo/src/shell/activity_panel.rml.rs` | `ActivityPanel` 视觉贡献 + host |
| `demo/src/cases/*.rml.rs` | 12 个案例视觉贡献 + 1 个 `StatusReady` 状态贡献 |
| `crates/engine/src/build/contribution_generator.rs` | build.rs 扫描 `#[contribute]` 按 `host_id` 分组 |

### 工具链

- `rust-toolchain.toml`: `channel = "nightly"`
- Rust trait upcasting（`dyn SubTrait` → `dyn SuperTrait`）自 1.86 稳定，nightly 可用

## Proposed Changes

### 第 1 步：`crates/core/src/command.rs` — `CallContext` + `ICommand : IContribution`

**改什么**：新增 `CallContext` 结构体；`ICommand` 改为继承 `IContribution`；`execute`/`can_execute` 签名改用 `&mut CallContext`；`RelayCommand` 补 `IContribution` dummy impl。

**为什么**：命令即贡献——`ICommand: IContribution` 使命令可经 `register_command` 路由到 host。`CallContext` 统一提供 `Window`/`App`，消除 `&dyn Any` 弱类型参数。`RelayCommand` 保留用于 ViewModel 字段绑定（`command={field}`），dummy `id`/`name` 不影响贡献路由。

**怎么改**：

```rust
use gpui::{App, SharedString, Window};

/// 命令执行上下文——封装 `Window` + `App`，替代 `(&dyn Any, &mut App)` 弱类型参数。
pub struct CallContext<'a> {
    pub window: &'a mut Window,
    pub app: &'a mut App,
}

impl<'a> CallContext<'a> {
    pub fn new(window: &'a mut Window, app: &'a mut App) -> Self {
        Self { window, app }
    }
}

/// 命令贡献 trait（对齐 WPF `ICommand`，继承 `IContribution`——命令本身是贡献点）。
///
/// 实现方需同时实现 `IContribution`（id/name/description/icon）和 `ICommand`（execute/can_execute）。
/// `#[contribute(command, ...)]` 宏编译期校验目标已实现 `IContribution`，路由到 `register_command`。
pub trait ICommand: IContribution {
    fn execute(&self, ctx: &mut CallContext);
    fn can_execute(&self, _ctx: &mut CallContext) -> bool { true }
}
```

`RelayCommand` 适配：
- 补 `impl IContribution for RelayCommand`（`id()` 返回 `"__relay__"`，`name()` 返回 `SharedString::default()`）
- `impl ICommand for RelayCommand`：`execute` 调 `(self.action)(ctx.app)`，`can_execute` 调 `can_run`
- 保留 `new<T, F>`/`action<F>`/`can_when<F>` 构造器不变
- 测试用 `AlwaysEnabled`/`AlwaysDisabled` 补 `IContribution` impl，`can_execute`/`execute` 签名改用 `&mut CallContext`

### 第 2 步：`crates/core/src/contribution.rs` — `add_command` + `register_command` + 移除 `slot` 字段

**改什么**：
1. `ContributionOptions` 移除 `slot` 字段，`effective_slot()` 只读 `properties["kind"]`
2. `IContributionHost` 新增 `fn add_command(&self, _command: Arc<dyn ICommand>, _options: ContributionOptions) {}`（默认空实现）
3. `IContributionRegistry` 新增 `fn register_command(&self, host_id: &str, command: Arc<dyn ICommand>, options: ContributionOptions);`

**为什么**：`slot` 与 `kind` 语义重复，统一走 `properties["kind"]` 简化数据模型。`add_command`/`register_command` 为命令贡献提供独立路由路径（与 `add`/`add_visual` 并列），host 按 `add_command` override 接收命令贡献。

**怎么改**：

```rust
// ContributionOptions 移除 slot 字段
#[derive(Debug, Clone, Default)]
pub struct ContributionOptions {
    pub order: i32,
    pub parent_id: Option<SharedString>,
    pub group: Option<SharedString>,
    pub properties: HashMap<SharedString, SharedString>,
}

impl ContributionOptions {
    // 移除 slot() builder
    pub fn effective_slot(&self) -> Option<&str> {
        self.properties.get("kind").map(|s| s.as_ref())
    }
    // ... 其余 builder 不变
}

// IContributionHost 新增 add_command
pub trait IContributionHost: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: ContributionOptions) {}
    fn add_visual(&self, _contribution: Arc<dyn IVisualContribution>, _options: ContributionOptions) {}
    fn add_command(&self, _command: Arc<dyn ICommand>, _options: ContributionOptions) {}
    fn remove(&self, _contribution_id: &str) {}
}

// IContributionRegistry 新增 register_command
pub trait IContributionRegistry: Send + Sync {
    fn add_host(&self, host: Arc<dyn IContributionHost>);
    fn remove_host(&self, host_id: &str);
    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions);
    fn register_visual(&self, host_id: &str, contribution: Arc<dyn IVisualContribution>, options: ContributionOptions);
    fn register_command(&self, host_id: &str, command: Arc<dyn ICommand>, options: ContributionOptions);
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
```

需在 `contribution.rs` 顶部 `use crate::command::ICommand;`。

### 第 3 步：`crates/core/src/prelude.rs` — 导出 `CallContext`

```rust
pub use crate::command::{CallContext, ICommand, RelayCommand};
```

### 第 4 步：`crates/app/src/contribution/host_handle.rs` — `HostOp::AddCommand` + `drain_host_ops` 新分支

**改什么**：`HostOp` 新增 `AddCommand(Arc<dyn ICommand>, ContributionOptions)` 变体；`EntityHostHandle` 实现 `add_command`；`drain_host_ops` 新增 `AddCommand` 分支。

```rust
use rml_core::command::ICommand;

pub enum HostOp {
    Add(Arc<dyn IContribution>, ContributionOptions),
    AddVisual(Arc<dyn IVisualContribution>, ContributionOptions),
    AddCommand(Arc<dyn ICommand>, ContributionOptions),
    Remove(String),
}

impl<T: 'static> IContributionHost for EntityHostHandle<T> {
    // ... add/add_visual/remove 不变
    fn add_command(&self, command: Arc<dyn ICommand>, options: ContributionOptions) {
        let _ = self.tx.send(HostOp::AddCommand(command, options));
    }
}

pub fn drain_host_ops<T: IContributionHost>(rx: &flume::Receiver<HostOp>, host: &T) {
    for op in rx.try_iter() {
        match op {
            HostOp::Add(c, o) => host.add(c, o),
            HostOp::AddVisual(c, o) => host.add_visual(c, o),
            HostOp::AddCommand(c, o) => host.add_command(c, o),
            HostOp::Remove(id) => host.remove(&id),
        }
    }
}
```

### 第 5 步：`crates/app/src/contribution/registry.rs` — 实现 `register_command`

```rust
impl IContributionRegistry for ContributionRegistry {
    // ... add_host/remove_host/register/register_visual/unregister 不变

    fn register_command(
        &self,
        host_id: &str,
        command: Arc<dyn ICommand>,
        options: ContributionOptions,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add_command(command, options);
        } else {
            let _ = (host_id, command, options);
        }
    }
}
```

### 第 6 步：`crates/macros/src/contribute.rs` — 宏重构

**改什么**：
1. 参数精简：固定 `host_id`/`id`/`parent_id`/`order`/`group` + flag `command`/`visual` + 任意 `key = "string"` → `properties`
2. 拒绝 `name`/`description`/`icon`（compile_error，提示手写 impl）
3. 不再自动生成 `impl IContribution`——改为编译期断言 `T: IContribution`
4. `command` flag → 生成 `register_command` 调用
5. `visual` flag（或 `#[component]` 叠加）→ 保留自动 `impl IVisualContribution`（仅 `render` 方法）+ 生成 `register_visual` 调用
6. 生成 `pub const CONTRIBUTION_ID: &str = #id;`

**为什么**：用户要求宏只做路由 + 编译期校验，不替用户实现接口。`name`/`description`/`icon` 可能动态需求，应在 impl 中手写。`kind` 等非固定参数统一进 `properties`。

**怎么改**：

参数解析逻辑：
```rust
struct ContributeArgs {
    host_id: LitStr,           // 必需
    id: LitStr,                // 必需
    parent_id: Option<LitStr>, // 可选
    order: Option<syn::LitInt>,// 可选
    group: Option<LitStr>,     // 可选
    command: bool,             // flag
    visual: bool,              // flag
    properties: Vec<(String, String)>, // 任意 key="value" 扩展
}
```

解析规则：
- `host_id = "..."` / `id = "..."` / `parent_id = "..."` / `group = "..."` → 字符串字面量，固定字段
- `order = N` → 整数字面量，固定字段
- `command` / `visual` → Path 形式 flag
- `name = ...` / `description = ...` / `icon = ...` → compile_error（"must be hand-written in impl IContribution"）
- 其他 `key = "string_literal"` → `properties.push((key, value))`
- 其他 `key = <non-string>` → compile_error（"extra properties must be string literals"）

生成代码（非 visual、非 command）：
```rust
quote! {
    #(#items)*

    impl #struct_name {
        pub const CONTRIBUTION_ID: &'static str = #id;
    }

    // 编译期断言：目标必须实现 IContribution（用户手写）
    const _: () = {
        fn assert_contribution<T: rml_core::contribution::IContribution>() {}
        fn check() { assert_contribution::<#struct_name>(); }
    };

    pub fn #register_fn(cx: &mut gpui::App) {
        use rml_app::contribution::ContributionRegistryExt;
        cx.get_contribution_registry().register(
            #host_id,
            std::sync::Arc::new(#struct_name::default()),
            rml_core::contribution::ContributionOptions::new()
                #parent_id
                #order
                #group
                #(#properties)*,
        );
    }
}
```

生成代码（command flag）：
```rust
// 同上结构，但 register 调用改为 register_command
cx.get_contribution_registry().register_command(
    #host_id,
    std::sync::Arc::new(#struct_name::default()),
    rml_core::contribution::ContributionOptions::new()
        #parent_id
        #order
        #group
        #(#properties)*,
);
```

生成代码（visual flag 或 `#[component]` 叠加）：
```rust
// 保留 impl IVisualContribution（仅 render 方法，用户仍需手写 impl IContribution）
impl rml_core::contribution::IVisualContribution for #struct_name {
    fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        let entity = rml_app::contribution::get_or_create_entity::<#struct_name>(cx);
        entity.update(cx, |this, ctx| {
            this.render(window, ctx).into_any_element()
        })
    }
}

// register 调用走 register_visual
```

`properties` 生成：每个 `(key, value)` 生成 `.property(#key, #value)`。

### 第 7 步：`crates/ui/src/components/menu.rs` — `on_click` 改用 `CallContext`

**改什么**：两处 `cmd.execute(&(), cx)` 改为 `cmd.execute(&mut CallContext::new(_window, cx))`。

**位置**：
- L330: `btn.on_click(move |_, _window, cx| cmd.execute(&mut CallContext::new(_window, cx)));`
- L410: `pmi.on_click(move |_, _window, cx| cmd.execute(&mut CallContext::new(_window, cx)));`

**导入**：`use rml_core::command::CallContext;`（在文件顶部 `use` 区添加）。

### 第 8 步：`demo/src/shell/shell_chrome.rs` — 双列表 `map_menu_items`

**改什么**：
1. 新增 `CommandEntry = (Arc<dyn ICommand>, ContributionOptions)` 类型别名
2. `map_menu_items` 签名改为 `(entries: &[ContribEntry], commands: &[CommandEntry]) -> MenuItems`
3. 合并两类条目按 `parent_id` 建树，命令条目挂接 `cmd.clone()` 到 `MenuItem::command()`

**为什么**：`ICommand : IContribution` 但 host 分两类存储——submenu root（`IContribution` only）走 `add`，leaf command（`ICommand`）走 `add_command`。投影层合并建树时需区分有无命令。

```rust
pub type CommandEntry = (Arc<dyn ICommand>, ContributionOptions);

pub fn map_menu_items(
    entries: &[ContribEntry],
    commands: &[CommandEntry],
) -> MenuItems {
    // 收集所有 menu slot 条目（含 submenu root 和 leaf）
    struct Node {
        id: String,
        name: SharedString,
        order: i32,
        command: Option<Arc<dyn ICommand>>,
    }

    let mut all: Vec<Node> = Vec::new();
    for (c, o) in entries.iter().filter(|(_, o)| o.effective_slot() == Some("menu")) {
        all.push(Node {
            id: c.id().to_string(),
            name: c.name(),
            order: o.order,
            command: None,
        });
    }
    for (c, o) in commands.iter().filter(|(_, o)| o.effective_slot() == Some("menu")) {
        // ICommand : IContribution，可调 IContribution 方法（trait upcasting）
        all.push(Node {
            id: c.id().to_string(),
            name: c.name(),
            order: o.order,
            command: Some(c.clone()),
        });
    }

    // 按 parent_id 建树
    let mut by_parent: HashMap<Option<String>, Vec<&Node>> = HashMap::new();
    for node in &all {
        // parent_id 从 ContributionOptions 取——需在收集时一并取出
        // （实际实现中 Node 应携带 parent_id）
    }
    // ... 递归 build_children，有 command 的挂 item.command(cmd)
}
```

实现细节：`Node` 需携带 `parent_id: Option<String>`。`build_children` 递归时，若 `node.command.is_some()` 则 `item.command(cmd)`。

### 第 9 步：`demo/src/shell/main_window.rml.rs` — 移除 `menu_commands`，新增 `command_entries`

**改什么**：
1. 移除 `menu_commands: HashMap<String, Arc<dyn ICommand>>` 字段
2. 新增 `command_entries: std::sync::RwLock<Vec<CommandEntry>>` 字段
3. `impl IContributionHost for MainWindow` override `add_command`：push 到 `command_entries`
4. `on_loaded` 移除所有 `menu_commands.insert(...)` 代码块（L108-L151）
5. `refresh_shell_chrome` 改为 `map_menu_items(&entries, &command_entries)`

**为什么**：命令贡献经 `register_command` → `add_command` → `command_entries` 入 host，不再需要手写 HashMap 侧表。菜单点击行为由贡献 struct 的 `impl ICommand` 承载。

```rust
impl IContributionHost for MainWindow {
    // ... id/add/add_visual/remove 不变

    fn add_command(&self, command: Arc<dyn ICommand>, options: ContributionOptions) {
        self.command_entries.write().unwrap().push((command, options));
    }
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self) {
        let entries = self.entries.read().unwrap();
        let commands = self.command_entries.read().unwrap();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries, &commands);
    }
}
```

### 第 10 步：`demo/src/shell/menu_shell_contribs.rs` — 手写 `IContribution` + `ICommand`

**改什么**：14 个 struct 全部：
1. `#[contribute]` 移除 `name = "..."`、`kind = "menu"` 改为 `kind = "menu"`（自动进 `properties`）
2. 手写 `impl IContribution`（`id()` 返回 `Self::CONTRIBUTION_ID`，`name()` 返回 `t_static(...)`）
3. 8 个叶子 struct 加 `command` flag + 手写 `impl ICommand`（`execute`/`can_execute`）

**叶子项**（加 `command` flag + `impl ICommand`）：

| struct | id | 原 RelayCommand 行为 |
|---|---|---|
| `MenuFileNew` | `menu.file.new` | `open_case("welcome")` |
| `MenuFileOpen` | `menu.file.open` | `open_case("components.button")` |
| `MenuFileExit` | `menu.file.exit` | `cx.quit()` |
| `MenuThemeToggleContrib` | `menu.theme_toggle` | `apply_toggle_theme` |
| `MenuLangEnContrib` | `menu.lang_en` | `apply_switch_en` |
| `MenuHelpGuide` | `menu.help.guide` | `open_case("components.menu.dropdown")` |
| `MenuHelpAbout` | `menu.help.about` | `open_case("welcome")` |
| `MenuOpenFeaturesContrib` | `menu.open_features` | `open_case("components.menu.features")` |

**问题**：`execute` 需访问 `MainWindow` 的方法（`open_case`/`apply_toggle_theme` 等）。`ICommand::execute(&self, ctx: &mut CallContext)` 中 `self` 是贡献 struct（如 `MenuFileNew`），不是 `MainWindow`。

**解决方案**：命令贡献 struct 通过 `DemoShellHost` 全局获取 `WeakEntity<MainWindow>`，在 `execute` 中 upgrade + update 调用 MainWindow 方法。

```rust
use rml_core::command::{CallContext, ICommand};
use rml_core::contribution::IContribution;
use rml_core::i18n::t_static;
use gpui::SharedString;

#[contribute(
    host_id = "demo.shell",
    id = "menu.file.new",
    parent_id = "menu.file",
    order = 0,
    kind = "menu",
    command,
)]
#[derive(Default)]
pub struct MenuFileNew;

impl IContribution for MenuFileNew {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("menu.file_new").into() }
}

impl ICommand for MenuFileNew {
    fn execute(&self, ctx: &mut CallContext) {
        if let Some(host) = ctx.app.try_global::<crate::shell::DemoShellHost>()
            .and_then(|h| h.0.upgrade())
        {
            host.update(ctx.app, |this, cx| {
                this.open_case("welcome".to_string(), cx);
            });
        }
    }
}
```

`MenuFileExit`（无 ViewModel 访问，直接 `cx.quit()`）：
```rust
impl ICommand for MenuFileExit {
    fn execute(&self, ctx: &mut CallContext) {
        ctx.app.quit();
    }
}
```

**submenu root 项**（仅 `impl IContribution`，无 `command` flag）：

`MenuFileRoot`/`MenuViewRoot`/`MenuHelpRoot`/`MenuHelpDocs`/`MenuHelpCases` 手写 `impl IContribution`，`name()` 返回 `t_static(...)`。

### 第 11 步：`demo/src/shell/activity_panel.rml.rs` — 手写 `IContribution`

**改什么**：
1. `#[contribute]` 移除 `name = "..."`、`icon = IconName::BookOpen`、`kind = "activity"`
2. `kind = "activity"` 保留（自动进 `properties`）
3. 手写 `impl IContribution for ActivityPanel`（`id`/`name`/`icon`）

```rust
#[contribute(
    host_id = "demo.shell",
    id = "samples",
    kind = "activity",
    order = 0,
)]
// ... 其余不变

impl IContribution for ActivityPanel {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.samples").into() }
    fn icon(&self) -> Option<SharedString> { Some("BookOpen".into()) }
}
```

### 第 12 步：`demo/src/cases/*.rml.rs` + `status_bar_case.rml.rs` — 手写 `IContribution`

**改什么**：12 个案例视觉贡献 + `StatusReady` 状态贡献：
1. `#[contribute]` 移除 `name = "..."`（`kind = "case"`/`kind = "status"` 保留自动进 `properties`）
2. 手写 `impl IContribution`（`id`/`name`）

**涉及文件**：
- `accordion_case.rml.rs` — `AccordionCase`
- `button_case.rml.rs` — `ButtonCase`
- `counter_case.rml.rs` — `CounterCase`
- `i18n_case.rml.rs` — `I18nCase`
- `menu_context_case.rml.rs` — `MenuContextCase`
- `menu_custom_case.rml.rs` — `MenuCustomCase`
- `menu_dropdown_case.rml.rs` — `MenuDropdownCase`
- `menu_editor_case.rml.rs` — `MenuEditorCase`
- `menu_features_case.rml.rs` — `MenuFeaturesCase`
- `slot_case.rml.rs` — `SlotCase`
- `status_bar_case.rml.rs` — `StatusBarCase`（case） + `StatusReady`（status）
- `two_way_case.rml.rs` — `TwoWayCase`

每个文件添加：
```rust
impl IContribution for XxxCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.xxx.title").into() }
}
```

### 第 13 步：`crates/engine/src/build/contribution_generator.rs` — 无需改动

build.rs 扫描 `#[contribute]` 仍按 `host_id` 分组生成 `__rml_register_*` 调用。宏内部决定走 `register`/`register_visual`/`register_command`，build.rs 不感知。

## Assumptions & Decisions

1. **trait upcasting 可用**：项目用 nightly，`dyn ICommand` → `dyn IContribution` upcasting 自 Rust 1.86 稳定。`map_menu_items` 中对 `Arc<dyn ICommand>` 调 `id()`/`name()` 依赖此特性。

2. **`RelayCommand` 保留用于 ViewModel 字段绑定**：不作为贡献注册（dummy `id`/`name`）。用户需注册命令贡献时手写 struct 实现 `IContribution` + `ICommand`。

3. **`icon` 不再是宏参数**：`IContribution::icon()` 由用户在 impl 中手写。若 host 需 icon 元数据，用 `property("icon", "BookOpen")` 传入 `properties`。

4. **`slot` 字段移除**：`ContributionOptions` 只保留 `order`/`parent_id`/`group`/`properties`。`effective_slot()` 读 `properties["kind"]`。原 `.slot()` builder 移除。

5. **`CallContext` 无 parameter**：命令贡献 struct 自身携带状态（`self`），无需外部 `&dyn Any` 参数。`RelayCommand` 闭包已捕获上下文，也不需 parameter。

6. **命令贡献访问 MainWindow**：通过 `DemoShellHost` 全局（`WeakEntity<MainWindow>`）在 `execute` 中 upgrade + update，与 `ActivityPanel::on_case_activate` 现有模式一致。

7. **`#[contribute]` 宏生成 `CONTRIBUTION_ID` 常量**：`pub const CONTRIBUTION_ID: &str = #id;`，用户在 `impl IContribution::id()` 中返回 `Self::CONTRIBUTION_ID` 避免 id 重复。

8. **视觉贡献保留自动 `impl IVisualContribution`**：`render` 方法是框架胶水（`get_or_create_entity` + `entity.update`），不应手写。用户只需手写 `impl IContribution`。

## Verification

1. `cargo build -p rust-rml-core` — 核心层编译通过
2. `cargo build -p rust-rml-macros` — 宏编译通过
3. `cargo build -p rust-rml-app` — 应用层编译通过
4. `cargo build -p rust-rml-ui` — UI 层编译通过（`menu.rs` CallContext 适配）
5. `cargo build -p rust-rml-demo` — demo 编译通过（所有贡献手写 impl）
6. `cargo test --workspace` — 全部测试通过
7. 手动运行 demo 验证：菜单点击（File > New/Open/Exit、View > Theme Toggle/Lang EN、Help > Docs > Guide/About、Help > Cases > Features）行为正确

## 实施顺序

1. 第 1-3 步（核心层：`command.rs` + `contribution.rs` + `prelude.rs`）
2. 第 4-5 步（应用层：`host_handle.rs` + `registry.rs`）
3. 第 6 步（宏：`contribute.rs`）
4. 第 7 步（UI：`menu.rs`）
5. 第 8-9 步（demo shell：`shell_chrome.rs` + `main_window.rml.rs`）
6. 第 10 步（demo 菜单贡献：`menu_shell_contribs.rs`）
7. 第 11-12 步（demo 其余贡献：`activity_panel.rml.rs` + `cases/*.rml.rs`）
8. 验证（build + test + 运行）
