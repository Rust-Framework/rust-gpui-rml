# RML 框架 gpui-component 组件完整支持迭代计划

> 参考来源：`https://longbridge.github.io/gpui-component/zh-CN/docs/components/`
> 计划制定日期：2026-07-07

## Summary

基于 gpui-component 官方组件清单（22 个组件）与 RML 框架当前支持情况的对比分析，规划 4 个阶段补齐 12 个缺失组件的声明式语法支持、属性转移逻辑（codegen）与 UI 封装。已支持的 10 个 gpui-component 组件保持现状；针对已有"近似实现"的组件（CodeEditor / Table / AlertDialog）按差异化判断：互补共存或独立新增。重型组件（Chart / DataTable / VirtualList / Sidebar）纳入规划但置后于迭代后期阶段。

---

## 1. Current State Analysis（现状分析）

### 1.1 gpui-component 官方组件清单（22 个，按文档分组）

| 分组 | 组件 |
|------|------|
| 基础组件 (9) | Accordion, Alert, Avatar, Badge, Button, Checkbox, Icon, Image, Tooltip |
| 表单组件 (8) | Input, Select, NumberInput, DatePicker, OtpInput, ColorPicker, Editor, Form |
| 布局与高级组件 (9) | Dialog, Popover, Resizable, Scrollable, Sidebar, Chart, DataTable, Tree, VirtualList |

### 1.2 RML 已支持的 gpui-component 组件（10 个）

