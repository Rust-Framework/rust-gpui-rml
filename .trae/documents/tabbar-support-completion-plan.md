# TabBar 声明式支持 —— 收尾规划（Phase A 验证 + B/C/D）

## Summary

`<TabBar>` / `<Tab>` 声明式支持的 Phase A（独立组件 codegen）已在前一轮对话中实现完毕：路由表登记、`compiler/tab_bar/` 模块（gen/setters/tab）、props_registry、tags 测试均到位。但 engine 编译被一个**预先存在且与本任务无关**的 `code_editor` 模块未注册问题阻断，导致 Phase A 单元测试无法运行验证。

本规划聚焦于：
1. 解除 `code_editor` 阻断（1 行修复）→ 完成 Phase A 编译/测试验证
2. 推进 Phase B：`<tab_window>` 扩展 `<template slot="tabs">` 模板定制
3. 推进 Phase C：新增 `tab_bar_case` demo 案例
4. 推进 Phase D：完整回归测试

事件签名与覆盖范围沿用上一轮已批准的决策：`on_click(idx: usize)`（TabBar 索引签名）+ 同时扩展独立 TabBar 与 tab_window shell。

## Current State Analysis

### Phase A 已完成（待编译验证）

| 文件 | 状态 | 说明 |
|------|------|------|
| [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) | 已修改 | `component_lookup` 登记 `TabBar`/`tab_bar`；`is_item_builder_tag` 扩展 `Tab`/`tab`；`canonical_tag` 添加 `tab_bar→TabBar`、`tab→Tab`；含 5 个新测试 |
| [compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L76-L98) | 已修改 | `StatelessWithItems` 分支按 `canonical_tag == "TabBar"` 委托到 `tab_bar::gen_tab_bar`，否则回退 accordion |
| [compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L86-L96) | 已修改 | `COMPONENT_PROPS` 新增 `TabBar` / `Tab` 条目；含 5 个新测试 |
| [compiler/tab_bar/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/mod.rs) | 新建 | 模块聚合：`pub mod gen / setters / tab`，re-export `gen_tab_bar` |
| [compiler/tab_bar/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/gen.rs) | 新建 | TabBar 容器 codegen，13 个内嵌测试（最小构造 / variant / Tab 子节点 / 模板子节点 / icon / on_click / selected_index bind / ref / 拒绝非 Tab 子节点 / Sizable / 多 Tab / gen_component 入口 / 小写别名 / `<tab>` 短形式） |
| [compiler/tab_bar/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs) | 新建 | TabBar/Tab 专用属性 → builder 方法映射；含 14 个内嵌测试；关键差异：`on_click` 在 TabBar（`idx: &usize`）与 Tab（None，回退 ClickEvent）间按 tag 分流 |
| [compiler/tab_bar/tab.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/tab.rs) | 新建 | 单个 `<Tab>` 子节点 codegen：直接构造 `rml_ui::Tab::new()...` 表达式（非闭包）；`label_set_by_attr` 跟踪；文本子节点 → `.label()`，element 子节点 → `.child()`（模板定制路径） |

### 阻断问题（预先存在，与本任务无关）

[compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) 缺少 `pub mod code_editor;` 声明，但：

- [compiler/code_editor/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/mod.rs) 与 [compiler/code_editor/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs) 文件已存在（git status 显示为 untracked，属于 LSP feature 引入但未完成模块注册）
- [compiler/component.rs:114-125](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L114-L125) 中 `tags::ComponentKind::Stateful { state_field: _ } if tag == "CodeEditor"` 分支引用 `crate::compiler::code_editor::gen_code_editor`

`cargo build -p rust-rml-engine` 因此报 `error[E0433]: cannot find code_editor in compiler`，连带 Phase A 的所有单元测试无法执行。

**注意**：此修复虽非 TabBar 任务范围，但是 Phase A 验证的前置条件。本规划将其作为"解除阻断"步骤纳入，但不重构或修改 code_editor 模块本身——只补一行 `pub mod code_editor;`。

### Phase B/C/D 未开始

