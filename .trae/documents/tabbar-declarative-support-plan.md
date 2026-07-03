# TabBar 声明式支持完整规划

## Summary

让 `<TabBar>` / `<Tab>` 可在 `.rml` 中声明式使用，覆盖 gpui-component Tabs 文档的全部场景（5 种 variant、4 种尺寸、prefix/suffix、menu、禁用、动态 tabs），并通过 `Tab` 的 `ParentElement` 能力实现**标签标题模板定制**这一高级能力。同步扩展 `<tab_window>` shell：新增 `<template slot="tabs">` 写法，让窗口壳内置的 TabBar 也支持模板定制（与现有 `tabs={Vec<TabItem>}` 简单模式互斥二选一）。

事件签名遵循 gpui-component 原生 `TabBar::on_click`：`on_click={method}` → 用户方法 `fn method(&mut self, index: usize, cx: &mut Context<Self>)`。

## Current State Analysis

### 已具备
- `crates/ui/src/components/tab/{tab.rs, tab_bar.rs, mod.rs}` 完整实现 `TabBar` / `Tab` / `TabVariant`，`crates/ui/src/lib.rs:85` 已 re-export `Tab, TabBar, TabVariant`
- `Tab` 实现 `ParentElement`（[tab.rs:606-610](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L606-L610)）—— 子节点直接作为标题内容渲染（模板定制的原生入口）
- `TabBar` 实现 `Styled + Sizable + ParentElement`，子节点接受 `impl Into<Tab>`
- `<tab_window>` shell 通过 `tabs={Vec<TabItem>}` 使用 TabBar，但 [TabItem](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L33-L51) 仅支持 `label: SharedString` + `icon: Option<IconName>`，无法表达模板

### 缺失
- `tags.rs::component_lookup` 路由表**未登记** `TabBar` / `Tab`，`.rml` 中写 `<TabBar>` 报 "unknown component"
- 无 `compiler/tab_bar/` codegen 模块
- `props_registry.rs::COMPONENT_PROPS` 未登记 TabBar/Tab 专用属性
- `<tab_window>` 不支持 `<template slot="tabs">` —— `partition_slot_children` 仅识别 menu/title/footer/left/right/bottom

### 参照样板
Accordion 是 `StatelessWithItems` 的成熟范例（[compiler/accordion/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/)）。本规划按其结构搭建 `compiler/tab_bar/`，但有关键差异：

| 维度 | Accordion | TabBar |
|------|-----------|--------|
| 子节点注入方式 | `.item(\|__rml_item\| __rml_item.title(...).child(...))` 闭包 | `.child(rml_ui::Tab::new().label(...).child(...))` 直接构造 |
| 子项构造 | codegen 在闭包内构造 AccordionItem | codegen 直接构造 `Tab::new()...` 表达式 |
| 事件签名 | `on_toggle_click(open_ixs: &[usize])` | `on_click(idx: &usize)` |
| 容器构造 | `Accordion::new(id)` | `TabBar::new(id)` |

## Proposed Changes

### Phase A — 独立 `<TabBar>` / `<Tab>` 声明式支持

#### A1. 路由表注册 — [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)

**`component_lookup`（L292-416）新增条目**：
```rust
"TabBar" | "tab_bar" => Some(ComponentTag {
    ctor_path: "rml_ui::TabBar",
    kind: ComponentKind::StatelessWithItems,
}),
```

**`is_item_builder_tag`（L423-426）扩展**：
```rust
pub fn is_item_builder_tag(tag: &str) -> bool {
    matches!(tag, "AccordionItem" | "item" | "Tab" | "tab")
        || normalize_component_tag(tag) == "AccordionItem"
        || normalize_component_tag(tag) == "Tab"
}
```

**`canonical_tag`（L154-161）扩展小写别名**：
```rust
"tab_bar" => "TabBar".to_string(),
"tab" => "Tab".to_string(),
```

**注意**：`tab` 不在 `BuiltinTag` 中（已确认 `build_tag_map` 无 `tab`），无冲突。

#### A2. codegen 模块 — 新建 `crates/engine/src/compiler/tab_bar/`

**`mod.rs`**：模块聚合 + re-export `gen_tab_bar`
```rust
pub mod gen;
pub mod tab;
pub mod setters;
pub use gen::gen_tab_bar;
```

**`gen.rs`** — TabBar 容器 codegen（参照 [accordion/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/gen.rs) 结构）：
- 构造：`rml_ui::TabBar::new((\"rml_el\", Nusize))` 或 `rml_ui::TabBar::new(\"rml_ref:<name>\")`（ref 指令）
- 属性循环：先调 `tab_bar::setters::*`，未命中回退 `component::component_*_setter`（处理 Sizable 等通用属性）
- 子节点处理（**与 Accordion 关键差异**）：
  - `Node::Element` 且 `is_item_builder_tag` 命中 → 调 `tab::gen_tab_child` 生成 `rml_ui::Tab::new()...` 表达式，用 `.child(...)` 链接
  - `Node::Element` 非法子节点 → 报错 "<TabBar> 仅支持 <Tab> 子节点"
  - `Node::Text` → 警告忽略（TabBar 不接受文本子节点）

