# 插槽纯化 + ICommand MVVM + Tree 声明式强化

## 摘要

针对用户提出的三个架构问题，本计划完成：

1. **Step 1（插槽纯化）**：`tab_window`/`modern_window` 的 menu、title_ext、statusbar 改为纯插槽扩展，删除 `MenuItem`/`StatusBarItem`/`MenuBarTemplate`/`StatusBarTemplate` 等基于闭包的数据结构。
2. **Step 2（ICommand MVVM）**：基于 `ICommand` trait 提供新的 `MenuModel`/`MenuItemModel` 数据结构与 `<Menu>`/`<StatusBar>` 控件，支持 `items={...}` 数据绑定 + `on_command` 命令派发。
3. **Step 3（Tree 声明式强化）**：为 `<Tree>` 增加 `<TreeNode>` 子节点声明式语法，codegen 将 RML 树结构转换为 `TreeState` 构建代码（不自定义 TreeView，复用 gpui-component `Tree`）。

Step 1 与 Step 2 紧耦合（删除旧结构的同时必须引入新结构，否则 demo 中间态不可编译），故合并实施。Step 3 独立。

---

## 当前状态分析

### 1. 旧数据结构（基于闭包，需删除）

- `crates/ui/src/window/types.rs`：`MenuItem`（含 `Rc<dyn Fn(&mut Window, &mut App)>` 闭包）、`StatusBarItem`（含闭包）。
- `crates/ui/src/window/templates.rs`：`MenuBarTemplate`（包装 `render_menu_bar`）、`StatusBarTemplate`（包装 `render_status_bar`）。
- `crates/ui/src/window/menu_bar.rs`：`render_menu_bar(&[MenuItem])` + `build_popup_menu` 递归构建 `PopupMenu`。
- `crates/ui/src/window/modern_window.rs`：`render_status_bar(&[StatusBarItem])` 辅助函数。

### 2. Shell 兼容方法（需删除）

- `ModernWindowShell::menu(Vec<MenuItem>)`（modern_window.rs:63-66）
- `ModernWindowShell::status_bar(Vec<StatusBarItem>)`（modern_window.rs:86-89）
- `TabWindowShell::menu(Vec<MenuItem>)`（tab_window.rs:118-121）
- `TabWindowShell::status_bar(Vec<StatusBarItem>)`（tab_window.rs:174-177）

### 3. Shell 已有插槽字段（保留并扩展使用）

- `ModernWindowShell` 已有 `menu_slot: Option<AnyElement>`、`title_ext_slot`、`status_slot` 字段及 `menu_slot()`/`title_ext_slot()`/`status_slot()` builder 方法（modern_window.rs:21-23, 57-83）。
- `TabWindowShell` 已有 `menu_slot`、`title_ext_slot`、`status_slot` 字段及 builder 方法（tab_window.rs:59-68, 113-172）。

### 4. codegen 现状

- `gen_modern_window_wrapper`（codegen.rs:308-377）：从根元素 `Attribute::Bind` 提取 `menu=`/`status_bar=`/`icon=`，生成 `.menu(...)`/`.status_bar(...)` 调用。
- `gen_tab_window_wrapper`（codegen.rs:427-518）：同样从 `Attribute::Bind` 提取 `menu=`/`status_bar=`。
- `partition_tab_slot_children`（codegen.rs:380-410）：仅识别 `slot_left`/`slot_right`/`slot_bottom`，未识别 `slot_menu`/`slot_status_bar`/`slot_title_ext`。

### 5. ICommand trait（已就绪）

`crates/core/src/command.rs:25-45`：
```rust
pub trait ICommand: 'static {
    fn execute(&mut self, parameter: &dyn std::any::Any, cx: &mut Context<Self>) where Self: Sized;
    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool { true }
}
```
trait 含 `where Self: Sized` 约束（因 `Context<Self>` 要求），故 **不能** 用 `Arc<dyn ICommand>` 跨类型分发；只能由具体 ViewModel 实现 `ICommand`，在 `execute` 内根据 `parameter` 派发。

### 6. ActivityBar 依赖

