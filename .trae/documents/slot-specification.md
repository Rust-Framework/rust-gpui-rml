# Slot 规范化实施计划

## 概述

规范化 RML 框架的 slot（插槽）规范，回答三个核心问题：
1. 自定义控件如何预留插槽位 + 标准语法
2. 使用方如何扩展插槽 + 标准语法（层次分明、简洁清晰）
3. 确保 RML 框架 codegen 属性映射齐全

采用 **Vue 风格插槽**（`<slot name="...">` 定义 + `<template slot="...">` 填充）+ **宏参数显式声明**（`#[component(slots = [...])]`）+ **Slot 字段注入**分发机制。

---

## 当前状态分析

### 已实现（上一轮迭代）

| 能力 | 状态 | 位置 |
|------|------|------|
| Shell 级 `<template slot="name">` | ✅ 已实现 | `shell.rs::partition_slot_children` |
| 6 个 shell 插槽 (menu/title/footer/left/right/bottom) | ✅ 已实现 | `shell.rs::gen_tab_window_wrapper` |
| AST `Element.slot_name` 字段 | ✅ 已实现 | `parser/ast.rs` + `parser/mod.rs` |
| `IComponent::slots()` trait 方法 | ✅ 已实现 | `core/src/component.rs` |
| `#[component(slots = [...])]` 宏参数解析 | ✅ 已实现 | `macros/src/component.rs` |
| `props_registry.rs` 单一信源 | ✅ 已实现 | `compiler/props_registry.rs` |
| `component_bind_setter` warning 机制 | ✅ 已实现 | `compiler/component.rs` |

### 缺失（本计划填补）

| 缺失 | 影响 |
|------|------|
| scanner.rs 不捕获 `#[component(slots)]` 参数 | codegen 无法知道自定义组件声明的插槽 |
| `UserComponentInfo` 无 slots 字段 | 父视图 codegen 无法校验 slot 名合法性 |
| 组件模板 `<slot>` 占位符不支持 | 自定义组件无法声明插槽位置 |
| `gen_user_component` 忽略所有子节点 | 父视图的 `<template slot="...">` 内容无法注入自定义组件 |
| validator 不校验 slot 名 | 未知 slot 名静默落入 body，不报错 |
| validator 不校验未知属性 | 用户拼写错误静默丢弃 |
| `props_for()` 丢弃专用属性（bug） | 查询函数返回不完整 |
| shell 属性无 warning 机制 | shell 属性映射缺失静默丢弃 |
| `component_static_setter` 无 warning | 静态属性映射缺失静默丢弃 |

---

## 规范定义（最终标准语法）

### 1. 组件预留插槽（组件开发者）

**Rust 侧声明**（`#[component]` 宏参数）：
```rust
#[component(slots = ["header", "footer", "default"])]
pub struct Card {
    title: String,
    // ...
}
```
- `slots` 为字符串数组字面量
- 保留名 `"default"` 对应模板内无 `name` 属性的 `<slot />`
- 不写 `slots` 参数 → 组件不接受任何插槽

**RML 模板侧声明**（`<slot>` 占位符）：
```html
<!-- components/card.rml -->
<component>
    <div class="card">
        <div class="card-header">
            <slot name="header" />
        </div>
        <div class="card-body">
            <slot />              <!-- default 插槽 -->
        </div>
        <div class="card-footer">
            <slot name="footer" />
        </div>
    </div>
</component>
```
- `<slot name="header" />` 声明具名插槽位置
- `<slot />`（无 name）声明默认插槽位置
- codegen 将 `<slot>` 替换为 `self.__rml_slot_<name>.take()` 渲染

### 2. 使用方扩展插槽（组件使用者）

```html
<!-- 父视图 .rml -->
<Card title="My Card">
    <template slot="header">
        <h2>Card Title</h2>
        <Button label="Close" ghost="" />
    </template>

    <template slot="footer">
        <Button label="OK" primary="" />
    </template>

    <!-- 默认插槽：无 slot 属性的子节点 -->
    <p>This is the card body content.</p>
</Card>
```
- `<template slot="name">` 填充具名插槽
- 无 `slot` 属性的子节点填充 `default` 插槽（仅当组件声明了 `"default"` 时）
- 未填充的插槽渲染为空

