# Menu/StatusBar MVVM 数据绑定 + each 插槽模板化方案

## 摘要

将 `menu-bar` / `status-bar` 从框架内置 `IMenuItem`/`IStatusBarItem` 数据结构 + `items={...}` 绑定模式，重构为 **WPF 风格的纯容器 + `each` 指令模板化**模式。框架不再定义菜单/状态栏数据结构，业务自行定义 `MenuViewModel`/`StatusViewModel`，通过 RML `<menu-item each={m in menus}>` / `<status-item each={s in status}>` 内联迭代实现数据驱动 UI。同时引入 `RelayCommand` 字段简化菜单命令实现，消除 `shell_chrome.rs` 与 `menu_shell_contribs.rs` 样板代码。

## 当前状态分析

### 框架侧（crates/）

**菜单 codegen**（`crates/engine/src/compiler/menu/`）:
- `menu_bar.rs::gen_menu_bar` 有两条路径：
  - `items={expr}` 绑定路径（L23-32）：生成 `MenuBar::new(...).items(self.{expr}.clone())`
  - 声明式路径（L34-122）：编译期遍历 `<MenuItem>` 子节点，生成 `menu_bar_button` + `dropdown_menu` 闭包
  - **不支持 `each` 指令**——无法在 `<menu-bar>` 内运行时迭代
- `item.rs::gen_command_closure`（L212-222）：生成 `entity.update(app, |this, _cx| this.{field}.clone())`，**不支持 loop_var**（无法访问 `m.command`）
- `item.rs::gen_menu_item_stmt`：`icon` 仅支持 `static_attr`，**不支持 bind**

**MenuBar 组件**（`crates/ui/src/components/menu.rs`）:
- 定义了 `IMenuItem` trait（L71-98）+ `MenuItem` struct（L100-213）——框架内置数据结构
- `MenuBar` 持有 `items: Vec<Arc<dyn IMenuItem>>` + `entry_children`，二选一渲染
- `build_popup_menu_from_items`（L353-424）：递归构建 PopupMenu 的 MVVM 专用路径

**StatusBar 组件**（`crates/ui/src/components/status_bar.rs`）:
- 定义了 `IStatusBarItem` trait（L21-26）+ `StatusBarItem` struct（L29-60）——仅支持 `SharedString` 文本
- `StatusBar` 持有 `items: Vec<Arc<dyn IStatusBarItem>>`，委托 `NativeStatusBar` 渲染
- **不支持富内容**（`NativeStatusBar` 仅接受 `SharedString`）

**tags/props 注册表**:
- `tags.rs`: `MenuBar`/`menu`/`StatusBar`/`status-bar` 均已注册
- `props_registry.rs`: `("MenuBar", &["items"])`, `("menu", &["items"])`, `("StatusBar", &["items"])`
- `setters.rs`: `items` bind setter 匹配 `menu`/`MenuBar`/`StatusBar`

### 业务侧（demo/）

**MainWindow**（`demo/src/shell/main_window.rml.rs`）:
- 持有 `menus: Vec<Arc<dyn IMenuItem>>` + `status: Vec<Arc<dyn IStatusBarItem>>`
- `project_entries()` 调用 `build_menu_tree(&entries)` + `Self::build_status_items(&entries)` 投影
- 无 `RelayCommand` 字段——命令经 `menu_shell_contribs.rs` 的 11 个 `ICommand` impl 路由

**shell_chrome.rs**（74 行）:
- `build_menu_tree`：按 `parent_id` 建树，包装为 `MenuItem`
- `ContribEntry` 类型别名

**menu_shell_contribs.rs**（323 行）:
- 11 个菜单贡献 struct（`MenuFileRoot`/`MenuFileNew`/...）
- 每个叶子：`#[contribute(command, ...)]` + `impl IContribution` + `impl ICommand`
- `with_main_window` helper：`get_service::<MainWindowRef>()` + upgrade + update 样板

**main_window.rml**:
- `<menu-bar items={menus} />` + `<status-bar items={status} />`

**StatusReady**（`demo/src/cases/status_bar_case.rml.rs`）:
- 仅实现 `IContribution`（文本 `name()`），**未实现 `IVisualContribution`**

## 设计决策

