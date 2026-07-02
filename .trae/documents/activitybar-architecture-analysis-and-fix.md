# ActivityBar 架构分析与修复计划

## 一、任务摘要

对比参考实现 `D:\GitCode\RF\rust-agent-ide` 中的 ActivityBar 组件，分析当前 `crates/ui/src/components/activity_bar.rs` 为何"整个逻辑不通，连面板都不显示内容"，并给出修复方案。

---

## 二、当前状态分析（Phase 1 探索结果）

### 2.1 参考实现（rust-agent-ide）—— 事件驱动的双 Entity 模型

核心文件：`crates/sdk-core/src/activity_bar.rs` + `crates/sdk-core/src/side_panel.rs`

**架构要点**：
1. **两个独立 Entity**：`ActivityBar`（仅图标）+ `SidePanel`（仅面板内容），职责分离
2. **事件解耦**：`ActivityBar` 点击 → `cx.emit(ActivityBarEvent::ItemActivated(id))` → `MainWindow` 订阅 → `side_panel.set_active_id(id)`
3. **三集合并行 + Arc 共享**：`ActivityBarHost` 持有 `items`/`entries`/`handles` 三个 `Arc<ContributionCollection>`，分别注入 ActivityBar 和 SidePanel
4. **PanelHandle trait**：`render(&self, window, cx) -> AnyElement` + `activate/deactivate` 生命周期钩子
5. **flush_pending 延迟回调**：`set_active_id` 只写 pending 字段，`SidePanel::render` 开头执行 `deactivate → activate` 时序
6. **启动默认激活**：MainWindow 构造时 `set_active_id` 第一项
7. **每帧快照无缓存**：`self.items.snapshot_sorted_by(|i| i.order)`，新贡献立即可见
8. **所有初始化在构造器**：`cx.new(|cx| { ... })`，绝不在 render 中做副作用

### 2.2 当前实现（rust-gpui-rml）—— 单组件 + 全局静态状态

核心文件：`crates/ui/src/components/activity_bar.rs` + `crates/app/src/contribution/activity_panel.rs`

**架构要点**：
1. **单一 `ActivityBar` (RenderOnce)**：同时承担图标栏 + 面板内容渲染
2. **全局静态状态持久化**：`OnceLock<Mutex<HashMap<String, ActiveState>>>` 按 `bar_id` 持久化激活态（因 RenderOnce 无状态）
3. **三级回退获取面板内容**：`panel_body` (Host 注入) → `panel.panel()` (RenderOnce 内调用) → `panel_children` (RML 子节点)
4. **codegen 注入条件**：`panel_body` 仅在 RML 同时绑定 `panels` + `active_panel_id` 时注入
5. **on_loaded 在 render 中执行**：`#[window]` 宏生成 `__rml_loaded` guard，首次 render 时调用 `on_loaded`，其中创建 Entity、修改 Global

---

## 三、根因分析：为什么面板不显示内容

### RC-1：codegen `panel_body` 注入在非受控模式下失效（直接原因）

