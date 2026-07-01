# P0 插槽修复 + Menu/StatusBar 数据绑定 + 组件注册表 + 命名空间支持

## Context

当前框架存在四类问题：
1. **插槽缺口**：用户规格定义 6 个插槽（`slot_menu`/`slot_title`/`slot_left`/`slot_right`/`slot_bottom`/`slot_footer`），框架缺少 `slot_title`，且 `slot_status` 命名与功能不符（实为 footer）
2. **Menu/StatusBar 无 MVVM 数据绑定**：违反 project_memory 硬约束"Menu, statusbar 等控件必须提供 MVVM 数据绑定解决方案"
3. **无法嵌入用户组件**：`component_lookup` 是硬编码表，`#[component]` 标注的结构体无法作为 `<CounterCase />` 标签嵌入父视图 `.rml`
4. **无命名空间支持**：无法 `<d3:chart/>` 引用外部库组件

本次改造同时引入 WPF/XAML 风格的命名空间支持（`rmlns:d3="gpui_d3rs"` + `<d3:chart/>`），为未来外部组件库集成铺路。

---

## Current State（探索验证）

### Phase 1 已完成（shell.rs 已修改）

`crates/engine/src/compiler/codegen/shell.rs` 已完成以下改动：
- ✅ `partition_tab_slot_children` → `partition_slot_children`，返回 7-tuple `(menu, title, footer, left, right, bottom, body)`
- ✅ 新增 `slot_title` / `slot_footer` match 分支，移除 `slot_status`
- ✅ `gen_modern_window_wrapper` 签名新增 `slot_title: Option<&str>` / `slot_footer: Option<&str>`，bind 属性 `status_bar` → `footer`，生成 `.title_ext_slot()` / `.footer_slot()`
- ✅ `gen_tab_window_wrapper` 签名新增 `slot_title: Option<&str>`，`slot_status` → `slot_footer`，bind 属性 `status_bar` → `footer`，生成 `.title_ext_slot()` / `.footer_slot()`

### Phase 1 待完成（4 个文件，当前代码处于不可编译状态）

**`crates/engine/src/compiler/codegen/mod.rs`** — L163-218 需更新：
- L163-168：6-tuple 解构 → 7-tuple；`partition_tab_slot_children` → `partition_slot_children`；`ShellWrap::Modern` 也调用 partition（当前 Modern 直接用 `elem.children.clone()`，会导致 `<modern_window>` 内的 slot_* 被当作 body 渲染）
- L188-191：`slot_status_code` → `slot_footer_code`；新增 `slot_title_code` 的 `gen_node` 调用
- L205-218：`gen_tab_window_wrapper` 调用新增 `slot_title_code.as_deref()` 参数；`gen_modern_window_wrapper` 调用新增 `slot_title_code.as_deref()` / `slot_footer_code.as_deref()` 参数

**`crates/ui/src/window/tab_window.rs`** — `title_ext_slot` 已存在（L60/L118/L255），需重命名：
- L68 字段 `status_slot` → `footer_slot`
- L90 init `status_slot: None` → `footer_slot: None`
- L164-167 builder `status_slot()` → `footer_slot()`
- L312 render `self.status_slot` → `self.footer_slot`

**`crates/ui/src/window/modern_window.rs`** — `title_ext_slot` 已存在（L24/L65/L120），需重命名：
- L25 字段 `status_slot` → `footer_slot`
- L37 init `status_slot: None` → `footer_slot: None`
- L76-79 builder `status_slot()` → `footer_slot()`
- L124 render `self.status_slot` → `self.footer_slot`

**`demo/src/shell/main_window.rml`**：
- L28 `<slot_status>` → `<slot_footer>`

### Phase 2-5 全部待实现

- `crates/ui/src/components/menu.rs` 不存在
- `crates/ui/src/components/status_bar_wrapper.rs` 不存在
- `CodegenCtx` 无 `user_components` / `namespaces` 字段
- `StructMetadata` 无 `is_component` 字段
- demo cases 全部内联在 `main_window.rml`，无独立 case 组件文件

### 关键探索结论

