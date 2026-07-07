# RML Slot 工业级支持 —— Step 5-8 续接执行计划

## 背景

本计划续接 [rml-slot-phase1-2-case-doc-page-execution.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-slot-phase1-2-case-doc-page-execution.md)，因上下文丢失后重新确认状态并继续执行。

用户核心诉求："case_doc_page模板如果真的需要，也应该按照.rml规范编写才对" —— 要求放弃 Rust builder 方案，改为以标准 RML 组件实现 CaseDocPage 共享模板。

## 当前进度确认

| 步骤 | 状态 | 验证依据 |
|------|------|---------|
| Step 1 (Phase 1.3): gen_user_component 属性传参 | ✅ 已完成 | [user_component.rs:117-194](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) `gen_prop_assign` + `gen_static_assign` + `gen_bind_assign` 已实现 |
| Step 2 (Phase 1.4): Phase 1 单元测试 | ✅ 已完成 | [user_component.rs:264-498](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 11 个测试用例 |
| Step 3 (Phase 2.1): CodegenCtx 添加 self_alias 字段 | ✅ 已完成 | [compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) `self_alias: Option<String>` 字段已添加 |
| Step 4 (Phase 2.2): 表达式生成支持 self_alias | ✅ 已完成 | [expr.rs:160-185](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) thread-local `CURRENT_SELF_ALIAS` + `with_self_alias` / `current_self_alias`（**实际实现采用 thread-local，优于原计划的参数透传，0 调用点修改**） |
| Step 5 (Phase 2.3): gen_user_component slot 闭包改造 | ⏳ 待实施 | 当前 [user_component.rs:79-101](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) slot 闭包未捕获父视图 self |
| Step 6 (Phase 2.4): Phase 2 单元测试 | ⏳ 待实施 | — |
| Step 7: CaseDocPage 新建 .rml + .rml.rs | ⏳ 待实施 | 当前 [case_doc_page.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rs) 仍是 Rust builder |
| Step 8: CaseDocPage 改造 table_case 验证 | ⏳ 待实施 | [table_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) 当前用 `<component>` 直接渲染 |

## 关键技术点确认（探索阶段验证）

1. **`cx.entity()` 可用**：[tab_bar/setters.rs:169](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs) 和 [code_editor/gen.rs:232](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs) 已使用，返回 `Entity<Self>`
2. **`Entity<Self>: Send + Sync + 'static`**：[state.rs:85-89](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs) 注释明确"不依赖 T 的 Send/Sync"
3. **`<slot name="xxx" />` 语法已支持**：[codegen/node.rs:255-264](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 生成 `self.__rml_state.slot(<name>)` 调用
4. **`#[component(slots = [...])]` 宏已支持**：[macros/src/component.rs:190-209](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs) 自动生成 `__rml_set_slot_<name>` setter
5. **thread-local self_alias 机制已就绪**：[expr.rs:172-180](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) `with_self_alias` + `current_self_alias`，[expr.rs:194-244](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) `to_rust_code_with_ctx` 已读取 alias
6. **codegen/text.rs 已读取 alias**：[text.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/text.rs) `gen_expr_code` 已用 `expr::current_self_alias()` 替换前缀
7. **component.rs 已读取 alias**：[component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) `component_bind_rust_expr` 已用 `expr::current_self_alias()` 替换前缀

## 实施步骤

### 步骤 5：Phase 2.3 —— gen_user_component slot 闭包改造

**目标**：让 slot 闭包通过 `Entity<Self>` 捕获父视图数据，使 `<template slot="demo">` 内能引用 `self.items` 等父视图字段。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

**修改 1：新增 expr 导入**（line 9-12 区域）

当前：
```rust
use crate::compiler::codegen::gen_node;
use crate::compiler::component::{component_bind_rust_expr, parse_bool};
use crate::compiler::{CodegenCtx, CodegenError, UserComponentInfo};
use crate::parser::ast::{Attribute, Element};
```

改为：
```rust
use crate::compiler::codegen::gen_node;
use crate::compiler::component::{component_bind_rust_expr, parse_bool};
use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError, UserComponentInfo};
use crate::parser::ast::{Attribute, Element};
```

**修改 2：在 `gen_user_component` 中，当有 slot 内容时生成 `__rml_self_entity` 捕获**

在 [user_component.rs:61-68](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 的 `let mut code = String::new();` 之后、属性注入之前，新增：

```rust
let has_slots = !slot_children.is_empty()
    || (!default_children.is_empty() && info.slots.iter().any(|s| s == "default"));

// 有 slot 内容时，捕获父视图 Entity，让 slot 闭包可通过 __rml_self_ref 引用父视图数据。
// Entity<Self>: Send + Sync + 'static（不依赖 T 的 Send/Sync），可被 move 闭包捕获。
if has_slots {
    code.push_str("    let __rml_self_entity = cx.entity();\n");
}
```

**修改 3：具名 slot 闭包改造**（[user_component.rs:79-90](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)）

当前：
```rust
for (slot_name, slot_nodes) in &slot_children {
    let slot_code = gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)?;
    let binding = format!("__rml_slot_{}_value", slot_name);
    code.push_str(&format!(
        "    let {}: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement {{ ({}).into_any_element() }});\n",
        binding, slot_code
    ));
    code.push_str(&format!(
        "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_{}({}); }});\n",
        slot_name, binding
    ));
}
```

改为：
```rust
for (slot_name, slot_nodes) in &slot_children {
    // 用 with_self_alias 包裹 gen_slot_content，使 slot 内容中的 self.xxx
    // 被替换为 __rml_self_ref.xxx（thread-local 机制，0 调用点修改）
    let slot_code = expr::with_self_alias("__rml_self_ref", || {
        gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)
    })?;
    let binding = format!("__rml_slot_{}_value", slot_name);
    code.push_str(&format!(
        "    let {}: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement {{ let __rml_self_ref = __rml_self_entity.read(cx); ({}).into_any_element() }});\n",
        binding, slot_code
    ));
    code.push_str(&format!(
        "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_{}({}); }});\n",
        slot_name, binding
    ));
}
```

**修改 4：default 插槽闭包改造**（[user_component.rs:92-101](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)）

当前：
```rust
if !default_children.is_empty() && info.slots.iter().any(|s| s == "default") {
    let default_code = gen_slot_content(&default_children, ctx, id_counter, loop_vars)?;
    code.push_str("    let __rml_slot_default_value: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement { (");
    code.push_str(&default_code);
    code.push_str(").into_any_element() });\n");
    code.push_str(
        "    __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_default(__rml_slot_default_value); });\n",
    );
}
```

改为：
```rust
if !default_children.is_empty() && info.slots.iter().any(|s| s == "default") {
    let default_code = expr::with_self_alias("__rml_self_ref", || {
        gen_slot_content(&default_children, ctx, id_counter, loop_vars)
    })?;
    code.push_str("    let __rml_slot_default_value: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement { let __rml_self_ref = __rml_self_entity.read(cx); (");
    code.push_str(&default_code);
    code.push_str(").into_any_element() });\n");
    code.push_str(
        "    __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_default(__rml_slot_default_value); });\n",
    );
}
```

**修改 5：更新模块级文档注释**（[user_component.rs:1-7](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)）

在处理场景中补充 slot 闭包捕获父视图数据的说明：
```rust
//! 处理场景：
//! - 无属性无 slot：直接 clone entity
//! - 有属性：clone entity 后通过 `entity.update(cx, ...)` 注入属性值
//! - 有 slot：clone entity 后通过 `entity.update(cx, ...)` 注入 slot 渲染闭包
//!   slot 闭包通过 `cx.entity()` 捕获父视图 Entity<Self>，闭包内用
//!   `__rml_self_ref = entity.read(cx)` 获取父视图引用，使 slot 内容可引用
//!   父视图字段（self.items 等）。Entity<Self>: Send + Sync + 'static，可被 move 捕获。
```

**验证**：`cargo check -p rust-rml-engine` 编译通过。

---

### 步骤 6：Phase 2.4 —— Phase 2 单元测试

**目标**：验证 slot 闭包通过 Entity 捕获父视图数据。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 测试模块新增用例。

**新增测试用例**：

1. `test_slot_closure_generates_self_entity_capture` —— 有 slot 时生成 `let __rml_self_entity = cx.entity();`
2. `test_slot_closure_no_self_entity_without_slots` —— 无 slot 时不生成 `__rml_self_entity`
3. `test_slot_closure_replaces_self_with_alias` —— slot 内引用 `self.items` → 生成 `__rml_self_ref.items`（验证 thread-local self_alias 生效）
4. `test_slot_closure_computed_method` —— slot 内调用 `self.format_items()`（computed）→ 生成 `__rml_self_ref.format_items()`
5. `test_default_slot_closure_uses_alias` —— default 插槽内引用 `self.data` → 生成 `__rml_self_ref.data`

**测试构造模式**：
- 用 `make_info_with_slots("CaseDocPage", &[("title", "SharedString")], &["demo", "api"])` 构造组件信息
- 用 `make_element("CaseDocPage", vec![], vec![template_slot_node("demo", vec![text_node("{items}")])])` 构造带 slot 的元素
- 注意：现有测试辅助函数 `make_element` / `static_attr` 等需扩展支持 `<template slot>` 子节点构造

**验证**：`cargo test -p rust-rml-engine --lib user_component` 全部通过；`cargo test -p rust-rml-engine` 全量测试通过（无回归）。

---

### 步骤 7：CaseDocPage 改造 —— 新建 .rml + .rml.rs

**目标**：用标准 RML 组件替换 [case_doc_page.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rs) 的 Rust builder 实现。

**新建文件 1**：`demo/src/cases/common/case_doc_page.rml`

```rml
<component>
  <div class="case-doc-page" v-flex="">
    <!-- 标题区 -->
    <div class="doc-header" v-flex="" gap="4">
      <h2 class="doc-title" text-size="22" font-weight="semibold">{title}</h2>
      <div class="doc-desc" text-size="14" text-color="muted" max-w="800">{description}</div>
    </div>

    <!-- 演示区 + 代码区 -->
    <div class="case-layout" h-flex="" gap="24" flex-wrap="" items-start="">
      <div class="case-demo-panel" v-flex="" flex-1="" min-w="320" flex-basis="420" gap="8">
        <div text-size="14" font-weight="semibold">{t("case.common.demo")}</div>
        <slot name="demo" />
      </div>
      <div class="case-code-panel" v-flex="" flex-1="" min-w="320" flex-basis="420" gap="8">
        <div text-size="14" font-weight="semibold">{t("case.common.code")}</div>
        <TabBar selected_index={code_tab} on_click={on_code_tab_change}>
          <Tab label=".rml" />
          <Tab label=".rml.rs" />
        </TabBar>
        <div class="code-block" w-full="" border-1="" border-color="border" bg="muted" rounded="2" p="12" px="16" text-color="foreground" text-size="13" line-height="20">
          {current_code}
        </div>
      </div>
    </div>

    <!-- API 区 -->
    <div class="case-api-panel" v-flex="" gap="8">
      <div text-size="14" font-weight="semibold">{t("case.common.api")}</div>
      <slot name="api" />
    </div>
  </div>
</component>
```

**说明**：
- `<slot name="demo" />` / `<slot name="api" />` 使用 Vue 风格语法（[codegen/node.rs:255](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 已支持）
- `{current_code}` 为 computed 方法（返回当前 Tab 对应的代码字符串），避免 RML 表达式内联 if
- `{t("case.common.demo")}` 等 i18n 调用已在 RML 中支持
- 样式以内联方式实现（与原 builder 一致），CSS class 仅作语义标记

**新建文件 2**：`demo/src/cases/common/case_doc_page.rml.rs`

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

/// 案例页共享模板组件
///
/// 统一所有案例页的四段式布局：标题区 + 演示区 + 代码区 + API 区。
/// 通过 `#[component(slots = ["demo", "api"])]` 声明具名插槽，
/// 父视图用 `<template slot="demo">...</template>` 注入内容。
#[component(slots = ["demo", "api"])]
#[derive(Default)]
pub struct CaseDocPage {
    /// 案例标题
    pub title: SharedString,
    /// 案例描述
    pub description: SharedString,
    /// .rml 源码
    pub code_rml: String,
    /// .rs 源码
    pub code_rust: String,
    /// 代码 Tab 当前索引（0=RML, 1=Rust）
    pub code_tab: usize,
}

impl CaseDocPage {
    /// 当前 Tab 对应的代码字符串
    ///
    /// RML 表达式不支持内联 if，用 computed 方法桥接。
    #[computed]
    pub fn current_code(&self) -> String {
        if self.code_tab == 0 {
            self.code_rml.clone()
        } else {
            self.code_rust.clone()
        }
    }

    /// 切换代码 Tab
    ///
    /// TabBar on_click 事件签名：`fn(&mut self, idx: usize, &mut Context<Self>)`
    /// （参考 [tab_bar/setters.rs:159-164](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs)）
    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, cx: &mut gpui::Context<Self>) {
        self.code_tab = idx;
        cx.notify();
    }
}
```

**删除文件**：`demo/src/cases/common/case_doc_page.rs`

**更新 mod.rs**：[demo/src/cases/common/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/mod.rs) 中 `mod case_doc_page;` 保持不变（RML 宏会自动处理 .rml.rs）。

**验证**：`cargo check -p rust-rml-demo` 编译通过。

---

### 步骤 8：CaseDocPage 改造 —— 改造 table_case 验证

**目标**：将 table_case 改造为使用 `<CaseDocPage>` + slot 的形式，端到端验证框架增强。

**修改文件**：[demo/src/cases/table_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) + [demo/src/cases/table_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml.rs)

**table_case.rml 改造**：

当前（`<component>` 直接渲染所有内容）→ 改造后（`<CaseDocPage>` + slot）：

```rml
<component>
  <CaseDocPage 
    title={t("case.table.title")} 
    description="Table 是 RML 的 WPF DataGrid 风格表格组件..."
    code_rml={rml_sample}
    code_rust={rust_sample}>
    <template slot="demo">
      <div class="demo-section">
        <h3>1. 数据绑定式（API 文档表格）</h3>
        <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
      </div>
      <div class="demo-section">
        <h3>2. 声明式 Column 定义</h3>
        <Table rows={user_rows} bordered="">
          <Column key="name" title="姓名" width="120" />
          <Column key="age" title="年龄" align="center" />
          <Column key="email" title="邮箱" />
        </Table>
      </div>
      <!-- ... 其他 demo-section ... -->
    </template>
    <template slot="api">
      <Table columns={api_columns} rows={api_rows} bordered="" />
    </template>
  </CaseDocPage>
</component>
```

**table_case.rml.rs 改造**：

1. **新增 `case_doc_page` 字段**：
```rust
#[component]
#[derive(Default)]
pub struct TableCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub user_rows: Vec<TableRow>,
    pub merged_rows: Vec<TableRow>,
    pub code_tab: usize,  // 保留（CaseDocPage 内部管理，但 table_case 无需直接使用）
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,  // 新增
}
```

2. **保留现有 `rml_sample` / `rust_sample` computed 方法**（返回精简示例代码，不改为 `include_str!`，避免改变示例内容语义）

3. **移除 `on_code_tab_change` 命令**（Tab 状态由 CaseDocPage 内部管理，table_case 不再需要）

4. **在 `on_loaded` 中初始化 `case_doc_page`**（若 RML 框架未自动初始化用户组件字段）

**关键决策**：
- **code_rml/code_rust 属性值来源**：复用现有 `rml_sample` / `rust_sample` computed 方法（返回精简示例），不改为 `include_str!`（保持示例代码的精简语义，避免展示完整文件中的样板代码）
- **Tab 状态管理**：由 CaseDocPage 内部管理（`code_tab` 字段 + `on_code_tab_change` 命令），table_case 不再持有 `code_tab` 字段（可移除）
- **description 属性**：用静态字符串（RML 静态属性），不通过 i18n（描述文本较长，i18n 维护成本高）

**验证**：
1. `cargo check -p rust-rml-demo` 编译通过
2. `cargo run -p rust-rml-demo` 运行，导航到 table case，验证：
   - 标题 + 描述正确显示
   - 演示区 6 个 Table 示例渲染正常
   - 代码区 Tab 切换正常（.rml / .rml.rs）
   - API 区 Table 渲染正常
   - 视觉效果与原 builder 模式一致

---

## 关键设计决策

### 决策 1：self_alias 用 thread-local 而非参数透传（Step 4 已实现）

原计划向 `to_rust_code_with_ctx` / `gen_expr_code` / `component_bind_rust_expr` 添加 `self_alias: Option<&str>` 参数，需修改 30+ 调用点。实际改用 thread-local `CURRENT_SELF_ALIAS`，0 调用点修改，更优雅。

Step 5 中 `gen_slot_content` 调用用 `expr::with_self_alias("__rml_self_ref", || { ... })` 包裹即可。

### 决策 2：CaseDocPage 的 code_tab 状态管理

CaseDocPage 内部持有 `code_tab: usize` 字段 + `on_code_tab_change` 命令，父视图无需管理。父视图仅通过 `code_rml` / `code_rust` 属性传入代码字符串。

### 决策 3：table_case 的示例代码来源

复用现有 `rml_sample` / `rust_sample` computed 方法（返回精简示例代码字符串），不改为 `include_str!`。原因：
- 现有示例代码是手工精简的演示版本，非完整文件
- `include_str!` 会展示完整文件（含样板代码），降低示例的可读性
- 保持现有行为，减少改造范围

### 决策 4：不实施 Phase 3（scoped slot）和 Phase 4（编译期校验）

CaseDocPage 不需要 scoped slot（slot 内仅引用父视图数据，不需要子组件回传）。编译期校验（Phase 4）提升健壮性但非阻塞，留待后续批次。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| `entity.read(cx)` 借用与 slot 内容表达式借用冲突 | `__rml_self_ref` 在闭包开头声明，后续 `__rml_self_ref.xxx` 是字段访问，不引入新借用 |
| TabBar `on_click` 事件签名不匹配 | 参照 [tab_bar/setters.rs:159-164](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs)，签名 `fn(&mut self, idx: usize, &mut Context<Self>)`，CaseDocPage 的 `on_code_tab_change` 已对齐 |
| CaseDocPage.rml 中样式属性可能不被 RML 解析器支持 | 用 RML 已支持的属性语法（`text-size` / `font-weight` / `border-1` 等），不引入新属性 |
| table_case.rml.rs 的 `case_doc_page` 字段未自动初始化 | 检查 RML 框架是否自动初始化用户组件字段；若否，在 `on_loaded` 中手动初始化 |

## 验证清单

- [ ] 步骤 5：`cargo check -p rust-rml-engine` 通过
- [ ] 步骤 6：`cargo test -p rust-rml-engine --lib user_component` 全部通过 + `cargo test -p rust-rml-engine` 全量无回归
- [ ] 步骤 7：`cargo check -p rust-rml-demo` 通过
- [ ] 步骤 8：`cargo run -p rust-rml-demo` 运行验证 table case 渲染正确

## 实施顺序

严格按步骤 5 → 6 → 7 → 8 顺序执行。每个步骤完成后立即验证，不批量推进。
