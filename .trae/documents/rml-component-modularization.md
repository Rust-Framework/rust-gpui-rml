# RML 组件支持模块化重构计划

## 摘要

将 `crates/engine/src/compiler/component.rs`（1549 行）中散落的各组件特定逻辑按职责提取到独立模块，建立"每个组件拥有自己的 codegen 模块"的统一模式。修正 Accordion 模块的倒置委托缺陷，使 `component.rs` 仅保留公共 dispatcher + 共享属性 setter。

## 现状分析

### component.rs 当前职责混杂（1549 行）

| 行号 | 内容 | 职责归属 | 问题 |
|------|------|---------|------|
| L29-182 | `gen_component` 主分发器 | 公共 | 应保留，但构造器分支含 Tree 特殊逻辑 |
| L202-267 | `gen_user_component` | 用户组件 slot 注入 | 126 行单一关注点，应提取 |
| L275-297 | `partition_user_component_children` | 用户组件 slot | 同上 |
| L304-327 | `gen_slot_content` | 用户组件 slot | 同上 |
| L339-421 | `component_static_setter` | 公共 + 组件特定 | L390-402 Accordion 残留 |
| L465-529 | `component_bind_setter` | 公共 + 组件特定 | L496-499 menu items、L501-514 Accordion 残留 |
| L539-616 | `component_event_setter` | 公共 + 组件特定 | L574 Input onchange、L586-600 Tree on_activate、L601-613 Accordion 残留 |
| L1330-1548 | 16 个 Accordion 测试 | 应在 accordion/ 模块 | 重复（accordion/mod.rs 已有 9 个测试） |

### Accordion 模块缺陷（倒置委托）

`accordion/mod.rs` L42-66 和 `accordion/item.rs` L28-47 调用 `super::component::component_*_setter`（即 component.rs 的公共 setter），而**未调用** `accordion/setters.rs` 中已实现的 `static_setter`/`bind_setter`/`event_setter`。

结果：
- `accordion/setters.rs` 是**死代码**（无人调用）
- `component.rs` 的 Accordion setter 臂与 `setters.rs` **双份实现**
- 公共 setter 函数仍承担 Accordion 特定职责，违反单一职责

### menu/ 模块先例

`menu/` 目录自包含 codegen（`gen_menu_element` 按 tag 分发到子文件），不依赖 `component.rs` 的 setter。但 `component_bind_setter` L496-499 仍有 `items` 臂为 menu/MenuBar/status_bar 残留。

### 已注册组件（tags.rs）

- **Stateless**：Button、ButtonGroup、Badge、Checkbox、Label、Separator、Tag、Progress、ProgressCircle、Slider、Switch、MenuBar、menu
- **Stateful**：Input、TextInput（input_state）、Tree（tree_state）
- **StatelessNoId**：TitleBar、StatusBar、status_bar
- **EntityRef**：ActivityBar
- **StatelessWithItems**：Accordion

## 设计原则

1. **每个拥有特定逻辑的组件 → 独立模块**（`<component>/mod.rs` + 可选 `setters.rs`/`item.rs`）
2. **component.rs 三大 setter 函数 → dispatcher 模式**：先按 tag 委托到组件模块的 setter，未命中回退到公共 match 臂
3. **公共 setter 臂**：仅保留所有组件共享的属性（label、placeholder、Sizable、font、layout、disabled、onclick 等）
4. **测试随实现走**：每个组件模块内联自己的 `#[cfg(test)] mod tests`，component.rs 仅保留公共 setter 测试

## 变更清单

### 变更 1：修正 Accordion 倒置委托（CRITICAL）

**文件**：`crates/engine/src/compiler/accordion/mod.rs`

将 L42-66 的属性处理改为先调 `self::setters::*`，未命中回退到 `super::component::component_*_setter`：

```rust
for attr in &elem.attributes {
    match attr {
        Attribute::Static { name, value } => {
            if let Some(s) = self::setters::static_setter(name, value, "Accordion") {
                code.push_str(&s);
            } else if let Some(s) =
                super::component::component_static_setter(name, value, "Accordion")
            {
                code.push_str(&s);
            }
        }
        Attribute::Bind { name, expr } => {
            if let Some(s) = self::setters::bind_setter(name, expr, &lv, &computed, "Accordion") {
                code.push_str(&s);
            } else if let Some(s) = super::component::component_bind_setter(
                name, expr, &lv, &computed, "Accordion",
            ) {
                code.push_str(&s);
            }
        }
        Attribute::Event { name, handler } => {
            if let Some(s) = self::setters::event_setter(name, handler, "Accordion") {
                code.push_str(&s);
            } else if let Some(s) =
                super::component::component_event_setter(name, handler, "Accordion")
            {
                code.push_str(&s);
            }
        }
    }
}
```