1. **Parser tokenizer**（`tokenizer.rs:174` `read_tag_name`）已接受 `:` 字符 → 命名空间无需改 parser
2. **Validator**（`validator.rs`）不校验属性名 → `rmlns:d3` 不会被拒绝，但 codegen 的 `apply_static_attr` 会把它当作未知属性生成 `.child("rmlns:d3=gpui_d3rs")`，需在 codegen 显式跳过
3. **ActivityBar**（`activity_bar.rs`）是黄金模板：`ITrait` + struct + `Vec<Arc<dyn ITrait>>` + 容器组件 + `into_arc()` + `panels()/actions()` builder
4. **ICommand**（`crates/core/src/command.rs`）已是 object-safe，`RelayCommand::new(cx, |this, cx| ...)` 捕获 `WeakEntity<T>`，可直接用于 `MenuItem::command()`
5. **Tree Stateful 模式**（`component.rs:67-70`）：`self.case_tree_state.as_ref().expect("init TreeState in on_loaded")` 是用户组件嵌入的模板
6. **`#[component]` 宏**（`macros/src/component.rs`）已生成 `impl IComponent`（`rml_tag()` 返回结构体名）+ `include!(...rml_generated/<snake>.rs)`，且 RML 根节点支持 `<component>`
7. **build.rs**（`build/mod.rs:262-284`）按文件名 stem → PascalCase 作为 `view_struct_name`，scanner 按 `#[window]`/`#[component]` 属性识别 struct
8. **component_lookup**（`tags.rs:213-298`）是硬编码 match 表，无 fallback 机制；`is_component()`（L124-129）只认 PascalCase

---

## Feature 1: slot_title 新增 + slot_footer 重命名（Phase 1 续）

### 变更文件（4 个，见 Current State 待完成列表）

1. **`crates/engine/src/compiler/codegen/mod.rs`** — 更新 7-tuple 解构 + Modern 也调用 partition + 两个 wrapper 调用签名
2. **`crates/ui/src/window/tab_window.rs`** — `status_slot` → `footer_slot`（字段/builder/render）
3. **`crates/ui/src/window/modern_window.rs`** — `status_slot` → `footer_slot`（字段/builder/render）
4. **`demo/src/shell/main_window.rml`** — `<slot_status>` → `<slot_footer>`

### 关键代码（codegen/mod.rs L163-218 改动后）

```rust
let (slot_menu, slot_title, slot_footer, slot_left, slot_right, slot_bottom, body_children) =
    if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
        shell::partition_slot_children(&elem.children)
    } else {
        (None, None, None, None, None, None, elem.children.clone())
    };

// ... body 生成不变 ...

let slot_menu_code = slot_menu.as_ref().map(|n| gen_node(n, ctx, 0, &mut id_counter, &empty).map(|(c,_)| c)).transpose()?;
let slot_title_code = slot_title.as_ref().map(|n| gen_node(n, ctx, 0, &mut id_counter, &empty).map(|(c,_)| c)).transpose()?;
let slot_footer_code = slot_footer.as_ref().map(|n| gen_node(n, ctx, 0, &mut id_counter, &empty).map(|(c,_)| c)).transpose()?;
let slot_left_code = ...; let slot_right_code = ...; let slot_bottom_code = ...;

let final_body = match shell {
    ShellWrap::Modern => shell::gen_modern_window_wrapper(
        elem, ctx, &body,
        slot_title_code.as_deref(),
        slot_footer_code.as_deref(),
    )?,
    ShellWrap::Tab => shell::gen_tab_window_wrapper(
        elem, ctx, &body,
        slot_menu_code.as_deref(),
        slot_title_code.as_deref(),
        slot_footer_code.as_deref(),
        slot_left_code.as_deref(),
        slot_right_code.as_deref(),
        slot_bottom_code.as_deref(),
    )?,
    ShellWrap::None => body,
};
```

---

## Feature 2: `<menu items={...}/>` + `<status_bar items={...}/>` MVVM 数据绑定

### 新增文件

**`crates/ui/src/components/menu.rs`** — 参照 `activity_bar.rs` 黄金模板

