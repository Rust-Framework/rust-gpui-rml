# ActivityBar 与 TabWindow 重构计划

## 概述

上一轮迭代结果不理想，需重构两个核心问题：
1. `crates/ui/src/window/rml_title_bar.rs` 过度封装（复制了 gpui-component 的 TitleBar 仅为了过滤 min/max/close 按钮），应删除
2. `tab_window.rs` 未严格按 spec 设计（chrome_toggle 仅在 menu_slot 为空时显示，而非始终作为 prefix 首项）

附带重构：ActivityBar 引入 `IActivityPanel`/`IActivityAct` trait 抽象（spec 要求）。

**范围确认（经用户澄清）**：
- i18n 本地化系统：保留现状不动
- RmlApplication 全局访问：保留现状不动
- 重点调整：ActivityBar（引入 trait）+ TabWindow（按 spec 重构）+ 删除 rml_title_bar.rs

---

## 当前状态分析

### rml_title_bar.rs（待删除）
- 完整复制了 gpui-component 的 `TitleBar` 实现（含 `ControlIcon`/`WindowControls`/`TitleBarState`）
- 唯一差异：`WindowControls` 增加 `controls: WindowControlButtons` 字段，用于过滤 min/max/close 可见性
- 删除后：直接使用 `gpui_component::TitleBar`（接受其内置 3 个按钮，放弃过滤能力）

### modern_window.rs（待重构）
- 第 13 行 `use super::rml_title_bar::RmlTitleBar;`
- 第 24 行 `window_controls: WindowControlButtons` 字段
- 第 61-64 行 `window_controls()` builder 方法
- 第 121-122 行 `RmlTitleBar::new().window_controls(self.window_controls).child(...)`

### tab_window.rs（待重构）
- 第 20 行 `use super::rml_title_bar::RmlTitleBar;`
- 第 17 行 `use rml_core::window::WindowControlButtons;`
- 第 60 行 `window_controls: WindowControlButtons` 字段
- 第 116-119 行 `window_controls()` builder 方法
- **关键缺陷**（第 249-253 行）：chrome_toggle 仅在 menu_slot 为空时作为 prefix，与 spec 不符
- 第 271 行使用 `RmlTitleBar::new().window_controls(self.window_controls)`

### activity_bar.rs（待重构）
- 使用具体 struct `ActivityPanelItem`/`ActivityActionItem`，无 trait 抽象
- `ActivityPanelItem` 实现了 `Clone`（但 `panel` 字段设为 None，存在 panel 丢失隐患）
- codegen 生成 `.panels(vec.clone())`/`.actions(vec.clone())`，依赖返回类型实现 Clone

### codegen.rs（待修改）
- 第 377 行：`gen_modern_window_wrapper` 末尾 `.window_controls(self.window_controls())`
- 第 519 行：`gen_tab_window_wrapper` 末尾 `.window_controls(self.window_controls())`
- 第 583-592 行：`gen_window_extra_methods` 中从 `show_minimize`/`show_maximize`/`show_close` 属性生成 `window_controls()` 方法

---

## 设计决策

### 1. TitleBar 替换策略
使用 `gpui_component::TitleBar`（已在 `lib.rs` re-export）。TitleBar 内置 `WindowControls` 渲染 min/max/close 三个按钮，不支持过滤。接受此限制（用户已确认）。

### 2. ActivityBar Trait 设计

**trait 定义**（object-safe，`'static` bound）：

```rust
pub trait IActivityPanel: 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn is_activated(&self) -> bool;
    fn panel(&self) -> Option<AnyElement>;
}

pub trait IActivityAct: 'static {
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn on_click(&self, window: &mut Window, cx: &mut App);
    fn context_menu(&self) -> Vec<MenuItem>;
}
```

**克隆策略**：使用 `Arc<dyn IActivityPanel>` 而非 `Box<dyn IActivityPanel>`，因为：
- `Arc<T>` 是 `Clone`（不需要手写 `clone_box`）
- `#[computed]` 返回 `Vec<Arc<dyn IActivityPanel>>` 可被 ComputedCache 缓存和克隆
- Arc 克隆只复制引用计数，不丢失 `panel` 元素（修复现有 Clone 丢 panel 的隐患）

**类型别名**（解决 `#[computed]` 返回类型提取问题）：
```rust
pub type ActivityPanels = Vec<Arc<dyn IActivityPanel>>;
pub type ActivityActs = Vec<Arc<dyn IActivityAct>>;
```
> `return_type_str()` 使用 `split_whitespace().collect()` 会将 `dyn IActivityPanel` 合并为 `dynIActivityPanel`（无效标识符）。类型别名 `ActivityPanels` 是单 token，可正确提取。

**具体 struct 保留**：`ActivityPanel`/`ActivityAct` 作为默认实现，提供 `into_arc()` 便捷方法：
```rust
impl ActivityPanel {
    pub fn into_arc(self) -> Arc<dyn IActivityPanel> { Arc::new(self) }
}
```

### 3. TabWindow prefix 逻辑（严格按 spec）

