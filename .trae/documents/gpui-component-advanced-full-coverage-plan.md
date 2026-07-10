# RML 框架 gpui-component 高级组件全量支持规划

> 参考来源：
> - https://longbridge.github.io/gpui-component/zh-CN/docs/components/virtual-list
> - https://longbridge.github.io/gpui-component/zh-CN/docs/components/settings
> - https://longbridge.github.io/gpui-component/zh-CN/docs/components/resizable
> - https://longbridge.github.io/gpui-component/zh-CN/docs/components/rating
> - https://longbridge.github.io/gpui-component/zh-CN/docs/components/otp-input
> - 以及 ColorPicker / ComboBox / Select / HoverCard / Clipboard / Sidebar / Dock / Notification / Sheet / FocusTrap / Chart / Plot / SearchableList / DatePicker / Tooltip / History
>
> 计划制定日期：2026-07-10
> gpui-component git rev：063e55bbc4fb13907a988111e3581595cbcaefde（v0.5.2）
> 组件源码路径：`C:\Users\lusid\.cargo\git\checkouts\gpui-component-95ce574d8a0da8b8\063e55b\crates\ui\src\`

## Summary

本规划覆盖 17 个尚未在 RML 中声明式支持的高级 gpui-component 组件，分 5 个阶段交付：

- **阶段 A（5 核心组件）**：VirtualList / Rating / OtpInput / Resizable / Settings —— 用户明确点名的 5 个，深度做透属性/事件/绑定/slot 全栈
- **阶段 B（选择与表单类，4 个）**：Select / ComboBox / ColorPicker / DatePicker —— 表单选择家族，复用 Input 事件订阅模式
- **阶段 C（弹层与通知类，4 个）**：Notification / Sheet / FocusTrap / HoverCard —— 弹层家族
- **阶段 D（布局与导航类，3 个）**：Sidebar / Dock / SearchableList —— 布局与导航家族
- **阶段 E（数据可视化类，2 个）**：Chart / Plot —— 高级数据可视化

每个组件均需遵循「6 项工作」标准流程：UI 封装层 / 编译器模块 / 标签路由 / 属性注册 / codegen 路由 / 演示案例。

**核心设计原则**：
1. **严格遵守 RML 框架最佳实践**（用户明确要求）—— 一个 rs 文件 = 一个组件 / 一个职责，`mod.rs` 仅 re-export，无 `rml_` 前缀
2. **属性全量映射**：static / bind / event 三类属性在 `props_registry.rs` 的 `COMPONENT_PROPS` 中完整登记，禁止静默丢弃
3. **声明式语法用 kebab-case 属性 + snake_case 内部**（如 `on-click` → `on_click`）
4. **Stateful 组件复用 Input 事件订阅模式**（`cx.subscribe` + `EventEmitter` block 表达式，参考 [compiler/input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs)）
5. **slot 模式优先**：容器型组件的具名子节点走 `slot="..."`（参考 [compiler/popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 trigger slot 模式）
6. **VirtualList 用 `<virtual-list>` + `<template slot="render" let="range">`**（用户明确选择）
7. **Settings 用结构化 + slot 自定义**（用户明确选择）—— 支持 `<Settings><SettingPage><SettingGroup><SettingItem>` 嵌套，`<SettingItem><template slot="field">` 注入自定义 field
8. **禁止兼容性设计**（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）

---

## 1. Current State Analysis（现状分析）

### 1.1 已支持的 gpui-component 组件（参考既有清单）

通过 [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `component_lookup` 与 [components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)：

- 基础：Accordion / Alert / Avatar / AvatarGroup / Badge / Button / ButtonGroup / Checkbox / Icon / Kbd / Label / Separator / Skeleton / Spinner / Tag / Tooltip（仅作通用属性）/ TitleBar / NativeStatusBar
- 表单：Input / TextInput / CodeEditor（基于 InputState 封装）/ Slider / Switch / Radio / RadioGroup / Pagination
- 容器：Card / Collapsible / GroupBox / Link / Popover / Table（WPF DataGrid 风格）/ Tabs / TabBar / Stepper / DescriptionList / Breadcrumb / Tree / AlertDialog / MenuBar

### 1.2 缺失的高阶组件（17 个，本规划覆盖范围）

| 阶段 | 组件 | 路径（gpui-component 源码） | 复杂度 |
|-----|------|---------------------------|-------|
| A | VirtualList | `ui/src/virtual_list.rs` | 高（函数式构造 + 闭包渲染） |
| A | Rating | `ui/src/rating.rs` | 低（Stateless，已部分 re-export） |
| A | OtpInput | `ui/src/input/otp_input.rs` | 中（Stateful，OtpState Entity） |
| A | Resizable | `ui/src/resizable/{mod,panel,resize_handle}.rs` | 高（Stateful + Panel 子项） |
| A | Settings | `ui/src/setting/{settings,page,group,item,fields}.rs` | 极高（嵌套层级 + AnySettingField trait） |
| B | Select | `ui/src/select.rs` | 中（Stateful） |
| B | ComboBox | `ui/src/combobox.rs` | 中（Stateful） |
| B | ColorPicker | `ui/src/color_picker.rs` | 中（Stateful） |
| B | DatePicker | `ui/src/time/date_picker.rs` | 中（Stateful） |
| C | Notification | `ui/src/notification.rs` | 中（命令式 API） |
| C | Sheet | `ui/src/sheet.rs` | 中（Root 管理） |
| C | FocusTrap | `ui/src/focus_trap.rs` | 低（容器包装） |
| C | HoverCard | `ui/src/hover_card.rs` | 中（trigger slot） |
| D | Sidebar | `ui/src/sidebar/{mod,footer,group,header,menu}.rs` | 高（嵌套 + 可折叠） |
| D | Dock | `ui/src/dock/{mod,...}.rs` | 极高（DockArea + Panel + Tile） |
| D | SearchableList | `ui/src/searchable_list.rs` | 中（Stateful） |
| E | Chart | `ui/src/chart/{mod,...}.rs` | 极高（Line/Bar/Pie/Area/Candlestick） |
| E | Plot | `ui/src/plot/{mod,...}.rs` | 极高（高级绘图） |

> Tooltip 已作为通用 `tooltip` 属性支持（[compiler/tooltip.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tooltip.rs)），不重复登记为独立标签；如需独立 `<Tooltip>` 标签在阶段 D 评估。

### 1.3 既有可复用模式

| 模式 | 参考实现 | 适用组件 |
|-----|---------|---------|
| Stateless (RenderOnce + ElementId) | [stateless.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateless.rs) | Rating |
| StatelessNoId (RenderOnce 无 id) | 同上 | FocusTrap / Chart / Plot |
| StatelessWithItems (闭包式 .item()) | [accordion/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/) | Resizable / Settings / Sidebar |
| Stateful (Entity + Input 事件订阅) | [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs) | OtpInput / Select / ComboBox / ColorPicker / DatePicker / SearchableList |
| 容器 + slot 路由 | [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) | HoverCard / Sheet / Settings(自定义 field) |
| 专属 translator (特殊构造) | [stepper/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/) | VirtualList / Notification / Dock |
| 命令式 API (window_ext) | [window/ext.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/ext.rs) | Notification / Dialog / Sheet 已部分 |

---

## 2. 实施规范（统一模板）

每个新增组件按以下顺序完成 6 项工作（参考 [gpui-component-completion-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/gpui-component-completion-plan.md) 第 2 节）：

1. **UI 封装层**：`crates/ui/src/components/<component>.rs`（薄 re-export）或 `<component>/` 目录（复杂封装）
2. **编译器模块**：`crates/engine/src/compiler/components/<component>/{mod,gen,setters}.rs`，`mod.rs` 仅 re-export
3. **标签路由注册**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `component_lookup` 添加 match 臂，含 `ctor_path` / `kind` / `container`
4. **属性注册登记**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `COMPONENT_PROPS` 添加完整属性清单（static / bind / event 分类）
5. **codegen 路由**：[components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs) + [translator/component/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/mod.rs) `register_all`
6. **演示案例**：`demo/src/cases/<component>_case.rml.rs` + [demo/src/cases/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 注册

### 2.1 属性命名约定（铁律）

- **声明式 kebab-case**：`on-click` / `item-sizes` / `default-open` / `scroll-strategy`
- **内部 snake_case**：normalize 后 `on_click` / `item_sizes` / `default_open` / `scroll_strategy`
- **禁止下划线属性名**（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律："所有属性和 tag 名在声明式编码中必须遵循 label-width 标准，禁止使用下划线"）
- **size 属性统一**：`size=small` / `size={size_value}`（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md)）
- **vertical 属性统一**：默认 horizontal，仅 `vertical=true` / `vertical={is_vertical}`（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md)）

### 2.2 ComponentKind 选型矩阵

| ComponentKind | 构造模式 | 适用场景 |
|--------------|---------|---------|
| Stateless | `Type::new(id)` + setter 链 | Rating |
| StatelessNoId | `Type::new()` 无参构造 | FocusTrap / Chart / Plot（RenderOnce） |
| Stateful | `Type::new(&entity)` + cx.subscribe | OtpInput / Select / ComboBox / ColorPicker / DatePicker / SearchableList |
| StatelessWithItems | `Type::new(id).item(\|item\| ...)` | Resizable / Settings / Sidebar |
| EntityRef | 从 Host 字段 clone Entity | Dock（如有 DockState Entity） |
| 专属 translator | 函数式构造 / 命令式 API | VirtualList / Notification |

---

## 3. Proposed Changes（按阶段详细规划）

### 阶段 A：5 核心组件（用户明确点名）

#### A.1 VirtualList（函数式构造 + slot=render 模式）

**gpui-component API 调研**（[virtual_list.rs:132-200](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/virtual_list.rs#L132-L200)）：

- 构造函数：`v_virtual_list(view: Entity<V>, id, item_sizes: Rc<Vec<Size<Pixels>>>, f: Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>)` / `h_virtual_list(...)` / `virtual_list(view, id, axis, ...)`
- builder 方法：`.track_scroll(&VirtualListScrollHandle)` / `.with_sizing_behavior(ListSizingBehavior)`
- `VirtualListScrollHandle`：`.scroll_to_item(ix, ScrollStrategy)` / `.scroll_to_bottom()` / `.base_handle()`
- 实现 `Styled` trait（可链式调用样式方法）

**RML 声明式语法设计**（用户已选 `<virtual-list>` + slot=render）：

```rml
<virtual-list
  direction="vertical"
  item-sizes={item_sizes}
  scroll-handle={scroll_handle}
  on-scroll-to={on_scroll_to_item}
