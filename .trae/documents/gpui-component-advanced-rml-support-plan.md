# gpui-component 高级组件 RML 支持 — 实施计划

> 关联文档：
> - 设计文档：[gpui-component-advanced-full-coverage-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/gpui-component-advanced-full-coverage-plan.md)（17 组件全量覆盖设计）
> - 执行计划 v1：[gpui-component-advanced-execution-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/gpui-component-advanced-execution-plan.md)（已批准，Phase 1 部分完成）
>
> 本计划为 v2，基于当前代码实际状态重新校准，聚焦 Phase A（5 核心组件）的可执行步骤。
> gpui-component git rev：063e55bbc4fb13907a988111e3581595cbcaefde（v0.5.2）

## Summary

为 RML 声明式框架全面支持 5 个用户点名的 gpui-component 高级组件：**Rating / OtpInput / VirtualList / Resizable / Settings**。每个组件需完整覆盖属性、样式、主题、事件、绑定能力，禁止遗漏。

Phase B/C/D/E（Select / ComboBox / ColorPicker / DatePicker / Notification / Sheet / FocusTrap / HoverCard / Sidebar / Dock / SearchableList / Chart / Plot 共 12 个）在设计文档中已有框架，Phase A 完成后按序推进，本计划不展开。

## Current State Analysis（当前状态校准）

### Phase 1: Rating — 部分完成

| 文件 | 状态 | 说明 |
|------|------|------|
| [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) L508-514 | ✅ 已注册 | `"Rating"` → `ComponentKind::Stateless`, `ctor_path: "rml_ui::Rating"` |
| [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) L199 | ✅ 已注册 | `("Rating", &["value", "max", "color", "on_click"])` |
| [setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs) L98-100 | ✅ 部分完成 | `value` / `max` static setter 已有 |
| [setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs) L465+ | ✅ 已完成 | `on_click` event setter 已有（`&usize` 参数模式） |
| [setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs) | ❌ 缺失 | `color` static setter 未注册 — 被 `apply_style_attr`（L48）拦截生成 `.text_color()` 而非 `.color()` |
| [crates/ui/src/components/rating.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/rating.rs) | ✅ 已存在 | `pub use gpui_component::rating::Rating;` |
| demo/src/cases/rating_case.* | ❌ 未创建 | 无 demo case |
| translator | ✅ 无需新增 | 使用通用 `StatelessComponentTranslator` |

**Rating 待完成工作**：
1. 修复 `color` 属性 — 在 `component_static_setter` 的组件委托区（apply_style_attr 之前）添加 Rating 专属 color 处理
2. 创建 demo case（.rml.rs + .rml）
3. 注册 demo case 到 [mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs)
4. 添加 i18n 条目

### Phase 2-5: OtpInput / VirtualList / Resizable / Settings — 均未开始

所有 4 个组件均无：UI 封装层、编译器模块、标签路由、属性注册、translator、demo case。

### 既有可复用模式（经实际验证）

| 模式 | 参考实现 | 适用组件 |
|------|---------|---------|
| Stateless 通用 translator + setter 分发 | [stateless.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateless.rs) | Rating |
| Stateful + InputEvent 事件订阅 | [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs) | OtpInput |
| 专属 translator（特殊构造） | [stepper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stepper.rs) | VirtualList |
| StatelessWithItems + .item() 子项 | [accordion/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/) | Resizable / Settings |
| slot 路由（trigger/content/render） | [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) | VirtualList(slot=render) / Settings(slot=field) |

---

## Proposed Changes

### Phase 1: Rating 修复 + Demo（最简，验证属性映射流程）

#### 1.1 修复 `color` 属性拦截问题

**文件**：[setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs)
**位置**：`component_static_setter` 函数，在 tooltip 委托（L43-45）之后、`apply_style_attr`（L48）之前
**改动**：添加 Rating 专属 color 委托

```rust
// Rating: color="red" → .color(cx.theme().red)，支持主题色名
// 必须在 apply_style_attr 之前处理，避免被 CSS color 属性拦截生成 .text_color()
if tag == "Rating" && name == "color" {
    return Some(format!(".color(cx.theme().{})", value));
}
```

