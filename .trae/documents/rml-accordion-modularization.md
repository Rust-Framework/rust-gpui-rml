# RML Accordion codegen 模块化拆分计划

## 概述

用户澄清要求：组件支持应按职责范围模块化开发，避免单个 rs 文件堆积大量逻辑。当前 Accordion codegen 逻辑散落在 `component.rs`（1502 行）的 8 处位置，占比 ~23%。本计划将 Accordion 专属代码抽取到独立 `compiler/accordion/` 模块，参照 `menu/` 子目录的现有模式，实现职责单一、高内聚低耦合。

## 当前状态分析

### 问题：component.rs 过度膨胀

[component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 共 1502 行，承载所有扩展组件的构造、属性 setter、事件 setter。Accordion 代码散落在：

| 位置 | 内容 | 行数 |
|---|---|---|
| 74-82 | StatelessWithItems 构造分支 | ~9 |
| 154-181 | is_items_container 子节点处理 | ~28 |
| 210-261 | gen_item_builder 函数 | 52 |
| 466-481 | static_setter 中 5 个 Accordion 分支 | ~16 |
| 579-589 | bind_setter 中 3 个 Accordion 分支 | ~11 |
| 680-682 | event_setter 中 on_toggle_click | ~3 |
| 1407-1557 | 7 个 gen_component_accordion_* 测试 | ~150 |
| 1559-1640 | 9 个 *_accordion_* setter 测试 | ~80 |

**生产代码 ~120 行 + 测试 ~230 行 = ~350 行需迁移。**

### 现有模式：menu/ 子目录

[menu/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/mod.rs) 是唯一的按组件族分模块先例：
- `mod.rs` 暴露 `gen_menu_element()` 统一入口 + `is_menu_container`/`is_menu_tag` 识别函数
- 子模块各含 `gen_<type>()` 函数（context.rs / dropdown.rs / menu_bar.rs）
- `item.rs` 独立处理 MenuItem 子项 builder
- `hoist.rs` 提取公共辅助逻辑

Accordion 结构与 menu 高度相似（容器 + 子项 builder），适合沿用同一模式。

### 不需拆分的文件

| 文件 | Accordion 代码 | 理由 |
|---|---|---|
| `tags.rs` | StatelessWithItems 变体 + component_lookup + is_item_builder_tag（~20 行） | 中央路由表，按全组件注册表组织，拆分增加跳转成本 |
| `props_registry.rs` | Accordion/AccordionItem 属性注册（4 行） | 中央属性注册表，同上 |
| `validator.rs` | is_item_builder_tag 校验（1 行） | 中央校验逻辑，同上 |

## 提议变更

### 变更 1：新建 `compiler/accordion/` 模块

#### 1a. `compiler/accordion/mod.rs` — 入口与编排

**职责**：Accordion 容器的构造 + 属性处理 + 子节点分发

```rust
//! Accordion codegen —— 闭包式 builder 组件的代码生成。
//!
//! 将 `<Accordion><AccordionItem ...>...</AccordionItem></Accordion>` 转译为
//! `rml_ui::Accordion::new(id).multiple(true).item(|__rml_item| __rml_item.title(...).child(...))`。

mod item;
mod setters;

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;

/// 生成 Accordion 构造代码（构造 + 属性 + 子节点 .item() 注入）
///
/// 由 `component::gen_component` 在 `StatelessWithItems` 分支调用。
pub fn gen_accordion(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    // 1. 构造器
    let mut code = if let Some(name) = ref_name {
        format!("rml_ui::Accordion::new({:?})", format!("rml_ref:{}", name))
    } else {
        format!("rml_ui::Accordion::new((\"rml_el\", {}usize))", id_val)
    };

    // 2. 属性 → setter（先尝试 Accordion 专用，再回退到通用）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(s) = super::component::component_static_setter(name, value, "Accordion") {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr } => {
                if let Some(s) = super::component::component_bind_setter(name, expr, &lv, &computed, "Accordion") {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler } => {
                if let Some(s) = super::component::component_event_setter(name, handler, "Accordion") {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 子节点 → .item(|__rml_item| ...) 闭包
    for child in &elem.children {
        match child {
            Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                let item_code = item::gen_item_builder(child_elem, ctx, id_counter, loop_vars)?;
                code.push_str(&format!("\n            .item({})", item_code));
            }
            Node::Text(text) => {
                eprintln!("[rml warning] <Accordion> 不支持文本子节点 {:?}，已忽略", text);
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!("<Accordion> 仅支持 <AccordionItem> 子节点，得到 <{}>", child_elem.tag),
                });
            }
            _ => {}
        }
    }

    Ok(code)
}
```

**设计要点**：
- 属性处理调用 `super::component::component_*_setter`（公共分发函数），由后者委托到 `accordion::setters` 再回退到通用 setter
- 子节点处理调用 `item::gen_item_builder`
- 整个 Accordion codegen 流程自包含，component.rs 仅一行 `return accordion::gen_accordion(...)` 分发

#### 1b. `compiler/accordion/item.rs` — AccordionItem 闭包生成

**职责**：生成 `|__rml_item: rml_ui::AccordionItem| __rml_item.<setters>.child(...)` 闭包

```rust
//! AccordionItem 闭包式 builder 代码生成。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};

/// 为 `<AccordionItem>` 子节点生成闭包式 builder 代码
///
/// 生成形如：
/// ```text
/// |__rml_item: rml_ui::AccordionItem| __rml_item.title("Section 1").open(true).child("Content")
/// ```
pub fn gen_item_builder(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("|__rml_item: rml_ui::AccordionItem| __rml_item");

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(s) = super::super::component::component_static_setter(name, value, "AccordionItem") {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr } => {
                if let Some(s) = super::super::component::component_bind_setter(name, expr, &lv, &computed, "AccordionItem") {
                    code.push_str(&s);
                }
            }
            _ => {}
        }
    }

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", child_code));
        } else {
            code.push_str(&format!(".child({})", child_code));
        }
    }

    Ok(code)
}
```

#### 1c. `compiler/accordion/setters.rs` — Accordion 专用属性映射

**职责**：Accordion/AccordionItem 的 `multiple`/`bordered`/`open`/`icon`/`title`/`on_toggle_click` 属性 → builder 方法映射

```rust
//! Accordion / AccordionItem 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter` / `component_event_setter`
//! 在 tag 为 "Accordion" 或 "AccordionItem" 时委托调用。未命中返回 None，由公共 setter 回退到通用属性。

