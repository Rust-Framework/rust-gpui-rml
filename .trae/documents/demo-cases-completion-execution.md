# RML Demo 案例官方文档化 — 完成执行计划

## 摘要

本计划承接已批准的 `demo-cases-official-docs-enhancement.md`，聚焦于完成 12 个已注册 RML 组件案例的迁移与校准。当前 Input 案例已迁移至 Canonical 模式但有 3 个编译错误；Tree/Accordion/Alert/Avatar/Popover 5 个案例仍为 Legacy 模式；Table 需去除 1 处 `<code>` 标签；Button/Badge/Checkbox/Icon/Tooltip 5 个 Canonical 案例需核对 API 准确性。

## 当前状态分析

### Input 案例（已迁移，3 个编译错误）

基于 [input_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml) 与 [input_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs) 实际状态：

| 错误 | 位置 | 根因 | 修复方案 |
|---|---|---|---|
| `no field input_state on type &mut InputCase` | .rml line 18 `<Input />`（无 ref） | [tags.rs:416](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L416) 硬编码 `state_field: "input_state"`，无 ref 的 `<Input />` 生成 `self.input_state.as_ref().expect(...)`，但字段名为 `placeholder_input` | 将 `placeholder_input` 字段重命名为 `input_state` |
| `the trait bound rml_ui::Size: From<&str> is not satisfied` | .rml line 38 `size={size_label}` | [component.rs:629-631](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L629) 绑定生成 `.with_size(self.size_label())`，但 `with_size` 接收 `impl Into<Size>`，`&str` 未实现该 trait | 改为条件渲染 4 个静态 `size="..."` 的 Input（if/else 分支） |
| `no field field on type &InputCase` | .rml line 58 文本中的 `model={field}` | RML 解析器将 `<p>` 文本中的 `model={field}` 视为绑定表达式 | 改写文本，避免 `{field}` 语法 |

### 5 个 Legacy 案例待迁移

| 组件 | 当前 section 数 | `<code>` 标签 | 关键工作 |
|---|---|---|---|
| [tree_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml.rs) | 1 | 否 | 迁移 + 补全 4 个 section（基础/expanded/嵌套/on-activate vs on-select） |
| [accordion_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs) | 6 | 是 | 纯迁移 + 去 `<code>` |
| [alert_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml.rs) | 7 | 是 | 纯迁移 + 去 `<code>` |
| [avatar_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml.rs) | 4 | 是 | 迁移 + 拆分 src/name/placeholder 各独立 section |
| [popover_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml.rs) | 3 | 是 | 迁移 + 补 slot="trigger" 机制说明 |

### 6 个 Canonical 案例待校准

| 组件 | 工作量 |
|---|---|
| [table_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) | 删除 line 52 的 1 处 `<code>` 标签 |
| [button_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml) | 核对 API 表格 |
| [badge_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/badge_case.rml) | 核对 |
| [checkbox_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/checkbox_case.rml) | 核对 |
| [icon_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/icon_case.rml) | 核对 |
| [tooltip_case](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tooltip_case.rml) | 核对 |

## 实施步骤

### Step 1: 修复 Input 案例 3 个编译错误

**文件**: [demo/src/cases/input_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs) + [demo/src/cases/input_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml)

**修复 1：字段重命名**
- .rml.rs：将 `pub placeholder_input: Option<gpui::Entity<InputState>>` 改为 `pub input_state: Option<gpui::Entity<InputState>>`
- .rml.rs：将 `on_loaded` 中 `self.placeholder_input = Some(...)` 改为 `self.input_state = Some(...)`
- .rml.rs：更新字段注释，明确此字段名是 tags.rs 硬编码要求

