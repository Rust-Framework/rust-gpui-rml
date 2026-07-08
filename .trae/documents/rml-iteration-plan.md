# RML 框架迭代计划

> 本文档记录 RML 框架当前不支持或存在 codegen bug 的场景，阻碍 demo 案例及应用界面完全遵循 MVVM 声明式 UI 约束。
> 每项限制包含：现象、根因、影响案例、当前临时方案、提议的框架层修复方向。

## 一、已修复

### 1.1 `each` 指令在 `<template slot>` 闭包内的 codegen bug

- **状态**：已修复（2026-07-08）
- **现象**：`each={item in field}` 指令在 `<template slot>` 闭包内生成 `self.field.iter()` 而非 `__rml_self_ref.field.iter()`，导致 E0521（lifetime may not live long enough）和 E0382（borrow of moved value: self）编译错误。
- **根因**：7 处 codegen 路径硬编码 `format!("self.{}", clause.iterable)`，未使用 `current_self_alias()` 机制（slot 闭包内 alias 为 `__rml_self_ref`，顶层 render 为 `self`）。
- **影响案例**：key_case、list_case、tab_preview_case（此前用命令式 `render_items`/`render_item_list` 绕过）
- **修复**：将 7 处 `format!("self.{}", ...)` 改为 `format!("{}.{}", expr::current_self_alias().unwrap_or("self"), ...)`：
  - `crates/engine/src/compiler/tabs/tab.rs:392`（`<Tab each={...}>`）
  - `crates/engine/src/compiler/codegen/node.rs:143`（`html={...} each={...}` 指令组合）
  - `crates/engine/src/compiler/menu/menu_bar.rs:89`、`:382`（菜单项 each 迭代）
  - `crates/engine/src/compiler/menu/item.rs:386`（菜单项 bind 表达式 fallback）
  - `crates/engine/src/compiler/codegen/shell.rs:106`、`:558`（shell bind 表达式 fallback）
- **回归验证**：key_case/list_case/tab_preview_case 已回退为声明式 `each`，915 engine 测试通过，demo 编译成功。

---

## 二、待迭代项

### 2.1 `once` 指令在 slot 闭包内生成 `&mut self` 代码

- **优先级**：高
- **现象**：`once` 指令在 `<template slot>` 闭包内生成 `self.__rml_state.once_get_or_init(...)` 调用，需要 `&mut self`，但 slot 闭包仅提供 `&self`（通过 `__rml_self_ref: &Self`），导致编译错误。
- **根因**：`once` 指令的 codegen 路径（`crates/engine/src/compiler/codegen/once.rs`）生成 `__rml_state.once_get_or_init(field, || { ... })` 调用，该方法签名要求 `&mut self`。slot 闭包的 `__rml_self_entity.update(_app, |this, cx| { let __rml_self_ref: &Self = this; ... })` 中 `this` 是 `&mut Self`，但 `__rml_self_ref` 被绑定为 `&Self`（不可变借用），无法调用 `&mut self` 方法。
- **影响案例**：`once_case` — 无法在 demo slot 内使用 `once` 指令演示快照行为，改用 `frozen_counter: Option<u32>` 字段 + `#[computed] once_counter()` 模拟。
- **当前临时方案**：不使用 `once` 指令，用 ViewModel 字段 + computed 方法模拟快照语义。
- **提议修复方向**：
  1. **方案 A**（推荐）：将 `once` 的快照存储改为 `RefCell` 或 `Mutex` 内部可变性，使 `once_get_or_init` 只需 `&self`。
  2. **方案 B**：在 slot 闭包内将 `__rml_self_ref` 升级为 `&mut Self`（需调整 slot 渲染闭包签名，影响面较大）。

### 2.2 RML 缺乏可复用的模板片段机制

- **优先级**：中
- **现象**：需要在模板内复用一段带参数的 UI 结构（如 "信息卡片" = Card + title + body 文本），但 RML 没有参数化模板/宏机制，只能通过 `<component content={method()} />` 调用 ViewModel 方法返回 `AnyElement`，迫使命令式 UI。
- **根因**：RML 的 `<template slot="name">` 仅用于父子组件插槽注入，不支持在同一组件内定义带参数的可复用模板片段。`<component content={expr} />` 是透明容器，expr 必须返回 `AnyElement`，而构建 `AnyElement` 需要命令式 GPUI API。
- **影响案例**：`template_slot_case` — `render_info_card(title, body)` 和 `render_stat_card(label, value)` 方法使用 `Card::new(...).child(div()...)` 命令式构造 UI。
- **当前临时方案**：保留命令式 `render_*` 方法，在 .rml.rs 注释中标注为 RML 限制。
- **提议修复方向**：
  1. **方案 A**（Vue 风格）：引入 `<template define="card_template(title, body)">` 定义 + `<template use="card_template" args="..." />` 引用语法。
  2. **方案 B**（XAML DataTemplate 风格）：支持在 .rml 中声明命名模板，通过绑定传参。
  3. **方案 C**（Rust 宏风格）：允许 .rml 中定义可在多处展开的命名片段，codegen 时内联展开。

