# Tabs/TabBar 拆分改造计划

## 概述

按 WPF TabControl 设计标准，将现有 `TabBar`（header+body 一体）拆分为：
- **`Tabs`**（WPF TabControl 风格）：header + body 切换，`bordered` 包裹整体，支持 `on_close`/`on_promote`
- **`TabBar`**（原生 gpui-component 形态）：纯 header 标签栏，无 body/无 close/无 promote，内部委托 `Tabs` 渲染

## 当前状态分析

### Phase 1（UI 源码）— 已完成 ✓

- [x] `crates/ui/src/components/tab/tabs.rs` — `Tabs` struct 存在，`bordered` 字段在 v_flex 外层包裹 header+body
- [x] `crates/ui/src/components/tab/tab_bar.rs` — 原生 `TabBar` wrapper（`inner: Tabs` 委托模式），不暴露 `bordered`/`on_close*`/`on_promote`
- [x] `crates/ui/src/components/tab/mod.rs` — 声明 `tab`/`tab_bar`/`tab_item`/`tabs` 四个模块
- [x] `crates/ui/src/components/tab/tab.rs` — `Tab` struct，有 `new()`/`label()`/`icon()` 方法
- [x] `crates/ui/src/components/tab/tab_item.rs` — `TabItem` struct，有 `From<Tab> for TabItem` 转换
- [x] `crates/ui/src/lib.rs`、`components/mod.rs`、`prelude.rs` — 导出 `Tabs`
- [x] `crates/ui/src/window/tab_window.rs` — 使用 `Tabs::new(...)`

### Phase 2（编译器）— 进行中 🔄（当前 engine crate 无法编译）

- [x] `crates/engine/src/compiler/tabs/` 目录存在（从 `tab_bar/` 重命名）
- [x] `tabs/mod.rs` — 导出 `gen_tabs`
- [x] `tabs/gen.rs` — `gen_tabs` 函数，生成 `rml_ui::Tabs::new(...)`
- [ ] **`tabs/setters.rs`** — 仍有 `"TabBar"` tag 参数，需改为 `"Tabs"`
- [ ] **`tab_bar/` 编译器模块不存在** — 需新建原生 TabBar codegen
- [ ] **`compiler/mod.rs` line 26** — `pub mod tab_bar;` 指向不存在的模块（编译断裂），需改为 `pub mod tabs;` + `pub mod tab_bar;`
- [ ] **`component.rs` line 118** — 路由 `"TabBar"` → `tab_bar::gen_tab_bar`（不存在），需拆分为 `"Tabs"` → `tabs::gen_tabs` + `"TabBar"` → `tab_bar::gen_tab_bar`
- [ ] **`tags.rs` line 510-516** — 仅有 `"TabBar" | "tab-bar"` → `rml_ui::TabBar`，缺 `"Tabs" | "tabs"` → `rml_ui::Tabs`
- [ ] **`props_registry.rs` line 117-130** — 仅有 `("TabBar", &[...])`（含 on_close/bordered），需拆分为 `("Tabs", &[全量])` + `("TabBar", &[原生子集])`

### Phase 3-5 — 未开始

- [ ] 文档（tabs.md + tab-bar.md + INDEX.md）
- [ ] 示范迁移（3-5 个 demo 文件）
- [ ] 全量测试验证

---

## 实施步骤

### 阶段 2：编译器改造（续）

#### 2.1 `tabs/setters.rs` — tag 参数 `"TabBar"` → `"Tabs"`

**文件**: `crates/engine/src/compiler/tabs/setters.rs`

**操作**: 全局替换 `"TabBar"` → `"Tabs"`（replace_all），影响：
- `static_setter` line 26/33/41：`if tag == "TabBar"` → `if tag == "Tabs"`
- `bind_setter` line 94/102/120：`if tag == "TabBar"` → `if tag == "Tabs"`
- `event_setter` line 161：`if tag != "TabBar"` → `if tag != "Tabs"`
- 所有测试函数中的 `"TabBar"` 参数 → `"Tabs"`
- 文档注释中的 `TabBar` → `Tabs`（仅描述 tag 参数的部分）

#### 2.2 新建 `tab_bar/` 编译器模块（原生 TabBar codegen）

**新建文件**:

**`crates/engine/src/compiler/tab_bar/mod.rs`**:
```rust
//! 原生 TabBar codegen 模块入口（纯 header 标签栏，无 body/无 close）。
//!
//! - `gen.rs`：TabBar 容器构造 + 属性 + 子节点 `.child(TabItem::new()...)` 注入
//! - `setters.rs`：TabBar 专用属性 → builder 方法映射（不含 bordered/on_close*）
//!
//! `<tab>` 子节点 codegen 复用 `tabs::tab::gen_tab_child`（生成 `TabItem::new()...`），
//! 因 TabBar 的 `child()` 接受 `impl Into<TabItem>`（通过 `From<Tab> for TabItem` 兼容 Tab）。

pub mod gen;
pub mod setters;

pub use gen::gen_tab_bar;
```

