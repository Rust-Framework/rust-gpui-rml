# 贡献点系统重设计计划

> **目标**：移除开发妥协产物（`IHostEntity`/`ContributionEntry`/`VisualExtractor`/`take_pending`/`i18n_version` 注入），还原设计意图：`IContribution/IVisualContribution → IContributionRegistry → IContributionHost`，registry 仅按 `host_id` 路由 `add/remove`，不存储贡献内容。

***

## 1. 设计原则

| 原则                                    | 含义                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| **Registry 是中介，不存内容**                 | `ContributionRegistry` 内部只存 `HashMap<String, Arc<dyn IContributionHost>>`，按 `host_id` 路由。无 pending 队列。                        |
| **Host 直接实现** **`IContributionHost`** | 不再有 `IHostEntity` 钩子。`#[contributehost]` 宏只生成 `pub const ID` + `__rml_install_host` + 编译期断言 `T: IContributionHost`。           |
| **Host 业务自决存储**                       | 框架不注入 `entries` 字段。Host 自行决定 `RwLock<Vec<...>>`/`ObservableVec<...>` 等存储结构。                                                   |
| **视觉贡献直达 host**                       | 视觉贡献经 `register_visual` 路由到 `host.add_visual(Arc<dyn IVisualContribution>, ...)`。无需 `VisualExtractor` 转换。                     |
| **i18n/theme 应用级处理**                  | 宏不注入 `i18n_version`，不生成 `observe_global`。`set_i18n`/`set_theme` 已有 `refresh_windows()`；computed cache 失效另议。                   |
| **Entity host 用 handle 桥接**           | `Entity<T>` 不能直接 `Arc<dyn IContributionHost>`。框架提供 `EntityHostHandle<T>` + flume channel，Entity 在 `on_loaded`/render 中 drain。 |

***

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│  contribute 宏生成 __rml_register_<Type>(cx)                │
│  └─ visual: register_visual(host_id, Arc<T>, opts)          │
│  └─ 非视觉: register(host_id, Arc<T>, opts)                  │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  IContributionRegistry（中介，仅存 host）                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ hosts: HashMap<host_id, Arc<dyn IContributionHost>> │    │
│  └─────────────────────────────────────────────────────┘    │
│  register(host_id, c, o)    → host.add(c, o)                │
│  register_visual(host_id, v, o) → host.add_visual(v, o)     │
│  unregister(host_id, id)    → host.remove(id)               │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
┌─────────────────────────────────────────────────────────────┐
│  IContributionHost（host 直接实现）                          │
│  - MainWindow: add_visual 收 ActivityPanel/case 视觉贡献     │
│  - ActivityPanel: add_visual 收 case 视觉贡献 → 构建 tree     │
│  - DbProviderHost: add 收 IDbProvider 能力贡献（非视觉）      │
└─────────────────────────────────────────────────────────────┘

Entity host 时序：
  on_loaded → __rml_install_host(entity, cx)
            → 创建 EntityHostHandle + flume::Sender
            → registry.add_host(Arc::new(handle))
            → register_rml_contributions_for(cx, Self::ID)
              └─ 调用所有该 host_id 的 __rml_register_*
                └─ registry.register_visual → handle.add_visual → tx.send(HostOp)
            → 返回 flume::Receiver
  Entity 持有 Receiver，在 on_loaded 末尾 drain_host_ops(self, cx)
            → 调用自身 IContributionHost::add_visual 等 &self 方法（用 RwLock 改存储）
            → cx.notify()