**原因**：Rating 的 `.color(impl Into<Hsla>)` 设置星标激活色，与 CSS `color`（映射 `.text_color()`）语义不同。`apply_style_attr` 在 match 块之前执行，会拦截所有 `color` 属性。必须在委托区提前返回。

#### 1.2 创建 Demo Case

**文件 1**：`demo/src/cases/rating_case.rml.rs`（新建）
- 参考 [stepper_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/stepper_case.rml.rs) 模式
- ViewModel: `rating_value: usize`, `max_stars: usize`, `is_readonly: bool`
- `#[contribute(...)]` order 接 stepper 之后（71）
- `#[command]` 事件处理: `fn on_rating_change(&mut self, value: &usize, cx: &mut Context<Self>)`
- API 表格列出: value / max / color / size / disabled / on_click

**文件 2**：`demo/src/cases/rating_case.rml`（新建）
- 展示：基础评分（value 绑定）、10 星上限（max=10）、disabled、自定义颜色（color="red"）、size 变体

**文件 3**：[demo/src/cases/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 注册
```rust
#[path = "rating_case.rml.rs"]
pub mod rating_case;
```

**文件 4**：i18n 条目
- [zh-CN.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json): `"case.rating.title": "星级评分 Rating"`
- [en-US.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/en-US.json): `"case.rating.title": "Rating"`

#### 1.3 验证
- `cargo build -p rust-rml-ui` 成功
- `cargo build -p rust-rml-engine` 成功
- `cargo test -p rust-rml-engine` 全量通过
- `cargo build -p rust-rml-demo` 成功
- demo 中 rating_case 可独立运行

---

### Phase 2: OtpInput（Stateful + state_ctor 注入 + 事件订阅）

**API 调研**（[otp_input.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/input/otp_input.rs)）：

- `OtpState::new(length: usize, window: &mut Window, cx: &mut Context<Self>) -> Self`
- `OtpState` 方法：`.default_value(SharedString)` / `.masked(bool)` / `.set_value(...)` / `.value()` / `.focus(...)`
- `OtpState: EventEmitter<InputEvent>` — Change / Focus / Blur
- `OtpState: Focusable` / `Render`（Entity 类型）
- `OtpInput::new(state: &Entity<OtpState>)` — 构造接受 Entity 引用
- `OtpInput` builder：`.groups(usize)` — 默认 2
- 实现 `Disableable` / `Sizable` / `RenderOnce` trait

#### 2.1 UI 封装层

**文件**：`crates/ui/src/components/otp_input.rs`（新建）
```rust
//! OtpInput 组件封装 —— 基于 gpui-component 的 OtpInput
pub use gpui_component::input::{OtpInput, OtpState};
```

**文件**：[crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs)
- 添加 `pub mod otp_input;`

**文件**：[crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)
- 确认 OtpInput / OtpState 通过 `pub use` 或 `pub mod` 可从 `rml_ui::` 路径访问

#### 2.2 编译器模块

**新建目录**：`crates/engine/src/compiler/components/otp_input/`

**文件 1**：`mod.rs` — 仅 re-export
```rust
pub mod gen;
pub mod setters;
pub mod event;
pub use gen::gen_otp_input;
```

**文件 2**：`gen.rs`
- 参考 [input/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/gen.rs) + [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs)
- **state_ctor 占位符注入**：`length` 属性值替换 state_ctor 中的 `__RML_OTP_LENGTH__`
  - `length="6"` → state_ctor 闭包 `|w, c| rml_ui::OtpState::new(6usize, w, c)`
- **masked / default_value 注入 state_ctor**：
  - `masked` → `|w, c| rml_ui::OtpState::new(6usize, w, c).masked(true)`
  - `default_value="123456"` → `.default_value("123456")` 追加到 state_ctor
- **事件订阅 block**：`on_change` / `on_focus` / `on_blur` → `cx.subscribe(&otp_state, move |this, state, window, cx| { ... })`
- **groups / size / disabled** → 走 builder 链 setter

**文件 3**：`setters.rs`
- `groups="2"` → `.groups(2usize)`（static）
- `length` / `masked` / `default_value` → 不生成 setter（已注入 state_ctor）

**文件 4**：`event.rs`
- 参考 [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs)
- `is_otp_input_event(name) -> bool`
- `gen_otp_input_event_subscribe(events, ref_name) -> String` — 生成 cx.subscribe block

#### 2.3 标签路由

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
"OtpInput" | "otp-input" => Some(ComponentTag {
    ctor_path: "rml_ui::OtpInput",
    kind: ComponentKind::Stateful {
        state_field: "otp_state",
        state_ctor: "|w, c| rml_ui::OtpState::new(__RML_OTP_LENGTH__, w, c)",
    },
    container: false,
}),
```

#### 2.4 属性注册

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
("OtpInput", &["length", "groups", "masked", "default_value", "on_change", "on_focus", "on_blur"]),
```

#### 2.5 codegen 路由

**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs) — `pub mod otp_input;`