**文件**：`crates/engine/src/compiler/accordion/item.rs`

L28-47 同理改为先调 `super::setters::*`（AccordionItem 共享 accordion/setters.rs），未命中回退到 `super::super::component::component_*_setter`。

### 变更 2：清理 component.rs 的 Accordion 残留（CRITICAL）

**文件**：`crates/engine/src/compiler/component.rs`

- **`component_static_setter`（L390-402）**：删除 `"multiple" | "bordered" | "open"`、`"icon"`、`"title"` 三个 match 臂
- **`component_bind_setter`（L500-514）**：删除 `"multiple" | "bordered" | "open"`、`"title"`、`"icon"` 三个 match 臂
- **`component_event_setter`（L601-613）**：删除 `"on_toggle_click" if tag == "Accordion"` 臂
- **测试块（L1328-1548）**：删除 16 个 `gen_component_accordion_*` 测试函数及 `StatelessWithItems 构造（Accordion）` 分隔注释

### 变更 3：提取 user_component + slot 逻辑到独立模块

**新文件**：`crates/engine/src/compiler/user_component.rs`

迁移以下 3 个函数（共 126 行）：
- `gen_user_component`（L202-267）
- `partition_user_component_children`（L275-297）
- `gen_slot_content`（L304-327）

函数签名保持不变，改为 `pub`。在 `component.rs` 中：
- 删除这 3 个函数的定义
- `gen_component` L44 的调用改为 `super::user_component::gen_user_component(info, elem, ctx, id_counter, loop_vars)`

**文件**：`crates/engine/src/compiler/mod.rs`

添加 `pub mod user_component;`（在 `pub mod component;` 之后）。

### 变更 4：迁移 menu items 绑定到 menu/ 模块

**文件**：`crates/engine/src/compiler/menu/mod.rs`

新增公开函数：

```rust
/// menu/MenuBar/status_bar 专用 bind setter
/// items={expr} → .items(self.<expr>.clone())
pub fn bind_setter(name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str], tag: &str) -> Option<String> {
    match name {
        "items" if matches!(tags::normalize_component_tag(tag).as_str(), "menu" | "MenuBar" | "status_bar") => {
            let rust_expr = super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".items({}.clone())", rust_expr))
        }
        _ => None,
    }
}
```

**文件**：`crates/engine/src/compiler/component.rs`

- `component_bind_setter` L496-499：删除 `items` 臂
- 在函数开头添加委托：

```rust
if let Some(s) = super::menu::bind_setter(name, expr_str, loop_vars, computed, tag) {
    return Some(s);
}
```

### 变更 5：提取 Input/TextInput 到 input/ 模块

**新文件**：`crates/engine/src/compiler/input/mod.rs`

```rust
//! Input / TextInput 组件 codegen —— 事件处理专用。
//!
//! 当前仅包含 on_change 事件 setter，构造器仍由 gen_component 的 Stateful 分支统一处理。

pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "onchange" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| {{\n                    \
                 this.{}(state, cx);\n                }}))",
                method
            ))
        }
        _ => None,
    }
}
```

**文件**：`crates/engine/src/compiler/mod.rs`

添加 `pub mod input;`。

**文件**：`crates/engine/src/compiler/component.rs`

- `component_event_setter` L574-585：删除 `onchange if tag == "Input" || tag == "TextInput"` 臂
- 在函数开头添加委托：

```rust
if tag == "Input" || tag == "TextInput" {
    if let Some(s) = super::input::event_setter(name, handler, tag) {
        return Some(s);
    }
}
```

### 变更 6：提取 Tree 到 tree/ 模块

**新文件**：`crates/engine/src/compiler/tree/mod.rs`