- [crates/ui/src/window/tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) 仍是 `tabs: Vec<TabItem>` 单一模式（无 `tab_children` 字段，render 方法 L393-399 直接构造 `Tab::new().label().icon()`）
- [compiler/codegen/shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) 的 `partition_slot_children`（L121-190）仅识别 menu/title/footer/left/right/bottom，未识别 tabs
- [compiler/codegen/render.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs#L49-L115) 解构 `partition_slot_children` 6 元组，无 slot_tabs 传递
- [compiler/validator.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs) 的 slot 校验仅针对用户自定义组件（L88-107），shell 根标签的 slot 名未做白名单校验（未知 slot 名会落入 body，validator 已注释"应在编译期拦截"——但 tab_window 当前未拦截）
- 无 `demo/src/cases/tab_bar_case.rml` 或 `.rml.rs` 文件
- [demo/src/cases/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 与 [demo/src/cases/catalog.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs) 未登记 tab_bar_case

## Proposed Changes

### Phase A-Verify — 解除 code_editor 阻断 + Phase A 测试验证

#### A-V1. 补 code_editor 模块声明 — [compiler/mod.rs:5-18](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L5-L18)

在现有 `pub mod` 列表（按字母序）中插入一行：

```rust
pub mod accordion;
pub mod avatar;
pub mod card;
pub mod code_editor;   // ← 新增：解除 E0433 阻断
pub mod codegen;
pub mod component;
// ... 其余不变
```

**只动这一行**。不修改 code_editor 内部实现，不动 component.rs 的 CodeEditor 分支。

#### A-V2. 运行 Phase A 测试

```powershell
cargo build -p rust-rml-engine
cargo test -p rust-rml-engine --lib compiler::tab_bar
cargo test -p rust-rml-engine --lib tags::normalize_tests
cargo test -p rust-rml-engine --lib compiler::props_registry
```

预期：tab_bar 模块的 27 个测试（gen 13 + setters 14）+ tags 的 5 个新测试 + props_registry 的 5 个新测试全部通过。

### Phase B — `<tab_window>` 扩展 `<template slot="tabs">` 模板定制

#### B1. TabWindowShell 扩展 — [tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs)

**新增字段**（L153-171 结构体）：
```rust
pub struct TabWindowShell {
    // ... 现有 15 字段 ...
    tab_children: Vec<Tab>,   // 新增：模板定制模式，与 tabs 互斥
}
```

**`new()` 初始化**（L174-194）：`tab_children: Vec::new()`

**新增 setter**（紧邻 `tabs()` 方法 L221-224）：
```rust
/// 模板定制模式：直接注入 Tab 列表，绕过 TabItem 的 label/icon 限制。
/// 与 `tabs(Vec<TabItem>)` 互斥；非空时优先使用。
pub fn tab_children(mut self, children: Vec<Tab>) -> Self {
    self.tab_children = children;
    self
}
```

**`render` 方法调整**（L393-399 替换为分支）：
```rust
if !self.tab_children.is_empty() {
    // 模板定制模式：直接注入 Tab，绕过 TabItem 限制
    for tab in self.tab_children.drain(..) {
        tab_bar = tab_bar.child(tab);
    }
} else {
    // 简单模式：沿用 TabItem → Tab::new().label().icon()
    for tab in &self.tabs {
        let mut t = Tab::new().label(tab.label.clone());
        if let Some(icon) = tab.icon.clone() {
            t = t.icon(icon);
        }
        tab_bar = tab_bar.child(t);
    }
}
```

#### B2. codegen shell.rs 扩展 — [shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs)

**`partition_slot_children` 改为 7 元组**（L121-190）：

签名扩展：
```rust
pub(super) fn partition_slot_children(
    children: &[Node],
) -> (
    Option<Node>,  // slot_menu
    Option<Node>,  // slot_title
    Option<Node>,  // slot_footer
    Option<Node>,  // slot_left
    Option<Node>,  // slot_right
    Option<Node>,  // slot_bottom
    Vec<Node>,     // slot_tabs（新增：所有 <Tab> 子节点，非单 Node）
    Vec<Node>,     // body
)
```

`<template slot="tabs">` 识别逻辑（在现有 match 中新增 arm）：
```rust
"tabs" => {
    // 不取单一 content，而是收集所有子节点（应为 <Tab> 元素）
    let tab_kids: Vec<Node> = elem.children.iter().cloned().collect();
    if !tab_kids.is_empty() {
        slot_tabs = tab_kids;
    }
    continue;
}
```

**`gen_tab_window_wrapper` 新增参数**（L219-229）：

签名加 `slot_tabs: Option<&[String]>`：
```rust
pub(super) fn gen_tab_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slot_menu: Option<&str>,
    slot_title: Option<&str>,
    slot_footer: Option<&str>,
    slot_left: Option<&str>,
    slot_right: Option<&str>,
    slot_bottom: Option<&str>,
    slot_tabs: Option<&[String]>,   // ← 新增
) -> Result<String, CodegenError>
```

**生成 `.tab_children(vec![...])`**（在 slot_bottom 处理之后、`.child(children_body)` 之前）：
```rust
if let Some(tabs) = slot_tabs {
    if !tabs.is_empty() {
        let joined = tabs.join(", ");
        code.push_str(&format!(".tab_children(vec![{}])", joined));
    }
}
```

**互斥校验**：若 `slot_tabs` 非空且根元素同时有 `tabs={...}` bind 属性，输出编译错误：
```rust
let has_tabs_bind = elem.attributes.iter().any(|a| matches!(a, Attribute::Bind { name, .. } if name == "tabs"));
if slot_tabs.map_or(false, |t| !t.is_empty()) && has_tabs_bind {
    return Err(CodegenError {
        message: "<tab_window> 不能同时使用 `tabs={...}` 属性和 `<template slot=\"tabs\">` 插槽".into(),
    });
}
```

#### B3. render.rs slot 传递 — [render.rs:49-115](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs#L49-L115)

**解构 7 元组**：
```rust
let (slot_menu, slot_title, slot_footer, slot_left, slot_right, slot_bottom, slot_tabs, body_children) =
    if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
        shell::partition_slot_children(&elem.children)
    } else {
        (None, None, None, None, None, None, Vec::new(), elem.children.clone())
    };