### 3. Shell 窗口插槽（已实现，保持不变）

```html
<tab_window title="App" ...>
    <template slot="left">...</template>
    <template slot="menu">...</template>
    <template slot="footer">...</template>
    <!-- 主内容 -->
</tab_window>
```

---

## 实施变更

### Step 1: scanner 捕获 slots 声明

**文件**: `crates/engine/src/build/scanner.rs`

**变更**:
1. `StructMetadata` 新增字段：
   ```rust
   pub slots: Vec<String>,
   ```
2. 在 `scan_struct_metadata` 第一遍扫描中，解析 `#[component(slots = [...])]` 属性的 TokenStream，提取字符串数组。复用 `macros/src/component.rs` 的 `ComponentArgs` 解析逻辑（或用 syn 直接解析 `Meta::List`）。

**原因**: codegen 需要知道自定义组件声明了哪些插槽，才能在父视图中校验 `<template slot="x">` 的 x 是否合法，以及为组件生成 slot 字段。

### Step 2: UserComponentInfo 携带 slots

**文件**: 
- `crates/engine/src/compiler/mod.rs`（`UserComponentInfo` 结构体）
- `crates/engine/src/build/mod.rs`（构建 user_components 注册表）

**变更**:
1. `UserComponentInfo` 新增字段：
   ```rust
   pub slots: Vec<String>,
   ```
2. `build/mod.rs` 中构建 `user_components` 时，从 `struct_metas` 的 `slots` 字段填充。

**原因**: 父视图 codegen 在 `gen_user_component` 时需要查询目标组件的 slots 列表，以分离 `<template slot="...">` 子节点。

### Step 3: `#[component]` 宏注入 slot 字段 + setter

**文件**: `crates/macros/src/component.rs`

**变更**:
1. `inject_tracking_fields` 新增逻辑：当 `slots` 非空时，为每个 slot 注入私有字段：
   ```rust
   #[allow(non_snake_case)]
   __rml_slot_<name>: Option<gpui::AnyElement>,
   ```
   注意：`default` slot 的字段名为 `__rml_slot_default`。
2. `expand_component_impls` 生成 slot setter 方法（在独立 `impl` 块中）：
   ```rust
   impl #struct_name {
       pub fn __rml_set_slot_<name>(&mut self, element: impl gpui::IntoElement) {
           self.__rml_slot_<name> = Some(element.into_any_element());
       }
   }
   ```
3. `Default` derive 兼容：`Option<AnyElement>::default() = None`，无需特殊处理。

**原因**: 组件实体持有 slot 内容，父视图通过 setter 注入，组件 render 通过 `.take()` 消费。

**限制说明**: slot 内容在组件 render 时被 `.take()` 消费。父视图每次 render 时重新注入 slot 内容，因此父视图状态变化驱动的 slot 更新生效。若组件独立 re-render（组件自身状态变化），slot 内容为空。这是 MVP 的已知限制，文档中标注。

### Step 4: codegen 支持 `<slot>` 占位符

**文件**: `crates/engine/src/compiler/codegen/mod.rs`（`gen_element` 函数）

**变更**:
1. 在 `gen_element` 中新增 `<slot>` 标签处理分支（在用户组件/扩展组件检查之前）：
   ```rust
   if tag == "slot" {
       // 从元素的 name 属性提取 slot 名（无 name 属性 = "default"）
       let slot_name = elem.attributes.iter()
           .find_map(|a| match a {
               Attribute::Static { name, value } if name == "name" => Some(value.clone()),
               _ => None,
           })
           .unwrap_or_else(|| "default".to_string());
       // 生成 .children(self.__rml_slot_<name>.take())
       return Ok((format!(".children(self.__rml_slot_{}.take())", slot_name), false));
   }
   ```
2. `<slot>` 标签不创建元素，仅作为占位符替换为字段访问。

