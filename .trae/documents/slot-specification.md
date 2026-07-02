# RML Slot 规范化方案

> 目标：统一 slot 语法为 Vue 风格，让自定义组件具备内容分发能力；建立框架内部属性映射注册表，杜绝 codegen 阶段的属性静默丢弃。

---

## 一、摘要

当前 RML 框架存在三套互不相关的 "slot" 概念，且文档与实现严重割裂：

| 概念 | 状态 |
|------|------|
| `slot_*` 硬编码标签（仅 tab_window / modern_window 根节点可用） | 唯一可用，命名错位（`slot_footer`→`status_slot`←`slot_status`） |
| `Directive::Slot(String)` | 死代码，parser 解析后 codegen 零消费 |
| `ContributionOptions::slot` | 运行时贡献点路由，与模板语法无关 |

文档 `docs/06-components/slots.md` 承诺的 Vue 风格 `<slot>` / `<template slot="...">` 完全未实现；`#[component]` 宏不声明 slot 契约；`component_bind_setter` 硬编码 + 未知属性静默丢弃。

本方案分四步：① 统一为 Vue 风格 slot 语法；② `#[component]` 宏显式声明 slot 契约；③ 废弃 `slot_*` 硬编码标签、改造 tab_window / modern_window 为通用机制；④ 建立框架内部属性映射注册表，配合单测确保 codegen 翻译齐全。

---

## 二、现状分析（基于 Phase 1 探索）

### 2.1 Slot 三套概念混淆

