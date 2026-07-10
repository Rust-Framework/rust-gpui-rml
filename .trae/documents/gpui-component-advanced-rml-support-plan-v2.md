# gpui-component 高级组件 RML 支持 — 实施计划 v2

> 基于当前代码状态验证，覆盖剩余 4 个组件：OtpInput（收尾）、VirtualList、Resizable、Settings。
> Phase 1 Rating 已完成，本计划从 Phase 2 收尾开始。

## 当前状态分析

### 已完成
- **Rating**：tags 路由、props 注册、`color` 委托（避免被 CSS 拦截）、demo case、i18n、UI re-export — 全部完成。

### Phase 2: OtpInput — 90% 完成，缺关键收尾
- ✅ `tags.rs` OtpInput 路由（Stateful, state_field="otp_state"）
- ✅ `props_registry.rs` 注册（length/groups/masked/default_value/on_change/on_focus/on_blur）
- ✅ `input/event.rs` `is_input_event` 扩展识别 OtpInput（on_change/on_focus/on_blur）
- ✅ `stateful.rs` `gen_stateful_body` 改为 `pub(crate)`，OtpInput 排除出 `StatefulComponentTranslator::matches`
- ✅ UI re-export（`otp_input.rs`, `components/mod.rs`, `lib.rs`）
- ✅ `compiler/components/otp_input/mod.rs` + `setters.rs`（有 bug）
- ✅ `compiler/components/mod.rs` 声明 `pub mod otp_input;`
- ✅ `setters.rs` OtpInput static_setter 委托已加
- ❌ **BUG**：`otp_input/setters.rs` L32 `super::super::setters::component_bind_rust_expr` 路径错误
- ❌ **缺失**：`setters.rs::component_bind_setter` 未加 OtpInput bind 委托
- ❌ **缺失**：`translator/component/otp_input.rs`（OtpInputTranslator）未创建
- ❌ **缺失**：`translator/component/mod.rs` 未注册 OtpInput
- ❌ **缺失**：demo case（`otp_input_case.rml.rs` + `otp_input_case.rml`）
- ❌ **缺失**：i18n 条目

### Phase 3-5: VirtualList / Resizable / Settings — 0% 完成

---

## Phase 2: OtpInput 收尾

### 2.1 修复 `otp_input/setters.rs` 路径 bug

**文件**：`crates/engine/src/compiler/components/otp_input/setters.rs` L32

**问题**：`super::super::setters::component_bind_rust_expr` 从 `compiler/components/otp_input/setters.rs` 出发：
- `super` = `compiler::components::otp_input`
- `super::super` = `compiler::components`（无 `setters` 模块）

**修复**：改为 `crate::compiler::setters::component_bind_rust_expr`

```rust
// L32 修改前
let rust_expr = super::super::setters::component_bind_rust_expr(expr_str, loop_vars, computed);
// 修改后
let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr_str, loop_vars, computed);
```

### 2.2 添加 OtpInput bind_setter 委托到 `setters.rs`

**文件**：`crates/engine/src/compiler/setters.rs`

在 `component_bind_setter` 函数中（L209+），在现有委托块之后、Breadcrumb 之前，添加 OtpInput 委托：

```rust
// OtpInput: groups={count} → .groups(self.count)
if let Some(s) = super::components::otp_input::setters::bind_setter(
    name, expr_str, loop_vars, computed, tag
) {
    return Some(s);
}
```

### 2.3 创建 OtpInputTranslator

**文件**：`crates/engine/src/compiler/translator/component/otp_input.rs`（新建）

**设计**：继承 StatefulComponentTranslator 模式，特化处理 `length`/`masked`/`default_value` 注入 state_ctor。

**核心逻辑**：
1. 从元素属性提取 `length`（默认 6）、`masked`（默认 false）、`default_value`（可选）
2. 构建自定义 state_ctor 字符串：
   ```
   |w, c| rml_ui::OtpState::new({length}usize, w, c).masked({masked}){.default_value("...")}
   ```