**原因**: 组件模板中的 `<slot>` 声明 slot 内容的渲染位置，codegen 将其替换为对 slot 字段的消费。

### Step 5: `gen_user_component` 支持插槽内容注入

**文件**: `crates/engine/src/compiler/component.rs`（`gen_user_component` 函数）

**变更**:
1. 修改 `gen_user_component` 签名，接收 `elem: &Element` 和 `ctx: &CodegenCtx`（需从 `gen_component` 传递），以便处理子节点。
2. 重写生成逻辑：
   ```rust
   fn gen_user_component(
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

       // 分离 slot 子节点与 default 子节点
       let (slot_children, default_children) = partition_user_component_children(elem, &info.slots);

       // 若无任何 slot 子节点，保持原行为（直接 clone entity）
       if slot_children.is_empty() && default_children.is_empty() {
           return Ok(entity_expr);
       }

       // 生成 slot 注入代码
       let mut code = String::new();
       code.push_str("{\n");
       code.push_str(&format!("    let __rml_entity = {};\n", entity_expr));

       // 为每个 slot 生成内容 + 注入
       for (slot_name, slot_nodes) in &slot_children {
           let slot_code = gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)?;
           code.push_str(&format!(
               "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_{}({}); }});\n",
               slot_name, slot_code
           ));
       }
       // default 插槽
       if !default_children.is_empty() && info.slots.contains(&"default".to_string()) {
           let default_code = gen_slot_content(&default_children, ctx, id_counter, loop_vars)?;
           code.push_str(&format!(
               "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_default({}); }});\n",
               default_code
           ));
       }

       code.push_str("    __rml_entity\n");
       code.push_str("}");
       Ok(code)
   }
   ```

3. 新增 `partition_user_component_children` 辅助函数：
   - 遍历子节点，将 `<template slot="name">` 的内容路由到 `slot_children: HashMap<String, Vec<Node>>`
   - 其余子节点收集到 `default_children: Vec<Node>`
   - 若组件未声明 `"default"` slot，default 子节点被忽略（validator 应在编译期拦截）

4. 新增 `gen_slot_content` 辅助函数：
   - 单节点：直接生成节点代码
   - 多节点：包裹 `gpui::div().child(...).child(...)` 容器

**原因**: 这是 slot 内容分发的核心——父视图 codegen 在 clone entity 后，通过 `entity.update(cx, |this, _cx| { this.__rml_set_slot_xxx(...); })` 注入 slot 内容。

### Step 6: validator 校验 slot 名 + 未知属性

**文件**: `crates/engine/src/compiler/validator.rs`

**变更**:
1. 新增 `validate_slot_names` 函数：遍历 AST，对每个用户组件标签的 `<template slot="x">` 子节点，校验 `x` 是否在该组件的 `slots()` 声明中。需要 `CodegenCtx.user_components` 信息传入 validator。
   - validator 当前签名 `validate(node: &Node)` 需扩展为 `validate(node: &Node, user_components: &HashMap<String, UserComponentInfo>)`
   - 调用方 `compile()` 传递 `ctx.user_components`
2. 新增未知属性校验：对扩展组件（`tags::is_extension_component(tag)`）的属性，若既不在 `props_registry::is_prop_registered` 中，也不在通用属性中，报 ValidationError。
   - shell 根标签用 `is_shell_prop_registered` 校验
   - 仅校验 bind/event 属性（static 属性可能有自定义用途，宽松处理）

**原因**: 编译期拦截用户拼写错误（error）+ 框架开发者映射缺失（warning），双层保障属性齐全。

### Step 7: props_registry 修复 + 补全

**文件**: `crates/engine/src/compiler/props_registry.rs`