spec 布局：`[图标(切换按钮)] [菜单] [标题] [Tab1] [Tab2] [Tab3] [扩展区] [窗口操作]`

prefix 逻辑：
1. chrome_toggle 按钮始终作为 prefix 首项（icon 设置时）
2. 当 `show_chrome=true`：菜单 + 标题也加入 prefix
3. 当 `show_chrome=false`：仅 chrome_toggle

---

## 变更计划

### Part 1: 删除 rml_title_bar.rs，重构 ModernWindowShell

**删除文件**：
- `crates/ui/src/window/rml_title_bar.rs` — 整个文件删除

**修改 `crates/ui/src/window/mod.rs`**：
- 删除第 14 行 `pub mod rml_title_bar;`

**修改 `crates/ui/src/window/modern_window.rs`**：
- 删除 `use super::rml_title_bar::RmlTitleBar;`（第 13 行）
- 删除 `use rml_core::window::WindowControlButtons;`（第 10 行）
- 添加 `use gpui_component::TitleBar;`（若 lib.rs 已 re-export 则用 `use crate::TitleBar;`）
- 删除 `window_controls: WindowControlButtons` 字段（第 24 行）
- 删除 `window_controls: WindowControlButtons::default()` 初始化（第 37 行）
- 删除 `pub fn window_controls(mut self, controls: WindowControlButtons) -> Self` 方法（第 61-64 行）
- render 中 `RmlTitleBar::new().window_controls(self.window_controls).child(...)` → `TitleBar::new().child(...)`（第 121-122 行）

### Part 2: 重构 TabWindowShell

**修改 `crates/ui/src/window/tab_window.rs`**：

1. **移除 import 和字段**：
   - 删除 `use super::rml_title_bar::RmlTitleBar;`（第 20 行）
   - 删除 `use rml_core::window::WindowControlButtons;`（第 17 行）
   - 添加 `use gpui_component::TitleBar;`（或 `use crate::TitleBar;`）
   - 删除 `window_controls: WindowControlButtons` 字段（第 60 行）
   - 删除 `window_controls: WindowControlButtons::default()` 初始化（第 83 行）
   - 删除 `pub fn window_controls(...)` 方法（第 116-119 行）

2. **重构 prefix 逻辑**（第 245-253 行替换）：
   ```rust
   let mut prefix_parts: SmallVec<[AnyElement; 3]> = SmallVec::new();

   // 1. chrome_toggle 始终作为首项（icon 设置时）
   if let Some(toggle) = chrome_toggle {
       prefix_parts.push(toggle);
   }

   // 2. show_chrome=true 时加入 menu + title
   if self.show_chrome {
       if let Some(menu) = self.menu_slot {
           prefix_parts.push(menu);
       }
       if let Some(title) = self.title {
           prefix_parts.push(
               div().px_2().child(title).into_any_element()
           );
       }
   }

   if !prefix_parts.is_empty() {
       tab_bar = tab_bar.prefix(h_flex().children(prefix_parts));
   }
   ```

3. **替换 TitleBar**（第 271-279 行）：
   ```rust
   // 旧：
   let title_bar = RmlTitleBar::new()
       .window_controls(self.window_controls)
       .child(div().flex_1().min_w_0().h_full().child(tab_bar));

   // 新：
   let title_bar = TitleBar::new()
       .child(div().flex_1().min_w_0().h_full().child(tab_bar));
   ```

4. **保留**：tabs_overflow / menu(tab_overflow) 逻辑、slot 布局（h_resizable/v_resizable）、on_tab_click / on_chrome_toggle 回调、default_sizes 方法

### Part 3: 重构 ActivityBar 引入 trait

**修改 `crates/ui/src/components/activity_bar.rs`**：

1. **新增 trait 定义**（文件顶部，import 之后）：
   ```rust
   use std::sync::Arc;

   pub trait IActivityPanel: 'static {
       fn id(&self) -> SharedString;
       fn icon(&self) -> IconName;
       fn title(&self) -> SharedString;
       fn is_activated(&self) -> bool;
       fn panel(&self) -> Option<AnyElement>;
   }

   pub trait IActivityAct: 'static {
       fn icon(&self) -> IconName;
       fn title(&self) -> SharedString;
       fn on_click(&self, window: &mut Window, cx: &mut App);
       fn context_menu(&self) -> Vec<MenuItem>;
   }

   pub type ActivityPanels = Vec<Arc<dyn IActivityPanel>>;
   pub type ActivityActs = Vec<Arc<dyn IActivityAct>>;
   ```

2. **重命名 + 实现 trait**：
   - `ActivityPanelItem` → `ActivityPanel`
   - `ActivityActionItem` → `ActivityAct`
   - 为两者实现 `IActivityPanel` / `IActivityAct` trait
   - 添加 `pub fn into_arc(self) -> Arc<dyn IActivityPanel>` / `Arc<dyn IActivityAct>` 便捷方法
   - `ActivityPanel` 的 `Clone` impl 可保留（用于内部使用），但不再是必需的

3. **ActivityBar 字段改为 trait 对象**：
   ```rust
   pub struct ActivityBar {
       panels: Vec<Arc<dyn IActivityPanel>>,
       actions: Vec<Arc<dyn IActivityAct>>,
       // ... 其他字段不变
   }
   ```

