# RML 框架 gpui-component 完整覆盖迭代计划

> 创建日期：2026-07-07
> 目标：对照 gpui-component 官方文档与实际模块，识别 RML 框架的组件支持缺口，规划完整的声明式语法 + codegen 转移逻辑补全迭代。

---

## 一、当前状态分析

### 1.1 RML 已支持组件清单（33 个）

来源：`crates/engine/src/tags.rs` 的 `component_lookup()` 函数

| 分类 | 组件 |
|------|------|
| **基础** | Button, ButtonGroup, Badge, Tag, Label, Separator, Icon, Kbd |
| **表单** | Input, TextInput（别名）, CodeEditor（基于 Input）, Checkbox, Switch, Slider |
| **容器** | Card, Accordion, DescriptionList, Popover, AvatarGroup, TabBar |
| **数据** | Table, Tree, Breadcrumb, Avatar |
| **窗口** | TitleBar, NativeStatusBar, ActivityBar（RML 自定义）, MenuBar |
| **反馈** | Alert, Progress, ProgressCircle |
| **Re-export** | AlertDialog（gpui-component 原生 re-export） |

**注**：
- `Alert` 已完成 codegen 实现（`crates/engine/src/compiler/alert.rs` + UI 封装 + tags + props_registry + component.rs 路由），但 **demo case 文件未补全**（`demo/src/cases/alert_case.rml` + `.rml.rs` 缺失）。
- `Tooltip` 仅作为通用属性 `.tooltip()` 存在（`compiler/tooltip.rs` 处理 Button/Checkbox/Switch 的 tooltip 属性），**未注册为独立组件**。

### 1.2 gpui-component 实际模块清单

来源：`C:\Users\lusid\.cargo\git\checkouts\gpui-component-95ce574d8a0da8b8\72f26f6\crates\ui\src\lib.rs`

gpui-component 共暴露约 50 个公共模块/重导出。剔除非 UI 组件模块（animation、clipboard、history、highlighter、text、theme、plot 等 utility 模块），实际可声明式封装的 UI 组件约 32 个尚未在 RML 中支持。

### 1.3 关键发现

1. **NumberInput 与 OtpInput 确实存在**：位于 `input/number_input.rs` 和 `input/otp_input.rs`，是独立组件（非 input 模块的内部类型）。
2. **Dialog 与 AlertDialog 是两个独立组件**：`dialog/dialog.rs` 和 `dialog/alert_dialog.rs` 分别定义，应共存。
3. **Table 与 DataTable 是两个独立组件**：`table/table.rs` 和 `table/data_table.rs` 分别定义。
4. **Scrollable 是 trait 而非独立组件**：`scroll/scrollable.rs` 定义 `ScrollableElement` trait，扩展 `Div` / `Stateful<Div>` 的 `.scrollable(axis)` 方法。RML 需封装为 `<Scroll>` 容器组件。
5. **Chart 有 5 个子类型**：LineChart、AreaChart、BarChart、CandlestickChart、PieChart，均泛型 `<T, X, Y>`。
6. **List 是泛型组件**：`List<D: ListDelegate>`，需自定义 delegate 类型。
7. **Dock 极度复杂**：DockArea + DockItem + DockState + Panel + StackPanel + TabPanel + Tiles，是 VSCode 风格完整面板系统。
8. **Form 不在 component_lookup 中**：当前 RML 未注册 Form 容器组件。

---

## 二、待补全组件清单（32 个）

按复杂度与依赖关系分为 5 个 Phase。

### Phase 1：基础无状态组件（8 个）—— 简单 RenderOnce / ParentElement

| 组件 | 构造器 | 关键属性 | 复杂度 |
|------|--------|----------|--------|
| **Spinner** | `Spinner::new()` | `.icon()`, `.color()`, `size`（Sizable） | 极低 |
| **Skeleton** | `Skeleton::new()` | `.secondary()` | 极低 |
| **Link** | `Link::new(id)` | `.href()`, `.on_click()`, `.disabled()` | 低 |
| **Collapsible** | `Collapsible::new(id)` | `.open()`, 子节点 | 低 |
| **GroupBox** | `GroupBox::new()` | `.title()`, `.id()`, 子节点, `variant` | 低 |
| **Pagination** | `Pagination::new()` | `.current_page()`, `.total_pages()`, `disabled` | 低 |
| **Tooltip（独立）** | `Tooltip::new()` | `.action()`, `.key_binding()` | 低 |
| **Radio + RadioGroup** | `Radio::new(id, label)`, `RadioGroup::new(id)` | `.checked()`, `.disabled()`, 子节点 Radio | 中（含 EventEmitter） |

