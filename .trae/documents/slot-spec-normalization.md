# Slot 规范化（权威规范 + 文档同步 + 清理 + Demo 验证）

## 概述

规范化 RML 框架的 slot（插槽）规范，回答用户的三个核心问题：

1. **组件开发者如何预留插槽位？标准语法是什么？**
2. **使用方如何填充插槽？标准语法是什么？如何保证 RML 代码层次分明、简洁清晰？**
3. **如何确保 RML 支持的属性配置齐全？**

上一轮迭代（`.trae/documents/slot-specification.md`）已完成 Steps 1-7 的**实现**（scanner/UserComponentInfo/宏/codegen/validator/props_registry 全部落地并通过编译与 218+18 测试）。本轮聚焦于：**把实现固化为权威规范**，**同步过时文档**，**清理死代码**，**用 demo 端到端验证**。

---

## 当前状态分析（Phase 1 探索结论）

### 已实现并验证（无需改动）

| 能力 | 位置 | 证据 |
|------|------|------|
| `StructMetadata.slots` + `parse_component_slots` | `crates/engine/src/build/scanner.rs:79,245` | 解析 `#[component(slots=[...])]` |
| `UserComponentInfo.slots` + build.rs 填充 | `crates/engine/src/compiler/mod.rs:71` + `build/mod.rs:197` | codegen 可查询 |
| 宏注入 `__rml_slot_<name>` 字段 + `__rml_set_slot_<name>` setter | `crates/macros/src/component.rs:85,152,209` | + `IComponent::slots()` override `:181` |
| `<slot>` 占位符 codegen | `crates/engine/src/compiler/codegen/mod.rs:334` | `self.__rml_slot_<name>.take()` |
| `gen_user_component` slot 注入 + `partition_user_component_children` + `gen_slot_content` | `crates/engine/src/compiler/component.rs:191,249,279` | 父视图注入 slot 内容 |
| validator 校验 slot 名 + 未知属性 | `crates/engine/src/compiler/validator.rs:29,89,126` | 编译期 error |
| `props_registry` 修复（tag 规范化 + `props_for` 合并 + `SHELL_PROPS` 含 component） | `crates/engine/src/compiler/props_registry.rs:84,134,166` | 单一信源 |
| codegen warning（registered-but-unmapped） | `component.rs:369` + `shell.rs:67,267` | 框架开发者补全提示 |
| AST `Element.slot_name` + parser 解析 `slot="..."` | `parser/ast.rs:45` + `parser/mod.rs:190` | `<template slot>` 识别 |
| `IComponent::slots()` trait 方法 | `crates/core/src/component.rs:39` | 契约定义 |

### 过时/错误（本计划修复）

| 问题 | 位置 | 现状 | 应为 |
|------|------|------|------|
| `slots.md` 标注自定义组件 slot 为"规划中" | `docs/06-components/slots.md:11,96,145` | 与实现不符 | 标为"已实现"，给出标准语法 |
| `slots.md` 缺 `<slot>` 占位符 + `<template slot>` 填充 + `#[component(slots)]` 完整示例 | `docs/06-components/slots.md` | 仅 shell 插槽示例 | 补自定义组件 slot 完整章节 |
| `custom-components.md` 6.2.7 语法错误 | `docs/06-components/custom-components.md:319-351` | 用了 `<slot name="header">默认标题</slot>`（默认内容，**不支持**）+ 无 slot 属性的 `<template>` 作默认插槽 | 改为 `<slot name="header" />`（无默认内容）+ `<template slot="header">` + 裸子节点填 default |
| `props-mapping.md` 未反映 registry 修复 | `docs/06-components/reference/props-mapping.md` | 未提 tag 规范化、warning 机制、default slot 须显式声明 | 同步维护规则 + slot 契约 |

### 死代码（本计划清理）

| 项 | 位置 | 说明 |
|----|------|------|
| `Directive::Slot(String)` 变体 | `crates/engine/src/parser/ast.rs:81` | parser 不再 push 此 directive（直接设 `Element.slot_name`），变体无用 |
| validator 中 `Directive::Slot` 不可达 arm | `crates/engine/src/compiler/validator.rs:73` | 配套删除 |
| `partition_user_component_children` 的 `_declared_slots` 参数 | `crates/engine/src/compiler/component.rs:251` | 未使用，slot 名校验已由 validator 负责 |

