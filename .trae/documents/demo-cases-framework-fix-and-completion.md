# RML Demo 案例官方文档化 — 框架修复与完成计划

## 摘要

本计划承接前序工作，聚焦于解决阻塞性框架限制并完成 12 个已注册 RML 组件案例的官方文档化。核心阻塞是 `<Input ref="...">` 等 Stateful 组件在 `<CaseDocPage><template slot="demo">` 插槽中无法编译——因为 `component.rs` 硬编码 `self.__rml_state.get_or_init_ref(...)`，而插槽闭包是 `Fn` 不能捕获 `&mut self`。本计划通过复用 `tab.rs::extract_body_deps` 的 prelude 提取机制修复此限制，然后完成 5 个 Legacy 案例迁移和 `<code>` 标签清理。

## 当前状态分析

### 框架限制（阻塞 Input 案例编译）

**根因链**：
1. [user_component.rs:107](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs#L107) 生成插槽闭包：
   ```rust
   move |_scope, _window, _app| {
       __rml_self_entity.update(_app, |this, cx| {
           let __rml_self_ref: &Self = this;  // 不可变借用
           (SLOT_CODE).into_any_element()
       })
   }
   ```
2. [component.rs:231,252](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L231) 硬编码 `self.__rml_state.get_or_init_ref(...)`（非表达式，不走 alias 替换）
3. `self` 被 `move` 闭包捕获，但 `self` 是 `&mut`，`Fn` 闭包不能捕获 `&mut`
4. [expr.rs:183](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs#L183) `current_self_alias()` 返回 `__rml_self_ref`（`&Self` 不可变），无法调用 `&mut self` 方法

**现有解决方案**：[tab.rs:52-153](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tabs/tab.rs#L52) 的 `extract_body_deps` 函数已为 `<tab>` body 闭包解决此问题——通过将 `self.__rml_state.get_or_init_ref(...)` 提取到闭包外的 prelude（render 作用域，`self` 是 `&mut Self`），闭包 `move` 捕获提取的 Entity 变量。

### 12 个案例当前状态

| 案例 | 模式 | section 数 | `<code>` 标签 | 状态 |
|---|---|---|---|---|
| badge | Canonical | 9 | 0 | ✓ 可用 |
| button | Canonical | 9 | 0 | ✓ 可用 |
| checkbox | Canonical | 4 | 0 | ✓ 可用 |
| icon | Canonical | 5 | 0 | ✓ 可用 |
| tooltip | Canonical | 4 | 0 | ✓ 可用 |
| table | Canonical | 6 | 2 | 待去 `<code>` |
| input | Canonical | 6 | 0 | ✗ 框架限制阻塞 |
| tree | Legacy | 1 | 0 | 待迁移 |
| accordion | Legacy | 3 | 18 | 待迁移 + 去 `<code>` |
| alert | Legacy | 4 | 18 | 待迁移 + 去 `<code>` |
| avatar | Legacy | 3 | 5 | 待迁移 + 去 `<code>` |
| popover | Legacy | 3 | 10 | 待迁移 + 去 `<code>` |

**Canonical 标志**：有 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段 + `include_str!`
**Legacy 标志**：有 `code_tab: usize` 字段 + `on_code_tab_change` 命令 + 硬编码代码字符串

### 官方文档标准（参考 button 组件）

每个组件文档包含：导入 → 用法（基础/变体/尺寸/图标/状态等子节）→ 示例（Tooltip/自定义内容等）。每个子节有代码示例和说明文字。RML 案例通过 CaseDocPage 模板对齐此标准：title + description + demo slot（多 section）+ api slot（属性表格）+ code tabs（include_str! 真实源码）。

## 实施步骤

### Step 1: 框架修复 — 提取 `extract_state_refs` 公共函数

**文件**: [crates/engine/src/compiler/tabs/tab.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tabs/tab.rs)

**改造**：
1. 从 `extract_body_deps` 函数（L52-153）中提取步骤 1（`get_or_init_ref` 提取）为独立公共函数：
   ```rust
   /// 提取 body_code 中的 self.__rml_state.get_or_init_ref(...) 调用到 prelude
   /// 返回 (prelude, replaced_body)
   /// prelude 在 render 作用域执行（self 是 &mut Self），闭包 move 捕获提取的 Entity 变量
   pub fn extract_state_refs(body_code: &str) -> (String, String) {
       // 复用 extract_body_deps 步骤 1 的逻辑：
       // 用括号匹配找到 self.__rml_state.get_or_init_ref(...) 完整调用
       // 提取为 let __rml_entity_N = self.__rml_state.get_or_init_ref(...);
       // 替换 body 中的调用为 __rml_entity_N
   }
   ```
2. `extract_body_deps` 内部调用 `extract_state_refs`（保持原有行为不变，避免回归）

**验证**: `cargo test -p rml-engine`（确保 tab 相关测试通过）

### Step 2: 框架修复 — 在插槽闭包中应用 `extract_state_refs`

**文件**: [crates/engine/src/compiler/user_component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

**改造**（L98-127 区域）：
1. 导入 `use crate::compiler::tabs::tab::extract_state_refs;`
2. 对每个具名 slot（L98-114）：
   ```rust
   for (slot_name, slot_nodes) in &slot_children {
       let slot_code = expr::with_self_alias("__rml_self_ref", || {
           gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)
       })?;
       // 新增：提取 get_or_init_ref 到 prelude
       let (prelude, slot_code_replaced) = extract_state_refs(&slot_code);
       let binding = format!("__rml_slot_{}_value", slot_name);
       // 先发射 prelude（render 作用域，self 是 &mut Self）
       if !prelude.is_empty() {
           code.push_str(&format!("    {}\n", prelude));
       }
       // 闭包捕获 prelude 中的 __rml_entity_N 变量
       code.push_str(&format!(
           "    let {}: rml_core::slot::SlotRenderer = Box::new({{ let __rml_self_entity = __rml_self_entity.clone(); move |_scope: &dyn rml_core::slot::ISlotScope, _window: &mut gpui::Window, _app: &mut gpui::App| -> gpui::AnyElement {{ __rml_self_entity.update(_app, |this, cx| {{ let __rml_self_ref: &Self = this; ({}).into_any_element() }}) }} }});\n",
           binding, slot_code_replaced
       ));
       // ... 后续 setter 注入不变 ...
   }
   ```
3. 对 default slot（L117-127）做同样处理

**原理**：
- prelude 在 render 方法作用域执行，`self` 是 `&mut Self`，`get_or_init_ref` 可用
- prelude 产生的 `let __rml_entity_0 = self.__rml_state.get_or_init_ref(...);` 返回 `Entity<T>`（by value，不借用 self）
- `move` 闭包捕获 `__rml_entity_0`（`Entity<T>: Send + Sync + 'static`）
- 闭包内用 `__rml_entity_0` 替代 `self.__rml_state.get_or_init_ref(...)`，不再需要 `self`
- 借用不冲突：prelude 的 `&mut self` 借用在 `get_or_init_ref` 返回后结束，后续属性注入的 `self.xxx()` 不受影响

**验证**: `cargo build -p rust-rml-demo`（Input 案例应能编译通过）

### Step 3: 验证 Input 案例

**文件**: [demo/src/cases/input_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml) + [demo/src/cases/input_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs)

**改造**：
- 框架修复后，当前 6 个 section 的 `<Input ref="..." />` 应能编译
- 检查生成的代码确认 `get_or_init_ref` 已被提取到 prelude
- 如有剩余编译错误，逐一修复（可能涉及 `__rml_self_ref` 借用与 `this` 的交互）

**6 个 section 教学价值核对**：
1. 基础用法 + ref 指令（ElementRef 模式）
2. placeholder 设置时机（Pattern B：on_loaded 中创建 InputState）
3. disabled 禁用（组件属性绑定）
4. 尺寸 size（Sizable trait + Size 枚举返回）
5. selected 选中态（Selectable trait）
6. 多 Input 组合（表单布局）

**验证**: `cargo build -p rust-rml-demo` 零错误，运行后 6 个 section 交互正常

### Step 4: 迁移 Tree 案例到 Canonical 模式

**文件**: [demo/src/cases/tree_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml.rs) + [demo/src/cases/tree_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab: usize` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!("tree_case.rml").to_string()` / `include_str!("tree_case.rml.rs").to_string()`
- `on_loaded` 中初始化 `case_doc_page` 和 API 表格
- 新增 section 2/3/4 所需的 TreeState Entity 字段

**.rml 改造**：
- 套 `<CaseDocPage title={t("case.tree.title")} description="..." code-rml={rml_sample} code-rust={rust_sample}>`
- demo 移入 `<template slot="demo">`
- 4 个 section：
  1. 基础用法 + on-activate 事件
  2. expanded 初始展开
  3. 多级嵌套树
  4. on-activate vs on-select 行为对比
- API 移入 `<template slot="api">`

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 5: 迁移 Accordion 案例到 Canonical 模式

**文件**: [demo/src/cases/accordion_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs) + [demo/src/cases/accordion_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`
- `on_loaded` 中初始化 `case_doc_page`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留现有 section（basic/multiple/sizes/icon/disabled/nested）
- **移除所有 18 处 `<code>` 标签**，改用纯文本（如 `<code>bordered</code>` → `bordered`）
- 删除 `<Card>`/`<TabBar>` 脚手架

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 6: 迁移 Alert 案例到 Canonical 模式

**文件**: [demo/src/cases/alert_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml.rs) + [demo/src/cases/alert_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留现有 section（variant 关联函数/variant 属性/title+banner/message 优先级/icon/on_close+if/size）
- **移除所有 18 处 `<code>` 标签**

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 7: 迁移 Avatar 案例到 Canonical 模式

**文件**: [demo/src/cases/avatar_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml.rs) + [demo/src/cases/avatar_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留现有 section，将"内容模式"拆为 src/name/placeholder 各独立 section
- 保留"动态绑定"section
- **移除所有 5 处 `<code>` 标签**

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 8: 迁移 Popover 案例到 Canonical 模式

**文件**: [demo/src/cases/popover_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml.rs) + [demo/src/cases/popover_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml)

**.rml.rs 改造**：
- 删除 `code_tab` 字段和 `on_code_tab_change` 命令
- 新增 `case_doc_page` 字段
- `rml_sample`/`rust_sample` 改为 `include_str!`

**.rml 改造**：
- 套 `<CaseDocPage>`
- 保留现有 section（基础/anchor/default-open）
- 新增 1 个 section 说明 `slot="trigger"` 机制
- **移除所有 10 处 `<code>` 标签**

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 9: Table 案例去除 `<code>` 标签

**文件**: [demo/src/cases/table_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml)

**改造**：
- 将 2 处 `<code>...</code>` 改为纯文本
- 如 `<code>template slot="cell"</code>` → `template slot="cell"`

**验证**: `cargo build -p rust-rml-demo` 通过，grep 确认无 `<code>` 标签

### Step 10: 校准 12 个案例 API 表格

**文件**: 12 个 `*_case.rml.rs` 文件的 `build_api_table` 调用

**改造**：
- 逐一核对 API 三元组（属性/类型/说明）与 RML tags.rs 和官方文档对齐
- 重点核对：
  - Button: 9 种 variant、size、disabled/selected/loading、compact、tooltip
  - Badge: count/max/dot/icon/size/子节点
  - Checkbox: checked/label/disabled/size/on_change
  - Icon: name/size/color
  - Tooltip: content/placement/trigger
  - Table: columns/rows/bordered/stripe/delegate
  - Input: ref/disabled/size/selected/on_change/on_enter/on_focus/on_blur/model/placeholder
  - Tree: ref/on_activate/on_select/TreeState::items
  - Accordion: items/expand_multiple/bordered
  - Alert: variant/title/message/banner/icon/on_close/size
  - Avatar: src/name/placeholder/size/icon
  - Popover: trigger/placement/default_open/slot="trigger"

**验证**: `cargo build -p rust-rml-demo` 通过

### Step 11: 全局验证

**编译验证**:
```powershell
cargo build -p rust-rml-demo
```
零错误零警告

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

1. **框架修复方案选择**：选择 `extract_state_refs` prelude 提取方案（而非 `this.` 替换方案），因为：
   - 复用 `tab.rs` 已验证的机制，风险最低
   - 不需要修改 `component.rs` 和 `event.rs` 的多处硬编码
   - 不存在 `__rml_self_ref`（`&Self`）与 `this`（`&mut Self`）的借用冲突
   - prelude 在 render 作用域执行，`self` 是 `&mut Self`，`get_or_init_ref` 可用

2. **Input 事件订阅（on-change 等）**：本次不修复 `gen_input_event_subscribe` 中的 `self.__rml_state.is_event_subscribed/mark_event_subscribed` 硬编码问题。Input 案例不使用 on-change 事件（避免 `entity.read(cx)` 与 `cx` 的借用冲突）。事件订阅在插槽中的支持留作后续框架增强。

3. **CodeEditor 在插槽中**：`extract_state_refs` 仅提取 `get_or_init_ref`，不提取 `let __code = __rml_self_ref.xxx();` 语句（因 `__rml_self_ref` 不在 prelude 作用域）。当前 12 个案例不在 demo slot 中使用 CodeEditor，故不影响。CodeEditor 在 `<tab>` body 中仍由 `extract_body_deps` 完整处理。

4. **include_str! 自引用**：`include_str!("input_case.rml")` 包含自身这行代码，形成"自引用"。这是预期行为（展示完整文件），代码区会显示递归字符串。教学价值：用户看到的就是真实源码。

5. **不新增 RML 框架特性**：如发现官方文档有但 RML 未映射的特性，在案例中跳过，不扩展 codegen。

## 风险

1. **prelude 变量捕获**：需验证 `move` 闭包正确捕获 prelude 中的 `__rml_entity_N` 变量。如果闭包体内未引用该变量（理论上不可能，因为 `extract_state_refs` 只在 `get_or_init_ref` 被使用时提取），不会生成 prelude。

2. **多 slot 场景**：CaseDocPage 有 demo 和 api 两个 slot。需确保每个 slot 的 prelude 变量名不冲突（`extract_state_refs` 内部有 `entity_counter` 递增，但跨 slot 调用需重置或用不同前缀）。方案：每个 slot 调用 `extract_state_refs` 时，counter 从 0 开始，但变量名用 `__rml_slot_{slot_name}_entity_{N}` 避免冲突。

3. **Canonical 案例微调回归**：修改 API 表格时需谨慎，仅更新内容不重构结构。

## 验证清单

- [ ] `extract_state_refs` 公共函数已从 `tab.rs` 提取
- [ ] `user_component.rs` 插槽闭包已应用 `extract_state_refs`
- [ ] Input 案例 6 个 section 编译通过且交互正常
- [ ] Tree 案例迁移到 Canonical 模式，4 个 section 完整
- [ ] Accordion 案例迁移到 Canonical 模式，无 `<code>` 标签
- [ ] Alert 案例迁移到 Canonical 模式，无 `<code>` 标签
- [ ] Avatar 案例迁移到 Canonical 模式，src/name/placeholder 拆分，无 `<code>` 标签
- [ ] Popover 案例迁移到 Canonical 模式，slot="trigger" 说明完整，无 `<code>` 标签
- [ ] Table 案例 2 处 `<code>` 标签已删除
- [ ] 12 个案例 API 表格已核对
- [ ] `cargo build -p rust-rml-demo` 零错误零警告
- [ ] Grep 全局校验通过（12 案例 include_str!/case_doc_page/无 code_tab/无 on_code_tab_change/无 `<code>`）