3. 调用 `gen_stateful_body(elem, &component, ref_name, "otp_state", &custom_state_ctor, loop_vars)`
4. 应用 CSS class 样式
5. 遍历属性应用 setter，但 **跳过** `length`/`masked`/`default_value`（已注入 state_ctor）

**参考模式**：`translator/component/stateful.rs` 的 `StatefulComponentTranslator::to_rust`

```rust
//! OtpInput 专用 translator —— 注入 length/masked/default_value 到 state_ctor
use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::{component_bind_setter, component_event_setter, component_static_setter};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct OtpInputTranslator;

impl IRmlTranslator for OtpInputTranslator {
    fn tag(&self) -> &'static str { "OtpInput" }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "OtpInput"
    }

    fn to_rust(&self, elem, ctx, _id_counter, loop_vars, parents) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;
        let resolved = tags::normalize_component_tag(tag);
        let component = tags::component_lookup_resolved(tag).ok_or_else(|| ...)?;

        let ref_name = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        // 提取 length/masked/default_value
        let length = extract_static_usize(elem, "length").unwrap_or(6);
        let masked = extract_static_bool(elem, "masked").unwrap_or(false);
        let default_value = extract_static_string(elem, "default_value");

        // 构建自定义 state_ctor
        let mut state_ctor = format!("|w, c| rml_ui::OtpState::new({}usize, w, c).masked({})", length, masked);
        if let Some(dv) = default_value {
            state_ctor.push_str(&format!(".default_value({:?})", dv));
        }

        // 调用 gen_stateful_body 生成构造表达式
        let mut code = super::stateful::gen_stateful_body(elem, &component, ref_name, "otp_state", &state_ctor, loop_vars)?;

        // CSS class 样式
        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        // 应用剩余 setter（跳过 length/masked/default_value）
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let skip = ["length", "masked", "default_value"];
        for attr in &elem.attributes {
            let name = attr.name();
            if skip.contains(&name) { continue; }
            match attr {
                Attribute::Static { name, value, .. } => {
                    if let Some(setter) = component_static_setter(name, value, &resolved) { code.push_str(&setter); }
                    else { crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "static")?; }
                }
                Attribute::Bind { name, expr, .. } => {
                    if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) { code.push_str(&setter); }
                    else { crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "bind")?; }
                }
                Attribute::Event { name, handler, .. } => {
                    if let Some(setter) = component_event_setter(name, handler, &resolved) { code.push_str(&setter); }
                }
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem, ctx) -> Result<String, PrintError> { super::super::utils::print_element(elem, ctx) }
    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("OtpInput", "OtpInput", ComponentCategory::Layout)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(OtpInputTranslator);
}
```

**注意**：已验证 `Attribute` 枚举无 `name()` 方法（ast.rs L55-77），需用 match 提取 name：

```rust
fn attr_name(attr: &Attribute) -> &str {
    match attr {
        Attribute::Static { name, .. } | Attribute::Bind { name, .. } | Attribute::Event { name, .. } => name,
    }
}
```

在 OtpInputTranslator 中用此 helper 判断是否跳过 `length`/`masked`/`default_value`。

### 2.4 注册 OtpInputTranslator

**文件**：`crates/engine/src/compiler/translator/component/mod.rs`

```rust
// L12 添加
pub mod otp_input;
// register_all 中添加（在 stepper 之后）
otp_input::register(registry);
```

### 2.5 创建 demo case

**文件**：
- `demo/src/cases/otp_input_case.rml.rs`（新建）
- `demo/src/cases/otp_input_case.rml`（新建）
- `demo/src/cases/mod.rs`（添加 `#[path = "otp_input_case.rml.rs"] pub mod otp_input_case;`）

**ViewModel**：
- `otp_value: String` — 接收 on_change 的值
- `case_doc_page: Option<Entity<CaseDocPage>>`
- `api_columns/api_rows` — API 表格
- `#[command] fn on_otp_change(&mut self, value: &SharedString, _cx: &mut Context<Self>)` — 接收 InputEvent::Change
- `#[contribute(...)]` order = 71（Rating 是 71，OtpInput 用 72）

