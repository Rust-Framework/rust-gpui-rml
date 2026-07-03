# TabBar 声明式支持完整规划（含模板定制）

> 参考文档：<https://longbridge.github.io/gpui-component/zh-CN/docs/components/tabs>
> 规划范围：独立 TabBar 组件 + tab\_window 外壳的标签模板定制

***

## 一、Summary（概要）

RML 框架对 gpui-component `TabBar` / `Tab` 的声明式支持已完成 **Phase A（独立组件）+ Phase B（tab\_window 模板定制）+ Phase C（demo）** 三阶段，共 38 个单元测试覆盖 codegen 路径，demo 可编译。本规划聚焦于：(1) 文档化已实现能力；(2) 识别并填补剩余小缺口；(3) 完成最终回归验证。

***

## 二、Current State Analysis（现状分析）

### 2.1 Phase A：独立 TabBar 组件 ✅ 已完成

| 维度              | 实现位置                                                                                                                              | 状态                             |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| 标签注册            | [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) `canonical_tag`/`component_lookup`/`is_item_builder_tag` | ✅ TabBar/tab 双向别名              |
| 属性注册            | [props\_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) `TabBar`(12)/`Tab`(12)     | ✅                              |
| 容器 codegen      | [tab\_bar/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/gen.rs)                                  | ✅ 15 测试                        |
| 属性映射            | [tab\_bar/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs)                          | ✅ 17 测试                        |
| Tab 子节点 codegen | [tab\_bar/tab.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/tab.rs)                                  | ✅ label/icon/disabled/children |
| UI 组件           | [components/tab/](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab) `Tab`(ParentElement) + `TabBar`               | ✅                              |
| lib.rs 导出       | [ui/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)                                                         | ✅ Tab + TabBar                 |

**已支持的 gpui-component API：**

TabBar：`selected_index` / `on_click(idx: usize)` / 5 种 variant（underline/pill/flat/outline/segmented）/ `menu` / `prefix` / `suffix` / `last_empty_space` / 4 种 size（xsmall/small/medium/large）

Tab：`label` / `icon` / `disabled` / `prefix` / `suffix` / element children（标题模板定制）/ `on_click`（ClickEvent）

**on\_click 签名差异**：TabBar 用 `Fn(&usize, &mut Window, &mut App)`（与原生一致），Tab 走通用 ClickEvent 路径。

### 2.2 Phase B：tab\_window 模板定制 ✅ 已完成

| 维度                | 实现位置                                                                                                                                                                   | 状态 |
| ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -- |
| TabWindowShell 字段 | [window/tab\_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) `tab_children: Vec<Tab>` 字段 + `tab_children()` setter                  | ✅  |
| render 分支         | 同上 `if !self.tab_children.is_empty() { drain } else { tabs.iter() }`                                                                                                   | ✅  |
| Slot 拆分           | [codegen/shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) `partition_slot_children` 8-tuple 含 `slot_tabs: Vec<Node>`         | ✅  |
| 互斥校验              | 同上 `gen_tab_window_wrapper`：`tabs={...}` bind + `<template slot="tabs">` 并存 → CodegenError                                                                             | ✅  |
| 代码生成              | [codegen/render.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs) 对 slot\_tabs 每个 `<Tab>` 调 `tab_bar::tab::gen_tab_child`          | ✅  |
| Slot 白名单          | [validator.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs) `tab_window` 允许 `["menu","title","footer","left","right","bottom","tabs"]` | ✅  |
| 集成测试              | shell.rs 6 个测试（partition/wrapper/互斥/空 slot）                                                                                                                            | ✅  |

**模板定制语法（高级能力）：**

```xml
<tab_window>
    <template slot="tabs">
        <Tab>
            <span>Account</span>
            <Badge label="3" />
        </Tab>
        <Tab>
            <span>Profile</span>
        </Tab>
    </template>
    <!-- 主内容 -->
</tab_window>
```

与简单模式 `<tab_window tabs={tab_items}>` 互斥（编译期报错）。

### 2.3 Phase C：Demo ✅ 已完成（含小缺口）

[demo/src/cases/tab\_bar\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tab_bar_case.rml) 包含 7 个演示 section：basic / variants / sizes / with\_icon / disabled / menu / template。

**i18n 键与 RML section 对齐缺口：**

| i18n 键                       | RML section                                      | 状态    |
| ---------------------------- | ------------------------------------------------ | ----- |
| `case.tab_bar.prefix_suffix` | 无                                                | ❌ 缺演示 |
| `case.tab_bar.status`        | 无（仅顶部 `<p>{status_text}</p>` 显示状态文本，无 `<h3>` 标题） | ❌ 缺标题 |

### 2.4 预存阻断项（与 TabBar 无关）

* `crates/ui/src/components/table/` 为 untracked，含 5 个 borrow checker 错误 + 2 个 E0599，阻塞 `cargo build -p rust-rml-ui`。本次规划 **不修复**（超出 TabBar 范围，遵循 CLAUDE.md "Surgical Changes" 原则）。

* `crates/engine/src/compiler/code_editor/` 已通过在 [compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) 添加 `pub mod code_editor;` 解锁。

***

## 三、Proposed Changes（建议变更）

### 3.1 填补 Demo 缺口（必做，\~15 行 RML）

**文件**：[demo/src/cases/tab\_bar\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tab_bar_case.rml)

**变更 1**：在 `with_icon` section 后插入 `prefix_suffix` section，演示 `prefix`/`suffix` 绑定按钮：

