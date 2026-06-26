# Phase 1 收尾执行计划

> 本计划聚焦于完成已批准的 `window-api-revision-and-phase1-completion-plan.md` 的剩余 Step 7-11。
> Step 1-6 已完成（app crate 修复 + crates/ui window 模块创建），见前述计划的"当前状态分析"。
>
> 用户的两条澄清已在前述计划"概念关系澄清"中明确体现：
> 1. **Window/ModernWindow 双层关系**：`.rml` 中既可手动组装 `<TitleBar>+<StatusBar>`，也可使用封装好的 `<ModernWindow>`，后者代码更少、更现代
> 2. **窗口打开 API 风格**：`Window::new(title, w, h).open::<V>(cx)` 和 `ModernWindow::new(title, w, h).open::<V>(cx)`，WPF 风格对象 API，不再用 helper 函数

---

## 一、剩余工作概览 Remaining Work Overview

| # | 范围 | 文件 | 状态 |
|---|------|------|------|
| Step 7 | tags.rs 路由扩展 | `crates/engine/src/tags.rs` | ⬜ 未做 |
| Step 8 | component.rs setter 扩展 | `crates/engine/src/compiler/component.rs` | ⬜ 未做 |
| Step 9 | demo main.rs 迁移 | `demo/src/main.rs` | ⬜ 未做 |
| Step 10 | demo counter.rml + .rml.rs 迁移 | `demo/src/counter.rml` + `counter.rml.rs` | ⬜ 未做 |
| Step 11 | 全量验证 | `cargo build --workspace` + `cargo test` + `cargo run` | ⬜ 未做 |

---

## 二、Step 7：扩展 tags.rs 路由

**文件**：`crates/engine/src/tags.rs`

### 7.1 在 `ComponentKind` 枚举中新增 `StatelessNoId` 变体

**原因**：`TitleBar`/`StatusBar`/`ModernWindow` 的 `new()` **无参数**，不匹配现有 `Stateless`（`Type::new(id)`）和 `Stateful`（`Type::new(&self.field)`）。