4. **builder 方法签名**：
   ```rust
   pub fn panels(mut self, panels: Vec<Arc<dyn IActivityPanel>>) -> Self
   pub fn actions(mut self, actions: Vec<Arc<dyn IActivityAct>>) -> Self
   ```

5. **render 方法调整**：
   - `self.panels.into_iter()` → `self.panels.iter()`（Arc 不需要消耗）
   - 通过 trait 方法访问字段：`panel.id.clone()` → `panel.id()`、`panel.icon` → `panel.icon()` 等
   - `panel.panel` → `panel.panel()`
   - `action.icon.clone()` → `action.icon()`
   - `action.title.clone()` → `action.title()`
   - `action.on_click.clone()` → 调用 `action.on_click(window, cx)`（trait 方法）
   - `action.context_menu.clone()` → `action.context_menu()`

### Part 4: 更新 exports、codegen、demo

**修改 `crates/ui/src/lib.rs`**（第 85 行）：
```rust
// 旧：
pub use components::{ActivityActionItem, ActivityBar, ActivityPanelItem, TreeView};

// 新：
pub use components::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels,
    IActivityAct, IActivityPanel, TreeView,
};
```

**修改 `crates/ui/src/prelude.rs`**：
在 `pub use crate::{...}` 中添加 ActivityBar 相关类型：
```rust
ActivityAct, ActivityBar, ActivityPanel, IActivityAct, IActivityPanel,
```

**修改 `crates/engine/src/compiler/codegen.rs`**：
1. 删除第 377 行：`code.push_str(".window_controls(self.window_controls())");`（gen_modern_window_wrapper）
2. 删除第 519 行：`code.push_str(".window_controls(self.window_controls())");`（gen_tab_window_wrapper）
3. 删除第 583-592 行：`show_minimize`/`show_maximize`/`show_close` 提取与 `window_controls()` 方法生成逻辑

**修改 `demo/src/shell/main_window.rml.rs`**：
```rust
// 旧：
use rml_ui::{ActivityPanelItem, IconName, MenuItem, StatusBarItem, TabItem, TreeState};

#[computed]
pub fn activity_icons(&self) -> Vec<ActivityPanelItem> {
    let _ = self.i18n_version;
    vec![
        ActivityPanelItem::new("samples", IconName::BookOpen, t_static("shell.samples"))
            .active(true),
    ]
}

// 新：
use rml_ui::{ActivityPanel, ActivityPanels, IconName, MenuItem, StatusBarItem, TabItem, TreeState};

#[computed]
pub fn activity_icons(&self) -> ActivityPanels {
    let _ = self.i18n_version;
    vec![
        ActivityPanel::new("samples", IconName::BookOpen, t_static("shell.samples"))
            .active(true)
            .into_arc(),
    ]
}
```

**`demo/src/shell/main_window.rml`**：无需修改（未使用 `show_minimize`/`show_maximize`/`show_close` 属性）

### Part 5: 验证

1. **编译验证**：`cargo build`（workspace 全量）
2. **测试验证**：`cargo test`（特别是 `crates/engine/tests/codegen_observable_test.rs`）
3. **demo 运行**：`cargo run -p demo` 确认 TabWindow 和 ActivityBar 正常显示
4. **关键检查点**：
   - TabWindow 的 chrome_toggle 按钮始终可见且可切换
   - show_chrome=false 时菜单和标题隐藏，chrome_toggle 仍显示
   - ActivityBar 面板切换正常
   - 窗口 min/max/close 按钮正常（来自 TitleBar 内置）
   - 插槽 resize 拖拽正常

---

## 假设与约束

1. `WindowControlButtons` 类型保留在 `crates/core/src/window.rs`（IWindow trait 的默认方法仍引用），仅 ModernWindowShell/TabWindowShell 不再消费
2. `IWindow::window_controls()` trait 方法保留（默认实现返回 `WindowControlButtons::default()`），codegen 不再生成覆盖方法
3. `gpui_component::TitleBar` 已在 `lib.rs` re-export（第 48 行确认存在）
4. ActivityBar codegen（`.panels(...)` / `.actions(...)`）无需修改——方法签名变化但 codegen 输出不变
5. `Arc<dyn IActivityPanel>` 不需要 `Send + Sync`（ComputedCache 通过 `unsafe impl Send + Sync` 已处理）

## 风险

1. **TitleBar 内置按钮无法过滤**：用户若需隐藏 min/max/close，需等待上游 gpui-component 支持。当前接受此限制。
2. **`#[computed]` 返回 `ActivityPanels` 类型别名**：需验证 `return_type_str()` 正确提取 `ActivityPanels`（单 token，应无问题）。若失败，回退方案为让 `activity_icons()` 返回具体 `Vec<ActivityPanel>` 并在传入 ActivityBar 前转换。
3. **TabBar prefix 单元素限制**：TabBar 的 `prefix()` 接受单个 `impl IntoElement`，需用 `h_flex()` 包裹多个 prefix 元素。