>
  <template slot="render" let="range">
    <div each={ix in range.start..range.end} class="row">
      {items[ix]}
    </div>
  </template>
</virtual-list>
```

**实现要点**：

- **ComponentKind**：专属 translator（VirtualListTranslator），不套用 Stateless/Stateful 通用模板
- **构造器**：codegen 生成 `rml_ui::v_virtual_list(cx.entity().clone(), ("rml_el", N), self.item_sizes.clone(), move |this, range, _window, cx| { ... })`
- **slot=render 处理**：参考 [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 slot 路由模式，`<template slot="render" let="range">` 子节点生成闭包体
- **let="range"**：在闭包签名中注入 `range: Range<usize>` 参数，闭包内的 `ix` 通过 `each` 指令迭代 `range.start..range.end`
- **direction 属性**：`direction="vertical"` → `v_virtual_list`，`direction="horizontal"` → `h_virtual_list`（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 默认 horizontal 铁律 → 这里需破例：VirtualList 默认 vertical 更符合直觉，但遵循铁律设默认 horizontal，用户写 `direction="vertical"`）

  > **决策**：遵循 [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律「vertical/horizontal 默认 horizontal，仅 `vertical=true` / `vertical={is_vertical}`」。VirtualList 同样默认 horizontal，用 `vertical={true}` 或 `vertical={is_vertical}` 切换 vertical。
  >
  > 即：`<virtual-list vertical>` 等价 `direction="vertical"`；不写 `vertical` 属性为 horizontal。简化为单一 `vertical` 属性，去除 `direction` 属性以避免双轨。

- **item-sizes 绑定**：`item-sizes={self.item_sizes}` → `.item_sizes(self.item_sizes.clone())`，类型为 `Rc<Vec<Size<Pixels>>>`
- **scroll-handle 绑定**：`scroll-handle={self.scroll_handle}` → 直接传入 Entity/字段引用，调用 `.track_scroll(&self.scroll_handle)`
- **ListSizingBehavior**：`sizing="auto"` / `sizing="infer"` → `.with_sizing_behavior(ListSizingBehavior::Auto)` / `::Infer`

**属性清单**（COMPONENT_PROPS 登记）：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| vertical | bool | static | vertical=true → v_virtual_list，否则 h_virtual_list |
| item_sizes | Rc<Vec<Size<Pixels>>> | bind | 每项尺寸 |
| scroll_handle | VirtualListScrollHandle | bind | 滚动句柄（用于编程式滚动） |
| sizing | "auto" / "infer" | static | ListSizingBehavior |
| on_scroll_to | Fn(usize, ScrollStrategy) | event | 编程式滚动触发（可选，通常通过 scroll_handle 调用） |

**实现路径**：

- [crates/ui/src/components/virtual_list.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/virtual_list.rs)：re-export `gpui_component::virtual_list::{v_virtual_list, h_virtual_list, VirtualList, VirtualListScrollHandle}`
- [crates/engine/src/compiler/components/virtual_list/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/virtual_list/)：`mod.rs` / `gen.rs` / `setters.rs`（含 slot=render 闭包生成）
- [crates/engine/src/compiler/translator/component/virtual_list.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/virtual_list.rs)：专属 translator
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"VirtualList" | "virtual-list"` 路由，ComponentKind 专属处理（不在 ComponentKind 枚举内，走专属 translator matches）