### Phase 2：表单输入组件（9 个）—— Stateful + EventEmitter

| 组件 | 状态类型 | 事件 | 复杂度 |
|------|----------|------|--------|
| **NumberInput** | 复用 `InputState` | `NumberInputEvent` | 中（复用 Input codegen 模式） |
| **OtpInput** | `OtpState::new(length, w, cx)` | `InputEvent` | 中 |
| **Select** | `SelectState::new()` | `SelectEvent`, `DismissEvent` | 中高 |
| **ComboBox** | `ComboboxState::new()` | `ComboboxEvent`, `DismissEvent` | 中高 |
| **ColorPicker** | `ColorPickerState::new()` | `ColorPickerEvent` | 中 |
| **DatePicker** | `DatePickerState::new()`（time 模块） | `CalendarEvent` | 中高 |
| **Calendar** | `CalendarState::new()` | `CalendarEvent` | 中 |
| **Rating** | `RatingState::new()` | `RatingEvent` | 中 |
| **Stepper** | 无（Stateless） | 无 | 低（items builder 模式） |

### Phase 3：覆盖层与容器组件（6 个）—— 中高复杂度

| 组件 | 构造器 | 特性 | 复杂度 |
|------|--------|------|--------|
| **Dialog** | `Dialog::new(cx: &mut App)` | `.trigger()`, `.title()`, `.footer()`, `on_close/on_ok/on_cancel` | 高（需 cx 参数） |
| **HoverCard** | `HoverCard::new(id)` + `HoverCardState` | `.trigger()`, `.content()` | 中 |
| **Sheet** | `Sheet::new()` | `EventEmitter<DismissEvent>`, `.title()`, `.footer()`, `.size()` | 中 |
| **Notification** | `Notification::new()` + `NotificationList` | `.message()`, `.title()`, `.with_type()` | 中（集成 ModernWindow） |
| **Scroll** | 封装 `div().scrollable(axis)` | `axis` 属性，子节点 | 中（trait 包装） |
| **Resizable** | `h_resizable(id)` / `v_resizable(id)` | `ResizablePanel` 子节点，`ResizableState` | 高（items builder） |

### Phase 4：表单容器（1 个）

| 组件 | 构造器 | 特性 | 复杂度 |
|------|--------|------|--------|
| **Form** | `Form::horizontal()` / `Form::vertical()` | `.child(Field)`, `.label_width()`, `.layout()` | 中（items builder + Field 子标签） |

### Phase 5：重型/复杂组件（8 个）—— 高复杂度

| 组件 | 特性 | 复杂度 |
|------|------|--------|
| **Sidebar** | `Sidebar<E: SidebarItem>` 泛型，`.collapsible()`, `.header()`, `.footer()` | 高 |
| **Settings** | `Settings::new(id)`, `.pages(Vec<SettingPage>)`, `.sidebar_width()` | 高 |
| **List** | `List<D: ListDelegate>` 泛型，需 delegate 类型 | 极高 |
| **SearchableList** | `SearchableListState` + delegate 模式 | 极高 |
| **VirtualList** | Stateful，`.track_scroll()`, `.with_sizing_behavior()` | 高 |
| **Chart** | 5 个子类型（Line/Area/Bar/Candlestick/Pie），泛型 `<T, X, Y>` | 极高 |
| **DataTable** | 独立于 Table，基于 delegate | 高 |
| **Dock** | DockArea + DockItem + 多面板类型，VSCode 风格 | 极高 |

---

## 三、实施规范（统一适用于所有组件）

每个组件必须完成以下 6 项交付物：

### 3.1 UI 封装层
- 文件：`crates/ui/src/components/<name>.rs`
- 内容：`pub use gpui_component::<module>::{<Type>, ...};` re-export 所需类型
- 注册：在 `crates/ui/src/components/mod.rs` 添加 `pub mod <name>;` + `pub use <name>::{...};`
- **铁律**：一个 rs 文件 = 一个组件（Radio + RadioGroup 可配对共存于 `radio.rs`，遵循 StatusBar + StatusBarItem 配对豁免）