```rust
//! Tree 组件 codegen —— 构造器与事件处理。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Element, EventHandler};
use crate::tags;

/// 生成 Tree 构造代码
/// Tree::new(self.<state_field>.as_ref()) —— 使用 as_ref() 而非 & 引用
pub fn gen_tree(
    elem: &Element,
    component: &tags::ComponentInfo,
    ctx: &CodegenCtx,
    _depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let state_field = match &component.kind {
        tags::ComponentKind::Stateful { state_field } => state_field,
        _ => return Err(CodegenError {
            message: format!("<Tree> component kind mismatch"),
        }),
    };
    let mut code = format!("rml_ui::Tree::new(self.{}.as_ref())", state_field);

    // 属性处理：委托到公共 setter（Tree 无特定 static/bind setter）
    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            crate::parser::ast::Attribute::Static { name, value } => {
                if let Some(s) = super::component::component_static_setter(name, value, &resolved) {
                    code.push_str(&s);
                }
            }
            crate::parser::ast::Attribute::Bind { name, expr } => {
                if let Some(s) = super::component::component_bind_setter(name, expr, &lv, &computed, &resolved) {
                    code.push_str(&s);
                }
            }
            crate::parser::ast::Attribute::Event { name, handler } => {
                if let Some(s) = self::event_setter(name, handler, &resolved) {
                    code.push_str(&s);
                } else if let Some(s) = super::component::component_event_setter(name, handler, &resolved) {
                    code.push_str(&s);
                }
            }
        }
    }
    Ok(code)
}

/// Tree 专用事件 setter
/// on_activate={fn} → .on_activate_rc(Rc::new({ let weak = cx.weak_entity(); move |item, ...| {...} }))
pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "on_activate" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_activate_rc(std::rc::Rc::new({{\n                    \
                 let weak = cx.weak_entity();\n                    \
                 move |item: rml_ui::TreeItem, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                 if let Some(entity) = weak.upgrade() {{\n                            \
                 entity.update(app, |this, cx| {{ this.{}(&item.id, cx); }});\n                        \
                 }}\n                    }}\n                }}))",
                method
            ))
        }
        _ => None,
    }
}
```

**文件**：`crates/engine/src/compiler/mod.rs`

添加 `pub mod tree;`。

**文件**：`crates/engine/src/compiler/component.rs`

- `gen_component` L89-92：删除 `Stateful { state_field } if tag == "Tree"` 构造分支，改为：

```rust
tags::ComponentKind::Stateful { state_field } if tag == "Tree" => {
    return crate::compiler::tree::gen_tree(elem, component, ctx, _depth, id_counter, loop_vars);
}
```

- `component_event_setter` L586-600：删除 `on_activate if tag == "Tree"` 臂（Tree 事件已在 `tree::gen_tree` 内处理）

## 实施步骤

1. **变更 1**：修正 `accordion/mod.rs` 和 `accordion/item.rs` 的倒置委托（先调 `self::setters`，回退 `super::component`）
2. **变更 2**：清理 `component.rs` 的 Accordion setter 臂（3 处）+ 删除 16 个 Accordion 测试
3. **变更 3**：创建 `user_component.rs`，迁移 3 个函数，更新 `mod.rs` 和 `component.rs` 引用
4. **变更 4**：在 `menu/mod.rs` 新增 `bind_setter`，删除 `component.rs` 的 `items` 臂并添加委托
5. **变更 5**：创建 `input/mod.rs`，删除 `component.rs` 的 `onchange` 臂并添加委托
6. **变更 6**：创建 `tree/mod.rs`，修改 `component.rs` 的 Tree 构造分支为委托，删除 `on_activate` 臂
7. **更新 mod.rs**：添加 `pub mod input;`、`pub mod tree;`、`pub mod user_component;`

## 假设与决策

1. **测试策略**：遵循 Rust 惯例，每个模块内联自己的 `#[cfg(test)] mod tests`，不拆到独立测试文件
2. **Input/Tree 提取阈值**：虽然当前各只有 1 个事件 setter，但按"充分模块化"原则仍提取独立模块，为未来扩展预留结构
3. **构造器归属**：
   - Tree 构造器有特殊 `as_ref()` 逻辑 → 提取到 `tree::gen_tree`
   - Input 构造器使用通用 `Stateful` 分支 → 保留在 `gen_component`，仅提取事件 setter
4. **公共 setter 保留范围**：Button variant、Sizable、font、layout、disabled/selected、label/placeholder/tooltip、content/value/checked bind、onclick event、parse_bool
5. **tags.rs 不变**：组件注册表和 ComponentKind 枚举保持现状，仅调整 codegen 层组织

## 验证步骤

1. `cargo check -p rust-rml-engine --lib` —— 编译通过，无错误
2. `cargo test -p rust-rml-engine --lib` —— 全部 240 个测试通过（Accordion 测试从 component.rs 迁移到 accordion/ 模块，总数不变）
3. `cargo test -p rust-rml-engine --lib accordion` —— accordion 模块测试通过（9 个 + setters.rs 的 11 个）
4. `cargo test -p rust-rml-engine --lib input` —— input 模块测试通过
5. `cargo test -p rust-rml-engine --lib tree` —— tree 模块测试通过
6. `cargo clippy -p rust-rml-engine --lib` —— 无新警告（特别是无 dead_code 警告，确认 setters.rs 不再是死代码）

## 重构后 component.rs 预期规模

- 生产代码：~450 行（gen_component 分发器 + 3 个公共 setter dispatcher + parse_bool）
- 测试代码：~450 行（公共 setter 测试，删除 16 个 Accordion 测试）
- 总计：~900 行（从 1549 行降至 ~900 行，减少 42%）
