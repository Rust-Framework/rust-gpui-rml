# Table 组件实现计划（WPF DataGrid 风格 + MVVM）

## Summary

在 RML 框架中实现 WPF DataGrid 风格的 Table 组件，解决当前 API 文档表格用 div+span 手工拼装的妥协问题。支持 Columns 数据绑定 + 声明式 Column 定义、列头/列数据模板（TableDelegate trait）、合并列（col\_span/row\_span）等高级功能，遵循"一个 rs 文件 = 一个职责"原则。

## Current State Analysis

### 现状

* `demo/src/cases/*.rml` 中的 API 表格使用 `<div class="api-table">` + `<div class="api-row">` + `<span class="api-prop-name">` 手工拼装

* RML 内置标签无 table/tr/td/th

* gpui-component 提供两种 Table：底层组合式 `Table`（需手写 TableRow/TableHead/TableCell）和复杂数据驱动 `DataTable<D>`（基于 TableDelegate trait，带虚拟滚动/选择/排序）—— 都不适合直接作为 RML 声明式组件

### 参考模式

* **Stateless 组件实现**：`crates/ui/src/components/card.rs`（Card 组件，struct + RenderOnce + ParentElement + Styled）

* **StatelessWithItems codegen**：`crates/engine/src/compiler/accordion/gen.rs`（AccordionItem 子标签 → `.item(|__rml_item| ...)`）

* **专用 setter 模式**：`crates/engine/src/compiler/card/setters.rs`（属性 → builder 方法映射）

* **标签注册**：`crates/engine/src/tags.rs` 的 `component_lookup` + `is_item_builder_tag`

* **属性注册**：`crates/engine/src/compiler/props_registry.rs` 的 `COMPONENT_PROPS`

## Proposed Changes

### 文件组织（遵循"一个 rs 文件 = 一个职责"约束）

```
crates/ui/src/components/table/          # UI 组件层
├── mod.rs                               # 仅 re-export
├── table.rs                             # Table 组件（Stateless）
├── table_column.rs                      # TableColumn 数据结构
├── table_row.rs                         # TableRow 数据结构
└── table_delegate.rs                    # TableDelegate trait + DefaultTableDelegate

crates/engine/src/compiler/table/        # codegen 层
├── mod.rs                               # 仅 re-export
├── setters.rs                           # 属性 setter（columns/rows/bordered/stripe/delegate）
└── gen.rs                               # 声明式 <Column> 子标签 codegen
```

### 新建文件（8 个）

#### 1. `crates/ui/src/components/table/table_column.rs`

**职责**：TableColumn 数据结构定义

```rust
pub struct TableColumn {
    pub key: SharedString,        // 字段 key（用于从 TableRow.cells 取值）
    pub title: SharedString,      // 列头文本
    pub width: Option<Pixels>,    // 列宽（px），None 表示自动
    pub align: Option<TextAlign>, // 对齐方式
}
impl TableColumn {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self;
    pub fn width(mut self, width: impl Into<Pixels>) -> Self;
    pub fn align(mut self, align: TextAlign) -> Self;
}
```

#### 2. `crates/ui/src/components/table/table_row.rs`

**职责**：TableRow 数据结构定义（含合并列支持）

```rust
pub struct TableRow {
    pub cells: HashMap<SharedString, SharedString>,           // key -> value
    pub col_spans: HashMap<SharedString, usize>,              // key -> 合并列数
    pub row_spans: HashMap<SharedString, usize>,              // key -> 合并行数
}
impl TableRow {
    pub fn new() -> Self;
    pub fn cell(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self;
    pub fn col_span(mut self, key: impl Into<SharedString>, span: usize) -> Self;
    pub fn row_span(mut self, key: impl Into<SharedString>, span: usize) -> Self;
}
impl Default for TableRow { ... }
```

#### 3. `crates/ui/src/components/table/table_delegate.rs`

**职责**：TableDelegate trait（列头/列数据模板）+ 默认实现