```

***

## 3. 核心契约重设计 — `crates/core/src/contribution.rs`

### 3.1 保留

* `ContributionOptions`（纯数据 builder，不变）

* `IContribution`（保留 `Any` supertrait，供 TypeId 与未来潜在 downcast）

* `IVisualContribution: IContribution`（`render` 签名不变）

### 3.2 修改 `IContributionHost`

```rust
pub trait IContributionHost: Send + Sync + 'static {
    fn id(&self) -> &'static str;

    /// 受理能力贡献（非视觉）。默认空实现，host 按需 override。
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: ContributionOptions) {}

    /// 受理视觉贡献。默认空实现，视觉 host override。
    fn add_visual(&self, _contribution: Arc<dyn IVisualContribution>, _options: ContributionOptions) {}

    /// 移除贡献。默认空实现。
    fn remove(&self, _contribution_id: &str) {}
}
```

**理由**：host 业务自决接受哪种贡献。MainWindow/ActivityPanel override `add_visual`；DbProviderHost override `add`。默认空实现避免强制 host 实现所有方法。

### 3.3 修改 `IContributionRegistry`

```rust
pub trait IContributionRegistry: Send + Sync {
    fn add_host(&self, host: Arc<dyn IContributionHost>);
    fn remove_host(&self, host_id: &str);

    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions);
    fn register_visual(&self, host_id: &str, contribution: Arc<dyn IVisualContribution>, options: ContributionOptions);
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
```

**变更**：`add`/`remove` → `add_host`/`remove_host`（语义更清晰）；新增 `register_visual`；**删除** **`take_pending`**。

### 3.4 删除

* `ContributionEntry`（host 业务自决存储结构）

* `IHostEntity`（host 直接实现 `IContributionHost`）

* `VisualExtractor` type alias（视觉贡献直达，无需提取器）

* `take_pending` 方法（无 pending 队列）

***

## 4. Registry 实现 — `crates/app/src/contribution/registry.rs`

```rust
pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, Arc<dyn IContributionHost>>>,
}

impl IContributionRegistry for ContributionRegistry {
    fn add_host(&self, host: Arc<dyn IContributionHost>) {
        let id = host.id().to_string();
        self.hosts.write().unwrap().insert(id, host);
    }

    fn remove_host(&self, host_id: &str) {
        self.hosts.write().unwrap().remove(host_id);
    }

    fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: ContributionOptions) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        } else {
            tracing::warn!(host_id, "host not registered; contribution dropped");
        }
    }

    fn register_visual(&self, host_id: &str, contribution: Arc<dyn IVisualContribution>, options: ContributionOptions) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add_visual(contribution, options);
        } else {
            tracing::warn!(host_id, "host not registered; visual contribution dropped");
        }
    }

    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.remove(contribution_id);
            true
        } else {
            false
        }
    }
}
```

**删除**：

* `VISUAL_EXTRACTORS` 静态表

* `register_visual_extractor` / `extract_visual` 函数

* `pending` HashMap

**时序保证**：host 未注册时 `register` 直接 drop（warn 日志）。Host 必须在 `on_loaded` 中先 `__rml_install_host` 注册自身，再触发 `register_rml_contributions_for`。

***

## 5. Entity Host 桥接 — 新增 `crates/app/src/contribution/host_handle.rs`

`Entity<T>` 不能直接 `Arc<dyn IContributionHost>`（update 需 cx）。提供 handle + channel 桥接：

```rust
use std::sync::Arc;
use gpui::{App, WeakEntity};
use rml_core::contribution::{IContributionHost, IContribution, IVisualContribution, ContributionOptions};
use rml_core::flume;

/// Host 操作队列（Entity host 在 on_loaded/render 中 drain）
pub enum HostOp {
    Add(Arc<dyn IContribution>, ContributionOptions),
    AddVisual(Arc<dyn IVisualContribution>, ContributionOptions),
    Remove(String),
}

/// Entity host 的 IContributionHost 桥接器。
/// 所有方法将操作入 channel，Entity 持有 Receiver 在 update 闭包内 drain。
pub struct EntityHostHandle<T: 'static> {
    id: &'static str,
    weak: WeakEntity<T>,
    tx: flume::Sender<HostOp>,
}