### 3.2 Compiler Codegen 模块
- 文件：`crates/engine/src/compiler/<name>.rs`
- 内容：`pub fn gen_<name>(elem, ctx, id_counter, loop_vars) -> Result<String, CodegenError>`
- 注册：在 `crates/engine/src/compiler/mod.rs` 添加 `pub mod <name>;`
- **必须包含单元测试**：覆盖构造器选择、variant 属性、关键 setter、事件绑定、ref/id 增量
- 参考实现：
  - 简单 Stateless：`compiler/badge.rs`、`compiler/label.rs`
  - variant 关联函数：`compiler/tag.rs`、`compiler/alert.rs`
  - 非 ElementId 构造：`compiler/icon.rs`、`compiler/kbd.rs`
  - Stateful + EventEmitter：`compiler/input.rs`
  - items builder：`compiler/accordion.rs`、`compiler/table.rs`

### 3.3 Tags 注册
- 文件：`crates/engine/src/tags.rs` 的 `component_lookup()` 函数
- 添加 match 分支：`"<PascalCase>" | "<kebab-case>" => Some(ComponentTag { ... })`
- 选择正确的 `ComponentKind`：
  - `Stateless`：有 ElementId 构造器，无状态
  - `StatelessNoId`：无 ElementId（RenderOnce）
  - `Stateful { state_field, state_ctor }`：有 Entity<State> 字段
  - `StatelessWithItems`：items builder 模式（闭包或 .child()）
  - `EntityRef`：通过 Entity 引用注入

### 3.4 属性注册
- 文件：`crates/engine/src/compiler/props_registry.rs` 的 `COMPONENT_PROPS` 静态数组
- 添加 `("<CanonicalTag>", &["prop1", "prop2", ...])` 条目
- 通用属性（label/placeholder/size/disabled/on_click 等）已在 `COMMON_*_PROPS` 中，无需重复
- 仅登记组件专用属性

### 3.5 Codegen 路由
- 文件：`crates/engine/src/compiler/component.rs` 的 `gen_component()` 函数
- 在通用分发前添加专属处理分支：
  ```rust
  if tags::canonical_tag(tag) == "<CanonicalTag>" {
      return crate::compiler::<name>::gen_<name>(elem, ctx, id_counter, loop_vars);
  }
  ```
- 仅在组件需要专属 codegen 逻辑时添加（如 variant 关联函数、非标准构造器、特殊参数提取）

### 3.6 Demo 案例
- 文件：`demo/src/cases/<name>_case.rml` + `demo/src/cases/<name>_case.rml.rs`
- 注册：在 `demo/src/cases/mod.rs` 添加 `#[path = "<name>_case.rml.rs"] pub mod <name>_case;`
- 内容要求：
  - 展示所有 variant / 关键属性
  - 演示 `#[computed]` 条件渲染 / 动态绑定
  - 演示 `#[command]` 事件处理
  - 包含示例代码 Tab（`.rml` + `.rml.rs` 源码展示）
  - 包含 API 表格（使用 `build_api_table` 辅助函数）
- i18n：在 `demo/locales/*.toml` 添加 `case.<name>.title` 条目

---

## 四、分阶段实施计划

### Phase 1：基础无状态组件（8 个组件，预计 2-3 天）

**目标**：补全所有简单 Stateless / StatelessNoId 组件，建立稳定的简单组件实现模式。

**实施顺序**（按依赖与难度递增）：

1. **Spinner**（StatelessNoId, RenderOnce）
   - 构造：`Spinner::new()`
   - 属性：`icon`（IconName）、`color`（Hsla）、`size`（通用 Sizable）
   - codegen 模式：参考 `icon.rs`（无 ElementId RenderOnce）

2. **Skeleton**（StatelessNoId, RenderOnce）
   - 构造：`Skeleton::new()`
   - 属性：`secondary`（布尔标志）
   - codegen 模式：参考 `badge.rs`

3. **Link**（Stateless, ParentElement）
   - 构造：`Link::new(id)`
   - 属性：`href`、`disabled`、`on_click`
   - codegen 模式：标准 Stateless + ParentElement 子节点

4. **Collapsible**（Stateless, ParentElement）
   - 构造：`Collapsible::new(id)`
   - 属性：`open`（bind/static）
   - codegen 模式：标准 Stateless + ParentElement

5. **GroupBox**（Stateless, ParentElement）
   - 构造：`GroupBox::new()`
   - 属性：`title`、`id`、`variant`（GroupBoxVariants）
   - codegen 模式：StatelessNoId + ParentElement

6. **Pagination**（Stateless, Disableable + Sizable）
   - 构造：`Pagination::new()`
   - 属性：`current_page`、`total_pages`、`on_change`（页码变更事件）
   - codegen 模式：StatelessNoId + 事件绑定