```rust
use std::sync::Arc;
use gpui::{AnyElement, App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window};
use gpui_component::{IconName, button::{Button, ButtonVariants as _}, h_flex, separator::Separator};

/// 菜单项接口（参照 IActivityPanel）
pub trait IMenuItem: 'static {
    fn label(&self) -> SharedString;
    fn icon(&self) -> Option<IconName> { None }
    fn disabled(&self) -> bool { false }
    fn separator(&self) -> bool { false }
    fn command(&self) -> Option<Arc<dyn rml_core::command::ICommand>> { None }
}

pub type MenuItems = Vec<Arc<dyn IMenuItem>>;

pub struct MenuItem {
    label: SharedString,
    icon: Option<IconName>,
    disabled: bool,
    separator: bool,
    command: Option<Arc<dyn rml_core::command::ICommand>>,
}

impl MenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self { ... }
    pub fn icon(mut self, icon: IconName) -> Self { ... }
    pub fn disabled(mut self, d: bool) -> Self { ... }
    pub fn separator(mut self) -> Self { self.separator = true; self }
    pub fn command(mut self, cmd: Arc<dyn rml_core::command::ICommand>) -> Self { ... }
    pub fn into_arc(self) -> Arc<dyn IMenuItem> { Arc::new(self) }
}

impl IMenuItem for MenuItem { ... }

/// Menu 容器组件（Stateless: Menu::new(id).items(items)）
#[derive(IntoElement)]
pub struct Menu {
    id: ElementId,
    items: MenuItems,
}

impl Menu {
    pub fn new(id: impl Into<ElementId>) -> Self { ... }
    pub fn items(mut self, items: MenuItems) -> Self { self.items = items; self }
}

impl RenderOnce for Menu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut bar = h_flex().h_full().items_center();
        for (ix, item) in self.items.iter().enumerate() {
            if item.separator() {
                bar = bar.child(Separator::vertical().h_full());
                continue;
            }
            let cmd = item.command();
            let disabled = item.disabled();
            let mut btn = Button::new((self.id.clone(), ix))
                .label(item.label())
                .ghost()
                .disabled(disabled);
            if let Some(icon) = item.icon() {
                btn = btn.icon(icon);
            }
            if let Some(cmd) = cmd {
                btn = btn.on_click(move |_, _window, cx| cmd.execute(&(), cx));
            }
            bar = bar.child(btn);
        }
        bar
    }
}
```

**`crates/ui/src/components/status_bar_wrapper.rs`** — 包装 gpui-component StatusBar

```rust
use std::sync::Arc;
use gpui::{IntoElement, RenderOnce, SharedString, Window, App};
use gpui_component::{StatusBar, h_flex, v_flex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusBarAlign { Left, Right, Center }

pub trait IStatusBarItem: 'static {
    fn content(&self) -> SharedString;
    fn align(&self) -> StatusBarAlign { StatusBarAlign::Left }
}

pub type StatusBarItems = Vec<Arc<dyn IStatusBarItem>>;

pub struct StatusBarItem {
    content: SharedString,
    align: StatusBarAlign,
}

impl StatusBarItem {
    pub fn new(content: impl Into<SharedString>) -> Self { ... }
    pub fn align(mut self, a: StatusBarAlign) -> Self { ... }
    pub fn into_arc(self) -> Arc<dyn IStatusBarItem> { Arc::new(self) }
}

impl IStatusBarItem for StatusBarItem { ... }

/// RML 状态栏包装（StatelessNoId: RmlStatusBar::new().items(items)）
#[derive(IntoElement)]
pub struct RmlStatusBar {
    items: StatusBarItems,
}

impl RmlStatusBar {
    pub fn new() -> Self { Self { items: Vec::new() } }
    pub fn items(mut self, items: StatusBarItems) -> Self { self.items = items; self }
}

impl Default for RmlStatusBar { fn default() -> Self { Self::new() } }

impl RenderOnce for RmlStatusBar {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut left = h_flex().items_center();
        let mut right = h_flex().items_center();
        for item in &self.items {
            match item.align() {
                StatusBarAlign::Left => left = left.child(item.content()),
                StatusBarAlign::Right => right = right.child(item.content()),
                StatusBarAlign::Center => {} // 简化：center 归入 left
            }
        }
        StatusBar::new().left(left).right(right)
    }
}
```

### 修改文件

**`crates/engine/src/tags.rs`**
- `component_lookup` 新增两个 lowercase 入口：
  ```rust
  "menu" => Some(ComponentTag { ctor_path: "rml_ui::Menu", kind: ComponentKind::Stateless }),
  "status_bar" => Some(ComponentTag { ctor_path: "rml_ui::RmlStatusBar", kind: ComponentKind::StatelessNoId }),
  ```
- 新增函数：
  ```rust
  pub fn is_special_lowercase_component(tag: &str) -> bool {
      matches!(tag, "menu" | "status_bar")
  }
  ```