**变更**：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// 无状态组件：构造调用形如 `Button::new(id)`
    /// `id: impl Into<ElementId>` — 由 codegen 自动分配 `("rml_el", N)` 元组
    Stateless,
    /// 无状态无 ID 组件：构造调用形如 `TitleBar::new()`
    /// 适用于 new() 无参数的 RenderOnce 组件（TitleBar/StatusBar/ModernWindow）
    StatelessNoId,
    /// 有状态组件：构造调用形如 `Input::new(&self.<field>)`
    /// 需要视图中持有对应 state entity 字段（如 `Entity<InputState>`）
    Stateful { state_field: &'static str },
}
```

### 7.2 在 `component_lookup` 中新增 3 个路由

**注**：`Kbd` 的 `new(stroke: Keystroke)` 签名特殊，留 Phase 2 处理。Phase 1 仅添加 `TitleBar`/`StatusBar`/`ModernWindow`。

**变更**：在 `_ => None` 之前插入：

```rust
"TitleBar" => Some(ComponentTag {
    ctor_path: "rml_ui::TitleBar",
    kind: ComponentKind::StatelessNoId,
}),
"StatusBar" => Some(ComponentTag {
    ctor_path: "rml_ui::StatusBar",
    kind: ComponentKind::StatelessNoId,
}),
"ModernWindow" => Some(ComponentTag {
    ctor_path: "rml_ui::ModernWindow",
    kind: ComponentKind::StatelessNoId,
}),
```

### 7.3 验证

```bash
cargo build -p rust-rml-engine
```
预期：编译通过（`StatelessNoId` 变体暂未在 component.rs 中处理，但枚举本身合法）。

---

## 三、Step 8：扩展 component.rs setter

**文件**：`crates/engine/src/compiler/component.rs`

### 8.1 在 `gen_component` 的 match 中处理 `StatelessNoId`

**位置**：第 53-66 行的 `match component.kind` 块

**变更**：在 `Stateless` 和 `Stateful` 分支之间插入 `StatelessNoId` 分支：

```rust
let mut code = match component.kind {
    tags::ComponentKind::Stateless => { /* 现有代码 */ }
    tags::ComponentKind::StatelessNoId => {
        // TitleBar::new() / StatusBar::new() / ModernWindow::new() —— 无参数构造
        format!("{}::new()", component.ctor_path)
    }
    tags::ComponentKind::Stateful { state_field } => { /* 现有代码 */ }
};
```

**注**：`StatelessNoId` 组件不需要 `ref` 指令处理，因为 `new()` 无 ID 参数。

### 8.2 修改 `component_bind_setter` 签名添加 `tag: &str` 参数

**位置**：第 188 行

**变更**：

```rust
// 旧
pub fn component_bind_setter(
    name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str]
) -> Option<String> {

// 新
pub fn component_bind_setter(
    name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str], tag: &str
) -> Option<String> {
```

### 8.3 在 `component_bind_setter` 中添加 ModernWindow 专用 setter

**位置**：match 块中，在 `_ => None` 之前插入：

```rust
match name {
    // 现有 setter
    "value" => Some(format!(".value({}.clone())", rust_expr)),
    "disabled" => Some(format!(".disabled({})", rust_expr)),
    "selected" => Some(format!(".selected({})", rust_expr)),
    "checked" => Some(format!(".selected({})", rust_expr)),
    "label" => Some(format!(".label({}.clone())", rust_expr)),

    // ModernWindow 专用 setter
    "menu" if tag == "ModernWindow" => Some(format!(".menu({})", rust_expr)),
    "status_bar" if tag == "ModernWindow" => Some(format!(".status_bar({})", rust_expr)),
    "title" if tag == "ModernWindow" => Some(format!(".title({})", rust_expr)),

    _ => None,
}
```

### 8.4 在 `component_static_setter` 中添加 ModernWindow title setter

**位置**：第 167-176 行附近

**变更**：在 `"disabled"` 分支后，添加：

```rust
"title" if tag == "ModernWindow" => {
    Some(format!(".title({:?})", value))
}
```

**注**：`title` 静态属性仅对 `ModernWindow` 有效，对其他组件返回 None。

### 8.5 更新 `gen_component` 中对 `component_bind_setter` 的调用

**位置**：第 85 行附近

**变更**：

```rust
// 旧
if let Some(setter) = component_bind_setter(name, expr, &lv, &computed) {

// 新
if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, tag) {
```

### 8.6 更新所有测试中的 `component_bind_setter` 调用

**位置**：tests 模块（第 504-576 行）

**变更**：所有 `component_bind_setter(...)` 测试调用需添加第 5 个参数 `tag`：

```rust
// 旧
let code = component_bind_setter("value", "count", &[], &[]).unwrap();

// 新
let code = component_bind_setter("value", "count", &[], &[], "Button").unwrap();
```

需更新的测试（按文件中顺序）：
- `bind_setter_value` / `bind_setter_value_with_expr` / `bind_setter_value_with_member_access`
- `bind_setter_disabled_with_expr` / `bind_setter_label_with_expr`
- `bind_setter_disabled` / `bind_setter_selected` / `bind_setter_checked_maps_to_selected`
- `bind_setter_label` / `bind_setter_unknown_returns_none` / `bind_setter_loop_var`

**新增测试**（验证 ModernWindow 专用 setter）：

```rust
#[test]
fn bind_setter_modern_window_menu() {
    let code = component_bind_setter("menu", "menu_items", &[], &[], "ModernWindow").unwrap();
    assert_eq!(code, ".menu(self.menu_items)");
}

#[test]
fn bind_setter_modern_window_status_bar() {
    let code = component_bind_setter("status_bar", "status_items", &[], &[], "ModernWindow").unwrap();
    assert_eq!(code, ".status_bar(self.status_items)");
}

#[test]
fn bind_setter_modern_window_title() {
    let code = component_bind_setter("title", "window_title", &[], &[], "ModernWindow").unwrap();
    assert_eq!(code, ".title(self.window_title)");
}

#[test]
fn bind_setter_menu_only_for_modern_window() {
    // menu setter 仅对 ModernWindow 有效，Button 应返回 None
    assert!(component_bind_setter("menu", "menu_items", &[], &[], "Button").is_none());
}

#[test]
fn static_setter_modern_window_title() {
    let code = component_static_setter("title", "My App", "ModernWindow").unwrap();
    assert_eq!(code, ".title(\"My App\")");
}

#[test]
fn static_setter_title_only_for_modern_window() {
    // title 静态属性仅对 ModernWindow 有效，Button 应返回 None
    assert!(component_static_setter("title", "My App", "Button").is_none());
}
```

### 8.7 新增 StatelessNoId 构造测试

```rust
#[test]
fn gen_component_titlebar_no_args() {
    let elem = make_element("TitleBar", vec![], vec![]);
    let mut id = 0;
    let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
    assert!(code.contains("rml_ui::TitleBar::new()"));
    // StatelessNoId 不使用计数器 ID
    assert!(!code.contains("rml_el"));
    // 计数器不应被消耗
    assert_eq!(id, 0);
}

#[test]
fn gen_component_statusbar_no_args() {
    let elem = make_element("StatusBar", vec![], vec![]);
    let mut id = 0;
    let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
    assert!(code.contains("rml_ui::StatusBar::new()"));
    assert_eq!(id, 0);
}

#[test]
fn gen_component_modern_window_with_setters() {
    let elem = make_element(
        "ModernWindow",
        vec![
            Attribute::Static {
                name: "title".into(),
                value: "My App".into(),
            },
            Attribute::Bind {
                name: "menu".into(),
                expr: "menu_items".into(),
            },
            Attribute::Bind {
                name: "status_bar".into(),
                expr: "status_items".into(),
            },
        ],
        vec![],
    );
    let mut id = 0;
    let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
    assert!(code.contains("rml_ui::ModernWindow::new()"));
    assert!(code.contains(".title(\"My App\")"));
    assert!(code.contains(".menu(self.menu_items)"));
    assert!(code.contains(".status_bar(self.status_items)"));
}
```

### 8.8 验证

```bash
cargo build -p rust-rml-engine
cargo test -p rust-rml-engine
```
预期：编译通过 + 180+ 现有测试通过 + 9 个新增测试通过。

---

## 四、Step 9：迁移 demo/src/main.rs

**文件**：`demo/src/main.rs`

### 9.1 变更：迁移到 IAppLifecycle + ModernWindow::new().open()

**当前代码**（旧 API，编译失败）：

```rust
use gpui::px;
use rml_app::RmlApplication;

#[path = "counter.rml.rs"]
mod counter;
#[path = "todos.rml.rs"]
mod todos;

fn main() {
    RmlApplication::new()
        .title("RML Todos Demo")
        .size(px(400.), px(400.))
        .run::<todos::Todos>();
}
```

**新代码**：

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

use gpui::{App, px};
use rml_app::{IAppLifecycle, ModernWindow, RmlApplication};

#[path = "counter.rml.rs"]
mod counter;
#[path = "todos.rml.rs"]
mod todos;

fn main() {
    RmlApplication::new().run::<DemoApp>();
}

#[derive(Default)]
struct DemoApp;

impl IAppLifecycle for DemoApp {
    fn on_launch(&mut self, cx: &mut App) {
        // 主窗口使用 ModernWindow 对象 + Counter 视图
        // Counter 的 .rml 根元素是 <ModernWindow>，因此窗口需透明标题栏
        ModernWindow::new("RML Counter Demo", px(400.), px(500.))
            .open::<counter::Counter>(cx);
    }
}
```

### 9.2 验证

```bash
cargo build -p rust-rml-demo
```
预期：编译通过（counter.rml.rs 还需要 Step 10 的更新才能完全编译）。

---

## 五、Step 10：迁移 counter.rml + counter.rml.rs

### 10.1 `demo/src/counter.rml`

**当前代码**：根元素是 `<div class="counter">`

**新代码**：根元素改为 `<ModernWindow>`，添加 menu/status_bar 绑定

```html
<ModernWindow title="RML 计数器" menu={menu_items} status_bar={status_items}>
    <div class="counter">
        <h1 ref="title">计数器</h1>
        <p class="count">{count}</p>
        <p class="next">下一个：{count + 1}</p>
        <p class="double">两倍（计算属性）：{double_count}</p>
        <p class="positive">是否为正：{count > 0}</p>
        <p class="hover">悬停状态：{hovered | BoolToYesNo}</p>
        <p once>版本: v0.1.0 (固定不变)</p>
        <div class="buttons" onhover={on_hover_change}>
            <Button ref="dec_btn" label="-" onclick={decrement} />
            <Button ref="inc_btn" label="+" primary="" onclick={increment} />
        </div>
    </div>
</ModernWindow>
```

### 10.2 `demo/src/counter.rml.rs`

**当前代码**：ViewModel 只有 `count`/`hovered` 字段

**新代码**：新增 `menu_items: Vec<MenuItem>` / `status_items: Vec<StatusBarItem>` 字段 + build_menu / build_status 方法

```rust
use rml::prelude::*;
use rml_ui::{MenuItem, StatusBarItem};

#[derive(Default)]
#[view]
pub struct Counter {
    pub count: i32,
    pub hovered: bool,
    pub menu_items