use crate::compiler::expr;
use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法
///
/// - `multiple=""` / `bordered=""` / `open=""` → `.multiple(true)` / `.bordered(true)` / `.open(true)`
/// - `icon="Settings"` → `.icon(rml_ui::IconName::Settings)`
/// - `title="Section 1"` → `.title("Section 1")`
pub fn static_setter(name: &str, value: &str, _tag: &str) -> Option<String> {
    match name {
        "multiple" | "bordered" | "open" => {
            let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".{}({})", name, bool_val))
        }
        "icon" => Some(format!(".icon(rml_ui::IconName::{})", value)),
        "title" => Some(format!(".title({:?})", value)),
        _ => None,
    }
}

/// 绑定属性 → builder 方法
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    _tag: &str,
) -> Option<String> {
    match name {
        "multiple" | "bordered" | "open" => {
            let rust_expr = super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        "title" | "icon" => {
            let rust_expr = super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        _ => None,
    }
}

/// 事件属性 → builder 方法
///
/// - `on_toggle_click={on_toggle}` → `.on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| { this.on_toggle(open_ixs, cx); }))`
pub fn event_setter(name: &str, handler: &EventHandler, _tag: &str) -> Option<String> {
    match name {
        "on_toggle_click" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {{\n                    \
                 this.{}(open_ixs, cx);\n                }}))",
                method
            ))
        }
        _ => None,
    }
}
```

#### 1d. `compiler/accordion/` 下的测试

将 component.rs 中 16 个 Accordion 测试迁移到各子模块的 `#[cfg(test)] mod tests`：
- `mod.rs`：7 个 `gen_component_accordion_*` 测试（构造 + 子节点 + 拒绝非 item 子节点）
- `setters.rs`：9 个 `*_accordion_*` setter 测试（static/bind/event）

### 变更 2：修改 `compiler/mod.rs` 注册新模块

**文件**：[compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)

```rust
pub mod accordion;  // 新增
pub mod codegen;
pub mod component;
// ...existing...
```

### 变更 3：精简 `compiler/component.rs` — 移除 Accordion 代码，添加委托