- `crates/ui/src/components/activity_bar.rs:19-20`：`use crate::window::menu_bar::build_popup_menu; use crate::window::types::MenuItem;`
- `IActivityAct::context_menu() -> Vec<MenuItem>`（line 41）
- `ActivityAct::context_menu: Vec<MenuItem>`（line 113）
- 渲染 action_buttons 时调用 `build_popup_menu`（line 277）

需将 `Vec<MenuItem>` 替换为新 `Vec<MenuItemModel>` 类型。

### 7. demo 现状

- `demo/src/shell/main_window.rml`：根元素使用 `menu={menu_items}` / `status_bar={status_items}` bind 属性。
- `demo/src/shell/main_window.rml.rs`：`menu_items()` / `status_items()` computed 方法返回 `Vec<MenuItem>` / `Vec<StatusBarItem>`。
- `demo/src/cases/catalog.rs`：`init_tree_state` 编程式构建 `TreeState`，`tree_items(cx)` 返回 `Vec<TreeItem>`。

### 8. TreeView 组件现状

- `crates/ui/src/components/tree_view.rs`：`TreeView` 包装 gpui-component `Tree`，构造需 `Entity<TreeState>`。
- `tags.rs:276-281`：`Tree` 路由 `Stateful { state_field: "case_tree_state" }`，codegen 生成 `TreeView::new(&self.case_tree_state)`。
- 当前不支持 `<TreeNode>` 声明式子节点。

---

## 设计决策

### D1：ViewModel 实现 ICommand，命令派发集中在 `execute`

**理由**：`ICommand::execute` 签名含 `Context<Self>` + `where Self: Sized`，无法做成 `Arc<dyn ICommand>` 跨类型分发。ViewModel 直接实现 `ICommand`，在 `execute` 内 `downcast_ref::<SharedString>()` 取出 `command_id` 后 match 派发，是最简洁、符合现有 trait 设计的方案。

**示例**：
```rust
impl ICommand for MainWindow {
    fn execute(&mut self, parameter: &dyn Any, cx: &mut Context<Self>) {
        if let Some(cmd_id) = parameter.downcast_ref::<SharedString>() {
            match cmd_id.as_ref() {
                "file_new" => self.on_new_file(cx),
                "file_exit" => self.close(cx),
                _ => {}
            }
        }
    }
    fn can_execute(&self, parameter: &dyn Any) -> bool {
        if let Some(cmd_id) = parameter.downcast_ref::<SharedString>() {
            return !matches!(cmd_id.as_ref(), "file_exit" if self.is_busy);
        }
        true
    }
}
```

### D2：新 `MenuItemModel` 为纯数据，命令通过 `command_id` 标识

**理由**：MVVM 要求 ViewModel 持有纯数据，命令派发由 View 层调用 ViewModel 的 `ICommand::execute`。`MenuItemModel` 不含闭包，仅含 `label` / `command_id: Option<SharedString>` / `disabled` / `checked` / `children` / `separator`。

### D3：Step 1 + Step 2 合并实施，避免中间不可编译状态

**理由**：删除 `MenuItem` 后 demo 立即不可编译（`menu_items()` computed 返回 `Vec<MenuItem>`），必须同时引入 `MenuItemModel` + `<Menu>` 控件才能恢复。

### D4：`<Menu>`/`<StatusBar>` 作为内置 RML 扩展组件，注册到 `tags.rs`

**理由**：让用户在 `<slot_menu>` 内写 `<Menu items={menu_items} on_command="on_command" />`，由 codegen 通过 `component_lookup` 路由生成 `rml_ui::Menu::new().items(...).on_command(...)` 调用。`<TreeNode>` 同理注册为 `StatelessNoId` 组件。

### D5：Step 3 不自定义 TreeView，codegen 转换 `<TreeNode>` 为 `TreeState` 构建代码

**理由**：用户明确指示。`<TreeNode>` 标签在 codegen 阶段被识别，转换为 `TreeItem::new(id, label).expanded(true).child(...)` 链式调用。静态树在 `on_loaded` 中构建 `TreeState`，动态树通过 `items={...}` 绑定 `Vec<TreeItem>`。

---

