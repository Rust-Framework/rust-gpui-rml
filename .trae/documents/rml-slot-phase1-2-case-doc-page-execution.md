# RML Slot 工业级支持 —— Phase 1 + Phase 2 + CaseDocPage 改造执行计划

## 背景

用户反馈"case_doc_page模板如果真的需要，也应该按照.rml规范编写才对"，要求放弃 Rust builder 方案，改为以标准 RML 组件实现 CaseDocPage 共享模板。

调研发现 RML 框架两个核心限制阻止了标准实现：
1. **用户组件不支持属性传参**：[user_component.rs:30-95](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 的 `gen_user_component` 完全忽略 `elem.attributes`
2. **slot 闭包不捕获父视图 self**：[slot.rs:16-17](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/slot.rs) 明确限制"slot 内容表达式不应直接引用父视图的 self 字段"

完整 4 阶段迭代计划见 [rml-slot-industrial-grade-support.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-slot-industrial-grade-support.md)。本计划聚焦**第一批执行**：Phase 1 + Phase 2 + CaseDocPage 改造验证。

## 当前进度

| 任务 | 状态 | 说明 |
|------|------|------|
| Phase 1.1: UserComponentInfo 扩展 field_types + computed_methods | ✅ 已完成 | [compiler/mod.rs:88-115](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) |
| Phase 1.2: build.rs 填充新字段 | ✅ 已完成 | [build/mod.rs:190-205](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs) |
| Phase 1.3: gen_user_component 处理属性传参 | ⏳ 待实施 | 当前 [user_component.rs:43-45](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 提前返回导致属性被丢弃 |
| Phase 1.4: Phase 1 单元测试 | ⏳ 待实施 | 仿 [component.rs:758-921](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 测试模式 |
| Phase 2.1-2.4: self_alias 机制 | ⏳ 待实施 | 让 slot 闭包通过 Entity<Self> 捕获父视图数据 |
| CaseDocPage 改造 | ⏳ 待实施 | 新建 .rml + .rml.rs，删除 .rs，改造 table_case 验证 |

## 实施步骤

### 步骤 1：Phase 1.3 —— gen_user_component 属性传参

**目标**：让 `<CaseDocPage title={t("case.table.title")} description="..." rml-sample={rml_sample}>` 中的属性注入到子组件 entity。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

**新增函数 `gen_prop_assign`**：

签名：
```rust
fn gen_prop_assign(
    info: &UserComponentInfo,
    attr: &Attribute,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Result<Option<String>, CodegenError>
```

逻辑：
1. 跳过非组件属性：`ref` / `class` / `id` / `style` / `slot`（这些不在 `info.field_types` 中，由其他路径处理）→ 返回 `Ok(None)`
2. 跳过事件属性 `Attribute::Event`（Phase 1 不处理用户组件事件）→ 返回 `Ok(None)`
3. 静态属性 `Attribute::Static { name, value, .. }`：
   - 查 `info.field_types[name]`，未命中 → 返回 `Ok(None)`（避免对未知属性报错，留待 Phase 4 校验）
   - 命中类型：
     - `String` / `SharedString` → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = {}.into(); }})", name, quote(value))`
     - `i32` / `u32` / `usize` / `i64` / `u64` / `f64` → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = \"{}\".parse().unwrap_or(0); }})", name, value)`
     - `bool` → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = {}; }})", name, parse_bool(value))`
     - 其他类型 → 返回 `Ok(None)`（不处理，避免误生成代码）
4. 绑定属性 `Attribute::Bind { name, expr, .. }`：
   - 查 `info.field_types[name]`，未命中 → 返回 `Ok(None)`
   - 用 `component_bind_rust_expr(expr, &loop_vars_slice, &computed_slice)` 解析表达式（自动处理 computed 方法 vs 字段）
   - 类型转换：
     - `String` / `SharedString` → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = ({}).into(); }})", name, rust_expr)`（绑定值类型可能不匹配，用 into()）
     - 数字类型 → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = {}; }})", name, rust_expr)`（直接赋值，依赖 Rust 类型推断）
     - `Vec<_>` → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = ({}).clone(); }})", name, rust_expr)`
     - 其他 → `format!("__rml_entity.update(cx, |this, _cx| {{ this.{} = ({}).clone(); }})", name, rust_expr)`（默认 clone）

