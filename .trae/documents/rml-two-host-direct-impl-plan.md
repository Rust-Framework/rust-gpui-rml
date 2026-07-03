# 双 Host 直接实现计划：MainWindow + ActivityPanel

## 摘要

本计划解决两个问题：

1. **`MainWindowHost` 多此一举** —— 用户质问"为何不直接 `impl IContributionHost for MainWindow`"。根因：`MainWindow` 是 GPUI Entity，无法提取 `Arc<MainWindow>` 以注册到 registry（registry 的 `add` 接受 `Arc<dyn IContributionHost>`）。当前用 `MainWindowHost` 作为 `Arc` 可共享的数据桥接。
2. **双 Host 架构** —— ActivityPanel 处理案例贡献（`demo.activity`），MainWindow 处理菜单/状态栏贡献（`demo.shell`）。

**核心解法**：为 registry 新增 `take_pending(host_id)` API —— Entity host 在 `on_loaded` 中直接取出 pending 贡献，调用 `self.add(...)` 受理。无需 `Arc<dyn IContributionHost>` 注册，无需 `MainWindowHost` 桥接结构。

**核心架构转变**：

```
旧：MainWindow → Arc<MainWindowHost> → registry.add(host) → host.add() 重放 pending
新：MainWindow → registry.take_pending("demo.shell") → self.add() 直接受理
```

***

## 为何 `MainWindowHost` 存在（根因分析）

### 约束链

1. `IContributionRegistry::add(host: Arc<dyn IContributionHost>)` —— 接受 `Arc`，不接受 Entity
2. `MainWindow` 是 GPUI Entity（`#[window]` 宏生成）—— GPUI 内部以 `Arc` 管理 Entity 生命周期，但不暴露 `Arc<MainWindow>`，无法从 `Entity<MainWindow>` 提取
3. 因此 `impl IContributionHost for MainWindow` 虽合法，但**无法注册**到 registry

### 当前解法（多此一举）

```rust
// MainWindowHost 作为 Arc 可共享的数据容器
pub struct MainWindowHost {
    entries: ObservableVec<ContributedEntry>,  // &self 可变（RwLock 内部可变性）
}
impl IContributionHost for MainWindowHost { ... }

// MainWindow Entity 持有 Arc<MainWindowHost>
pub struct MainWindow {
    host: Option<Arc<MainWindowHost>>,  // ← 多余的间接层
}

// on_loaded 中注册 host
cx.get_contribution_registry().add(host);  // Arc<MainWindowHost> → Arc<dyn IContributionHost>
```

**问题**：`MainWindowHost` 只是 `ObservableVec` 的包装，`MainWindow` 仍需通过 `host.entries.read()` 间接访问数据。Entity 与 host 数据分离，增加心智负担。

### 新解法：`take_pending` 绕过 Arc 注册

```rust
// IContributionRegistry 新增方法
fn take_pending(&self, host_id: &str) -> Vec<(Arc<dyn IContribution>, ContributionOptions)>;

// MainWindow 直接 impl IContributionHost
pub struct MainWindow {
    entries: ObservableVec<ContributedEntry>,  // 直接持有，无间接层
    ...
}
impl IContributionHost for MainWindow {
    fn add(&self, c, o) { self.entries.push(ContributedEntry { contribution: c, options: o }); }
    fn remove(&self, id) { self.entries.retain(|e| e.contribution.id() != id); }
}

// on_loaded 中直接取出 pending
let pending = cx.get_contribution_registry().take_pending(Self::ID);
for (c, o) in pending { self.add(c, o); }  // 直接调用，无需 Arc
```

**为何可行**：
- `register(host_id, contribution, options)` 在 host 未注册时入 pending 队列（现有逻辑）
- `take_pending(host_id)` 取出并清空 pending 队列（新增）
- Entity host 的 `add(&self, ...)` 操作自身 `ObservableVec`（`&self` 安全），无需 `cx`
- channel + spawn 仍负责通知 Entity 重渲（`ObservableVec::push` 发 `flume` 信号 → spawn 调 `cx.notify()`）