**RML demo 内容**：
- 基础用法：`<otp-input length={6} on_change={on_otp_change} />`
- 分组：`<otp-input groups={2} length={6} />`
- 掩码：`<otp-input masked={true} />`
- 默认值：`<otp-input default-value="123456" />`
- 禁用：`<otp-input disabled={true} />`
- 尺寸：`<otp-input size="small" />`

### 2.6 i18n

**文件**：
- `demo/assets/i18n/zh-CN.json`：`"case.otp_input.title": "OTP 输入 OtpInput"`
- `demo/assets/i18n/en-US.json`：`"case.otp_input.title": "OtpInput"`

### 2.7 验证

```bash
cargo build -p rust-rml-engine
cargo build -p rust-rml-demo
cargo test -p rust-rml-engine -- otp_input
```

---

## Phase 3: VirtualList

### API 分析

```rust
// 构造函数（非 ::new）
pub fn v_virtual_list<R, V>(
    view: Entity<V>,
    id: impl Into<ElementId>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    f: impl Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>,
) -> VirtualList
// h_virtual_list 同理，Axis::Horizontal

// VirtualListScrollHandle
VirtualListScrollHandle::new()
.scroll_to_item(ix: usize, strategy: ScrollStrategy)
.scroll_to_bottom()

// VirtualList 实现 Styled trait
```

**难点**：
1. 构造器是函数 `v_virtual_list(...)` 而非 `VirtualList::new(id)`，需要特化 tag 路由
2. `view: Entity<V>` 参数 — 需传入当前 ViewModel 的 entity（`cx.entity()`）
3. `item_sizes: Rc<Vec<Size<Pixels>>>` — 需从 ViewModel 绑定或静态构造
4. 渲染闭包 `Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>` — 通过 `slot="render"` 模板注入

### 3.1 UI re-export

**文件**：`crates/ui/src/components/virtual_list.rs`（新建）

```rust
//! VirtualList 组件封装
pub use gpui_component::virtual_list::{VirtualList, VirtualListScrollHandle, v_virtual_list, h_virtual_list};
```

**文件**：`crates/ui/src/components/mod.rs` + `crates/ui/src/lib.rs` — 添加导出

### 3.2 tags.rs 路由

VirtualList 构造器是函数调用，非 `::new(id)` 模式。使用 `ComponentKind::EntityRef` 或新建一个 `ComponentKind::Special` 不可行。

**方案**：使用 `ComponentKind::Stateless` + 专用 translator 覆盖构造逻辑。tags.rs 注册：

```rust
// VirtualList：虚拟列表，构造器 v_virtual_list/h_virtual_list 函数
// 由 VirtualListTranslator 特化处理 slot="render" 闭包注入
"VirtualList" | "virtual-list" => Some(ComponentTag {
    ctor_path: "rml_ui::v_virtual_list",
    kind: ComponentKind::Stateless,
    container: false,
}),
```

### 3.3 props_registry.rs

```rust
("VirtualList", &["direction", "item_sizes", "on_scroll"]),
```

### 3.4 VirtualListTranslator

**文件**：
- `crates/engine/src/compiler/components/virtual_list/mod.rs`（新建）
- `crates/engine/src/compiler/components/virtual_list/gen.rs`（新建）
- `crates/engine/src/compiler/translator/component/virtual_list.rs`（新建）

**gen_virtual_list 核心逻辑**：
1. 从 `direction` 属性决定 `v_virtual_list` 或 `h_virtual_list`（默认 vertical）
2. 从 `item_sizes` 属性（bind 表达式）获取 `Rc<Vec<Size<Pixels>>>`
3. 从 `<template slot="render">` 子节点提取渲染闭包
4. 生成代码：
```rust
rml_ui::v_virtual_list(
    cx.entity(),
    ("rml_vlist", N),
    self.item_sizes.clone(),
    |this: &mut Self, range, window, cx| {
        // slot="render" 模板展开
        range.map(|i| { ... }).collect()
    }
)
```