通过 [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 的 `component_lookup` 注册表与 [compiler/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/) 目录确认：

| 组件 | 标签 | ComponentKind | 实现位置 |
|------|------|--------------|---------|
| Accordion | `<Accordion>` / `<accordion>` | StatelessWithItems | [compiler/accordion/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/) |
| Avatar | `<Avatar>` | StatelessNoId | [compiler/avatar/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/avatar/) |
| AvatarGroup | `<AvatarGroup>` | StatelessNoId (container) | 同上 |
| Badge | `<Badge>` | StatelessNoId (container) | [compiler/badge/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/badge/) |
| Button | `<Button>` | Stateless | [component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) |
| ButtonGroup | `<ButtonGroup>` | Stateless (container) | 同上 |
| Checkbox | `<Checkbox>` | Stateless | 同上 |
| Icon | `<Icon>` | StatelessNoId | [compiler/icon.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/icon.rs) |
| Input | `<Input>` / `<TextInput>` | Stateful (`InputState`) | [compiler/input/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/input/) |
| Popover | `<Popover>` / `<popover>` | StatelessWithItems | [compiler/popover.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/popover.rs) |
| Tooltip | 作为通用 `tooltip` 属性 | （非独立标签） | [compiler/tooltip.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tooltip.rs) |
| Tree | `<Tree>` | Stateful (`TreeState`) | [compiler/tree/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tree/) |

**附加封装**：CodeEditor（基于 `InputState` 的代码编辑器封装，mono 字体 + 默认高度）→ [compiler/code_editor/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/)

### 1.3 RML 自定义组件（非 gpui-component 路由）

ActivityBar / Breadcrumb / Card / DescriptionList / Kbd / Label / MenuBar / NativeStatusBar / Progress / ProgressCircle / Separator / Slider / StatusBar / Switch / TabBar / Table（WPF DataGrid 风格）/ Tag / TitleBar / AlertDialog（gpui-component AlertDialog 的 re-export）

### 1.4 缺失的 gpui-component 组件（12 个）

| 分组 | 缺失组件 |
|------|---------|
| 基础组件 (2) | Alert, Image |
| 表单组件 (7) | Select, NumberInput, DatePicker, OtpInput, ColorPicker, Editor, Form |
| 布局与高级组件 (5) | Dialog, Resizable, Scrollable, Sidebar, Chart, DataTable, VirtualList（共 7 个；其中 Sidebar/Chart/DataTable/VirtualList 为重型组件）|

### 1.5 关键依赖

- [Cargo.toml](file:///d:/GitCode/RF/rust-gpui-rml/Cargo.toml) 第 18 行：`gpui-component = { git = "https://github.com/longbridge/gpui-component.git", features = ["tree-sitter-languages"] }` — git 依赖，包含最新组件
- [crates/ui/Cargo.toml](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/Cargo.toml) 第 15 行：`ui-components` feature 控制扩展组件启用

### 1.6 已有"近似实现"的差异化判断

| RML 现状 | gpui-component 对应 | 判断 | 依据 |
|----------|-------------------|------|------|
| CodeEditor（基于 InputState 封装） | Editor（独立多行编辑器） | **互补共存** | CodeEditor 是 Input 的代码模式封装；Editor 是 gpui-component 独立组件，定位互补 |
| Table（WPF DataGrid 风格声明式） | DataTable（高性能数据表格） | **共存，各自定位** | Table 已是 gpui-component Table 的 WPF 风格封装；DataTable 走同样路径基于 gpui-component DataTable，提供 WPF 风格声明式 API；用户明确表示"声明式语法做进一步优化"以简洁覆盖原生控件能力 |
| AlertDialog（gpui-component AlertDialog re-export） | Dialog（gpui-component Dialog） | **共存** | 二者均为 gpui-component 独立组件，RML 当前仅暴露 AlertDialog，需新增 Dialog 标签 |

---

## 2. 实施规范（每个组件的统一模板）

每个新增组件需按以下顺序完成 6 项工作：

1. **UI 封装层**：`crates/ui/src/components/<component>.rs`（如非薄 re-export，需独立文件；薄 re-export 可合并到相关文件，但要遵循"一个 rs 文件 = 一个组件 / 一个职责"铁律）
2. **编译器模块**：`crates/engine/src/compiler/<component>/mod.rs`（含 `gen_<component>`、`setters.rs`、必要时 `item.rs` / `gen.rs`）
3. **标签路由注册**：[tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 的 `component_lookup` 函数添加 match 臂
4. **属性注册登记**：[props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) 的 `COMPONENT_PROPS` 添加属性清单
5. **codegen 路由**：[component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 在 `gen_component` 中按 ComponentKind 添加分支（如需专属处理）或委托到 `<component>::gen_<component>` 模块
6. **演示案例**：`demo/src/cases/<component>_case.rml.rs` + [demo/src/cases/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 注册

每个组件需明确以下元信息（用于 `ComponentTag` 注册）：
- PascalCase 标签名 + kebab-case 别名（如适用）
- `ComponentKind`：Stateless / StatelessNoId / Stateful / StatelessWithItems / EntityRef
- `container`：是否实现 `ParentElement`（支持 `.child()`）
- 属性分类清单：static（静态）/ bind（绑定）/ event（事件，使用 `on-kebab-case` 声明式）

---

## 3. Proposed Changes（迭代阶段详细规划）

### 阶段 1：基础组件补全（2 个组件）

#### 1.1 Alert 组件

- **标签**：`<Alert>` / `<alert>`（PascalCase + 小写别名，参考 Accordion 模式）
- **ComponentKind**：`StatelessWithItems`（多 variant 子项：`<AlertItem>` 或对应 `info`/`success`/`warning`/`error` variant）
- **container**：`false`
- **属性**：
  - static: `variant`（info/success/warning/error）、`title`、`description`、`closable`、`show_icon`、`bordered`
  - bind: `title`、`description`、`closable`
  - event: `on_close`
- **实现路径**：
  - `crates/ui/src/components/alert.rs`（re-export `gpui_component::alert::*`，类型导出）
  - `crates/engine/src/compiler/alert/mod.rs`、`setters.rs`、`item.rs`（如需）
  - [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Alert" | "alert"` 路由
- **验证**：`cargo test -p rust-rml-engine alert`、新增 `alert_case.rml.rs` 展示 4 个 variant + 关闭事件

#### 1.2 Image 组件

- **标签**：`<Image>` / `<img>`（注意：`<img>` 当前为内置 HTML 标签，需扩展为 gpui-component Image，参考 Input 双轨制）
- **ComponentKind**：`StatelessNoId`（gpui-component Image 通常为 RenderOnce）
- **container**：`false`
- **属性**：
  - static: `src`（URL 或路径）、`alt`（回退文本）、`fallback`（图标名或图片）、`object_fit`（cover/contain/fill）
  - bind: `src`、`fallback`
- **实现路径**：
  - `crates/ui/src/components/image.rs`（封装 gpui-component Image，处理 src 字符串 → `IconName` / 路径的统一加载）
  - `crates/engine/src/compiler/image.rs`（参考 `compiler/icon.rs` 模式）
  - [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Image"` 路由（`<img>` 维持原生 div 行为，避免破坏现有用法）
- **验证**：新增 `image_case.rml.rs` 展示本地路径 + HTTP URL + fallback

---

### 阶段 2：表单组件补全（7 个组件）

#### 2.1 Select 组件

- **标签**：`<Select>` / `<select>`
- **ComponentKind**：`Stateful { state_field: "select_state", state_ctor: "|w, c| rml_ui::SelectState::new(w, c)" }`（参考 Input 模式）
- **container**：`false`
- **属性**：
  - static: `placeholder`、`size`、`disabled`、`clearable`、` searchable`、`multiple`、`max_count`
  - bind: `value`、`options`、`disabled`
  - event: `on_change`（通过 EventEmitter + `cx.subscribe` 模式，参考 Input）
- **实现路径**：
  - `crates/ui/src/components/select.rs`（re-export + SelectState 封装）
  - `crates/engine/src/compiler/select/mod.rs`、`setters.rs`、`event.rs`
  - [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"Select" | "select"` 路由
  - 参考 [compiler/input/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/input/) 实现事件订阅 block 表达式
- **验证**：新增 `select_case.rml.rs` 展示单选 + 多选 + searchable + on_change 绑定

#### 2.2 NumberInput 组件

- **标签**：`<NumberInput>` / `<number-input>`
- **ComponentKind**：`Stateful { state_field: "number_state", state_ctor: "|w, c| rml_ui::NumberInputState::new(w, c)" }`
- **container**：`false`
- **属性**：
  - static: `placeholder`、`size`、`disabled`、`min`、`max`、`step`、`precision`、`prefix`、`suffix`
  - bind: `value`、`min`、`max`、`disabled`、`prefix`、`suffix`
  - event: `on_change`（EventEmitter 模式）
- **实现路径**：
  - `crates/ui/src/components/number_input.rs`
  - `crates/engine/src/compiler/number_input/mod.rs`、`setters.rs`
  - [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)：添加 `"NumberInput" | "number-input"` 路由
- **验证**：新增 `number_input_case.rml.rs` 展示 min/max 限制 + 步进按钮 + 双向绑定

#### 2.3 DatePicker 组件

- **标签**：`<DatePicker>` / `<date-picker>`
- **ComponentKind**：`Stateful { state_field: "date_picker_state", state_ctor: "|_w, c| rml_ui::DatePickerState::new(c)" }`
- **container**：`false`
- **属性**：
  - static: `placeholder`、`size`、`disabled`、`format`、`mode`（date/datetime/month）、`week_start`、`show_time`
  - bind: `value`、`disabled`
  - event: `on_change`（EventEmitter 模式）
- **实现路径**：
  - `crates/ui/src/components/date_picker.rs`
  - `crates/engine/src/compiler/date_picker/mod.rs`、`setters.rs`
- **验证**：新增 `date_picker_case.rml.rs` 展示日期选择 + 月份选择 + 范围限制

#### 2.4 OtpInput 组件

- **标签**：`<OtpInput>` / `<otp-input>`
- **ComponentKind**：`Stateful { state_field: "otp_state", state_ctor: "|_w, c| rml_ui::OtpState::new(c)" }`（或 StatelessNoId，取决于 gpui-component 实现）
- **container**：`false`
- **属性**：
  - static: `length`（位数）、`size`、`disabled`、`mask`、`type`（text/number）
  - bind: `value`、`disabled`
  - event: `on_change`、`on_complete`
- **实现路径**：
  - `crates/ui/src/components/otp_input.rs`
  - `crates/engine/src/compiler/otp_input.rs`（单文件，逻辑简单）
- **验证**：新增 `otp_input_case.rml.rs` 展示 6 位 OTP + mask 模式 + on_complete 事件

#### 2.5 ColorPicker 组件

- **标签**：`<ColorPicker>` / `<color-picker>`
- **ComponentKind**：`Stateful { state_field: "color_picker_state", state_ctor: "|_w, c| rml_ui::ColorPickerState::new(c)" }`
- **container**：`false`
- **属性**：
  - static: `size`、`disabled`、`show_alpha`、`show_hex`、`default_color`
  - bind: `value`、`disabled`
  - event: `on_change`
- **实现路径**：
  - `crates/ui/src/components/color_picker.rs`
  - `crates/engine/src/compiler/color_picker/mod.rs`、`setters.rs`
- **验证**：新增 `color_picker_case.rml.rs` 展示颜色选择 + 透明度 + hex 输入

#### 2.6 Editor 组件

- **标签**：`<Editor>`（保留 `<CodeEditor>` 作为 Input 封装共存）
- **ComponentKind**：`Stateful { state_field: "editor_state", state_ctor: "|_w, c| rml_ui::EditorState::new(c)" }`
- **container**：`false`
- **属性**：
  - static: `language`（rust/json/markdown/...）、`readonly`、`show_line_numbers`、`word_wrap`、`height`
  - bind: `value`、`language`
  - event: `on_change`、`on_focus`、`on_blur`
- **实现路径**：
  - `crates/ui/src/components/editor.rs`（明确区别于 [code_editor](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/)）
  - `crates/engine/src/compiler/editor/mod.rs`、`setters.rs`、`event.rs`
  - 参考 [compiler/code_editor/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/) 但独立模块（避免与现有 CodeEditor 混淆）
- **验证**：新增 `editor_case.rml.rs` 展示多语言 + readonly + 行号 + 双向绑定

#### 2.7 Form 组件

- **标签**：`<Form>` / `<form>`
- **ComponentKind**：`Stateless`（容器，参考 Card 模式）
- **container**：`true`
- **属性**：
  - static: `layout`（vertical/horizontal/inline）、`label_align`（left/right）、`label_width`、`required_mark`
  - bind: `model`（表单数据模型）、`errors`
  - event: `on_submit`、`on_change`
- **实现路径**：
  - `crates/ui/src/components/form.rs`（封装 gpui-component Form）
  - `crates/engine/src/compiler/form/mod.rs`、`setters.rs`
- **验证**：新增 `form_case.rml.rs` 展示 vertical 布局 + 校验错误显示 + 提交事件（与现有 `validation_case.rml.rs` 联动）

---

### 阶段 3：布局与中等复杂度组件（3 个组件）

#### 3.1 Dialog 组件

> 注意：与现有 `<dialog>` 根节点标记（AlertDialog）共存。新 `<Dialog>` 标签用于业务侧主动调用 gpui-component Dialog。

- **标签**：`<Dialog>` / `<dialog>`（需评估与现有 `<dialog>` 根节点冲突）
- **决策**：现有 `<dialog>` 根节点保持为 AlertDialog 入口；新增 `<Dialog>` 标签为独立组件（PascalCase 强制区分）
- **ComponentKind**：`Stateless`（容器，承载 content/footer slot）
- **container**：`true`
- **属性**：
  - static: `title`、`width`、`closable`、`mask_closable`、`placement`（center/right）
  - bind: `open`、`title`
  - event: `on_close`、`on_open_change`
- **实现路径**：
  - `crates/ui/src/components/dialog.rs`（re-export gpui-component Dialog 及其子组件 DialogContent/DialogHeader/DialogFooter/DialogTitle/DialogDescription/DialogClose/DialogAction）
  - `crates/engine/src/compiler/dialog/mod.rs`、`setters.rs`
  - 参考 [alert_dialog.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/alert_dialog.rs) re-export 模式
- **验证**：新增 `dialog_case.rml.rs` 展示自定义内容 + footer slot + 受控 open 状态

#### 3.2 Resizable 组件

- **标签**：`<Resizable>` / `<resizable>`，子项 `<ResizablePanel>` / `<resizable-panel>`
- **ComponentKind**：`StatelessWithItems`（参考 Accordion 模式）
- **container**：`false`
- **属性**：
  - Resizable: `direction`（horizontal/vertical）、`sizes`、`min_size`、`max_size`
  - ResizablePanel: `default_size`、`min_size`、`max_size`、`collapsible`
- **实现路径**：
  - `crates/ui/src/components/resizable.rs`
  - `crates/engine/src/compiler/resizable/mod.rs`、`panel.rs`、`setters.rs`
  - 参考 [compiler/accordion/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/) StatelessWithItems 模式
- **验证**：新增 `resizable_case.rml.rs` 展示水平 + 垂直布局 + 折叠面板

#### 3.3 Scrollable 组件

- **标签**：`<Scrollable>` / `<scrollable>`
- **ComponentKind**：`Stateless`（容器）
- **container**：`true`
- **属性**：
  - static: `direction`（both/horizontal/vertical）、`show_scrollbar`、`scrollbar_style`（overlay/attached）
  - bind: `scroll_offset`、`scroll_to`
- **实现路径**：
  - `crates/ui/src/components/scrollable.rs`
  - `crates/engine/src/compiler/scrollable.rs`（单文件）
- **验证**：新增 `scrollable_case.rml.rs` 展示超大内容滚动 + 受控滚动位置

---

### 阶段 4：重型组件（4 个组件，置后阶段）

> 此阶段组件涉及复杂数据结构、虚拟化算法或大量属性映射，建议作为独立里程碑分批交付。每项在进入实施前需进一步细化设计文档。

#### 4.1 Sidebar 组件

- **标签**：`<Sidebar>` / `<sidebar>`，子项 `<SidebarSection>` / `<SidebarItem>`
- **ComponentKind**：`StatelessWithItems`
- **container**：`false`
- **关键属性**：sections、items、collapsed、on_select、theme（light/dark）
- **设计要点**：参考 VSCode 侧边栏结构，含 section group、icon、active state
- **实现路径**：
  - `crates/ui/src/components/sidebar/`（目录结构，参考 [activity_bar/](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar/) 拆分）
  - `crates/engine/src/compiler/sidebar/`（多文件模块）

#### 4.2 DataTable 组件（基于 gpui-component DataTable + WPF 风格声明式优化）

> 用户明确："RML 的 Table 实际对标的是 gpui-component 的 table，但为了更好的声明式编码体验，声明式语法偏向于 WPF 风格，未来 DataTable 实际上也是这个思路，为了声明式语法更简洁且完整覆盖原生控件所有能力，声明式语法做进一步优化"

- **标签**：`<DataTable>` / `<data-table>`
- **ComponentKind**：`StatelessWithItems`（与 Table 模式相同）
- **container**：`false`
- **与 Table 共存策略**：
  - `<Table>`：保留现有 WPF DataGrid 风格声明式（已实现）
  - `<DataTable>`：基于 gpui-component DataTable，提供更高性能的虚拟化渲染 + 简洁声明式 API
- **关键属性**：columns（绑定）、rows（绑定）、delegate（WPF 风格委托）、scroll_virtual、row_height、selection_mode（single/multiple/none）、on_row_click、on_select
- **设计要点**：
  - 声明式 `<Column>` 子项定义列（沿用 Table 的 Column 子标签模式）
  - 大数据量场景启用虚拟滚动
  - WPF 风格 cell template 通过 `<template slot="cell">` 注入
- **实现路径**：
  - `crates/ui/src/components/data_table/`（多文件结构）
  - `crates/engine/src/compiler/data_table/`（多文件，参考 [compiler/table/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/table/) 结构）
  - 需先完成 WPF 风格声明式 API 设计评审（不在本计划范围）
- **进入实施前需细化**：声明式 API 草案、与 Table 的属性差异表、虚拟化触发阈值

#### 4.3 VirtualList 组件

- **标签**：`<VirtualList>` / `<virtual-list>`
- **ComponentKind**：`Stateless`（容器，通过 each 指令渲染）
- **container**：`true`
- **关键属性**：item_count、item_height（固定高度，虚拟化基础）、overscan、render（item 渲染函数）
- **设计要点**：
  - 与 RML `each` 指令深度整合：`<div each={item in items} />` 内嵌 VirtualList 自动启用虚拟化
  - 或独立 `<VirtualList items={data} item_height={40}>{render}</VirtualList>` 模式
- **实现路径**：
  - `crates/ui/src/components/virtual_list.rs`
  - `crates/engine/src/compiler/virtual_list.rs`

#### 4.4 Chart 组件

- **标签**：`<Chart>` / `<chart>`
- **ComponentKind**：`StatelessNoId`
- **container**：`false`
- **关键属性**：data（数据集）、series（系列配置）、x_field、y_field、x_axis、y_axis、legend、tooltip、theme
- **设计要点**：
  - 子项 `<ChartSeries>` 定义系列（line/bar/pie/scatter）
  - 与 RML 数据绑定深度整合：`<Chart data={stats} x_field="date" y_field="value" type="line" />`
- **实现路径**：
  - `crates/ui/src/components/chart/`（多文件结构）
  - `crates/engine/src/compiler/chart/`（多文件）

---

## 4. Assumptions & Decisions（假设与决策）

### 4.1 设计决策（基于用户反馈）

| 决策项 | 选择 | 依据 |
|-------|------|------|
| 已有近似实现的处理 | 按组件差异化判断 | 用户答复 |
| 优先级排序原则 | 按官方文档分组顺序（基础→表单→布局/高级） | 用户答复 |
| 重型组件处理 | 纳入但置后 | 用户答复 + 用户补充："声明式语法做进一步优化" |

### 4.2 差异化判断结果

| 组件对 | 处理 | 原因 |
|-------|------|------|
| CodeEditor vs Editor | 互补共存 | CodeEditor 是 InputState 代码模式封装；Editor 是 gpui-component 独立组件 |
| Table vs DataTable | 共存，各自定位 | Table = gpui-component Table 的 WPF 风格封装（已实现）；DataTable = gpui-component DataTable 的 WPF 风格封装（待优化声明式） |
| AlertDialog vs Dialog | 共存 | 二者均为 gpui-component 独立组件；当前仅暴露 AlertDialog，需新增 Dialog 标签 |

### 4.3 命名约定（遵循 [CLAUDE.md](file:///d:/GitCode/RF/rust-gpui-rml/CLAUDE.md)）

- 标签名：PascalCase 主标签 + kebab-case 别名（如 `<Select>` / `<select>`）
- 属性名：snake_case 内部 + kebab-case 声明式（如 `on-change` → normalize → `on_change`）
- 编译器模块目录：`crates/engine/src/compiler/<component_snake_case>/`（如 `number_input/`）
- UI 封装文件：`crates/ui/src/components/<component_snake_case>.rs`（薄 re-export）或 `<component_snake_case>/` 目录（复杂封装）

### 4.4 关键假设

1. **gpui-component git 依赖包含所有目标组件**：Alert、Image、Select、NumberInput、DatePicker、OtpInput、ColorPicker、Editor、Form、Dialog、Resizable、Scrollable、Sidebar、Chart、DataTable、VirtualList 均在 git 仓库中可用（实施时需逐项验证 API）
2. **CodeEditor 模式可复用**：[compiler/code_editor/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/) 的 InputState 复用模式可作为 Editor 实现参考（但 Editor 使用独立 State）
3. **Stateful 事件订阅模式成熟**：[compiler/input/](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/input/) 已建立 `EventEmitter + cx.subscribe` block 表达式模式，所有 Stateful 表单组件（Select/NumberInput/DatePicker/OtpInput/ColorPicker/Editor）复用此模式
4. **`<img>` 标签冲突**：现有 [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 第 26 行 `BuiltinTag::Img` 降级为 `gpui::div()`，新增 `<Image>` 仅用 PascalCase 形式，避免破坏现有 `<img>` 用法
5. **重型组件阶段细化**：阶段 4 的 4 个组件仅给出框架性规划，进入实施时需先输出独立设计文档（声明式 API 草案 + gpui-component API 调研）

### 4.5 铁律遵循

- 一个 rs 文件 = 一个组件 / 一个职责（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-d-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）
- 所有 `mod.rs` 仅 re-export，不写业务代码
- 多个独立 `pub struct` 组件拆分独立文件（如 Sidebar 的 SidebarSection / SidebarItem 各自独立文件）
- 无 `rml_` 前缀（用户偏好）
- 优先扩展现有枚举的 variant，而非暴露新接口（用户偏好）

---

## 5. Verification（验证标准）

### 5.1 每个组件的验证清单

- [ ] `cargo build -p rust-rml-ui` 成功（UI 封装层编译通过）
- [ ] `cargo build -p rust-rml-engine` 成功（编译器模块编译通过）
- [ ] `cargo test -p rust-rml-engine --test props_registry_complete` 通过（属性注册一致性）
- [ ] `cargo build -p rust-rml-demo` 成功（demo 编译通过）
- [ ] 新增 `<component>_case.rml.rs` 可在 demo 应用中独立运行
- [ ] [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 的 `component_lookup` 单元测试覆盖新标签
- [ ] [props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `COMPONENT_PROPS` 包含所有新增组件的属性清单

### 5.2 阶段性验证

| 阶段 | 完成标志 |
|------|---------|
| 阶段 1 | Alert + Image 案例可运行，基础组件覆盖率 11/9（含 Tooltip 属性模式） |
| 阶段 2 | 7 个表单组件案例可运行，表单组件覆盖率 8/8 |
| 阶段 3 | Dialog + Resizable + Scrollable 案例可运行，布局组件覆盖率 5/8（剩 4 个重型） |
| 阶段 4 | 4 个重型组件案例可运行，整体覆盖率 22/22 |

### 5.3 集成验证

- `cargo test --workspace` 全部通过
- `cargo clippy --workspace -- -D warnings` 无警告
- demo 应用启动后所有 case 项可点击运行，无运行时 panic
- 新增组件在 `.rml` 文件中使用时，LSP/codegen 错误信息准确（[source_map](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/source_map.rs) 记录正确 span）

---

## 6. 实施顺序与依赖

```
阶段 1 (基础组件)
  ├─ Alert (独立，无依赖)
  └─ Image (独立，无依赖)
       ↓
阶段 2 (表单组件)
  ├─ Select (依赖 Input 事件订阅模式，已存在)
  ├─ NumberInput (依赖 Input 模式)
  ├─ DatePicker (依赖 Input 模式)
  ├─ OtpInput (独立)
  ├─ ColorPicker (依赖 Input 模式)
  ├─ Editor (参考 CodeEditor，独立)
  └─ Form (依赖其他表单组件存在性，建议最后实施)
       ↓
阶段 3 (布局/中等复杂度)
  ├─ Dialog (独立，与 AlertDialog 共存)
  ├─ Resizable (参考 Accordion StatelessWithItems 模式)
  └─ Scrollable (独立)
       ↓
阶段 4 (重型组件，置后)
  ├─ Sidebar (参考 ActivityBar 拆分模式)
  ├─ DataTable (需先输出声明式 API 设计文档)
  ├─ VirtualList (与 each 指令深度整合，需设计评审)
  └─ Chart (多系列复杂结构，需 gpui-component Chart API 调研)
```

---

## 7. 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| gpui-component git 依赖中部分组件 API 不稳定 | 阶段 4 组件可能需要适配 | 实施前先核对 gpui-component 实际 API；薄 re-export 优先于深度封装 |
| 表单组件 State 命名差异 | SelectState / NumberInputState 等命名可能与 gpui-component 不一致 | 在 `crates/ui/src/components/<component>.rs` 中 type alias 统一命名 |
| `<img>` 与 `<Image>` 标签冲突 | 用户混淆原生降级 div 与 gpui-component Image | 文档明确区分；`<img>` 维持现有降级为 `gpui::div()` 行为 |
| DataTable 声明式 API 设计复杂 | 阶段 4 实施延期 | 阶段 4 进入前必须先输出独立设计文档，不与阶段 1-3 并行 |
| Chart 依赖较重（可能引入图表库） | 编译时间增加、依赖膨胀 | 评估是否启用 feature gate；必要时通过 `ui-components` feature 控制 |

---

## 8. 附录：组件清单速查表

| # | 组件 | 阶段 | ComponentKind | container | 标签别名 |
|---|------|------|---------------|-----------|---------|
| 1 | Alert | 1 | StatelessWithItems | false | `<alert>` |
| 2 | Image | 1 | StatelessNoId | false | （仅 PascalCase） |
| 3 | Select | 2 | Stateful | false | `<select>` |
| 4 | NumberInput | 2 | Stateful | false | `<number-input>` |
| 5 | DatePicker | 2 | Stateful | false | `<date-picker>` |
| 6 | OtpInput | 2 | Stateful | false | `<otp-input>` |
| 7 | ColorPicker | 2 | Stateful | false | `<color-picker>` |
| 8 | Editor | 2 | Stateful | false | （仅 PascalCase） |
| 9 | Form | 2 | Stateless | true | `<form>` |
| 10 | Dialog | 3 | Stateless | true | （仅 PascalCase，避免与 `<dialog>` 根节点冲突） |
| 11 | Resizable | 3 | StatelessWithItems | false | `<resizable>` |
| 12 | Scrollable | 3 | Stateless | true | `<scrollable>` |
| 13 | Sidebar | 4 | StatelessWithItems | false | `<sidebar>` |
| 14 | DataTable | 4 | StatelessWithItems | false | `<data-table>` |
| 15 | VirtualList | 4 | Stateless | true | `<virtual-list>` |
| 16 | Chart | 4 | StatelessNoId | false | `<chart>` |