impl<T: 'static> IContributionHost for EntityHostHandle<T> {
    fn id(&self) -> &'static str { self.id }
    fn add(&self, c: Arc<dyn IContribution>, o: ContributionOptions) {
        let _ = self.tx.send(HostOp::Add(c, o));
    }
    fn add_visual(&self, c: Arc<dyn IVisualContribution>, o: ContributionOptions) {
        let _ = self.tx.send(HostOp::AddVisual(c, o));
    }
    fn remove(&self, contribution_id: &str) {
        let _ = self.tx.send(HostOp::Remove(contribution_id.to_string()));
    }
}

/// 由 `#[contributehost]` 宏生成的 `__rml_install_host` 调用。
/// 注册 handle + 触发该 host_id 的所有贡献注册，返回 Receiver 供 Entity drain。
pub fn install_entity_host<T: IContributionHost + 'static>(
    id: &'static str,
    entity: &gpui::Entity<T>,
    cx: &mut App,
) -> flume::Receiver<HostOp> {
    let (tx, rx) = flume::unbounded();
    let handle = EntityHostHandle { id, weak: entity.downgrade(), tx };
    cx.get_contribution_registry().add_host(Arc::new(handle));
    // 触发该 host_id 的所有贡献注册（同步：register_visual → handle.add_visual → tx.send）
    super::global::bootstrap_host_contributions(cx, id);
    rx
}

/// Entity host 在 on_loaded/render 中调用：drain 接收到的操作，分派到自身 IContributionHost 实现。
pub fn drain_host_ops<T: IContributionHost>(rx: &flume::Receiver<HostOp>, host: &T) {
    for op in rx.try_iter() {
        match op {
            HostOp::Add(c, o) => host.add(c, o),
            HostOp::AddVisual(c, o) => host.add_visual(c, o),
            HostOp::Remove(id) => host.remove(&id),
        }
    }
}
```

**关键点**：

* `EntityHostHandle::add_visual` 仅 `tx.send`，不阻塞、不需 cx

* Entity 在 `update` 闭包中调 `drain_host_ops(&rx, self)`，此时 `&self` 可调用用户实现的 `IContributionHost::add_visual`，后者用 `RwLock` 改存储

* drain 后 Entity 调 `cx.notify()` 触发重渲

***

## 6. 宏变更

### 6.1 `#[contributehost]` — `crates/macros/src/contributehost.rs`

**精简为**：

1. 解析 `id` 参数（必填），拒绝 `bindings`/`on_changed`（已废弃）
2. 生成 `pub const ID: &'static str = "..."`
3. 编译期断言 `T: IContributionHost`（不再是 `IHostEntity`）
4. 生成 `pub fn __rml_install_host(this: &Entity<Self>, cx: &mut App) -> flume::Receiver<HostOp>`

```rust
quote! {
    #(#items)*

    impl #struct_name {
        pub const ID: &'static str = #id;

        pub fn __rml_install_host(
            this: &gpui::Entity<Self>,
            cx: &mut gpui::App,
        ) -> rml_core::flume::Receiver<rml_app::contribution::HostOp> {
            rml_app::contribution::install_entity_host(Self::ID, this, cx)
        }
    }

    const _: () = {
        fn assert_host<T: rml_core::contribution::IContributionHost>() {}
        fn check() { assert_host::<#struct_name>(); }
    };
}
```

**删除**：

* 注入 `entries` / `i18n_version` 字段

* 自动生成 `impl IContributionHost`（用户手写）

* 自动生成 `impl ILifecycle`（用户手写，调用 `__rml_install_host` + `drain_host_ops`）

* `IHostEntity` 断言

### 6.2 `#[contribute]` — `crates/macros/src/contribute.rs`

**变更**：

1. 保留 `impl IContribution` 生成
2. 保留 `impl IVisualContribution` 生成（`#[contribute]` + `#[component]` 叠加时）
3. **删除** `#[ctor::ctor]` 视觉提取器注册（整个 `visual_extractor` 块）
4. `__rml_register_<Type>(cx)` 改用 `register_visual` 或 `register`：