**注意**：闭包签名 `Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>)` 中 V = 当前 ViewModel 类型，闭包需捕获 self 引用。这是 RML slot 机制的变体。

### 3.5 demo case

**文件**：`demo/src/cases/virtual_list_case.rml.rs` + `.rml`

展示 1000 行虚拟列表，含滚动到指定项、水平/垂直方向。

### 3.6 验证

```bash
cargo build -p rust-rml-engine && cargo build -p rust-rml-demo
```

---

## Phase 4: Resizable

### API 分析

```rust
// 构造函数（非 ::new）
h_resizable(id) -> ResizablePanelGroup  // 默认 Axis::Horizontal
v_resizable(id) -> ResizablePanelGroup
resizable_panel() -> ResizablePanel

// ResizablePanelGroup
.with_state(&Entity<ResizableState>)
.axis(Axis)
.child(ResizablePanel)
.children(iter)
.size(Pixels)
.on_resize(Fn(&Entity<ResizableState>, &mut Window, &mut App))

// ResizablePanel（实现 Styled + ParentElement）
.size(impl Into<Pixels>)
.size_range(impl Into<Range<Pixels>>)
.visible(bool)

// ResizableState: Entity, EventEmitter<ResizablePanelEvent::Resized>
ResizableState::new(...)
.resize_panel(ix, size, window, cx)
```

**难点**：
1. 构造器是 `h_resizable(id)` 函数，非 `Resizable::new(id)`
2. 子节点是 `ResizablePanel`，通过 `resizable_panel()` 构造，非 `::new(id)`
3. Panel 的子节点是任意元素（实现 ParentElement）
4. 需要支持 `ref` + `ResizableState` entity 管理

### 4.1 UI re-export

**文件**：`crates/ui/src/components/resizable.rs`（新建）

```rust
pub use gpui_component::resizable::{
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent,
    h_resizable, v_resizable, resizable_panel,
};
```

### 4.2 tags.rs 路由

```rust
// Resizable：可调整面板组，构造器 h_resizable(id)/v_resizable(id)
// 子节点为 <resizable-panel>
"Resizable" | "resizable" => Some(ComponentTag {
    ctor_path: "rml_ui::h_resizable",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
// ResizablePanel：面板，构造器 resizable_panel()，实现 ParentElement
"ResizablePanel" | "resizable-panel" => Some(ComponentTag {
    ctor_path: "rml_ui::resizable_panel",
    kind: ComponentKind::Stateless,
    container: true,
}),
```

**`is_item_builder_tag` 扩展**：添加 `"ResizablePanel" | "resizable-panel"`

### 4.3 ResizableTranslator

**文件**：
- `crates/engine/src/compiler/components/resizable/mod.rs`
- `crates/engine/src/compiler/components/resizable/gen.rs`
- `crates/engine/src/compiler/components/resizable/panel.rs`（Panel 构造 + setter）
- `crates/engine/src/compiler/translator/component/resizable.rs`

**gen_resizable 核心逻辑**：
1. 从 `direction` 属性决定 `h_resizable` 或 `v_resizable`（默认 horizontal）
2. 生成 `rml_ui::h_resizable(("rml_resizable", N))`
3. 若有 `ref="name"`，生成 `.with_state(&self.__rml_state.get_or_init_ref("name", w, c, |w, c| rml_ui::ResizableState::new(...)))`
4. 遍历子节点，对每个 `<resizable-panel>` 生成 `.child(resizable_panel().size(...).child(...))`
5. 应用 `on_resize` 事件 setter

### 4.4 props_registry.rs

```rust
("Resizable", &["direction", "size", "on_resize"]),
("ResizablePanel", &["size", "size_range", "visible"]),
```

### 4.5 demo case