**`tab.rs`** — 单个 `<Tab>` 子节点 codegen（参照 [accordion/item.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/item.rs)，但**不生成闭包**）：
- 输出：`rml_ui::Tab::new().label("...").icon(rml_ui::IconName::User).disabled(true).child(<element>)...`
- 属性处理：先调 `tab_bar::setters::*`，未命中回退 `component::component_*_setter`
- 子节点处理：`<Tab>` 的 element 子节点通过 `.child(...)` 注入（每个子节点一次），文本子节点映射到 `.label(...)`（与 Button 一致行为，但仅当无 `label` 属性时生效）
- 返回 `String`（非闭包字符串）

**`setters.rs`** — TabBar/Tab 专用属性 → builder 方法映射：

```rust
// TabBar 静态属性
"underline" | "pill" | "flat" | "outline" | "segmented" => .<name>()  // variant 快捷方法
"menu" => .menu(true) / .menu(false)

// TabBar 绑定属性
"selected_index" => .selected_index(<expr>)
"prefix" => .prefix(<expr>)          // element，不加 .clone()
"suffix" => .suffix(<expr>)
"last_empty_space" => .last_empty_space(<expr>)
"menu" => .menu(<bool expr>)

// Tab 静态属性
"label" => .label(<str>)              // 与 component_static_setter 重叠，回退即可
"icon" => .icon(rml_ui::IconName::<Name>)   // 参照 accordion icon 处理
"underline" | "pill" | "flat" | "outline" | "segmented" => .<name>()

// Tab 绑定属性
"prefix" => .prefix(<expr>)
"suffix" => .suffix(<expr>)

// TabBar 事件
"on_click" => .on_click(cx.listener(move |this, idx: &usize, _window, cx| {
    this.<method>(*idx, cx);
}))
// 用户方法签名：fn method(&mut self, index: usize, cx: &mut Context<Self>)

// Tab 事件（与 TabBar::on_click 不同 —— 接受 ClickEvent）
"on_click" => .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {
    let rml_ev = rml_convert::from_gpui_click(_ev);
    this.<method>(&rml_ev, cx);
}))
```

**TabBar 与 Tab 同名 `on_click` 区分**：在 `setters::event_setter(name, handler, tag)` 中根据 `tag` 参数（"TabBar" vs "Tab"）选择签名。

#### A3. component.rs 委托 — [component.rs:76-86](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L76-L86)

在 `StatelessWithItems` 分支增加 TabBar 委托（与 Accordion 并列）：
```rust
tags::ComponentKind::StatelessWithItems => {
    // Accordion 与 TabBar 都使用 StatelessWithItems，按 tag 分派
    if tag == "TabBar" || tag == "tab_bar" {
        return crate::compiler::tab_bar::gen_tab_bar(
            elem, ref_name, id_val, ctx, id_counter, loop_vars,
        );
    }
    // 默认委托到 accordion（保持向后兼容）
    return crate::compiler::accordion::gen_accordion(
        elem, ref_name, id_val, ctx, id_counter, loop_vars,
    );
}
```

#### A4. props_registry.rs — [props_registry.rs:66-86](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L66-L86)

`COMPONENT_PROPS` 新增：
```rust
("TabBar", &["selected_index", "on_click", "prefix", "suffix",
             "last_empty_space", "menu", "track_scroll",
             "underline", "pill", "flat", "outline", "segmented"]),
("Tab", &["label", "icon", "disabled", "selected", "prefix", "suffix",
          "on_click",
          "underline", "pill", "flat", "outline", "segmented"]),
```

### Phase B — 扩展 `<tab_window>` tabs 模板定制

#### B1. TabWindowShell 扩展 — [tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs)

新增字段与 setter：
```rust
pub struct TabWindowShell {
    // ... 现有字段 ...
    tab_children: Vec<Tab>,   // 新增：模板定制模式
}

impl TabWindowShell {
    /// 模板定制模式：直接注入 Tab 列表，绕过 TabItem 的 label/icon 限制。
    /// 与 `tabs(Vec<TabItem>)` 互斥；非空时优先使用。
    pub fn tab_children(mut self, children: Vec<Tab>) -> Self {
        self.tab_children = children;
        self
    }
}
```