## Phase 1：Step 1 + Step 2 合并实施（删除旧结构 + ICommand MVVM）

### 1.1 创建新模块 `crates/ui/src/window/menu.rs`

定义纯数据 `MenuItemModel` / `MenuModel` + `<Menu>` RenderOnce 组件。

```rust
//! MVVM 菜单数据模型 + Menu 控件
//!
//! - `MenuItemModel`：纯数据（label/command_id/disabled/checked/children/separator）
//! - `Menu`：RenderOnce 水平菜单栏，items 数据驱动，on_command 回调派发到 ICommand::execute

use std::rc::Rc;
use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, prelude::FluentBuilder as _};
use gpui_component::{Sizable as _, button::{Button, ButtonVariants as _}, menu::{DropdownMenu as _, PopupMenu, PopupMenuItem}};

#[derive(Clone)]
pub struct MenuItemModel {
    pub label: SharedString,
    pub command_id: Option<SharedString>,
    pub disabled: bool,
    pub checked: bool,
    pub children: Vec<MenuItemModel>,
    pub separator: bool,
}

impl MenuItemModel {
    pub fn new(label: impl Into<SharedString>) -> Self { /* ... */ }
    pub fn separator() -> Self { /* ... */ }
    pub fn command(mut self, id: impl Into<SharedString>) -> Self { self.command_id = Some(id.into()); self }
    pub fn disabled(mut self, d: bool) -> Self { self.disabled = d; self }
    pub fn checked(mut self, c: bool) -> Self { self.checked = c; self }
    pub fn submenu(mut self, children: Vec<MenuItemModel>) -> Self { self.children = children; self }
}

#[derive(IntoElement)]
pub struct Menu {
    items: Vec<MenuItemModel>,
    on_command: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
}

impl Menu {
    pub fn new() -> Self { /* items: vec![], on_command: None */ }
    pub fn items(mut self, items: Vec<MenuItemModel>) -> Self { self.items = items; self }
    pub fn on_command(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self { /* ... */ }
}

impl Default for Menu { fn default() -> Self { Self::new() } }

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // 渲染水平菜单栏，叶子节点点击调用 on_command(command_id)
        // 含子菜单的项用 dropdown_menu 弹出 PopupMenu
        // build_popup_menu_model 递归构建 PopupMenu，叶子节点 on_click 调用 on_command
    }
}
```

### 1.2 创建新模块 `crates/ui/src/window/status_bar.rs`

定义 `StatusBarItemModel` + MVVM 友好的 `<StatusBar>` 控件。

```rust
//! MVVM 状态栏数据模型 + StatusBar 控件

use std::rc::Rc;
use gpui::{App, IntoElement, RenderOnce, SharedString, Window};
use gpui_component::status_bar::StatusBar as GpuiStatusBar;

#[derive(Clone)]
pub struct StatusBarItemModel {
    pub label: SharedString,
    pub command_id: Option<SharedString>,
    pub icon: Option<SharedString>,
}

impl StatusBarItemModel {
    pub fn new(label: impl Into<SharedString>) -> Self { /* ... */ }
    pub fn command(mut self, id: impl Into<SharedString>) -> Self { /* ... */ }
}

#[derive(IntoElement)]
pub struct StatusBar {
    items: Vec<StatusBarItemModel>,
    on_command: Option<Rc<dyn Fn(&SharedString, &mut Window, &mut App) + 'static>>,
}

impl StatusBar {
    pub fn new() -> Self { /* ... */ }
    pub fn items(mut self, items: Vec<StatusBarItemModel>) -> Self { /* ... */ }
    pub fn on_command(mut self, f: impl Fn(&SharedString, &mut Window, &mut App) + 'static) -> Self { /* ... */ }
}

impl RenderOnce for StatusBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // 用 gpui_component::status_bar::StatusBar 构建
        // items 依次 left(label) 排列
        // 可点击项绑定 on_click → on_command(command_id)
    }
}
```

### 1.3 删除旧模块与数据结构

