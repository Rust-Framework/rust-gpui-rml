# Table 组件实现补充计划（插槽模板 + 小写标签 + codegen 层）

## Summary

延续前一轮工作，完成 Table 组件的剩余实现：UI 层收尾（mod.rs / 命名冲突 / 死代码清理）、codegen 层全量实现（`<Column>` 子标签 + `<template slot="header/cell/footer">` 插槽模板）、标签注册（支持 `<Table>` / `<table>` 双形式）、属性注册、以及端到端 demo 验证。

核心目标：让 RML 支持 WPF DataGrid 风格的声明式表格，兼顾小写标签匹配模式 `<table .../>` 和 `<table ...>...</table>`，并通过插槽机制实现声明式模板定义。

## Current State Analysis

### 已完成（前一轮）

| 文件 | 状态 | 说明 |
|------|------|------|
| `crates/ui/src/components/table/table_column.rs` | ✅ | TableColumn 数据结构 + builder + 3 测试 |
| `crates/ui/src/components/table/table_row.rs` | ✅ | TableRow 数据结构 + col_span/row_span + 7 测试 |
| `crates/ui/src/components/table/table_delegate.rs` | ✅ | TableDelegate trait + DefaultTableDelegate |
| `crates/ui/src/components/table/table_template.rs` | ✅ | HeaderTemplate/CellTemplate/FooterTemplate 类型定义（Arc 闭包） |
| `crates/ui/src/components/table/table.rs` | ⚠️ | Table 组件主体已实现，但含 3 个死代码方法 + 1 处语法错误 |

### 未完成

1. **UI 层**：缺 `table/mod.rs`；`components/mod.rs` 未注册；`lib.rs` 存在 `gpui_component::table::Table` 命名冲突；`table.rs` 死代码待清理
2. **Codegen 层**：`crates/engine/src/compiler/table/` 目录完全不存在
3. **标签注册**：`tags.rs` 未注册 Table/table，`is_item_builder_tag` 未识别 Column，`canonical_tag` 无 column 别名
4. **属性注册**：`props_registry.rs` 未登记 Table/Column 属性
5. **Demo 验证**：无 table demo case

### 关键约束发现

- **命名冲突**：`crates/ui/src/lib.rs` line 63 re-export 了 `gpui_component::table::Table`，必须移除以让 `rml_ui::Table` 指向自定义 Table
- **validator 无需修改**：validator.rs line 148 通过 `is_extension_component || is_item_builder_tag` 校验 props，扩展 `is_item_builder_tag` 识别 Column 即可；slot 校验（line 88-107）仅对 `user_components` 生效，不影响内置 Table 的 `<template slot="...">`
- **slot 闭包限制**：user_component.rs 的 slot 机制（`SlotRenderer = Box<dyn Fn(window, cx)>`）不能直接复用，因 Table 模板闭包签名带额外参数（`CellTemplate = Arc<dyn Fn(row_idx, col_idx, row_data, column, cx)>`）。Table codegen 需独立处理 `<template slot="...">` 解析与闭包生成
- **模板内容限制**：插槽模板内容只能用静态元素 + `self.field` 绑定（闭包 move 捕获），不能引用闭包参数（col_idx/row_data 等）。需要参数访问时用 TableDelegate trait
- **Column 是纯数据结构**：codegen 生成直接构造表达式 `TableColumn::new(...).width(...)` 而非闭包（与 AccordionItem 闭包式 builder 不同）

## Proposed Changes

### Phase 1: UI 层收尾（4 项修改）

#### 1.1 清理 `crates/ui/src/components/table/table.rs` 死代码

**问题**：lines 141-197 的 `render_header_cell`/`render_data_cell`/`style_cell` 三个私有方法未被 `RenderOnce::render` 调用（render 在 lines 221-375 内联了渲染逻辑）。其中 `style_cell` 含语法错误 `let theme = gpui_component::ActiveTheme as _;`（trait 不能这样别名）。

**操作**：删除 lines 141-197 的三个方法（`render_header_cell`、`render_data_cell`、`style_cell`）。

**验证**：render 方法逻辑不受影响（已自包含完整渲染）。

#### 1.2 新建 `crates/ui/src/components/table/mod.rs`

**职责**：仅 re-export（遵循"mod.rs 仅聚合"约束）