**`render` 方法调整**（L393-399）：
```rust
if !self.tab_children.is_empty() {
    // 模板定制模式：直接注入 Tab，不修改 label/icon
    for tab in self.tab_children.drain(..) {
        tab_bar = tab_bar.child(tab);
    }
} else {
    // 简单模式：沿用 TabItem → Tab::new().label().icon()
    for tab in &self.tabs {
        let mut t = Tab::new().label(tab.label.clone());
        if let Some(icon) = tab.icon.clone() { t = t.icon(icon); }
        tab_bar = tab_bar.child(t);
    }
}
```

#### B2. codegen shell 扩展 — [shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs)

**`partition_slot_children`（L121-190）**：
- 返回元组新增 `slot_tabs: Option<Vec<Node>>`（与其他单 Node slot 不同，因为 tabs 内可放多个 `<Tab>`）
- 识别 `<template slot="tabs">`，将其所有子节点（应为 `<Tab>` 元素）作为 Vec 返回

**`gen_tab_window_wrapper`（L219-334）**：
- 新增参数 `slot_tabs: Option<&[String]>`（每个元素是一个 Tab 子节点的 codegen 输出）
- 若 `slot_tabs` 非空：生成 `.tab_children(vec![<Tab1>, <Tab2>, ...])`

#### B3. render.rs slot 传递 — [render.rs:49-115](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs#L49-L115)

- 解构 `partition_slot_children` 增加 `slot_tabs`
- 对 `slot_tabs` 中每个 `<Tab>` 子节点调 `tab_bar::tab::gen_tab_child` 生成代码
- 拼接为 `Vec<String>` 传给 `gen_tab_window_wrapper`

#### B4. validator.rs — 允许 tabs slot

