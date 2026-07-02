# ActivityBar 单 Entity 重写计划

## 一、任务概述

彻底删除当前 ActivityBar 的双 Entity（ActivityBar + ActivitySidePanel + ActivityBarShell + EventEmitter）实现，回归 RML 框架设计初衷，参照 `D:\GitCode\RF\rust-agent-ide\crates\sdk-core\src\activity_bar.rs` 的可用方案，重写为**单 Entity**架构。

### 用户确认的设计决策

- **架构**：单 Entity（合并图标栏 + 面板内容到一个 `ActivityBar` Entity）
- **面板内容渲染**：保留 `IActivityPanel::panel()` 当前贡献系统方式

## 二、当前状态分析

### 现有问题

当前 [activity_bar.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs) 采用「双 Entity + 事件订阅」模型：

1. `ActivityBar` Entity：仅渲染图标栏，`set_active_id` 时 `cx.emit(ActivityBarEvent)`
2. `ActivitySidePanel` Entity：仅渲染面板内容，由 Host 订阅事件后调用 `set_active_id`
3. `ActivityBarShell` RenderOnce：水平排列两个 Entity
4. Host（MainWindow）在 `on_loaded` 中 `cx.subscribe` 联动

**运行时失败根因**：在 RML 的 `on_loaded`（位于 render 上下文内）中创建 Entity + 订阅事件 + 触发初始激活，事件传递时序脆弱：构造器 emit 的事件早于 subscribe 注册；`bar.update` 内的 emit 可能延迟到 update 闭包返回后投递，导致首次 render 时 SidePanel 仍为 `active_id = None`，面板空白。

### 参考实现（rust-agent-ide）可用方案