```rust
pub mod table;
pub mod table_column;
pub mod table_delegate;
pub mod table_row;
pub mod table_template;

pub use table::Table;
pub use table_column::TableColumn;
pub use table_delegate::{DefaultTableDelegate, TableDelegate};
pub use table_row::TableRow;
pub use table_template::{CellTemplate, FooterTemplate, HeaderTemplate};
```

#### 1.3 修改 `crates/ui/src/components/mod.rs`

在 `pub mod tree;` 之后添加 `pub mod table;`，并在 re-export 块添加：

```rust
pub use table::{
    CellTemplate, DefaultTableDelegate, FooterTemplate, HeaderTemplate, Table, TableColumn,
    TableDelegate, TableRow,
};
```

#### 1.4 修改 `crates/ui/src/lib.rs` 解决命名冲突

**问题**：line 63 `table::Table` re-export 了 gpui-component 的 Table，与自定义 Table 冲突。

**操作**：从 line 42-68 的 `pub use gpui_component::{...}` 块中移除 `table::Table,` 这一行。

**影响**：`rml_ui::Table` 将指向自定义 Table（来自 `components::table::Table`）。gpui-component 的 Table 不再直接可用（RML 不需要它）。

**验证**：`cargo build -p rust-rml-ui` 通过。

---

### Phase 2: Codegen 层实现（4 个新建文件 + 2 个修改文件）

参考 `crates/engine/src/compiler/accordion/` 的四文件结构（mod.rs/gen.rs/item.rs/setters.rs），Table 增加 `template.rs` 处理插槽模板（Accordion 无此机制）。

#### 2.1 新建 `crates/engine/src/compiler/table/setters.rs`

**职责**：Table 专用属性 → builder 方法映射

**静态属性**：
- `bordered="true"`/`bordered=""` → `.bordered(true)`
- `bordered="false"` → `.bordered(false)`
- `borderless=""` → `.borderless()`
- `stripe="true"`/`stripe=""` → `.stripe(true)`
- `stripe="false"` → `.stripe(false)`

**绑定属性**：
- `columns={expr}` → `.columns(self.expr.clone())`
- `rows={expr}` → `.rows(self.expr.clone())`
- `delegate={expr}` → `.delegate(self.expr.clone())`（Rc<dyn TableDelegate>）
- `bordered={expr}` → `.bordered(self.expr)`
- `stripe={expr}` → `.stripe(self.expr)`

**Column 静态属性**（tag == "Column" 或 canonical_tag == "Column"）：
- `width="100"` → `.width(gpui::px(100.))`（数值字面量）
- `align="center"`/`"left"`/`"right"` → `.align(gpui::TextAlign::Center/Left/Right)`

**Column 绑定属性**：
- `width={expr}` → `.width(self.expr)`
- `align={expr}` → `.align(self.expr)`

**函数签名**（参考 accordion/setters.rs）：
```rust
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String>
pub fn bind_setter(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str], tag: &str) -> Option<String>
```

#### 2.2 新建 `crates/engine/src/compiler/table/column.rs`

**职责**：Column 子标签 codegen —— 生成直接构造表达式（非闭包）

**生成目标**：
```rust
rml_ui::TableColumn::new("key", "Title").width(gpui::px(120.)).align(gpui::TextAlign::Center)
```

**流程**：
1. 从 Column 元素的属性提取 `key` 和 `title`（必填，静态或绑定）
   - 静态：`key="name"` → `"name"`
   - 绑定：`key={field}` → `self.field.clone()`（SharedString）
   - 缺失：报 CodegenError
2. 构造 `rml_ui::TableColumn::new(KEY, TITLE)`
3. 遍历其余属性，委托 `super::setters::static_setter/bind_setter` 生成 `.width(...)/.align(...)` 链

**函数签名**：
```rust
pub fn gen_column(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError>
```

**注意**：Column 是纯数据结构（无 children），不处理子节点。文本子节点报警告并忽略。

#### 2.3 新建 `crates/engine/src/compiler/table/template.rs`

**职责**：`<template slot="header/cell/footer">` 插槽模板 codegen

**生成目标**：

`<template slot="header">...</template>` →
```rust
.header_template(std::sync::Arc::new(move |_col_idx: usize, _column: &rml_ui::TableColumn, _cx: &mut gpui::App| -> gpui::AnyElement {
    (CONTENT).into_any_element()
}))
```