**验证**：`cargo test virtual_list`、`virtual_list_case.rml.rs` 展示 1000 项垂直列表 + 编程式滚动到第 100 项 + horizontal 卡片列表

---

#### A.2 Rating（Stateless，已部分 re-export，需补 codegen）

**gpui-component API 调研**（[rating.rs:24-102](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/rating.rs#L24-L102)）：

- 构造器：`Rating::new(id: impl Into<ElementId>)`
- builder：`.with_size(Size)` / `.disabled(bool)` / `.color(Hsla)` / `.value(usize)` / `.max(usize)` / `.on_click(Fn(&usize, &mut Window, &mut App))`
- 实现 `Styled` / `Sizable` / `Disableable` trait
- 内部用 `window.use_keyed_state` 管理 value/hovered，**无需 Entity 字段**

**RML 声明式语法**：

```rml
<Rating value="3" max="5" on-click={on_rating_change} />
<Rating value={current_rating} max="10" disabled={is_readonly} />
<Rating size="small" color="#ffcc00" on-click={on_rating_change} />
```

**实现要点**：

- **ComponentKind**：`Stateless`（构造器接受 ElementId）
- **color 属性**：需要 Hsla 解析，复用 CSS color 解析（`#ffcc00` → `gpui::hsla(...)`），参考 css/mapper.rs
- **on_click 事件签名**：`Fn(&usize, ...)` —— 用户方法签名约定 `fn on_rating_change(&mut self, value: &usize, cx: &mut Context<Self>)`，参考 Stepper 的 `idx: &usize` 模式（[stepper/setters.rs:74-91](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/setters.rs#L74-L91)）

**属性清单**：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| value | usize | static+bind | 初始值，0..=max |
| max | usize | static | 最大星数，默认 5 |
| color | Hsla | static+bind | 激活颜色（默认主题 yellow） |
| size | Size | static+bind | 走通用 Sizable（size=small/large） |
| disabled | bool | static+bind | 走通用 Disableable |
| on_click | Fn(&usize) | event | 点击评分回调 |

**实现路径**：

- [crates/ui/src/components/rating.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/rating.rs)：已有 re-export，补充文档注释
- `crates/engine/src/compiler/components/rating/{mod,gen,setters}.rs`：专属 codegen（因 on_click 非 ClickEvent 而是 &usize）
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Rating"` 路由（Stateless）
- [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)：登记 `("Rating", &["value", "max", "color", "on_click"])`

**验证**：`rating_case.rml.rs` 展示受控评分 + disabled + 自定义颜色 + 10 星上限

---

#### A.3 OtpInput（Stateful + OtpState Entity + InputEvent 订阅）

**gpui-component API 调研**（[input/otp_input.rs:12-273](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/input/otp_input.rs#L12-L273)）：

- `OtpState::new(length: usize, window, cx: &mut Context<Self>)` —— state 构造需 length 参数
- `OtpState` 方法：`.default_value(impl Into<SharedString>)` / `.set_value(...)` / `.value()` / `.masked(bool)` / `.set_masked(...)` / `.focus(...)`
- `OtpState: EventEmitter<InputEvent>` —— 支持 `Change` / `Focus` / `Blur` 事件
- `OtpInput::new(state: &Entity<OtpState>)` —— 构造接受 Entity 引用
- builder：`.groups(usize)` / `.disabled(bool)` / `.with_size(Size)` —— 默认 groups=2
- 实现 `Disableable` / `Sizable` / `RenderOnce` trait

**RML 声明式语法**：

```rml
<OtpInput ref="otp_state" length="6" groups="2" masked on-change={on_otp_change} on-focus={on_otp_focus} />
<OtpInput ref="pin_state" length="4" groups="1" size="small" disabled={is_locked} />
```

**实现要点**：

- **ComponentKind**：`Stateful { state_field: "otp_state", state_ctor: "|w, c| rml_ui::OtpState::new(6, w, c)" }`
  - **难点**：`state_ctor` 闭包需硬编码 length（构造器参数），但 RML 的 length 属性是动态的。需要特殊处理：codegen 从 `length` 属性提取数值注入 `state_ctor`
  - **决策**：state_ctor 使用占位符 `|w, c| rml_ui::OtpState::new(__RML_OTP_LENGTH__, w, c)`，codegen 在生成 `get_or_init_ref` 调用前根据 `length` 属性值替换占位符
- **事件订阅**：复用 Input 事件订阅模式（[input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs)）—— `on_change` / `on_focus` / `on_blur` 通过 `cx.subscribe(&otp_state, ...)` block 表达式包装构造器
- **masked 属性**：`masked` / `masked="true"` → state 构造时调用 `.masked(true)`；由于 state 在 `get_or_init_ref` 中构造，需把 masked 也注入 state_ctor 闭包：`|w, c| rml_ui::OtpState::new(6, w, c).masked(true)`
- **default_value 属性**：同理注入 state_ctor：`.default_value("123456")`
- **groups 属性**：`.groups(N)` 直接 builder 链
- **on_change 事件签名**：`Fn(&Entity<OtpState>, &mut Window, &mut App)` 通过 cx.subscribe → 用户方法约定 `fn on_otp_change(&mut self, state: &Entity<OtpState>, cx: &mut Context<Self>)`，闭包内通过 `state.read(cx).value()` 取值

**属性清单**：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| length | usize | static | OTP 位数（注入 state_ctor） |
| groups | usize | static+bind | 分组数，默认 2 |
| masked | bool | static | 掩码显示（注入 state_ctor） |
| default_value | SharedString | static | 初始值（注入 state_ctor） |
| size | Size | static+bind | 走通用 Sizable |
| disabled | bool | static+bind | 走通用 Disableable |
| on_change | Fn(&Entity<OtpState>) | event | 输入完成回调 |
| on_focus | Fn(&Entity<OtpState>) | event | 获得焦点 |
| on_blur | Fn(&Entity<OtpState>) | event | 失去焦点 |

**实现路径**：

- [crates/ui/src/components/otp_input.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/otp_input.rs)：re-export `gpui_component::input::{OtpInput, OtpState}`
- [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)：在 input re-export 处补充 OtpInput/OtpState
- `crates/engine/src/compiler/components/otp_input/{mod,gen,setters,event}.rs`：专属 codegen
- `crates/engine/src/compiler/translator/component/otp_input.rs`：专属 translator（继承 StatefulComponentTranslator 模式但需特化处理 length/masked/default_value 注入 state_ctor）
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"OtpInput" | "otp-input"` 路由（Stateful）

**验证**：`otp_input_case.rml.rs` 展示 6 位 SMS + 4 位 PIN（masked）+ groups=1 + on_change 验证 + disabled 锁定

---

#### A.4 Resizable（Stateful + ResizablePanelGroup + Panel 子项）

**gpui-component API 调研**（[resizable/mod.rs:14-110](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/resizable/mod.rs#L14-L110) + [resizable/panel.rs:31-270](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/resizable/panel.rs#L31-L270)）：

- 构造函数：`h_resizable(id)` / `v_resizable(id)` / `ResizablePanelGroup::new(id).axis(Axis)`
- `ResizablePanelGroup::new(id)` builder：`.with_state(&Entity<ResizableState>)` / `.axis(Axis)` / `.child(impl Into<ResizablePanel>)` / `.children(I)` / `.size(Pixels)` / `.on_resize(Fn(&Entity<ResizableState>, ...))`
- `ResizablePanel::new()` builder：`.visible(bool)` / `.size(Pixels)` / `.size_range(Range<Pixels>)` —— 实现 `Styled` + `ParentElement`
- `ResizableState` 方法：`.sizes()` / `.resize_panel(ix, size, window, cx)`
- `ResizablePanelEvent::Resized` 事件（EventEmitter）

**RML 声明式语法**：

```rml
<Resizable ref="resizable_state" on-resize={on_panel_resize}>
  <resizable-panel size="220" min-size="100">
    <Sidebar />
  </resizable-panel>
  <resizable-panel>
    <Content />
  </resizable-panel>
  <resizable-panel size="280" min-size="150" max-size="400">
    <Metadata />
  </resizable-panel>
</Resizable>

<!-- 垂直布局 -->
<Resizable vertical={true} ref="v_state">
  <resizable-panel size="200">顶部</resizable-panel>
  <resizable-panel>底部</resizable-panel>
</Resizable>
```

**实现要点**：

- **ComponentKind**：`StatelessWithItems`（参考 Accordion 模式，[accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/)）
  - 容器：`ResizablePanelGroup::new(id).axis(...)`，根据 `vertical` 属性选择 `h_resizable` / `v_resizable` 快捷构造
  - 子项：`<resizable-panel>` 标签（kebab-case）或 `<ResizablePanel>`（PascalCase）→ `.child(resizable_panel().size(...).child(...))`
- **vertical 属性**（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）：默认 horizontal，`vertical={true}` / `vertical={is_vertical}` 切换
- **on-resize 事件签名**：`Fn(&Entity<ResizableState>, &mut Window, &mut App)` → 用户方法约定 `fn on_panel_resize(&mut self, state: &Entity<ResizableState>, window: &mut Window, cx: &mut Context<Self>)`
  - 由于闭包需访问 window，不能用 `cx.listener`（listener 闭包是 `Fn(this, ..., cx)`），需用 `Rc<dyn Fn>` 直接构造：`.on_resize(Rc::new(move |state, window, app| { ... }))` + weak entity
- **ref 指令**：`ref="resizable_state"` 生成 `Entity<ResizableState>` 字段，用于编程式 `resize_panel` 调用
- **size/size_range 属性**：`size="220"` → `.size(px(220.))`，`min-size="100"` + `max-size="400"` → `.size_range(px(100.)..px(400.))`

**属性清单**：

Resizable：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| vertical | bool | static | vertical=true → v_resizable，否则 h_resizable |
| size | Pixels | static+bind | 容器尺寸（horizontal=height，vertical=width） |
| on_resize | Fn(&Entity<ResizableState>) | event | 拖拽结束回调 |
| ref | - | 指令 | ref="state" → Entity<ResizableState> 字段 |

ResizablePanel：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| size | Pixels | static+bind | 初始尺寸 |
| min_size | Pixels | static | size_range 下限 |
| max_size | Pixels | static | size_range 上限 |
| visible | bool | static+bind | 可见性 |

**实现路径**：

- [crates/ui/src/components/resizable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/resizable.rs)：re-export `gpui_component::resizable::{h_resizable, v_resizable, resizable_panel, ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent}`
- `crates/engine/src/compiler/components/resizable/{mod,gen,setters,panel}.rs`：容器 + panel 子项 codegen
- `crates/engine/src/compiler/translator/component/resizable.rs`：StatelessWithItems translator
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Resizable" | "resizable"` + `"ResizablePanel" | "resizable-panel"` 路由
- `is_item_builder_tag` 函数添加 `"ResizablePanel" | "resizable-panel"` 识别

**验证**：`resizable_case.rml.rs` 展示三栏布局（220/自适应/280）+ min/max 限制 + vertical 布局 + on_resize 持久化尺寸

---

#### A.5 Settings（嵌套层级 + AnySettingField trait + slot=field 自定义）

**gpui-component API 调研**（[setting/settings.rs:27-100](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/settings.rs#L27-L100) + [setting/page.rs:20-90](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/page.rs#L20-L90) + [setting/group.rs:16-120](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/group.rs#L16-L120) + [setting/item.rs:22-326](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/item.rs#L22-L326)）：

层级：`Settings → SettingPage → SettingGroup → SettingItem`

- `Settings::new(id)` builder：`.page(SettingPage)` / `.pages(I)` / `.sidebar_width(Pixels)` / `.with_group_variant(GroupBoxVariant)` / `.sidebar_style(&StyleRefinement)` / `.default_selected_index(SelectIndex)` / `.header_style(&StyleRefinement)`
- `SettingPage::new(title)` builder：`.title(...)` / `.icon(Icon)` / `.description(...)` / `.default_open(bool)` / `.resettable(bool)` / `.group(SettingGroup)` / `.groups(I)` / `.header_style(...)`
- `SettingGroup::new()` builder：`.title(...)` / `.description(...)` / `.item(SettingItem)` / `.items(I)` —— 实现 `Styled`
- `SettingItem::new<F: AnySettingField>(title, field)` / `SettingItem::render<R>(render_closure)` builder：`.description(Text)` / `.keywords(Vec<SharedString>)` / `.layout(Axis)` / `.disabled(bool)` / `.on_reset(is_dirty, reset_closure)`

内置 field 类型（[setting/fields.rs](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/setting/fields.rs) 未读，但 docs 提及）：`BoolField` / `StringField` / `NumberField` / `DropdownField`

**RML 声明式语法**（用户已选结构化 + slot 自定义）：

```rml
<Settings ref="settings_state" sidebar-width="280" default-selected-index="0">
  <SettingPage title="通用" icon="Settings" default-open>
    <SettingGroup title="外观">
      <SettingItem title="主题" field={theme_field}>
        <template slot="description">选择应用主题</template>
      </SettingItem>
      <SettingItem title="语言" field={language_field} />
    </SettingGroup>
    <SettingGroup title="通知">
      <SettingItem title="启用通知" field={notify_field} />
    </SettingGroup>
  </SettingPage>

  <SettingPage title="高级">
    <SettingGroup>
      <SettingItem title="自定义">
        <template slot="field">
          <Button on-click={on_custom_click}>自定义按钮</Button>
        </template>
      </SettingItem>
    </SettingGroup>
  </SettingPage>
</Settings>
```

**实现要点**：

- **ComponentKind**：`StatelessWithItems`（容器 + 多层嵌套子项）
- **多层 item builder**：
  - `<Settings>` 容器：`Settings::new(id).page(SettingPage::new(...).group(...).item(...))`
  - `<SettingPage>`：`SettingPage::new(title).group(SettingGroup::new().item(SettingItem::new(title, field)))`
  - `<SettingGroup>`：`SettingGroup::new().item(SettingItem::new(...))`
  - `<SettingItem>`：`SettingItem::new(title, field)` 或 `SettingItem::render(closure)`（slot=field）
- **field 绑定**：`field={theme_field}` → `field` 属性为 ViewModel 字段引用，类型为 `Rc<dyn AnySettingField>` 或具体 `BoolField` / `StringField` 等
  - 用户在 ViewModel 实现 `AnySettingField` trait 的字段：`theme_field: BoolField::new(true)`
  - codegen：`field={theme_field}` → `SettingItem::new("主题", self.theme_field.clone())`
- **slot=field 自定义**：`<template slot="field">` 子节点 → `SettingItem::render(move |options, window, cx| { <自定义元素> })`
  - 参考 [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 slot 路由模式
  - 闭包签名：`Fn(&RenderOptions, &mut Window, &mut App) -> AnyElement`
- **多层 is_item_builder_tag 扩展**：`is_item_builder_tag` 添加 `"SettingPage" | "setting-page"` / `"SettingGroup" | "setting-group"` / `"SettingItem" | "setting-item"`
- **default_selected_index**：类型为 `SelectIndex`（gpui-component 内部类型），codegen 用 `rml_ui::SelectIndex::new(N)` 构造

**属性清单**：

Settings：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| sidebar_width | Pixels | static+bind | 侧边栏宽度，默认 250px |
| group_variant | GroupBoxVariant | static | normal/fill/outline |
| default_selected_index | SelectIndex | static+bind | 默认选中页 |
| on_reset | Fn(&mut Window, &mut App) | event | 重置回调（可选） |

SettingPage：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| title | SharedString | static+bind | 页面标题（构造器参数） |
| icon | Icon | static | 页面图标 |
| description | SharedString | static+bind | 描述 |
| default_open | bool | static | 默认展开 |
| resettable | bool | static | 可重置 |

SettingGroup：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| title | SharedString | static+bind | 组标题 |
| description | SharedString | static+bind | 组描述 |

SettingItem：

| 属性 | 类型 | 分类 | 说明 |
|-----|------|-----|------|
| title | SharedString | static+bind | 项标题（构造器参数） |
| field | AnySettingField | bind | 字段对象 |
| description | Text | static | 项描述 |
| layout | Axis | static | horizontal/vertical |
| disabled | bool | static+bind | 走通用 |
| keywords | Vec<SharedString> | bind | 搜索关键词 |
| on_reset | Fn | event | 自定义重置 |

**实现路径**：

- [crates/ui/src/components/settings.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/settings.rs)：re-export `gpui_component::setting::{Settings, SettingPage, SettingGroup, SettingItem, SelectIndex}` + 内置 field 类型（BoolField/StringField/NumberField/DropdownField）
- `crates/engine/src/compiler/components/settings/{mod,gen,setters,page,group,item}.rs`：多层嵌套 codegen
- `crates/engine/src/compiler/translator/component/settings.rs`：StatelessWithItems translator
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Settings" | "settings"` / `"SettingPage" | "setting-page"` / `"SettingGroup" | "setting-group"` / `"SettingItem" | "setting-item"` 路由
- `is_item_builder_tag`：添加上述 4 种子标签

**验证**：`settings_case.rml.rs` 展示多页 Settings + 内置 BoolField/StringField + slot=field 自定义按钮 + 搜索 + 重置

---

### 阶段 B：选择与表单类（4 个，复用 Input 事件订阅模式）

#### B.1 Select（Stateful）

**API 调研**（[select.rs:110+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/select.rs)）：

- `SelectState::new(window, cx)` 或类似
- `Select::new(&Entity<SelectState>)` builder：`.placeholder()` / `.clearable()` / `.searchable()` / `.disabled()` / `.options()` / `.value()`
- `SelectState: EventEmitter<SelectEvent>` —— on_change 等事件

**RML 声明式语法**：

```rml
<Select ref="select_state" placeholder="选择语言" clearable searchable
        options={language_options} value={current_lang}
        on-change={on_lang_change} />
```

**属性清单**：`placeholder` / `clearable` / `searchable` / `multiple` / `max_count` / `options`(bind) / `value`(bind) / `disabled`(bind) / `on_change`

**实现路径**：

- `crates/ui/src/components/select.rs` re-export
- `crates/engine/src/compiler/components/select/{mod,gen,setters,event}.rs` 复用 Input 事件订阅模式
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `"Select" | "select"` Stateful

**验证**：`select_case.rml.rs` 展示单选 + 多选 + searchable + on_change

---

#### B.2 ComboBox（Stateful）

**API 调研**（[combobox.rs:710+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/combobox.rs)）：

- 构造类似 Select，但支持自由输入
- 事件同 Select 模式

**RML 声明式语法**：

```rml
<ComboBox ref="combo_state" placeholder="输入或选择"
          options={city_options} value={current_city}
          on-change={on_city_change} />
```

**属性清单**：`placeholder` / `options` / `value` / `disabled` / `clearable` / `on_change` / `on_focus` / `on_blur`

**实现路径**：同 Select 模式（Stateful + 事件订阅）

---

#### B.3 ColorPicker（Stateful）

**API 调研**（[color_picker.rs:330+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/color_picker.rs)）：

**RML 声明式语法**：

```rml
<ColorPicker ref="color_state" value={current_color} show-alpha on-change={on_color_change} />
```

**属性清单**：`value` / `show_alpha` / `show_hex` / `default_color` / `disabled` / `on_change`

**实现路径**：同 Stateful 模式

---

#### B.4 DatePicker（Stateful）

**API 调研**（[time/date_picker.rs:15+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/time/date_picker.rs)）：

**RML 声明式语法**：

```rml
<DatePicker ref="date_state" placeholder="选择日期" format="YYYY-MM-DD"
            mode="date" value={current_date} on-change={on_date_change} />
```

**属性清单**：`placeholder` / `format` / `mode`(date/datetime/month) / `week_start` / `show_time` / `value` / `disabled` / `on_change`

**实现路径**：同 Stateful 模式

---

### 阶段 C：弹层与通知类（4 个）

#### C.1 Notification（命令式 API + 容器）

**API 调研**（[notification.rs:62+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/notification.rs)）：

- `Notification::new(id)` builder：`.title()` / `.description()` / `.variant()` / `.auto_close()` / `.position()`
- `NotificationList` 由 Root 管理，通过 `window.push_notification(...)` 命令式调用

**RML 声明式语法**（容器型，用户主动声明 `<Notification>`）：

```rml
<Notification ref="notif" title="提示" description="操作成功" variant="success" auto-close />
```

**属性清单**：`title` / `description` / `variant`(info/success/warning/error) / `auto_close` / `position` / `icon` / `on_close`

**实现路径**：

- `crates/ui/src/components/notification.rs` re-export
- `crates/engine/src/compiler/components/notification/{mod,gen,setters}.rs`
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) Stateless

---

#### C.2 Sheet（Root 管理 + slot）

**API 调研**（[sheet.rs:46+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/sheet.rs)）：

- 由 `Root` 管理，通过 `window.open_sheet(...)` 命令式调用
- `Sheet::new(id)` builder：`.title()` / `.placement()` / `.size()` / `.content()` / `.on_close()`

**RML 声明式语法**：

```rml
<Sheet ref="sheet" title="设置" placement="right" size="400" on-close={on_sheet_close}>
  <template slot="content">
    <div>Sheet 内容</div>
  </template>
</Sheet>
```

**属性清单**：`title` / `placement`(left/right/top/bottom) / `size` / `open`(bind) / `on_close`

**实现路径**：参考 Popover 的 slot 模式

---

#### C.3 FocusTrap（StatelessNoId 容器）

**API 调研**（[focus_trap.rs:110+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/focus_trap.rs)）：

- `FocusTrap::new()` RenderOnce 无参构造
- 实现 `ParentElement` —— 接受子元素

**RML 声明式语法**：

```rml
<FocusTrap>
  <Input placeholder="用户名" />
  <Input placeholder="密码" />
  <Button label="登录" />
</FocusTrap>
```

**属性清单**：仅通用（无专用属性）

**实现路径**：StatelessNoId 容器

---

#### C.4 HoverCard（trigger slot + content slot）

**API 调研**（[hover_card.rs:16+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/hover_card.rs)）：

**RML 声明式语法**：

```rml
<HoverCard>
  <template slot="trigger">
    <Button label="悬停查看" />
  </template>
  <div>弹出内容</div>
</HoverCard>
```

**属性清单**：`default_open` / `hover_delay` / `placement` / `overlay_closable`

**实现路径**：参考 Popover 的 trigger slot 模式

---

### 阶段 D：布局与导航类（3 个）

#### D.1 Sidebar（嵌套 + 可折叠）

**API 调研**（[sidebar/mod.rs:221+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/sidebar/mod.rs)）：

- `Sidebar::new()` builder：`.collapsible(SidebarCollapsible)` / `.collapsed(bool)` / `.side(Side)` / `.width(Pixels)` / `.on_collapsed(...)`
- 子项：`SidebarMenu` / `SidebarMenuItem` / `SidebarGroup` / `SidebarHeader` / `SidebarFooter`

**RML 声明式语法**：

```rml
<Sidebar ref="sidebar" collapsible="icon" collapsed={is_collapsed} width="255">
  <SidebarHeader>标题</SidebarHeader>
  <SidebarMenu>
    <SidebarMenuItem icon="Home" label="首页" on-click={on_home_click} />
    <SidebarMenuItem icon="Settings" label="设置" on-click={on_settings_click} />
  </SidebarMenu>
  <SidebarFooter>底部</SidebarFooter>
</Sidebar>
```

**属性清单**：`collapsible`(icon/offcanvas/none) / `collapsed` / `side`(left/right) / `width` / `on_collapsed`

**实现路径**：StatelessWithItems，参考 Accordion 多层 item 模式

---

#### D.2 Dock（DockArea + Panel）

**API 调研**（[dock/mod.rs:44+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/dock/mod.rs)）：

- `DockArea::new(id)` Stateful（`DockState`）
- 复杂的 Panel/Tile 嵌套结构

**RML 声明式语法**（先简化，复杂场景走命令式）：

```rml
<DockArea ref="dock_state">
  <template slot="panel" let="panel_id">
    {render_panel(panel_id)}
  </template>
</DockArea>
```

**属性清单**：`panels`(bind) / `layout`(bind) / `on_panel_close` / `on_panel_focus`

**实现路径**：Stateful + slot=panel 闭包渲染

> 注：Dock 极复杂，阶段 D 实施前需先输出独立设计文档

---

#### D.3 SearchableList（Stateful）

**API 调研**（[searchable_list.rs:10+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/searchable_list.rs)）：

**RML 声明式语法**：

```rml
<SearchableList ref="list_state" items={items} placeholder="搜索..."
                on-select={on_item_select} />
```

**属性清单**：`items` / `placeholder` / `selected_index` / `on_select`

**实现路径**：Stateful

---

### 阶段 E：数据可视化类（2 个，极复杂）

#### E.1 Chart（多 variant）

**API 调研**（[chart/mod.rs:1-11](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/chart/mod.rs)）：

- `LineChart` / `BarChart` / `PieChart` / `AreaChart` / `CandlestickChart` 各自构造

**RML 声明式语法**：

```rml
<Chart type="line" data={stats} x-field="date" y-field="value" />
<Chart type="bar" data={sales} x-field="month" y-field="revenue">
  <template slot="tooltip" let="item">
    <div>{item.label}: {item.value}</div>
  </template>
</Chart>
```

**属性清单**：`type`(line/bar/pie/area/candlestick) / `data` / `x_field` / `y_field` / `legend` / `theme` / `on_hover`

**实现路径**：StatelessNoId + type 属性选择构造器（参考 Alert variant 模式）

> 注：Chart/Plot 极复杂，阶段 E 实施前需先输出独立设计文档

---

#### E.2 Plot（高级绘图）

**API 调研**（[plot/mod.rs:23+](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/plot/mod.rs)）：

**RML 声明式语法**：

```rml
<Plot ref="plot_state" data={plot_data} x-axis="time" y-axis="value">
  <template slot="series" let="series">
    <PlotSeries name={series.name} color={series.color} />
  </template>
</Plot>
```

**属性清单**：`data` / `x_axis` / `y_axis` / `tooltip` / `grid` / `on_hover`

**实现路径**：Stateful + slot=series

---

## 4. Assumptions & Decisions（假设与决策）

### 4.1 设计决策（基于用户答复 + [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）

| 决策项 | 选择 | 依据 |
|-------|------|------|
| 规划范围 | 全量高级组件（17 个） | 用户明确选择「全量高级组件」 |
| VirtualList 语法 | `<virtual-list>` + slot=render + let=range | 用户明确选择 |
| Settings 深度 | 结构化 + slot=field 自定义 | 用户明确选择 |
| vertical 属性统一 | 默认 horizontal，仅 `vertical=true` / `vertical={is_vertical}` | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| size 属性统一 | `size=small` / `size={size_value}` | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| 属性命名 | kebab-case 声明式 + snake_case 内部 | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| Stateful 事件订阅 | 复用 Input 事件订阅模式（cx.subscribe + block 表达式） | 既有成熟模式 |
| Stateful state_ctor 注入 | OtpInput 的 length/masked/default_value 通过占位符注入 state_ctor | OtpState 构造器需要 length 参数 |
| 重型组件阶段 | Sidebar/Dock/Chart/Plot 置后到阶段 D/E | 复杂度高，需独立设计文档 |

### 4.2 关键假设

1. **gpui-component git 依赖包含所有目标组件**：所有 17 个组件均在 v0.5.2 git 依赖中可用
2. **Stateful 事件订阅模式成熟**：[input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs) 模式可复用于 Select/ComboBox/ColorPicker/DatePicker/SearchableList/OtpInput
3. **slot 路由模式成熟**：[popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 trigger slot 模式可复用于 HoverCard/Sheet/Settings(自定义 field)/VirtualList(slot=render)
4. **StatelessWithItems 模式成熟**：[accordion/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/) 模式可复用于 Resizable/Settings/Sidebar
5. **重型组件阶段细化**：阶段 D/E 的 5 个组件仅给出框架性规划，进入实施时需先输出独立设计文档（声明式 API 草案 + gpui-component API 调研）

### 4.3 铁律遵循

- **一个 rs 文件 = 一个组件 / 一个职责**（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）
- 所有 `mod.rs` 仅 re-export，不写业务代码
- 多个独立 `pub struct` 组件拆分独立文件（如 Settings 的 SettingPage/SettingGroup/SettingItem 各自独立文件）
- 无 `rml_` 前缀（用户偏好）
- 优先扩展现有枚举的 variant，而非暴露新接口
- 禁止兼容性设计（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律：「No tolerance for compatibility designs」）
- 属性命名禁止下划线（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）

### 4.4 文件组织规范

- **UI 封装层**：`crates/ui/src/components/<component_snake_case>.rs`（薄 re-export）或 `<component_snake_case>/` 目录（复杂封装，如 sidebar/chart）
- **编译器模块**：`crates/engine/src/compiler/components/<component_snake_case>/{mod,gen,setters}.rs`，`mod.rs` 仅 re-export
- **translator**：`crates/engine/src/compiler/translator/component/<component_snake_case>.rs`
- **演示案例**：`demo/src/cases/<component_snake_case>_case.rml.rs` + .rml 文件

---

## 5. Verification（验证标准）

### 5.1 每个组件的验证清单

- [ ] `cargo build -p rust-rml-ui` 成功（UI 封装层编译通过）
- [ ] `cargo build -p rust-rml-engine` 成功（编译器模块编译通过）
- [ ] `cargo test -p rust-rml-engine --test props_registry_complete` 通过（属性注册一致性）
- [ ] `cargo test -p rust-rml-engine <component>` 通过（codegen 单元测试）
- [ ] `cargo build -p rust-rml-demo` 成功（demo 编译通过）
- [ ] 新增 `<component>_case.rml.rs` 可在 demo 应用中独立运行
- [ ] [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 的 `component_lookup` 单元测试覆盖新标签
- [ ] [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `COMPONENT_PROPS` 包含所有新增组件的属性清单
- [ ] 属性全量映射：static / bind / event 三类属性均有对应 setter（无静默丢弃）
- [ ] `is_item_builder_tag` 函数覆盖所有子项标签（如 resizable-panel / setting-page / setting-group / setting-item）

### 5.2 阶段性验证

- **阶段 A 验证**：5 核心组件 demo case 完整覆盖属性 + 事件 + 绑定 + slot
- **阶段 B 验证**：4 表单组件均展示 on_change 受控绑定 + searchable/clearable 等 bool 切换
- **阶段 C 验证**：4 弹层组件均展示 slot=trigger/content 路由 + open 受控状态
- **阶段 D/E 验证**：重型组件独立设计文档 + 实施前评审

### 5.3 整体验证

- [ ] 全部 17 个组件在 demo 中可独立运行
- [ ] 所有属性在 `props_registry.rs` 中完整登记，无静默丢弃
- [ ] `cargo test -p rust-rml-engine` 全量测试通过
- [ ] demo 应用 `cargo run` 启动正常，无编译警告

---

## 6. 实施顺序（推荐）

1. **阶段 A.2 Rating**（最简单，Stateless，已部分 re-export）→ 验证属性全量映射流程
2. **阶段 A.3 OtpInput**（Stateful，复用 Input 事件订阅模式）→ 验证 state_ctor 注入机制
3. **阶段 A.1 VirtualList**（函数式构造 + slot=render）→ 验证 slot 闭包生成
4. **阶段 A.4 Resizable**（StatelessWithItems + 多层 panel）→ 验证子项 builder 模式
5. **阶段 A.5 Settings**（极复杂，多层嵌套 + slot=field）→ 综合验证
6. **阶段 B**（4 个 Stateful 表单）→ 复用 A.3 模式，快速交付
7. **阶段 C**（4 个弹层）→ 复用 A.1 slot 模式
8. **阶段 D**（Sidebar/SearchableList/Dock）→ 阶段 D 实施前需细化设计
9. **阶段 E**（Chart/Plot）→ 阶段 E 实施前需细化设计

每个阶段完成后运行 `cargo test -p rust-rml-engine` 全量回归，确保无破坏性变更。
