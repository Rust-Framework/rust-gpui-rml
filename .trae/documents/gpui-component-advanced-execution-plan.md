# gpui-component 高级组件支持 — 执行计划

> 设计文档：[gpui-component-advanced-full-coverage-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/gpui-component-advanced-full-coverage-plan.md)
> 计划制定日期：2026-07-10
> 状态：**未开始实施**（tags.rs / props_registry.rs / compiler/components/ 均未注册任何新组件）

## Summary

本计划将已批准的 17 组件设计文档转化为可执行的实施步骤。设计决策、API 调研、RML 语法设计均已在设计文档中完成，本计划聚焦于**具体文件改动、代码模式、验证步骤**。

**当前状态确认**：
- [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `component_lookup`：无 VirtualList / Rating / OtpInput / Resizable / Settings / Select / ComboBox / ColorPicker / DatePicker / Notification / Sheet / FocusTrap / HoverCard / Sidebar / Dock / SearchableList / Chart / Plot
- [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `COMPONENT_PROPS`：同上，无任何新组件属性登记
- [compiler/components/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/) 目录：无 rating / otp_input / virtual_list / resizable / settings 等子目录
- [translator/component/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/) 目录：无新组件 translator
- [crates/ui/src/components/](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/) 目录：仅 `rating.rs` 已存在（薄 re-export），其余 16 个均无

**实施顺序**（遵循设计文档第 6 节）：
1. Phase A.2 Rating → 验证属性全量映射流程（Stateless）
2. Phase A.3 OtpInput → 验证 state_ctor 注入（Stateful）
3. Phase A.1 VirtualList → 验证 slot 闭包生成（专属 translator）
4. Phase A.4 Resizable → 验证 StatelessWithItems + 子项
5. Phase A.5 Settings → 综合验证（多层嵌套 + slot=field）
6. Phase B/C/D/E → 设计文档已有框架，Phase A 完成后按序推进

---

## Phase 1：Rating（Stateless，最简单，验证属性映射流程）

### 1.1 UI 封装层
**文件**：`crates/ui/src/components/rating.rs`（已存在）
**状态**：已有 `pub use gpui_component::rating::Rating;`，**无需改动**
**验证**：`cargo build -p rust-rml-ui` 成功

### 1.2 编译器模块
**新建目录**：`crates/engine/src/compiler/components/rating/`

**文件 1**：`crates/engine/src/compiler/components/rating/mod.rs`
```rust
//! Rating codegen 模块入口（星级评分）。
//!
//! - `gen.rs`：Rating 构造 + 属性映射
//! - `setters.rs`：Rating 专用属性 → builder 方法映射
//!
//! on_click 闭包参数为 `&usize`（评分值），非 ClickEvent，需专属 codegen。

pub mod gen;
pub mod setters;

pub use gen::gen_rating;
```

**文件 2**：`crates/engine/src/compiler/components/rating/gen.rs`
- 参考 [stepper/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/gen.rs) 模式
- 构造器：`rml_ui::Rating::new(("rml_el", N))` 或 `rml_ui::Rating::new("rml_ref:name")`
- 属性遍历 → 调用 `setters::static_setter` / `setters::bind_setter` / `setters::event_setter`，fallback 到通用 `component_static_setter` / `component_bind_setter` / `component_event_setter`
- CSS class 样式：`append_css_class_styles(&mut code, elem, "Rating", ...)`
- 无子节点处理（Rating 非 ParentElement）

**文件 3**：`crates/engine/src/compiler/components/rating/setters.rs`
- 参考 [stepper/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/setters.rs) 模式
- `value="3"` → `.value(3usize)`（static）/ `.value(self.field)`（bind）
- `max="5"` → `.max(5usize)`（static）
- `color="#ffcc00"` → 需 Hsla 解析，复用 CSS color 解析（参考 css/mapper.rs），暂可标记为 TODO 或先支持基础颜色名
- `on_click` → `.on_click(cx.listener(move |this, value: &usize, _window, cx| { this.{method}(value, cx); }))`（闭包参数 `&usize`，参考 Stepper 的 `idx: &usize` 模式）
- `disabled` / `size` 走通用 setter（COMMON_STATIC_PROPS / COMMON_BIND_PROPS）

### 1.3 标签路由注册
**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
**位置**：`component_lookup` 函数，在 `"Stepper"` 分支后添加：
```rust
// Rating：Stateless 星级评分，构造器 Rating::new(id)
// on_click 闭包参数为 &usize（评分值），非 ClickEvent，由 compiler/rating 专属处理
"Rating" => Some(ComponentTag {
    ctor_path: "rml_ui::Rating",
    kind: ComponentKind::Stateless,
    container: false,
}),
```

### 1.4 属性注册登记
**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
**位置**：`COMPONENT_PROPS` 数组末尾添加：
```rust
// Rating：星级评分，value/max 为数值，on_click 签名为 Fn(&usize, ...)
("Rating", &["value", "max", "color", "on_click"]),
```

### 1.5 codegen 路由
**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)
- 添加 `pub mod rating;`

**文件 2**：[compiler/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 或 `gen_component` 函数
- 添加 Rating 分支：`"Rating" => rating::gen_rating(elem, ...)`（或走通用 Stateless 分支 + setter 分发）
- **注意**：如果 Stateless 通用 translator 已能处理 Rating（通过 `component_static_setter` / `component_bind_setter` / `component_event_setter` fallback），则只需在 setters.rs 注册 Rating 专属 setter，无需独立 gen 分支

**文件 3**：[translator/component/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/mod.rs)
- 如果 Rating 走通用 Stateless translator，则**无需新增 translator**（StatelessComponentTranslator 自动处理）
- 如果需要专属 translator（因 on_click 非标准 ClickEvent），则新建 `translator/component/rating.rs` 并在 `register_all` 注册

### 1.6 演示案例
**文件 1**：`demo/src/cases/rating_case.rml.rs`
- 展示：受控评分（value 绑定）、10 星上限（max=10）、disabled、on_click 回调、自定义颜色
- ViewModel：`rating_value: usize`，`on_rating_change(&mut self, value: &usize, cx)`

**文件 2**：`demo/src/cases/rating_case.rml`（如有 .rml 文件）
**文件 3**：[demo/src/cases/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 注册新 case

### 1.7 验证清单
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine --test props_registry_complete` 通过
- [ ] `cargo test -p rust-rml-engine rating` 通过（codegen 单元测试）
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] `rating_case.rml.rs` 可在 demo 应用中独立运行
- [ ] `is_prop_registered("Rating", "value")` / `is_prop_registered("Rating", "on_click")` 返回 true

---

## Phase 2：OtpInput（Stateful，验证 state_ctor 注入）

### 2.1 UI 封装层
**文件**：`crates/ui/src/components/otp_input.rs`（新建）
```rust
//! OtpInput 组件封装 —— 基于 gpui-component 的 OtpInput
//!
//! 一次性密码输入框，Stateful 构造器接受 &Entity<OtpState>。
//! OtpState 构造需 length 参数，通过 state_ctor 占位符注入。

pub use gpui_component::input::{OtpInput, OtpState};
```

**文件**：[crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)
- 在 input re-export 处补充 OtpInput / OtpState（如未已导出）

**文件**：[crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs)
- 添加 `pub mod otp_input;` 或 `pub use otp_input::*;`

### 2.2 编译器模块
**新建目录**：`crates/engine/src/compiler/components/otp_input/`

**文件 1**：`crates/engine/src/compiler/components/otp_input/mod.rs`
```rust
//! OtpInput codegen 模块入口（OTP 输入）。
//!
//! - `gen.rs`：OtpInput 构造 + state_ctor 注入 + 事件订阅
//! - `setters.rs`：OtpInput 专用属性映射
//! - `event.rs`：InputEvent 订阅（Change/Focus/Blur）

pub mod gen;
pub mod setters;
pub mod event;

pub use gen::gen_otp_input;
```

**文件 2**：`crates/engine/src/compiler/components/otp_input/gen.rs`
- 参考 [input/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/gen.rs) + [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs) 模式
- **state_ctor 注入**：从 `length` 属性提取数值，替换 state_ctor 闭包中的占位符
  - state_ctor: `|w, c| rml_ui::OtpState::new(__RML_OTP_LENGTH__, w, c)`
  - codegen: 根据 `length="6"` 替换为 `|w, c| rml_ui::OtpState::new(6usize, w, c)`
- **masked 属性注入**：`masked` → `.masked(true)` 追加到 state_ctor 闭包
- **default_value 属性注入**：`default_value="123456"` → `.default_value("123456")` 追加到 state_ctor
- **事件订阅 block 表达式**：`on_change` / `on_focus` / `on_blur` → `cx.subscribe(&otp_state, move |this, state, window, cx| { ... })`
- **groups 属性**：`.groups(2usize)` 直接 builder 链

**文件 3**：`crates/engine/src/compiler/components/otp_input/setters.rs`
- `groups="2"` → `.groups(2usize)`（static）
- `length` / `masked` / `default_value` → 不生成 setter（已注入 state_ctor）

**文件 4**：`crates/engine/src/compiler/components/otp_input/event.rs`
- 参考 [input/event.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/input/event.rs)
- `is_otp_input_event(name) -> bool`：判断是否为 OtpInput 事件
- `gen_otp_input_event_subscribe(events, ref_name) -> String`：生成 `cx.subscribe` block 表达式

### 2.3 标签路由注册
**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
// OtpInput：Stateful OTP 输入，构造器 OtpInput::new(&Entity<OtpState>)
// OtpState 构造需 length 参数，通过 state_ctor 占位符注入
"OtpInput" | "otp-input" => Some(ComponentTag {
    ctor_path: "rml_ui::OtpInput",
    kind: ComponentKind::Stateful {
        state_field: "otp_state",
        state_ctor: "|w, c| rml_ui::OtpState::new(__RML_OTP_LENGTH__, w, c)",
    },
    container: false,
}),
```

### 2.4 属性注册登记
**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
// OtpInput：OTP 输入，length/masked/default_value 注入 state_ctor，on_change/on_focus/on_blur 走事件订阅
("OtpInput", &["length", "groups", "masked", "default_value", "on_change", "on_focus", "on_blur"]),
```

### 2.5 codegen 路由
**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)
- 添加 `pub mod otp_input;`

**文件 2**：`crates/engine/src/compiler/translator/component/otp_input.rs`（新建）
- 继承 StatefulComponentTranslator 模式，特化处理 length/masked/default_value 注入 state_ctor
- 在 `register_all` 注册

### 2.6 演示案例
**文件**：`demo/src/cases/otp_input_case.rml.rs`
- 展示：6 位 SMS 验证码、4 位 PIN（masked）、groups=1、on_change 验证、disabled 锁定

### 2.7 验证清单
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine otp_input` 通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] state_ctor 占位符替换正确（`__RML_OTP_LENGTH__` → 实际数值）
- [ ] `cx.subscribe` block 表达式生成正确

---

## Phase 3：VirtualList（专属 translator，验证 slot 闭包生成）

### 3.1 UI 封装层
**文件**：`crates/ui/src/components/virtual_list.rs`（新建）
```rust
//! VirtualList 组件封装 —— 基于 gpui-component 的 VirtualList
//!
//! 虚拟列表，函数式构造 v_virtual_list / h_virtual_list。
//! 通过 <template slot="render" let="range"> 注入渲染闭包。

pub use gpui_component::virtual_list::{
    v_virtual_list, h_virtual_list, virtual_list,
    VirtualList, VirtualListScrollHandle,
};
```

**文件**：[crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs)
- 添加 `pub mod virtual_list;`

### 3.2 编译器模块
**新建目录**：`crates/engine/src/compiler/components/virtual_list/`

**文件 1**：`crates/engine/src/compiler/components/virtual_list/mod.rs`
```rust
//! VirtualList codegen 模块入口（虚拟列表）。
//!
//! - `gen.rs`：函数式构造 + slot=render 闭包生成
//! - `setters.rs`：VirtualList 专用属性映射

pub mod gen;
pub mod setters;

pub use gen::gen_virtual_list;
```

**文件 2**：`crates/engine/src/compiler/components/virtual_list/gen.rs`
- **函数式构造**：根据 `vertical` 属性选择 `v_virtual_list` / `h_virtual_list`
  - 默认 horizontal（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）
  - `vertical` / `vertical={true}` → `v_virtual_list`
- **构造签名**：`rml_ui::v_virtual_list(cx.entity().clone(), ("rml_el", N), self.item_sizes.clone(), move |this, range, _window, cx| { ... })`
- **slot=render 处理**：参考 [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 slot 路由模式
  - `<template slot="render" let="range">` 子节点生成闭包体
  - `let="range"` 注入 `range: Range<usize>` 参数
  - 闭包内 `each={ix in range.start..range.end}` 迭代
- **item_sizes 绑定**：`item-sizes={self.item_sizes}` → `self.item_sizes.clone()`
- **scroll_handle 绑定**：`scroll-handle={self.scroll_handle}` → `.track_scroll(&self.scroll_handle)`

**文件 3**：`crates/engine/src/compiler/components/virtual_list/setters.rs`
- `vertical` → 选择构造器（不生成 setter，在 gen.rs 处理）
- `item_sizes` → bind setter：`self.item_sizes.clone()`
- `scroll_handle` → bind setter：`.track_scroll(&self.scroll_handle)`
- `sizing` → static setter：`"auto"` → `.with_sizing_behavior(ListSizingBehavior::Auto)`

### 3.3 标签路由注册
**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- VirtualList 使用**专属 translator**，不在 ComponentKind 枚举内
- 在 `component_lookup` 添加路由（标记为专属处理）：
```rust
// VirtualList：函数式构造 v_virtual_list / h_virtual_list，由专属 translator 处理
"VirtualList" | "virtual-list" => Some(ComponentTag {
    ctor_path: "rml_ui::v_virtual_list",  // 实际由 translator 选择 v/h
    kind: ComponentKind::Stateless,  // 占位，实际由专属 translator 处理
    container: false,
}),
```

### 3.4 属性注册登记
**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
// VirtualList：虚拟列表，vertical 选择构造器，item_sizes/scroll_handle 为绑定
("VirtualList", &["vertical", "item_sizes", "scroll_handle", "sizing"]),
```

### 3.5 codegen 路由
**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)
- 添加 `pub mod virtual_list;`

**文件 2**：`crates/engine/src/compiler/translator/component/virtual_list.rs`（新建）
- 专属 translator，matches 优先于 StatelessComponentTranslator
- 处理函数式构造 + slot=render 闭包
- 在 `register_all` 注册

### 3.6 演示案例
**文件**：`demo/src/cases/virtual_list_case.rml.rs`
- 展示：1000 项垂直列表、编程式滚动到第 100 项、horizontal 卡片列表

### 3.7 验证清单
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine virtual_list` 通过
- [ ] slot=render 闭包生成正确（`range: Range<usize>` 参数注入）
- [ ] `vertical` 属性正确选择 `v_virtual_list` / `h_virtual_list`

---

## Phase 4：Resizable（StatelessWithItems + Panel 子项）

### 4.1 UI 封装层
**文件**：`crates/ui/src/components/resizable.rs`（新建）
```rust
//! Resizable 组件封装 —— 基于 gpui-component 的 Resizable
//!
//! 可调整面板布局，StatelessWithItems 构造。
//! 子节点为 <resizable-panel>，通过 .child(ResizablePanel::new()...) 注入。

pub use gpui_component::resizable::{
    h_resizable, v_resizable, resizable_panel,
    ResizablePanelGroup, ResizablePanel, ResizableState, ResizablePanelEvent,
};
```

**文件**：[crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs)
- 添加 `pub mod resizable;`

### 4.2 编译器模块
**新建目录**：`crates/engine/src/compiler/components/resizable/`

**文件 1**：`crates/engine/src/compiler/components/resizable/mod.rs`
```rust
//! Resizable codegen 模块入口（可调整面板）。
//!
//! - `gen.rs`：ResizablePanelGroup 构造 + Panel 子项注入
//! - `setters.rs`：Resizable / ResizablePanel 专用属性映射

pub mod gen;
pub mod setters;

pub use gen::gen_resizable;
```

**文件 2**：`crates/engine/src/compiler/components/resizable/gen.rs`
- 参考 [accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/gen.rs) + [stepper/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/stepper/gen.rs) 模式
- **构造器**：根据 `vertical` 属性选择 `h_resizable(id)` / `v_resizable(id)`（默认 horizontal）
- **子项处理**：`<resizable-panel>` 子节点 → `.child(resizable_panel().size(...).child(...))`
- **on_resize 事件**：`Rc<dyn Fn>` 直接构造（因闭包需访问 window，不能用 `cx.listener`）

**文件 3**：`crates/engine/src/compiler/components/resizable/setters.rs`
- Resizable:
  - `vertical` → 选择构造器（在 gen.rs 处理）
  - `size="220"` → `.size(px(220.))`（static）
  - `on_resize` → event setter（`Rc<dyn Fn(&Entity<ResizableState>, ...)>`）
- ResizablePanel:
  - `size="220"` → `.size(px(220.))`（static）
  - `min-size="100"` + `max-size="400"` → `.size_range(px(100.)..px(400.))`
  - `visible="false"` → `.visible(false)`（static）

### 4.3 标签路由注册
**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
// Resizable：StatelessWithItems，构造器 h_resizable / v_resizable
"Resizable" | "resizable" => Some(ComponentTag {
    ctor_path: "rml_ui::h_resizable",  // 实际由 gen.rs 选择 v/h
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
// ResizablePanel：item builder 子标签
"ResizablePanel" | "resizable-panel" => Some(ComponentTag {
    ctor_path: "rml_ui::resizable_panel",
    kind: ComponentKind::StatelessNoId,
    container: true,
}),
```

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `is_item_builder_tag` 函数：
- 添加 `"ResizablePanel" | "resizable-panel"` 识别

### 4.4 属性注册登记
**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
// Resizable：可调整面板，vertical 选择构造器，on_resize 回调
("Resizable", &["vertical", "size", "on_resize"]),
// ResizablePanel：面板子项，size/min_size/max_size/visible
("ResizablePanel", &["size", "min_size", "max_size", "visible"]),
```

### 4.5 codegen 路由
**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)
- 添加 `pub mod resizable;`

**文件 2**：`crates/engine/src/compiler/translator/component/resizable.rs`（新建）
- StatelessWithItems translator，参考 [accordion.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/accordion.rs) 模式
- 在 `register_all` 注册

### 4.6 演示案例
**文件**：`demo/src/cases/resizable_case.rml.rs`
- 展示：三栏布局（220/自适应/280）+ min/max 限制 + vertical 布局 + on_resize 持久化

### 4.7 验证清单
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine resizable` 通过
- [ ] `is_item_builder_tag("resizable-panel")` 返回 true
- [ ] size_range 正确生成（`px(100.)..px(400.)`）

---

## Phase 5：Settings（极复杂，多层嵌套 + slot=field）

### 5.1 UI 封装层
**文件**：`crates/ui/src/components/settings.rs`（新建）
```rust
//! Settings 组件封装 —— 基于 gpui-component 的 Settings
//!
//! 设置面板，StatelessWithItems 多层嵌套构造。
//! 层级：Settings → SettingPage → SettingGroup → SettingItem
//! SettingItem 支持 field 绑定（内置 BoolField/StringField 等）或 slot=field 自定义渲染。

pub use gpui_component::setting::{
    Settings, SettingPage, SettingGroup, SettingItem,
    SelectIndex,
};
// 内置 field 类型（需确认导出路径）
// pub use gpui_component::setting::{BoolField, StringField, NumberField, DropdownField};
```

**文件**：[crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs)
- 添加 `pub mod settings;`

### 5.2 编译器模块
**新建目录**：`crates/engine/src/compiler/components/settings/`

**文件 1**：`crates/engine/src/compiler/components/settings/mod.rs`
```rust
//! Settings codegen 模块入口（设置面板）。
//!
//! - `gen.rs`：Settings 容器构造 + page/group/item 多层注入
//! - `setters.rs`：Settings / SettingPage / SettingGroup / SettingItem 专用属性映射
//! - `item.rs`：SettingItem field 绑定 + slot=field 自定义渲染

pub mod gen;
pub mod setters;
pub mod item;

pub use gen::gen_settings;
```

**文件 2**：`crates/engine/src/compiler/components/settings/gen.rs`
- 参考 [accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/gen.rs) 多层 item 模式
- **Settings 容器**：`Settings::new(id).sidebar_width(...).page(SettingPage::new(...).group(...))`
- **SettingPage**：`SettingPage::new(title).group(SettingGroup::new().item(SettingItem::new(title, field)))`
- **SettingGroup**：`SettingGroup::new().item(SettingItem::new(...))`
- **SettingItem**：`SettingItem::new(title, field)` 或 `SettingItem::render(closure)`（slot=field）

**文件 3**：`crates/engine/src/compiler/components/settings/setters.rs`
- Settings: `sidebar_width="280"` → `.sidebar_width(px(280.))`，`default_selected_index="0"` → `.default_selected_index(SelectIndex::new(0))`
- SettingPage: `title` → 构造器参数，`icon="Settings"` → `.icon(Icon::new(IconName::Settings))`，`default_open` → `.default_open(true)`
- SettingGroup: `title` → `.title(...)`，`description` → `.description(...)`
- SettingItem: `title` → 构造器参数，`field` → bind（`self.theme_field.clone()`），`description` → `.description(...)`，`disabled` → `.disabled(true)`

**文件 4**：`crates/engine/src/compiler/components/settings/item.rs`
- **field 绑定**：`field={theme_field}` → `SettingItem::new("主题", self.theme_field.clone())`
- **slot=field 自定义**：`<template slot="field">` → `SettingItem::render(move |options, window, cx| { <自定义元素> })`
  - 参考 [popover/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/popover/gen.rs) 的 slot 路由模式

### 5.3 标签路由注册
**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
```rust
// Settings：StatelessWithItems 多层嵌套
"Settings" | "settings" => Some(ComponentTag {
    ctor_path: "rml_ui::Settings",
    kind: ComponentKind::StatelessWithItems,
    container: false,
}),
// SettingPage / SettingGroup / SettingItem：item builder 子标签
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

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `is_item_builder_tag` 函数：
- 添加 `"SettingPage" | "setting-page" | "SettingGroup" | "setting-group" | "SettingItem" | "setting-item"` 识别

### 5.4 属性注册登记
**文件**：[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
```rust
// Settings：设置面板容器
("Settings", &["sidebar_width", "group_variant", "default_selected_index", "on_reset"]),
// SettingPage：设置页
("SettingPage", &["title", "icon", "description", "default_open", "resettable"]),
// SettingGroup：设置组
("SettingGroup", &["title", "description"]),
// SettingItem：设置项（field 为绑定，slot=field 为自定义渲染）
("SettingItem", &["title", "field", "description", "layout", "disabled", "keywords", "on_reset"]),
```

### 5.5 codegen 路由
**文件 1**：[compiler/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/mod.rs)
- 添加 `pub mod settings;`

**文件 2**：`crates/engine/src/compiler/translator/component/settings.rs`（新建）
- StatelessWithItems translator，处理多层嵌套
- 在 `register_all` 注册

### 5.6 演示案例
**文件**：`demo/src/cases/settings_case.rml.rs`
- 展示：多页 Settings + 内置 BoolField/StringField + slot=field 自定义按钮 + 搜索 + 重置

### 5.7 验证清单
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine settings` 通过
- [ ] `is_item_builder_tag("setting-page")` / `is_item_builder_tag("setting-group")` / `is_item_builder_tag("setting-item")` 返回 true
- [ ] 多层嵌套正确生成（Settings → SettingPage → SettingGroup → SettingItem）
- [ ] slot=field 自定义渲染闭包正确生成

---

## Phase B/C/D/E：后续阶段（Phase A 完成后推进）

设计文档已有框架性规划，Phase A 完成后按序推进：

- **Phase B**（4 个 Stateful 表单）：Select / ComboBox / ColorPicker / DatePicker
  - 复用 Phase 2（OtpInput）的 Stateful + 事件订阅模式
  - 每个组件：UI re-export + compiler 模块 + tags 路由 + props 登记 + translator + demo case

- **Phase C**（4 个弹层）：Notification / Sheet / FocusTrap / HoverCard
  - 复用 Phase 3（VirtualList）的 slot 模式 + Phase 1（Rating）的 Stateless 模式
  - Notification 命令式 API 需特殊处理

- **Phase D**（3 个布局/导航）：Sidebar / Dock / SearchableList
  - Sidebar 复用 Phase 4（Resizable）的 StatelessWithItems 模式
  - Dock 极复杂，需先输出独立设计文档

- **Phase E**（2 个数据可视化）：Chart / Plot
  - 极复杂，需先输出独立设计文档

---

## Assumptions & Decisions（假设与决策）

### 设计决策（源自设计文档，已用户确认）
| 决策项 | 选择 | 依据 |
|-------|------|------|
| 规划范围 | 全量高级组件（17 个） | 用户明确选择 |
| VirtualList 语法 | `<virtual-list>` + slot=render + let=range | 用户明确选择 |
| Settings 深度 | 结构化 + slot=field 自定义 | 用户明确选择 |
| vertical 属性统一 | 默认 horizontal，仅 `vertical=true` | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| size 属性统一 | `size=small` / `size={size_value}` | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| 属性命名 | kebab-case 声明式 + snake_case 内部 | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |
| Stateful 事件订阅 | 复用 Input 事件订阅模式 | 既有成熟模式 |
| 禁止兼容性设计 | 不保留旧 API | [project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律 |

### 实施假设
1. **gpui-component git 依赖**包含所有目标组件（v0.5.2, git rev 063e55bbc4fb13907a988111e3581595cbcaefde）
2. **Stateless 通用 translator** 可处理 Rating（通过 setter fallback），如 on_click 需专属处理则新建 translator
3. **Stateful 事件订阅模式**可复用于 OtpInput / Select / ComboBox / ColorPicker / DatePicker / SearchableList
4. **slot 路由模式**可复用于 VirtualList(slot=render) / Settings(slot=field) / HoverCard(slot=trigger) / Sheet(slot=content)
5. **StatelessWithItems 模式**可复用于 Resizable / Settings / Sidebar

### 铁律遵循
- **一个 rs 文件 = 一个组件 / 一个职责**（[project_memory](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 铁律）
- 所有 `mod.rs` 仅 re-export，不写业务代码
- 无 `rml_` 前缀
- 属性命名禁止下划线（声明式用 kebab-case）
- 禁止兼容性设计

---

## Verification（整体验证）

### 每个 Phase 完成后
- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine` 全量回归通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] 新增 demo case 可独立运行
- [ ] `props_registry_complete` 测试通过（注册表一致性）
- [ ] `component_props_tags_align_with_routing_table` 测试通过（tags ↔ props 一致性）

### 全部 Phase A 完成后
- [ ] 5 核心组件 demo case 完整覆盖属性 + 事件 + 绑定 + slot
- [ ] 所有属性在 `props_registry.rs` 中完整登记，无静默丢弃
- [ ] `tags.rs` `component_lookup` 覆盖所有新标签
- [ ] `is_item_builder_tag` 覆盖所有子项标签