**文件 2**：`crates/engine/src/compiler/translator/component/otp_input.rs`（新建）
- 继承 StatefulComponentTranslator 模式，特化处理 length/masked/default_value 注入 state_ctor
- 在 [translator/component/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/mod.rs) `register_all` 注册

#### 2.6 Demo Case

**文件**：`demo/src/cases/otp_input_case.rml.rs` + `.rml`
- 展示：6 位 SMS 验证码、4 位 PIN（masked）、groups=1、on_change 验证、disabled 锁定

#### 2.7 验证
- state_ctor 占位符替换正确
- cx.subscribe block 生成正确
- `cargo test -p rust-rml-engine` 通过

---

### Phase 3: VirtualList（专属 translator + slot=render 闭包）

**API 调研**（[virtual_list.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/virtual_list.rs)）：

- `v_virtual_list(view: Entity<V>, id, item_sizes: Rc<Vec<Size<Pixels>>>, f: Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>)` — 垂直
- `h_virtual_list(...)` — 水平
- `VirtualList` 实现 `Styled` trait（可链式样式）
- builder：`.with_sizing_behavior(ListSizingBehavior)` 
- `VirtualListScrollHandle::new()` / `.scroll_to_item(ix, ScrollStrategy)` / `.scroll_to_bottom()` / `.base_handle()`
- 闭包签名：`Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>` where `R: IntoElement, V: Render`

#### 3.1 UI 封装层

**文件**：`crates/ui/src/components/virtual_list.rs`（新建）
```rust
pub use gpui_component::virtual_list::{
    v_virtual_list, h_virtual_list, virtual_list,
    VirtualList, VirtualListScrollHandle,
};
pub use gpui::{ListSizingBehavior, ScrollStrategy};
```

#### 3.2 编译器模块

**新建目录**：`crates/engine/src/compiler/components/virtual_list/`

**文件 1**：`mod.rs` — re-export

**文件 2**：`gen.rs`
- **函数式构造**：根据 `vertical` 属性选择 `v_virtual_list` / `h_virtual_list`（默认 horizontal）
- **构造签名**：
  ```rust
  rml_ui::v_virtual_list(
      cx.entity().clone(),
      ("rml_el", N),
      self.item_sizes.clone(),
      move |this, range, _window, cx| { /* slot=render 闭包体 */ }
  )
  ```