**`crates/engine/src/compiler/tab_bar/gen.rs`**:
- `gen_tab_bar` 函数签名与 `gen_tabs` 一致
- 构造器：`rml_ui::TabBar::new(id)` / `rml_ui::TabBar::new("rml_ref:{name}")`
- 属性路由：先调 `tab_bar::setters::*_setter(name, value, "TabBar")`，未命中回退到 `component::*_setter(name, value, "TabBar")`
- 子节点路由：复用 `super::super::tabs::tab::gen_tab_child(child_elem, ctx, id_counter, loop_vars)` 生成 `TabItem::new()...`
- 错误信息：`<TabBar> 仅支持 <Tab> 子节点`

**`crates/engine/src/compiler/tab_bar/setters.rs`**:
- `static_setter(name, value, tag)`：
  - `tag == "TabBar"`：`underline`/`pill`/`flat`/`outline`/`segmented`（variant 快捷）、`menu`（bool）
  - **不含 `bordered`**（TabBar 不暴露此方法）
  - `tag == "Tab"`：复用 `tabs::setters::static_setter` 逻辑（`icon`→`.title_icon()`、`label`→`.title()`、`closable`、`preview`）
- `bind_setter(name, expr, ..., tag)`：
  - `tag == "TabBar"`：`selected_index`/`menu`/`last_empty_space`/`track_scroll`/`prefix`/`suffix`
  - **不含 `bordered`**
  - `tag == "Tab"`：复用 `tabs::setters::bind_setter`
- `event_setter(name, handler, tag)`：
  - `tag == "TabBar"`：仅 `on_click`
  - **不含 `on_close`/`on_close_all`/`on_close_others`/`on_promote`**
  - `tag == "Tab"`：返回 None（走通用 ClickEvent 路径）

**设计决策**：`tab_bar/setters.rs` 中 `tag == "Tab"` 的情况直接委托 `super::super::tabs::setters::*_setter`，避免重复 Tab 子节点 setter 逻辑。

#### 2.3 `compiler/mod.rs` — 添加 `pub mod tabs;`

**文件**: `crates/engine/src/compiler/mod.rs` line 26

**操作**: 将 `pub mod tab_bar;` 替换为：
```rust
pub mod tab_bar;
pub mod tabs;
```

#### 2.4 `component.rs` — 路由拆分

**文件**: `crates/engine/src/compiler/component.rs` line 118

**操作**: 将现有：
```rust
if resolved_tag == "TabBar" {
    return crate::compiler::tab_bar::gen_tab_bar(...);
}
```
替换为：
```rust
if resolved_tag == "Tabs" {
    return crate::compiler::tabs::gen_tabs(...);
}
if resolved_tag == "TabBar" {
    return crate::compiler::tab_bar::gen_tab_bar(...);
}
```

#### 2.5 `tags.rs` — 注册 `"Tabs" | "tabs"` 标签

**文件**: `crates/engine/src/tags.rs` line 510-516

**操作**: 在现有 `TabBar` 条目前添加 `Tabs` 条目：
```rust
// Tabs：WPF TabControl 风格标签容器，header + body 切换
// PascalCase: <Tabs>，kebab-case: <tabs>
"Tabs" | "tabs" => Some(ComponentTag {
    ctor_path: "rml_ui::Tabs",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
// TabBar：原生 gpui-component 形态标签栏（纯 header，无 body）
"TabBar" | "tab-bar" => Some(ComponentTag {
    ctor_path: "rml_ui::TabBar",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
```

**同步更新**：
- line 604-605 文档注释：补充 `Tabs` 支持说明
- 测试函数：添加 `Tabs`/`tabs` 的 canonical_tag/component_lookup 断言

#### 2.6 `props_registry.rs` — 拆分 props

**文件**: `crates/engine/src/compiler/props_registry.rs` line 117-130

**操作**: 将现有 `("TabBar", &[...])` 拆分为：
```rust
// Tabs 专用（WPF TabControl：全量属性，含 on_close/bordered）
("Tabs", &[
    "selected_index", "on_click", "on_close", "on_close_all", "on_close_others", "on_promote",
    "prefix", "suffix", "last_empty_space",
    "menu", "track_scroll",
    "bordered",
    "underline", "pill", "flat", "outline", "segmented",
]),
// TabBar 专用（原生形态：不含 on_close*/bordered）
("TabBar", &[
    "selected_index", "on_click",
    "prefix", "suffix", "last_empty_space",
    "menu", "track_scroll",
    "underline", "pill", "flat", "outline", "segmented",
]),
```

**同步更新测试**：
- `tab_bar_props_registered` 测试拆分为 `tabs_props_registered` + `tab_bar_props_registered`
- 验证 `TabBar` 不含 `bordered`/`on_close`

#### 2.7 `shell.rs` — 测试 mock 数据更新（可选）

**文件**: `crates/engine/src/compiler/codegen/shell.rs`

**操作**: 将测试中的 `rml_ui::Tab::new().label("A")` mock 数据更新为 `rml_ui::TabItem::new().title("A")`，保持与实际 codegen 输出一致。这是非功能性变更，仅影响测试可读性。

