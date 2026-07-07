# RML Slot 工业级支持迭代计划

## Context（背景）

当前 RML 框架的 slot 机制存在两个核心限制，导致无法实现共享模板组件（如 CaseDocPage）：

1. **用户组件不支持属性传参**：`gen_user_component`（[crates/engine/src/compiler/user_component.rs:30-95](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)）完全忽略 `elem.attributes`，导致 `<CaseDocPage title={...} description={...}>` 中的属性被丢弃。

2. **slot 闭包不捕获父视图 self**：user_component.rs:65 注释明确说"闭包不捕获父视图的 self（生命周期不允许）"，导致 `<template slot="demo">` 内无法引用 `self.items`、`self.api_columns` 等父视图字段。

**动因**：在 rml-demo-case-pages-ui-iteration 计划中尝试用 Rust builder 实现 CaseDocPage 共享模板，但用户反馈"应该按 .rml 规范编写"。调研确认 RML 框架限制阻止了标准 RML 组件实现，因此需要完善框架本身的 slot 支持达到工业级水准。

**技术可行性已确认**：
- `Entity<Self>` 是 `Send + Sync + 'static`（[crates/ui/src/state.rs:85-89](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs)），不依赖 T 的 Send/Sync
- `cx.entity()` 可在 render 方法中获取 `Entity<Self>`（已在 RML 代码中广泛使用）
- slot 闭包可 `move` 捕获 `Entity<Self>`，闭包内通过 `entity.read(cx)` 获取 `&Self`

## 目标能力

| 能力 | 当前 | 目标 |
|------|------|------|
| 用户组件属性传参 | ❌ 完全忽略 | ✅ 支持静态/绑定/computed 三类属性 |
| slot 引用父视图字段 | ❌ 生命周期限制 | ✅ 通过 Entity 捕获绕过 |
| scoped slot（子→父回传） | ❌ 不支持 | ✅ slot-props 语法 |
| 编译期校验 | ⚠️ 部分 | ✅ slot 名 + 属性名校验 |

## 分阶段实现

### Phase 1: 用户组件属性传参（Props）

**目标**：让 `<CaseDocPage title={t("case.overflow.title")} description="..." rml-sample={rml_sample}>` 生效。

**修改文件**：
- [crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)：`UserComponentInfo` 扩展 `field_types: HashMap<String, String>` 和 `computed_methods: Vec<String>`
- [crates/engine/src/build/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs):192-200：构造 `UserComponentInfo` 时从 `StructMetadata` 拷贝 field_types 和 computed_methods
- [crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)：`gen_user_component` 在 slot 处理前遍历 `elem.attributes`，生成属性注入代码

**新增函数**：`gen_prop_assign`（user_component.rs）
- 根据 `info.field_types[field]` 生成类型转换：
  - `String`/`SharedString` → `.into()`（静态）或 `.clone()`（绑定）
  - `i32`/`u32`/`usize`/`f64` → `parse().unwrap_or(0)`（静态）或原样（绑定）
  - `bool` → `as bool` / 直接表达式
  - `Vec<_>` → `.clone()`
- 绑定属性中，若 `name` 在 `info.computed_methods` 中，生成 `self.name().clone()`；否则 `self.name.clone()`

**生成代码示例**：
```rust
// Before：属性被丢弃
{
    let __rml_entity = self.case_doc_page.as_ref().expect("...").clone();
    __rml_entity
}

// After：属性注入
{
    let __rml_entity = self.case_doc_page.as_ref().expect("...").clone();
    __rml_entity.update(cx, |this, _cx| { this.title = t("case.table.title").into(); });
    __rml_entity.update(cx, |this, _cx| { this.description = "...".into(); });
    __rml_entity.update(cx, |this, _cx| { this.rml_sample = self.rml_sample().clone(); });
    // slot 处理...
    __rml_entity
}
```