***

## 双 Host 架构

### 贡献路由

| host_id         | Host Entity    | 接收的贡献                                  | 渲染位置           |
| --------------- | -------------- | ------------------------------------------- | ------------------ |
| `demo.shell`    | `MainWindow`   | `kind = "menu"` + `kind = "status"` 贡献    | MainWindow 自身    |
| `demo.activity` | `ActivityPanel`| `kind = "case"` 贡献                        | ActivityPanel Entity（ActivityBar 面板） |

### 数据流

```
App 启动
  ↓
register_rml_contributions(cx)
  → 所有 #[contribute] 调 registry.register(host_id, ...)
  → host 未注册 → 入 pending 队列
  ↓
MainWindow::on_loaded
  → take_pending("demo.shell") → self.add(menu/status 贡献)
  → 创建 ActivityPanel Entity
  ↓
ActivityPanel::on_loaded
  → take_pending("demo.activity") → self.add(case 贡献)
  → channel + spawn 通知重渲
  ↓
渲染：MainWindow 创建 ActivityBar，ActivityBar 面板内容 = ActivityPanel Entity.render()
```

### ActivityPanel 角色

ActivityPanel **不再是 `#[contribute]` 视觉贡献**，而是：
1. **Entity**（由 MainWindow `cx.new` 创建）—— 实现 `Render` 渲染案例树
2. **IContributionHost**（`demo.activity`）—— 接收 case 贡献，存入 `ObservableVec`

ActivityBar 通过 `ActivityPanelEntityAdapter`（包装 `Entity<ActivityPanel>` 为 `IActivityPanel`）获取面板内容。

***

## 变更清单

### Phase 1：Registry 新增 `take_pending`

**文件**：`crates/core/src/contribution.rs`

`IContributionRegistry` trait 新增方法：

```rust
pub trait IContributionRegistry: Send + Sync {
    fn add(&self, host: Arc<dyn IContributionHost>);
    fn remove(&self, host_id: &str);
    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions);
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;

    /// Entity host 在 on_loaded 中调用：取出 pending 贡献，自行 add 受理。
    /// 取出后 pending 队列清空。后续 register 调用仍入 pending（Entity host 不注册 Arc）。
    fn take_pending(&self, host_id: &str) -> Vec<(Arc<dyn IContribution>, ContributionOptions)>;
}
```

**文件**：`crates/app/src/contribution/registry.rs`

`ContributionRegistry` impl `take_pending`：

```rust
fn take_pending(&self, host_id: &str) -> Vec<(Arc<dyn IContribution>, ContributionOptions)> {
    let mut pending = self.pending.write().unwrap();
    pending.remove(host_id).unwrap_or_default()
}
```

### Phase 2：MainWindow 直接 impl IContributionHost

**文件**：`demo/src/shell/main_window.rml.rs`

**移除**：
- `MainWindowHost` struct
- `#[contributehost(id = "demo.shell")]` on `MainWindowHost`
- `impl IContributionHost for MainWindowHost`
- `host: Option<Arc<MainWindowHost>>` 字段

**变更**：
- `MainWindow` 新增 `#[contributehost(id = "demo.shell")]` 属性（生成 `pub const ID`）
- `MainWindow` 新增字段 `entries: ObservableVec<ContributedEntry>`
- `MainWindow` 直接 `impl IContributionHost`（`add`/`remove` 操作 `self.entries`）
- `on_loaded` 中：
  - 创建 channel + spawn（同当前模式）
  - `self.entries = ObservableVec::with_notifier(tx)`
  - `let pending = cx.get_contribution_registry().take_pending(Self::ID);`
  - `for (c, o) in pending { self.add(c, o); }`
  - 创建 ActivityPanel Entity + ActivityBar（见 Phase 3）
- `refresh_shell_chrome` 读取 `self.entries.read()`（不再通过 `host.entries`）
- `active_case_view` 读取 `self.entries.read()` 查找 `IVisualContribution`
- **移除** `case_tree_items()` 方法（案例树数据移至 ActivityPanel）