### 2.3 RML 事件处理器无法传递循环变量作为命令参数

- **优先级**：中
- **现象**：在 `each` 循环内，每个列表项需要绑定点击事件并传递该项的标识（如 `item.id`）给命令方法，但 RML 事件处理器语法 `on-click={command_name}` 仅支持无参命令（签名 `fn(&mut self, &ClickEvent, &mut Context<Self>)`），不支持 `on-click={open_case(item.id)}` 形式。
- **根因**：RML 事件处理器 codegen 将 `on-click={name}` 编译为 `cx.listener(move |this, _ev, _window, cx| { this.name(&rml_ev, cx); })`，闭包不捕获循环变量。要支持 `on-click={method(arg)}` 需要解析方法调用表达式，在闭包内捕获 `arg` 并传给方法。
- **影响案例**：`welcome_case` — `render_group` 方法内为每个卡片创建 `Card::new().on_click(cx.listener(move |this, _, _, cx| { this.open_case(case_id, cx); }))`，命令式捕获 `case_id`。整组渲染逻辑（分组标题 + 卡片行）因此全部命令式。
- **当前临时方案**：`welcome_case` 保留 `render_group` 命令式方法，通过 `<component each={group in grouped_items} content={self.render_group(group, _window, cx)} />` 注入。
- **提议修复方向**：
  1. **方案 A**（推荐）：扩展事件处理器语法支持 `on-click={command(expr)}`，codegen 生成 `let __rml_arg = expr; cx.listener(move |this, _ev, _window, cx| { this.command(__rml_arg, cx); })`。
  2. **方案 B**：引入 `data-*` 属性 + `on-click` 事件中通过 `ev` 访问 `data-id`（类似 HTML data attributes），但 GPUI 事件对象不支持自定义数据。

> **详细 Bug 工单**：参见 [p1-event-handler-bugs.md](./p1-event-handler-bugs.md) — 将本节细化为 7 个可执行 Bug（BUG-P1-01 ~ BUG-P1-07），含代码位置、复现、修复方向与优先级排序。

### 2.4 `IVisual::render` 框架接口要求返回 AnyElement

- **优先级**：低
- **现象**：状态栏贡献点（`#[contribute(kind = "status")]`）通过 `IVisual` trait 的 `render` 方法渲染，该方法签名 `fn render(&self, window, cx) -> AnyElement` 本质上是命令式的，要求实现者用 GPUI 链式 API 构造元素。
- **根因**：`IVisual` 是 `rml_core` 中的框架接口，设计上要求返回 `AnyElement`。RML 模板编译为 `Render::render` 方法，但 `IVisual::render` 是独立的 trait 实现，不由 RML 模板驱动。
- **影响案例**：`status_bar_case` — `StatusReady` struct 实现 `IVisual::render`，使用 `gpui::div().text_xs().child(...).into_any_element()` 命令式构造。
- **当前临时方案**：保留 `StatusReady` 的命令式 `IVisual::render` 实现，在 .rml.rs 注释中标注为框架限制。
- **提议修复方向**：
  1. **方案 A**：为状态栏贡献点提供 RML 模板支持，允许 `#[contribute(kind = "status")]` 的组件关联一个 .rml 模板片段，框架自动将其编译为 `IVisual::render` 实现。
  2. **方案 B**：提供声明式 builder 宏（如 `rml_status_element! { div().text_xs().child(...) }`），减少命令式代码量（但仍是命令式）。

---

## 三、迭代优先级排序

| 优先级 | 项目 | 影响面 | 复杂度 |
|--------|------|--------|--------|
| P0 | 2.1 `once` 指令 slot 闭包 bug | 1 case + 框架核心能力 | 低（RefCell 内部可变性） |
| P1 | 2.3 事件处理器传递循环变量 | 1 case + 列表交互通用能力 | 中（codegen 扩展） |
| P2 | 2.2 可复用模板片段 | 1 case + 模板复用通用能力 | 高（新语法设计） |
| P3 | 2.4 IVisual::render 声明式 | 1 case + 状态栏贡献点 | 中（框架接口改造） |