```

**对 `slot_tabs` 中每个 `<Tab>` 子节点调 `tab_bar::tab::gen_tab_child`** 生成代码：
```rust
let slot_tabs_codes: Vec<String> = slot_tabs
    .iter()
    .map(|node| {
        if let Node::Element(tab_elem) = node {
            crate::compiler::tab_bar::tab::gen_tab_child(tab_elem, ctx, &mut id_counter, &empty)
        } else {
            Err(CodegenError {
                message: format!("<template slot=\"tabs\"> 仅支持 <Tab> 子节点，得到 {:?}", node),
            })
        }
    })
    .collect::<Result<Vec<_>, _>>()?;
let slot_tabs_ref: Option<Vec<String>> = if slot_tabs_codes.is_empty() {
    None
} else {
    Some(slot_tabs_codes)
};
```

**传给 `gen_tab_window_wrapper`**：
```rust
ShellWrap::Tab => shell::gen_tab_window_wrapper(
    elem,
    ctx,
    &body,
    slot_menu_code.as_deref(),
    slot_title_code.as_deref(),
    slot_footer_code.as_deref(),
    slot_left_code.as_deref(),
    slot_right_code.as_deref(),
    slot_bottom_code.as_deref(),
    slot_tabs_ref.as_deref(),   // ← 新增
)?,
```

#### B4. validator.rs — tabs slot 白名单

[validator.rs:87-107](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs#L87-L107) 当前仅校验用户自定义组件的 slot 名。shell 根标签的 slot 名未做白名单校验。

**新增 shell slot 白名单校验**（在 `validate_element` 中，`validate_unknown_props` 之前）：
```rust
// Shell 根标签的 slot 名白名单校验
if let Some(root_tag) = tags::root_tag_lookup(tag) {
    let allowed_slots: &[&str] = match root_tag {
        tags::RootTag::TabWindow => &[
            "menu", "title", "footer", "left", "right", "bottom", "tabs",
        ],
        tags::RootTag::ModernWindow => &["menu", "title", "footer"],
        _ => &[],
    };
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            if child_elem.tag == "template" {
                if let Some(slot_name) = &child_elem.slot_name {
                    if !allowed_slots.contains(&slot_name.as_str()) {
                        return Err(ValidationError {
                            message: format!(
                                "unknown slot name `{}` for <{}>: allowed slots are {:?}",
                                slot_name, tag, allowed_slots
                            ),
                        });
                    }
                }
            }
        }
    }
}
```

#### B5. codegen shell.rs 单元测试 — [shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) 末尾新增 `#[cfg(test)] mod tests`

参照 [accordion/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/gen.rs) 测试结构，至少覆盖：
- `partition_slot_children_extracts_tabs_slot` — `<template slot="tabs"><Tab /><Tab /></template>` 正确分到 slot_tabs
- `partition_slot_children_tabs_unknown_in_modern_window` — 在 modern_window 中 tabs slot 应落入 body（validator 会先报错）
- `gen_tab_window_wrapper_with_slot_tabs` — 生成 `.tab_children(vec![rml_ui::Tab::new()..., rml_ui::Tab::new()...])`
- `gen_tab_window_wrapper_tabs_mutual_exclusion_error` — 同时有 `tabs={...}` 与 `<template slot="tabs">` 报错