展示水平/垂直 ResizablePanelGroup，含 Panel 尺寸、范围限制、可见性切换。

### 4.6 验证

```bash
cargo build -p rust-rml-engine && cargo build -p rust-rml-demo
```

---

## Phase 5: Settings

### API 分析（最复杂）

```
Settings (顶层)
  .page(SettingPage)              // 添加页面
  .sidebar_width(Pixels)
  .with_group_variant(GroupBoxVariant)
  .default_selected_index(usize)
  .sidebar_style(&StyleRefinement)
  .header_style(&StyleRefinement)

SettingPage::new(title)
  .title(title)
  .icon(Icon)
  .description(desc)
  .default_open(bool)
  .resettable(bool)
  .group(SettingGroup)
  .groups(iter)

SettingGroup::new()
  .title(title)
  .description(desc)
  .item(SettingItem)
  .items(iter)

SettingItem::new(title, field: impl AnySettingField)  // 字段型
SettingItem::render(closure)                          // 自定义元素型
  .on_reset(is_dirty, reset)
```

**难点**：
1. 四层嵌套：Settings > Page > Group > Item
2. SettingItem 有两种形态：`Item { title, field }` 和 `Element { render closure }`
3. `field` 需实现 `AnySettingField` trait（BoolField/DropdownField/NumberField/StringField）
4. `SettingItem::render` 需要 `Fn(&RenderOptions, &mut Window, &mut App) -> AnyElement` 闭包

### 5.1 设计方案

采用 **结构化数据 + slot 模板** 混合方案：

```rml
<settings sidebar-width={250}>
  <template slot="page" title="常规" icon="settings">
    <setting-group title="外观">
      <setting-item title="主题" field-type="dropdown" options={themes} value={current_theme} on-change={on_theme_change} />
      <setting-item title="启用通知" field-type="bool" value={notifications_enabled} on-change={on_notify_change} />
      <setting-item title="字号" field-type="number" min={12} max={24} value={font_size} on-change={on_font_change} />
      <template slot="item" title="自定义">
        <!-- 自定义 render 内容 -->
      </template>
    </setting-group>
  </template>
</settings>
```

### 5.2 UI re-export

**文件**：`crates/ui/src/components/settings.rs`（新建）

```rust
pub use gpui_component::setting::{
    Settings, SettingPage, SettingGroup, SettingItem,
    SettingsState, RenderOptions,
};
```

### 5.3 tags.rs 路由

```rust
// Settings：设置页容器，多层嵌套
"Settings" | "settings" => Some(ComponentTag {
    ctor_path: "rml_ui::Settings",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
// SettingPage：页面
"SettingPage" | "setting-page" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingPage",
    kind: ComponentKind::Stateless,
    container: false,
}),
// SettingGroup：分组
"SettingGroup" | "setting-group" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingGroup",
    kind: ComponentKind::Stateless,
    container: false,
}),
// SettingItem：设置项
"SettingItem" | "setting-item" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingItem",
    kind: ComponentKind::Stateless,
    container: false,
}),
```

**`is_item_builder_tag` 扩展**：添加 `"SettingPage" | "setting-page" | "SettingGroup" | "setting-group" | "SettingItem" | "setting-item"`

### 5.4 SettingsTranslator

**文件**：
- `crates/engine/src/compiler/components/settings/mod.rs`
- `crates/engine/src/compiler/components/settings/gen.rs`（Settings 构造 + Page 收集）
- `crates/engine/src/compiler/components/settings/page.rs`（Page 构造）
- `crates/engine/src/compiler/components/settings/group.rs`（Group 构造）
- `crates/engine/src/compiler/components/settings/item.rs`（Item 构造 + field 生成）
- `crates/engine/src/compiler/translator/component/settings.rs`

**gen_settings 核心逻辑**：
1. 生成 `rml_ui::Settings::new(("rml_settings", N))`
2. 遍历 `<template slot="page">` 子节点，为每个生成 `SettingPage::new(title).icon(...).group(...)` 并 `.page(setting_page)`
3. 应用 `sidebar_width`/`group_variant`/`default_selected_index` setter