### 缺失（本计划新增）

- 无自定义组件 slot 的 demo → 新增 `Card` 组件 + `SlotCase` 案例端到端验证

---

## 权威 Slot 规范（回答三个核心问题）

> 本节是规范本身。文档同步步骤将把此内容落地到 `slots.md`。

### Q1：组件开发者如何预留插槽？标准语法

**两处声明缺一不可：**

**① Rust 侧 —— `#[component]` 宏参数声明插槽契约：**

```rust
#[component(slots = ["header", "default", "footer"])]
#[derive(IModel)]
pub struct Card {
    pub title: SharedString,
}
```

- `slots` 为字符串数组字面量
- 保留名 `"default"` 对应模板内无 `name` 属性的 `<slot />`
- 不写 `slots` 参数 → 组件不接受任何插槽（父视图传 `<template slot>` 会被 validator 报 error）
- 宏自动为每个 slot 注入私有字段 `__rml_slot_<name>: Option<gpui::AnyElement>` 与 setter `__rml_set_slot_<name>`

**② RML 模板侧 —— `<slot>` 占位符声明渲染位置：**

```html
<!-- components/card.rml -->
<component>
    <div class="card">
        <div class="card-header">
            <slot name="header" />
        </div>
        <div class="card-body">
            <slot />
        </div>
        <div class="card-footer">
            <slot name="footer" />
        </div>
    </div>
</component>
```

- `<slot name="header" />` 声明具名插槽位置
- `<slot />`（无 `name`）声明默认插槽位置（对应 `"default"`）
- codegen 将 `<slot>` 替换为 `self.__rml_slot_<name>.take()`，不创建 GPUI 元素
- **`<slot>` 不支持默认内容**：`<slot>默认文本</slot>` 中的子节点会被忽略，未填充的插槽渲染为空

### Q2：使用方如何填充插槽？标准语法

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

    <!-- 默认插槽：无 slot 属性的裸子节点（仅当 Card 声明了 "default"） -->
    <p>This is the card body content.</p>
</Card>
```

**标准语法规则：**

| 场景 | 语法 | 说明 |
|------|------|------|
| 填充具名插槽 | `<template slot="name">...</template>` | `name` 必须在组件 `slots` 声明中，否则编译期 error |
| 填充默认插槽 | 裸子节点（无 `slot` 属性） | 仅当组件声明了 `"default"`；否则被忽略 + validator error |
| 自闭合空填充 | `<template slot="header"></template>` | 等价于不填充，渲染为空 |
| 未填充的插槽 | — | 渲染为空（`Option::None`） |

**层次分明、简洁清晰的原则：**
- 具名插槽内容一律用 `<template slot="...">` 包裹，与 default 内容视觉分离
- 一个 `<template slot>` 内可放多节点（codegen 自动包裹 `div` 容器）
- default 内容放在所有 `<template slot>` 之后，作为"主内容"
- 不要在 `<slot>` 占位符内放内容（不支持默认内容）

### Q3：如何确保 RML 支持的属性配置齐全？

**单一信源 + 双层校验：**

1. **单一信源**：`crates/engine/src/compiler/props_registry.rs`
   - `COMPONENT_PROPS`：扩展组件专用属性（Button/Input/Tree/MenuBar/status_bar...）
   - `SHELL_PROPS`：shell 根标签属性（window/tab_window/modern_window/component）
   - `COMMON_STATIC_PROPS` / `COMMON_BIND_PROPS` / `COMMON_EVENT_PROPS`：通用属性
   - `is_prop_registered(tag, attr)` / `is_shell_prop_registered(tag, attr)`：查询时自动 kebab-case → PascalCase 规范化

2. **编译期 error（用户拼写错误）**：`crates/engine/src/compiler/validator.rs`
   - shell 根标签的 bind/event 属性若不在 `SHELL_PROPS` → `ValidationError`
   - 扩展组件的 bind/event 属性若不在 `COMPONENT_PROPS` + 通用 → `ValidationError`
   - 用户组件的 `<template slot="x">` 中 `x` 若不在 `slots` 声明 → `ValidationError`

3. **codegen warning（框架开发者映射缺失）**：
   - `component_static_setter` / `component_bind_setter` 未命中分支：若 `is_prop_registered` 为 true → `eprintln!("[rml warning] ...")`
   - `gen_tab_window_wrapper` / `gen_modern_window_wrapper` 未命中分支：若 `is_shell_prop_registered` 为 true → warning
   - 提示开发者在对应 match 添加分支

**维护规则**（写入 `props-mapping.md`）：添加新组件/属性时必须三同步 —— ① registry 登记 → ② setter match 分支 → ③ `cargo test -p rust-rml-engine` 通过。

---

## 实施变更

### Step 1: 重写 `docs/06-components/slots.md`

**文件**: `docs/06-components/slots.md`

**变更**: 完整重写，结构如下：
1. **当前实现状态**：删除"规划中"标注，自定义组件 slot 标为 ✅ 已实现；作用域插槽保留"规划中"
2. **6.3.1 插槽概念**：保留（更新图示为通用父-子组件，不限于 shell）
3. **6.3.2 Shell 窗口插槽**：保留（已实现，不变）
4. **6.3.3 自定义组件插槽（新，替代旧 6.3.3+6.3.4）**：落地"权威 Slot 规范"Q1+Q2 内容
   - 组件开发者：`#[component(slots=[...])]` + `<slot>` 占位符（含"不支持默认内容"说明）
   - 使用方：`<template slot="...">` + 裸子节点 default
   - 完整 Card 示例（.rml.rs + .rml + 父视图）
