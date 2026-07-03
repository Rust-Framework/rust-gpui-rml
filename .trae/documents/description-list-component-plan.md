# DescriptionList 系列组件实现计划

## Context

用户要求实现 RML 框架的 DescriptionList 系列组件和声明式支持，严格按照项目组件开发规范，职责单一，高内聚低耦合，禁止一个文件大量逻辑。

用户指定 RML 标签命名为短语义小写形式：

```html
<descriptions ...>
  <description .../>
  <separator />
</descriptions>
```

gpui-component 已有完整的 `DescriptionList` / `DescriptionItem` / `DescriptionText` 实现（位于 `gpui-component/src/description_list.rs`），本任务只需：
1. 在 `crates/ui` 中 re-export 这些类型
2. 在 `crates/engine` 中添加 RML 声明式 codegen 支持（参照 `tab_bar/` 模式）

### gpui-component API 关键约束

- `DescriptionList::new()` 不接受 ElementId（与 TabBar 不同）
- `.child()` 仅接受 `impl Into<DescriptionItem>`，不能传入任意元素
- `DescriptionItem::new(label)` 构造器必填 label（无 `.label()` setter）
- `DescriptionItem::Separator` 是枚举变体，不是方法
- `DescriptionText: From<&str/String/SharedString/Text/AnyElement>`

## 设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| ComponentKind | `StatelessWithItems` | `.child()` 仅接受 DescriptionItem，需专属 codegen 处理类型化子节点（同 TabBar） |
| 构造器 | `DescriptionList::new()` 无 id | API 不接受 ElementId，ref 指令静默忽略（同 TitleBar 行为） |
| vertical/horizontal | `.layout(gpui::Axis::*)` setter | 与 codegen setter 链模式一致，避免构造器分支 |
| Separator | `<separator />` 独立标签 | `Separator` 是枚举变体无法通过 `::new(label)` 构造，语义清晰 |
| 富文本 value | element 子节点 → `.value(element)` | `DescriptionText: From<AnyElement>`，与 Tab label/child 模式平行 |
| label_width 解析 | `label_width="200"` → `.label_width(gpui::px(200.))` | 与 Column `width="100"` 处理完全一致 |

### 标签命名规范

用户指定短语义小写标签，同时支持 PascalCase 别名（与 `tab_bar`/`TabBar` 双写模式一致）：

| RML 标签 | 规范化名 (canonical) | Rust 类型 | 说明 |
|----------|---------------------|-----------|------|
| `<descriptions>` / `<DescriptionList>` | `DescriptionList` | `rml_ui::DescriptionList` | 容器组件 |
| `<description>` / `<DescriptionItem>` | `DescriptionItem` | `rml_ui::DescriptionItem` | 数据项子节点 |
| `<separator>` / `<DescriptionSeparator>` | `DescriptionSeparator` | `rml_ui::DescriptionItem::Separator` | 分隔符子节点 |

## RML 语法设计

```html
<!-- 基础用法 -->
<descriptions>
  <description label="Name" value="John" />
  <description label="Email" value="john@example.com" />
</descriptions>

<!-- 垂直布局 + 列数 + 无边框 -->
<descriptions vertical columns="2" bordered="false">
  <description label="Name" value="John" />
  <description label="Age" value="30" span="2" />
</descriptions>

<!-- 分隔符 -->
<descriptions>
  <description label="Name" value="John" />
  <separator />
  <description label="Email" value="john@example.com" />
</descriptions>

<!-- 富文本 value（element 子节点） -->
<description label="Status">
  <Badge success>Active</Badge>
</description>

<!-- 文本子节点作为 value -->
<description label="Name">John Doe</description>

<!-- PascalCase 别名（同样支持） -->
<DescriptionList vertical>
  <DescriptionItem label="Name" value="John" />
  <DescriptionSeparator />
</DescriptionList>

<!-- 绑定表达式 -->
<descriptions columns={col_count} bordered={show_border}>
  <description label={item.label} value={item.value} span={item.span} />
</descriptions>
```

## 文件清单

### 新建文件（4 个，均在 `crates/engine/src/compiler/description_list/`）

参照 `tab_bar/` 模块结构，按职责拆分：

1. **`mod.rs`** — 模块入口，声明子模块并 re-export gen 函数
2. **`gen.rs`** — DescriptionList 容器 codegen（构造 + 属性 + 子节点 `.child()`/`.separator()` 注入）
3. **`item.rs`** — DescriptionItem 子节点 codegen（label 构造器提取 + value/span setter + 子节点作为 value）
4. **`setters.rs`** — DescriptionList/DescriptionItem 专用属性 → builder 方法映射

### 修改文件（6 个）