```rust
let register_call = if use_component_visual {
    quote! {
        cx.get_contribution_registry().register_visual(
            #host_id,
            std::sync::Arc::new(#struct_name::default()),
            rml_core::contribution::ContributionOptions::new() #slot #parent_id #order #group #align,
        );
    }
} else {
    quote! {
        cx.get_contribution_registry().register(
            #host_id,
            std::sync::Arc::new(#struct_name::default()),
            rml_core::contribution::ContributionOptions::new() #slot #parent_id #order #group #align,
        );
    }
};
```

***

## 7. build.rs 生成器 — `crates/engine/src/build/contribution_generator.rs`

按 `host_id` 分组，生成 `register_rml_contributions_for(cx, host_id)`：

```rust
pub fn generate(contributions: &[ContributionRegistrar], output_dir: &Path) -> Result<(), BuildError> {
    // 按 host_id 分组
    let mut by_host: BTreeMap<String, Vec<&ContributionRegistrar>> = BTreeMap::new();
    for c in contributions {
        by_host.entry(c.host_id.clone()).or_default().push(c);
    }

    let mut body = String::from(
        "// Auto-generated by RML build.rs — do not edit.\n\
         pub fn register_rml_contributions_for(cx: &mut gpui::App, host_id: &str) {\n\
             match host_id {\n",
    );
    for (host_id, regs) in &by_host {
        body.push_str(&format!("        \"{}\" => {{\n", host_id));
        for reg in regs {
            body.push_str(&format!(
                "            {}::__rml_register_{}(cx);\n",
                reg.module_path, reg.register_suffix
            ));
        }
        body.push_str("        }\n");
    }
    body.push_str("        _ => {}\n    }\n}\n\n");

    body.push_str(
        "#[rml_core::ctor::ctor]\n\
         fn __rml_install_contribution_bootstrap() {\n\
             rml_app::contribution::install_contribution_bootstrap(register_rml_contributions_for);\n\
         }\n",
    );
    // ... write to out_path
}
```

`ContributionRegistrar` 增加字段：

```rust
pub struct ContributionRegistrar {
    pub module_path: String,
    pub register_suffix: String,
    pub host_id: String,  // 新增：从 #[contribute(host_id = "...")] 解析
}
```

`parse_contribution_registrars` 解析 `#[contribute(...)]` 的 `host_id` 参数。

***

## 8. App 级 i18n/theme — `crates/app/src/application.rs`

```rust
fn bootstrap_runtime(cx: &mut App) {
    ensure_i18n(cx);
    ensure_theme(cx);
    gpui_component::init(cx);
    gpui_component::Theme::global_mut(cx).font_size = px(14.);
    crate::contribution::ensure_contribution_registry(cx);
    // 删除：bootstrap_contributions(cx) —— 注册由 host 在 on_loaded 中触发
}
```

**i18n/theme 处理**：

* `set_i18n` / `set_theme` 已调用 `cx.refresh_windows()`（见 `i18n.rs:209`、`theme.rs:202`），所有窗口自动重渲

* `#[contributehost]` 不再注入 `i18n_version` 字段或 `observe_global`

* **遗留**：`#[computed]` 方法若依赖 `t_static`，缓存失效需另议（见 §12 遗留问题）

***

## 9. 模块导出 — `crates/app/src/contribution/mod.rs`

```rust
mod entity_cache;
mod global;
mod host_handle;  // 新增
mod registry;

pub use entity_cache::{get_or_create_entity, visual_entity};  // 删除 build_activity_panels
pub use global::{
    bootstrap_host_contributions, ensure_contribution_registry,
    install_contribution_bootstrap, ContributionRegistryExt,
};
pub use host_handle::{drain_host_ops, install_entity_host, EntityHostHandle, HostOp};
pub use registry::ContributionRegistry;

// 删除：extract_visual, register_visual_extractor
```

`global.rs` 变更：