1. **框架不定义数据结构**：删除 `IMenuItem`/`MenuItem`/`IStatusBarItem`/`StatusBarItem`，`MenuBar`/`StatusBar` 成为纯 `ParentElement` 容器
2. **`each` 指令内联迭代**：业务在 RML 中用 `<menu-item each={m in menus}>` / `<status-item each={s in status}>` 自定义渲染
3. **`RelayCommand` 字段**：MainWindow 持有 `Arc<dyn ICommand>` 字段（WPF MVVM 模式），`command={field}` 绑定
4. **ViewModel 由业务定义**：`MenuViewModel`/`StatusViewModel` 在 demo 层，框架不感知
5. **StatusBar 富内容**：`StatusItem` 组件接收 `IVisualContribution` 渲染的 `AnyElement`，非纯文本

## 实施步骤

### Part A: 扩展菜单 codegen 支持 `each` 指令

**目标**：让 `<menu-bar>` 内的 `<menu-item each={m in menus}>` 能运行时迭代。

#### A1: `gen_menu_bar` 增加 `each` 检测

**文件**: `crates/engine/src/compiler/menu/menu_bar.rs`

在声明式路径（L34）之前，检测 `<MenuItem>` 子节点是否带 `each` 指令：

```rust
// 新增：检测 top_items 中是否有 each 指令
let each_clause = top_items.first().and_then(|item| {
    item.directives.iter().find_map(|d| match d {
        Directive::Each(c) => Some(c.clone()),
        _ => None,
    })
});

if let Some(clause) = each_clause {
    // 运行时迭代路径
    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    child_loop_vars.push(clause.item.clone());
    
    // 为单个 menu_item 生成按钮代码（使用 child_loop_vars）
    let button_code = gen_menu_bar_button_for_item(
        top_items.first().unwrap(), // 模板项
        ctx,
        depth,
        id_counter,
        &child_loop_vars,
    )?;
    
    let iter_code = format!(
        "self.{}.iter().map(|{}| {{\n                {}\n            }})",
        clause.iterable, clause.item, button_code
    );
    
    return Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).children({})\n        }}",
        iter_code
    ));
}
```

提取 `gen_menu_bar_button_for_item` 辅助函数（从现有 L53-117 重构），支持 loop_vars 上下文中的 `label={m.name}` / `command={m.command}` / `<menu-item each={c in m.children}>` 嵌套。

#### A2: `gen_command_closure` 支持 loop_var

**文件**: `crates/engine/src/compiler/menu/item.rs` L212-222

当前生成 `entity.update(app, |this, _cx| this.{field}.clone())`。对于 loop_var 上下文（如 `command={m.command}`），需直接访问 loop_var 字段：

```rust
fn gen_command_closure(
    cmd_expr: &str,
    ctx: &CodegenCtx,
    loop_vars: &[String],  // 新增参数
) -> Result<String, CodegenError> {
    // 检测 cmd_expr 是否以 loop_var 开头（如 "m.command"）
    let loop_prefix = loop_vars.iter().find(|lv| {
        cmd_expr == **lv || cmd_expr.starts_with(&format!("{}.", lv))
    });
    
    if let Some(lv) = loop_prefix {
        // loop_var 上下文：直接访问，不经 entity.update
        let access = crate::compiler::expr::parse(cmd_expr)
            .map(|p| crate::compiler::expr::to_rust_code_with_ctx(&p, &loop_vars.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
            .unwrap_or_else(|_| format!("{}.{cmd_expr}", lv)); // fallback
        Ok(format!(
            ".on_click({{\n                        let weak = __rml_menu_weak.clone();\n                        move |_ev, window, app| {{\n                            let __rml_cmd = {access}.clone();\n                            let mut __rml_ctx = rml_core::command::CallContext::new(window, app);\n                            if __rml_cmd.can_execute(&mut __rml_ctx) {{\n                                __rml_cmd.execute(&mut __rml_ctx);\n                            }}\n                        }}\n                    }})"
        ))
    } else {
        // 原有路径：entity.update
        let is_computed = ctx.computed_methods.iter().any(|c| c == cmd_expr);
        let field_access = if is_computed {
            format!("this.{}()", cmd_expr)
        } else {
            format!("this.{}", cmd_expr)
        };
        Ok(format!(
            ".on_click({{\n                        let weak = __rml_menu_weak.clone();\n                        move |_ev, window, app| {{\n                            if let Some(entity) = weak.upgrade() {{\n                                let __rml_cmd = entity.update(app, |this, _cx| {{\n                                    {field_access}.clone()\n                                }});\n                                let mut __rml_ctx = rml_core::command::CallContext::new(window, app);\n                                if __rml_cmd.can_execute(&mut __rml_ctx) {{\n                                    __rml_cmd.execute(&mut __rml_ctx);\n                                }}\n                            }}\n                        }}\n                    }})"
        ))
    }
}
```