**验证**：单元测试覆盖静态/绑定/computed 三类属性 + 类型转换（仿 [component.rs:758-921](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 测试模式）。

---

### Phase 2: Slot 闭包捕获父视图数据

**目标**：让 `<template slot="demo">` 内能引用 `self.items`、`self.api_columns` 等父视图字段。

**修改文件**：
- [crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)：`CodegenCtx` 新增 `self_alias: Option<String>` 字段
- [crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)：
  - `gen_user_component` 在 `code.push_str("{\n")` 后生成 `let __rml_self_entity = cx.entity();`
  - slot 闭包改为捕获 `__rml_self_entity`，闭包内 `let __rml_self_ref = __rml_self_entity.read(cx);`
  - 生成 slot 内容前 clone ctx 并设置 `self_alias = Some("__rml_self_ref".to_string())`
- [crates/engine/src/compiler/expr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs)：`to_rust_code_with_ctx` 新增 `self_alias: Option<&str>` 参数，`Expr::Field(name)` 分支中 self_alias 优先于 loop_vars
- [crates/engine/src/compiler/codegen/text.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/text.rs)：`gen_expr_code` 新增 `self_alias: Option<&str>` 参数，透传到 `to_rust_code_with_ctx`
- [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs):485：`component_bind_setter` 新增 `self_alias: Option<&str>` 参数
- 所有调用点：默认传 `None`，行为不变

**生成代码示例**：
```rust
// Before：slot 闭包不捕获 self，引用 self 编译失败
let __rml_slot_demo_value = Box::new(move |_window, cx| {
    (self.items.iter().map(...).collect())  // ERROR
});

// After：通过 Entity 捕获绕过生命周期限制
let __rml_self_entity = cx.entity();
let __rml_slot_demo_value: SlotRenderer = Box::new(move |_window, cx| {
    let __rml_self_ref = __rml_self_entity.read(cx);
    (__rml_self_ref.items.iter().map(...).collect()).into_any_element()
});
```

**风险与缓解**：
- self_alias 机制侵入 `gen_expr_code`、`to_rust_code_with_ctx`、`component_bind_setter` 三个函数签名，约 8-10 个调用点
- 用 `Option<&str>` 默认 `None`，所有现有调用点传 None 行为不变
- computed 方法用 `&self`（ComputedCache 内部互斥），`entity.read(cx)` 返回的 `&Self` 足够

**验证**：
- expr.rs 新增 self_alias 单元测试：`Field("items")` + `Some("__rml_self_ref")` → `__rml_self_ref.items`
- 集成测试：slot 内引用父视图字段 + computed 方法

---

### Phase 3: Scoped Slot（slot 回传数据）

**目标**：让子组件通过 slot 向父组件回传数据（如列表项渲染）。

**修改文件**：
- [crates/core/src/slot.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/slot.rs)：新增 `SlotProps` 类型（`HashMap<String, Box<dyn Any + Send + Sync>>`），`SlotRenderer` 签名扩展为 `Fn(&mut Window, &mut App, &SlotProps) -> AnyElement`，旧闭包通过适配层包装
- [crates/engine/src/parser/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser)：解析 `slot-props="{item, index}"` 语法
- [crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)：slot 闭包签名加 `__rml_slot_props: &SlotProps` 参数，闭包内解构

**兼容性**：用适配层包装旧闭包（无 slot-props 时传空 SlotProps），保证向后兼容。

**验证**：scoped slot 端到端测试（子组件传列表项 → 父组件 slot 渲染）。

---

### Phase 4: 编译期校验与默认 slot

**目标**：强化开发体验，编译期捕获错误。

**修改文件**：
- [crates/engine/src/compiler/validator.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs)：
  - 校验 `<template slot="x">` 的 `x` 在目标组件 `UserComponentInfo.slots` 中
  - 校验用户组件属性名在 `field_types` 中（未命中发 error）
- [crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)：完善 default slot 机制（`partition_user_component_children` 已支持，需确保 `info.slots` 含 `"default"` 时才收集）

**验证**：validator 单元测试覆盖非法 slot 名 + 非法属性名。

## Phase 间依赖

```
Phase 1 (属性传参) ──┐
                     ├──→ Phase 4 (校验)
Phase 2 (self 捕获) ─┘
                      Phase 3 (scoped slot) ──→ Phase 4
```

- Phase 1、2 都改 `gen_user_component`，建议串行（先 1 后 2）
- Phase 3 依赖 Phase 2（闭包捕获机制）+ Phase 1（属性传参概念）
- Phase 4 依赖 Phase 1-3 全部完成

## CaseDocPage 最小可用集

**Phase 1 + Phase 2** 即可让 CaseDocPage 以标准 RML 组件实现。

CaseDocPage 设计：
- `case_doc_page.rml`：四段式布局模板（标题区 + 演示区 + 代码区 + API 区），含 `<template slot="demo" />`、`<template slot="api" />`
- `case_doc_page.rml.rs`：`#[component(slots = ["demo", "api"])]`，pub 字段 `title/description/rml_sample/rust_sample/code_tab`，`#[command] on_code_tab_change`
- 案例使用：`<CaseDocPage title={...} description={...} rml-sample={rml_sample} rust-sample={rust_sample}><template slot="demo">...</template><template slot="api">...</template></CaseDocPage>`

Phase 3（scoped slot）对 CaseDocPage 非必需。Phase 4 提升健壮性但非阻塞。

## 端到端验证

1. **Phase 1+2 完成后**：
   - 新建 `demo/src/cases/common/case_doc_page.rml` + `case_doc_page.rml.rs` 作为标准 RML 组件
   - 删除 `case_doc_page.rs`（Rust builder）
   - 改造 1 个案例（如 `table_case.rml`）使用 `<CaseDocPage>` + slot
   - `cargo check -p rust-rml-demo` 编译通过
   - 运行 demo 验证渲染结果与原 builder 模式一致

2. **Phase 3 完成后**：
   - 新增 scoped slot 演示案例（如自定义列表项渲染）

3. **Phase 4 完成后**：
   - 编译期捕获非法 slot 名 + 非法属性名

## 实现节奏建议

1. **第一批**：Phase 1 + Phase 2 + CaseDocPage 改造验证（核心价值）
2. **第二批**：Phase 3（高级特性）
3. **第三批**：Phase 4（质量保证）