5. **6.3.4 属性齐全性保障（新）**：落地 Q3 内容（单一信源 + 双层校验 + 维护规则链接）
6. **6.3.5 已知限制**：独立 re-render 时 slot 内容被 `.take()` 消费为空（MVP 限制）
7. **6.3.6 规划中特性**：作用域插槽 `<slot let-item={item}>`
8. **6.3.7 小结**

### Step 2: 修正 `docs/06-components/custom-components.md` 6.2.7

**文件**: `docs/06-components/custom-components.md`（6.2.7 章节，约 315-353 行）

**变更**:
1. 删除错误的 `<slot name="header">默认标题</slot>`（默认内容不支持）
2. 删除无 slot 属性的 `<template>` 作默认插槽的写法
3. 改为标准语法：
   - Rust 侧加 `#[component(slots = ["header", "default", "footer"])]`
   - 模板侧 `<slot name="header" />` / `<slot />` / `<slot name="footer" />`
   - 父视图 `<template slot="header">` + 裸子节点 default
4. 添加"不支持默认内容"提示 + 链接到 `slots.md`

### Step 3: 同步 `docs/06-components/reference/props-mapping.md`

**文件**: `docs/06-components/reference/props-mapping.md`

**变更**:
1. **维护规则**章节：补充"三同步"流程 + tag 规范化说明（kebab-case → PascalCase 自动转换）
2. **新增"属性齐全性双层保障"小节**：说明 validator error（未知属性）+ codegen warning（已注册未映射）
3. **新增"插槽属性"小节**：`<slot name="...">` 占位符 + `<template slot="...">` 填充 + `#[component(slots=[...])]` 契约
4. 确认 `<component>` 标签的 `content` 属性已登记（`SHELL_PROPS` 已含）

### Step 4: 清理死代码

**文件**:
- `crates/engine/src/parser/ast.rs`：删除 `Directive::Slot(String)` 变体
- `crates/engine/src/compiler/validator.rs`：删除 `Directive::Slot` 的 match arm（约 `:73`）
- `crates/engine/src/compiler/component.rs`：`partition_user_component_children` 删除 `_declared_slots: &[String]` 参数，更新调用点（`gen_user_component:209`）

**原因**: 这些是上一轮迭代的遗留死代码。parser 直接设 `Element.slot_name`，不再走 directive；slot 名校验已由 validator 负责，`partition` 不需要声明列表。

**验证**: `cargo build -p rust-rml-engine` 通过，无新 warning。

### Step 5: 新增 Card 组件 + SlotCase demo