```rust
/// 表格模板委托 —— 支持自定义列头和单元格渲染（WPF DataTemplate 等价）
pub trait TableDelegate: 'static {
    /// 渲染列头。默认实现返回 column.title 文本。
    fn render_header(&self, col: usize, column: &TableColumn, cx: &mut App) -> AnyElement {
        div().child(column.title.clone()).into_any_element()
    }
    /// 渲染单元格。默认实现返回 row_data.cells[column.key] 文本。
    fn render_cell(
        &self,
        row: usize,
        col: usize,
        column: &TableColumn,
        row_data: &TableRow,
        cx: &mut App,
    ) -> AnyElement {
        let text = row_data.cells.get(&column.key).cloned().unwrap_or_default();
        div().child(text).into_any_element()
    }
}

/// 默认委托（纯文本渲染）
pub struct DefaultTableDelegate;
impl TableDelegate for DefaultTableDelegate {}
```

#### 4. `crates/ui/src/components/table/table.rs`

**职责**：Table 组件（Stateless，StatelessWithItems 模式以支持 `<Column>` 子标签）

```rust
#[derive(IntoElement)]
pub struct Table {
    base: Div,
    id: ElementId,
    columns: Vec<TableColumn>,
    rows: Vec<TableRow>,
    delegate: Option<Rc<dyn TableDelegate>>,
    bordered: bool,
    stripe: bool,
    size: Size,
}
impl Table {
    pub fn new(id: impl Into<ElementId>) -> Self;
    /// 数据绑定式列定义（与 `<Column>` 子标签可混用，声明式追加到尾部）
    pub fn columns(mut self, columns: Vec<TableColumn>) -> Self;
    /// 行数据绑定
    pub fn rows(mut self, rows: Vec<TableRow>) -> Self;
    /// 声明式 Column 子标签追加（codegen 生成 `.column(...)` 调用）
    pub fn column(mut self, column: TableColumn) -> Self;
    /// 模板委托（自定义渲染）
    pub fn delegate(mut self, delegate: impl TableDelegate) -> Self;
    pub fn bordered(mut self, bordered: bool) -> Self;
    pub fn borderless(mut self) -> Self;
    pub fn stripe(mut self, stripe: bool) -> Self;
}
impl Styled for Table { ... }
impl Sizable for Table { ... }
impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // 布局：[header: TableRow of TableColumn.title] → [body: rows of cells]
        // 使用 div + flex 模拟表格结构（避免注册 table/thead/tbody/tr/td 内置标签）
        // delegate 为 None 时使用 DefaultTableDelegate
        // stripe=true 时奇数行使用交替背景色
        // bordered=true 时显示边框 + 单元格分隔线
    }
}
```

**渲染策略**：使用 `div().flex().flex_col()` 构建表格容器，header 和每行都是 `div().flex()` 行布局，单元格通过 `flex_grow()` 或固定 `width()` 分配列宽。不依赖 gpui-component 的底层 Table（避免 ChildElement 复杂性），直接用 div 模拟。

#### 5. `crates/ui/src/components/table/mod.rs`

**职责**：仅 re-export

```rust
pub mod table;
pub mod table_column;
pub mod table_row;
pub mod table_delegate;

pub use table::Table;
pub use table_column::TableColumn;
pub use table_row::TableRow;
pub use table_delegate::{DefaultTableDelegate, TableDelegate};
```

#### 6. `crates/engine/src/compiler/table/setters.rs`

**职责**：Table 专用属性 setter

**静态属性**：

* `bordered="true"`/`bordered=""` → `.bordered(true)` / `bordered="false"` → `.bordered(false)`

* `borderless=""` → `.borderless()`

* `stripe="true"`/`stripe=""` → `.stripe(true)` / `stripe="false"` → `.stripe(false)`

**绑定属性**：

* `columns={expr}` → `.columns(self.expr.clone())`（Vec<TableColumn>，需 clone）

* `rows={expr}` → `.rows(self.expr.clone())`（Vec<TableRow>，需 clone）