**`crates/engine/src/compiler/codegen/mod.rs`** — `gen_element` L293-297 路由条件：
```rust
if tags::is_component(tag) || tags::is_special_lowercase_component(tag) {
    let code = comp::gen_component(elem, ctx, depth, id_counter, loop_vars)?;
    return Ok((code, false));
}
```

**`crates/engine/src/compiler/component.rs`**
- `component_bind_setter` L260-269 新增 `items` 分支：
  ```rust
  "items" if tag == "menu" || tag == "status_bar" => Some(format!(".items({}.clone())", rust_expr)),
  ```
- `is_container` L116-117 排除 menu/status_bar（它们通过 `items` 绑定数据，不接受 element 子节点）：
  ```rust
  let is_container = (matches!(component.kind, tags::ComponentKind::StatelessNoId) || tag == "ActivityBar")
      && tag != "menu" && tag != "status_bar";
  ```

**`crates/ui/src/components/mod.rs`** — 新增模块声明 + re-export
**`crates/ui/src/lib.rs`** — re-export `IMenuItem`/`Menu`/`MenuItem`/`MenuItems`/`IStatusBarItem`/`RmlStatusBar`/`StatusBarItem`/`StatusBarItems`/`StatusBarAlign`
**`crates/ui/src/prelude.rs`** — 同步 re-export

---

## Feature 3: 用户组件注册表（`#[component]` 嵌入）

### 数据结构

**`crates/engine/src/compiler/mod.rs`** — CodegenCtx 新增：

```rust
pub user_components: HashMap<String, UserComponentInfo>,

#[derive(Debug, Clone, Default)]
pub struct UserComponentInfo {
    pub struct_name: String,      // "CounterCase"
    pub entity_field: String,     // "counter_case"（snake_case）
}
```

### 变更文件

**`crates/engine/src/build/scanner.rs`**
- `StructMetadata` 新增 `pub is_component: bool` 字段
- 扫描时：`is_component = has_component_attr && !has_window_attr`（`#[window]` 标注的 struct 不算用户组件）

**`crates/engine/src/build/mod.rs`**
- 构建 `CodegenCtx` 时收集所有 `is_component == true` 的 struct → `user_components` HashMap
- key = struct_name（PascalCase），value = `UserComponentInfo { struct_name, entity_field: to_snake_case(struct_name) }`
- `to_snake_case` 函数已存在于 `build/mod.rs:464`

**`crates/engine/src/compiler/component.rs`**
- `gen_component` L37-42：`component_lookup` 未命中后，检查 `ctx.user_components`
- 新增 `gen_user_component` 函数（参照 Tree Stateful 模式 L67-70）：
  ```rust
  fn gen_user_component(info: &UserComponentInfo) -> String {
      let field = &info.entity_field;
      let struct_name = &info.struct_name;
      format!(
          "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
          field, struct_name
      )
  }
  ```
- 生成代码：`self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()`
- 返回 `Entity<CounterCase>`，因 `CounterCase: Render`（由 `#[component]` 生成），`Entity<T: Render>: IntoElement`

### 父视图模式

```rust
#[window]
pub struct MainWindow {
    counter_case: Option<Entity<CounterCase>>,
    two_way_case: Option<Entity<TwoWayCase>>,
    // ...
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window, cx) {
        if self.counter_case.is_none() {
            self.counter_case = Some(cx.new(|_| CounterCase::default()));
        }
        // ...
    }
}
```

RML：`<CounterCase />` → 生成 `self.counter_case.as_ref().expect(...).clone()`

---

## Feature 4: RML Namespace 支持

### 关键确认

- Parser tokenizer（`tokenizer.rs:174` `read_tag_name`）已接受 `:` 字符 → `<d3:chart/>` 可正确解析
- `rmlns:d3="gpui_d3rs"` 被解析为 `Attribute::Static { name: "rmlns:d3", value: "gpui_d3rs" }`
- Validator 不校验属性名 → 不会拒绝
- **问题**：`apply_static_attr`（`codegen/mod.rs:420-429`）会把 `rmlns:d3` 当未知属性生成 `.child("rmlns:d3=gpui_d3rs")` → 需显式跳过

### 变更文件

**`crates/engine/src/compiler/mod.rs`** — CodegenCtx 新增：

```rust
pub namespaces: HashMap<String, String>,  // {"d3" => "gpui_d3rs"}
```

**`crates/engine/src/compiler/codegen/mod.rs`**