更新 `gen_menu_item_stmt` 中两处调用（L99, L147）传入 `loop_vars`。

#### A3: `icon` 支持 bind 属性

**文件**: `crates/engine/src/compiler/menu/item.rs`

当前 `gen_menu_item_stmt` 中 `icon` 仅用 `static_attr`（L78, L132, L145）。增加 `bind_attr` 检测：

```rust
let icon = if let Some(icon_expr) = bind_attr(elem, "icon", loop_vars, ctx, hoist)? {
    Some(format!("Some({})", icon_expr))  // 如 Some(m.icon)
} else {
    static_attr(elem, "icon").map(|i| format!("Some(rml_ui::IconName::{})", i))
};
```

#### A4: `gen_popup_menu_body` 支持嵌套 `each`

**文件**: `crates/engine/src/compiler/menu/item.rs` L21-44

在 `for item in items` 循环前，检测 items 中是否有 `each` 指令的元素，单独处理：

```rust
pub fn gen_popup_menu_body(
    items: &[&Element],
    config_elem: Option<&Element>,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    menu_param: &str,
    hoist: &MenuHoist,
) -> Result<String, CodegenError> {
    let mut lines = Vec::new();
    lines.push(format!("let mut menu = {menu_param};"));
    for line in hoist.rebind_non_copy_in_closure(ctx) {
        lines.push(line);
    }
    if let Some(config) = config_elem {
        lines.push(apply_popup_config(config)?);
    }
    for item in items {
        // 检测 each 指令
        let each_clause = item.directives.iter().find_map(|d| match d {
            Directive::Each(c) => Some(c.clone()),
            _ => None,
        });
        if let Some(clause) = each_clause {
            let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
            child_loop_vars.push(clause.item.clone());
            let stmt = gen_menu_item_stmt(item, ctx, depth, id_counter, &child_loop_vars, hoist)?;
            lines.push(format!(
                "for {} in self.{}.iter() {{\n                {}\n            }}",
                clause.item, clause.iterable, stmt
            ));
        } else {
            lines.push(gen_menu_item_stmt(item, ctx, depth, id_counter, loop_vars, hoist)?);
        }
    }
    lines.push("menu".to_string());
    Ok(lines.join("\n                "))
}
```

注意：嵌套 `each` 在 `dropdown_menu` 闭包内，需捕获外层 loop_var。由于闭包已 `move`，loop_var 会被捕获。

---

### Part B: 简化 MenuBar 组件

**目标**：删除 `IMenuItem`/`MenuItem`，`MenuBar` 成为纯 `ParentElement` 容器。

**文件**: `crates/ui/src/components/menu.rs`

1. **删除** `IMenuItem` trait（L71-98）、`MenuItem` struct（L100-213）、`build_popup_menu_from_items`（L353-424）、`render_menu_bar_from_items`（L348-350）
2. **删除** `Menu = MenuBar` 别名（L216）—— 保留 `menu` 标签在 `tags.rs` 指向 `MenuBar`
3. **简化** `MenuBar` struct：
   ```rust
   #[derive(IntoElement)]
   pub struct MenuBar {
       id: ElementId,
       entry_children: SmallVec<[AnyElement; 4]>,
       gap: f32,
       button_margin_y: f32,
       button_pad_x: f32,
       button_pad_y: f32,
   }
   ```
4. **删除** `items()` 方法
5. **简化** `RenderOnce`：仅渲染 `h_flex().id(self.id).h_full().items_center().gap(gap).children(self.entry_children)`
6. **保留** `menu_bar_button` / `styled_menu_bar_button` / `configure_menu_bar_popup` 公共函数（codegen 仍引用）
7. **删除** `use rml_core::command::{CallContext, CommandAbilityExt};` 及相关 import

