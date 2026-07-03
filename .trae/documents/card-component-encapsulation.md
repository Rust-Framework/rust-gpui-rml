# 计划：将 Card 卡片组件封装到 ui crate

## Summary

参考 [Ant Design Card](https://ant.design/components/card-cn) 标准，把 `demo/src/components/` 下基于 RML 模板 + slot 机制的 Card 升级为 `crates/ui` 中 code-based Rust struct 组件，注册到 `component_lookup` 路由表，让用户 `<Card>` 开箱即用。

API 范围：**标准核心** — `title` + `extra` + `cover` + `bordered/borderless` + `hoverable` + `size` + body 内容（`.child()`）。不包含 actions 数组（可用 `.footer()` 自行实现）和 tabs（Tab 组件职责）。

## Current State Analysis

### 现有 demo Card（待替换）

* [demo/src/components/card.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml) — RML 模板，使用 `<slot name="header"/>` / `<slot/>` / `<slot name="footer"/>` 三个插槽

* [demo/src/components/card.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml.rs) — `#[component(slots = ["header","default","footer"])]` 用户组件 struct

* [demo/src/components/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/mod.rs) — 通过 `#[path = "card.rml.rs"] pub mod card;` 引入

* [demo/src/cases/slot\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml) — 使用 `<Card>` + `<template slot="...">` 填充

* [demo/src/cases/slot\_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml.rs) — `SlotCase` 持有 `card: Option<gpui::Entity<Card>>` 字段并在 `on_loaded` 初始化

### ui crate 现有约定

ui crate 现有 code-based 组件均遵循以下模式（参考 [crates/ui/src/components/status\_bar.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs)、[crates/ui/src/components/tab/tab.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs)）：

* `#[derive(IntoElement)]` struct，内部含 `base: Div` 字段

* impl `ParentElement`（body 内容）、`Styled`（转发到 `base.style()`）、`Sizable`（尺寸变体）

* 需要交互（hover/click）的组件 impl `InteractiveElement` + `StatefulInteractiveElement`，使用 `Stateless` kind（构造器 `new(id: impl Into<ElementId>)`，codegen 自动注入 `("rml_el", N)` id）

* 不需要交互的组件使用 `StatelessNoId` kind（构造器 `new()`）

### engine 路由 / props\_registry 现状

* [crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) — `component_lookup` 注册扩展组件（PascalCase 标签），含 `ComponentKind` 枚举区分构造模式

* [crates/engine/src/compiler/props\_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) — `COMPONENT_PROPS` 数组登记组件专用属性，供 validator 校验 + codegen 提示未映射属性

* [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) — `component_static_setter` / `component_bind_setter` / `component_event_setter` 三个 dispatcher，已支持委托到组件专用模块（avatar/menu/accordion/input/tree）

* 组件专用 setter 模块采用 `<name>/mod.rs` + `<name>/setters.rs` 拆分（参考 [crates/engine/src/compiler/avatar/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/avatar/)），mod.rs 仅 re-export

### gpui-component 现状

* gpui-component **未提供** Card 组件，需基于 `gpui::div()` 自建布局

* 已暴露 `Sizable`/`Size`/`ActiveTheme`/`h_flex`/`v_flex` 等基础设施供复用

## Proposed Changes

### 1. 新建 `crates/ui/src/components/card.rs` — Card 组件实现

**职责**：单文件承载 `CardVariant` 枚举 + `Card` struct + 全部 impl（满足铁律：一个独立组件独占一个 rs 文件，预估 \~180 行无需拆分）。

**API 设计**：

```rust
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq)]
pub enum CardVariant {
    #[default]
    Default,        // 默认带边框
    Borderless,     // 无边框
}

#[derive(IntoElement)]
pub struct Card {
    base: gpui::Div,
    title: Option<SharedString>,
    extra: Option<AnyElement>,
    cover: Option<AnyElement>,
    footer: Option<AnyElement>,
    variant: CardVariant,
    size: Size,
    hoverable: bool,
    children: Vec<AnyElement>,
}
```

**Builder 方法**：

* `Card::new(id: impl Into<ElementId>)` — 与 Button 一致的 Stateless 构造

* `.title(impl Into<SharedString>)` — 卡片标题

* `.extra(impl IntoElement)` — 标题栏右侧附加区域

* `.cover(impl IntoElement)` — 顶部封面图

* `.footer(impl IntoElement)` — 底部区域（actions 容器）

* `.variant(CardVariant)` — 设置变体

* `.borderless()` — Borderless 快捷方法

* `.bordered(bool)` — 显式控制边框（`false` → Borderless，`true` → Default）

* `.hoverable()` — 启用悬浮提升（shadow + 微弱背景变化）

**Trait 实现**（参考 Tab 模式）：

* `ParentElement` → body children 通过 `.child(...)` / `.children(...)` 注入

* `Styled` → 转发到 `base.style()`，支持 `class` / `style` / 内联样式

* `Sizable` → 支持 `.small()` / `.large()`，控制 body padding（Default: `px_6`/`py_6` ≈ 24px；Small: `px_4`/`py_4` ≈ 16px）

* `InteractiveElement` + `StatefulInteractiveElement` → 转发到 `base.interactivity()`，支持 hover

**Render 布局**（垂直，参考 Ant Design）：

```
┌─────────────────────────┐
│   cover (顶部图)        │  ← .cover()
├─────────────────────────┤
│ title    |    extra     │  ← header 区，仅当 title/extra 有值时渲染
├─────────────────────────┤
│                         │
│   body (children)       │  ← .child(...) 累积
│                         │
├─────────────────────────┤
│   footer (底部)         │  ← .footer()
└─────────────────────────┘
```

主题取值：`bg = cx.theme().background`、`border_color = cx.theme().border`、`radius = cx.theme().radius`。`hoverable` 时 `.hover(|s| s.shadow_md())`，需通过 `base.id(...)` 启用 stateful。

### 2. 修改 `crates/ui/src/components/mod.rs`

在 [crates/ui/src/components/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs) 添加：

```rust
pub mod card;
// 并在 pub use 列表中追加：
pub use card::{Card, CardVariant};
```

### 3. 修改 `crates/ui/src/lib.rs`

在 [crates/ui/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs) 第 81-87 行的 `pub use components::{...}` 中追加 `Card, CardVariant`。

### 4. 新建 `crates/engine/src/compiler/card/mod.rs`

仅 mod 声明 + re-export（遵守 mod.rs 铁律）：

```rust
//! Card 组件 codegen 模块入口。
//! 构造器由 component::gen_component 的 Stateless 分支统一处理，
//! 本模块仅提供专用 setter（title/extra/cover/footer/bordered/borderless/hoverable）。

pub mod setters;
pub use setters::{bind_setter, static_setter};
```

### 5. 新建 `crates/engine/src/compiler/card/setters.rs`

参考 [crates/engine/src/compiler/avatar/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/avatar/setters.rs) 模式，实现 Card 专用属性 → builder 方法映射：

**静态属性**：

* `title="..."` → `.title("...")`（SharedString，需 `{:?}` 转义）

* `borderless=""` / `borderless="true"` → `.borderless()`（variant 标志，empty/true 启用）

* `hoverable=""` / `hoverable="true"` → `.hoverable()`（empty/true 启用）

* `bordered="true"` → 不生成（默认即 Default 变体）

* `bordered="false"` → `.borderless()`（语义等价）

**绑定属性**：

* `title={expr}` → `.title(self.expr.clone())`（SharedString 需 clone）

* `extra={expr}` → `.extra(expr)`（IntoElement，不 clone）

* `cover={expr}` → `.cover(expr)`（IntoElement）

* `footer={expr}` → `.footer(expr)`（IntoElement）

* `bordered={expr}` → `.bordered(expr)`（bool 表达式）

* `hoverable={expr}` → `.hoverable(expr)`（bool 表达式）

含完整单元测试覆盖 static/bind 各分支（参考 avatar setters 的测试结构）。

### 6. 修改 `crates/engine/src/compiler/mod.rs`

在 [crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) 第 5-16 行的 `pub mod` 列表中添加：

```rust
pub mod card;
```

### 7. 修改 `crates/engine/src/compiler/component.rs`

在 [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 中：

* `component_static_setter`（L209-279）顶部追加委托：

  ```rust
  if let Some(s) = super::card::static_setter(name, value, &resolved) {
      return Some(s);
  }
  ```

* `component_bind_setter`（L323-377）顶部追加委托：

  ```rust
  if let Some(s) = super::card::bind_setter(name, expr_str, loop_vars, computed, &resolved) {
      return Some(s);
  }
  ```

### 8. 修改 `crates/engine/src/tags.rs`

在 [crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `component_lookup` 函数（L292-403）中添加：

```rust
"Card" => Some(ComponentTag {
    ctor_path: "rml_ui::Card",
    kind: ComponentKind::Stateless,
}),
```

使用 `Stateless` kind（而非 StatelessNoId）原因：Card 需支持 `hoverable` 悬浮效果，GPUI 中 `.hover()` 闭包需 stateful div（带 id）。codegen 自动注入 `("rml_el", N)` id，用户在 RML 中无需手写 id。

### 9. 修改 `crates/engine/src/compiler/props_registry.rs`

在 [crates/engine/src/compiler/props\_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `COMPONENT_PROPS` 数组（L66-84）追加：

```rust
("Card", &["title", "extra", "cover", "footer", "bordered", "borderless", "hoverable"]),
```

注：`small`/`large` 已在 `COMMON_STATIC_PROPS` 通用清单中，无需重复登记。

### 10. 删除 demo 旧 Card 文件

* 删除 [demo/src/components/card.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml)

* 删除 [demo/src/components/card.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml.rs)

### 11. 修改 `demo/src/components/mod.rs`

[demo/src/components/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/mod.rs) 删除 `card` 模块声明。如果 mod.rs 仅剩 `//! 可复用自定义组件` 注释无任何模块，保留空文件以备后续扩展。

### 12. 修改 `demo/src/cases/slot_case.rml.rs`

[demo/src/cases/slot\_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml.rs) 适配新 API：

* 移除 `use crate::components::card::Card;` 导入（改用 `rml::prelude::*` 中的 `Card`，从 ui crate re-export）

* 移除 `card: Option<gpui::Entity<Card>>` 字段

* 移除 `on_loaded` 中 `self.card = Some(cx.new(|_| Card::new()));` 初始化（Card 不再是 Entity，无需预创建）

### 13. 修改 `demo/src/cases/slot_case.rml`

[demo/src/cases/slot\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml) 改写为 Ant Design 风格 API：

```rml
<component>
    <div v_flex="" class="case-pane">
        <h2>{t("case.slot.title")}</h2>
        <Card title={t("case.slot.header")} hoverable="">
            <p>{t("case.slot.body")}</p>
            <template slot="footer">
                <Button label={t("case.slot.footer")} primary="" />
            </template>
        </Card>
    </div>
</component>
```

**关键变化**：`<template slot="header">` + `<h3>` 改为 `title={...}` 属性（直接绑定到 i18n 文本），footer 仍用 `<template slot="footer">`（注：RML `<template slot>` 仅对 `#[component(slots=[...])]` 用户组件生效，对 code-based 组件无效，需改写为 `.footer()` 内容）。

**修正后写法**：footer 通过 `.footer()` 属性注入，但 RML 中 footer 内容是 element 而非字符串，无法直接绑定。两种方案：

* **方案 A**：footer 内容简化为文本/简单元素，通过 `footer={t("...")}` 绑定（失去 Button 复杂内容）

* **方案 B（采纳）**：将 Button 直接作为 Card 的最后一个 child，不再用 footer 区域（语义上 footer 是 Ant Design actions 区域的替代，本 demo 不需要展示 footer 分区）

**最终 slot\_case.rml**：

```rml
<component>
    <div v_flex="" class="case-pane">
        <h2>{t("case.slot.title")}</h2>
        <Card title={t("case.slot.header")} hoverable="">
            <p>{t("case.slot.body")}</p>
            <Button label={t("case.slot.footer")} primary="" />
        </Card>
    </div>
</component>
```

i18n key `case.slot.footer` 语义从"页脚插槽"变为"页脚按钮"，文本可保持不变。

### 14. 文档同步（可选，不在本任务必做范围）

[docs/06-components/slots.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md) 中的 Card 示例可保留（slots 机制文档仍有效），新增的 ui crate Card 可在 [docs/06-components/](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/) 下另起 `card.md` 描述。本计划不强制要求，由用户决定是否补充。

## Assumptions & Decisions

### 关键决策

1. **封装形式**：Code-based Rust struct（非 RML 模板组件）— 与 ui crate 现有约定一致（StatusBar/Avatar/Tree），由用户在 Phase 2 确认。

2. **API 范围**：标准核心（title + extra + cover + bordered + hoverable + size + body）— 由用户在 Phase 2 确认。不包含 actions 数组、loading 骨架屏、tabs（这些超出标准核心）。

3. **构造 kind 选择 Stateless 而非 StatelessNoId**：Card 需支持 `hoverable` 悬浮效果，GPUI `.hover()` 闭包要求 stateful div（带 id）。Stateless kind 让 codegen 自动注入 `("rml_el", N)` id，用户 RML 标记 `<Card>` 无需手写 id，与 `<Button>` 一致。

4. **CardVariant 枚举**：使用 `CardVariant::Default | CardVariant::Borderless` 而非 `bordered: bool` 字段 — 遵循用户偏好（"adding variants to existing enums over exposing new interfaces"），与 `TabVariant` 一致。`bordered(bool)` 方法作为 Ant Design 兼容性的语法糖，内部转译到 variant。

5. **Size 复用 gpui-component 的** **`Size`** **枚举**：不单独定义 `CardSize`，通过 `Sizable` trait 复用 — 与 Tab/Avatar 约定一致。仅消费 `Size::Small` 和 `Size::default()`（Default），其他档位行为退化为 Default。

6. **不实现** **`onclick`** **等事件**：Ant Design Card 本身不暴露 click 事件（通过 actions 区域内的按钮交互）。用户如需点击行为可通过 `.child(Button)` 实现。

### 已知风险

1. **slot\_case.rml.rs 中移除** **`Entity<Card>`** **字段后**，`ILifecycle::on_loaded` 可能为空 — 此时应保留空 `on_loaded` 实现或移除该 trait impl（取决于其他逻辑依赖）。本计划保留空 `on_loaded` 以最小化变动。

2. **demo 现有错误**（来自贡献系统重构）：根据 topics.md 记录，demo 存在 `MenuItems/ActivityPanels not found` 预存错误，与本任务无关。本计划验证以 `cargo build -p rust-rml-ui`、`cargo build -p rust-rml-engine`、`cargo test -p rust-rml-engine` 为准，demo 编译错误需用户单独解决贡献系统问题后再验证。

## Verification Steps

### 单元测试

1. **engine card setters 单元测试**：

   ```bash
   cargo test -p rust-rml-engine --lib card
   ```

   验证 `static_setter` 和 `bind_setter` 各分支输出正确。

2. **props\_registry 一致性测试**：

   ```bash
   cargo test -p rust-rml-engine --test props_registry_complete
   ```

   验证 `Card` 的 7 个专用属性全部登记且 setter 已映射。

3. **tags 路由测试**：

   ```bash
   cargo test -p rust-rml-engine --lib tags
   ```

   验证 `component_lookup("Card")` 返回 Stateless kind。

### 编译验证

1. **ui crate 编译**：

   ```bash
   cargo build -p rust-rml-ui
   ```

   验证 `card.rs` 实现 + re-export 正确。

2. **engine crate 编译**：

   ```bash
   cargo build -p rust-rml-engine
   ```

   验证 `card/mod.rs` + `card/setters.rs` + 委托逻辑正确。

3. **demo 编译**（可能因预存错误失败）：

   ```bash
   cargo build -p rust-rml-demo
   ```

   预期：本任务相关的修改（Card 用法、slot\_case 适配）应编译通过；预存的贡献系统错误可能仍存在，需用户单独处理。

### Clippy 检查

1. **零警告检查**：

   ```bash
   cargo clippy -p rust-rml-ui
   cargo clippy -p rust-rml-engine
   ```

### 全量回归

1. **engine 测试套件**：

   ```bash
   cargo test -p rust-rml-engine
   ```

   确保现有 294 lib tests + 42 integration tests 全部通过，无回归。

## 文件清单

| 操作     | 路径                                            | 说明                                         |
| ------ | --------------------------------------------- | ------------------------------------------ |
| NEW    | crates/ui/src/components/card.rs              | Card 组件实现（\~180 行）                         |
| NEW    | crates/engine/src/compiler/card/mod.rs        | engine codegen 模块入口                        |
| NEW    | crates/engine/src/compiler/card/setters.rs    | Card 专用 setter + 测试                        |
| MODIFY | crates/ui/src/components/mod.rs               | 添加 card 模块 + re-export                     |
| MODIFY | crates/ui/src/lib.rs                          | 在 components re-export 追加 Card/CardVariant |
| MODIFY | crates/engine/src/compiler/mod.rs             | 添加 pub mod card                            |
| MODIFY | crates/engine/src/compiler/component.rs       | 在 static/bind setter 委托 card               |
| MODIFY | crates/engine/src/tags.rs                     | component\_lookup 注册 Card → Stateless      |
| MODIFY | crates/engine/src/compiler/props\_registry.rs | COMPONENT\_PROPS 登记 Card 专用属性              |
| DELETE | demo/src/components/card.rml                  | 旧 RML 模板                                   |
| DELETE | demo/src/components/card.rml.rs               | 旧用户组件 struct                               |
| MODIFY | demo/src/components/mod.rs                    | 移除 card 模块声明                               |
| MODIFY | demo/src/cases/slot\_case.rml                 | 改用新 Card API                               |
| MODIFY | demo/src/cases/slot\_case.rml.rs              | 移除 Entity<Card> 字段和 on\_loaded 初始化         |

