# 窗口 Shell 插槽化 + ICommand + Demo 修复

## Context

用户提出 3 个相互关联的架构方向 + 5 个 demo 运行 bug：

**架构方向**
1. `tab_window` / `modern_window` 的 menu、title_ext、statusbar 改为**纯插槽扩展**，不再用数据结构（`Vec<MenuItem>` / `Vec<StatusBarItem>`）直接传入，清理相关数据结构定义
2. menu、statusbar 等控件要提供 **MVVM 数据绑定方案**，采用 WPF `ICommand` 命令绑定（用户选择，非字符串派发）
3. RML 框架对**复杂数据类型、层级结构数据**的绑定方案，优先强化 TreeView 组件（用户选择）

**Demo Bug**
1. `LoginWindow` 不模态，与 `MainWindow` 同时显示
2. `TabWindow` 的 TabBar 标签栏下方有一条边框线
3. demo 主窗口 `MainWindow` 没有菜单栏
4. 活动栏图标按钮无外边距，且切换面板功能不工作
5. 活动栏面板空白，无案例数据

## 诊断根因

| Bug | 根因 | 位置 |
|-----|------|------|
| 1 | 声明式入口 `RmlApplication::new().main_window::<W>().lifecycle::<L>().run()` 在 `L::on_launch` 后立即 `W::default().open(cx)`，GPUI 无原生 modal | [application.rs:67-77](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs#L67-L77) |
| 2 | gpui-component `TitleBar` 自带 `.border_b_1().border_color(cx.theme().title_bar_border)`，无法改源码 | tab_window.rs:273-280 |
| 3 | `MainWindow.show_chrome: bool` 经 `#[derive(Default)]` 默认 `false`，导致 menu_slot/title 不渲染 | main_window.rml.rs:16, tab_window.rs:120-128 |
| 4 | ActivityBar 按钮无外边距；demo 未绑定 `on_panel_change`；`activity_icons` 写死 `.active(true)` | activity_bar.rs:238-252, main_window.rml.rs:53-61 |
| 5 | `IActivityPanel::panel()` 用 `RefCell::take()` 一次性抽干；`active_panel` fallback 也用 `.take()` | activity_bar.rs:103-105, 220-235 |

## 阶段 0：Demo Bug 修复（不引入新架构，立即可跑）

**目标**：让 demo 端到端跑通，作为后续重构的基线。

### Bug 1 — LoginWindow 模态化
- `demo/src/app.rs`：改回命令式入口 `RmlApplication::new().run::<AppBootstrap>()`（[application.rs:38-50](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs#L38-L50)），`on_launch` 只打开 `LoginWindow`
- `demo/src/login/login_window.rml.rs::on_login`：校验通过后 `MainWindow::default().open(cx)` + `self.close(cx)`
- 放弃上轮"框架自动打开主窗口"的声明式入口（GPUI 无原生 modal，命令式是务实方案）

### Bug 2 — TabBar 下方边框线
- `crates/ui/src/window/tab_window.rs::TabWindowShell::render`：在 `TitleBar` 外层 `div` 加 `.border_b_0()` 不可行（父层 border 不能从子层抹除）
- 方案：在 `TitleBar` 之后追加一个 `gpui::div().h(px(0.)).bg(cx.theme().background)` 覆盖那条线；或把 `tab_bar` 直接用 `gpui::div().h(px(40.))` 自绘，绕开 `TitleBar`
- 优先选第一种（最小改动）

### Bug 3 — MainWindow 菜单栏
- `demo/src/shell/main_window.rml.rs`：`on_loaded` 中设 `self.show_chrome = true`（或字段默认改 `true`，但 `Default` derive 不支持自定义初值，故走 `on_loaded`）
- 保留现有 `menu={menu_items}` / `status_bar={status_items}` 数据绑定（阶段 1 再迁移到插槽）

### Bug 4 — ActivityBar 外边距 + 切换
- `crates/ui/src/components/activity_bar.rs:243-244`：按钮加 `.my(px(2.))` 或在 `v_flex` 容器加 `.gap(px(4.))`
- `demo/src/shell/main_window.rml.rs`：新增 `active_panel_id: String` 字段（默认 `"samples"`）
- `activity_icons` computed 改为按 `active_panel_id` 判定 `panel.active(active_id == "samples")`
- 新增 `#[command] on_panel_change(&mut self, id: &SharedString, cx)`：写 `active_panel_id` + `cx.notify()`
- `main_window.rml`：`<ActivityBar panels={activity_icons} on_panel_change="on_panel_change">`

### Bug 5 — 活动栏面板空白
- `crates/ui/src/components/activity_bar.rs:103-105`：`IActivityPanel::panel()` 的 `take()` 改为 `clone()`（要求 `AnyElement: Clone`，gpui 已实现）或返回 `&Option` 借用
- `activity_bar.rs:234`：`active_panel = panel.panel().or_else(|| panel_fallback.take())` 改为优先用 `panel_children`，且 `panel_fallback` 不 take
- 验证：`<Tree on_activate="on_case_activate" />` 在 `case_tree_state` 初始化后能正确渲染

### 验证
- `cargo build` + `cargo test`（265 测试不回归）
- `cargo run --package rust-rml-demo`：登录窗单独显示 → 登录后主窗口显示 → 菜单栏/活动栏/Welcome Tab/案例树均可见 → 点击活动栏按钮可切换

---

## 阶段 1：Step 1 插槽纯化

**目标**：删除数据驱动兼容层，统一走 RML 插槽语法。

### 删除清单
- `crates/ui/src/window/types.rs`（`MenuItem` / `StatusBarItem`）
- `crates/ui/src/window/templates.rs`（`MenuBarTemplate` / `StatusBarTemplate`）
- `crates/ui/src/window/mod.rs` 中相关 re-export
- `ModernWindowShell::menu(Vec<MenuItem>)` / `status_bar(Vec<StatusBarItem>)`（[modern_window.rs:63-66, 86-89](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs#L63-L89)）
- `TabWindowShell::menu(Vec<MenuItem>)` / `status_bar(Vec<StatusBarItem>)`（如存在）
- `crates/engine/src/compiler/codegen.rs` 中 `gen_modern_window_wrapper` 的 `menu`/`status_bar` bind 分支（[codegen.rs:325-349](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L325-L349)）
- `gen_tab_window_wrapper` 的 `menu`/`status_bar` bind 分支（[codegen.rs:463-470](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L463-L470)）

### 新增 slot 标签识别
RML 新增三个标签（与已有 `<slot_left>` / `<slot_right>` / `<slot_bottom>` 同机制，硬编码识别）：
- `<slot_menu>...</slot_menu>`
- `<slot_status_bar>...</slot_status_bar>`
- `<slot_title_ext>...</slot_title_ext>`

### codegen 修改
- `partition_tab_slot_children`（[codegen.rs:380-410](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L380-L410)）：扩展识别三个新标签，返回元组增加 `slot_menu` / `slot_status_bar` / `slot_title_ext` 字段
- `gen_tab_window_wrapper`：输出 `.menu_slot(Some(...))` / `.status_slot(Some(...))` / `.title_ext_slot(Some(...))`
- `gen_modern_window_wrapper`（[codegen.rs:308-377](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L308-L377)）：同样走插槽路径（之前只处理 bind 属性，现在改为读子节点 slot 标签）
- `slot_element_content`（[codegen.rs:413-424](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L413-L424)）复用，三新标签走同一拆包逻辑

### demo 协调
- `demo/src/shell/main_window.rml`：`menu={menu_items}` → `<slot_menu>...</slot_menu>`、`status_bar={status_items}` → `<slot_status_bar>...</slot_status_bar>`
- 阶段 1 槽内暂用占位（如 `<slot_menu><div>...</div></slot_menu>`），阶段 2 替换为 `Menu` 控件

### 验证
- `cargo test` 全通过
- demo 运行：菜单栏/状态栏通过插槽渲染，与之前数据绑定效果一致

---

## 阶段 2：Step 2 ICommand + Menu/StatusBar 控件

**目标**：MVVM 数据契约采用 ICommand，告别 `Rc<dyn Fn>` 闭包。

### 复用 ICommand
`ICommand` 已定义于 [crates/core/src/command.rs:25](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs#L25)：
```rust
pub trait ICommand: 'static {
    fn execute(&mut self, parameter: &dyn std::any::Any, cx: &mut Context<Self>) where Self: Sized;
    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool { true }
}
```

### IMenuItem / IStatusItem 数据契约
- `crates/ui/src/components/menu.rs`：定义 `IMenuItem` trait
  ```rust
  pub trait IMenuItem: Send + Sync + 'static {
      fn label(&self) -> SharedString;
      fn icon(&self) -> Option<IconName>;
      fn disabled(&self) -> bool;
      fn checked(&self) -> bool;
      fn separator(&self) -> bool;
      fn children(&self) -> Vec<Arc<dyn IMenuItem>>;
      fn command(&self) -> Option<&Arc<dyn ICommand>>;
      fn parameter(&self) -> Option<&Arc<dyn Any>>;
  }
  ```
- `crates/ui/src/components/status_bar.rs`：定义 `IStatusItem` trait（label/icon/command）
- **关键约束**：所有字段用 `Arc<dyn ICommand>` / `Arc<dyn Any>`，满足 `Send + Sync`，可与 `#[computed]` 缓存兼容

### Menu / StatusBar 控件
- `Menu { items: Vec<Arc<dyn IMenuItem>>, on_command: Rc<dyn Fn(...)> }`：渲染复用 gpui-component `DropdownMenu`/`PopupMenu`
- 点击菜单项时：控件 emit 事件 → ViewModel `#[command]` 处理，或直接调 `ICommand::execute(parameter, cx)`
- codegen 注册新控件到 [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)

### RML 用法
```xml
<slot_menu>
    <Menu items={menu_items} />
</slot_menu>
```
- `menu_items: Vec<Arc<dyn IMenuItem>>` 由 `#[computed]` 产出
- 命令实现为 ViewModel 的 `#[command]` 方法包装成 `ICommand` 适配器

### demo 重写
- `demo/src/shell/main_window.rml.rs`：`menu_items` 返回 `Vec<Arc<dyn IMenuItem>>`，每个 item 的 command 是 ViewModel 命令适配器
- 命令适配器示例：`struct OpenFileCommand { weak: WeakEntity<MainWindow> }`，impl `ICommand::execute` 调 `weak.upgrade().update(cx, |this, cx| this.open_file(cx))`

### 验证
- `cargo test` 全通过
- demo 运行：菜单点击触发 ViewModel 命令，无 `Rc<dyn Fn>` 闭包

---

## 阶段 3：TreeView 强化（后续单独）
**不应该自定义TreeView吧，gpui-component已经提供了Tree控件，要解决的是RML的声明式语法转换为原生Tree控件的代码。**

**目标**：解决硬编码与 take() 抽干等历史问题。

### 强化方向
- 解耦 `state_field` 硬编码（[tags.rs:279](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L279)）：支持 RML `state={field}` 显式绑定
- 修复 `IActivityPanel::panel()` 的 `take()` 抽干问题（阶段 0 已临时修复）
- 补充 `on_expand` / `on_select` 事件与 `#[computed]` 数据源绑定
- 支持节点图标自定义、动态加载

---

## 实施顺序

1. **阶段 0**（本次优先）：5 个 demo bug 修复 → 验证 demo 跑通
2. **阶段 1**（本次次之）：插槽纯化 → 删除数据结构、添加 slot 标签
3. **阶段 2**（后续）：ICommand + Menu/StatusBar 控件
4. **阶段 3**（更后）：TreeView 强化

阶段 0 + 阶段 1 是本次实施范围；阶段 2、3 留作后续单独规划。

## 关键文件

- [crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs) — codegen 主逻辑
- [crates/ui/src/window/tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) — TabWindowShell
- [crates/ui/src/window/modern_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs) — ModernWindowShell
- [crates/ui/src/components/activity_bar.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs) — ActivityBar
- [crates/ui/src/components/tree_view.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tree_view.rs) — TreeView
- [crates/ui/src/window/types.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/types.rs) — 待删除
- [crates/ui/src/window/templates.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/templates.rs) — 待删除
- [crates/core/src/command.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs) — ICommand 复用
- [crates/app/src/application.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs) — 命令式入口
- [demo/src/app.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/app.rs) / [demo/src/login/login_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/login/login_window.rml.rs) / [demo/src/shell/main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) / [demo/src/shell/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

## 验证方法

1. `cargo build` 通过
2. `cargo test` 265+ 测试不回归
3. `cargo run --package rust-rml-demo`：
   - 登录窗单独显示
   - 输入用户名 → 进入 → 主窗口显示
   - 菜单栏可见（File / Help）
   - 活动栏图标按钮有 4 外边距，可点击切换激活状态
   - 活动栏面板显示案例树（3 个分类 + 子项）
   - TabBar 下方无边框线
   - 选中案例 → 打开新 Tab
   - Welcome Tab 默认显示
4. 阶段 1 完成后：RML 中 `<slot_menu>` / `<slot_status_bar>` 标签可用，旧 `menu=` / `status_bar=` 绑定被移除