5. **`crates/ui/src/lib.rs`** — 在 `pub use gpui_component::{...}` 添加 `description_list::{DescriptionList, DescriptionItem, DescriptionText}`
6. **`crates/ui/src/prelude.rs`** — 在 `pub use crate::{...}` 添加 `DescriptionList, DescriptionItem, DescriptionText`
7. **`crates/engine/src/compiler/mod.rs`** — 添加 `pub mod description_list;`
8. **`crates/engine/src/tags.rs`** — 三处修改：
   - `component_lookup`：注册 `"DescriptionList" | "descriptions"` 为 `StatelessWithItems`
   - `canonical_tag`：添加 `descriptions`→`DescriptionList`、`description`→`DescriptionItem`、`separator`→`DescriptionSeparator` 别名
   - `is_item_builder_tag`：添加 `description`/`separator` 及 PascalCase 形式
9. **`crates/engine/src/compiler/component.rs`** — 三处修改：
   - `StatelessWithItems` 分支：添加 `DescriptionList` 分发到 `gen_description_list`
   - `component_static_setter`：添加 `super::description_list::setters::static_setter` 委托
   - `component_bind_setter`：添加 `super::description_list::setters::bind_setter` 委托
10. **`crates/engine/src/compiler/props_registry.rs`** — 在 `COMPONENT_PROPS` 添加：
    - `("DescriptionList", &["vertical", "horizontal", "bordered", "columns", "label_width"])`
    - `("DescriptionItem", &["label", "value", "span"])`

## Setter 映射表

### DescriptionList（canonical tag = "DescriptionList"）

| 属性 | 静态 | 绑定 |
|------|------|------|
| `vertical` | `.layout(gpui::Axis::Vertical)` | — |
| `horizontal` | `.layout(gpui::Axis::Horizontal)` | — |
| `bordered` | `.bordered(true/false)` | `.bordered(self.expr)` |
| `columns` | `.columns(N)` | `.columns(self.expr)` |
| `label_width` | `.label_width(gpui::px(N.))` | `.label_width(self.expr)` |
| `small`/`large` 等 | 通用 Sizable（公共 setter 处理） | — |

### DescriptionItem（canonical tag = "DescriptionItem"）

| 属性 | 静态 | 绑定 |
|------|------|------|
| `label` | 构造器 `::new("...")`（在 item.rs 提取） | 构造器 `::new(self.expr.clone())` |
| `value` | `.value("...")` | `.value(self.expr.clone())` |
| `span` | `.span(N)` | `.span(self.expr)` |

### DescriptionSeparator

无属性，生成 `.separator()`（容器端识别 `<separator>` 标签后调用 `DescriptionList::separator()` 方法）

## 子节点处理规则（gen.rs）

```
<descriptions>
  ├─ <description>   → .child(rml_ui::DescriptionItem::new(label)...)
  ├─ <separator>     → .separator()
  ├─ 文本节点        → 警告并忽略
  └─ 其他元素        → CodegenError
```

容器 gen.rs 通过 `canonical_tag(child.tag)` 判断子节点类型：
- `DescriptionItem` → 调用 `item::gen_description_item()` 生成 `.child(...)` 
- `DescriptionSeparator` → 生成 `.separator()`
- 其他 → 报错

## DescriptionItem value 处理规则（item.rs）

优先级：`value` 属性 > 文本子节点 > element 子节点

1. 有 `value` 属性 → `.value(...)`，忽略所有子节点
2. 无 `value` 属性 + 文本子节点 → `.value("text")`
3. 无 `value` 属性 + 单个 element 子节点 → `.value(element_code)`
4. 无 `value` 属性 + 多个 element 子节点 → `.value(gpui::div().child(e1).child(e2)...)`

## 参考文件

- [tab_bar/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/gen.rs) — StatelessWithItems 容器 codegen 模板
- [tab_bar/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs) — 专用 setter 映射模板
- [table/column.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/table/column.rs) — 子项 codegen + 必填参数提取模板
- [table/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/table/setters.rs) — `width="100"` → `.width(gpui::px(100.))` 数值解析参考
- [component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) — 分发分支 + setter 委托位置
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) — 组件注册位置
- [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) — 属性注册 + 一致性测试位置

## 实施顺序

1. UI crate re-export（`lib.rs` + `prelude.rs`）
2. tags.rs 注册（ComponentKind + canonical_tag + is_item_builder_tag）
3. props_registry.rs 注册
4. compiler/mod.rs 添加模块声明
5. 创建 description_list/ 模块（setters.rs → item.rs → gen.rs → mod.rs）
6. component.rs 添加分发和 setter 委托
7. 运行测试，修复直至全绿

## 验证

```bash
# 编译检查（0 warnings）
cargo build -p rust-rml-ui
cargo build -p rust-rml-engine

# 单元测试 + 一致性测试
cargo test -p rust-rml-engine

# 关键测试项
cargo test -p rust-rml-engine -- component_props_tags_align  # 注册表一致性
cargo test -p rust-rml-engine -- description_list            # 新模块测试
```

每个新建文件内含完整单元测试，参照 tab_bar/ 和 table/ 的测试风格覆盖：
- 最小构造、静态属性、绑定属性、子节点注入、separator、ref 忽略、非法子节点报错、小写别名（`<descriptions>`/`<description>`/`<separator>`）、PascalCase 别名、端到端 gen_component 调度