```rust
static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App, &str)>> = Mutex::new(None);

pub fn install_contribution_bootstrap(f: fn(&mut App, &str)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

pub fn bootstrap_host_contributions(cx: &mut App, host_id: &str) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx, host_id);
    }
}

// 删除：bootstrap_contributions(cx)（旧的统一注册）
```

***

## 10. 实体缓存清理 — `crates/app/src/contribution/entity_cache.rs`

* **保留** `get_or_create_entity` / `visual_entity`（视觉贡献 Entity 单例，供 `IVisualContribution::render` 复用）

* **删除** `build_activity_panels`（依赖 `extract_visual`，不再需要）

* ActivityPanel 构建改由 host 业务代码直接从 `add_visual` 受理的 `Arc<dyn IVisualContribution>` 列表构造

***

## 11. Demo 代码变更

### 11.1 `demo/src/shell/main_window.rml.rs`

```rust
#[window]
#[contributehost(id = "demo.shell")]
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

    // 用户自管存储（替代宏注入的 entries）
    visual_entries: std::sync::RwLock<Vec<(Arc<dyn IVisualContribution>, ContributionOptions)>>,
    // host handle receiver
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl rml_core::contribution::IContributionHost for MainWindow {
    fn id(&self) -> &'static str { Self::ID }

    fn add_visual(&self, c: Arc<dyn IVisualContribution>, o: ContributionOptions) {
        self.visual_entries.write().unwrap().push((c, o));
    }

    fn remove(&self, contribution_id: &str) {
        let mut entries = self.visual_entries.write().unwrap();
        entries.retain(|(c, _)| c.id() != contribution_id);
    }
}

impl rml_core::lifecycle::ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 1. 注册 host + 触发 demo.shell 的所有贡献注册
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);

        // 2. drain（register_visual 同步入队，此时已可 drain）
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }

        // 3. 初始化 welcome tab / DemoShellHost / menu_commands（原 host_on_loaded 逻辑）
        // ...（保留原代码）

        // 4. 构建 ActivityBar（从 visual_entries 过滤 slot="activity"）
        let panels = build_activity_panels_from(&self.visual_entries.read());
        self.activity_bar = Some(cx.new(|_| ActivityBar::new(panels)));
        // ...

        cx.notify();
    }
}

impl MainWindow {
    pub fn active_case_view(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let entries = self.visual_entries.read().unwrap();
        if let Some((visual, _)) = entries.iter().find(|(c, _)| c.id() == self.active_case_id) {
            return visual.render(window, cx);
        }
        gpui::div().into_any_element()
    }

    fn refresh_shell_chrome(&mut self) {
        let entries = self.visual_entries.read().unwrap();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries, &self.menu_commands);
    }
    // ... 其余方法不变，删除 #[computed] 中的 i18n_version 引用
}
```

### 11.2 `demo/src/shell/activity_panel.rml.rs`

```rust
#[contribute(host_id = "demo.shell", id = "samples", name = "shell.samples", icon = IconName::BookOpen, kind = "activity", order = 0)]
#[contributehost(id = "demo.activity")]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    // 用户自管存储：case 视觉贡献
    case_entries: std::sync::RwLock<Vec<(Arc<dyn IVisualContribution>, ContributionOptions)>>,
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl rml_core::contribution::IContributionHost for ActivityPanel {
    fn id(&self) -> &'static str { Self::ID }

    fn add_visual(&self, c: Arc<dyn IVisualContribution>, o: ContributionOptions) {
        self.case_entries.write().unwrap().push((c, o));
    }

    fn remove(&self, id: &str) {
        let mut e = self.case_entries.write().unwrap();
        e.retain(|(c, _)| c.id() != id);
    }
}

impl rml_core::lifecycle::ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }
        self.refresh_tree(cx);
        cx.notify();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let entries = self.case_entries.read().unwrap();
        let items = map_case_tree_items(&entries);
        self.set_tree_items(items, cx);
    }
    // ... 其余不变
}
```