1. `codegen` 函数：从根元素 `rmlns:*` 属性提取 namespace map，存入 ctx（在 `gen_render_impl_from_children` 之前）
   ```rust
   // 在 codegen() 函数中，gen_render_impl_from_children 调用前
   let mut ctx = ctx.clone();
   for attr in &elem.attributes {
       if let Attribute::Static { name, value } = attr {
           if let Some(prefix) = name.strip_prefix("rmlns:") {
               ctx.namespaces.insert(prefix.to_string(), value.clone());
           }
       }
   }
   ```

2. `apply_static_attr` L420-429：跳过 `rmlns:*` 属性
   ```rust
   "class" | "id" => String::new(),
   "ref" => String::new(),
   _ if name.starts_with("rmlns:") => String::new(),  // 新增
   "style" => apply_inline_style(value),
   ...
   ```

3. `gen_element` L293-297：在 `is_component`/`is_special_lowercase_component` 检查之前，检查 namespace 前缀
   ```rust
   // 命名空间组件：<d3:chart />
   if let Some((prefix, local_tag)) = tag.split_once(':') {
       if let Some(crate_name) = ctx.namespaces.get(prefix) {
           return gen_extern_component(elem, ctx, crate_name, local_tag, depth, id_counter, loop_vars);
       } else {
           return Err(CodegenError {
               message: format!("unknown namespace prefix: {}", prefix),
           });
       }
   }
   ```

4. 新增 `gen_extern_component` 函数：
   ```rust
   fn gen_extern_component(
       elem: &Element, ctx: &CodegenCtx, crate_name: &str, local_tag: &str,
       _depth: usize, id_counter: &mut usize, loop_vars: &[String],
   ) -> Result<GenResult, CodegenError> {
       let pascal = to_pascal_case(local_tag);  // chart → Chart
       let id_val = *id_counter;
       *id_counter += 1;
       let mut code = format!("{}::{}::new((\"rml_el\", {}usize))", crate_name, pascal, id_val);
       // 复用 component_static_setter / component_bind_setter / component_event_setter
       // 子节点走 .child()/.children() 路径
       ...
       Ok((code, false))
   }
   ```

### 生成代码示例

```xml
<tab_window rmlns:d3="gpui_d3rs" ...>
    <d3:chart data={chart_data} />
</tab_window>
```

生成：
```rust
gpui_d3rs::Chart::new(("rml_el", 0usize)).data(self.chart_data.clone())
```

---

## Feature 5: Demo 改造（依赖 Feature 1-3）

### 新增案例组件文件

| 文件 | struct | 状态字段 | 命令 |
|------|--------|---------|------|
| `demo/src/cases/counter_case.rml` + `.rml.rs` | `CounterCase` | `count: i32` | `on_click` |
| `demo/src/cases/two_way_case.rml` + `.rml.rs` | `TwoWayCase` | `name: String`, `age: i32`（带 `#[validate(range(min=0,max=150))]`） | 无（双向绑定） |
| `demo/src/cases/button_case.rml` + `.rml.rs` | `ButtonCase` | `button_clicks: i32` | `on_button_demo_click` |
| `demo/src/cases/i18n_case.rml` + `.rml.rs` | `I18nCase` | `i18n_version: u32` | `on_switch_en`/`on_toggle_theme` |

- welcome 案例无状态，保持内联在 main_window.rml
- 每个 case 的 `.rml` 根节点为 `<component>`，`.rml.rs` 用 `#[component]` 标注 struct
- `i18n_case.rml.rs` 在 `on_loaded` 中 `cx.observe_global::<I18nState>` 监听语言切换，bump `i18n_version`

### case .rml 模板示例（counter_case.rml）

```xml
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.counter.title")}</h2>
        <p class="count">{counter_text}</p>
        <Button ref="click_btn" label={t("demo.click_btn")} primary="" onclick={on_click} />
    </div>
</component>
```

### MainWindow 改造

**`demo/src/shell/main_window.rml.rs`**
- 移除 `count`/`name`/`age`/`button_clicks` 字段（迁移到各 case 组件）
- 新增 `counter_case: Option<Entity<CounterCase>>` / `two_way_case` / `button_case` / `i18n_case` 字段
- `on_loaded` 初始化各 case entity（`cx.new(|_| CounterCase::default())`）
- 新增 `menu_items: MenuItems` computed（返回 `MenuItem::new(...).command(...).into_arc()` 列表）
- 新增 `status_items: StatusBarItems` computed
- `on_switch_en`/`on_toggle_theme` 保留，包装为 `Arc<dyn ICommand>` 通过 `RelayCommand::new(cx, |this, cx| this.on_switch_en(...))` 在 `on_loaded` 中创建
- 移除 `counter_text`/`button_demo_text`/`profile_summary` computed（迁移到 case）