[validator.rs:148](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs#L148) 中 `<template slot="...">` 校验增加 `"tabs"` 合法名（仅 tab_window）。

### Phase C — Demo 案例

#### C1. 新增 `demo/src/cases/tab_bar_case.rml`

参照 [accordion_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml) 结构，演示：
1. **基础 Tabs**：`<TabBar selected_index={0}><Tab label="Account" /><Tab label="Profile" /></TabBar>`
2. **5 种 variant**：`underline` / `pill` / `outline` / `segmented` / 默认 tab
3. **尺寸**：`xsmall` / `small` / `large`
4. **带图标**：`<Tab icon="User" label="Account" />`
5. **prefix/suffix**：`<TabBar prefix={...} suffix={...}>`
6. **禁用**：`<Tab disabled="true" />`
7. **menu 模式**：`<TabBar menu="true">`（标签过多时显示下拉）
8. **模板定制**（核心高级能力）：
   ```xml
   <TabBar selected_index={active_tab} on_click={on_tab_select}>
       <Tab>
           <Icon name="User" />
           <span>Account</span>
           <Badge label="3" />
       </Tab>
       <Tab prefix={...} suffix={...}>
           <span>Profile</span>
       </Tab>
   </TabBar>
   ```
9. **动态 tabs**：`each={tab in tabs}` 迭代生成 `<Tab>`
10. **事件回调**：`on_click={on_tab_select}` 显示选中索引

#### C2. 新增 `demo/src/cases/tab_bar_case.rml.rs`

参照 [accordion_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs) 结构：
- `TabBarCase` 结构体 + `#[component]` 宏
- `active_tab: usize` 字段
- `on_tab_select(&mut self, index: usize, cx)` 方法
- `code_sample` 计算属性展示用法

#### C3. 注册到 cases mod

- [demo/src/cases/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 新增 `pub mod tab_bar_case;`
- [demo/src/cases/catalog.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs) 新增路由条目（参照 accordion 在 catalog 中的注册）

### Phase D — 测试

#### D1. engine 单元测试

**`crates/engine/src/compiler/tab_bar/gen.rs` 内嵌测试**（参照 [accordion/gen.rs:96-334](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/gen.rs#L96-L334)）：
- `gen_tab_bar_minimal` — `<TabBar />` 最简
- `gen_tab_bar_with_static_props` — `<TabBar underline menu="true">`
- `gen_tab_bar_with_tab_child` — `<TabBar><Tab label="A" /></TabBar>` 生成 `.child(rml_ui::Tab::new().label("A"))`
- `gen_tab_bar_with_tab_template_child` — `<TabBar><Tab><Icon /><span>A</span></Tab></TabBar>` 生成 `.child(...)` 链
- `gen_tab_bar_with_on_click` — `<TabBar on_click={handler} />` 验证 `idx: &usize` 签名
- `gen_tab_bar_with_ref_uses_stable_id` — ref 指令
- `gen_tab_bar_rejects_non_tab_child` — `<TabBar><div /></TabBar>` 报错
- `gen_tab_bar_with_sizable` — `<TabBar small underline>` 混用通用与专用属性
- `gen_tab_bar_via_gen_component_dispatch` — 通过 `gen_component` 入口端到端
- `gen_tab_bar_lowercase_tag` — `<tab_bar>` 别名

**`setters.rs` 内嵌测试**：每个 setter 分支覆盖。

**`tags.rs` 测试**：`component_lookup("TabBar")` 命中、`is_item_builder_tag("Tab")` 命中、`canonical_tag("tab_bar")` == "TabBar"。

**`props_registry.rs` 测试**：`is_prop_registered("TabBar", "selected_index")` 等。

#### D2. 编译验证

- `cargo build -p rust-rml-engine` — engine 编译通过
- `cargo test -p rust-rml-engine` — 全部单元测试通过
- `cargo build -p rust-rml-ui` — ui crate 编译通过（TabWindowShell 扩展）
- `cargo build` — demo 编译通过
- 手动运行 demo，打开 tab_bar_case，验证 5 种 variant、模板定制、事件回调正常

## Assumptions & Decisions

1. **TabBar 与 Accordion 共享 `StatelessWithItems` kind，但 codegen 路径不同**：Accordion 用 `.item(closure)`，TabBar 用 `.child(Tab::new()...)`。在 `component.rs` 的 `StatelessWithItems` 分支按 tag 委托，不引入新 `ComponentKind` 变体。
2. **`<Tab>` 同时支持 label/icon 属性与 element 子节点**：二者并存时由 gpui-component Tab 运行时决定渲染优先级（icon > label/children）。codegen 不会阻止组合，但 demo 案例展示典型用法。
3. **TabBar 与 Tab 都支持 `on_click`，签名不同**：TabBar 接收 `&usize`（索引），Tab 接收 `&ClickEvent`。在 `setters::event_setter` 中按 tag 区分。运行时 TabBar 的 on_click 会覆盖 Tab 的 on_click（gpui-component 行为）。
4. **`<tab_window>` 的 `<template slot="tabs">` 与 `tabs={Vec<TabItem>}` 互斥**：TabWindowShell 优先检查 `tab_children`，非空时忽略 `tabs`。validator 应在编译期检测二者并存并报错。
5. **不删除 TabItem**：TabItem 仍适用于 `#[computed] fn tab_bar_items() -> Vec<TabItem>` 简单场景（Clone 友好）。模板定制走 `<template slot="tabs">` 路径。
6. **`track_scroll` 属性**：需要 `ScrollHandle` 引用，与 `ref="handle"` 指令配合复杂，本规划暂不实现 track_scroll 的声明式绑定，仅在 props_registry 登记（占位）。
7. **`with_variant` bind 属性**：直接传 `TabVariant` 枚举值场景罕见，本规划通过 5 个快捷方法（`underline`/`pill`/...）覆盖，不实现 `with_variant={expr}`。
8. **`<Tab>` 不支持 `each` 直接迭代**：动态 tabs 通过在 `<TabBar>` 内写 `<Tab each={tab in tabs} label={tab.name} />` 实现（`each` 指令在子节点上工作）。需确认 `gen_tab_child` 支持 `each` 指令路径。

## Verification Steps

1. **engine 单元测试**：`cargo test -p rust-rml-engine` 全绿，新增测试覆盖 TabBar 路由、属性、子节点、事件、模板定制。
2. **ui crate 编译**：`cargo build -p rust-rml-ui` 通过，TabWindowShell 扩展无破坏。
3. **demo 编译运行**：`cargo run` 启动后能在 case 列表中打开 "TabBar" 案例，所有 5 种 variant、尺寸、prefix/suffix、禁用、模板定制、事件回调按预期工作。
4. **回归验证**：现有 `<tab_window>` demo（main_window.rml 用 `tabs={tab_bar_items}`）行为不变。
5. **props_registry 一致性测试**：`cargo test -p rust-rml-engine --test props_registry_complete` 通过（如存在）。

## Implementation Order

1. **Phase A1-A4**（engine 注册 + codegen 模块）→ 验证：`cargo test -p rust-rml-engine` 通过新增单元测试
2. **Phase B1-B4**（tab_window 扩展）→ 验证：`cargo build -p rust-rml-ui` 通过，engine 测试不回归
3. **Phase C1-C3**（demo 案例）→ 验证：`cargo build` + 手动运行 demo
4. **Phase D**（测试完善 + 回归）→ 全部 cargo test 通过

## Out of Scope

- `track_scroll` 与 `ScrollHandle` 的声明式绑定（高级滚动场景，后续迭代）
- `with_variant={TabVariant::Pill}` 直接 bind 枚举（用快捷方法替代）
- TabBar 在 `<dialog>` / `<modern_window>` 内的特殊布局适配（无需求）
- 修改 `TabItem` 结构（保留其 Clone 性质，简单场景仍可用）