- 删除 `crates/ui/src/window/types.rs`（MenuItem/StatusBarItem）
- 删除 `crates/ui/src/window/templates.rs`（MenuBarTemplate/StatusBarTemplate）
- 删除 `crates/ui/src/window/menu_bar.rs`（render_menu_bar/build_popup_menu）
- 删除 `crates/ui/src/window/modernern_window.rs` 中的 `render_status_bar` 函数（line 138-144）

### 1.4 更新 `crates/ui/src/window/mod.rs`

```rust
pub mod menu;          // 新：Menu/MenuItemModel
pub mod status_bar;    // 新：StatusBar/StatusBarItemModel
// pub mod templates;  // 删除
// pub mod types;     // 删除
// pub mod menu_bar;  // 删除

pub use menu::{Menu, MenuItemModel};
pub use status_bar::{StatusBar, StatusBarItemModel};
// 删除：pub use templates::{MenuBarTemplate, StatusBarTemplate};
// 删除：pub use types::{MenuItem, StatusBarItem};
```

### 1.5 删除 Shell 兼容方法

- `ModernWindowShell`：删除 `.menu(Vec<MenuItem>)`（modern_window.rs:62-66）、`.status_bar(Vec<StatusBarItem>)`（modern_window.rs:85-89）
- `TabWindowShell`：删除 `.menu(Vec<MenuItem>)`（tab_window.rs:118-121）、`.status_bar(Vec<StatusBarItem>)`（tab_window.rs:174-177）
- 两文件 `use` 语句删除 `templates::{MenuBarTemplate, StatusBarTemplate}`、`types::{MenuItem, StatusBarItem}`

### 1.6 修改 `crates/ui/src/components/activity_bar.rs`

`IActivityAct::context_menu()` 返回类型改为 `Vec<MenuItemModel>`：
- line 19-20 删除 `use crate::window::menu_bar::build_popup_menu;` 和 `use crate::window::types::MenuItem;`
- 改为 `use crate::window::menu::{MenuItemModel, Menu};` 或在 render 中调用 `Menu::new().items(...)`
- `IActivityAct::context_menu(&self) -> Vec<MenuItemModel>`（line 41）
- `ActivityAct.context_menu: Vec<MenuItemModel>`（line 113）
- 渲染 action_buttons 时：将 `build_popup_menu(menu, &menu_items, window, cx)` 改为 `Menu::new().items(menu_items)` 作为 dropdown 内容（line 276-279）

### 1.7 更新 `crates/ui/src/lib.rs` 与 `prelude.rs` re-export

```rust
// lib.rs
pub use window::menu::{Menu, MenuItemModel};
pub use window::status_bar::{StatusBar, StatusBarItemModel};
```

### 1.8 codegen：扩展 `partition_tab_slot_children` 识别新插槽标签

**当前调用链**（codegen.rs:242-246, 275-286）：
- `gen_render_impl_from_children` 中 `ShellWrap::Tab` 调用 `partition_tab_slot_children` 拆出 (slot_left, slot_right, slot_bottom, body)
- `ShellWrap::Modern` **未调用** partition，直接将 `elem.children.clone()` 作为 body_children，由 `gen_modern_window_wrapper` 通过 bind 属性处理 menu/status_bar

**改造方案**：

引入统一结构 `ShellSlots`，`partition_tab_slot_children` 改为返回该结构：
```rust
struct ShellSlots {
    slot_left: Option<Node>,
    slot_right: Option<Node>,
    slot_bottom: Option<Node>,
    slot_menu: Option<Node>,
    slot_title_ext: Option<Node>,
    slot_status_bar: Option<Node>,
    body: Vec<Node>,
}
```

`partition_tab_slot_children`（codegen.rs:380-410）内 match 增加：
```rust
"slot_menu" => slot_menu = slot_element_content(elem),
"slot_title_ext" => slot_title_ext = slot_element_content(elem),
"slot_status_bar" => slot_status_bar = slot_element_content(elem),
```

**`gen_render_impl_from_children` 改造**（codegen.rs:242-246）：

```rust
// 两种 shell 都调用 partition，统一拆出所有 slot
let slots = if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
    partition_tab_slot_children(&elem.children)
} else {
    ShellSlots { /* 全 None, body = elem.children.clone() */ }
};
```