**文件**：[component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

#### 3a. gen_component 构造分支：StatelessWithItems → 委托到 accordion 模块

将 74-82 行的 StatelessWithItems 构造分支 + 154-181 行的 is_items_container 子节点处理，替换为在构造 match 中直接委托：

```rust
tags::ComponentKind::StatelessWithItems => {
    // Accordion 等闭包式 builder 组件：整个 codegen 流程委托到专属模块
    return crate::compiler::accordion::gen_accordion(
        elem, ref_name, id_val, ctx, id_counter, loop_vars,
    );
}
```

**删除**：
- 74-82 行 StatelessWithItems 构造分支
- 154-181 行 `is_items_container` 变量及其子节点处理分支
- 210-261 行 `gen_item_builder` 函数

#### 3b. 三个 setter 函数：移除 Accordion 分支，添加委托

**`component_static_setter`**（~418 行）：
- **删除** `"multiple" | "bordered" | "open"`、`"icon"`、`"title"` 三个 match 臂
- **在 match 之前添加委托**：
```rust
pub fn component_static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    // Accordion / AccordionItem 专用 setter 委托
    if tag == "Accordion" || tag == "AccordionItem" {
        if let Some(s) = super::accordion::setters::static_setter(name, value, tag) {
            return Some(s);
        }
    }
    match name {
        // ...existing common setters (label, primary, small, disabled, etc.)...
    }
}
```

**`component_bind_setter`**（~544 行）：
- **删除** `"multiple" | "bordered" | "open"`、`"title"`、`"icon"` 三个 match 臂
- **在 match 之前添加委托**：
```rust
pub fn component_bind_setter(name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str], tag: &str) -> Option<String> {
    if tag == "Accordion" || tag == "AccordionItem" {
        if let Some(s) = super::accordion::setters::bind_setter(name, expr_str, loop_vars, computed, tag) {
            return Some(s);
        }
    }
    match name {
        // ...existing common bind setters...
    }
}
```

**`component_event_setter`**（~618 行）：
- **删除** `"on_toggle_click" if tag == "Accordion"` match 臂
- **在 match 之前添加委托**：
```rust
pub fn component_event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    if tag == "Accordion" {
        if let Some(s) = super::accordion::setters::event_setter(name, handler, tag) {
            return Some(s);
        }
    }
    match name {
        // ...existing common event setters...
    }
}
```

#### 3c. 删除 Accordion 测试

删除 component.rs 测试模块中所有 `gen_component_accordion_*`（7 个）和 `*_accordion_*`（9 个）测试。这些测试迁移到 `accordion/` 模块。

### 变更 4：更新 `compiler/accordion/mod.rs` 的测试辅助

Accordion 测试中使用的 `make_element`、`ctx()` 等测试辅助函数当前在 component.rs 的测试模块中。需要：
- 若这些辅助函数是 `pub(crate)` 或 `pub(super)`，直接引用
- 若是私有的，在 accordion 测试模块中复制（测试辅助函数通常很小）

**探索发现**：需确认 `make_element` 和 `ctx()` 的可见性，若不可见则提取到共享测试辅助模块或在 accordion 测试内重新定义。

## 假设与决策

| 决策点 | 选项 | 决策 | 理由 |
|---|---|---|---|
| 模块结构 | A. 单文件 `accordion.rs`<br>B. 子目录 `accordion/` (mod.rs + item.rs + setters.rs) | **B** | 与 menu/ 模式一致；item 生成与 setter 映射职责不同，分文件更清晰 |
| 属性处理位置 | A. gen_accordion 内自处理属性循环<br>B. gen_component 处理属性，gen_accordion 仅构造+子节点 | **A** | 整个 codegen 流程自包含，component.rs 仅一行委托，最大化解耦 |
| setter 委托方向 | A. 公共 setter 委托到 accordion::setters<br>B. accordion::setters 委托到公共 setter | **A** | 公共 setter 是分发入口，accordion 专用 setter 是实现；未命中时回退到通用属性（Sizable 等） |
| `component_bind_rust_expr` 可见性 | A. 保持 pub<br>B. 改为 pub(crate) | **A** | accordion::setters 需调用此函数，保持 pub 无副作用 |
| tags.rs / props_registry.rs / validator.rs 是否拆分 | A. 拆分<br>B. 不拆分 | **B** | 中央注册表，Accordion 仅占几行，拆分增加跳转成本 |

## 验证步骤

### 1. 编译验证

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo check -p rust-rml-engine --lib 2>&1"
```

**预期**：无错误。检查无 `unused import` / `unused function` 警告。

### 2. Accordion 单元测试

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p rust-rml-engine --lib accordion 2>&1"
```

**预期**：16 个测试全部通过（7 个 codegen + 9 个 setter），分布在 `accordion::tests` 和 `accordion::setters::tests`。

### 3. Engine 全量测试回归

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p rust-rml-engine --lib 2>&1"
```

**预期**：240 passed, 0 failed（与拆分前一致）。

### 4. 行数验证

拆分后 component.rs 应减少 ~350 行（从 1502 → ~1150），accordion/ 三个文件合计 ~350 行。

## 实施顺序

1. **变更 2**：`compiler/mod.rs` 添加 `pub mod accordion;`
2. **变更 1c**：创建 `accordion/setters.rs`（setter 实现 + 测试）
3. **变更 1b**：创建 `accordion/item.rs`（gen_item_builder）
4. **变更 1a**：创建 `accordion/mod.rs`（gen_accordion 入口 + 测试）
5. **变更 3a**：component.rs gen_component StatelessWithItems 分支改为委托
6. **变更 3b**：component.rs 三个 setter 函数移除 Accordion 分支 + 添加委托
7. **变更 3c**：删除 component.rs 中 16 个 Accordion 测试
8. **变更 4**：确认测试辅助函数可见性，必要时调整
9. **验证 1-4**：编译 + 测试 + 行数检查

## 文件变更清单

| 文件 | 类型 | 行数变化 |
|---|---|---|
| `crates/engine/src/compiler/mod.rs` | 修改 | +1 |
| `crates/engine/src/compiler/accordion/mod.rs` | 新增 | ~110（含测试） |
| `crates/engine/src/compiler/accordion/item.rs` | 新增 | ~60 |
| `crates/engine/src/compiler/accordion/setters.rs` | 新增 | ~130（含测试） |
| `crates/engine/src/compiler/component.rs` | 修改 | -350（移除 Accordion 代码+测试，+15 委托代码） |

净效果：component.rs 减重 ~335 行，Accordion 逻辑内聚到独立模块。