### 11.3 `demo/src/shell/shell_chrome.rs`

签名从 `&[ContributionEntry]` 改为 `&[(Arc<dyn IVisualContribution>, ContributionOptions)]`：

```rust
use rml_core::contribution::{IVisualContribution, ContributionOptions};

type VisualEntry<'a> = &'a (std::sync::Arc<dyn IVisualContribution>, ContributionOptions);

fn entries_in_slot<'a>(entries: &'a [(std::sync::Arc<dyn IVisualContribution>, ContributionOptions)], slot: &str) -> Vec<VisualEntry<'a>> {
    entries.iter().filter(|(_, o)| o.effective_slot() == Some(slot)).collect()
}

pub fn map_status_items(entries: &[(std::sync::Arc<dyn IVisualContribution>, ContributionOptions)]) -> StatusBarItems {
    let mut items = entries_in_slot(entries, "status");
    items.sort_by_key(|(_, o)| o.order);
    items.into_iter().map(|(c, o)| {
        let align = match o.properties.get("align").map(|s| s.as_ref()) {
            Some("right") => StatusBarAlign::Right,
            _ => StatusBarAlign::Left,
        };
        StatusBarItem::new(c.name()).align(align).into_arc()
    }).collect()
}

pub fn map_menu_items(entries: &[(std::sync::Arc<dyn IVisualContribution>, ContributionOptions)], commands: &HashMap<String, Arc<dyn ICommand>>) -> MenuItems {
    // 类似改写：用 c.id() / c.name()
}

pub fn map_case_tree_items(entries: &[(std::sync::Arc<dyn IVisualContribution>, ContributionOptions)]) -> Vec<TreeItem> {
    // 类似改写：用 c.id() / c.name()
}
```

**新增**：`build_activity_panels_from`（从 `visual_entries` 过滤 `slot="activity"`，构造 `ActivityPanels`）。可放 `shell_chrome.rs` 或 `main_window.rml.rs`。

***

## 12. 遗留问题与 Follow-up

### 12.1 `#[computed]` 与 i18n 缓存失效

`#[computed]` 方法基于字段版本号缓存。若方法内调用 `t_static`（不依赖字段），i18n 切换后缓存不失效。

**当前方案**：`#[contributehost]` 宏移除 `i18n_version` 注入后，依赖 `t_static` 的 `#[computed]` 需改为普通方法（每次调用重算），或显式依赖一个 i18n 版本字段。

**Follow-up**：框架提供应用级 i18n 版本号，`#[computed]` 可声明 `#[computed(deps = [i18n])]` 依赖全局版本。本计划不实现，另行设计。

### 12.2 `register_rml_contributions_for` 路径

宏生成的 `__rml_install_host` 调用 `rml_app::contribution::bootstrap_host_contributions(cx, Self::ID)`，后者通过 `CONTRIBUTION_BOOTSTRAP` 全局函数指针回调 build.rs 生成的 `register_rml_contributions_for`。无需宏知道生成文件路径。

### 12.3 host 未注册时的贡献丢失

`register`/`register_visual` 在 host 不存在时 drop 贡献（warn 日志）。要求 host `on_loaded` 必须先 `__rml_install_host` 再触发任何业务注册。`#[contributehost]` 宏生成的 `__rml_install_host` 已保证此顺序（先 `add_host` 再 `bootstrap_host_contributions`）。

### 12.4 ActivityPanel 双重角色

ActivityPanel 既是贡献（to `demo.shell`）又是 host（`demo.activity`）：

* 作为贡献：`__rml_register_activitypanel(cx)` 在 `register_rml_contributions_for("demo.shell")` 中被调用 → `register_visual("demo.shell", ...)` → MainWindow\.add\_visual

* 作为 host：自身 `on_loaded` 调 `__rml_install_host` → `add_host("demo.activity")` → `bootstrap_host_contributions("demo.activity")` → 注册所有 case 贡献到 ActivityPanel.add\_visual