7. **Tooltip（独立组件）**
   - 构造：`Tooltip::new()`
   - 属性：`action`、`key_binding`
   - 注意：保留现有 `.tooltip()` 通用属性处理，新增独立 `<Tooltip>` 标签
   - codegen 模式：StatelessNoId RenderOnce

8. **Radio + RadioGroup**（Stateless + ParentElement + EventEmitter）
   - 构造：`Radio::new(id, label)`、`RadioGroup::new(id)`
   - 属性：`Radio`: `label`、`checked`、`disabled`、`value`；`RadioGroup`: `on_change`
   - 事件：RadioGroup emits `ChangeEvent<usize>`
   - codegen 模式：RadioGroup 为容器（StatelessWithItems），子节点 Radio 为 items builder
   - 单文件 `radio.rs`（配对豁免）

**验证**：
- `cargo build -p rust-rml-ui` 成功
- `cargo build -p rust-rml-engine` 成功
- `cargo test -p rust-rml-engine -- <component>` 各组件测试通过
- `cargo test -p rust-rml-engine --test props_registry_complete` 通过
- `cargo build -p rust-rml-demo` 成功

### Phase 2：表单输入组件（9 个组件，预计 4-5 天）

**目标**：补全所有 Stateful 表单输入组件，复用 Input 已有的 EventEmitter 订阅模式。

**实施顺序**：

1. **NumberInput**（Stateful, 复用 InputState）
   - 构造：`NumberInput::new(&Entity<InputState>)`
   - 状态：复用 `input_state` 字段（与 Input 相同）
   - 事件：`NumberInputEvent`（由 InputState 发出）
   - codegen 模式：参考 `code_editor.rs`（复用 InputState 的延迟初始化）
   - 关键差异：需在 `cx.subscribe` 回调中处理 NumberInputEvent 而非 InputEvent

2. **OtpInput**（Stateful, 独立 OtpState）
   - 构造：`OtpState::new(length, w, cx)` + `OtpInput::new(&Entity<OtpState>)`
   - 状态：`otp_state: Option<Entity<OtpState>>`，`otp_length: usize`（构造器参数）
   - 事件：`InputEvent`
   - 属性：`length`（构造器参数，必需）、`default_value`

3. **Rating**（Stateful, RatingState）
   - 构造：`RatingState::new()` + `Rating::new(&Entity<RatingState>)`
   - 属性：`value`（bind）、`max`、`allow_half`、`disabled`
   - 事件：`RatingEvent`

4. **Stepper**（Stateless, items builder）
   - 构造：`Stepper::new(id)`
   - 属性：`current`、`direction`（horizontal/vertical）
   - 子节点：`<StepperItem>`（item builder，参考 AccordionItem 模式）
   - codegen 模式：参考 `accordion.rs`（StatelessWithItems + 闭包 builder）

5. **ColorPicker**（Stateful, ColorPickerState）
   - 构造：`ColorPickerState::new()` + `ColorPicker::new(&Entity<ColorPickerState>)`
   - 属性：`default_value`、`placeholder`
   - 事件：`ColorPickerEvent`

6. **Calendar**（Stateful, CalendarState）
   - 构造：`CalendarState::new()` + `Calendar::new(&Entity<CalendarState>)`
   - 属性：`disabled_matcher`、`year_range`
   - 事件：`CalendarEvent`
   - 注意：位于 `time/calendar.rs`，需 `pub use gpui_component::time::calendar::{Calendar, CalendarState};`

7. **DatePicker**（Stateful, DatePickerState）
   - 构造：`DatePickerState::new()` + `DatePicker::new(&Entity<DatePickerState>)`
   - 属性：`placeholder`、`cleanable`、`default_value`
   - 事件：DatePicker 事件
   - 注意：位于 `time/date_picker.rs`，需 `pub use gpui_component::time::date_picker::{DatePicker, DatePickerState};`

8. **Select**（Stateful, SelectState）
   - 构造：`SelectState::new()` + `Select::new(&Entity<SelectState>)`
   - 属性：`placeholder`、`menu_width`、`menu_max_h`、`icon`、`value`（bind）
   - 事件：`SelectEvent`、`DismissEvent`
   - 复杂度中高：需处理选项数据绑定（可能为 `Vec<SelectItem>` 或闭包式构建）

9. **ComboBox**（Stateful, ComboboxState）
   - 构造：`ComboboxState::new()` + `Combobox::new(&Entity<ComboboxState>)`
   - 属性：同 Select + `search_placeholder`
   - 事件：`ComboboxEvent`、`DismissEvent`

**验证**：同 Phase 1，另需运行各组件 demo 案例验证交互。