### Phase C — Demo 案例

#### C1. 新增 `demo/src/cases/tab_bar_case.rml`

参照 [accordion_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml) 结构（Card 包裹 + API 表 + 演示区），分 8 个 Card 演示：

1. **基础 Tabs**：`<TabBar selected_index={active_tab} on_click={on_tab_select}><Tab label="Account" /><Tab label="Profile" /></TabBar>`
2. **5 种 variant**：underline / pill / flat / outline / segmented 各一个 TabBar
3. **尺寸**：xsmall / small / large TabBar
4. **带图标**：`<Tab icon="User" label="Account" />`
5. **prefix/suffix**：`<TabBar prefix={prefix_btn} suffix={suffix_btn}>`
6. **禁用 + selected**：`<Tab disabled="true" /><Tab selected="true" />`
7. **menu 模式**：`<TabBar menu="true">` 多 Tab 演示下拉
8. **模板定制**（核心高级能力）：
   ```xml
   <TabBar selected_index={active_tab} on_click={on_tab_select}>
       <Tab>
           <Icon name="User" />
           <span>Account</span>
       </Tab>
       <Tab>
           <span>Profile</span>
           <Badge label="3" />
       </Tab>
   </TabBar>
   ```
9. **状态显示**：`<p>当前选中索引：{active_tab}</p>` 验证 `on_click(idx)` 事件回调

#### C2. 新增 `demo/src/cases/tab_bar_case.rml.rs`

参照 [accordion_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs)：

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.tab_bar",
    kind = "case",
    group = "components",
    order = 11,   // 紧随 accordion（order=10）之后
)]
#[component]
#[derive(Default)]
pub struct TabBarCase {
    pub active_tab: usize,
}

impl IContribution for TabBarCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.tab_bar.title").into() }
}

impl ILifecycle for TabBarCase {}