```xml
<div class="demo-section">
    <h3>{t("case.tab_bar.prefix_suffix")}</h3>
    <TabBar prefix={nav_prefix} suffix={nav_suffix} selected_index={0}>
        <Tab label="Account" />
        <Tab label="Profile" />
        <Tab label="Settings" />
    </TabBar>
</div>
```

**变更 2**：在 demo 顶部 `status_text` 显示处加 `<h3>{t("case.tab_bar.status")}</h3>` 标题：

```xml
<div class="demo-section">
    <h3>{t("case.tab_bar.status")}</h3>
    <p>{status_text}</p>
</div>
```

**文件**：[demo/src/cases/tab\_bar\_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tab_bar_case.rml.rs)

**变更 3**：为 `TabBarCase` 添加 `nav_prefix` / `nav_suffix` computed 方法（返回 `gpui::AnyElement` 或预构建按钮组合），供 prefix\_suffix section 绑定。具体实现参考现有 `status_text`/`code_sample` 模式。

**验证**：`cargo build -p rust-rml-demo` 成功；运行 demo 后 8 个 section 标题与 i18n 键一一对应。

### 3.2 显式化 Tab.selected 映射（可选，\~10 行）

**背景**：当前 `Tab selected="true"` 通过 [component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 的 `Selectable` trait 公共 setter 回退路径工作，行为正确但显式性弱。

**文件**：[tab\_bar/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/setters.rs)

**变更**：在 `static_setter` 中为 Tab 增加 `selected` 分支：

```rust
"selected" if tag == "Tab" => {
    let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") { "true" } else { "false" };
    Some(format!(".selected({})", bool_val))
}
```

**验证**：新增 1 个测试 `static_setter_tab_selected`，断言 `<Tab selected="true">` → `.selected(true)`；demo 中 `<Tab label="Selected" selected="true" />` 行为不变。

> 注：此变更非必要，因 Selectable trait 已正确处理。若遵循"minimum code"原则可跳过。建议执行以提升 codegen 显式性。

### 3.3 不实施的项目（明确排除）

| 项目                                   | 原因                                                                      |
| ------------------------------------ | ----------------------------------------------------------------------- |
| `TabBar.track_scroll(handle)`        | 需传 `&ScrollHandle` 引用，RML 声明式无法表达；若未来需要可考虑 `scrollable={true}` 包装器，本次不做 |
| `Tab.empty()` / `Tab.id()`           | `<Tab />` 空 Tab 已可用；RML 用 element id 机制而非 Tab.id()                      |
| `TabBar.with_variant(variant)` 字符串参数 | 5 个 variant 快捷属性已覆盖，传 variant 字符串无额外价值                                  |
| 修复 table 模块错误                        | 超出 TabBar 范围，预存 untracked 代码                                            |

***

## 四、Assumptions & Decisions（假设与决策）

1. **覆盖范围**：独立 TabBar + tab\_window 外壳双向支持（已在上一会话确认）。
2. **on\_click 签名**：TabBar 用 `fn(index: usize, cx: &mut Context<Self>)`（与原生 `Fn(&usize, &mut Window, &mut App)` 对齐，解引用后传入）。
3. **模板定制路径**：Tab 实现 `ParentElement`，element 子节点通过 `.child()` 注入；与 `label` 属性互斥（属性优先）。
4. **互斥双保险**：编译期 codegen 报错 + 运行期 `tab_children` 非空优先，双重防护 `tabs={...}` + `<template slot="tabs">` 并存。
5. **不修复 table 模块**：遵循 CLAUDE.md "Touch only what you must"，table 错误为预存 untracked 代码，与 TabBar 无关。
6. **D4 手动 demo 运行**：本环境无法执行 GUI 交互，仅验证编译通过；运行时行为靠 codegen 单测 + 类型系统保证。

***

## 五、Verification Steps（验证步骤）

| 步骤 | 命令 / 操作                                                                                                                 | 期望结果                                                                      |
| -- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| V1 | `cargo test -p rust-rml-engine`                                                                                         | 全部 456+ 测试通过（含新增 Tab.selected 测试）；doc-test 因 nightly rustdoc 缺失报错属环境问题，忽略 |
| V2 | `cargo build -p rust-rml-engine`                                                                                        | 编译通过（无 E0433/E0432）                                                       |
| V3 | `cargo build -p rust-rml-demo`                                                                                          | demo 编译通过，新增 prefix\_suffix/status section 不破坏构建                          |
| V4 | `cargo build -p rust-rml-ui`                                                                                            | **预期失败**：被预存 table 模块错误阻断，与 TabBar 无关；如需全量 build，单独修 table 模块（不在本规划范围）    |
| V5 | 回归验证：[main\_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml) 仍用 `tabs={tab_bar_items}` bind 模式 | 互斥校验仅在 `tabs` bind + `<template slot="tabs">` 并存时报错；单独使用 `tabs` bind 路径不变 |
| V6 | i18n 键与 RML section 一一对应检查                                                                                              | 10 个 `case.tab_bar.*` 键全部被 RML 引用（修复后）                                    |

***

## 六、Implementation Order（实施顺序）

1. **3.1 Demo 缺口填补** → V3 通过
2. **3.2 Tab.selected 显式映射**（可选）→ V1 + V2 通过
3. **V5/V6 回归验证** → 确认无副作用
4. 总结输出最终状态

***

## 七、成功标准

* 38 个原有 codegen 测试 + 1 个新增 Tab.selected 测试全部通过

* demo 编译通过且 10 个 i18n 键全部被 RML 引用

* `main_window.rml` 现有 `tabs={...}` 路径行为不变

* TabBar 声明式支持覆盖 gpui-component 文档列出的核心 API（除 `track_scroll` 外）