**`demo/src/shell/main_window.rml`**
- `<slot_menu>` 内容改为 `<menu items={menu_items} />`
- `<slot_footer>` 内容改为 `<status_bar items={status_items} />`
- case-host 的 `if` 块改为用户组件标签：
  ```xml
  <div if={active_case_id == "binding.counter"} class="case-pane">
      <CounterCase />
  </div>
  ```
  （或直接 `<CounterCase if={active_case_id == "binding.counter"} />`）

**`demo/src/cases/mod.rs`**
- 新增模块声明（由于 `.rml.rs` 双扩展名，需用 `#[path]`）：
  ```rust
  #[path = "counter_case.rml.rs"] pub mod counter_case;
  #[path = "two_way_case.rml.rs"] pub mod two_way_case;
  #[path = "button_case.rml.rs"] pub mod button_case;
  #[path = "i18n_case.rml.rs"] pub mod i18n_case;
  ```

---

## 实现顺序

```
Phase 1: Feature 1 续 (slot_title + slot_footer)   — 4 文件，~80 行（shell.rs 已完成）
Phase 2: Feature 2 (menu/status_bar)               — 2 新文件 + 6 修改，~400 行
Phase 3: Feature 3 (用户组件注册表)                  — 4 修改文件，~150 行
Phase 4: Feature 4 (namespace)                     — 2 修改文件，~100 行
Phase 5: Feature 5 (Demo 改造)                      — 8 新文件 + 3 修改，~500 行
Phase 6: 验证                                        — cargo build/test + 3 集成测试
```

Feature 2 和 3 都修改 `gen_component`，Feature 2 先做（内置 lowercase 路由），Feature 3 后做（用户组件 fallback）。
Feature 4 的 `gen_extern_component` 在 `gen_element` 更早的分支，复用 Feature 2 修改的 setter 函数。

---

## 验证方式

1. `cargo build --workspace` — 编译通过
2. `cargo test --workspace` — 现有测试不回归
3. 新增集成测试 `crates/engine/tests/codegen_menu_statusbar_test.rs`：
   - `<menu items={menu_items} />` 生成 `.items(self.menu_items().clone())`
   - `<status_bar items={status_items} />` 生成 `.items(self.status_items().clone())`
   - `<slot_title>`/`<slot_footer>` 正确分区到对应 builder
4. 新增集成测试 `crates/engine/tests/codegen_user_component_test.rs`：
   - `<CounterCase />` 生成 `self.counter_case.as_ref().expect(...).clone()`
5. 新增集成测试 `crates/engine/tests/codegen_namespace_test.rs`：
   - `<d3:chart data={x} />` 生成 `gpui_d3rs::Chart::new(...).data(self.x.clone())`
6. 运行 demo：`cargo run -p rust-rml-demo`，验证：
   - 菜单栏显示 `<menu>` 数据绑定项，点击执行命令
   - 状态栏显示 `<status_bar>` 数据绑定项
   - Tab 切换显示各独立案例组件，状态独立
   - `slot_title` 扩展区内容显示在 tab 栏右侧

---

## Assumptions & Decisions

1. **标签命名**：用户已确认 lowercase（`<menu>`/`<status_bar>` 非 PascalCase）
2. **组件嵌入机制**：用户已确认实现组件注册表（非 entity 字段绑定 workaround）
3. **Menu/StatusBar 不接受子节点**：仅通过 `items={...}` 绑定数据，`is_container` 排除
4. **命名空间组件构造**：假设外部库组件构造签名为 `Type::new(impl Into<ElementId>)`（与 gpui-component 一致），无 ID 的 RenderOnce 组件暂不支持
5. **welcome 案例保持内联**：无状态，不值得独立组件
6. **case 组件用 `Option<Entity<T>>`**：惰性初始化，`on_loaded` 中创建
7. **`#[component]` 宏无需改动**：已生成 `impl IComponent` + `include!`，RML 根节点 `<component>` 已支持
8. **RML 根节点 `<component>` 已由 `tags::is_root_tag` 识别**（`tags.rs:160-165`），codegen 已处理（`codegen/mod.rs:110` `RootTag::Component => {}`）