---

### Part C: 重构 StatusBar

**目标**：删除 `IStatusBarItem`/`StatusBarItem`，`StatusBar` 成为纯 `ParentElement` 容器，支持富内容子节点。

**文件**: `crates/ui/src/components/status_bar.rs`

1. **删除** `IStatusBarItem` trait（L21-26）、`StatusBarItem` struct（L29-60）、`StatusBarAlign` enum（保留供 ViewModel 用，但需移到 demo 或保留为公共 enum）
2. **简化** `StatusBar` struct：
   ```rust
   #[derive(IntoElement)]
   pub struct StatusBar {
       children: SmallVec<[AnyElement; 4]>,
   }
   ```
3. **实现** `ParentElement`：
   ```rust
   impl ParentElement for StatusBar {
       fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
           self.children.extend(elements);
       }
   }
   ```
4. **简化** `RenderOnce`：用 `gpui::div()` flex 布局渲染子节点（不再委托 `NativeStatusBar`）：
   ```rust
   impl RenderOnce for StatusBar {
       fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
           gpui::div()
               .h_full()
               .w_full()
               .flex()
               .items_center()
               .gap(gpui::px(8.))
               .children(self.children)
       }
   }
   ```
5. **保留** `StatusBarAlign` enum 并 `pub use`（demo ViewModel 需要引用）

**文件**: `crates/ui/src/lib.rs`
- 确认 `StatusBar` / `StatusBarAlign` 仍 `pub use`
- **删除** `IMenuItem` / `MenuItem` 的 `pub use`（若存在）

---

### Part D: 创建 MenuViewModel / StatusViewModel

**文件**: `demo/src/shell/menu_view_model.rs`（新建）

```rust
//! 菜单视图模型 —— 解包 (IContribution, ContributionOptions) 为类型化树结构。
//!
//! 供 MainWindow.menus 集合持有，RML <menu-item each={m in menus}> 直接消费。

use std::sync::Arc;
use gpui::SharedString;
use rml_core::command::{CommandAbilityExt, ICommand};
use rml_core::contribution::{ContributionOptions, IContribution};

#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub label: SharedString,
    pub order: i32,
    /// 叶子节点携带命令引用（submenu root 为 None）
    pub command: Option<Arc<dyn ICommand>>,
    /// 子菜单（按 order 排序）
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// 从贡献条目列表构建菜单树（按 parent_id 建树，按 order 排序）。
    pub fn build_tree(entries: &[(Arc<dyn IContribution>, ContributionOptions)]) -> Vec<Self> {
        // 收集 menu 槽位贡献
        let nodes: Vec<MenuNode> = entries
            .iter()
            .filter(|(_, o)| o.effective_slot() == Some("menu"))
            .map(|(c, o)| MenuNode {
                id: c.id().to_string(),
                name: c.name(),
                order: o.order,
                parent_id: o.parent_id.as_ref().map(|s| s.to_string()),
                contribution: c.as_command().map(|_| c.clone()),
            })
            .collect();
        
        Self::build_children(None, &nodes)
    }
    
    fn build_children(parent_id: Option<&str>, nodes: &[MenuNode]) -> Vec<Self> {
        let mut siblings: Vec<&MenuNode> = nodes
            .iter()
            .filter(|n| n.parent_id.as_deref() == parent_id)
            .collect();
        siblings.sort_by_key(|n| n.order);
        
        siblings
            .into_iter()
            .map(|node| {
                let children = Self::build_children(Some(&node.id), nodes);
                let command = node.contribution.as_ref().and_then(|c| c.as_command()).map(|_| {
                    // Arc<dyn IContribution> → Arc<dyn ICommand> 经 as_command 能力查询
                    // 但 as_command 返回 &dyn ICommand，需重新构造 Arc
                    // 由于 ICommand: IContribution，可 trait upcast
                    node.contribution.clone()
                });
                MenuViewModel {
                    id: node.id.clone().into(),
                    label: node.name.clone(),
                    order: node.order,
                    command,
                    children,
                }
            })
            .collect()
    }
    
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

struct MenuNode {
    id: String,
    name: SharedString,
    order: i32,
    parent_id: Option<String>,
    contribution: Option<Arc<dyn IContribution>>,
}
```