### Phase 3：ActivityPanel 直接 impl IContributionHost

**文件**：`demo/src/shell/activity_panel.rml.rs`

**完整重写**。移除 `#[contribute]` 属性（不再是视觉贡献），改为：

```rust
#[contributehost(id = "demo.activity")]
#[derive(Default)]
pub struct ActivityPanel {
    entries: ObservableVec<ContributedEntry>,
    tree_state: Option<gpui::Entity<TreeState>>,
}

impl IContributionHost for ActivityPanel {
    fn id(&self) -> &'static str { Self::ID }
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions) {
        self.entries.push(ContributedEntry { contribution, options });
    }
    fn remove(&self, contribution_id: &str) {
        self.entries.retain(|e| e.contribution.id() != contribution_id);
    }
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // channel + spawn 通知重渲
        let (tx, rx) = flume::unbounded::<()>();
        self.entries = ObservableVec::with_notifier(tx);
        cx.spawn(async move |this, cx| {
            while rx.recv_async().await.is_ok() {
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        }).detach();

        // 取出 case 贡献
        let pending = cx.get_contribution_registry().take_pending(Self::ID);
        for (c, o) in pending { self.add(c, o); }

        // 初始化树
        self.refresh_tree(cx);

        // observe i18n 变化 → 刷新树
        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        }).detach();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let entries = self.entries.read();
        let items = map_case_tree_items(&entries);
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| s.set_items(items, cx));
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }

    pub fn case_tree_items(&self) -> Vec<TreeItem> {
        let entries = self.entries.read();
        map_case_tree_items(&entries)
    }
}

impl Render for ActivityPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 渲染案例树（使用 self.tree_state）
        // 命令绑定：on_case_activate → 通过 DemoShellHost 调 MainWindow::open_case
    }
}
```

### Phase 4：ActivityBar 面板适配

**文件**：`demo/src/shell/main_window.rml.rs`（MainWindow::on_loaded 内）

创建 `ActivityPanelEntityAdapter` 包装 `Entity<ActivityPanel>` 为 `IActivityPanel`：

```rust
// 在 MainWindow::on_loaded 中：
let activity_entity = cx.new(|cx| {
    let mut panel = ActivityPanel::default();
    // on_loaded 由 RML 框架自动调用
    panel
});

// 创建 adapter
let adapter = ActivityPanelEntityAdapter::new(
    activity_entity.clone(),
    "samples",
    IconName::BookOpen,
    cx.t("shell.samples"),
);

let panels: ActivityPanels = vec![adapter.into_arc()];
self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

// ActivityBar observe ActivityPanel → 面板内容刷新
cx.observe(&activity_entity, {
    let bar = self.activity_bar.clone().unwrap();
    move |_, _, cx| {
        bar.update(cx, |_, cx| cx.notify());
    }
}).detach();
```

**新增 struct**：`ActivityPanelEntityAdapter`（在 `main_window.rml.rs` 或 `activity_panel.rml.rs` 中）

```rust
struct ActivityPanelEntityAdapter {
    entity: gpui::Entity<ActivityPanel>,
    id: gpui::SharedString,
    icon: IconName,
    title: gpui::SharedString,
}

impl ActivityPanelEntityAdapter {
    fn new(entity, id, icon, title) -> Self { ... }
    fn into_arc(self) -> Arc<dyn IActivityPanel> { Arc::new(self) }
}

impl IActivityPanel for ActivityPanelEntityAdapter {
    fn id(&self) -> SharedString { self.id.clone() }
    fn icon(&self) -> IconName { self.icon }
    fn title(&self) -> SharedString { self.title.clone() }
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(self.entity.update(cx, |panel, cx| {
            panel.render(window, cx).into_any_element()
        }))
    }
}
```

### Phase 5：Case 贡献路由变更

**文件**：`demo/src/cases/*.rml.rs`（10 个案例文件）

将 `host_id = "demo.shell"` 改为 `host_id = "demo.activity"`：