**gen_setting_item 核心逻辑**：
1. 从 `field-type` 属性决定 field 类型（bool/dropdown/number/string）
2. 生成 `rml_ui::SettingItem::new("title", rml_ui::BoolField::new(...).value(...).on_change(...))`
3. 若有 `<template slot="item">`，生成 `SettingItem::render(|opts, w, cx| { ... })`

### 5.5 Field 类型支持

需为 4 种 field 类型实现 setter 映射：

| field-type | Rust 类型 | 属性 |
|-----------|----------|------|
| bool | `BoolField` | value(bool), on_change(Fn(bool)) |
| dropdown | `DropdownField` | options(Vec), value(usize), on_change(Fn(usize)) |
| number | `NumberField` | min, max, value, on_change(Fn(f64)) |
| string | `StringField` | value(String), on_change(Fn(String)) |

### 5.6 props_registry.rs

```rust
("Settings", &["sidebar_width", "group_variant", "default_selected_index"]),
("SettingPage", &["title", "icon", "description", "default_open", "resettable"]),
("SettingGroup", &["title", "description"]),
("SettingItem", &["title", "description", "field_type", "value", "options", "min", "max", "on_change"]),
```

### 5.7 demo case

展示完整设置面板：多页面、多分组、多种 field 类型。

### 5.8 验证

```bash
cargo build -p rust-rml-engine && cargo build -p rust-rml-demo
```

---

## 假设与决策

### 假设
1. **Attribute::name() 方法**：已验证 `Attribute` 枚举无 `name()` 方法（ast.rs L55-77），translator 内用 match 提取 name（见 2.3）。
2. **cx.entity() 可用性**：VirtualList 需要 `Entity<V>` 参数，假设 codegen 上下文中可用 `cx.entity()` 获取当前 ViewModel entity。
3. **slot="render" 模板机制**：假设 RML 已有 slot 模板支持（Tabs/CodeEditor 已用），可复用于 VirtualList 闭包和 Settings 自定义 Item。
4. **SettingField 类型路径**：假设 `BoolField`/`DropdownField`/`NumberField`/`StringField` 在 `gpui_component::setting::fields` 模块。

### 决策
1. **VirtualList 构造器**：使用 `ComponentKind::Stateless` + 专用 translator 覆盖，避免新增 ComponentKind 变体
2. **Resizable 构造器**：同上，`StatelessWithItems` + 专用 translator 处理函数构造器
3. **Settings 多层嵌套**：使用 `StatelessWithItems` + slot 模板，子节点通过 `is_item_builder_tag` 识别
4. **kebab-case 属性**：所有声明式属性使用 kebab-case（`sidebar-width`, `field-type`, `default-open` 等）
5. **vertical 默认 horizontal**：Resizable/VirtualList 的 direction 属性默认 horizontal，仅 `direction="vertical"` 时切换

## 实施顺序

1. **Phase 2 OtpInput 收尾**（2.1-2.7）→ 验证：cargo build + test
2. **Phase 3 VirtualList**（3.1-3.6）→ 验证：cargo build
3. **Phase 4 Resizable**（4.1-4.6）→ 验证：cargo build
4. **Phase 5 Settings**（5.1-5.8）→ 验证：cargo build

每个 Phase 完成后独立验证，确保增量可构建。

## 验证清单

- [ ] Phase 2: `cargo build -p rust-rml-engine` 成功，OtpInput demo 可用
- [ ] Phase 2: `cargo test -p rust-rml-engine -- otp_input` 通过
- [ ] Phase 3: VirtualList demo 可渲染 1000 项虚拟列表
- [ ] Phase 4: Resizable demo 可拖拽调整面板大小
- [ ] Phase 5: Settings demo 展示多页面、多分组、多 field 类型
- [ ] 所有组件：属性、样式、事件、绑定能力齐全
- [ ] 无 `cargo clippy` 警告