**新增文件**:
- `demo/src/components/mod.rs`：声明 `pub mod card;`
- `demo/src/components/card.rml.rs`：`#[component(slots = ["header", "default", "footer"])]` 的 `Card` 组件（含 `title: SharedString` 字段）
- `demo/src/components/card.rml`：`<slot name="header" />` + `<slot />` + `<slot name="footer" />` 占位符布局
- `demo/src/cases/slot_case.rml.rs`：`#[contribute]` + `#[component]` 的 `SlotCase`，使用 `<Card>` + `<template slot="...">` 填充
- `demo/src/cases/slot_case.rml`：case 模板，引用 Card 并填充 header/default/footer

**修改文件**:
- `demo/src/main.rs`（或 `lib.rs`）：新增 `mod components;` 声明（参照现有 `mod cases;` / `mod shell;`）
- `demo/src/cases/mod.rs`：新增 `#[path = "slot_case.rml.rs"] pub mod slot_case;` + `pub use slot_case::SlotCase;`
- `demo/src/cases/catalog.rs`：注册 `SlotCase`（参照 `ButtonCase`，分组 `components`，order 紧随 button）

**demo 内容要点**（教学性）:
- `Card` 模板：标题栏（header slot）+ 正文（default slot）+ 页脚（footer slot），用 div + 简单样式
- `SlotCase`：渲染一个 Card，header 放标题文本 + 关闭按钮，default 放说明段落，footer 放操作按钮
- 案例树节点名称：`case.slot.title`（i18n key，参照现有命名）

### Step 6: 验证

1. `cargo build` —— 全量编译通过（含 demo）
2. `cargo test -p rust-rml-engine --lib` —— 218 测试通过
3. `cargo test -p rust-rml-engine --tests` —— 18 集成测试通过
4. `cargo run -p rust-rml-demo` —— demo 启动，SlotCase 在案例树 components 分组下可见，点击后 Card 三插槽渲染正常
5. 手动负向验证（在 slot_case.rml 临时改错）：
   - `<template slot="hdr">` → 编译期 error `unknown slot name`
   - `<Card unkown_prop={x}>` → 编译期 error `unknown property`

---

## 假设与决策

### 决策

1. **规范以现有实现为准**：Vue 风格 `<slot name>` 占位 + `<template slot>` 填充 + `#[component(slots)]` 契约已落地，本轮将其固化为权威文档，不改动实现语义。

2. **`<slot>` 不支持默认内容**：实现中 `<slot>` 的子节点被忽略。文档明确标注，避免误导。默认内容需求由父视图显式填充解决。

3. **default 插槽须显式声明**：组件必须在 `slots` 数组中包含 `"default"`，父视图的裸子节点才会被路由到 default slot；否则 validator 报 error。这是编译期安全保障。

4. **属性齐全性 = 单一信源 + 双层校验**：registry 是唯一信源，validator error 拦截用户错误，codegen warning 提示框架开发者补全。不在本期引入运行期校验。

5. **死代码清理纳入本轮**：`Directive::Slot` 等是上一轮迭代遗留，趁规范化一并清除，保持代码与规范一致。

### 已知限制（写入文档）

- **独立 re-render 限制**：组件独立 re-render（组件自身状态变化，父视图未 re-render）时，slot 内容已被 `.take()` 消费，渲染为空。父视图状态变化驱动的 slot 更新正常。MVP 限制，文档标注。
- **作用域插槽延后**：`<slot let-item={item}>` 不在本期范围，保留"规划中"。

---

## 验证步骤

1. `cargo build` — 全量编译通过
2. `cargo test -p rust-rml-engine` — 单元 + 集成测试全通过
3. `cargo run -p rust-rml-demo` — demo 启动，SlotCase 渲染 Card 三插槽正常
4. 负向验证：故意写错 slot 名 / 属性名，确认编译期 error
5. 文档审查：`slots.md` / `custom-components.md` 6.2.7 / `props-mapping.md` 语法示例与实现一致

---

## 实施顺序

```
Step 1 (重写 slots.md) — 权威规范落地
  → Step 2 (修正 custom-components.md 6.2.7)
  → Step 3 (同步 props-mapping.md)
  → Step 4 (清理死代码)
  → Step 5 (Card + SlotCase demo)
  → Step 6 (验证)
```

Step 1-3 为文档规范化（核心交付），Step 4 为代码与规范对齐，Step 5-6 为端到端验证。
