# 框架自动化 Host 架构计划：视觉贡献实体缓存 + Host 生命周期自动化

## 摘要

本计划解决用户核心诉求：**业务代码只需声明结构体 + 业务方法，框架自动处理所有接线（channel、spawn、take\_pending、i18n、实体缓存、适配器桥接）**。

**两个根本问题：**

1. **ActivityPanel 双重角色矛盾** —— ActivityPanel 既是视觉贡献（为 MainWindow 贡献活动栏），又是 Host（接收案例贡献）。当前 `#[contribute]` + `#[component]` 生成的 `IVisualContribution::render` 每次调用 `cx.new(|_| Self::default())` 创建全新 Entity，丢失所有状态（entries、tree\_state）。这是之前被迫移除 `#[contribute]` 的根因。
2. **业务代码臃肿** —— 每个 Host 需手写 \~15 行 channel+spawn+take\_pending 样板、手动 `observe_global::<I18nState>`、30+ 行 `ActivityPanelEntityAdapter` 桥接代码。框架该干的活一件没干好。

**核心解法（两层）：**

**第一层：Entity 缓存（根因修复）** —— `IVisualContribution::render` 不再每次 `cx.new(|_| Self::default())`，改为从框架实体缓存复用 Entity。这是让 ActivityPanel 双重角色可行的根本：视觉贡献 Entity 持久化，`entries`/`tree_state` 不丢。MainWindow 也因此可直接 `impl IContributionHost`（障碍从来不是 trait impl，而是视觉贡献丢状态）。

**第二层：宏自动化（样板消除）** —— 在 Entity 缓存之上，扩展 `#[contributehost]` 宏自动生成 `IContributionHost` + `ILifecycle`（含 channel/spawn/take\_pending/i18n observe），业务代码只写 `IHostEntity` 钩子。

| 问题                            | 方案                                                                                      | 层次       |
| ----------------------------- | --------------------------------------------------------------------------------------- | -------- |
| 视觉贡献 Entity 丢失状态              | 框架透明实体缓存（`TypeId → WeakEntity<T>`），`IVisualContribution::render` 复用                     | 第一层（根因）  |
| ActivityPanelEntityAdapter 桥接 | 框架通用 `VisualActivityPanel` + `build_activity_panels`                                    | 第一层      |
| Host 生命周期样板                   | `#[contributehost]` 自动注入 `entries`/`i18n_version` + 生成 `IContributionHost`/`ILifecycle` | 第二层（自动化） |
| i18n 手动更新                     | 宏生成 `on_locale_changed` 自动 bump `i18n_version` + 调 `IHostEntity::on_locale_changed`     | 第二层      |

> **用户决策（2026-07-02）**：选择"Entity 缓存 + 宏自动化"完整方案。Entity 缓存是根因修复，宏自动化是消除 host 样板的额外自动化。两者均在本次实施。

***

## 目标开发体验

**ActivityPanel（双重角色：视觉贡献 + Host）：**

```rust
#[contribute(host_id="demo.shell", id="samples", name="shell.samples", icon=IconName::BookOpen, kind="activity", order=0)]
#[contributehost(id="demo.activity")]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    // entries: ObservableVec<ContributionEntry>  ← 宏自动注入
    // i18n_version: u32                          ← 宏自动注入
}

// 业务代码仅写：Host 钩子 + 业务方法
impl IHostEntity for ActivityPanel {
    fn host_on_loaded(&mut self, _window, cx) { self.refresh_tree(cx); }
    fn on_locale_changed(&mut self, cx) { self.refresh_tree(cx); }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx) { ... }
    #[command]
    pub fn on_case_activate(&mut self, item_id, cx) { ... }
}
```

**MainWindow（Host + 视觉消费者）：**

```rust
#[window]
#[contributehost(id="demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    // entries, i18n_version ← 宏自动注入
}

impl IHostEntity for MainWindow {
    fn host_on_loaded(&mut self, _window, cx) {
        // 仅业务逻辑：welcome tab、menu_commands、ActivityBar 创建
        let panels = rml_ui::build_activity_panels(&self.entries);
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));
        // observe ActivityPanel entity（1 行）
        let panel = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel, |_, _, cx| cx.notify()).detach();
    }
}
```