`<template slot="cell" field="name">...</template>` →
```rust
.cell_template("name", std::sync::Arc::new(move |_row_idx: usize, _col_idx: usize, _row_data: &rml_ui::TableRow, _column: &rml_ui::TableColumn, _cx: &mut gpui::App| -> gpui::AnyElement {
    (CONTENT).into_any_element()
}))
```

`<template slot="footer">...</template>` →
```rust
.footer_template(std::sync::Arc::new(move |_cx: &mut gpui::App| -> gpui::AnyElement {
    (CONTENT).into_any_element()
}))
```

**CONTENT 生成**：委托 `crate::compiler::codegen::gen_node` 处理 template 的子节点（复用 `gen_slot_content` 模式：空→`gpui::Empty`，单节点→直接代码，多节点→`gpui::div().child(...).child(...)`）。

**field 属性提取**：`<template slot="cell" field="key">` 中 `field` 是普通属性，从 `elem.attributes` 中查找 `Attribute::Static { name: "field", value }`。cell 模板必填 field，缺失报错；header/footer 模板忽略 field。

**函数签名**：
```rust
pub fn gen_template(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError>
```

**返回值**：完整的 `.header_template(...)` / `.cell_template(...)` / `.footer_template(...)` setter 调用字符串。

**限制说明**（写入文件头注释）：模板内容只能用静态元素 + `self.field` 绑定（move 捕获），不能引用闭包参数。需要参数访问时用 TableDelegate trait。

#### 2.4 新建 `crates/engine/src/compiler/table/gen.rs`

**职责**：Table 容器 codegen（构造 + 属性 + Column 子节点 + template slot 子节点）

**流程**（参考 `accordion/gen.rs`）：

1. **构造器**：
   ```rust
   rml_ui::Table::new(("rml_el", Nusize))
   // 或 ref: rml_ui::Table::new("rml_ref:name")
   ```

2. **属性 setter**：遍历 `elem.attributes`，委托 `super::setters::static_setter/bind_setter` → 回退 `super::super::component::component_static_setter/bind_setter`

3. **子节点处理**（三类）：
   - `Node::Element(child)` 且 `tags::is_item_builder_tag(&child.tag)`（Column/column）→ `super::column::gen_column` 生成 `.column(COLUMN_EXPR)`
   - `Node::Element(child)` 且 `child.tag == "template"` 且 `child.slot_name.is_some()` → `super::template::gen_template` 生成 `.header_template(...)` / `.cell_template(...)` / `.footer_template(...)`
   - `Node::Text(text)` → 警告并忽略
   - 其他 `Node::Element(child)` → 报 CodegenError（`<table> 仅支持 <Column> 或 <template slot="..."> 子节点`）

**函数签名**：
```rust
pub fn gen_table(
    elem: &Element,
    ref_name: Option<&str>,
    id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError>
```

#### 2.5 新建 `crates/engine/src/compiler/table/mod.rs`

**职责**：仅 re-export

```rust
pub mod column;
pub mod gen;
pub mod setters;
pub mod template;

pub use gen::gen_table;
```

#### 2.6 修改 `crates/engine/src/compiler/mod.rs`

在 `pub mod tab_bar;` 之后添加 `pub mod table;`。

#### 2.7 修改 `crates/engine/src/compiler/component.rs`

在 `StatelessWithItems` 分支（lines 76-98）添加 Table 委托：

```rust
tags::ComponentKind::StatelessWithItems => {
    let resolved_tag = tags::canonical_tag(tag);
    if resolved_tag == "TabBar" {
        return crate::compiler::tab_bar::gen_tab_bar(...);
    }
    if resolved_tag == "Table" {
        return crate::compiler::table::gen_table(
            elem, ref_name, id_val, ctx, id_counter, loop_vars,
        );
    }
    return crate::compiler::accordion::gen_accordion(...);
}
```

同时在 `component_static_setter`（line 241 区域）和 `component_bind_setter`（line 359 区域）添加 table setter 委托（参考 Card 委托模式，在 Card 委托之后）：

```rust
// 在 Card 委托之后
if let Some(s) = super::table::setters::static_setter(name, value, tag) {
    return Some(s);
}
```

```rust
// 在 Card 委托之后
if let Some(s) = super::table::setters::bind_setter(name, expr_str, loop_vars, computed, tag) {
    return Some(s);
}
```

---

### Phase 3: 标签注册与属性注册（2 个修改文件）