[sdk-core/src/activity_bar.rs](file:///D:/GitCode/RF/rust-agent-ide/crates/sdk-core/src/activity_bar.rs) 关键设计：
- 单 `ActivityBar` Entity：`items + active_id`，`EventEmitter<ActivityBarEvent>`
- `set_active_id` 直接修改 `self.active_id` + `cx.notify()` + `cx.emit`
- `render` 内直接渲染图标按钮 + 激活态高亮
- 面板内容由独立的 `SidePanel` Entity 渲染（参考实现里确实是双 Entity，但 Host 在 `new()` 中创建并 subscribe，不在 render 上下文）

### 重写策略

**核心简化**：在 RML 场景下，将图标栏 + 面板内容合并到**同一个** `ActivityBar` Entity：
- 消除事件订阅：单 Entity 内 `set_active_id` 直接改字段 + `cx.notify()` 触发自身重渲
- 消除 SidePanel/Shell：`render` 直接 `h_flex(bar + panel_body)`
- 消除 EventEmitter：单 Entity 无需事件同步自身
- 面板内容仍由 `IActivityPanel::panel()` 提供（保留贡献系统）

## 三、具体修改方案

### 文件 1：[crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)

**目标**：新增 `ComponentKind::EntityRef` 变体，注册 `"ActivityBar"`，移除 `"ActivityBarShell"`。

**修改点**：

1. **`ComponentKind` 枚举**（L240-251）：新增 `EntityRef` 变体
   ```rust
   pub enum ComponentKind {
       Stateless,
       StatelessNoId,
       Stateful { state_field: &'static str },
       /// Entity 引用组件：从 Host 的 `Entity<T>` 字段直接 clone
       /// 配合 `ref="field_name"` 指令指定字段名
       /// 生成 `self.<field>.as_ref().expect("init in on_loaded").clone()`
       EntityRef,
   }
   ```

2. **组件路由表**（L336-339）：替换 `"ActivityBarShell"` 注册为 `"ActivityBar"`
   ```rust
   // 删除：
   "ActivityBarShell" => Some(ComponentTag {
       ctor_path: "rml_ui::ActivityBarShell",
       kind: ComponentKind::StatelessNoId,
   }),
   // 新增：
   "ActivityBar" => Some(ComponentTag {
       ctor_path: "rml_ui::ActivityBar",
       kind: ComponentKind::EntityRef,
   }),
   ```

### 文件 2：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

**目标**：为 `EntityRef` 添加 codegen 分支（早返回，跳过 setter/children），移除 `ActivityBarShell` 特例。

**修改点**：

1. **构造器生成**（L65-86）：在 `match component.kind` 中新增 `EntityRef` 分支
   ```rust
   let mut code = match component.kind {
       tags::ComponentKind::Stateless => { /* ... */ }
       tags::ComponentKind::StatelessNoId => { /* ... */ }
       tags::ComponentKind::Stateful { state_field } if tag == "Tree" => { /* ... */ }
       tags::ComponentKind::Stateful { state_field } => { /* ... */ }
       tags::ComponentKind::EntityRef => {
           // EntityRef：必须配合 ref="field_name" 指令
           let name = ref_name.ok_or_else(|| CodegenError {
               message: format!(
                   "EntityRef component <{}> requires `ref=\"field_name\"` directive",
                   tag
               ),
           })?;
           return Ok(format!(
               "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
               name, name
           ));
       }
   };
   ```

2. **容器组件排除列表**（L126-130）：移除 `&& resolved != "ActivityBarShell"`
   ```rust
   let is_container = matches!(component.kind, tags::ComponentKind::StatelessNoId)
       && resolved != "menu"
       && resolved != "MenuBar"
       && resolved != "status_bar";
   // 移除：&& resolved != "ActivityBarShell"
   ```

3. **`component_bind_setter`**（L300-307）：移除 `bar`/`panel` 的 `ActivityBarShell` 特例
   ```rust
   // 删除：
   "bar" if tag == "ActivityBarShell" => Some(format!(
       ".bar({}.as_ref().expect(\"init ActivityBar\").clone())",
       rust_expr
   )),
   "panel" if tag == "ActivityBarShell" => Some(format!(
       ".panel({}.as_ref().expect(\"init ActivitySidePanel\").clone())",
       rust_expr
   )),
   ```

### 文件 3：[crates/ui/src/components/activity_bar.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs)

**目标**：完全重写为单 Entity，删除所有旧代码。

**新文件内容**：

```rust
//! ActivityBar —— VS Code 风格左侧活动栏（单 Entity 模型）
//!
//! 架构：
//! - 单 `ActivityBar` Entity：同时渲染图标栏 + 面板内容
//! - `set_active_id` 直接修改字段 + `cx.notify()` 触发自身重渲
//! - 无 EventEmitter、无 SidePanel、无 Shell
//!
//! RML 用法：`<ActivityBar ref="activity_bar" />`
//! Host 在 `on_loaded` 中 `cx.new(|_| ActivityBar::new(panels))` 创建并激活首项。

use std::sync::Arc;

use gpui::{
    AnyElement, App, Context, Entity, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div, px, prelude::FluentBuilder as _,
};
use gpui_component::{
    ActiveTheme, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use smallvec::SmallVec;

// ── Trait 定义 ──

/// 活动栏面板项接口
pub trait IActivityPanel: Send + Sync + 'static {
    fn id(&self) -> SharedString;
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let _ = (window, cx);
        None
    }
}

/// 活动栏底部动作项接口
pub trait IActivityAct: Send + Sync + 'static {
    fn icon(&self) -> IconName;
    fn title(&self) -> SharedString;
    fn on_click(&self, window: &mut Window, cx: &mut App);
}

pub type ActivityPanels = Vec<Arc<dyn IActivityPanel>>;
pub type ActivityActs = Vec<Arc<dyn IActivityAct>>;

// ── 默认实现 ──

pub struct ActivityPanel {
    id: SharedString,
    icon: IconName,
    title: SharedString,
}

impl ActivityPanel {
    pub fn new(
        id: impl Into<SharedString>,
        icon: IconName,
        title: impl Into<SharedString>,
    ) -> Self {
        Self { id: id.into(), icon, title: title.into() }
    }
    pub fn into_arc(self) -> Arc<dyn IActivityPanel> {
        Arc::new(self)
    }
}