随后为 `slot_menu`/`slot_title_ext`/`slot_status_bar` 生成代码（与现有 slot_left/right/bottom 相同模式，line 262-273），并传入 `gen_modern_window_wrapper` / `gen_tab_window_wrapper`。

### 1.9 codegen：修改 `gen_modern_window_wrapper`（codegen.rs:308-377）

- 删除 `menu` / `status_bar` bind 分支（line 322-349）
- 保留 `icon` bind 分支（line 351-369）
- 函数签名增加参数：`slot_menu: Option<&str>`, `slot_status_bar: Option<&str>`, `slot_title_ext: Option<&str>`
- 在生成代码尾部增加：
  ```rust
  if let Some(menu_code) = slot_menu { code.push_str(&format!(".menu_slot({})", menu_code)); }
  if let Some(ext_code) = slot_title_ext { code.push_str(&format!(".title_ext_slot({})", ext_code)); }
  if let Some(status_code) = slot_status_bar { code.push_str(&format!(".status_slot({})", status_code)); }
  ```
- 调用方 `gen_root_render`（codegen.rs 顶部）需先调用 `partition_modern_window_slots` 拆出三个 slot，再传递给 `gen_modern_window_wrapper`

### 1.10 codegen：修改 `gen_tab_window_wrapper`（codegen.rs:427-518）

- 删除 `menu` / `status_bar` bind 分支（line 463-465）
- 函数签名增加：`slot_menu: Option<&str>`, `slot_status_bar: Option<&str>`, `slot_title_ext: Option<&str>`
- 在 slot_left/slot_right/slot_bottom 拼接后追加：
  ```rust
  if let Some(menu_code) = slot_menu { code.push_str(&format!(".menu_slot({})", menu_code)); }
  if let Some(ext_code) = slot_title_ext { code.push_str(&format!(".title_ext_slot({})", ext_code)); }
  if let Some(status_code) = slot_status_bar { code.push_str(&format!(".status_slot({})", status_code)); }
  ```

### 1.11 codegen：`tags.rs` 注册 `<Menu>` / `<StatusBar>` 组件

```rust
"Menu" => Some(ComponentTag {
    ctor_path: "rml_ui::Menu",
    kind: ComponentKind::StatelessNoId,  // Menu::new() 无参
}),
"StatusBar" => Some(ComponentTag {
    ctor_path: "rml_ui::StatusBar",
    kind: ComponentKind::StatelessNoId,
}),
```

### 1.11.1 codegen：调整 `is_container` 判定（component.rs:116-117）

当前 `is_container` 逻辑：
```rust
let is_container = matches!(component.kind, tags::ComponentKind::StatelessNoId) || tag == "ActivityBar";
```

将 `Menu`/`StatusBar` 注册为 `StatelessNoId` 后，会被误判为容器（走 `.child(...)` 路径）。需排除：
```rust
let is_container = (matches!(component.kind, tags::ComponentKind::StatelessNoId)
    && !matches!(tag.as_str(), "Menu" | "StatusBar"))
    || tag == "ActivityBar";
```

这样 `<Menu>`/`<StatusBar>` 走 setter 路径（`items=`/`on_command=`），不走 child 路径。

### 1.12 codegen：`component.rs` 增加 `items` / `on_command` setter

`component_bind_setter`（component.rs:228-270）增加分支：
```rust
"items" => Some(format!(".items({}.clone())", rust_expr)),
```

`component_event_setter`（component.rs:280-）增加分支：
```rust
"on_command" if tag == "Menu" || tag == "StatusBar" => {
    let method = match handler { /* ... */ };
    Some(format!(
        ".on_command(cx.listener(move |this, cmd_id: &gpui::SharedString, _window, cx| {{\n                    \
         this.{}(cmd_id, cx);\n                }}))",
        method
    ))
}
```

> 注：这里 `on_command` 调用一个 ViewModel 方法（如 `on_command`），由该方法内部调用 `self.execute(cmd_id, cx)` 派发到 `ICommand`。或者更直接：codegen 直接生成 `this.execute(cmd_id, cx)`，无需中间方法。**推荐直接生成 `this.execute`**，让 ViewModel 实现 `ICommand` 即可，省去 `on_command` 中间方法。