- `button_case.rml.rs`
- `counter_case.rml.rs`
- `i18n_case.rml.rs`
- `menu_context_case.rml.rs`
- `menu_custom_case.rml.rs`
- `menu_dropdown_case.rml.rs`
- `menu_editor_case.rml.rs`
- `two_way_case.rml.rs`
- `menu_features_case.rml.rs`
- `status_bar_case.rml.rs` 中 `kind = "case"` 的贡献

注意：`status_bar_case.rml.rs` 中 `kind = "status"` 的贡献保持 `host_id = "demo.shell"`。

### Phase 6：清理

**移除**：
- `demo/src/shell/main_window.rml.rs` 中 `MainWindowHost` struct 及相关代码
- `demo/src/shell/main_window.rml.rs` 中 `case_tree_items()` 方法
- `demo/src/shell/activity_panel.rml.rs` 中 `#[contribute]` 属性
- `DemoShellHost` global 保留（ActivityPanel 仍需通过它调 `MainWindow::open_case`）

**保留**：
- `shell_chrome.rs` 中 `map_case_tree_items`、`map_menu_items`、`map_status_items`（投影函数不变）
- `ContributedEntry` struct（不变）
- `DemoShellHost(WeakEntity<MainWindow>)` global（ActivityPanel 用它调 `open_case`）

***

## 文件影响矩阵

| 文件 | 变更类型 | 说明 |
| --- | --- | --- |
| `crates/core/src/contribution.rs` | 新增方法 | `IContributionRegistry::take_pending` |
| `crates/app/src/contribution/registry.rs` | 新增 impl | `take_pending` 实现 |
| `demo/src/shell/main_window.rml.rs` | 重写 | 移除 `MainWindowHost`，直接 impl，ActivityBar 接线 |
| `demo/src/shell/activity_panel.rml.rs` | 重写 | 移除 `#[contribute]`，直接 impl IContributionHost + Render |
| `demo/src/cases/*.rml.rs`（10 文件） | 改属性 | `host_id = "demo.activity"` |
| `demo/src/shell/shell_chrome.rs` | 不变 | 投影函数保持 |
| `demo/src/shell/mod.rs` | 可能调整 | 导出 `ActivityPanelEntityAdapter`（如放在 activity_panel 模块） |

***

## 假设与决策

1. **`take_pending` 仅支持一次性取出** —— Entity host 在 `on_loaded` 调用一次，后续动态 `register` 入 pending 但不会被取出。对 demo 足够（所有贡献在启动期静态注册）。
2. **ActivityPanel 不再是视觉贡献** —— 移除 `#[contribute]`，由 MainWindow 直接创建 Entity。ActivityBar 面板内容通过 `ActivityPanelEntityAdapter` 桥接。
3. **两个 Entity host 都用 channel + spawn 模式** —— `ObservableVec::push` 发 `flume` 信号，spawn 调 `cx.notify()` 重渲。
4. **`DemoShellHost` global 保留** —— ActivityPanel 的 `on_case_activate` 仍需通过它调 `MainWindow::open_case`。
5. **ActivityBar observe ActivityPanel** —— ActivityPanel 重渲时触发 ActivityBar 重渲，保证面板内容同步。

***

## 验证步骤

1. `cargo build -p rust-rml-core` —— `take_pending` trait 方法编译通过
2. `cargo build -p rust-rml-app` —— registry impl 编译通过
3. `cargo build -p rust-rml-demo` —— 双 host 架构编译通过
4. `cargo run -p rust-rml-demo` —— 验证：
   - 菜单项正常显示（File/View/Help）
   - 状态栏显示 `status.ready`
   - 案例树显示所有 case 贡献
   - 点击案例树节点打开对应 tab
   - ActivityBar 面板切换正常
5. `grep -r "MainWindowHost" demo/` —— 确认无残留引用
6. `grep -r "host_id = \"demo.shell\"" demo/src/cases/` —— 确认 case 贡献已改为 `demo.activity`