### Phase 3：覆盖层与容器组件（6 个组件，预计 4-5 天）

**目标**：补全 overlay 与特殊容器组件。

1. **Dialog**（特殊：cx 参数构造）
   - 构造：`Dialog::new(cx: &mut App)` —— **注意**：需 cx 参数，与其他组件不同
   - 属性：`trigger`（slot）、`title`、`footer`、`on_close`、`on_ok`、`on_cancel`
   - codegen 挑战：构造器需要 `cx: &mut App`，codegen 需在 render 闭包内调用 `Dialog::new(cx)`
   - 子节点：`<template slot="trigger">` + content 子节点
   - 与 AlertDialog 共存（AlertDialog 已 re-export，Dialog 独立实现）

2. **HoverCard**（StatelessWithItems, HoverCardState） ✅ 完成
   - 构造：`HoverCard::new(id)` —— Stateless 容器，trigger slot + content 子节点
   - 属性：`anchor`（8 方向）、`appearance`（bool）、`open_delay`（ms→Duration）、`close_delay`（ms→Duration）
   - codegen 模式：参考 `popover.rs`（slot 路由 `.trigger()` + content 注入 `.child()`）
   - 实现文件：`crates/ui/src/components/hover_card.rs`、`crates/engine/src/compiler/components/hover_card/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/hover_card.rs`
   - Demo：`demo/src/cases/hover_card_case.rml` + `.rml.rs`（6 个 demo section：基础用法/anchor 定位/顶部锚点/延迟控制/样式控制/富内容）
   - 测试：7 个 codegen 单元测试 + 6 个 setter 单元测试，全 workspace 1470 tests passed

3. **Sheet**（Stateless, EventEmitter<DismissEvent>） ✅ 完成
   - 构造：`Sheet::new(_: &mut Window, cx: &mut App)` —— codegen 生成 `Sheet::new(_window, cx)` 使用 render 上下文变量
   - 属性：`title`（string）、`footer`（string）、`size`（px/%/裸数字）、`resizable`（bool）、`overlay`（bool）、`overlay_closable`（bool）、`on_close`（event，cx.listener 桥接）
   - ParentElement：子节点通过 `.child()` / `.children()` 注入为 content
   - 实现文件：`crates/ui/src/components/sheet.rs`、`crates/engine/src/compiler/components/sheet/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/sheet.rs`
   - Demo：`demo/src/cases/sheet_case.rml` + `.rml.rs`（5 个 demo section：基础用法/尺寸控制/页脚/交互控制/富内容）
   - 测试：8 个 codegen 单元测试 + 13 个 setter 单元测试，全 workspace 1491 tests passed

4. **Dialog**（Stateless, ParentElement, EventEmitter<DismissEvent>） ✅ 完成
   - 构造：`Dialog::new(cx: &mut App)` —— codegen 生成 `Dialog::new(cx)` 使用 render 上下文变量（仅需 cx，不需要 _window）
   - 属性：`title`（string）、`footer`（string）、`width`（px/裸数字）、`overlay`（bool）、`overlay_closable`（bool）、`close_button`（bool）、`keyboard`（bool）、`on_close`（event，cx.listener 桥接）、`on_ok`（event，entity 捕获闭包返回 true）、`on_cancel`（event，entity 捕获闭包返回 true）
   - slot="trigger" → `.trigger()`，其余子节点 → `.child()` / `.children()`（ParentElement）
   - 注意：仅 PascalCase `<Dialog>` 为本组件，小写 `<dialog>` 为 `RootTag::DialogWindow`
   - on_ok/on_cancel 特殊处理：签名 `Fn(&ClickEvent, &mut Window, &mut App) -> bool`，`cx.listener()` 无法适配（不返回 bool），改用 `cx.entity()` 捕获 entity + `entity.update()` 调用方法，固定返回 `true`
   - 实现文件：`crates/engine/src/compiler/components/dialog/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/dialog.rs`
   - Demo：`demo/src/cases/dialog_case.rml` + `.rml.rs`（5 个 demo section：基础用法/宽度控制/页脚/交互控制/富内容）
   - 测试：10 个 codegen 单元测试 + 15 个 setter 单元测试，全 workspace tests passed