* `delegate={expr}` → `.delegate(self.expr.clone())`（Rc<dyn TableDelegate>，Rc clone 廉价）

* `bordered={expr}` / `stripe={expr}` → `.method(self.expr)`（bool 表达式）

#### 7. `crates/engine/src/compiler/table/gen.rs`

**职责**：Table 组件 codegen（StatelessWithItems 模式）

参考 `accordion/gen.rs`，处理流程：

1. 构造器：`rml_ui::Table::new(("rml_el", N))` 或 `rml_ui::Table::new("rml_ref:...")`
2. 属性 setter：委托 `table::setters` + 通用 `component_static_setter`/`component_bind_setter`
3. 子节点处理：

   * `<Column>` / `<column>` 子标签 → `.column(rml_ui::TableColumn::new("key", "title").width(...).align(...))`

   * 其他子节点 → 报错（Table 仅支持 Column 子标签）

   * 文本子节点 → 警告并忽略

**Column 子标签属性映射**：

* `key="..."` / `key={expr}` → 构造器第一参数（必填）

* `title="..."` / `title={expr}` → 构造器第二参数（必填）

* `width="100"` → `.width(gpui::px(100.))`（静态数值）

* `width={expr}` → `.width(expr)`（绑定 Pixels 表达式）

* `align="center"` → `.align(gpui::TextAlign::Center)`

* `align={expr}` → `.align(expr)`

#### 8. `crates/engine/src/compiler/table/mod.rs`

**职责**：仅 re-export

```rust
pub mod gen;
pub mod setters;

pub use gen::gen_table;
pub use setters::{bind_setter, static_setter};
```

### 修改现有文件（6 个）

#### 9. `crates/ui/src/components/mod.rs`

新增 `pub mod table;` + `pub use table::{Table, TableColumn, TableRow, DefaultTableDelegate, TableDelegate};`

#### 10. `crates/engine/src/compiler/mod.rs`

新增 `pub mod table;`

#### 11. `crates/engine/src/compiler/component.rs`

在 `component_static_setter` 和 `component_bind_setter` 中添加 Table setter 委托（参考 Card 委托模式）：

```rust
// 在 Card 委托之后
if let Some(s) = super::table::static_setter(name, value, tag) {
    return Some(s);
}
```

#### 12. `crates/engine/src/tags.rs`

* 在 `component_lookup` 中注册 Table 标签为 `StatelessWithItems`：

  ```rust
  "Table" | "table" => Some(ComponentTag {
      ctor_path: "rml_ui::Table",
      kind: ComponentKind::StatelessWithItems,
  }),
  ```

* 扩展 `is_item_builder_tag` 识别 Column：

  ```rust
  pub fn is_item_builder_tag(tag: &str) -> bool {
      matches!(tag, "AccordionItem" | "item" | "Column" | "column")
          || normalize_component_tag(tag) == "AccordionItem"
          || normalize_component_tag(tag) == "Column"
  }
  ```

* 在 `canonical_tag` 中添加 `"column" => "Column"` 别名（参考 `"item" => "AccordionItem"`）

#### 13. `crates/engine/src/compiler/props_registry.rs`

在 `COMPONENT_PROPS` 中注册 Table 属性：

```rust
("Table", &["columns", "rows", "delegate", "bordered", "borderless", "stripe"]),
// Column 子标签属性（item builder，不在 component_lookup 中）
("Column", &["key", "title", "width", "align"]),
```

#### 14. `crates/engine/src/compiler/validator.rs`

检查 validator 是否需要调整以允许 Column 子标签（参考 AccordionItem 的处理，line 146-148）。如果 validator 已经通过 `is_item_builder_tag` 放行，则无需修改。

### RML 用法示例

#### 简单数据绑定式（API 文档表格）

```xml
<Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
```