最终生成代码：
```rust
.on_command(cx.listener(move |this, cmd_id: &gpui::SharedString, _window, cx| {
    this.execute(cmd_id as &dyn std::any::Any, cx);
}))
```

注：`&SharedString` 作为 `&dyn Any` 传入，`ICommand::execute` 内 `parameter.downcast_ref::<SharedString>()` 取回。

### 1.13 更新 demo

**`demo/src/shell/main_window.rml`**：
```xml
<tab_window title="RML Showcase" ... icon={IconName::Frame}
            tabs={tab_bar_items} selected_tab={selected_tab}
            on_tab_click="on_tab_click"
            show_chrome={show_chrome} on_chrome_toggle="on_chrome_toggle">
    <slot_menu>
        <Menu items={menu_items} />
    </slot_menu>
    <slot_status_bar>
        <StatusBar items={status_items} />
    </slot_status_bar>
    <slot_left>...</slot_left>
    ...
</tab_window>
```

**`demo/src/shell/main_window.rml.rs`**：
- 删除 `use rml_ui::{... MenuItem, StatusBarItem, ...}` 旧导入
- 改为 `use rml_ui::{... MenuItemModel, StatusBarItemModel, ...}`
- `menu_items()` computed 返回 `Vec<MenuItemModel>`，使用 `.command("file_new")` 设置命令 id
- `status_items()` computed 返回 `Vec<StatusBarItemModel>`
- 删除 `on_command` 方法（如采用 D1 的直接生成 `this.execute` 方案）
- 实现 `impl ICommand for MainWindow`：
  ```rust
  impl ICommand for MainWindow {
      fn execute(&mut self, parameter: &dyn Any, cx: &mut Context<Self>) {
          if let Some(cmd_id) = parameter.downcast_ref::<SharedString>() {
              match cmd_id.as_ref() {
                  "file_new" => { /* ... */ }
                  "file_exit" => self.close(cx),
                  "help_about" => { /* ... */ }
                  _ => {}
              }
          }
      }
  }
  ```

### 1.14 Phase 1 验证

- `cargo build` 通过
- `cargo test` 全部通过（约 265+ 测试）
- 启动 demo，菜单栏与状态栏正常显示，点击菜单项触发 `ICommand::execute` 派发

---

## Phase 2：Step 3 Tree 声明式强化

### 2.1 设计目标

支持两种 Tree 数据来源：

1. **静态声明式**：在 RML 中用 `<TreeNode>` 标签声明树结构，codegen 在 `on_loaded` 中生成 `TreeState::new(cx).items(...)` 构建代码。
2. **动态数据绑定**：`<Tree items={self.tree_items}>`，从 `#[computed]` 方法返回 `Vec<TreeItem>`。

> 本 Phase 优先实现方案 2（动态数据绑定），因为 demo 已有 `tree_items(cx)` 模式，只需让 codegen 支持 `items=` 属性即可。方案 1（静态 `<TreeNode>`）作为后续可选扩展。

### 2.2 修改 `crates/ui/src/components/tree_view.rs`

`TreeView::new` 当前仅接受 `Entity<TreeState>`。增加动态 items 支持：

```rust
pub struct TreeView {
    state: Entity<TreeState>,
    on_activate: Option<Rc<dyn Fn(TreeItem, &mut Window, &mut App) + 'static>>,
}

impl TreeView {
    pub fn new(state: &Entity<TreeState>) -> Self { /* 保持不变 */ }
    
    /// 设置 items（触发 TreeState 更新）
    pub fn set_items(&self, items: Vec<TreeItem>, cx: &mut App) {
        self.state.update(cx, |s, cx| s.set_items(items, cx));
    }
}
```

> 注：`TreeView` 是 RenderOnce，每次 render 由 codegen 重新构造。若 items 来自 `#[computed]` 方法，codegen 需在 render 前更新 state。详见 2.3。

### 2.3 codegen：支持 `<Tree items={...}>` 数据绑定