**注意**：`Arc<dyn IContribution>` 无法直接转为 `Arc<dyn ICommand>`。`MenuViewModel.command` 字段类型改为 `Option<Arc<dyn IContribution>>`，在 RML 渲染时通过 `as_command()` 查询。或更简洁：直接持有 `Option<Arc<dyn IContribution>>` 命名为 `contribution`（对齐用户原始设计）。

修正设计（对齐用户原案 `contribution -> ICommand`）：

```rust
#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub label: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
    /// 叶子节点携带贡献引用（含 ICommand 能力），submenu root 为 None
    pub contribution: Option<Arc<dyn IContribution>>,
    pub children: Vec<MenuViewModel>,
}
```

**文件**: `demo/src/shell/status_view_model.rs`（新建）

```rust
//! 状态栏视图模型 —— 解包 (IVisualContribution, ContributionOptions) 为类型化结构。

use std::sync::Arc;
use gpui::SharedString;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};
use rml_ui::StatusBarAlign;

#[derive(Clone)]
pub struct StatusViewModel {
    pub id: SharedString,
    pub align: StatusBarAlign,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
}

impl StatusViewModel {
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("status") {
            return None;
        }
        c.as_visual()?; // 确保是视觉贡献
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
    
    pub fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        self.contribution
            .as_visual()
            .expect("StatusViewModel requires IVisualContribution")
            .render(window, cx)
    }
}

pub fn build_status_view_models(
    entries: &[(Arc<dyn IContribution>, ContributionOptions)],
) -> Vec<StatusViewModel> {
    let mut items: Vec<StatusViewModel> = entries
        .iter()
        .filter_map(|(c, o)| StatusViewModel::from_contribution(c.clone(), o.clone()))
        .collect();
    items.sort_by_key(|s| s.order);
    items
}
```

---

### Part E: 重构 MainWindow

**文件**: `demo/src/shell/main_window.rml.rs`

1. **更新 import**：移除 `IMenuItem`/`IStatusBarItem`/`StatusBarItem`/`StatusBarAlign`，新增 `MenuViewModel`/`StatusViewModel`
2. **更新 struct 字段**：
   ```rust
   #[window]
   #[contributehost(id = "demo.shell")]
   #[derive(Default)]
   pub struct MainWindow {
       pub cases: Vec<CaseViewModel>,
       pub menus: Vec<MenuViewModel>,
       pub status: Vec<StatusViewModel>,
       activities: Vec<Arc<dyn IActivityPanel>>,
       
       // RelayCommand 字段（WPF MVVM 模式）
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
       
       activity_bar: Option<gpui::Entity<ActivityBar>>,
       entries: std::sync::RwLock<Vec<ContribEntry>>,
       host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
       manager: Option<Arc<DemoWorkbenchManager>>,
       lsp_client: Option<Arc<LspClient>>,
   }
   ```
3. **更新 `on_loaded`**：在 `project_entries()` 后初始化 `RelayCommand` 字段：
   ```rust
   self.open_welcome_command = Arc::new(RelayCommand::new(cx, |this, cx| {
       this.open_case("welcome".to_string(), cx);
   }));
   self.open_button_case_command = Arc::new(RelayCommand::new(cx, |this, cx| {
       this.open_case("components.button".to_string(), cx);
   }));
   // ... 其余命令
   self.exit_command = Arc::new(RelayCommand::action(|cx| cx.quit()));
   ```
4. **更新 `project_entries`**：
   ```rust
   fn project_entries(&mut self) {
       let entries = self.entries.read().unwrap();
       self.cases = entries.iter()
           .filter_map(|(c, o)| CaseViewModel::from_contribution(c.clone(), o.clone()))
           .collect();
       self.menus = MenuViewModel::build_tree(&entries);
       self.status = build_status_view_models(&entries);
       self.activities = entries.iter()
           .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
           .filter_map(|(c, _)| VisualActivityPanel::new(c.clone()).map(|p| Arc::new(p) as Arc<dyn IActivityPanel>))
           .collect();
   }
   ```
5. **删除** `project_chrome` / `build_status_items`（不再需要）
6. **更新 `apply_switch_en`**：直接 `project_entries()` 刷新全部 chrome
7. **删除** `ContribEntry` import from shell_chrome（将类型别名移到 main_window 或 case_view_model）