**变更**:
1. **修复 `props_for()` bug**：当前 line 101 `let _ = (bind_extra, event_extra);` 丢弃了专用属性。重写为正确合并通用 + 专用属性并返回。
2. **新增 shell 属性 warning**：在 `shell.rs` 的 `gen_tab_window_wrapper` / `gen_modern_window_wrapper` 的 bind 属性 match 中，未命中分支添加 warning（参考 `component_bind_setter` 的 warning 逻辑）。
3. **新增 `component_static_setter` warning**：在 `component_static_setter` 的未命中分支添加 warning，检查 `is_prop_registered`。
4. **补全 `SHELL_PROPS`**：确认 `tab_window` 的 slot 相关属性（`left_size` 等已登记），添加缺失项。

**原因**: 确保框架 codegen 翻译时属性映射齐全，单一信源 + warning 机制 + 测试验证三重保障。

### Step 8: 文档同步

**文件**:
- `docs/06-components/slots.md`（重写）
- `docs/06-components/custom-components.md`（补充 slot 章节）
- `docs/06-components/reference/props-mapping.md`（同步 registry 变更）

**变更**:
1. `slots.md`：将"规划中"标注改为"已实现"，补充自定义组件 slot 完整示例（`<slot>` 占位符 + `<template slot>` 填充 + `#[component(slots)]` 声明）。
2. `custom-components.md`：新增"组件插槽"章节。
3. `props-mapping.md`：同步 props_registry 的修复。

### Step 9: demo 验证

**文件**: `demo/src/components/` 或 `demo/src/cases/`

**变更**:
1. 新增一个带 slot 的自定义组件 demo（如 `Card` 组件），验证：
   - `#[component(slots = ["header", "default", "footer"])]` 声明
   - `.rml` 模板中 `<slot>` 占位符渲染
   - 父视图 `<template slot="...">` 填充
   - 编译期校验未知 slot 名报错
2. 运行 `cargo build` 全量编译
3. 运行 `cargo test -p rust-rml-engine` 验证 props_registry 一致性测试

---

## 假设与决策

### 决策

1. **Slot 分发机制 = Slot 字段注入**：组件 struct 持有 `Option<AnyElement>` slot 字段，父视图 codegen 通过 `entity.update(cx, |this, _cx| { this.__rml_set_slot_xxx(...); })` 注入。选择此方案因其最简单、最契合现有 Entity 模式。

2. **属性校验 = 编译期 error + warning 双层**：
   - validator 对「完全未知属性」报编译 error（用户拼写错误立即发现）
   - codegen 对「已注册但未映射」属性输出 warning（框架开发者补全）

3. **`<slot>` 标签在 codegen 中是占位符**：不创建 GPUI 元素，仅替换为 `self.__rml_slot_<name>.take()` 字段访问。

4. **default 插槽用 `<slot />`（无 name 属性）**：保留名 `"default"` 在 `slots` 数组中对应。

### 已知限制（MVP）

- **独立 re-render 限制**：组件独立 re-render（组件自身状态变化，父视图未 re-render）时，slot 内容已被 `.take()` 消费，渲染为空。父视图状态变化驱动的 slot 更新正常。文档中标注此限制。
- **作用域插槽延后**：`<slot let-item={item}>` 作用域插槽不在本期范围，保持"规划中"。

---

## 验证步骤

1. `cargo build` — 全量编译通过
2. `cargo test -p rust-rml-engine` — props_registry 一致性测试通过
3. `cargo run -p rust-rml-demo` — demo 启动，Card 组件 slot 渲染正常
4. 手动验证：在 demo 中故意写错 slot 名（如 `<template slot="hdr">`），确认编译期报 error
5. 手动验证：在 demo 中故意写错属性名（如 `<Button labl="...">`），确认编译期报 error

---

## 实施顺序

```
Step 1 (scanner 捕获 slots)
  → Step 2 (UserComponentInfo 携带 slots)
  → Step 3 (宏注入 slot 字段 + setter)
  → Step 4 (codegen <slot> 占位符)
  → Step 5 (gen_user_component 注入 slot)
  → Step 6 (validator 校验)
  → Step 7 (props_registry 修复)
  → Step 8 (文档同步)
  → Step 9 (demo 验证)
```

Step 1-2 为数据流基础设施，Step 3-5 为 slot 核心机制，Step 6-7 为校验与齐全性保障，Step 8-9 为文档与验证。
