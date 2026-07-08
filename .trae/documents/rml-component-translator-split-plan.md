# RML 组件 translator 拆分计划

## 背景与目标

当前 `translator/component/` 下仍存在两类集中式分发：

1. **`items.rs`** 用 `match canonical.as_str()` 把 Tabs / TabBar / Table / DescriptionList / Popover / Accordion 路由到各自 `gen_xxx`。
2. **`special.rs`** 用 `match canonical.as_str()` 把 Label / Separator / Icon / Kbd / Tag / Alert / RadioGroup / ActivityBar 路由到各自 `gen_xxx` 或内联构造。
3. **`stateful.rs`** 在 `to_rust` 顶部用 `if canonical == "Tree" / "CodeEditor"` 做特殊分支。
4. **`stateless.rs`** 的 `gen_stateless_body` 标记为 `pub(crate)`，但仅本模块使用。

这些集中式 match/if 分发违反"一个 rs 文件 = 一个组件 / 一个职责"原则，且与已迁移的 `UserComponentTranslator` 模式不一致。

本计划完成 Phase 6 的最终清理：消除所有集中式组件分发，为每个组件建立独立 translator 文件，使用户组件与扩展组件走统一的 per-component translator 架构。`gen_xxx` 仍保留在 `compiler/<component>/` 下（遵守"组件代码与 codegen 禁止同文件共存"约束），translator 仅作薄包装。

## 设计原则

- **per-component translator**：每个组件独占一个 rs 文件，文件内仅定义 `XxxTranslator` struct + `impl IRmlTranslator` + `register` 函数。构造逻辑通过调用 `compiler::<component>::gen_xxx` 委托。
- **保留 `compiler/<component>/gen.rs`**：codegen 逻辑与 translator 分离，符合项目"组件代码与 codegen 禁止同文件共存"铁律。
- **Stateless / Stateful 通用 translator 保留**：`StatelessComponentTranslator` / `StatefulComponentTranslator` 通过 `component_lookup + setter` 通用分发，不针对单个组件 match，属于合理的通用抽象，不拆分。但需移除 `Stateful` 中的 Tree / CodeEditor 特例分支。
- **`gen_stateless_body` / `gen_stateful_body` 改为模块私有**：移除 `pub(crate)`，与 `gen_user_component_body` 一致。

## 实施步骤

### 1. 拆分 `items.rs` 为 6 个独立 translator

在 `translator/component/` 下新建：

- `tabs.rs` → `TabsTranslator`，调用 `crate::compiler::tabs::gen_tabs`
- `tab_bar.rs` → `TabBarTranslator`，调用 `crate::compiler::tab_bar::gen_tab_bar`
- `table.rs` → `TableTranslator`，调用 `crate::compiler::table::gen_table`
- `description_list.rs` → `DescriptionListTranslator`，调用 `crate::compiler::description_list::gen_description_list`
- `popover.rs` → `PopoverTranslator`，调用 `crate::compiler::popover::gen_popover`
- `accordion.rs` → `AccordionTranslator`，调用 `crate::compiler::accordion::gen_accordion`

每个 translator 模板：

```rust
use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct TabsTranslator;

impl IRmlTranslator for TabsTranslator {
    fn tag(&self) -> &'static str { "Tabs" }
    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Tabs"
    }
    fn to_rust(...) -> Result<(String, bool), CodegenError> {
        let ref_name = /* extract ref */;
        let id_val = *id_counter; *id_counter += 1;
        let mut code = crate::compiler::tabs::gen_tabs(elem, ref_name, id_val, ctx, id_counter, loop_vars)?;
        // apply CSS via apply_css_styles
        Ok((code, false))
    }
    fn to_rml(...) -> Result<String, PrintError> { super::super::utils::print_element(elem, ctx) }
    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Tabs", "Tabs", ComponentCategory::Layout).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TabsTranslator);
}
```

删除 `translator/component/items.rs`。

### 2. 拆分 `special.rs` 为 8 个独立 translator

- `label.rs` → `LabelTranslator`，调用 `crate::compiler::label::gen_label`
- `separator.rs` → `SeparatorTranslator`，调用 `crate::compiler::separator::gen_separator`
- `icon.rs` → `IconTranslator`，调用 `crate::compiler::icon::gen_icon`
- `kbd.rs` → `KbdTranslator`，调用 `crate::compiler::kbd::gen_kbd`
- `tag.rs` → `TagTranslator`，调用 `crate::compiler::tag::gen_tag`
- `alert.rs` → `AlertTranslator`，调用 `crate::compiler::alert::gen_alert`
- `radio_group.rs` → `RadioGroupTranslator`，调用 `crate::compiler::radio_group::gen_radio_group`
- `activity_bar.rs` → `ActivityBarTranslator`，内联 EntityRef 构造（原 `gen_activity_bar` 移入此文件）

删除 `translator/component/special.rs`。

### 3. 从 `stateful.rs` 抽出 Tree / CodeEditor

- 新建 `translator/component/tree.rs` → `TreeTranslator`，调用 `crate::compiler::tree::gen_tree`
- 新建 `translator/component/code_editor.rs` → `CodeEditorTranslator`，调用 `crate::compiler::code_editor::gen_code_editor`

`stateful.rs` 的 `to_rust` 移除 Tree / CodeEditor 分支，仅保留通用 `gen_stateful_body` 路径。`StatefulComponentTranslator::matches` 无需改动（Tree/CodeEditor 由 dedicated translator 优先匹配，注册顺序保证）。