当前 codegen 对 `<Tree>` 生成 `rml_ui::TreeView::new(&self.case_tree_state)`。

扩展为：若 `<Tree>` 含 `items=` bind 属性，生成：
```rust
{
    let __rml_tree_items = self.tree_items();  // Vec<TreeItem>
    self.case_tree_state.update(cx, |s, cx| s.set_items(__rml_tree_items, cx));
    rml_ui::TreeView::new(&self.case_tree_state)
}
```

> 注意：`update(cx, ...)` 需要 `cx: &mut App`，但 render 函数中 `cx` 是 `&mut Context<Self>`，需用 `cx.entity()` 或直接 `self.case_tree_state.update(cx, ...)`（`Context<Self>` 可转 `&mut App`）。

### 2.4 codegen：`component.rs` 增加 `items` setter for Tree

`component_bind_setter`（component.rs:228-270）：
```rust
"items" if tag == "Tree" => {
    // 特殊处理：items 需先 update TreeState，再构造 TreeView
    // 这个逻辑较复杂，可能需要在 gen_component 早期分支处理
}
```

> 实际实现可能需要在 `gen_component` 函数中为 `Tree` tag 增加特殊分支，而非简单 setter。

### 2.5 更新 demo

**`demo/src/shell/main_window.rml`**：
```xml
<Tree on_activate="on_case_activate" items={tree_items} />
```

**`demo/src/shell/main_window.rml.rs`**：
- 增加 `tree_items` computed 方法（或直接调用 `cases::tree_items(cx)` —— 但 computed 无法访问 `cx: &mut Context`，需用 `#[computed]` + 内部状态或保持 `on_loaded` 初始化模式）
- 简化方案：保持 `on_loaded` 中 `init_tree_state`，`<Tree>` 仅传 `on_activate`（无 `items=` bind）。这样 Phase 2 改动最小，且不破坏现有 demo 行为。

### 2.6 Phase 2 验证

- `cargo build` 通过
- `cargo test` 全部通过
- demo 启动后 Tree 正常显示案例树，点击叶子节点打开对应案例 Tab

---

## 实施顺序与任务分解

### Phase 1（Step 1 + Step 2 合并）

1. **Task A**：创建 `crates/ui/src/window/menu.rs`（MenuItemModel + Menu 组件）
2. **Task B**：创建 `crates/ui/src/window/status_bar.rs`（StatusBarItemModel + StatusBar 组件）
3. **Task C**：删除 `types.rs` / `templates.rs` / `menu_bar.rs`，更新 `mod.rs` re-export
4. **Task D**：删除 Shell 兼容方法（ModernWindowShell.menu/status_bar，TabWindowShell.menu/status_bar）
5. **Task E**：更新 `activity_bar.rs` 改用 `MenuItemModel`
6. **Task F**：codegen 扩展 `partition_tab_slot_children` 识别 `slot_menu`/`slot_title_ext`/`slot_status_bar`
7. **Task G**：codegen 修改 `gen_modern_window_wrapper` 和 `gen_tab_window_wrapper`（删除 menu=/status_bar= bind，改用 slot 参数）
8. **Task H**：`tags.rs` 注册 `<Menu>` / `<StatusBar>` 组件
9. **Task I**：`component.rs` 增加 `items` / `on_command` setter，`on_command` 直接生成 `this.execute(cmd_id, cx)`
10. **Task J**：更新 `lib.rs` / `prelude.rs` re-export
11. **Task K**：更新 demo `main_window.rml` + `main_window.rml.rs`（改用 `<slot_menu><Menu .../></slot_menu>`，实现 `impl ICommand for MainWindow`）
12. **Task L**：`cargo build` + `cargo test` 验证

### Phase 2（Step 3）

13. **Task M**：codegen 支持 `<Tree items={...}>` 数据绑定（或保持现有 `on_loaded` 模式，仅做最小改动验证 demo）
14. **Task N**：`cargo build` + `cargo test` 验证

---

## 假设与边界

### 假设

