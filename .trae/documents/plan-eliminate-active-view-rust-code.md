# 消除 active_view() Rust UI 代码实施计划

## Context

用户要求 demo 中禁止出现 Rust 代码构建 UI，必须全部走 `.rml` + `.rml.rs` MVVM 标准方案。当前 `main_window.rml.rs` 中仅剩 `active_view()` 方法（L295-304）使用 Rust 构建 UI（`gpui::div().into_any_element()` + `visual.render(window, cx)`）。

**核心发现**：TabWindowShell 已有"简单绑定模式"（`tabs: Vec<Arc<dyn IValue>>`），内部自动构建带 body 闭包的 TabItem，逻辑与 `active_view()` 完全一致。切换到该模式后 `active_view()` 变为冗余。

## 当前进度

- [x] `tab_items()` computed 属性已添加（main_window.rml.rs L324-334）
- [ ] TabWindowShell 简单绑定模式补充 `.closable(true)`
- [ ] RML 模板切换到 `tabs={tab_items}` 并删除显式内容容器
- [ ] 删除 `active_view()` 方法
- [ ] 编译验证

## 实施步骤

### 步骤 1：补充简单绑定模式的 closable 支持

**文件**：`crates/ui/src/window/tab_window.rs` L758

当前简单绑定模式未设置 `.closable(true)`，而模板模式支持 closable。修改 L758：

```rust
// 修改前
let item = TabItem::new().title(title).body(move |window, cx| {

// 修改后
let item = TabItem::new().title(title).closable(true).body(move |window, cx| {
```

### 步骤 2：更新 RML 模板

**文件**：`demo/src/shell/main_window.rml`

1. 在 `<tab-window>` 属性中添加 `tabs={tab_items}`
2. 删除 `<template slot="tabs" each={w in workbenches}>`（L16-18）
3. 删除 `<div class="flex-1 overflow-y-auto" id="active-view-container">`（L64-66）

修改后模板结构：

```xml
<tab-window
    ...
    tabs={tab_items}
    selected-index={selected_tab}
    ...>

    <template slot="left">...</template>
    <template slot="menu">...</template>
    <template slot="bottom" scope={panel}>...</template>
    <template slot="footer">...</template>

</tab-window>
```

**原理**：`tabs={tab_items}` 触发简单绑定模式，TabWindowShell 内部为每个 `IValue` 构建带 body 闭包的 TabItem（`as_visual()?.render()`），并通过 `selected_index` 渲染激活 tab 的 body。无需外部 `active_view()` 方法。

### 步骤 3：删除 active_view() 方法

**文件**：`demo/src/shell/main_window.rml.rs` L292-304

删除 `active_view()` 方法及其文档注释。同时清理不再需要的 imports（`VisualAbilityExt`、`IContribution` 等——需检查其他方法是否仍使用）。

### 步骤 4：编译验证

```bash
cargo build -p rust-rml-demo
```

验证：
- 编译通过
- Tab 标签正确显示且可关闭
- 点击 tab 切换内容
- 菜单、状态栏、底部面板功能正常

## 涉及文件

| 文件 | 变更 |
|------|------|
| `crates/ui/src/window/tab_window.rs` L758 | 添加 `.closable(true)` |
| `demo/src/shell/main_window.rml` | 添加 `tabs={tab_items}`，删除 tabs 模板和内容容器 |
| `demo/src/shell/main_window.rml.rs` L292-304 | 删除 `active_view()` 方法 |