- **可用层**：[shell.rs:91-152](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L91-L152) 的 `partition_slot_children` 按标签名字符串硬编码拆分 `slot_menu`/`slot_title`/`slot_footer`/`slot_left`/`slot_right`/`slot_bottom`，仅对 `<tab_window>` / `<modern_window>` 根节点生效（[codegen/mod.rs:168-173](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L168-L173)）。
- **死代码**：[parser/mod.rs:186-189](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/mod.rs#L186-L189) 解析 `slot="name"` 为 `Directive::Slot(String)`，但 codegen 全程无消费。
- **无关概念**：`crates/core/src/contribution.rs:23,48-51` 的 `ContributionOptions.slot` 是贡献点路由字段，不应共享 slot 词汇。

### 2.2 命名错位三角

| RML 标签 | codegen 变量 | builder 方法 | TabWindowShell 字段 |
|----------|--------------|--------------|---------------------|
| `slot_menu` | `slot_menu` | `.menu_slot()` | `menu_slot` |
| `slot_title` | `slot_title` | `.title_ext_slot()` | `title_ext_slot` |
| `slot_footer` | `slot_footer`（[mod.rs:222](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L222)） | `.status_slot()` | `status_slot`（参数名 `slot_status` 见 [shell.rs:175](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L175)） |

`slot_footer` → `status_slot` 的语义割裂源于历史命名（status_bar 是 gpui-component 原生控件，TabWindowShell 把 footer slot 装入 status_bar）。Some 包裹也不统一：`menu_slot` 不包 `Some`，`status_slot`/`slot_left` 包 `Some`（[shell.rs:74-82, 254-271](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L74-L82)）。

### 2.3 ComponentKind 无 Slot 变体

[tags.rs:241-255](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L241-L255) 仅有 `Stateless` / `StatelessNoId` / `Stateful{state_field}` / `EntityRef` 四变体。`#[component]` 宏（[macros/src/component.rs:148-176](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs#L148-L176)）仅生成 `IModel`+`IViewModel`+`IComponent`（`rml_template()`+`rml_tag()`），无 slot 契约。

### 2.4 bind_setter 硬编码 + 静默丢弃

[component.rs:298-336](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L298-L336) 的 `component_bind_setter` 逐属性名 match，未命中返回 `None` 静默丢弃。`tag` 参数虽传入但 `_tag = tag` 未使用。`shell.rs:208-216` 的 tab_window 属性绑定（tabs/selected_tab/show_chrome/left_size 等）同样逐名硬编码。

### 2.5 文档承诺 vs 实现真空

[docs/06-components/slots.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md) 描述了完整的 Vue 风格 slot：默认插槽 `<slot></slot>`、具名插槽 `<slot name="...">`、默认内容、作用域插槽 `<slot let-item={item}>`、`<template slot="...">` 填充语法——**全部未实现**。

---

## 三、提议变更

### Step 1：统一 Slot 语法（Vue 风格）

**目标**：让 `<slot>` / `<template slot="...">` 真正可用，与文档对齐。

#### 1.1 定义端（组件作者在 .rml 模板内）

```rml
<!-- components/card.rml -->
<div class="card">
    <div class="card-header">
        <slot name="header">默认标题</slot>
    </div>
    <div class="card-body">
        <slot />                              <!-- 默认插槽 -->
    </div>
    <div class="card-footer">
        <slot name="footer" />                <!-- 无默认内容的具名插槽 -->
    </div>
</div>
```

- `<slot>` 标签由 parser 识别为新的 `Node::Slot { name: Option<String>, default_children: Vec<Node> }` 变体（替代死代码 `Directive::Slot`）。
- 无 `name` 属性 → 默认插槽（每组件最多一个）。
- 标签内子节点 → 默认内容（父视图未填充时显示）。

#### 1.2 使用端（父视图填充）

```rml
<!-- views/my_view.rml -->
<Card>
    <template slot="header">
        <h2>用户信息</h2>
    </template>
    <p>姓名: 张三</p>                         <!-- 默认插槽内容（无需 template 包装） -->
    <template slot="footer">
        <button onclick={edit}>编辑</button>
    </template>
</Card>
```

- `<template slot="name">` 块的子节点 → 注入到组件内对应 `<slot name="name">` 位置。
- 直接子节点（非 `<template slot=...>`）→ 注入到默认 `<slot />` 位置。
- 沿用 `Directive::Slot` 现有的 `slot="name"` 静态属性解析（[parser/mod.rs:186-189](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/mod.rs#L186-L189)），但 codegen 必须真正消费它。

#### 1.3 编译流程改造

| 文件 | 改造内容 |
|------|----------|
| `crates/engine/src/parser/ast.rs` | 新增 `Node::Slot { name: Option<String>, default_children: Vec<Node> }` 变体；`Element` 增加 `slot_name: Option<String>` 字段（来自 `slot="..."` 指令） |
| `crates/engine/src/parser/mod.rs:186-189` | 保留 `Directive::Slot(s)` 解析；在 `parse_element` 末尾把它升级为 `Element.slot_name` 字段，不再仅作 directive |
| `crates/engine/src/parser/mod.rs` | 新增 `parse_slot_element`：识别 `<slot>` 标签，构造 `Node::Slot` 而非 `Node::Element` |
| `crates/engine/src/compiler/codegen/mod.rs` | 自定义组件调用路径新增 `inject_slot_children`：把 `<template slot="x">` 子节点收集到 `slots: HashMap<String, Vec<Node>>`，传给组件 codegen |
| `crates/engine/src/compiler/codegen/component_render.rs`（新文件） | 自定义组件渲染时，扫描其 .rml 模板的 `Node::Slot` 占位符，用父视图传入的 slots HashMap 替换；未填充则用 default_children |

#### 1.4 分期决策：作用域插槽延后

`<slot let-item={item}>` 作用域插槽涉及子→父反向数据流，与 `each` 列表渲染深度耦合，第一期不实现。slots.md 中相关章节标注为"规划中"。

---

### Step 2：Slot 契约显式声明（`#[component]` 宏参数）

**目标**：编译时强校验，未知 slot 名编译报错。

#### 2.1 宏语法

```rust
#[component(slots = ["header", "footer", "default"])]
pub struct Card { ... }

// 无 slots 参数 → 不接受任何 slot（默认）
#[component]
pub struct Counter { ... }
```

- `slots` 参数为可选字符串数组字面量。
- 字符串 `"default"` 保留为默认插槽标识（与 `<slot />` 对应）。
- 不写 `slots` 参数 → 组件不接受 slot，父视图传 `<template slot="...">` 编译报错。

#### 2.2 宏改造

| 文件 | 改造内容 |
|------|----------|
| `crates/macros/src/component.rs:181-189` | `expand(args, ...)` 接受 `slots = [...]` 参数解析为 `Vec<String>`；移除"takes no arguments"硬性拒绝 |
| `crates/macros/src/component.rs:148-176` | `expand_component_impls` 新增 `slots: &[String]` 参数，生成 `IComponent::slots() -> &'static [&'static str]` 方法 |
| `crates/core/src/component.rs:23-30` | `IComponent` trait 新增 `fn slots() -> &'static [&'static str] { &[] }` 默认实现 |

#### 2.3 编译器校验

| 文件 | 改造内容 |
|------|----------|
| `crates/engine/src/compiler/validator.rs` | 使用端：父视图 `<template slot="x">` 时，查目标组件 `IComponent::slots()`，若 `x` 不在其中 → 编译错误 `"Component X does not have slot 'x'. Available slots: header, footer"` |
| `crates/engine/src/compiler/validator.rs` | 定义端：组件 .rml 模板内 `<slot name="y">` 时，`y` 必须在 `#[component(slots=[...])]` 声明中，否则编译错误 |

---

### Step 3：废弃 `slot_*` 硬编码标签，统一到 Vue 风格

**目标**：消除 shell.rs 的硬编码 partition，让 tab_window / modern_window 也走通用 slot 机制。

#### 3.1 TabWindow / ModernWindow 改造为 `#[component]`

将 `TabWindowShell` / `ModernWindowShell` 视为内置 `#[component]` 组件，显式声明 slots：

```rust
// crates/ui/src/window/tab_window.rs
#[component(slots = ["menu", "title", "footer", "left", "right", "bottom", "default"])]
pub struct TabWindowShell { ... }

// crates/ui/src/window/modern_window.rs
#[component(slots = ["menu", "title", "footer", "default"])]
pub struct ModernWindowShell { ... }
```

#### 3.2 .rml 改写

`demo/src/shell/main_window.rml` 改为 Vue 风格：

```rml
<tab_window title="RML Showcase" ...>
    <template slot="left">
        <ActivityBar ref="activity_bar" />
    </template>
    <template slot="menu">
        <menu-bar items={menu_items} />
    </template>
    <template slot="title">
        <Button label="Docs" ghost="" />
    </template>
    <template slot="bottom">
        <div>Output panel — drag the top edge to resize</div>
    </template>
    <template slot="footer">
        <status_bar items={status_items} />
    </template>
    <component content={self.active_case_view(_window, cx)} />
</tab_window>
```

#### 3.3 shell.rs 重写

| 文件 | 改造内容 |
|------|----------|
| `crates/engine/src/compiler/codegen/shell.rs:91-152` | 删除 `partition_slot_children` 硬编码 match；改为通用 `partition_template_slots`：扫描子节点中的 `<template slot="x">`，按 `x` 收集到 `HashMap<String, Node>`，剩余子节点作为 default |
| `crates/engine/src/compiler/codegen/shell.rs:155-166` | `slot_element_content` 改名为 `template_block_content`，逻辑不变（单子节点 unwrap，多子节点包 div） |
| `crates/engine/src/compiler/codegen/shell.rs:74-82, 254-271` | builder 链生成改为查 TabWindowShell 的 slots 声明动态生成 `.slot_xxx(...)` 调用；Some 包裹统一（所有 slot setter 接受 `Option<AnyElement>`） |
| `crates/ui/src/window/tab_window.rs:209, 245` | 统一 setter 签名：`menu_slot(Option<AnyElement>)`、`status_slot(Option<AnyElement>)`、`slot_left(Option<AnyElement>)` 等，消除 Some 包裹不一致 |
| `crates/ui/src/window/modern_window.rs:19-27` | 同上统一签名 |

#### 3.4 命名一致性

| RML slot 名 | builder 方法 | 字段名 |
|-------------|--------------|--------|
| `menu` | `.slot_menu(Option<AnyElement>)` | `slot_menu` |
| `title` | `.slot_title(Option<AnyElement>)` | `slot_title` |
| `footer` | `.slot_footer(Option<AnyElement>)` | `slot_footer` |
| `left` | `.slot_left(Option<AnyElement>)` | `slot_left` |
| `right` | `.slot_right(Option<AnyElement>)` | `slot_right` |
| `bottom` | `.slot_bottom(Option<AnyElement>)` | `slot_bottom` |
| `default` | `.child(AnyElement)` | (走现有 child 链) |

消除 `slot_footer`↔`status_slot`↔`slot_status` 三角错位。

---

### Step 4：框架内部属性映射注册表（确保 codegen 翻译齐全）

**目标**：RML 框架自身在做 .rml → Rust 代码翻译时，确保所有 gpui-component 组件的可绑定属性都有映射，避免静默丢弃。这是框架开发规范，不是面向最终用户的属性校验。

#### 4.1 集中化属性注册表

新建 `crates/engine/src/compiler/props_registry.rs`：

```rust
/// 每个组件的可绑定属性白名单（framework 维护，单测覆盖）
pub static COMPONENT_PROPS: &[(&str, &[&str])] = &[
    ("Button", &["label", "disabled", "selected", "on_click", "variant", "size", "icon", "ghost"]),
    ("Input",  &["value", "disabled", "placeholder", "on_change", "on_submit"]),
    ("Checkbox", &["checked", "label", "on_change"]),
    ("Tree",  &["items", "on_activate", "on_select"]),
    // ... 所有 rml_ui 重导出的组件
];

/// 窗口外壳组件的可绑定属性
pub static SHELL_PROPS: &[(&str, &[&str])] = &[
    ("tab_window", &["title", "width", "height", "icon", "tabs", "selected_tab",
                     "show_chrome", "left_size", "right_size", "bottom_size",
                     "on_tab_click", "on_chrome_toggle"]),
    ("modern_window", &["title", "width", "height", "icon", "menu", "footer"]),
];
```

#### 4.2 bind_setter 改为查表 + 未命中处理

| 文件 | 改造内容 |
|------|----------|
| `crates/engine/src/compiler/component.rs:298-336` | `component_bind_setter` 改为：① 查 `COMPONENT_PROPS` 表确认属性属于该组件；② 命中 → 生成 `.xxx()` 调用；③ 未命中 → 编译 warning（含组件名 + 属性名 + 可用属性列表） |
| `crates/engine/src/compiler/codegen/shell.rs:208-216` | tab_window / modern_window 属性绑定改为查 `SHELL_PROPS` 表 |
| `crates/engine/src/compiler/component.rs:330` | `items` 属性去掉 `tag == "menu" || ...` 硬编码，改为注册表中按组件声明 |

#### 4.3 单测覆盖确保齐全

新建 `crates/engine/tests/props_registry_complete.rs`：

```rust
#[test]
fn all_gpui_component_setters_are_registered() {
    // 通过反射 rml_ui 重导出的所有组件，遍历其 .xxx() setter 方法
    // 断言每个 setter 都在 COMPONENT_PROPS 表中有对应条目
    // 漏登记 → 测试失败，列出缺失的 setter
}

#[test]
fn all_shell_props_are_registered() {
    // 遍历 TabWindowShell / ModernWindowShell 的 pub builder 方法
    // 断言都在 SHELL_PROPS 中
}
```

#### 4.4 文档化属性对照清单

新建 `docs/07-reference/props-mapping.md`，维护 gpui-component setter ↔ RML 属性名对照表，每次新增组件时同步更新（CI 检查单测通过即保证文档与注册表一致）。

---

## 四、假设与决策

### 4.1 关键决策

1. **Slot 语法采用 Vue 风格**（用户确认）：`<slot>` 定义 + `<template slot="...">` 填充，与 `docs/06-components/slots.md` 对齐。
2. **Slot 契约宏参数显式声明**（用户确认）：`#[component(slots = ["header", "footer", "default"])]`，编译时强校验。
3. **属性齐全性是框架内部规范**（用户确认）：不是面向最终用户的属性校验，而是 framework codegen 自身的清单/单测约束。
4. **废弃 `slot_*` 硬编码标签**：与 ActivityBar 重写一致，遵循"删除过封装、回到设计意图"的偏好，不保留兼容层。
5. **作用域插槽 `<slot let-item={item}>` 延后到第二期**：与 `each` 列表渲染深度耦合，第一期不实现，slots.md 相关章节标注"规划中"。
6. **`Directive::Slot` 升级为 `Element.slot_name` 字段**：保留 parser 解析能力，但 codegen 真正消费。
7. **Slot setter 签名统一为 `Option<AnyElement>`**：消除 `menu_slot` 不包 Some、`slot_left` 包 Some 的不一致。

### 4.2 兼容性影响

- `demo/src/shell/main_window.rml` 需改写（`slot_xxx` → `<template slot="xxx">`）。
- `crates/ui/src/window/tab_window.rs` 和 `modern_window.rs` 的 setter 签名变更（Some 包裹统一），影响所有调用方（仅框架内部）。
- 用户已有的自定义组件（如 demo 中的 cases）未使用 slot，不受影响。

### 4.3 不在本次范围

- 作用域插槽（`<slot let-item={item}>`）。
- 动态 slot 名（`<slot name={dynamic}>`）。
- `ContributionOptions.slot` 改名（与本次 slot 规范化无关，保留）。
- 第三方组件库的属性自动发现（仅维护 rml_ui 内置组件的注册表）。

---

## 五、验证步骤

### 5.1 编译验证

```powershell
cargo build -p rust-rml-engine
cargo build -p rust-rml-macros
cargo build -p rust-rml-ui
cargo build -p rust-rml-demo
```

### 5.2 单测验证

```powershell
cargo test -p rust-rml-engine                    # 现有 251 个测试全通过
cargo test -p rust-rml-engine --test props_registry_complete   # 新增注册表齐全性测试
cargo test -p rust-rml-macros                    # 宏测试
```

### 5.3 功能验证

启动 demo：`cargo run -p rust-rml-demo`，确认：

1. tab_window 标题栏左侧 ActivityBar 正常显示（`<template slot="left">` 生效）。
2. 菜单栏正常显示（`<template slot="menu">` 生效）。
3. 标题栏右侧 "Docs" 按钮正常显示（`<template slot="title">` 生效）。
4. 底部 Output panel 正常显示（`<template slot="bottom">` 生效）。
5. 底部状态栏正常显示（`<template slot="footer">` 生效）。
6. 点击 ActivityBar 图标可切换面板（slot 内容的 ref 指令仍工作）。

### 5.4 错误注入验证

故意写错 slot 名，确认编译报错：

```rml
<tab_window>
    <template slot="nonexistent">...</template>
</tab_window>
```

预期编译错误：`"Component TabWindowShell does not have slot 'nonexistent'. Available slots: menu, title, footer, left, right, bottom, default"`。

### 5.5 属性齐全性验证

故意在 .rml 中写未注册的属性：

```rml
<Button label="OK" unknown_prop="x" />
```

预期编译 warning：`"Button does not support property 'unknown_prop'. Available: label, disabled, selected, ..."`。

---

## 六、实施顺序（建议）

1. **Step 2 先行**：扩 `IComponent` trait + `#[component]` 宏参数解析（不影响现有代码，纯增量）。
2. **Step 1.3 编译流程**：parser 新增 `Node::Slot` + `Element.slot_name`，codegen 消费 `Directive::Slot`（保持 `slot_*` 标签仍工作，渐进式）。
3. **Step 3 shell 改造**：`partition_slot_children` 改为 `partition_template_slots`，tab_window / modern_window 改用 Vue 风格，删除 `slot_*` 标签支持。
4. **Step 4 属性注册表**：新建 `props_registry.rs`，`bind_setter` 改查表，加单测。
5. **demo 验证 + 文档同步**：改写 main_window.rml，更新 docs/06-components/slots.md 标注作用域插槽为"规划中"，新增 docs/07-reference/props-mapping.md。