impl IActivityPanel for ActivityPanel {
    fn id(&self) -> SharedString { self.id.clone() }
    fn icon(&self) -> IconName { self.icon.clone() }
    fn title(&self) -> SharedString { self.title.clone() }
}

pub struct ActivityAct {
    icon: IconName,
    title: SharedString,
    on_click: Option<Arc<dyn Fn(&mut Window, &mut App) + Send + Sync>>,
}

impl ActivityAct {
    pub fn new(icon: IconName, title: impl Into<SharedString>) -> Self {
        Self { icon, title: title.into(), on_click: None }
    }
    pub fn on_click(
        mut self,
        f: impl Fn(&mut Window, &mut App) + Send + Sync + 'static,
    ) -> Self {
        self.on_click = Some(Arc::new(f));
        self
    }
    pub fn into_arc(self) -> Arc<dyn IActivityAct> {
        Arc::new(self)
    }
}

impl IActivityAct for ActivityAct {
    fn icon(&self) -> IconName { self.icon.clone() }
    fn title(&self) -> SharedString { self.title.clone() }
    fn on_click(&self, window: &mut Window, cx: &mut App) {
        if let Some(f) = &self.on_click { f(window, cx); }
    }
}

// ── ActivityBar Entity（单 Entity：图标栏 + 面板内容） ──

pub struct ActivityBar {
    panels: ActivityPanels,
    actions: ActivityActs,
    active_id: Option<SharedString>,
    bar_width: gpui::Pixels,
}

impl ActivityBar {
    pub fn new(panels: ActivityPanels) -> Self {
        Self {
            panels,
            actions: Vec::new(),
            active_id: None,
            bar_width: px(48.),
        }
    }

    /// 激活首个面板。Host 在 `on_loaded` 中创建 Entity 后调用。
    pub fn activate_first(&mut self, cx: &mut Context<Self>) {
        if let Some(first) = self.panels.first() {
            self.set_active_id(Some(first.id()), cx);
        }
    }

    pub fn set_panels(&mut self, panels: ActivityPanels, cx: &mut Context<Self>) {
        self.panels = panels;
        cx.notify();
    }

    pub fn set_actions(&mut self, actions: ActivityActs, cx: &mut Context<Self>) {
        self.actions = actions;
        cx.notify();
    }

    pub fn set_active_id(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        if self.active_id == id {
            return;
        }
        self.active_id = id;
        cx.notify();
    }

    pub fn active_id(&self) -> Option<&str> {
        self.active_id.as_deref()
    }
}