#### 2.8 TabBar UI 微调 — `child()` 接受 `impl Into<TabItem>`

**文件**: `crates/ui/src/components/tab/tab_bar.rs`

**操作**: 将 `child()` 和 `children()` 的参数类型从 `impl Into<Tab>` 改为 `impl Into<TabItem>`：
```rust
pub fn children(mut self, children: impl IntoIterator<Item = impl Into<TabItem>>) -> Self {
    self.inner = self.inner.children(children);
    self
}

pub fn child(mut self, child: impl Into<TabItem>) -> Self {
    self.inner = self.inner.child(child);
    self
}
```

**原因**: codegen 统一生成 `TabItem::new()...`（复用 `tabs::tab::gen_tab_child`），TabBar 需直接接受 `TabItem`。`From<Tab> for TabItem` 仍兼容手动传入 `Tab` 的场景。

**验证**: `cargo build -p rust-rml-ui` 通过

---

### 阶段 3：文档

#### 3.1 `docs/06-components/reference/tabs.md`

**新建文件**。内容：
- 概述：WPF TabControl 风格，header + body 切换
- 声明式语法：`<tabs variant="underline" selected_index={active}><tab label="A">body</tab></tabs>`
- 属性表：`variant`/`selected_index`/`menu`/`bordered`/`track_scroll`/`prefix`/`suffix`/`last_empty_space`
- 事件：`on_click`/`on_close`/`on_close_all`/`on_close_others`/`on_promote`
- 子节点：`<tab>`（label/icon/closable/preview + body 内容）
- 示例：基础用法、带 body、带 close、each 迭代

#### 3.2 `docs/06-components/reference/tab-bar.md`

**新建文件**。内容：
- 概述：原生 gpui-component 形态，纯 header 标签栏
- 声明式语法：`<tab-bar underline selected_index={active}><tab label="A" /></tab-bar>`
- 属性表：`variant`/`selected_index`/`menu`/`track_scroll`/`prefix`/`suffix`/`last_empty_space`（**无 bordered**）
- 事件：`on_click`（**无 on_close***）
- 子节点：`<tab>`（label/icon，**无 body**）
- 示例：基础用法、variant 切换、each 迭代

#### 3.3 `docs/06-components/INDEX.md`

**操作**: 添加 `tabs` 和 `tab-bar` 两个条目，链接到对应文档。

---

### 阶段 4：示范迁移

迁移 3 个 demo 文件作为示范（不批量迁移全部 43 个）：

1. **查找使用 `<tab-bar>` 或 `<TabBar>` 的 demo 文件**（Grep `tab-bar\|TabBar` in `crates/demo/`）
2. **按用途分类迁移**：
   - 纯 header 场景 → 保持 `<tab-bar>`（原生形态）
   - header + body 场景 → 改为 `<tabs>`（TabControl 形态）
3. **更新示范文件**：
   - 修改 `.rml` 文件中的标签
   - 验证生成的代码正确

---

### 阶段 5：测试与验证

1. `cargo build -p rust-rml-ui` — UI crate 编译
2. `cargo build -p rust-rml-engine` — engine crate 编译
3. `cargo test -p rust-rml-engine` — engine 单测（含 tabs/tab_bar codegen 测试）
4. `cargo test -p rust-rml-ui` — UI 单测
5. `cargo build -p rust-rml-demo` — demo 编译
6. 全量 `cargo test` — 无回归

---

## 关键设计决策

1. **`<tab>` 标签统一编译为 `TabItem::new()...`**：无论父容器是 `<tabs>` 还是 `<tab-bar>`，子节点都生成 `TabItem`。TabBar 的 `child()` 接受 `impl Into<TabItem>`，通过 `From<Tab> for TabItem` 兼容手动传 `Tab` 的场景。

2. **`tab_bar/` 编译器模块复用 `tabs::tab::gen_tab_child`**：不重复实现 `<tab>` 子节点 codegen，直接调用 `super::super::tabs::tab::gen_tab_child`。

3. **`tab_bar/setters.rs` 中 `tag == "Tab"` 委托 `tabs::setters`**：避免重复 Tab 子节点 setter 逻辑，保持单一数据源。

4. **`bordered` 仅 `<tabs>` 支持**：TabBar 是纯 header，无边框概念。`bordered` 在 `tabs/setters.rs` 中处理（`tag == "Tabs"`），不在 `tab_bar/setters.rs` 中出现。

5. **`on_close`/`on_close_all`/`on_close_others`/`on_promote` 仅 `<tabs>` 支持**：TabBar 不暴露这些方法，对应 setter 不在 `tab_bar/setters.rs` 中出现。

## 假设

- `From<Tab> for TabItem` 转换已存在且正确（已在 Phase 1 验证）
- `tabs::tab::gen_tab_child` 生成的 `TabItem::new()...` 表达式可直接用于 TabBar 的 `.child()` 调用
- `tags::is_item_builder_tag` 已识别 `"Tab"`/`"tab"`，无需修改
- 现有 `tabs/gen.rs` 的测试模式可作为 `tab_bar/gen.rs` 测试的参考模板