---

### Part F: 更新 StatusReady 实现 IVisualContribution

**文件**: `demo/src/cases/status_bar_case.rml.rs` L42-54

```rust
use gpui::{AnyElement, SharedString};
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_core::contribution::IVisualContribution;

#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IContribution for StatusReady {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.status_ready").into() }
}

impl IVisualContribution for StatusReady {
    fn render(&self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> AnyElement {
        // 渲染状态栏文本
        gpui::div()
            .text_xs()
            .text_color(rml_core::theme::color("--text-muted"))
            .child(t_static("shell.status_ready"))
            .into_any_element()
    }
}
```

---

### Part G: 更新 main_window.rml 模板

**文件**: `demo/src/shell/main_window.rml`

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
                <menu-item each={c in m.children} label={c.label} command={c.contribution} />
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
            <status-item each={s in status} content={s} />
        </status-bar>
    </template>

    <component content={self.active_view(_window, cx)} />

</tab-window>
```

**注意**：
- `<menu-item each={m in menus}>` 嵌套 `<menu-item each={c in m.children}>` 实现递归菜单
- `command={c.contribution}` 绑定 `Option<Arc<dyn IContribution>>`——需 codegen 识别 `Option<Arc<dyn ICommand>>` 能力查询
- `<status-item each={s in status} content={s} />`——`content={s}` 传递 `StatusViewModel`，由 `StatusItem` 组件调用 `s.render(window, cx)`

**StatusItem 组件设计**：
由于 `<status-item>` 需要渲染 `StatusViewModel`（含 `IVisualContribution`），但 `RenderOnce` 无 `Window`/`App` 参数访问能力，需特殊处理。**方案**：`StatusItem` 为 `EntityRef` 组件，从 ViewModel 字段引用 `Entity<StatusViewModel>`——但 ViewModel 是值类型。

**修正方案**：`<status-item>` 不作为独立组件，直接在 `<status-bar>` 内用 `<component content={s.render(_window, cx)} />`：
```rml
<template slot="footer">
    <status-bar>
        <component each={s in status} content={s.render(_window, cx)} />
    </status-bar>
</template>
```

但 `<component>` 当前不支持 `each`。需确认 `<component each={...}>` 是否工作——查看 `node.rs` L55-69，`<component>` 提前返回，不进入 `each` 处理。

**最终方案**：在 `node.rs` 中让 `<component>` 支持 `each` 指令，或用 `<div each={s in status}>{s.render(_window, cx)}</div>`。选择后者更简单：
```rml
<template slot="footer">
    <status-bar>
        <div each={s in status} content={s.render(_window, cx)} />
    </status-bar>
</template>
```

但 `<div>` 不接受 `content` 属性。用 `<component>`：
```rml
<template slot="footer">
    <status-bar>
        <component each={s in status} content={s.render(_window, cx)} />
    </status-bar>
</template>
```

需更新 `node.rs` 的 `<component>` 处理逻辑，支持 `each` 指令（包装在迭代器中）。

---

### Part H: 删除 shell_chrome.rs 和 menu_shell_contribs.rs

#### H1: 删除 shell_chrome.rs

**删除文件**: `demo/src/shell/shell_chrome.rs`

**更新** `demo/src/shell/mod.rs`：
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

将 `ContribEntry` 类型别名移到 `main_window.rml.rs` 或 `case_view_model.rs`（作为公共类型）。

#### H2: 删除 menu_shell_contribs.rs

**删除文件**: `demo/src/shell/menu_shell_contribs.rs`

菜单贡献现由 MainWindow 的 `RelayCommand` 字段 + `MenuViewModel::build_tree` 处理。但需保留 submenu root 贡献（`MenuFileRoot`/`MenuViewRoot`/`MenuHelpRoot`/`MenuHelpDocs`/`MenuHelpCases`）——这些是无命令的纯分组节点。

**方案**：将 submenu root 贡献移到 `main_window.rml.rs` 或新建 `menu_groups.rs`：
```rust
// main_window.rml.rs 末尾
mod menu_groups {
    use gpui::SharedString;
    use rml::prelude::*;
    use rml_core::i18n::t_static;
    