impl Render for ActivityBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_id = self.active_id.clone();

        // ── 图标栏 ──
        let mut panel_buttons: SmallVec<[AnyElement; 4]> = SmallVec::new();
        for (ix, panel) in self.panels.iter().enumerate() {
            let id = panel.id();
            let icon = panel.icon();
            let title = panel.title();
            let active = active_id.as_ref() == Some(&id);

            panel_buttons.push(
                Button::new(("activity-panel", ix))
                    .ghost()
                    .icon(icon)
                    .tooltip(title)
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .when(active, |btn| btn.bg(cx.theme().sidebar_accent))
                    .on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {
                        let new_id = if this.active_id.as_ref() == Some(&id) {
                            None
                        } else {
                            Some(id.clone())
                        };
                        this.set_active_id(new_id, cx);
                    }))
                    .into_any_element(),
            );
        }

        let action_buttons: SmallVec<[AnyElement; 4]> = self
            .actions
            .iter()
            .enumerate()
            .map(|(ix, action)| {
                let action = action.clone();
                Button::new(("activity-action", ix))
                    .ghost()
                    .icon(action.icon())
                    .tooltip(action.title())
                    .h(px(36.))
                    .w(px(36.))
                    .my(px(2.))
                    .on_click(move |_, window, cx| action.on_click(window, cx))
                    .into_any_element()
            })
            .collect();

        let bar = v_flex()
            .w(self.bar_width)
            .h_full()
            .flex_shrink_0()
            .justify_between()
            .bg(cx.theme().sidebar)
            .child(v_flex().w_full().items_center().children(panel_buttons))
            .child(v_flex().w_full().items_center().children(action_buttons));

        // ── 面板内容 ──
        let panel_body = if active_id.is_some() {
            let active_id = self.active_id.clone();
            let body = self
                .panels
                .iter()
                .find(|p| p.id() == active_id.as_deref().unwrap_or(""))
                .and_then(|panel| panel.panel(window, cx));
            match body {
                Some(body) => div()
                    .flex_1()
                    .h_full()
                    .min_w_0()
                    .overflow_hidden()
                    .bg(cx.theme().sidebar)
                    .child(body)
                    .into_any_element(),
                None => div().w_0().h_full().into_any_element(),
            }
        } else {
            div().w_0().h_full().into_any_element()
        };

        h_flex().h_full().child(bar).child(panel_body)
    }
}
```

**关键变化**：
- 删除 `ActivityBarEvent` 枚举 + `EventEmitter<ActivityBarEvent> for ActivityBar`
- 删除 `ActivitySidePanel` 结构体 + impl
- 删除 `ActivityBarShell` 结构体 + impl + `#[derive(IntoElement)]`
- `ActivityBar::set_active_id` 简化：仅改字段 + `cx.notify()`（无 emit）
- `Render::render` 合并：`h_flex(bar + panel_body)`
- 保留 `IActivityPanel`/`IActivityAct`/`ActivityPanel`/`ActivityAct`/`ActivityPanels`/`ActivityActs` 接口与默认实现（贡献系统依赖）

### 文件 4：[crates/ui/src/components/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs) + [crates/ui/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)

**目标**：移除已删除类型的 re-export。

**`mod.rs` L9-12 修改**：
```rust
pub use activity_bar::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels, IActivityAct,
    IActivityPanel,
};
// 移除：ActivityBarEvent, ActivityBarShell, ActivitySidePanel
```

**`lib.rs` L81-87 修改**：
```rust
pub use components::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels, DialogDragState,
    DialogTitleBar, IActivityAct, IActivityPanel, IMenuItem, IStatusBarItem, Menu, MenuBar,
    MenuItem, MenuItems, NativeStatusBar, StatusBar, StatusBarAlign, StatusBarItem,
    StatusBarItems, Tree, configure_menu_bar_popup, menu_bar_button, render_menu_bar_from_items,
};
// 移除：ActivityBarEvent, ActivityBarShell, ActivitySidePanel
```

### 文件 5：[demo/src/shell/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

**目标**：移除 `side_panel` 字段、`cx.subscribe`、直接激活逻辑简化。

**修改点**：

1. **import**（L9-12）：移除 `ActivityBarEvent`/`ActivitySidePanel`
   ```rust
   use rml_ui::{
       ActivityBar, ActivityPanels, MenuItems, StatusBarItems, TabItem,
   };
   ```

2. **结构体字段**（L34-35）：移除 `side_panel`
   ```rust
   activity_bar: Option<gpui::Entity<ActivityBar>>,
   // 移除：side_panel: Option<gpui::Entity<ActivitySidePanel>>,
   ```

3. **`on_loaded` 中 ActivityBar 初始化**（L120-152）：简化
   ```rust
   // 构造 ActivityBar 单 Entity（在 on_loaded 中，非 render）
   let panels = self.activity_panels.clone();
   self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

   // 激活首项 —— 单 Entity 内 set_active_id 直接 cx.notify() 触发重渲
   if let Some(bar) = &self.activity_bar {
       bar.update(cx, |bar, cx| bar.activate_first(cx));
   }
   ```
   移除：`cx.subscribe` 订阅块、`side_panel` 创建、`panel.update(set_active_id)` 直接同步。