**修复 2：size 绑定改为条件渲染**
- .rml.rs：删除 `current_size: u8` 字段和 `size_label` computed 方法
- .rml.rs：新增 `current_size: u8` 字段保留（用于切换状态），新增 `size_kind` computed 返回 0-3 索引
- .rml line 32-40：改为 4 个条件渲染的 Input，每个用静态 size 属性：
  ```rml
  <Input ref="sized_input" size="xsmall" if={current_size == 0} />
  <Input ref="sized_input" size="small" if={current_size == 1} />
  <Input ref="sized_input" size="medium" if={current_size == 2} />
  <Input ref="sized_input" size="large" if={current_size == 3} />
  ```
  注：if 指令在 RML 中控制元素是否渲染。ref 名相同但仅一个分支生效，满足 ElementRef 注入。但需验证 if 与 ref 共存时的 codegen 行为；若不支持，改为 4 个不同 ref 名（sized_xsmall_input/sized_small_input/...）。

**修复 3：文本中避免 `{field}` 语法**
- .rml line 58：将 "每个小写 input model={field} 独立双向同步" 改为 "每个小写 input 通过 model 指令独立双向同步到对应字段"

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 2: 迁移 Tree 案例到 Canonical 模式

**文件**: [demo/src/cases/tree_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml.rs) + [demo/src/cases/tree_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab: usize` 字段
- 新增 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!("tree_case.rml").to_string()` / `include_str!("tree_case.rml.rs").to_string()`
- 删除 `on_code_tab_change` 命令
- `on_loaded` 中新增 `self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));`
- 新增 section 2/3/4 所需字段：
  - `expanded_state: Option<gpui::Entity<TreeState>>`（section 2：expanded 初始展开）
  - `nested_state: Option<gpui::Entity<TreeState>>`（section 3：多级嵌套）
  - `select_state: Option<gpui::Entity<TreeState>>`（section 4：on-select 演示）
  - `last_selected: SharedString`（section 4：on-select 状态）
- 新增 computed：`select_status_text` 返回 on-select 状态文本
- 新增 command：`on_select(&mut self, item_id: &SharedString, cx)` 处理 on-select 事件
- API 表格扩展为 4 行：ref/on_activate/on_select/TreeState::items

**.rml 改造**：
- 套 `<CaseDocPage title={t("case.tree.title")} description="..." code-rml={rml_sample} code-rust={rust_sample}>`
- demo 移入 `<template slot="demo">`
- 4 个 `<div class="demo-section">`：
  1. 基础用法 + TreeState 初始化（ref="tree_state" on-activate={on_activate}）
  2. expanded 初始展开（ref="expanded_state"，第二个 Tree 含 expanded 节点）
  3. 多级嵌套树（ref="nested_state"，深层 child 链）
  4. on-activate vs on-select（ref="select_state" 同时绑定 on-activate 和 on-select，对比行为差异）
- API 移入 `<template slot="api">`
- 删除 `<Card>`/`<TabBar>`/`<CodeEditor>` 脚手架

**验证**: `cargo build -p rust-rml-demo` 通过，运行后 4 个 section 交互正常

### Step 3: 迁移 Accordion 案例到 Canonical 模式

**文件**: [demo/src/cases/accordion_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs) + [demo/src/cases/accordion_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab: usize` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`
- `on_loaded` 中初始化 `case_doc_page`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留 6 个 section（basic/multiple/sizes/icon/disabled/nested）
- 移除所有 `<code>` 标签，改用纯文本描述（如 `<code>bordered</code>` → `bordered`）
- 删除 `<Card>`/`<TabBar>` 脚手架

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 4: 迁移 Alert 案例到 Canonical 模式

**文件**: [demo/src/cases/alert_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml.rs) + [demo/src/cases/alert_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留 7 个 section（variant 关联函数/variant 属性/title+banner/message 优先级/icon/on_close+if/size）
- 移除所有 `<code>` 标签

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 5: 迁移 Avatar 案例到 Canonical 模式

**文件**: [demo/src/cases/avatar_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml.rs) + [demo/src/cases/avatar_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 将"内容模式"拆为 src/name/placeholder 各独立 section（教学价值：明确三种内容模式的差异）
- 保留"动态绑定"section（model + computed + if 三联，高价值）
- 移除所有 `<code>` 标签

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 6: 迁移 Popover 案例到 Canonical 模式

**文件**: [demo/src/cases/popover_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml.rs) + [demo/src/cases/popover_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留 3 个 section（基础/anchor/default-open）
- 新增 1 个 section 专门说明 `slot="trigger"` 机制（具名 slot 路由到 .trigger() 的 codegen 路径）
- 移除所有 `<code>` 标签

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 7: Table 案例去除 1 处 `<code>` 标签

**文件**: [demo/src/cases/table_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) line 52

**改造**：
- 将 `<code>&lt;template slot="cell" field="name"&gt;</code>` 改为纯文本 `template slot="cell" field="name"`
- 将 `<code>row_idx</code>` 改为纯文本 `row_idx`

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 8: 校准 5 个 Canonical 案例 API 表格

**文件**：
- [button_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml.rs)
- [badge_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/badge_case.rml.rs)
- [checkbox_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/checkbox_case.rml.rs)
- [icon_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/icon_case.rml.rs)
- [tooltip_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tooltip_case.rml.rs)

**改造**：
- 逐一核对 `build_api_table` 三元组与 RML 真实属性对齐
- 仅更新内容，不重构结构
- 重点核对：
  - Button: 9 种 variant、size、disabled/selected/loading、compact、tooltip
  - Badge: count/max/dot/icon/size/子节点
  - Checkbox: checked/label/disabled/size/on_change
  - Icon: name/size/color
  - Tooltip: content/placement/trigger

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 9: 全局验证

**编译验证**:
```powershell
cargo build -p rust-rml-demo
```
零错误零警告（特别关注未使用字段/导入警告）

**Grep 全局校验**:
- 12 个案例都有 `include_str!`
- 12 个案例都有 `case_doc_page` 字段
- 12 个案例都无 `on_code_tab_change` 命令
- 12 个案例都无 `code_tab` 字段
- 12 个 `.rml` 文件都无 `<code>` 标签

**运行验证**:
```powershell
cargo run -p rust-rml-demo
```
逐个案例逐 section 交互，对照代码区显示的源码确认一致

## 假设与决策

1. **Input size 条件渲染**：因 `&str` 不实现 `Into<Size>`，size 绑定不可用。改用 if 指令条件渲染 4 个静态 size 的 Input。若 if+ref 共存 codegen 不支持，则用 4 个不同 ref 名字段。
2. **include_str! 自引用**：`include_str!("input_case.rml")` 会包含自身这行，形成"自引用"。这是预期行为（展示完整文件），代码区会显示递归字符串。
3. **Tree on-select 事件签名**：参考 on-activate 的 `&SharedString` 签名，on-select 应类似。若实际签名不同，以 [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 中 Tree 的事件订阅代码生成为准。
4. **Canonical 案例微调**：仅核对 API 表格准确性，不重写已工作的 section，避免引入回归。
5. **不新增 RML 框架特性**：如发现官方文档有但 RML 未映射的特性，在案例中跳过，不扩展 codegen。

## 风险

1. **Input if+ref 共存**：需验证 if 指令与 ref 指令在同一元素上的 codegen 行为。若不支持，需用 4 个不同 ref 字段。
2. **Tree on-select 签名**：需验证实际回调签名，可能与 on-activate 不同。
3. **Canonical 案例微调回归**：修改 API 表格时需谨慎，仅更新内容不重构结构。
4. **include_str! 编译时检查**：若文件路径错误，编译时报错，需确保路径正确。

## 验证清单

- [ ] Input 案例 3 个编译错误已修复
- [ ] Tree 案例迁移到 Canonical 模式，4 个 section 完整
- [ ] Accordion 案例迁移到 Canonical 模式，无 `<code>` 标签
- [ ] Alert 案例迁移到 Canonical 模式，无 `<code>` 标签
- [ ] Avatar 案例迁移到 Canonical 模式，src/name/placeholder 拆分
- [ ] Popover 案例迁移到 Canonical 模式，slot="trigger" 说明完整
- [ ] Table 案例 1 处 `<code>` 标签已删除
- [ ] 5 个 Canonical 案例 API 表格已核对
- [ ] `cargo build -p rust-rml-demo` 零错误零警告
- [ ] Grep 全局校验通过（12 案例 include_str!/case_doc_page/无 code_tab/无 on_code_tab_change/无 `<code>`）