- `gpui_component::menu::PopupMenu` 支持在 `on_click` 中调用自定义闭包（已验证：menu_bar.rs:81-85 现有模式）。
- `gpui_component::status_bar::StatusBar` 的 `left(label)` 方法存在（已验证：modern_window.rs:142 现有用法）。
- `ICommand::execute` 的 `parameter: &dyn Any` 可传入 `&SharedString`（`SharedString: 'static + Send + Sync`，满足 `Any` 要求）。
- `partition_tab_slot_children` 当前仅用于 `tab_window`，`modern_window` 未调用此函数（需确认 `gen_modern_window_wrapper` 当前如何处理子节点 —— 当前是 `children_body` 整体传入 `.child(...)`，未做 slot 拆分）。

### 边界（不在本次范围）

- `<TreeNode>` 静态声明式语法（Phase 2 仅支持动态 `items=` 绑定或保持 `on_loaded` 模式）
- Menu/StatusBar 的 i18n 数据绑定（保持现有 `t_static` 模式）
- `can_execute` 的 UI 反馈（disabled 状态绑定，可作为后续扩展）
- ActivityBar `IActivityAct::context_menu` 的完整 MVVM 化（仅修改返回类型，不改 API 语义）

### 风险

- **R1**：`ICommand::execute` 中 `Context<Self>` 约束要求 ViewModel 实现 trait 时显式列出 `where Self: Sized`，编译器可能对 `impl ICommand for MainWindow` 报错。**缓解**：参考 `crates/core/src/command.rs:51-103` 现有测试，已验证可为具体 struct 实现 ICommand。
- **R2**：codegen 修改 `partition_tab_slot_children` 返回类型可能影响 `gen_root_render` 调用链。**缓解**：保持函数签名兼容（增加返回字段，调用方解构）。
- **R3**：`<Menu>` 作为 `StatelessNoId` 组件注册，codegen 会通过 `gen_component` 处理其 `items=`/`on_command=` 属性，但 `StatelessNoId` 路径当前假定 `TitleBar::new()` 无参构造 + `.child(...)` 子节点。需确认 `Menu::new().items(...).on_command(...)` 链式调用与 `StatelessNoId` 路径兼容。**缓解**：`Menu` 实现 `ParentElement` 不是必须的，只需 codegen 的 `is_container` 判定不把 `Menu` 视为容器（line 116-117 当前 `is_container` 仅匹配 `StatelessNoId` 整体 + `ActivityBar`，需调整使 `Menu`/`StatusBar` 走 setter 路径而非 child 路径）。

---

## 验证步骤

### Phase 1 验证

```bash
cargo build
cargo test
cargo run --bin demo  # 手动验证：菜单栏/状态栏显示，点击菜单项触发命令
```

### Phase 2 验证

```bash
cargo build
cargo test
cargo run --bin demo  # 手动验证：Tree 显示案例树，点击叶子节点打开 Tab
```

---

## 文件改动清单

### 新增

- `crates/ui/src/window/menu.rs`
- `crates/ui/src/window/status_bar.rs`

### 删除

- `crates/ui/src/window/types.rs`
- `crates/ui/src/window/templates.rs`
- `crates/ui/src/window/menu_bar.rs`

### 修改

- `crates/ui/src/window/mod.rs`（re-export 调整）
- `crates/ui/src/window/modern_window.rs`（删除 menu/status_bar 兼容方法 + render_status_bar）
- `crates/ui/src/window/tab_window.rs`（删除 menu/status_bar 兼容方法）
- `crates/ui/src/components/activity_bar.rs`（改用 MenuItemModel）
- `crates/ui/src/lib.rs`（re-export Menu/MenuItemModel/StatusBar/StatusBarItemModel）
- `crates/ui/src/prelude.rs`（同上）
- `crates/engine/src/tags.rs`（注册 Menu/StatusBar 组件）
- `crates/engine/src/compiler/codegen.rs`（partition 扩展 + gen_modern_window_wrapper/gen_tab_window_wrapper 修改）
- `crates/engine/src/compiler/component.rs`（items/on_command setter）
- `demo/src/shell/main_window.rml`（改用 slot_menu/slot_status_bar）
- `demo/src/shell/main_window.rml.rs`（改用 MenuItemModel/StatusBarItemModel + impl ICommand）