4. **`refresh_bindings` 中同步面板数据**（L168-173）：移除 `side_panel` 分支
   ```rust
   if let Some(bar) = &self.activity_bar {
       bar.update(cx, |bar, cx| bar.set_panels(activity_panels.clone(), cx));
   }
   // 移除：if let Some(panel) = &self.side_panel { ... }
   ```

### 文件 6：[demo/src/shell/main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

**目标**：替换 RML 标签。

**L13-15 修改**：
```xml
<slot_left>
    <ActivityBar ref="activity_bar" />
</slot_left>
```
移除：`<ActivityBarShell bar={activity_bar} panel={side_panel} />`

## 四、假设与决策

1. **`ref` 指令复用**：当前 `ref="name"` 用于 Stateless 组件的稳定 ElementId。对 `EntityRef` kind 复用此指令指定 Host 字段名，语义自然延伸（"引用 Host 的某字段"），无需新增指令。

2. **无 `pending_*` 字段**：参考实现的 `flush_pending` 模式用于双 Entity 间在 render 时执行 activate/deactivate 生命周期回调。单 Entity 模型下 `IActivityPanel` 无生命周期回调（仅 `panel()` 渲染方法），故无需 pending 缓冲。

3. **无 EventEmitter**：单 Entity 的 `set_active_id` 直接修改自身 `active_id` + `cx.notify()` 触发重渲，无需事件通知外部。Host 若需感知激活变化（当前 MainWindow 无此需求），可后续通过 `cx.observe` 监听 Entity。

4. **`panel()` 在 render 中调用**：`IActivityPanel::panel(&self, window, cx)` 接收 `&self`（不可变引用），在 `ActivityBar::render` 的 `&mut self` 上下文中调用安全（`panels` 字段遍历时只读）。

5. **保留 `set_panels`/`set_actions`**：供 Host 在 `refresh_bindings` 中同步贡献数据。

## 五、验证步骤

### 编译验证

```bash
cargo build -p rust-rml-engine
cargo build -p rust-rml-ui
cargo build -p rust-rml-demo
```

### 单元测试

```bash
cargo test -p rust-rml-engine
```
验证：tags.rs 的 `component_lookup("ActivityBar")` 返回 `EntityRef` kind；`component_lookup("ActivityBarShell")` 返回 `None`。

### 集成测试

确认 `crates/engine/tests/codegen_observable_test.rs` 等 7 个测试仍通过（与 ActivityBar 无关，但需确认 codegen 改动无回归）。

### 运行时验证

启动 demo：
```bash
cargo run -p rust-rml-demo
```
预期：
1. 主窗口左侧出现 ActivityBar 图标栏（48px 宽，sidebar 背景色）
2. 首个面板（samples / BookOpen 图标）自动激活，右侧显示 CaseActivityPanel 的树内容
3. 点击已激活图标 → 面板收起（`w_0`），再点击 → 重新展开
4. 点击其他图标 → 切换面板内容
5. 无空白面板、无事件丢失

### 回归检查

- 确认 `case_activity_panel.rml.rs` 的 `#[contribute]` 贡献系统无需改动（`IActivityPanel::panel()` 接口未变）
- 确认 `shell_chrome.rs` 的 `map_activity_panels` 无需改动（返回 `ActivityPanels` 类型未变）
- 确认 `contribution/activity_panel.rs` 的 `ContributedActivityPanel` 无需改动

## 六、实施顺序

1. **tags.rs**：新增 `EntityRef` + 注册 `ActivityBar` + 移除 `ActivityBarShell`
2. **component.rs**：新增 `EntityRef` codegen 分支 + 移除 `ActivityBarShell` 特例
3. **activity_bar.rs**：完全重写（单 Entity）
4. **mod.rs + lib.rs**：更新 exports
5. **main_window.rml.rs**：简化 Host 代码
6. **main_window.rml**：替换 RML 标签
7. **编译 + 测试 + 运行验证**

按此顺序可保证每一步编译可过（先改框架路由，再改组件实现，最后改消费方）。