5. **AlertDialog**（Stateless, ParentElement, RenderOnce） ✅ 完成
   - 构造：`AlertDialog::new(cx: &mut App)` —— codegen 生成 `AlertDialog::new(cx)` 使用 render 上下文变量
   - 与 Dialog 的区别已明确消除二义性：
     - Dialog 默认 `close_button(true)` + `overlay_closable(true)`，通用模态对话框
     - AlertDialog 默认 `close_button(false)` + `overlay_closable(false)`，警示确认场景
     - AlertDialog 专属方法：`.description()` / `.confirm()` / `.show_cancel()`
     - AlertDialog footer 按钮居中对齐，Dialog 右对齐
   - 属性：`title`、`description`（专属）、`width`、`confirm`（布尔属性）、`show_cancel`、`overlay_closable`、`close_button`、`keyboard`、`on_close`、`on_ok`、`on_cancel`
   - slot="trigger" → `.trigger()`，其余子节点 → `.child()` / `.children()`
   - **根标签 `<dialog>` 修复**：从 `window.open_alert_dialog()` 改为 `window.open_dialog()`，使用 Dialog（非 AlertDialog）作为底层，语义正确
   - 实现文件：`crates/engine/src/compiler/components/alert_dialog/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/alert_dialog.rs`
   - 文档：`crates/ui/src/components/alert_dialog.rs` 添加 AlertDialog vs Dialog 对比表
   - Demo：`demo/src/cases/alert_dialog_case.rml` + `.rml.rs`（5 个 demo section：基础用法/确认对话框/宽度控制/交互控制/与 Dialog 对比）
   - 测试：8 个 codegen 单元测试 + 15 个 setter 单元测试，全 workspace tests passed

6. **Notification**（特殊：NotificationTrigger 包装器） ✅ 完成
   - 构造：`NotificationTrigger::new()`（RenderOnce 无 ElementId、无 cx 参数）
   - **设计原因**：`Notification` 实现 `Render`（非 `RenderOnce`），通过 `window.push_notification()` 命令式推送，无法直接作为 RML 组件
   - **NotificationTrigger 包装器**：存储 title/message/type/autohide 字段，包裹 `slot="trigger"` 子元素，点击时构造 `Notification` 并调用 `window.push_notification()` 推送
   - 属性：`title`（string）、`message`（string）、`success`/`info`/`warning`/`error`（独立布尔属性 → `.with_type(NotificationType::X)`）、`autohide`（默认 true，`autohide=false` 关闭）
   - 子节点：`slot="trigger"` → `.trigger()`，不支持其余子节点（NotificationTrigger 不实现 ParentElement）
   - 实现文件：`crates/ui/src/components/notification_trigger.rs`、`crates/engine/src/compiler/components/notification/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/notification.rs`
   - Demo：`demo/src/cases/notification_case.rml` + `.rml.rs`（5 个 demo section：基础用法/通知类型/仅消息/禁用自动隐藏/不同触发器）
   - 测试：10 个 codegen 单元测试 + 13 个 setter 单元测试，全 workspace 1259 tests passed

7. **Scroll**（特殊：trait 包装） ✅ 完成
   - **设计决策**：封装为 `<Scroll>` 容器组件，包装 gpui-component 的 `ScrollableElement` trait
   - 构造：`Scroll::new()`（RenderOnce 无 ElementId、无 cx，ParentElement）
   - **底层实现**：`RenderOnce::render()` 中根据 axis 调用 `div().overflow_y_scrollbar()` / `.overflow_x_scrollbar()` / `.overflow_scrollbar()`，返回 `Scrollable<Div>` 包装器（自带滚动条 UI）
   - 属性：`vertical` / `horizontal` / `both`（独立布尔属性 → `.vertical()` / `.horizontal()` / `.both()`，默认 vertical）
   - 子节点：通过 `.child()` / `.children()` 注入（ParentElement）
   - 实现文件：`crates/ui/src/components/scroll.rs`、`crates/engine/src/compiler/components/scroll/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/scroll.rs`
   - Demo：`demo/src/cases/scroll_case.rml` + `.rml.rs`（4 个 demo section：垂直滚动/水平滚动/双向滚动/嵌套使用）
   - 测试：7 个 codegen 单元测试 + 7 个 setter 单元测试，全 workspace 1273 tests passed

8. **Resizable**（Stateful, items builder）
   - 构造：`h_resizable(id)` / `v_resizable(id)`（关联函数选择方向）
   - 状态：`ResizableState`
   - 子节点：`<ResizablePanel>`（item builder，含 `size_range`、`default_size`）
   - 属性：`direction`（horizontal/vertical，等价于 h_/v_ 关联函数选择）
   - codegen 模式：参考 `tab_bar.rs`（variant 关联函数 + items builder）

**验证**：同前，另需手动验证 Dialog/Sheet/HoverCard 的弹出交互。

### Phase 4：表单容器（1 个组件，预计 1-2 天）