```rust
// .rml.rs
pub api_columns: Vec<TableColumn>,
pub api_rows: Vec<TableRow>,

// 初始化
api_columns: vec![
    TableColumn::new("prop", "属性"),
    TableColumn::new("type", "类型"),
    TableColumn::new("desc", "说明"),
],
api_rows: vec![
    TableRow::new().cell("prop", "bordered").cell("type", "布尔").cell("desc", "显示边框"),
    TableRow::new().cell("prop", "stripe").cell("type", "布尔").cell("desc", "斑马纹"),
],
```

#### 声明式 Column 定义

```xml
<Table rows={rows} bordered="">
    <Column key="name" title="Name" width="120" />
    <Column key="age" title="Age" align="center" />
    <Column key="email" title="Email" />
</Table>
```

#### 混合式（绑定 + 声明式）

```xml
<Table rows={rows} columns={base_columns} bordered="">
    <Column key="actions" title="Actions" width="100" />
</Table>
```

#### 自定义模板（TableDelegate）

```rust
// .rml.rs
pub struct UserTableCase {
    pub rows: Vec<TableRow>,
    pub columns: Vec<TableColumn>,
    pub delegate: Rc<dyn TableDelegate>,
}

impl UserTableCase {
    fn new() -> Self {
        Self {
            rows: vec![...],
            columns: vec![...],
            delegate: Rc::new(UserTableDelegate),
        }
    }
}

struct UserTableDelegate;
impl TableDelegate for UserTableDelegate {
    fn render_cell(&self, row: usize, col: usize, column: &TableColumn, row_data: &TableRow, cx: &mut App) -> AnyElement {
        if column.key == "actions" {
            Button::new(("edit", row)).label("Edit").into_any_element()
        } else {
            // 默认文本渲染
            let text = row_data.cells.get(&column.key).cloned().unwrap_or_default();
            div().child(text).into_any_element()
        }
    }
}
```

```xml
<Table columns={columns} rows={rows} delegate={delegate} bordered="" />
```

#### 合并列

```rust
TableRow::new()
    .cell("name", "John")
    .cell("email", "john@example.com")
    .col_span("name", 2)  // name 占 2 列
```

## Assumptions & Decisions

### 关键决策

1. **Table 使用 StatelessWithItems 模式**：支持 `<Column>` 声明式子标签，参考 AccordionItem 模式
2. **TableColumn 是纯数据结构**（非 IntoElement 组件），codegen 生成 `TableColumn::new(...).<setters>` 调用
3. **模板通过 TableDelegate trait 实现**：默认渲染文本，用户可实现 trait 自定义渲染（WPF DataTemplate 等价）
4. **合并列通过 TableRow 字段支持**：col\_spans/row\_spans HashMap 按 column.key 索引
5. **delegate 字段类型为** **`Option<Rc<dyn TableDelegate>>`**：用户持有 `Rc<dyn TableDelegate>` 字段，Rc clone 廉价
6. **渲染用 div+flex 模拟表格**：避免注册 table/thead/tbody/tr/td 内置标签，降低复杂度
7. **columns 绑定与** **`<Column>`** **声明式可混用**：声明式 Column 追加到绑定 columns 之后

### 假设

* GPUI 的 `TextAlign` 类型可用（gpui-component 的 Table 用了 `gpui::TextAlign`）

* `SharedString` 支持 `Clone` + `HashMap` key（已确认，现有代码已用）

* `Rc<dyn TableDelegate>` 可作为 .rml.rs 字段类型并 clone（Rc 是 Clone 的）

* validator 通过 `is_item_builder_tag` 放行 Column 子标签（需验证，可能需要小调整）

### 范围边界

* **本次实现**：Table/TableColumn/TableRow/TableDelegate + 数据绑定 + 声明式 Column + 模板 + 合并列

* **未来扩展**：虚拟滚动、列拖拽排序、列宽调整（可后续基于 gpui-component DataTable 扩展）

* **不实现**：`<THead>`/`<TBody>`/`<TR>`/`<TH>`/`<TD>` 子标签式 HTML 风格（用户已选 WPF 风格，无需 HTML 风格）

## Implementation Steps

### Step 1: UI 组件层（crates/ui/）