#### 3.1 修改 `crates/engine/src/tags.rs`

**3.1.1 在 `component_lookup` 中注册 Table**（在 TabBar 之后）：

```rust
// Table：WPF DataGrid 风格声明式表格，子节点为 <Column> / <template slot="...">
"Table" | "table" => Some(ComponentTag {
    ctor_path: "rml_ui::Table",
    kind: ComponentKind::StatelessWithItems,
}),
```

**3.1.2 扩展 `is_item_builder_tag`** 识别 Column：

```rust
pub fn is_item_builder_tag(tag: &str) -> bool {
    matches!(tag, "AccordionItem" | "item" | "Tab" | "tab" | "Column" | "column")
        || normalize_component_tag(tag) == "AccordionItem"
        || normalize_component_tag(tag) == "Tab"
        || normalize_component_tag(tag) == "Column"
}
```

**3.1.3 在 `canonical_tag` 中添加别名**：

```rust
pub fn canonical_tag(tag: &str) -> String {
    let normalized = normalize_component_tag(tag);
    match normalized.as_str() {
        "accordion" => "Accordion".to_string(),
        "item" => "AccordionItem".to_string(),
        "tab_bar" => "TabBar".to_string(),
        "tab" => "Tab".to_string(),
        "table" => "Table".to_string(),
        "column" => "Column".to_string(),
        _ => normalized,
    }
}
```

#### 3.2 修改 `crates/engine/src/compiler/props_registry.rs`

在 `COMPONENT_PROPS` 末尾（Tab 条目之后）添加：

```rust
// Table 专用（WPF DataGrid 风格表格）
("Table", &["columns", "rows", "delegate", "bordered", "borderless", "stripe"]),
// Column 专用（item builder 子标签，不在 component_lookup 中）
("Column", &["key", "title", "width", "align", "field"]),
```

**注意**：`field` 属性用于 `<template slot="cell" field="key">`，虽不在 Column 上但为方便统一注册（template 标签的属性校验较宽松，此处登记避免 warning）。

---

### Phase 4: 端到端验证（1 个新建 demo + 全量构建）

#### 4.1 新建 `demo/src/cases/table_case.rml`

展示 Table 的三种用法：

```xml
<component title="Table 表格" icon="Table">
  <div class="case-section">
    <h3>1. 数据绑定式（API 文档表格）</h3>
    <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />

    <h3>2. 声明式 Column 定义</h3>
    <Table rows={user_rows} bordered="">
      <Column key="name" title="姓名" width="120" />
      <Column key="age" title="年龄" align="center" />
      <Column key="email" title="邮箱" />
    </Table>

    <h3>3. 插槽模板（列头自定义）</h3>
    <Table rows={user_rows} bordered="">
      <Column key="name" title="姓名" />
      <Column key="age" title="年龄" />
      <template slot="header">
        <span style="color: blue; font-weight: bold;">自定义列头</span>
      </template>
      <template slot="footer">
        <span>共 {user_rows.len()} 条记录</span>
      </template>
    </Table>
  </div>
</component>
```

#### 4.2 新建 `demo/src/cases/table_case.rml.rs`

对应的 ViewModel，包含 `api_columns`/`api_rows`/`user_rows` 字段，在 `on_loaded` 中初始化。

#### 4.3 注册 demo case

在 `demo/src/cases/mod.rs` 和 demo 主界面注册 Table case。

#### 4.4 全量构建验证

```bash
cargo build -p rust-rml-ui
cargo test -p rust-rml-engine
cargo build
```

---

## Assumptions & Decisions

### 关键决策

1. **Table 使用 StatelessWithItems 模式**：支持 `<Column>` 声明式子标签和 `<template slot="...">` 插槽子节点
2. **Column codegen 生成直接构造表达式**（非闭包）：`TableColumn::new(...).width(...)` —— 因 TableColumn 是纯数据结构
3. **template codegen 生成 Arc 闭包**：匹配 `HeaderTemplate`/`CellTemplate`/`FooterTemplate` 类型签名，闭包参数加 `_` 前缀（不使用，仅满足签名）
4. **模板内容限制**：插槽模板内容只能用静态元素 + `self.field` 绑定（move 捕获），不能引用闭包参数。需要参数访问时用 TableDelegate trait（双模板机制）
5. **`field` 属性仅 cell 模板必填**：header/footer 模板忽略 field；cell 模板缺失 field 时报 CodegenError
6. **命名冲突解决**：移除 `lib.rs` 中 `gpui_component::table::Table` re-export，让 `rml_ui::Table` 指向自定义 Table
7. **小写标签双形式**：`<Table>` / `<table>` 均合法，在 `component_lookup` 和 `canonical_tag` 中注册别名；`<Column>` / `<column>` 同理