**重构 `gen_user_component`**：

```rust
pub fn gen_user_component(
    info: &UserComponentInfo,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let entity_expr = format!(
        "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
        info.entity_field, info.struct_name
    );

    // 生成属性赋值代码
    let mut prop_assigns: Vec<String> = Vec::new();
    for attr in &elem.attributes {
        if let Some(code) = gen_prop_assign(info, attr, ctx, loop_vars)? {
            prop_assigns.push(code);
        }
    }

    // 分离 slot 子节点
    let (slot_children, default_children) = partition_user_component_children(elem);

    // 无属性赋值且无 slot 内容：直接 clone entity（保持原行为）
    if prop_assigns.is_empty() && slot_children.is_empty() && default_children.is_empty() {
        return Ok(entity_expr);
    }

    let mut code = String::new();
    code.push_str("{\n");
    code.push_str(&format!("    let __rml_entity = {};\n", entity_expr));

    // 属性注入（在 slot 处理前）
    for assign in &prop_assigns {
        code.push_str(&format!("    {}\n", assign));
    }

    // slot 处理（保持现有逻辑不变）
    // ... 现有 slot 闭包生成代码 ...

    code.push_str("    __rml_entity\n");
    code.push('}');
    Ok(code)
}
```

**辅助函数**：复用 [component.rs:436](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 的 `parse_bool`（需改为 pub(crate) 或在 user_component.rs 内联实现）。

**验证**：手工编写 .rml 测试用例（在 user_component.rs 测试模块中），验证生成的代码字符串包含正确的 `__rml_entity.update(cx, |this, _cx| { this.title = ... })` 调用。

---

### 步骤 2：Phase 1.4 —— Phase 1 单元测试

**目标**：覆盖静态/绑定/computed 三类属性 + 类型转换。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 末尾新增 `#[cfg(test)] mod tests` 模块。

**测试用例**：
1. `test_static_string_prop` —— `<MyComp title="hello">` → 生成 `this.title = "hello".into();`
2. `test_static_numeric_prop` —— `<MyComp count="42">` → 生成 `this.count = "42".parse().unwrap_or(0);`
3. `test_static_bool_prop` —— `<MyComp disabled="">` → 生成 `this.disabled = true;`
4. `test_bind_field_prop` —— `<MyComp title={title}>` → 生成 `this.title = (self.title).into();`
5. `test_bind_computed_prop` —— `<MyComp sample={sample}>`（sample 在 ctx.computed_methods 中）→ 生成 `this.sample = (self.sample()).into();`
6. `test_skip_non_prop_attributes` —— `<MyComp class="foo" ref="bar">` → 不生成任何属性赋值
7. `test_skip_event_attributes` —— `<MyComp onclick={handler}>` → 不生成属性赋值
8. `test_mixed_props_and_slots` —— `<MyComp title="x"><template slot="demo">...</template></MyComp>` → 同时生成属性赋值和 slot 闭包
9. `test_no_props_no_slots` —— `<MyComp>` → 直接返回 entity_expr（不进入 block）

**测试模式**：仿 [component.rs:758-921](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 测试，构造 `UserComponentInfo` + `Element` + `CodegenCtx`，调用 `gen_user_component`，断言输出字符串。

**验证**：`cargo test -p rust-rml-engine --lib user_component`。

---

### 步骤 3：Phase 2.1 —— CodegenCtx 添加 self_alias 字段

**目标**：为 slot 闭包内引用父视图数据提供机制。

**修改文件**：[crates/engine/src/compiler/mod.rs:118-208](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)

在 `CodegenCtx` 结构体中新增字段：
```rust
pub struct CodegenCtx {
    // ... 现有字段 ...
    /// slot 闭包内引用父视图数据时的 self 别名
    ///
    /// 由 `gen_user_component` 在生成 slot 内容前 clone ctx 并设置为
    /// `Some("__rml_self_ref".to_string())`。`to_rust_code_with_ctx` 等函数
    /// 据此把 `self.xxx` 替换为 `__rml_self_ref.xxx`，绕过 slot 闭包的生命周期限制。
    pub self_alias: Option<String>,
}
```

由于 `CodegenCtx` 已 derive `Default`，`Option<String>` 默认为 `None`，无需修改 `Default` 实现。

**验证**：`cargo check -p rust-rml-engine` 编译通过（无逻辑变化，仅新增字段）。

---

### 步骤 4：Phase 2.2 —— 表达式生成支持 self_alias

**目标**：让 `{items}` / `{api_columns}` 等插值在 slot 闭包内生成 `__rml_self_ref.items` 而非 `self.items`。

**修改文件 1**：[crates/engine/src/compiler/expr.rs:153](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs)

修改 `to_rust_code_with_ctx` 签名，新增 `self_alias: Option<&str>` 参数：
```rust
pub fn to_rust_code_with_ctx(expr: &Expr, loop_vars: &[&str], self_alias: Option<&str>) -> String {
    match expr {
        Expr::Field(name) => {
            if name == "self" {
                // 若有 self_alias，slot 闭包内用别名引用父视图
                return self_alias.unwrap_or("self").to_string();
            } else if loop_vars.iter().any(|v| *v == name) {
                return name.clone();
            } else if let Some(alias) = self_alias {
                return format!("{}.{}", alias, name);
            }
            format!("self.{}", name)
        }
        Expr::Member(target, name) => {
            format!("{}.{}", to_rust_code_with_ctx(target, loop_vars, self_alias), name)
        }
        // ... 所有递归调用都透传 self_alias ...
    }
}
```

**修改文件 2**：[crates/engine/src/compiler/codegen/text.rs:33](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/text.rs)

修改 `gen_expr_code` 签名，新增 `self_alias: Option<&str>`：
```rust
pub(crate) fn gen_expr_code(
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    self_alias: Option<&str>,
) -> String {
    // ... 透传 self_alias 到 to_rust_code_with_ctx ...
}
```

**修改文件 3**：[crates/engine/src/compiler/component.rs:444](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

修改 `component_bind_rust_expr` 和 `component_bind_setter` 签名，新增 `self_alias: Option<&str>`：
```rust
pub fn component_bind_rust_expr(
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    self_alias: Option<&str>,
) -> String { ... }

pub fn component_bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
    self_alias: Option<&str>,
) -> Option<String> { ... }
```

**调用点更新**（所有现有调用点传 `None`，行为不变）：
- [codegen/node.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 中调用 `gen_expr_code` / `to_rust_code_with_ctx` 的位置
- [codegen/attribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs) 中调用 `component_bind_setter` 的位置
- 各组件专属 setter 模块（[tab_bar/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs)、[card/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/card/setters.rs) 等）中调用 `component_bind_rust_expr` 的位置

**验证**：`cargo check -p rust-rml-engine` 编译通过；现有测试 `cargo test -p rust-rml-engine` 全部通过（self_alias=None 行为不变）。

---

### 步骤 5：Phase 2.3 —— gen_user_component 生成 self 捕获 + slot 闭包改造

**目标**：让 slot 闭包通过 `Entity<Self>` 捕获父视图数据。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

在 `gen_user_component` 中，当存在 slot 子节点时：

1. **在 `code.push_str("{\n")` 后生成**：
   ```rust
   code.push_str("    let __rml_self_entity = cx.entity();\n");
   ```

2. **slot 闭包改造**：
   ```rust
   // 当前（不捕获 self）：
   let __rml_slot_demo_value: SlotRenderer = Box::new(move |_window, cx| {
       (self.items.iter().map(...).collect()).into_any_element()
   });
   
   // 改造后（通过 Entity 捕获）：
   let __rml_slot_demo_value: SlotRenderer = Box::new(move |_window, cx| {
       let __rml_self_ref = __rml_self_entity.read(cx);
       (__rml_self_ref.items.iter().map(...).collect()).into_any_element()
   });
   ```

3. **生成 slot 内容时设置 self_alias**：
   ```rust
   let mut slot_ctx = ctx.clone();
   slot_ctx.self_alias = Some("__rml_self_ref".to_string());
   let slot_code = gen_slot_content(slot_nodes, &slot_ctx, id_counter, loop_vars)?;
   ```

4. **闭包内先声明 `__rml_self_ref`**：
   ```rust
   code.push_str(&format!(
       "    let {}: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement {{ let __rml_self_ref = __rml_self_entity.read(cx); ({}).into_any_element() }});\n",
       binding, slot_code
   ));
   ```

**注意**：
- `cx.entity()` 在 `render(&mut self, _window, cx)` 方法中可用，返回 `Entity<Self>`
- `Entity<Self>: Send + Sync + 'static`（[state.rs:85-89](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs) 已确认），可被 slot 闭包 `move` 捕获
- `entity.read(cx)` 返回 `&Self`，足够调用 computed 方法（`&self`）和访问字段

**验证**：在 user_component.rs 测试模块新增测试，验证 slot 闭包内引用 `self.items` 生成 `__rml_self_ref.items`。

---

### 步骤 6：Phase 2.4 —— Phase 2 单元测试 + 编译验证

**目标**：验证 self_alias 机制正确工作。

**修改文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 测试模块新增用例。

**测试用例**：
1. `test_slot_closure_captures_self` —— slot 内引用 `self.items` → 生成 `let __rml_self_entity = cx.entity();` + 闭包内 `let __rml_self_ref = __rml_self_entity.read(cx);` + `__rml_self_ref.items`
2. `test_slot_closure_computed_method` —— slot 内调用 `self.format_items()`（computed）→ 生成 `__rml_self_ref.format_items()`
3. `test_slot_closure_mixed_self_and_loop_var` —— slot 内 `each={item in items}` 引用 `self.items`（迭代源）+ `item`（迭代变量）→ `__rml_self_ref.items` + `item`

**验证**：`cargo test -p rust-rml-engine --lib user_component` 全部通过；`cargo check -p rust-rml-engine` 编译通过。

---

### 步骤 7：CaseDocPage 改造 —— 新建 .rml + .rml.rs

**目标**：用标准 RML 组件替换 Rust builder 实现。

**新建文件 1**：`demo/src/cases/common/case_doc_page.rml`

slot 渲染使用 Vue 风格的 `<slot name="xxx" />` 语法（[codegen/node.rs:241-261](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 已支持），codegen 生成 `self.__rml_state.slot("xxx").map_or(gpui::Empty.into_any_element(), |f| f(_window, cx))`。

```rml
<component>
  <div class="case-doc-page">
    <!-- 标题区 -->
    <div class="doc-header">
      <h2 class="doc-title">{title}</h2>
      {description}
    </div>
    
    <!-- 演示区 + 代码区 -->
    <div class="case-layout">
      <div class="case-demo-panel">
        <slot name="demo" />
      </div>
      <div class="case-code-panel">
        <TabBar selected_index={code_tab} on_click={on_code_tab_change}>
          <tab label="RML" />
          <tab label="Rust" />
        </TabBar>
        <div class="code-block">
          {current_code}
        </div>
      </div>
    </div>
    
    <!-- API 区 -->
    <div class="case-api-panel">
      <slot name="api" />
    </div>
  </div>
</component>
```

**注意**：`{current_code}` 为 computed 方法（返回当前 Tab 对应的代码字符串），避免在 RML 中写 `if` 表达式（RML 的 `if` 是指令 `if={cond}` 用于条件渲染，不支持表达式内 if）。在 case_doc_page.rml.rs 中实现：
```rust
#[computed]
fn current_code(&self) -> String {
    if self.code_tab == 0 { self.code_rml.clone() } else { self.code_rust.clone() }
}
```

**新建文件 2**：`demo/src/cases/common/case_doc_page.rml.rs`

```rust
use gpui::SharedString;
use rml_core::i18n::t_static;
use rml_macros::component;

/// 案例页共享模板组件
#[component(slots = ["demo", "api"])]
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
    /// 切换代码 Tab
    #[command]
    fn on_code_tab_change(&mut self, index: usize, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.code_tab = index;
        cx.notify();
    }
}
```

**删除文件**：`demo/src/cases/common/case_doc_page.rs`

**父视图注册**：在使用 CaseDocPage 的父视图（如 `table_case.rml.rs`）中，添加 `case_doc_page: Option<Entity<CaseDocPage>>` 字段，并在 `on_loaded` 中初始化。

**验证**：`cargo check -p rust-rml-demo` 编译通过。

---

### 步骤 8：CaseDocPage 改造 —— 改造 table_case 验证

**目标**：将一个案例改造为使用 `<CaseDocPage>` + slot 的形式，端到端验证框架增强。

**修改文件**：`demo/src/cases/table_case.rml` + `table_case.rml.rs`

改造前（builder 模式）：
```rust
// table_case.rml.rs
fn render_doc_page(&mut self, window, cx) -> AnyElement {
    CaseDocPage::new(Self::CONTRIBUTION_ID)
        .title(t("case.table.title"))
        .description(t("case.table.description"))
        .demo(self.render_demo(window, cx))
        .code_rml(include_str!("table_case.rml"))
        .code_rust(include_str!("table_case.rml.rs"))
        .api(self.render_api(window, cx))
        .render(window, cx)
}
```

```rml
<!-- table_case.rml -->
<component content={self.render_doc_page(_window, cx)} />
```

改造后（标准 RML 组件）：
```rml
<!-- table_case.rml -->
<component>
  <CaseDocPage 
    title={t("case.table.title")} 
    description={t("case.table.description")}
    code_rml={code_rml_content}
    code_rust={code_rust_content}>
    <template slot="demo">
      <!-- 原 render_demo 的内容，可引用 self.items / self.api_columns -->
      <Table columns={api_columns} rows={items} bordered />
    </template>
    <template slot="api">
      <!-- 原 render_api 的内容 -->
      <DescriptionList items={api_description} bordered />
    </template>
  </CaseDocPage>
</component>
```

**table_case.rml.rs 新增 computed 方法**（包装 `include_str!`，避免 RML parser 解析宏调用）：
```rust
impl TableCase {
    #[computed]
    fn code_rml_content(&self) -> String {
        include_str!("table_case.rml").to_string()
    }
    
    #[computed]
    fn code_rust_content(&self) -> String {
        include_str!("table_case.rml.rs").to_string()
    }
}
```

**注意**：
- `code_rml_content` / `code_rust_content` 用 computed 方法包装 `include_str!`（RML parser 不支持宏调用，必须用 computed 方法桥接）
- `code_tab` 由 CaseDocPage 内部管理（决策 3），父视图无需传递
- slot 内引用 `self.items` / `self.api_columns` 由 Phase 2 self_alias 机制支持

**验证**：
1. `cargo check -p rust-rml-demo` 编译通过
2. `cargo run -p rust-rml-demo` 运行，导航到 table case，验证：
   - 标题 + 描述正确显示
   - 演示区 Table 渲染正常
   - 代码区 Tab 切换正常（RML / Rust）
   - API 区 DescriptionList 渲染正常
   - 视觉效果与原 builder 模式一致

---

## 关键设计决策

### 决策 1：属性类型转换策略

- **静态属性**：必须做类型转换（字符串字面量 → 字段类型）
- **绑定属性**：尽量直接赋值，依赖 Rust 类型推断；对 `String`/`SharedString` 用 `.into()`，对 `Vec<_>` 用 `.clone()`
- **未识别类型**：返回 `Ok(None)` 跳过，不报错（Phase 4 补全编译期校验）

### 决策 2：self_alias 通过参数传递而非 ctx 字段

虽然步骤 3 在 CodegenCtx 中添加了 `self_alias` 字段，但步骤 4 仍通过函数参数传递。原因：
- `to_rust_code_with_ctx` / `gen_expr_code` 是 pub/pub(crate) 函数，签名修改影响外部调用者
- 通过参数传递更显式，符合现有 `loop_vars` / `computed` 的传递模式
- ctx 中的 `self_alias` 字段供 `gen_user_component` 设置，再由调用方提取传参

**实际实现**：`gen_user_component` clone ctx 并设置 `self_alias`，调用 `gen_slot_content` 时从 ctx 读取并作为参数传递给 `gen_expr_code`。

### 决策 3：CaseDocPage 的 code_tab 状态管理

两种方案：
- **A**：code_tab 在 CaseDocPage 内部（组件自管理），父视图通过 `code_tab={...}` 绑定（受控）
- **B**：code_tab 在父视图，CaseDocPage 仅展示

选择 **A**：CaseDocPage 持有 code_tab 字段 + `on_code_tab_change` 命令，父视图无需管理。但为支持外部读取/控制，`code_tab` 字段保持 pub。

### 决策 4：不实施 Phase 3（scoped slot）和 Phase 4（编译期校验）

CaseDocPage 不需要 scoped slot（slot 内仅引用父视图数据，不需要子组件回传）。编译期校验（Phase 4）提升健壮性但非阻塞，留待后续批次。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| self_alias 修改影响 8-10 个调用点 | 所有调用点默认传 `None`，行为不变；用 `cargo check` 全量验证 |
| RML parser 不支持宏调用（如 `include_str!`） | 用 computed 方法包装：`code_rml_content()` 返回 `include_str!` 结果（步骤 8 已采用） |
| RML 表达式不支持内联 `if` | 用 computed 方法 `current_code()` 返回当前 Tab 代码（步骤 7 已采用） |
| slot 闭包内 `entity.read(cx)` 借用与 `gen_slot_content` 生成的代码借用冲突 | `__rml_self_ref` 在闭包开头声明，后续表达式中 `__rml_self_ref.xxx` 是字段访问，不引入新借用 |
| CaseDocPage.rml 中 `<TabBar on_click={on_code_tab_change}>` 的事件签名可能不匹配 | `on_code_tab_change` 需符合 TabBar 的 `on_click` 事件签名（接收索引参数）；若不匹配，改用 `on_click={on_tab_click}` 中转 |

## 验证清单

- [ ] 步骤 1：`cargo check -p rust-rml-engine` 通过
- [ ] 步骤 2：`cargo test -p rust-rml-engine --lib user_component` 全部通过
- [ ] 步骤 3：`cargo check -p rust-rml-engine` 通过
- [ ] 步骤 4：`cargo check -p rust-rml-engine` + `cargo test -p rust-rml-engine` 全部通过
- [ ] 步骤 5：`cargo check -p rust-rml-engine` 通过
- [ ] 步骤 6：`cargo test -p rust-rml-engine --lib user_component` 全部通过
- [ ] 步骤 7：`cargo check -p rust-rml-demo` 通过
- [ ] 步骤 8：`cargo run -p rust-rml-demo` 运行验证 table case 渲染正确

## 实施顺序

严格按步骤 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 顺序执行。每个步骤完成后立即验证，不批量推进。

Phase 1（步骤 1-2）和 Phase 2（步骤 3-6）之间存在依赖：Phase 2 的 self_alias 改造影响 `gen_user_component` 中 slot 闭包生成代码，需在 Phase 1 属性传参完成后再实施。