1. 新建 `crates/ui/src/components/table/table_column.rs` — TableColumn 数据结构 + builder 方法
2. 新建 `crates/ui/src/components/table/table_row.rs` — TableRow 数据结构 + builder 方法 + Default
3. 新建 `crates/ui/src/components/table/table_delegate.rs` — TableDelegate trait + DefaultTableDelegate
4. 新建 `crates/ui/src/components/table/table.rs` — Table 组件（struct + new + builder 方法 + RenderOnce）
5. 新建 `crates/ui/src/components/table/mod.rs` — 仅 re-export
6. 修改 `crates/ui/src/components/mod.rs` — 注册 table 模块 + re-export
   → **verify**: `cargo build -p rust-rml-ui` 通过

### Step 2: codegen 层（crates/engine/）

1. 新建 `crates/engine/src/compiler/table/setters.rs` — Table 专用属性 setter + 单元测试
2. 新建 `crates/engine/src/compiler/table/gen.rs` — Table codegen（构造 + 属性 + Column 子标签）
3. 新建 `crates/engine/src/compiler/table/mod.rs` — 仅 re-export
4. 修改 `crates/engine/src/compiler/mod.rs` — 注册 table 模块
5. 修改 `crates/engine/src/compiler/component.rs` — 委托 table setter（static + bind）
6. 修改 `crates/engine/src/tags.rs` — 注册 Table 标签 + 扩展 is\_item\_builder\_tag + canonical\_tag 别名
7. 修改 `crates/engine/src/compiler/props_registry.rs` — 注册 Table + Column 属性
8. 检查 `crates/engine/src/compiler/validator.rs` — 确保 Column 子标签放行
   → **verify**: `cargo test -p rust-rml-engine` 通过（含 props\_registry 一致性测试）

### Step 3: codegen 集成

1. 修改 `crates/engine/src/compiler/component.rs` 的 `gen_component` — 在 StatelessWithItems 分支委托 `table::gen::gen_table`
2. 检查 StatelessWithItems 分支当前是否只处理 Accordion，需扩展为委托模式
   → **verify**: `cargo build -p rust-rml-engine` 通过

### Step 4: 端到端验证

1. `cargo build` 全量构建（处理预存在的 lsp/ 模块问题，与本次改动无关）
2. 创建一个简单的 Table demo case 验证 RML 语法可用（可选，如用户需要）
   → **verify**: 构建通过 + Table 组件可正常渲染

## Verification Steps

1. **UI 组件编译**：`cargo build -p rust-rml-ui` 通过
2. **Engine 编译**：`cargo build -p rust-rml-engine` 通过
3. **单元测试**：`cargo test -p rust-rml-engine` 通过（含 setters 测试 + props\_registry 一致性测试）
4. **全量构建**：`cargo build` 通过（忽略预存在的 lsp/ 模块错误）
5. **RML 语法验证**：编写一个测试 .rml 文件验证 `<Table>` 标签可被正确解析和 codegen

## Risks & Mitigations

| 风险                                                              | 缓解措施                                                           |
| --------------------------------------------------------------- | -------------------------------------------------------------- |
| `Rc<dyn TableDelegate>` 作为 .rml.rs 字段可能不兼容 `#[derive(Default)]` | delegate 字段用 `Option<Rc<dyn TableDelegate>>`，Default 返回 None   |
| StatelessWithItems 分支当前硬编码 Accordion，需重构为委托模式                   | 参考 gen.rs 的 gen\_accordion 模式，提取 gen\_table 并在 component.rs 委托 |
| TextAlign 类型可能不在 gpui 预导入                                       | 在 table.rs 显式 `use gpui::TextAlign`                            |
| Column 子标签的 width 静态属性需解析为 Pixels                               | setters.rs 中 `width="100"` → `.width(gpui::px(100.))`          |
| validator 可能拒绝 Column 子标签                                       | 检查 validator.rs line 146-148，确保 is\_item\_builder\_tag 放行      |