### 假设

- `gpui::TextAlign` 可用（gpui-component Table 已用，确认）
- `std::sync::Arc` 满足 `Send + Sync + 'static`（确认，Arc 闭包类型已在 table_template.rs 定义）
- `gpui::px(f32)` 函数可用（现有代码已用）
- validator 不拦截 `<template>` 子节点（确认：slot 校验仅对 user_components 生效，内置组件不受限）
- `props_registry` 的 `component_props_tags_align_with_routing_table` 测试不会因 Column 报错（确认：测试通过 `is_item_builder_tag` 跳过 item builder 子标签）

### 范围边界

- **本次实现**：Table 完整 codegen + 插槽模板 + 小写标签 + demo 验证
- **不实现**：模板内容引用闭包参数（如 `{row_data.field}` 绑定到 row_data）—— 此场景用 TableDelegate trait
- **不实现**：虚拟滚动、列拖拽、列宽调整（未来扩展）

## Implementation Steps

### Step 1: UI 层收尾

1. 清理 `crates/ui/src/components/table/table.rs` 死代码（删除 lines 141-197 三个方法）
2. 新建 `crates/ui/src/components/table/mod.rs`（仅 re-export）
3. 修改 `crates/ui/src/components/mod.rs`（添加 `pub mod table;` + re-export）
4. 修改 `crates/ui/src/lib.rs`（移除 `table::Table` re-export）
   → **verify**: `cargo build -p rust-rml-ui` 通过

### Step 2: Codegen 层实现

1. 新建 `crates/engine/src/compiler/table/setters.rs`（Table + Column 属性 setter + 单元测试）
2. 新建 `crates/engine/src/compiler/table/column.rs`（Column 直接构造表达式 codegen + 单元测试）
3. 新建 `crates/engine/src/compiler/table/template.rs`（插槽模板 codegen + 单元测试）
4. 新建 `crates/engine/src/compiler/table/gen.rs`（Table 容器 codegen + 单元测试）
5. 新建 `crates/engine/src/compiler/table/mod.rs`（仅 re-export）
6. 修改 `crates/engine/src/compiler/mod.rs`（添加 `pub mod table;`）
7. 修改 `crates/engine/src/compiler/component.rs`（StatelessWithItems 分支添加 Table 委托 + setter 委托）
   → **verify**: `cargo test -p rust-rml-engine` 通过

### Step 3: 标签与属性注册

1. 修改 `crates/engine/src/tags.rs`（注册 Table/table + 扩展 is_item_builder_tag + canonical_tag 别名）
2. 修改 `crates/engine/src/compiler/props_registry.rs`（注册 Table + Column 属性）
   → **verify**: `cargo test -p rust-rml-engine` 通过（含 props_registry 一致性测试）

### Step 4: 端到端验证

1. 新建 `demo/src/cases/table_case.rml` + `table_case.rml.rs`
2. 在 demo 注册 Table case
3. 全量构建：`cargo build`
   → **verify**: 编译通过，demo 可运行展示 Table 三种用法

## Verification Checklist

- [ ] `cargo build -p rust-rml-ui` 通过（UI 层无命名冲突、无死代码警告）
- [ ] `cargo test -p rust-rml-engine` 通过（codegen 单元测试 + props_registry 一致性测试）
- [ ] `cargo build` 全量通过（demo 集成无误）
- [ ] `<Table columns={...} rows={...} />` 数据绑定式可用
- [ ] `<table>` 小写标签可用
- [ ] `<Table><Column key="..." title="..." /></Table>` 声明式列定义可用
- [ ] `<column>` 小写子标签可用
- [ ] `<template slot="header">` 列头模板可用
- [ ] `<template slot="cell" field="key">` 单元格模板可用
- [ ] `<template slot="footer">` 底部模板可用
- [ ] `<Table ...></Table>` 双标签形式可用（含子节点）
- [ ] `<Table ... />` 自闭合形式可用（无子节点）