**无需手写：** `impl IContributionHost`、`impl ILifecycle`、channel+spawn、take\_pending、`observe_global::<I18nState>`、`ActivityPanelEntityAdapter`、`i18n_version` bump。

***

## 当前状态分析

### 1. `IVisualContribution::render` 创建全新 Entity（根因）

文件：[contribute.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs#L254-L267)

```rust
impl IVisualContribution for #struct_name {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = cx.new(|_| Self::default());  // ← 每次调用创建全新 Entity
        entity.update(cx, |this, ctx| {
            this.render(window, ctx).into_any_element()
        })
    }
}
```

每次 `render` 创建新 Entity → `on_loaded` 重复执行、`entries`/`tree_state` 丢失。这使得 ActivityPanel 无法同时作为视觉贡献和 Host。

### 2. `#[contributehost]` 仅生成 const ID + 断言

文件：[contributehost.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contributehost.rs#L91-L104)

当前宏仅生成 `pub const ID` + 编译期断言。不注入字段、不生成 `IContributionHost` impl、不生成 `ILifecycle`。所有 host 逻辑需手写。

### 3. Host `on_loaded` 样板代码

文件：[activity\_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/activity_panel.rml.rs#L40-L69)（15 行样板）

每个 Host 的 `on_loaded` 必须手写：

* `flume::unbounded::<()>()` channel 创建

* `ObservableVec::with_notifier(tx)` 替换默认实例

* `cx.spawn(async move |this, cx| { while rx.recv_async().await.is_ok() { ... } })` 后台任务

* `cx.get_contribution_registry().take_pending(Self::ID)` + 循环 `self.add(c, o)`

* `cx.observe_global::<I18nState>(|this, cx| { this.refresh_tree(cx); cx.notify(); })` i18n 监听

### 4. `ActivityPanelEntityAdapter` 桥接代码（30+ 行）

文件：[main\_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L289-L337)

`ActivityBar` 需要 `Arc<dyn IActivityPanel>`，但 ActivityPanel 是 Entity。当前手写 adapter 包装 `Entity<ActivityPanel>` 为 `IActivityPanel`，每次 `panel()` 调用 `entity.update(cx, |this, cx| this.render(...))`。

### 5. i18n 手动更新

MainWindow 需手动维护 `i18n_version: u32` 字段，在 `apply_switch_en` 和 `on_loaded` 中手动 bump。ActivityPanel 需手动 `observe_global::<I18nState>` 调用 `refresh_tree`。

***

## 架构设计

### 组件 A：框架视觉贡献实体缓存

**新文件**：`crates/app/src/contribution/entity_cache.rs`

```rust
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};
use gpui::App;

type CacheMap = HashMap<TypeId, Box<dyn Any + Send + Sync>>;
static CACHE: OnceLock<RwLock<CacheMap>> = OnceLock::new();

fn cache() -> &'static RwLock<CacheMap> {
    CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// 获取或创建视觉贡献的缓存 Entity。
/// 首次调用创建 Entity 并缓存 WeakEntity<T>；后续调用 upgrade 复用。
/// Entity 的 on_loaded 由自动生成的 Render::render 经 __rml_loaded 标志触发。
pub fn get_or_create_entity<T: 'static + Send + Sync + Default>(
    cx: &mut App,
) -> gpui::Entity<T> {
    let type_id = TypeId::of::<T>();
    {
        let cache = cache().read().unwrap();
        if let Some(entry) = cache.get(&type_id) {
            if let Some(weak) = entry.downcast_ref::<gpui::WeakEntity<T>>() {
                if let Some(entity) = weak.upgrade() {
                    return entity;
                }
            }
        }
    }
    let entity = cx.new(|_| T::default());
    let weak = entity.downgrade();
    cache().write().unwrap().insert(type_id, Box::new(weak));
    entity
}

/// 获取已缓存的视觉贡献 Entity（不创建）。用于 observe。
pub fn visual_entity<T: 'static + Send + Sync + Default>(
    cx: &mut App,
) -> gpui::Entity<T> {
    get_or_create_entity::<T>(cx)
}
```

**关键约束**：`WeakEntity<T>: Send + Sync` 当且仅当 `T: Send + Sync`。所有视觉贡献（含 `#[component]` 注入字段）均满足此约束（`ObservableVec` 用 `RwLock`，`Option<Entity<T>>` 满足 `Send + Sync`）。

### 组件 B：`IHostEntity` Trait

**修改文件**：`crates/core/src/contribution.rs`

```rust
/// Host Entity 钩子：业务代码实现此 trait 提供 host 特有逻辑。
/// 框架生成的 ILifecycle::on_loaded 在完成标准 setup 后调用 host_on_loaded；
/// i18n 变更时调用 on_locale_changed（默认 cx.notify() + bump i18n_version）。
pub trait IHostEntity {
    /// 框架标准 setup（channel/spawn/take_pending/i18n observe）完成后调用。
    /// 业务代码在此执行 host 特有初始化（refresh_tree、创建 ActivityBar 等）。
    fn host_on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>)
    where
        Self: Sized,
    {}

    /// locale 变更时调用。默认实现为空（框架已在外层 bump i18n_version + cx.notify）。
    /// 业务代码可覆写以执行额外逻辑（如 refresh_tree）。
    fn on_locale_changed(&mut self, _cx: &mut gpui::Context<Self>)
    where
        Self: Sized,
    {}
}
```

**设计决策**：`IHostEntity` 与 `ILifecycle` 分离。`ILifecycle` 由 `#[contributehost]` 宏生成（框架标准 setup），`IHostEntity` 由业务代码实现（host 特有逻辑）。Host 不再手写 `impl ILifecycle`。

### 组件 C：`ContributionEntry` 框架类型

**修改文件**：`crates/core/src/contribution.rs`

```rust
/// 贡献条目：host 受理的贡献 + 注册选项。
/// 从 demo 的 ContributedEntry 移入框架，供 #[contributehost] 宏注入 entries 字段使用。
pub struct ContributionEntry {
    pub contribution: Arc<dyn IContribution>,
    pub options: ContributionOptions,
}
```

### 组件 D：`VisualActivityPanel` + `build_activity_panels`

**修改文件**：`crates/ui/src/components/activity_bar.rs`

```rust
/// 通用视觉贡献 → IActivityPanel 适配器。
/// 包装 Arc<dyn IVisualContribution>，id/icon/title 从 IContribution 元数据提取，
/// panel() 委托给 IVisualContribution::render（经实体缓存复用 Entity）。
pub struct VisualActivityPanel {
    visual: Arc<dyn IVisualContribution>,
    id: SharedString,
    icon_name: IconName,
    title: SharedString,
}

impl VisualActivityPanel {
    pub fn new(visual: Arc<dyn IVisualContribution>) -> Option<Self> {
        let id: SharedString = visual.id().to_string().into();
        let title = visual.name();
        let icon_name = visual.icon()
            .and_then(|s| parse_icon_name(&s))
            .unwrap_or(IconName::PanelLeft);
        Some(Self { visual, id, icon_name, title })
    }
}

impl IActivityPanel for VisualActivityPanel {
    fn id(&self) -> SharedString { self.id.clone() }
    fn icon(&self) -> IconName { self.icon_name }
    fn title(&self) -> SharedString { self.title.clone() }
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(self.visual.render(window, cx))
    }
}

/// 从 host entries 构建 ActivityPanels（kind="activity" 的视觉贡献）。
pub fn build_activity_panels(entries: &[ContributionEntry]) -> ActivityPanels {
    entries.iter()
        .filter(|e| e.options.effective_slot() == Some("activity"))
        .filter_map(|e| rml_app::contribution::extract_visual(&e.contribution))
        .filter_map(VisualActivityPanel::new)
        .map(|p| Arc::new(p) as Arc<dyn IActivityPanel>)
        .collect()
}

/// 解析图标名字符串 → IconName（IconName 需实现 FromStr 或此处用 match）
fn parse_icon_name(s: &str) -> Option<IconName> {
    // IconName::from_str(s).ok()  —— 若 gpui-component 提供 FromStr
    // 否则用 match 映射常用图标
    ...
}
```

### 组件 E：`#[contribute]` 宏修改

**修改文件**：`crates/macros/src/contribute.rs`（第 254-267 行）

将 `IVisualContribution::render` 从 `cx.new(|_| Self::default())` 改为使用实体缓存：

```rust
let visual_impl = if use_component_visual {
    quote! {
        impl rml_core::contribution::IVisualContribution for #struct_name {
            fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
                let entity = rml_app::contribution::get_or_create_entity::<#struct_name>(cx);
                entity.update(cx, |this, ctx| {
                    this.render(window, ctx).into_any_element()
                })
            }
        }
    }
} else {
    quote! {}
};
```

### 组件 F：`#[contributehost]` 宏扩展

**修改文件**：`crates/macros/src/contributehost.rs`

**新增功能**：

1. 向 struct 注入 `entries: ObservableVec<rml_core::contribution::ContributionEntry>` 字段
2. 向 struct 注入 `i18n_version: u32` 字段
3. 生成 `impl IContributionHost`（add/remove 操作 entries）
4. 生成 `impl ILifecycle`（框架标准 setup + 委托 IHostEntity 钩子）

```rust
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    // ... 解析 args、items、struct_name ...

    quote! {
        #(#items)*  // struct（含注入字段）+ #[component] 生成的 impls

        impl #struct_name {
            pub const ID: &'static str = #id;
        }

        // 编译期断言：目标类型必须实现 IHostEntity（或使用 trait 默认实现）
        const _: () = {
            fn assert_host_entity<T: rml_core::contribution::IHostEntity>() {}
            fn check() { assert_host_entity::<#struct_name>(); }
        };

        // 自动生成 IContributionHost
        impl rml_core::contribution::IContributionHost for #struct_name {
            fn id(&self) -> &'static str { Self::ID }
            fn add(&self, contribution: std::sync::Arc<dyn rml_core::contribution::IContribution>, options: rml_core::contribution::ContributionOptions) {
                self.entries.push(rml_core::contribution::ContributionEntry { contribution, options });
            }
            fn remove(&self, contribution_id: &str) {
                self.entries.retain(|e| e.contribution.id() != contribution_id);
            }
        }

        // 自动生成 ILifecycle：框架标准 setup + IHostEntity 钩子委托
        impl rml_core::lifecycle::ILifecycle for #struct_name {
            fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
                // 1. channel + spawn：ObservableVec 变更 → cx.notify()
                let (tx, rx) = flume::unbounded::<()>();
                self.entries = rml_core::observable::ObservableVec::with_notifier(tx);
                cx.spawn(async move |this, cx| {
                    while rx.recv_async().await.is_ok() {
                        let _ = this.update(cx, |_, cx| cx.notify());
                    }
                }).detach();

                // 2. take_pending → self.add 受理
                let pending = cx.get_contribution_registry().take_pending(Self::ID);
                for (c, o) in pending { self.add(c, o); }

                // 3. i18n observe：locale 变更 → bump i18n_version + on_locale_changed + cx.notify
                cx.observe_global::<rml_core::i18n::I18nState>(|this, cx| {
                    this.i18n_version = this.i18n_version.wrapping_add(1);
                    rml_core::contribution::IHostEntity::on_locale_changed(this, cx);
                    cx.notify();
                }).detach();

                // 4. 委托 IHostEntity 钩子（业务代码的 host 特有逻辑）
                rml_core::contribution::IHostEntity::host_on_loaded(self, _window, cx);
            }
        }
    }
}
```

**字段注入**：在找到 struct 后，向 `Fields::Named` 追加两个字段：

```rust
let mut item = struct_item.clone();
if let syn::Fields::Named(named) = &mut item.fields {
    named.named.push(parse_quote! {
        #[allow(non_snake_case, dead_code)]
        entries: rml_core::observable::ObservableVec<rml_core::contribution::ContributionEntry>
    });
    named.named.push(parse_quote! {
        #[allow(non_snake_case, dead_code)]
        i18n_version: u32
    });
}
```

**宏顺序约束**：`#[contributehost]` 必须在 `#[component]`/`#[window]` 之下（source 中），因为 `#[component]`/`#[window]` 先展开（内层先），`#[contributehost]` 后展开看到已注入 `__rml_*` 字段的结构体，再追加 `entries` + `i18n_version`。`#[contribute]` 在最外层最后展开。

### 组件 G：Scanner i18n 检测扩展

**修改文件**：`crates/engine/src/build/scanner.rs`（第 184 行）

当前检查：

```rust
if visitor.uses_i18n && meta.observable_fields.contains(&"i18n_version".to_string())
```

改为：

```rust
if visitor.uses_i18n && (meta.observable_fields.contains(&"i18n_version".to_string()) || meta.is_contributehost)
```

**原因**：`#[contributehost]` 宏注入的 `i18n_version` 字段不在源码中，scanner 无法从源码检测到。但 scanner 已检测 `is_contributehost`（第 105 行），可作为替代条件。生成的 `#[computed]` 包装器引用 `self.i18n_version`，编译期该字段已由宏注入。

***

## 实施阶段

### Phase 1：框架基础设施（core + app + ui）

**文件 1**：`crates/core/src/contribution.rs`

* 新增 `ContributionEntry` struct（从 demo 移入）

* 新增 `IHostEntity` trait（`host_on_loaded` + `on_locale_changed`，默认空实现）

* 导出至 `crates/core/src/prelude.rs` 和 `lib.rs`

**文件 2**：`crates/app/src/contribution/entity_cache.rs`（新建）

* 实现 `get_or_create_entity<T>(cx) -> Entity<T>` + `visual_entity<T>(cx) -> Entity<T>`

* `static CACHE: OnceLock<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>`

**文件 3**：`crates/app/src/contribution/mod.rs`

* 新增 `mod entity_cache;`

* 导出 `get_or_create_entity`、`visual_entity`

**文件 4**：`crates/ui/src/components/activity_bar.rs`

* 新增 `VisualActivityPanel` struct + `impl IActivityPanel`

* 新增 `build_activity_panels(entries) -> ActivityPanels`

* 新增 `parse_icon_name(s) -> Option<IconName>`（若 `IconName` 无 `FromStr`，用 match 映射 demo 用到的图标）

### Phase 2：宏改造

**文件 5**：`crates/macros/src/contribute.rs`（第 254-267 行）

* `IVisualContribution::render` 改用 `rml_app::contribution::get_or_create_entity::<Self>(cx)`

**文件 6**：`crates/macros/src/contributehost.rs`

* 向 struct 注入 `entries` + `i18n_version` 字段

* 生成 `impl IContributionHost`（add/remove 操作 entries）

* 生成 `impl ILifecycle`（channel + spawn + take\_pending + i18n observe + IHostEntity 委托）

* 编译期断言改为 `IHostEntity`（不再断言 `IContributionHost`，因其已自动生成）

**文件 7**：`crates/engine/src/build/scanner.rs`（第 184 行）

* i18n 检测条件增加 `|| meta.is_contributehost`

### Phase 3：Demo 迁移 — ActivityPanel

**文件 8**：`demo/src/shell/activity_panel.rml.rs`（重写）

```rust
use gpui::Window;
use rml::prelude::*;
use rml_core::contribution::IHostEntity;
use rml_ui::TreeState;
use crate::shell::shell_chrome::map_case_tree_items;
use crate::shell::{DemoShellHost, MainWindow};

#[contribute(host_id="demo.shell", id="samples", name="shell.samples", icon=IconName::BookOpen, kind="activity", order=0)]
#[contributehost(id="demo.activity")]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    // entries, i18n_version ← #[contributehost] 自动注入
}

// 无 impl IContributionHost —— 宏自动生成
// 无 impl ILifecycle —— 宏自动生成

impl IHostEntity for ActivityPanel {
    fn host_on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
    }
    fn on_locale_changed(&mut self, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let entries = self.entries.read();
        let items = map_case_tree_items(&entries);
        self.set_tree_items(items, cx);
    }
    fn set_tree_items(&mut self, items: Vec<rml_ui::TreeItem>, cx: &mut Context<Self>) {
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| { s.set_items(items, cx); });
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }
    #[command]
    pub fn on_case_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
            host.update(cx, |main, cx| { main.open_case(item_id.to_string(), cx); });
        }
    }
}
```

### Phase 4：Demo 迁移 — MainWindow

**文件 9**：`demo/src/shell/main_window.rml.rs`（重写）

```rust
use std::sync::Arc;
use gpui::{BorrowAppContext, Global, IntoElement, Window};
use gpui_component::IconName;
use rml::prelude::*;
use rml_core::contribution::IHostEntity;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityBar, MenuItems, StatusBarItems, TabItem, build_activity_panels};
use crate::cases::{self, OpenTab};
use crate::shell::activity_panel::ActivityPanel;
use crate::shell::shell_chrome::{map_menu_items, map_status_items};

pub struct DemoShellHost(pub WeakEntity<MainWindow>);
impl Global for DemoShellHost {}

#[window]
#[contributehost(id="demo.shell")]
#[derive(Default)]
pub struct MainWindow {
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    status_items: StatusBarItems,
    menu_items: MenuItems,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
    slot_left_size: gpui::Pixels,
    // entries, i18n_version ← #[contributehost] 自动注入
}

// 无 impl IContributionHost —— 宏自动生成
// 无 impl ILifecycle —— 宏自动生成

impl IHostEntity for MainWindow {
    fn host_on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab { id: "welcome".to_string(), title: cx.t("shell.welcome").to_string() });
            self.selected_tab = 0;
            self.active_case_id = "welcome".to_string();
        }
        self.show_chrome = true;

        let shell_weak = cx.weak_entity();
        cx.set_global(DemoShellHost(shell_weak));

        // menu_commands 初始化（同当前代码）
        self.menu_commands.insert("menu.file.new".to_string(), Arc::new(RelayCommand::new(cx, |this, cx| { this.open_case("welcome".to_string(), cx); })));
        // ... 其他 menu_commands ...

        // 刷新 shell chrome
        self.refresh_shell_chrome(cx);

        // 构建 ActivityBar（从 entries 中的 activity 视觉贡献）
        let panels = rml_ui::build_activity_panels(&self.entries);
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));

        // observe ActivityPanel entity → ActivityBar 重渲
        let panel_entity = rml_app::contribution::visual_entity::<ActivityPanel>(cx);
        cx.observe(&panel_entity, |_, _, cx| cx.notify()).detach();

        // 激活首项
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.activate_first(cx));
        }
        self.slot_left_size = gpui::px(260.);

        // observe ActivityBar → slot_left_size
        if let Some(bar) = &self.activity_bar {
            cx.observe(bar, |this, bar, cx| {
                let collapsed = bar.read(cx).active_id().is_none();
                this.slot_left_size = if collapsed { gpui::px(48.) } else { gpui::px(260.) };
                cx.notify();
            }).detach();
        }
    }
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self, cx: &mut Context<Self>) {
        let entries = self.entries.read();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries, &self.menu_commands);
    }

    pub fn active_case_view(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        use rml_app::contribution::extract_visual;
        let entries = self.entries.read();
        if let Some(entry) = entries.iter().find(|e| e.contribution.id() == self.active_case_id) {
            if let Some(visual) = extract_visual(&entry.contribution) {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }

    #[computed]
    pub fn tab_bar_items(&self) -> Vec<TabItem> {
        let _ = self.i18n_version;
        self.open_tabs.iter().map(|tab| TabItem::new(tab.title.as_str())).collect()
    }

    #[command]
    pub fn on_chrome_toggle(&mut self, cx: &mut Context<Self>) { self.show_chrome = !self.show_chrome; }

    #[command]
    pub fn open_case(&mut self, case_id: String, cx: &mut Context<Self>) { ... }

    #[command]
    pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) { ... }

    fn apply_toggle_theme(&mut self, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" { "light" } else { "dark" };
        cx.set_theme(next);
        cx.notify();
    }

    fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
        let mut tabs = std::mem::take(&mut self.open_tabs);
        tabs.iter_mut().for_each(|tab| { tab.title = cx.t(cases::case_title_key(&tab.id)).to_string(); });
        self.open_tabs = tabs;
        self.refresh_shell_chrome(cx);
        cx.notify();
    }
}
```

**移除内容**：

* `ActivityPanelEntityAdapter` struct 及 impl（30+ 行）

* 手动 `impl IContributionHost for MainWindow`

* 手动 `impl ILifecycle for MainWindow`（channel/spawn/take\_pending/observe\_global 样板）

* `entries` 字段声明（宏注入）

* `i18n_version` 字段声明 + 手动 bump（宏注入 + 自动 bump）

* `activity_panel` 字段（不再需要直接持有 Entity）

### Phase 5：shell\_chrome 类型迁移

**文件 10**：`demo/src/shell/shell_chrome.rs`

* 移除 `ContributedEntry` struct 定义

* 改用 `use rml_core::contribution::ContributionEntry;`

* 所有 `&[ContributedEntry]` → `&[ContributionEntry]`

* 投影函数内部逻辑不变

### Phase 6：Case 贡献路由变更

**文件**：`demo/src/cases/*.rml.rs`（10 个案例文件）

将 `host_id = "demo.shell"` 改为 `host_id = "demo.activity"`：

* `button_case.rml.rs`

* `counter_case.rml.rs`

* `i18n_case.rml.rs`

* `menu_context_case.rml.rs`

* `menu_custom_case.rml.rs`

* `menu_dropdown_case.rml.rs`

* `menu_editor_case.rml.rs`

* `two_way_case.rml.rs`

* `menu_features_case.rml.rs`

* `status_bar_case.rml.rs` 中 `kind = "case"` 的贡献（`StatusReady` 的 `kind = "status"` 保持 `demo.shell`）

### Phase 7：编译与验证

1. `cargo build -p rust-rml-core` —— `ContributionEntry` + `IHostEntity` 编译通过
2. `cargo build -p rust-rml-app` —— 实体缓存编译通过
3. `cargo build -p rust-rml-ui` —— `VisualActivityPanel` + `build_activity_panels` 编译通过
4. `cargo build -p rust-rml-macros` —— 宏改造编译通过
5. `cargo build -p rust-rml-engine` —— scanner 改造编译通过
6. `cargo build -p rust-rml-demo` —— 全部迁移编译通过
7. `cargo run -p rust-rml-demo` —— 运行验证：

   * 菜单项正常显示（File/View/Help）

   * 状态栏显示 `status.ready`

   * 案例树显示所有 case 贡献

   * 点击案例树节点打开对应 tab

   * ActivityBar 面板切换正常

   * 切换语言 → 案例树标签刷新 + tab 标题刷新

   * 切换主题 → 界面主题切换

***

## 假设与决策

1. **实体缓存为进程级静态表** —— `OnceLock<RwLock<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>`。简单且用户无感。Entity 弱引用在窗口关闭后自动释放（upgrade 返回 None 时重建）。

2. **`#[contributehost]`** **生成** **`impl ILifecycle`** —— Host 不再手写 `impl ILifecycle`，改用 `impl IHostEntity`。`#[component]`/`#[window]` 不生成 `impl ILifecycle`，无冲突。

3. **字段注入命名约束** —— 宏注入 `entries` + `i18n_version` 字段。Host 结构体不得手动声明同名字段。`#[derive(Default)]` 在所有属性宏之后展开，包含注入字段。

4. **宏展开顺序** —— `#[component]`/`#[window]`（底层先展开）→ `#[contributehost]`（注入 host 字段）→ `#[contribute]`（最外层最后展开）。`#[contributehost]` 解析 `Vec<Item>` 可处理 `#[component]` 展开后的多 item 输出。

5. **Scanner i18n 检测** —— scanner 已检测 `is_contributehost`（第 105 行）。利用此标志替代 `observable_fields.contains("i18n_version")` 检测，因为宏注入的 `i18n_version` 不在源码中。

6. **ActivityPanel 双重角色** —— `#[contribute(host_id="demo.shell", kind="activity")]` + `#[contributehost(id="demo.activity")]` + `#[component]`。视觉贡献的 Entity 由框架缓存，Host 的 entries 由宏注入。两个角色互不干扰。

7. **`VisualActivityPanel`** **图标解析** —— `IContribution::icon()` 返回 `Option<SharedString>`（字符串形式的图标名）。`VisualActivityPanel` 需将字符串解析回 `IconName`。若 `gpui-component` 的 `IconName` 未实现 `FromStr`，用 match 映射 demo 用到的图标子集。

8. **`DemoShellHost`** **global 保留** —— ActivityPanel 的 `on_case_activate` 仍需通过它调 `MainWindow::open_case`。此为 demo 特定通信，不属于框架职责。

9. **`take_pending`** **一次性语义** —— Entity host 在 `on_loaded` 调用一次，后续动态 `register` 入 pending 但不会被取出。对 demo 足够（所有贡献在启动期静态注册）。

10. **`activity_panel`** **字段移除** —— MainWindow 不再直接持有 `Entity<ActivityPanel>`。需要观察 ActivityPanel 变化时，通过 `visual_entity::<ActivityPanel>(cx)` 从缓存获取 Entity handle。

***

## 验证步骤

### 编译验证

```bash
cargo build -p rust-rml-core   # ContributionEntry + IHostEntity
cargo build -p rust-rml-app     # entity_cache
cargo build -p rust-rml-ui      # VisualActivityPanel + build_activity_panels
cargo build -p rust-rml-macros  # contribute + contributehost 宏改造
cargo build -p rust-rml-engine  # scanner i18n 检测
cargo build -p rust-rml-demo     # 全部迁移
```

### 运行验证

```bash
cargo run -p rust-rml-demo
```

验证清单：

* [ ] 菜单项正常显示（File/View/Help）

* [ ] 状态栏显示 `status.ready`

* [ ] 案例树显示所有 case 贡献（10 个案例节点）

* [ ] 点击案例树节点打开对应 tab

* [ ] ActivityBar 面板切换正常（点击图标展开/收起）

* [ ] 切换语言（中文→英文）→ 案例树标签刷新 + tab 标题刷新

* [ ] 切换主题（暗色↔亮色）→ 界面主题切换

* [ ] ActivityPanel Entity 状态持久（切换 tab 后回来树状态保留）

### 残留检查

* `grep -r "ActivityPanelEntityAdapter" demo/` —— 确认已移除

* `grep -r "impl IContributionHost" demo/` —— 确认无手动实现（宏自动生成）

* `grep -r "impl ILifecycle" demo/` —— 确认 Host 无手动 ILifecycle（宏自动生成）

* `grep -r "host_id = \"demo.shell\"" demo/src/cases/` —— 确认 case 贡献已改为 `demo.activity`

* `grep -r "flume::unbounded\|take_pending\|observe_global::<I18nState>" demo/src/shell/` —— 确认样板代码已移除

### 实体缓存验证

* 切换 tab 离开 ActivityPanel 再回来 → `entries` 和 `tree_state` 保留（Entity 未重建）

* 多次调用 `active_case_view` → 同一 case 的 Entity 状态保留（不重复 `on_loaded`）