1. **Form + Field**（StatelessNoId, ParentElement） ✅ 完成
   - **Form** 构造：`Form::vertical()`（默认）/ `Form::horizontal()`（horizontal 属性切换）
   - **Form** 属性：`horizontal`/`vertical`（独立布尔属性，构造器选择）、`label-width`（像素值）、`label-text-size`（rems）、`columns`（列数）
   - **Form** 子节点：`.child(impl Into<Field>)`，子节点必须为 `<Field>` 元素（空白文本被忽略）
   - **Field** 构造：`Field::new()`（RenderOnce + ParentElement）
   - **Field** 属性：`label`、`description`、`required`（布尔，默认 false）、`visible`（默认 true）、`label-indent`、`col-span`、`col-start`、`col-end`
   - **Field** 子节点：任意 `AnyElement`（Switch、Checkbox、Button、Label 等表单控件）
   - 实现文件：`crates/ui/src/components/form.rs`、`crates/engine/src/compiler/components/form/{gen,setters,mod}.rs`、`crates/engine/src/compiler/components/field/{gen,setters,mod}.rs`、`crates/engine/src/compiler/translator/component/{form,field}.rs`
   - Demo：`demo/src/cases/form_case.rml` + `.rml.rs`（5 个 demo section：垂直布局/水平布局/必填与描述/多列布局/表单控件组合）
   - 测试：38 个单元测试（8 form setters + 8 form gen + 13 field setters + 9 field gen），全 workspace 1311 tests passed

### Phase 5：重型/复杂组件（8 个组件，预计 10+ 天）

**目标**：补全所有高复杂度组件，完成 gpui-component 全覆盖。

1. **Sidebar**（泛型 Sidebar<E: SidebarItem>）
   - 挑战：泛型参数 E 需在 RML 中具象化
   - 方案：RML 封装为非泛型 `Sidebar`，内部使用 `dyn SidebarItem` 或预定义常用 SidebarItem 类型
   - 属性：`collapsible`、`header`（slot）、`footer`（slot）
   - 子节点：`<SidebarItem>` / `<SidebarGroup>`

2. **Settings**（Stateful）
   - 构造：`Settings::new(id)`
   - 属性：`sidebar_width`、`pages`（bind Vec<SettingPage>）
   - 挑战：SettingPage 是复杂结构体，需 RML 声明式构建或 bind 数据

3. **List**（泛型 List<D: ListDelegate>，完全声明式构建）
   - **用户决策**：通过 `<ListItem>` 子标签声明式构建 delegate，RML codegen 自动生成 delegate impl
   - 设计：
     - RML 框架内置 `RmlListDelegate` 类型（实现 `ListDelegate` trait）
     - `<List>` 标签的子节点 `<ListItem>` 在 codegen 时收集为 `Vec<AnyElement>` + 元数据
     - 生成代码：`ListState::new(RmlListDelegate::new(items, render_fn), w, cx)`
   - 属性：`search_placeholder`、`sizing_behavior`
   - 子节点：`<ListItem>`（含 `text`、`icon`、`on_click` 等）
   - 挑战：render_fn 闭包需捕获 .rml.rs 的 computed 方法，codegen 需生成闭包

4. **SearchableList**（delegate 模式，完全声明式构建）
   - 同 List 模式，内置 `RmlSearchableListDelegate`
   - 属性：`search_placeholder`、`searchable`
   - 子节点：`<ListItem>`

5. **VirtualList**（Stateful）
   - 构造：`VirtualList::new(state)` 
   - 属性：`track_scroll`、`sizing_behavior`
   - 挑战：需定义 VirtualListScrollHandle 状态管理

6. **Chart**（5 个子类型，泛型 <T, X, Y>）
   - 挑战：5 个独立图表组件 + 泛型数据绑定
   - 方案：每个图表类型独立标签 `<LineChart>` / `<AreaChart>` / `<BarChart>` / `<CandlestickChart>` / `<PieChart>`
   - 属性：`data`（bind Vec<T>）、`x`（字段名，codegen 生成闭包）、`y`（字段名）、`stroke`、`fill` 等

7. **DataTable**（独立于 Table）
   - 与 Table 共存，基于 delegate 模式
   - 声明式语法偏向 WPF DataGrid 风格（用户已确认方向）

8. **Dock**（极复杂）
   - DockArea + DockItem + Panel + StackPanel + TabPanel + Tiles
   - 挑战：完整的 VSCode 风格面板系统，状态管理复杂
   - 方案：可能需要 RML 层重新设计声明式语法，而非直接 1:1 映射