    #[contribute(host_id = "demo.shell", id = "menu.file", kind = "menu", order = 0)]
    #[derive(Default)]
    pub struct MenuFileRoot;
    impl IContribution for MenuFileRoot {
        fn id(&self) -> &str { Self::CONTRIBUTION_ID }
        fn name(&self) -> SharedString { t_static("menu.file").into() }
    }
    // ... 其余 root 贡献
}
```

或更简洁：直接在 `main_window.rml.rs` 中声明所有 submenu root，作为内部 mod。

---

### Part I: 清理 items 绑定路径

#### I1: 移除 props_registry 中的 items 注册

**文件**: `crates/engine/src/compiler/props_registry.rs`

删除：
```rust
("MenuBar", &["items"]),
("menu", &["items"]),
("StatusBar", &["items"]),
```

#### I2: 移除 setters.rs 中的 items bind setter

**文件**: `crates/engine/src/compiler/menu/setters.rs`

删除 `items` 分支，或整个文件简化为空（若仅 `items` 一个 setter）。

#### I3: 移除 menu_bar.rs 中的 items 绑定路径

**文件**: `crates/engine/src/compiler/menu/menu_bar.rs` L23-32

删除 `items={expr}` 绑定路径，仅保留声明式 + `each` 路径。

#### I4: 让 `<component>` 支持 `each` 指令

**文件**: `crates/engine/src/compiler/codegen/node.rs` L55-69

当前 `<component>` 提前返回，不处理 `each`。更新为：
```rust
if tag == "component" {
    let content_expr = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name, expr } = attr {
            if name == "content" { return Some(expr.clone()); }
        }
        None
    });
    if let Some(expr) = content_expr {
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let code = crate::compiler::codegen::gen_expr_code(expr, &lv, &computed);
        
        // 检测 each 指令
        let each_clause = elem.directives.iter().find_map(|d| match d {
            Directive::Each(c) => Some(c.clone()),
            _ => None,
        });
        if let Some(clause) = each_clause {
            let iter_code = format!(
                "self.{}.iter().map(|{}| {})",
                clause.iterable, clause.item, code
            );
            return Ok((iter_code, true));
        }
        return Ok((code, false));
    }
    return Err(CodegenError {
        message: "<component> 标签必须提供 content={expr} 属性".to_string(),
    });
}
```

---

## 假设与决策

1. **`MenuViewModel.contribution` 类型**：持有 `Option<Arc<dyn IContribution>>`（而非 `Arc<dyn ICommand>`），因为 `Arc<dyn IContribution>` 无法 upcast 到 `Arc<dyn ICommand>`。RML 渲染时 `command={c.contribution}` 传递 `Option<Arc<dyn IContribution>>`，codegen 生成的闭包内通过 `as_command()` 查询。

2. **`command={c.contribution}` codegen**：需更新 `gen_command_closure` 处理 `Option<Arc<dyn IContribution>>` 类型——若 `None` 则 no-op，若 `Some(c)` 则 `c.as_command()?.execute()`。

3. **`StatusBar` 布局**：简化为单层 flex 容器，不再区分 left/right/center 对齐（对齐由 ViewModel 在 RML 中用 `<div flex-1 />` 占位实现，或后续扩展）。

4. **submenu root 贡献保留**：`MenuFileRoot` 等无命令贡献仍需注册（提供 `name()` 作为菜单标签），移到 `main_window.rml.rs` 内部 mod。

5. **`ContribEntry` 类型**：移到 `case_view_model.rs` 作为公共别名，供 `MainWindow` 和 `CaseViewModel` 共用。

## 验证步骤

1. `cargo build -p rust-rml-engine` —— 验证 codegen 改动编译通过
2. `cargo build -p rust-rml-ui` —— 验证 MenuBar/StatusBar 组件改动
3. `cargo build -p rust-rml-demo` —— 验证 MainWindow + ViewModel 改动
4. `cargo test -p rust-rml-engine` —— 验证 props_registry 一致性测试
5. 运行 demo，验证：
   - 菜单栏正常显示 File/View/Help 三组
   - 点击叶子菜单项触发对应命令
   - 状态栏显示 `StatusReady` 内容
   - 主题切换、语言切换功能正常
   - 嵌套子菜单（Help → Docs → Guide/About）正常显示