时序无冲突：MainWindow 先于 ActivityPanel Entity 创建（ActivityPanel Entity 由 `get_or_create_entity` 在 MainWindow 渲染时创建），但 ActivityPanel 作为贡献的注册在 MainWindow 的 `__rml_install_host` 中触发（同步），此时 ActivityPanel Entity 可能尚未创建。`IVisualContribution::render` 通过 `get_or_create_entity` 懒创建 Entity，所以注册时只需 `Arc::new(ActivityPanel::default())`（数据对象，非 Entity），Entity 在首次 render 时创建。

***

## 13. 迁移检查清单

### crates/core

* [ ] `contribution.rs`：删除 `ContributionEntry`/`IHostEntity`/`VisualExtractor`/`take_pending`

* [ ] `contribution.rs`：`IContributionHost` 改为 `add`/`add_visual`/`remove` 默认空实现

* [ ] `contribution.rs`：`IContributionRegistry` 改名 `add_host`/`remove_host`，新增 `register_visual`，删 `take_pending`

### crates/app

* [ ] `contribution/registry.rs`：删除 `VISUAL_EXTRACTORS`/`extract_visual`/`register_visual_extractor`/`pending`

* [ ] `contribution/registry.rs`：实现新 `IContributionRegistry`（仅存 hosts）

* [ ] `contribution/host_handle.rs`：新增 `EntityHostHandle`/`HostOp`/`install_entity_host`/`drain_host_ops`

* [ ] `contribution/global.rs`：`install_contribution_bootstrap` 签名改 `fn(&mut App, &str)`；新增 `bootstrap_host_contributions`；删 `bootstrap_contributions`

* [ ] `contribution/entity_cache.rs`：删 `build_activity_panels`

* [ ] `contribution/mod.rs`：更新导出

* [ ] `application.rs`：删 `bootstrap_contributions(cx)` 调用

### crates/macros

* [ ] `contributehost.rs`：删字段注入、IContributionHost/ILifecycle 自动 impl；生成 `ID` + `__rml_install_host` + `T: IContributionHost` 断言

* [ ] `contribute.rs`：删 `visual_extractor` 块；`__rml_register_*` 按视觉/非视觉调 `register_visual`/`register`

### crates/engine

* [ ] `build/contribution_generator.rs`：`ContributionRegistrar` 加 `host_id` 字段；生成 `register_rml_contributions_for(cx, host_id)`；`install_contribution_bootstrap` 新签名

### demo

* [ ] `shell/main_window.rml.rs`：手写 `impl IContributionHost` + `impl ILifecycle`；用 `visual_entries: RwLock<Vec<...>>` 替代 `entries`；删 `i18n_version`/`extract_visual`/`build_activity_panels` 调用

* [ ] `shell/activity_panel.rml.rs`：同样模式重写

* [ ] `shell/shell_chrome.rs`：签名改 `&[(Arc<dyn IVisualContribution>, ContributionOptions)]`

### 验证

* [ ] `cargo build -p rust-rml-core`

* [ ] `cargo build -p rust-rml-app`

* [ ] `cargo build -p rust-rml-macros`

* [ ] `cargo build -p rust-rml-engine`

* [ ] `cargo build -p rust-rml-demo`

* [ ] `cargo test -p rust-rml-engine`

* [ ] demo 运行：菜单/状态栏/活动栏/案例切换/i18n/theme 切换正常

***

## 14. 实施顺序

1. **crates/core**：`contribution.rs` 契约重设计（破坏性变更，先做）
2. **crates/app**：`registry.rs` + `host_handle.rs` + `global.rs` + `entity_cache.rs` + `mod.rs`
3. **crates/macros**：`contributehost.rs` + `contribute.rs`
4. **crates/engine**：`contribution_generator.rs`
5. **demo**：`main_window.rml.rs` + `activity_panel.rml.rs` + `shell_chrome.rs`
6. **验证**：build + test + 运行