- **slot=render 处理**：参考 [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 slot 路由
  - `<template slot="render" let="range">` 子节点生成闭包体
  - `let="range"` 注入 `range: Range<usize>` 参数
  - 闭包内 `each={ix in range.start..range.end}` 迭代
- **item_sizes 绑定**：`item-sizes={self.item_sizes}` → `self.item_sizes.clone()`
- **scroll_handle 绑定**：`scroll-handle={self.scroll_handle}` → `.track_scroll(&self.scroll_handle)`（注：VirtualList 构造时已内置 scroll_handle，需确认是否可通过 builder 覆盖）

**文件 3**：`setters.rs`
- `vertical` → 选择构造器（在 gen.rs 处理，不生成 setter）
- `sizing="auto"` → `.with_sizing_behavior(rml_ui::ListSizingBehavior::Auto)`
- `sizing="infer"` → `.with_sizing_behavior(rml_ui::ListSizingBehavior::Infer)`

#### 3.3 标签路由

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
"VirtualList" | "virtual-list" => Some(ComponentTag {
    ctor_path: "rml_ui::v_virtual_list",  // 实际由 translator 选择 v/h
    kind: ComponentKind::Stateless,  // 占位，实际由专属 translator 处理
    container: false,
}),
```

#### 3.4 属性注册

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
("VirtualList", &["vertical", "item_sizes", "scroll_handle", "sizing"]),
```

#### 3.5 专属 Translator

**文件**：`crates/engine/src/compiler/translator/component/virtual_list.rs`（新建）
- 专属 translator，matches 优先于 StatelessComponentTranslator
- 处理函数式构造 + slot=render 闭包
- 在 `register_all` 注册

#### 3.6 Demo Case

**文件**：`demo/src/cases/virtual_list_case.rml.rs` + `.rml`
- 展示：1000 项垂直列表、编程式滚动到第 100 项、horizontal 卡片列表

#### 3.7 验证
- slot=render 闭包生成正确（`range: Range<usize>` 参数注入）
- `vertical` 属性正确选择构造器
- `cargo test -p rust-rml-engine` 通过

---

### Phase 4: Resizable（StatelessWithItems + Panel 子项）

**API 调研**（[resizable/mod.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/resizable/mod.rs) + [panel.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/resizable/panel.rs)）：

- `h_resizable(id) -> ResizablePanelGroup` / `v_resizable(id)` — 快捷构造
- `resizable_panel() -> ResizablePanel` — Panel 子项构造
- `ResizablePanelGroup::new(id)` builder：`.axis(Axis)` / `.child(ResizablePanel)` / `.with_state(&Entity<ResizableState>)` / `.on_resize(Fn(&Entity<ResizableState>, &mut Window, &mut App))`
- `ResizablePanel::new()` builder：`.visible(bool)` / `.size(Pixels)` / `.size_range(Range<Pixels>)` — 实现 `Styled` + `ParentElement`
- `ResizableState` — Entity, EventEmitter<ResizablePanelEvent::Resized>
- `ResizableState` 方法：`.sizes()` / `.resize_panel(ix, size, window, cx)`

#### 4.1 UI 封装层

**文件**：`crates/ui/src/components/resizable.rs`（新建）
```rust
pub use gpui_component::resizable::{
    h_resizable, v_resizable, resizable_panel,
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent,
};
```

#### 4.2 编译器模块

**新建目录**：`crates/engine/src/compiler/components/resizable/`

**文件 1**：`mod.rs` — re-export

**文件 2**：`gen.rs`
- 参考 [accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/gen.rs) + [stepper/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/gen.rs)
- **构造器**：根据 `vertical` 属性选择 `h_resizable(id)` / `v_resizable(id)`（默认 horizontal）
- **子项处理**：`<resizable-panel>` → `.child(resizable_panel().size(...).child(...))`
- **on_resize 事件**：`Rc<dyn Fn(&Entity<ResizableState>, &mut Window, &mut App)>` 直接构造（需访问 window，不能用 cx.listener）

**文件 3**：`setters.rs`
- Resizable: `size="220"` → `.size(px(220.))`（static）
- ResizablePanel: `size="220"` → `.size(px(220.))`，`min-size="100"` + `max-size="400"` → `.size_range(px(100.)..px(400.))`，`visible="false"` → `.visible(false)`

#### 4.3 标签路由

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
"Resizable" | "resizable" => Some(ComponentTag {
    ctor_path: "rml_ui::h_resizable",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
"ResizablePanel" | "resizable-panel" => Some(ComponentTag {
    ctor_path: "rml_ui::resizable_panel",
    kind: ComponentKind::StatelessNoId,
    container: true,
}),
```

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `is_item_builder_tag` — 添加 `"ResizablePanel" | "resizable-panel"`

#### 4.4 属性注册

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
("Resizable", &["vertical", "size", "on_resize"]),
("ResizablePanel", &["size", "min_size", "max_size", "visible"]),
```

#### 4.5 Translator

**文件**：`crates/engine/src/compiler/translator/component/resizable.rs`（新建）
- StatelessWithItems translator，参考 [accordion.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/accordion.rs)
- 在 `register_all` 注册

#### 4.6 Demo Case

**文件**：`demo/src/cases/resizable_case.rml.rs` + `.rml`
- 展示：三栏布局（220/自适应/280）+ min/max 限制 + vertical 布局 + on_resize 持久化

#### 4.7 验证
- `is_item_builder_tag("resizable-panel")` 返回 true
- size_range 正确生成（`px(100.)..px(400.)`）
- `cargo test -p rust-rml-engine` 通过

---

### Phase 5: Settings（多层嵌套 + slot=field）

**API 调研**（[setting/settings.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/settings.rs) + page.rs + group.rs + item.rs）：

层级：`Settings → SettingPage → SettingGroup → SettingItem`

- `Settings::new(id)` builder：`.page(SettingPage)` / `.pages(I)` / `.sidebar_width(Pixels)` / `.with_group_variant(GroupBoxVariant)` / `.default_selected_index(SelectIndex)` / `.sidebar_style(&StyleRefinement)` / `.header_style(&StyleRefinement)`
- `SettingPage::new(title)` builder：`.title(...)` / `.icon(Icon)` / `.description(...)` / `.default_open(bool)` / `.resettable(bool)` / `.group(SettingGroup)` / `.groups(I)`
- `SettingGroup::new()` builder：`.title(...)` / `.description(...)` / `.item(SettingItem)` / `.items(I)` — 实现 `Styled`
- `SettingItem::new<F: AnySettingField>(title, field)` / `SettingItem::render<R>(closure)` builder：`.description(Text)` / `.keywords(Vec<SharedString>)` / `.layout(Axis)` / `.disabled(bool)` / `.on_reset(is_dirty, reset_closure)`

#### 5.1 UI 封装层

**文件**：`crates/ui/src/components/settings.rs`（新建）
```rust
pub use gpui_component::setting::{
    Settings, SettingPage, SettingGroup, SettingItem, SelectIndex,
};
```

#### 5.2 编译器模块

**新建目录**：`crates/engine/src/compiler/components/settings/`

**文件 1**：`mod.rs` — re-export

**文件 2**：`gen.rs`
- 参考 [accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/gen.rs) 多层 item 模式
- **Settings 容器**：`Settings::new(id).sidebar_width(...).page(SettingPage::new(...).group(...))`
- **SettingPage**：`SettingPage::new(title).group(SettingGroup::new().item(SettingItem::new(title, field)))`
- **SettingGroup**：`SettingGroup::new().item(SettingItem::new(...))`
- **SettingItem**：`SettingItem::new(title, field)` 或 `SettingItem::render(closure)`（slot=field）

**文件 3**：`setters.rs`
- Settings: `sidebar_width="280"` → `.sidebar_width(px(280.))`，`default_selected_index="0"` → `.default_selected_index(rml_ui::SelectIndex::new(0))`，`group_variant="fill"` → `.with_group_variant(GroupBoxVariant::Fill)`
- SettingPage: `icon="Settings"` → `.icon(rml_ui::Icon::new(rml_ui::IconName::Settings))`，`default_open` → `.default_open(true)`，`resettable` → `.resettable(true)`
- SettingGroup: `title` → `.title(...)`，`description` → `.description(...)`
- SettingItem: `field={theme_field}` → bind（`self.theme_field.clone()`），`description` → `.description(...)`，`disabled` → `.disabled(true)`

**文件 4**：`item.rs`
- **field 绑定**：`field={theme_field}` → `SettingItem::new("主题", self.theme_field.clone())`
- **slot=field 自定义**：`<template slot="field">` → `SettingItem::render(move |options, window, cx| { <自定义元素> })`

#### 5.3 标签路由

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
"Settings" | "settings" => Some(ComponentTag {
    ctor_path: "rml_ui::Settings",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
"SettingPage" | "setting-page" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingPage",
    kind: ComponentKind::StatelessNoId,
    container: false,
}),
"SettingGroup" | "setting-group" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingGroup",
    kind: ComponentKind::StatelessNoId,
    container: false,
}),
"SettingItem" | "setting-item" => Some(ComponentTag {
    ctor_path: "rml_ui::SettingItem",
    kind: ComponentKind::StatelessNoId,
    container: false,
}),
```

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `is_item_builder_tag` — 添加 `"SettingPage" | "setting-page" | "SettingGroup" | "setting-group" | "SettingItem" | "setting-item"`

#### 5.4 属性注册

**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
("Settings", &["sidebar_width", "group_variant", "default_selected_index", "on_reset"]),
("SettingPage", &["title", "icon", "description", "default_open", "resettable"]),
("SettingGroup", &["title", "description"]),
("SettingItem", &["title", "field", "description", "layout", "disabled", "keywords", "on_reset"]),
```

#### 5.5 Translator

**文件**：`crates/engine/src/compiler/translator/component/settings.rs`（新建）
- StatelessWithItems translator，处理多层嵌套
- 在 `register_all` 注册

#### 5.6 Demo Case

**文件**：`demo/src/cases/settings_case.rml.rs` + `.rml`
- 展示：多页 Settings + 内置 field 绑定 + slot=field 自定义按钮 + 搜索 + 重置

#### 5.7 验证
- `is_item_builder_tag("setting-page")` / `"setting-group"` / `"setting-item"` 返回 true
- 多层嵌套正确生成
- slot=field 自定义渲染闭包正确生成
- `cargo test -p rust-rml-engine` 通过

---

## Assumptions & Decisions

### 设计决策（源自设计文档，已确认）

| 决策项 | 选择 | 依据 |
|-------|------|------|
| 实施范围 | Phase A 5 组件做透，B-E 后续推进 | 用户明确点名 5 个 |
| VirtualList 语法 | `<virtual-list>` + slot=render + let=range | 用户明确选择 |
| Settings 深度 | 结构化 + slot=field 自定义 | 用户明确选择 |
| vertical 属性统一 | 默认 horizontal，仅 `vertical=true` / `vertical={is_vertical}` | project_memory 铁律 |
| size 属性统一 | `size=small` / `size={size_value}` | project_memory 铁律 |
| 属性命名 | kebab-case 声明式 + snake_case 内部 | project_memory 铁律 |
| Rating color 处理 | 在 apply_style_attr 之前委托拦截 | 避免 CSS color 拦截 |
| Rating translator | 通用 StatelessComponentTranslator | 无需专属 translator |
| OtpInput state_ctor | 占位符 `__RML_OTP_LENGTH__` 注入 | OtpState 构造需 length 参数 |
| VirtualList translator | 专属 translator | 函数式构造 + slot 闭包 |
| Resizable kind | StatelessWithItems | 参考 Accordion 模式 |
| 禁止兼容性设计 | 不保留旧 API | project_memory 铁律 |

### 实施假设

1. gpui-component git 依赖（v0.5.2）包含所有 5 个目标组件
2. Stateful 事件订阅模式（input/event.rs）可复用于 OtpInput
3. slot 路由模式（popover/gen.rs）可复用于 VirtualList(slot=render) / Settings(slot=field)
4. StatelessWithItems 模式（accordion/）可复用于 Resizable / Settings

### 铁律遵循

- 一个 rs 文件 = 一个组件 / 一个职责
- `mod.rs` 仅 re-export，不写业务代码
- 无 `rml_` 前缀
- 属性命名禁止下划线（声明式用 kebab-case）
- 禁止兼容性设计

---

## Verification

### 每个 Phase 完成后
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine` 全量回归通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] 新增 demo case 可独立运行
- [ ] `props_registry_complete` 测试通过（注册表一致性）
- [ ] `component_props_tags_align_with_routing_table` 测试通过（tags ↔ props 一致性）

### 全部 Phase A 完成后
- [ ] 5 组件 demo case 完整覆盖属性 + 事件 + 绑定 + slot
- [ ] 所有属性在 props_registry.rs 中完整登记，无静默丢弃
- [ ] tags.rs component_lookup 覆盖所有新标签
- [ ] is_item_builder_tag 覆盖所有子项标签

### 实施顺序
1. **Phase 1: Rating 修复 + Demo** → 验证 color 修复 + 属性映射流程
2. **Phase 2: OtpInput** → 验证 state_ctor 注入 + 事件订阅
3. **Phase 3: VirtualList** → 验证 slot=render 闭包生成
4. **Phase 4: Resizable** → 验证 StatelessWithItems + Panel 子项
5. **Phase 5: Settings** → 综合验证多层嵌套 + slot=field