impl TabBarCase {
    #[computed]
    pub fn status_text(&self) -> String {
        format!("当前选中索引：{}", self.active_tab)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<TabBar selected_index={active_tab} on_click={on_tab_select}>
    <Tab label="Account" />
    <Tab label="Profile" />
</TabBar>"#.to_string()
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}
```

**注意 `on_tab_select` 签名**：`fn(&mut self, usize, &mut Context<Self>)` —— 对齐 TabBar 的 `on_click(idx: usize)` 决策（无 `&`，由 setter 内部 `*idx` 解引用）。

#### C3. 注册到 cases mod + catalog

**[demo/src/cases/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs)** 新增（紧随 `accordion_case` 之后）：
```rust
#[path = "tab_bar_case.rml.rs"]
pub mod tab_bar_case;
```

**[demo/src/cases/catalog.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs)** 的 `case_title_key` 新增 arm：
```rust
"components.tab_bar" => "case.tab_bar.title",
```

#### C4. i18n key 登记

[demo/assets/i18n/zh-CN.json](file:///d:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json) 与 [en-US.json](file:///d:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/en-US.json) 新增：
```json
"case.tab_bar.title": "标签栏 TabBar",
"case.tab_bar.basic": "基础用法",
"case.tab_bar.variants": "5 种 variant",
"case.tab_bar.sizes": "尺寸",
"case.tab_bar.with_icon": "带图标",
"case.tab_bar.prefix_suffix": "prefix / suffix",
"case.tab_bar.disabled": "禁用与选中",
"case.tab_bar.menu": "menu 模式（下拉）",
"case.tab_bar.template": "模板定制（高级）",
"case.tab_bar.status": "事件回调状态"
```

en-US.json 对应英文翻译。

### Phase D — 测试与回归

#### D1. engine 单元测试

```powershell
cargo test -p rust-rml-engine
```

预期：
- tab_bar 模块 27 个测试通过
- tags.rs 5 个新测试通过
- props_registry 5 个新测试通过
- codegen/shell.rs 新增的 4 个测试通过（B5）
- 现有所有测试无回归

#### D2. ui crate 编译

```powershell
cargo build -p rust-rml-ui
```

验证 TabWindowShell 的 `tab_children` 字段、setter、render 分支无破坏。

#### D3. demo 编译

```powershell
cargo build
```

验证 tab_bar_case.rml 与 .rml.rs 编译通过，cases/mod.rs 与 catalog.rs 注册一致。

#### D4. demo 运行 + 手动验证

```powershell
cargo run
```

在 case 列表中打开 "TabBar" 案例，验证：
- 5 种 variant 渲染正确（underline 下划线、pill 胶囊、flat 扁平、outline 描边、segmented 分段）
- 尺寸切换正常
- icon 显示
- prefix/suffix 显示
- disabled Tab 不可点击
- menu 模式下多 Tab 出现下拉
- 模板定制 Tab 内的 Icon + span + Badge 正确渲染
- 点击 Tab 触发 `on_tab_select`，状态文本更新

#### D5. 回归验证

- 现有 `<tab_window>` demo（[main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) 用 `tabs={tab_bar_items}`）行为不变
- accordion_case、avatar_case、card_case 等现有 case 不受影响
- 现有 `<tab_window>` 不使用 `<template slot="tabs">` 的写法仍正常

## Assumptions & Decisions

1. **code_editor 修复仅 1 行**：只补 `pub mod code_editor;` 到 `compiler/mod.rs`，不重构 code_editor 内部，不修改 component.rs 的 CodeEditor 分支。修复后 engine 应能编译；若 code_editor 内部还有其他错误，则属于 LSP feature 范畴，本规划不处理。
2. **`<template slot="tabs">` 与 `tabs={Vec<TabItem>}` 互斥**：codegen 在 shell.rs 检测到二者并存时报 `CodegenError`；validator 同时增加 shell slot 白名单（防止未知 slot 名静默落入 body）。
3. **`slot_tabs` 是 `Vec<Node>` 而非 `Option<Node>`**：与其他单 Node slot 不同，tabs 内可放多个 `<Tab>`，需要保留顺序与数量。
4. **`tab_children` 非空优先**：TabWindowShell::render 中 `tab_children` 非空时忽略 `tabs`，与 codegen 互斥校验形成双重保险（运行时 + 编译期）。
5. **`on_tab_select` 用户方法签名 `fn(&mut self, index: usize, ...)`**：与上一轮已批准的 `on_click(idx: usize)` 决策一致，setter 内部 `this.<method>(*idx, cx)` 解引用 `&usize`。
6. **不修改 TabItem**：TabItem 仍适用于 `#[computed] fn tab_bar_items() -> Vec<TabItem>` 简单场景（main_window.rml 现有用法），模板定制走 `<template slot="tabs">` 路径。
7. **不实现 track_scroll**：与上一轮决策一致，`track_scroll` 仅在 props_registry 占位，不实现声明式绑定。
8. **demo order=11**：紧随 accordion（order=10）之后，避免与现有 case 冲突。

## Verification Steps

1. `cargo build -p rust-rml-engine` 通过（code_editor 阻断解除）
2. `cargo test -p rust-rml-engine` 全绿（含 tab_bar 模块 27 个新测试 + shell.rs 4 个新测试 + tags/props_registry 10 个新测试）
3. `cargo build -p rust-rml-ui` 通过（TabWindowShell 扩展无破坏）
4. `cargo build` demo 通过
5. `cargo run` demo 启动，"TabBar" 案例可打开，5 种 variant、模板定制、事件回调按预期工作
6. main_window.rml 现有 `tabs={tab_bar_items}` 写法行为不变（回归）

## Implementation Order

1. **A-V1**（1 行）→ `cargo build -p rust-rml-engine` 验证阻断解除
2. **A-V2** → `cargo test -p rust-rml-engine --lib compiler::tab_bar` 验证 Phase A 测试全绿
3. **B1**（TabWindowShell 扩展）→ `cargo build -p rust-rml-ui` 验证
4. **B2-B3**（codegen shell.rs + render.rs）→ `cargo build -p rust-rml-engine` 验证
5. **B4**（validator.rs）→ `cargo test -p rust-rml-engine` 验证不回归
6. **B5**（shell.rs 单元测试）→ `cargo test -p rust-rml-engine --lib compiler::codegen::shell` 验证
7. **C1-C4**（demo 案例 + 注册 + i18n）→ `cargo build` 验证
8. **D1-D5**（完整测试 + 手动验证 + 回归）→ 全绿后任务完成

## Out of Scope

- code_editor 模块内部实现（属于 LSP feature 范畴）
- track_scroll 与 ScrollHandle 的声明式绑定
- with_variant={TabVariant::Pill} 直接 bind 枚举（用快捷方法替代）
- TabItem 结构修改（保留其 Clone 性质）
- TabBar 在 `<dialog>` / `<modern_window>` 内的特殊布局适配（无需求）