### 4. 收紧 stateless / stateful 模块私有性

- `stateless.rs`：`pub(crate) fn gen_stateless_body` → `fn gen_stateless_body`
- `stateful.rs`：`gen_stateful_body` 已是私有，无需改动

### 5. 更新 `translator/component/mod.rs`

```rust
pub mod accordion;
pub mod alert;
pub mod activity_bar;
pub mod code_editor;
pub mod description_list;
pub mod icon;
pub mod kbd;
pub mod label;
pub mod popover;
pub mod radio_group;
pub mod separator;
pub mod tab_bar;
pub mod table;
pub mod tabs;
pub mod tag;
pub mod tree;
pub mod stateful;
pub mod stateless;

// 保留 ComponentTranslator（透明容器）

pub fn register_all(registry: &mut TranslatorRegistry) {
    stateless::register(registry);
    stateful::register(registry);
    // items 系列
    tabs::register(registry);
    tab_bar::register(registry);
    table::register(registry);
    description_list::register(registry);
    popover::register(registry);
    accordion::register(registry);
    // special 系列
    label::register(registry);
    separator::register(registry);
    icon::register(registry);
    kbd::register(registry);
    tag::register(registry);
    alert::register(registry);
    radio_group::register(registry);
    activity_bar::register(registry);
    // 从 stateful 抽出
    tree::register(registry);
    code_editor::register(registry);
    // 透明容器
    registry.register(ComponentTranslator);
}
```

`stateless::register_all` / `stateful::register_all` 统一改名为 `register`，与其他 translator 一致。

### 6. 清理过时 doc 注释

- `stateless.rs` 顶部注释中"构造器特殊，委托到 compiler/tree 与 compiler/code_editor"删除（已迁出）
- `stateful.rs` 顶部注释同步更新
- `component.rs` 顶部注释更新为反映新架构
- 各 `compiler/<component>/gen.rs` 顶部"由 XxxTranslator 调用"注释更新

## 注册顺序与 matches 优先级

`TranslatorRegistry::resolve` 按 `HashMap` 迭代顺序返回首个 `matches == true` 的 translator。为避免歧义：

- `TabsTranslator::matches` 严格匹配 `canonical_tag == "Tabs"`，不会与 `TabBarTranslator` 冲突
- `TreeTranslator::matches` 匹配 `canonical_tag == "Tree"`，与 `StatefulComponentTranslator::matches`（`ComponentKind::Stateful`）重叠，但 `TreeTranslator` 注册后 `resolve` 仍可能先命中 `StatefulComponentTranslator`。

**风险**：HashMap 迭代顺序不确定。需确认 `resolve` 是否保证注册顺序优先。若不保证，需调整 `resolve` 为按注册顺序遍历（Vec 而非 HashMap），或让 `StatefulComponentTranslator::matches` 显式排除 Tree/CodeEditor。

**实施方案**：在 `StatefulComponentTranslator::matches` 中显式排除 Tree / CodeEditor：

```rust
fn matches(&self, elem: &Element) -> bool {
    let canonical = tags::canonical_tag(&elem.tag);
    if matches!(canonical.as_str(), "Tree" | "CodeEditor") {
        return false;
    }
    matches!(
        tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
        Some(tags::ComponentKind::Stateful { .. })
    )
}
```

同理 `StatelessComponentTranslator::matches` 无需排除（items/special 组件的 ComponentKind 不是 Stateless）。

## 关键文件

新建（16 个 translator 文件）：
- `crates/engine/src/compiler/translator/component/{tabs,tab_bar,table,description_list,popover,accordion}.rs`
- `crates/engine/src/compiler/translator/component/{label,separator,icon,kbd,tag,alert,radio_group,activity_bar}.rs`
- `crates/engine/src/compiler/translator/component/{tree,code_editor}.rs`

修改：
- `crates/engine/src/compiler/translator/component/mod.rs`（注册与新模块声明）
- `crates/engine/src/compiler/translator/component/stateless.rs`（`gen_stateless_body` 改私有，`register_all` → `register`）
- `crates/engine/src/compiler/translator/component/stateful.rs`（移除 Tree/CodeEditor 分支，`matches` 排除 Tree/CodeEditor，`register_all` → `register`）

删除：
- `crates/engine/src/compiler/translator/component/items.rs`
- `crates/engine/src/compiler/translator/component/special.rs`

## 验证

```bash
cargo check -p rust-rml-engine --lib
cargo test -p rust-rml-engine --lib
cargo clippy -p rust-rml-engine --lib
```

关键测试点：
- 各 items/special 组件的 `gen_xxx` 单元测试保持通过（codegen 函数未动）
- `translator::component::*::register` 注册后 `resolve` 能正确命中
- Tree / CodeEditor 不再走 `StatefulComponentTranslator`
- `<component content={...}>` 透明容器仍由 `ComponentTranslator` 处理

## 成功标准

- `items.rs` / `special.rs` 删除，无集中式 match 分发
- 每个 items/special/Tree/CodeEditor 组件独占一个 translator rs 文件
- `gen_stateless_body` 不再是 `pub(crate)`
- `StatefulComponentTranslator::matches` 显式排除 Tree / CodeEditor
- `cargo test -p rust-rml-engine --lib` 839+ 测试全部通过
- `cargo clippy` 无新增 warning