**位置**：[crates/engine/src/compiler/component.rs#L159-L178](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L159-L178)

```rust
if let (Some(panels), Some(active)) = (panels_expr, active_expr) {
    code.push_str("\n.panel_body(rml_app::contribution::resolve_active_panel_body(...))");
}
```

**问题**：要求 `panels` 和 `active_panel_id` **同时**绑定才注入 `.panel_body(...)`。

**当前 RML**（[demo/src/shell/main_window.rml#L14](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml#L14)）：
```html
<ActivityBar ref="activity-bar" panels={activity_panels} on_panel_change="on_panel_change" />
```
只绑定 `panels`，**没有** `active_panel_id`（非受控模式）→ codegen 不注入 `panel_body` → `panel_body = None`。

**后果**：[activity_bar.rs#L317-L321](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L317-L321) 回退到 `panel.panel(window, cx)`，在 `RenderOnce` 内调用。

### RC-2：RenderOnce 内执行副作用（Global 修改）—— 违反 GPUI 纯函数契约

**调用链**：
```
ActivityBar::render (RenderOnce)
  → panel.panel(window, cx)                          [activity_bar.rs:321]
    → ContributedActivityPanel::panel
      → render_contribution_visual(&self.visual, ...) [activity_panel.rs:71]
        → cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
              cache.render_view("samples", CaseActivityPanel::default(), ctx)
          })                                          [render.rs:15-23]
```

**问题**：`RenderOnce::render` 内调用 `cx.update_global` —— 在渲染过程中修改全局状态。GPUI 要求 `render` 是纯函数。这会导致：
- 潜在无限循环（Global 修改 → 通知 → 重渲染 → Global 修改）
- 状态不一致或静默失败
- GPUI 可能 panic

**注意**：[activity_panel.rs#L75](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/activity_panel.rs#L75) 的注释明确写了"勿在 `RenderOnce` 内调用 `panel()`"，但 [activity_bar.rs#L317-L321](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L317-L321) 的回退路径恰恰违反了这条规则。

### RC-3：on_loaded 在 render 内执行（首次渲染副作用）

**位置**：`#[window]` 宏生成的 `__rml_loaded` guard 在 `MainWindow::render` 首次调用时触发 `on_loaded`。

**on_loaded 中的副作用**（[main_window.rml.rs#L39-L114](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L39-L114)）：
- `cx.new(|_| CaseActivityPanel::default())` —— 在 render 中创建 Entity
- `cx.update_global::<ContributionRegistryGlobal, _>(...)` —— 在 render 中修改 Global
- `self.refresh_bindings(cx)` —— 在 render 中修改 self 状态

**对比参考实现**：所有初始化在 `cx.new(|cx| { ... })` 构造器闭包中完成，绝不放在 render。

### RC-4：缓存未命中创建根 Entity（无父级观察链接）

**位置**：[contribution_cache.rs#L36,L44](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution_cache.rs#L36)

```rust
let entity = ctx.cx.new(|_| view);   // App::new —— 创建根 Entity
```

**问题**：`App::new` 创建的根 Entity 与 `MainWindow` 无父子观察链接。Entity 内的 `cx.notify()` 无法冒泡到父窗口触发重绘。

**当前缓解措施**：`pre_register` 在 `on_loaded` 中用 `Context::<MainWindow>::new`（有父级链接）创建 Entity 并注入缓存。但这只是权宜之计：
- 若缓存被清除或类型不匹配，回退到 `App::new` 创建根 Entity
- 首次 `on_loaded` 在 render 中执行 → pre_register 也在 render 中 → 副作用叠加

### RC-5：图标栏与面板内容耦合在单一组件

**当前**：`ActivityBar` 同时渲染图标栏（左 48px）+ 面板内容（flex_1 区域），用 `h_flex` 拼接。

**参考实现**：`ActivityBar`（仅图标）和 `SidePanel`（仅内容）是两个独立 Entity，通过事件同步。

**问题**：
- 图标栏和面板内容有不同生命周期和刷新频率
- 耦合导致面板内容渲染必须经过 RenderOnce，无法用独立 Entity 的 `cx.notify()` 局部刷新
- 全局静态状态 `BarState` 是为了绕过 RenderOnce 无状态限制的补丁

### RC-6：全局静态状态持久化（脆弱）

**位置**：[activity_bar.rs#L52-L62](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L52-L62)

```rust
fn active_state_for_bar(bar_id: &ElementId) -> ActiveState {
    static STATES: OnceLock<Mutex<HashMap<String, ActiveState>>> = OnceLock::new();
    // ...
}
```

**问题**：
- 状态以 `format!("{bar_id:?}")` 为键，跨窗口泄漏
- 无法重置、难以调试
- 违反 GPUI 的 Entity 状态管理模型
- 参考实现的状态在 `ActivityBar` Entity 字段中，随 Entity 生命周期管理

### RC-7：`on_panel_change` 是空操作

**位置**：[main_window.rml.rs#L143-L146](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L143-L146)

```rust
#[command]
pub fn on_panel_change(&mut self, _panel_id: &gpui::SharedString, cx: &mut Context<Self>) {
    cx.notify();   // 仅重绘，不处理 panel_id
}
```

非受控模式下 ViewModel 不跟踪激活面板 id，导致无法在 Host render 中解析面板内容（需要 `active_panel_id` 才能调用 `resolve_active_panel_body`）。

### RC-8：target/ 目录生成代码过时（可能加重症状）

源码修改时间 2026/7/2，`target/` 最后构建 2026/7/1。过时的生成代码引用已删除的字段（`active_panel_id`、`case_activity_panel`、`welcome_case`），项目当前可能无法编译。需 `cargo build` 重新生成。

---

## 四、根因总结（一句话）

**当前 ActivityBar 是一个 `RenderOnce` 无状态组件，却试图在 `render` 内完成"激活态管理 + 面板内容解析 + Global 修改 + Entity 创建"等副作用；codegen 的 `panel_body` 注入路径因非受控模式未绑定 `active_panel_id` 而失效，迫使回退到 `RenderOnce` 内调用 `panel.panel()` → `cx.update_global`，违反 GPUI 渲染纯函数契约，导致面板内容无法正确显示。**

参考实现的核心优势：**双 Entity + 事件解耦 + 构造器初始化 + flush_pending 延迟回调**，所有副作用都在 render 之外执行。

---

## 五、修复方案

### 方案选择：重构为双 Entity 事件驱动模型（对齐参考实现）

> 理由：当前单 RenderOnce + 全局静态状态的架构存在多个结构性缺陷（RC-2/3/4/5/6），补丁式修复会不断引入新问题。参考实现的双 Entity 模型已经过验证，职责清晰。

### 5.1 拆分为 `ActivityBar` Entity（图标栏）+ `ActivitySidePanel` Entity（面板内容）

**文件**：`crates/ui/src/components/activity_bar.rs`（重写）

**`ActivityBar` Entity**（仅图标 + 底部动作）：
```rust
pub struct ActivityBar {
    id: ElementId,
    panels: ActivityPanels,
    actions: ActivityActs,
    active_id: Option<SharedString>,   // Entity 自身持有，非全局静态
}

impl EventEmitter<ActivityBarEvent> for ActivityBar {}

impl ActivityBar {
    pub fn new(panels: ActivityPanels, cx: &mut Context<Self>) -> Self {
        let mut bar = Self { panels, active_id: None, ... };
        // 启动默认激活首项（在构造器，非 render）
        if let Some(first) = bar.panels.first() {
            bar.set_active_id(Some(first.id()), cx);
        }
        bar
    }

    pub fn set_active_id(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        if self.active_id == id { return; }
        match (&self.active_id, &id) {
            (Some(old), Some(new)) if old != new => {
                cx.emit(ActivityBarEvent::ItemDeactivated(old.clone()));
                cx.emit(ActivityBarEvent::ItemActivated(new.clone()));
            }
            (Some(old), None) => cx.emit(ActivityBarEvent::ItemDeactivated(old.clone())),
            (None, Some(new)) => cx.emit(ActivityBarEvent::ItemActivated(new.clone())),
            _ => {}
        }
        self.active_id = id;
        cx.notify();
    }

    pub fn active_id(&self) -> Option<&str> { self.active_id.as_deref() }
}

impl Render for ActivityBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 纯渲染：仅图标按钮，点击 toggle active_id + emit 事件
        // 不解析面板内容，不修改 Global
    }
}
```

**`ActivitySidePanel` Entity**（仅面板内容）：
```rust
pub struct ActivitySidePanel {
    panels: ActivityPanels,
    active_id: Option<SharedString>,
    pending_deactivate: Option<SharedString>,
    pending_activate: Option<SharedString>,
}

impl ActivitySidePanel {
    pub fn set_active_id(&mut self, id: Option<SharedString>, cx: &mut Context<Self>) {
        if self.active_id == id { return; }
        self.pending_deactivate = self.active_id.take();
        self.active_id = id.clone();
        self.pending_activate = id;
        cx.notify();
    }

    fn flush_pending(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // deactivate old → activate new（生命周期钩子）
    }
}

impl Render for ActivitySidePanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.flush_pending(window, cx);
        let has_active = self.active_id.is_some();
        div()
            .when(!has_active, |el| el.w_0())
            .when(has_active, |el| {
                el.child(
                    div().flex_1().min_w_0().min_h_0().overflow_hidden()
                        .children(
                            self.panels.iter()
                                .find(|p| p.id() == self.active_id.as_deref().unwrap_or(""))
                                .and_then(|p| p.panel(window, cx))
                        )
                )
            })
    }
}
```

**`ActivityBarEvent`**：
```rust
#[derive(Clone)]
pub enum ActivityBarEvent {
    ItemActivated(SharedString),
    ItemDeactivated(SharedString),
    SettingsRequested,
}
```

### 5.2 MainWindow 持有双 Entity + 事件订阅

**文件**：`demo/src/shell/main_window.rml.rs`

```rust
pub struct MainWindow {
    activity_bar: Entity<ActivityBar>,
    side_panel: Entity<ActivitySidePanel>,
    // ... 其他字段
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 构造 ActivityBar Entity（构造器内默认激活首项）
        let panels = map_activity_panels(Self::ID, cx);
        self.activity_bar = Some(cx.new(|cx| ActivityBar::new(panels, cx)));
        self.side_panel = Some(cx.new(|_| ActivitySidePanel::new()));

        // 订阅 ActivityBar 事件 → 同步 SidePanel
        if let Some(bar) = &self.activity_bar {
            cx.subscribe(bar, |this, _emitter, event, cx| match event {
                ActivityBarEvent::ItemActivated(id) => {
                    if let Some(panel) = &this.side_panel {
                        panel.update(cx, |p, cx| p.set_active_id(Some(id.clone()), cx));
                    }
                }
                ActivityBarEvent::ItemDeactivated(_) => {
                    if let Some(panel) = &this.side_panel {
                        panel.update(cx, |p, cx| p.set_active_id(None, cx));
                    }
                }
                _ => {}
            }).detach();
        }
    }
}
```

### 5.3 RML 布局调整

**文件**：`demo/src/shell/main_window.rml`

将 `<ActivityBar>` 从 `slot_left` 移除，改为 codegen 直接渲染双 Entity：

```html
<tab_window ...>
    <slot_left>
        <!-- 由 codegen 注入 ActivityBar + SidePanel 的 h_flex -->
        <ActivityBarShell bar={activity_bar} panel={side_panel} />
    </slot_left>
    ...
</tab_window>
```

或更简单：在 `slot_left` 中分别放置两个 Entity（需 codegen 支持 Entity 字段直接渲染）。

### 5.4 移除全局静态状态

删除 `active_state_for_bar` + `OnceLock<Mutex<HashMap<...>>>`（[activity_bar.rs#L43-L62](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L43-L62)）。状态由 `ActivityBar` Entity 字段管理。

### 5.5 移除 RenderOnce 内的 `panel.panel()` 回退

`ActivityBar`（图标 Entity）不再渲染面板内容，彻底消除 `RenderOnce` 内 `cx.update_global` 的副作用。

### 5.6 修复 codegen panel_body 注入逻辑（过渡方案）

若短期不重构为双 Entity，至少修复 codegen：在非受控模式下也注入 `panel_body`。

**文件**：[crates/engine/src/compiler/component.rs#L159-L178](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L159-L178)

将条件从"同时绑定 panels + active_panel_id"改为"只要绑定 panels 就注入"，`active_id` 从 ActivityBar 内部状态读取（需暴露只读访问器）。

但此方案仍无法解决 RC-2（RenderOnce 内副作用），仅作为过渡。

### 5.7 将 on_loaded 移出 render（长期）

`#[window]`/`#[component]` 宏应生成构造器钩子而非 render 内 guard。这需要修改 `crates/macros`，影响面较大，列为长期改进。

---

## 六、假设与决策

1. **假设**：GPUI 的 `RenderOnce` 契约要求 render 为纯函数，`cx.update_global` 在 render 中调用会导致未定义行为
2. **假设**：`Context::<T>::new` 创建的 Entity 有父级观察链接（T 观察新 Entity），`App::new` 创建根 Entity 无链接
3. **决策**：采用双 Entity 重构（方案 5.1-5.4），而非补丁式修复。理由：当前架构有 6 个结构性缺陷，补丁无法根治
4. **决策**：保留 `IActivityPanel` trait 接口（`id`/`icon`/`title`/`panel`），仅重构 ActivityBar 组件本身
5. **决策**：`on_loaded` 移出 render 列为长期改进，本期不修改 `crates/macros`

---

## 七、验证步骤

1. **编译验证**：`cargo build` 确认项目可编译（先清理 target/ 过时代码）
2. **面板显示验证**：启动 demo，确认左侧 ActivityBar 图标点击后面板内容正确显示
3. **toggle 验证**：点击已激活图标 → 面板收起；再点击 → 面板展开
4. **多面板切换验证**：若有多于一个 activity 贡献，切换时面板内容正确更新
5. **i18n 切换验证**：切换语言后，Tree 内容刷新
6. **默认激活验证**：启动时自动激活第一个面板
7. **无副作用验证**：在 `ActivityBar::render` 和 `ActivitySidePanel::render` 中无 `cx.update_global` / `cx.new` 调用