**验证**：每个组件独立验证 + 整体 workspace 构建 + demo 运行时验证。

---

## 五、补全 Alert Demo Case（前置任务）

Alert 组件 codegen 已实现，但 demo case 缺失。作为本计划的第 0 步先行补全：

- 创建 `demo/src/cases/alert_case.rml`：展示 4 个 variant（info/success/warning/error）、title、banner、on_close 事件、size 变化
- 创建 `demo/src/cases/alert_case.rml.rs`：struct + ILifecycle + computed（rml_sample, rust_sample）+ command（on_close）
- 注册到 `demo/src/cases/mod.rs`
- 添加 i18n 条目 `case.alert.title`
- 验证：`cargo build -p rust-rml-demo`

---

## 六、关键决策与假设

### 6.1 已确认决策（来自用户先前的澄清）

1. **差异化判断策略**：已有 RML 自定义版本的组件（CodeEditor vs Editor / Table vs DataTable / AlertDialog vs Dialog）按组件差异化判断，不替换而是共存。
2. **优先级排序**：按官方文档分组顺序。
3. **重型组件纳入但置后**：Sidebar/DataTable/VirtualList/Chart/Dock 等纳入本计划，但置于 Phase 5。
4. **Table 与 DataTable 共存**：RML Table 偏向 WPF DataGrid 风格，DataTable 同样走 WPF 风格优化。

### 6.2 本计划新增决策

1. **Scrollable 映射为独立 `<Scroll>` 容器组件**（而非通用属性），符合 WPF 风格。
2. **Tooltip 独立组件与 `.tooltip()` 通用属性共存**：保留现有通用属性，新增 `<Tooltip>` 标签。
3. **List/SearchableList 泛型处理**：RML 声明式仅生成 `List::new(&Entity)`，delegate 类型由 .rml.rs 后端定义。
4. **Chart 拆分为 5 个独立标签**：`<LineChart>` / `<AreaChart>` / `<BarChart>` / `<CandlestickChart>` / `<PieChart>`。
5. **NumberInput 复用 InputState**：与 Input 共享状态字段，codegen 参考 CodeEditor 模式。
6. **不纳入的模块**：animation、clipboard、history、highlighter、text、theme、plot（utility/内部模块，非独立 UI 组件）。

### 6.3 已确认的开放问题（用户决策）

1. **Dock 组件纳入 Phase 5 但允许推迟**：保留在 Phase 5 计划中，若实施时发现阻塞其他组件，可独立推迟到下一迭代，不阻塞其他 7 个重型组件。
2. **List / SearchableList 完全声明式构建**：通过 `<ListItem>` 子标签声明式构建 delegate，RML codegen 自动生成 delegate impl。RML 框架需内置一个泛型 `RmlListDelegate`，从 `<ListItem>` 子节点收集数据，codegen 生成 `ListState::new(RmlListDelegate::new(items), w, cx)`。
3. **Form 的 Field 作为布局容器标签**：`<Field label="用户名" name="username"><Input /></Field>`，Field 内嵌任意表单组件，自动布局 label + content。Field 是 Form 的子项 builder，在 RML 中作为独立标签 `<Field>` 注册。

---

## 七、风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| Dialog 构造器需 `cx: &mut App` 参数 | codegen 需特殊处理 | 在 render 闭包内调用 `Dialog::new(cx)`，与其他组件构造时机不同 |
| List/Chart 泛型参数 | RML 声明式难以表达泛型 | 采用"RML 生成 Entity 引用 + 后端定义 delegate"模式 |
| Dock 复杂度过高 | 可能阻塞 Phase 5 | 允许 Dock 推迟到独立迭代，不阻塞其他组件 |
| Notification 与 ModernWindow 集成 | 需修改 ModernWindow 内部实现 | 在 Phase 3 实施前先设计 helper trait API |
| gpui-component 版本更新 | API 可能变化 | 锁定 git checkout 版本，定期同步 |

---

## 八、验证清单

每个 Phase 完成后必须通过：

- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] `cargo test -p rust-rml-engine` 全部通过
- [ ] `cargo test -p rust-rml-engine --test props_registry_complete` 通过
- [ ] 新增组件的 demo case 在 demo 应用中可正常渲染与交互
- [ ] `crates/engine/src/compiler/mod.rs` 与 `crates/ui/src/components/mod.rs` 仅做 re-export，无业务代码
- [ ] 每个组件独占一个 rs 文件（配对豁免除外）
- [ ] 无 `rml_` 前缀标识符
