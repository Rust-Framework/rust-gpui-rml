# 移除 HostHandle + ObservableVec 数据绑定架构计划

## 摘要

本计划解决 RML 贡献点架构的三个核心问题：

1. **`HostHandle`** **是多余的胶水层** —— 注册器应直接投递给 `IContributionHost` 实现方，不需要类型擦除的 `WeakEntity<T>` 包装
2. **`cx: &mut App`** **不应出现在 trait 方法签名中** —— 从架构角度，`IContributionHost::add`/`remove` 和 `IContributionRegistry::register`/`unregister`/`add`/`remove` 都是纯数据操作，不需要 GPUI 上下文
3. **缺少真正的数据驱动 UI** —— `ObservableVec` 提供集合响应式能力，host 的 `add`/`remove` 直接操作 `ObservableVec`，UI 自动更新

**核心架构转变**：host 不再是 Entity 本身，而是独立的 `Arc<dyn IContributionHost>` 数据对象。Entity 持有 `Arc<Host>` 引用，在 `render` 中只读访问 host 的 `ObservableVec` 字段。`ObservableVec` 内部使用 `RwLock` + `flume` channel 实现 `&self` 安全变更 + 自动 UI 通知。

***

## 用户决策

| # | 决策                                                                                    | 来源                                           |
| - | ------------------------------------------------------------------------------------- | -------------------------------------------- |
| 1 | `HostHandle` trait 应完全移除，注册器直接投递给 `IContributionHost`                                 | "最终HostHandle应该都是不需要的，统一注册器直接投递给host实现方即可"   |
| 2 | `IContributionRegistry::add` 接收 `Arc<dyn IContributionHost>`，不是 `Box<dyn HostHandle>` | "add接收的应该是IContributionHost本身，而不是HostHandle" |
| 3 | 所有 trait 方法移除 `cx: &mut App` 参数                                                       | "根据设计的理解是不需要的"                               |
| 4 | 使用 `ObservableVec` 方案实现数据绑定                                                           | 选中 "ObservableVec 方案"                        |
| 5 | 保持接口设计简洁，从架构角度解决问题                                                                    | "保持接口设计的简洁应用，从架构角度解决问题"                      |

**继承的已有决策**（不变）：

* `IContribution`/`IVisualContribution` trait 方法签名禁止修改

* `IContribution` 禁止添加 `as_visual()`，使用 `Any` supertrait + `VisualExtractor` 自由函数

* `IVisualContribution::render(&self, window: &mut Window, cx: &mut App) -> AnyElement`

* 框架不存储贡献数据、不缓存 Entity

* `#[computed_with_cx]` 宏否决，`contribution_entries` 不出现在业务代码

***

## 当前状态分析

### 问题 1：`HostHandle` 多余的胶水层

```
当前架构：
  registry.register(host_id, contribution, options, cx)
    → hosts.get(host_id) → HostHandle::add(contribution, options, cx)
      → cx.defer → entity.update(cx, |host, ctx| host.add(contribution, options, ctx))

问题：HostHandle 只是 WeakEntity<T> 的类型擦除包装，add/remove 委托给 entity.update
     这一层完全多余——如果 host 本身是 Arc<dyn IContributionHost>，可以直接调 host.add()
```

**文件**：[contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs#L98-L104) `HostHandle` trait 定义
**文件**：[global.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/global.rs#L69-L114) `EntityHostHandleBox` + `HostHandle` impl

### 问题 2：`cx: &mut App` 污染 trait 接口

```rust
// 当前 IContributionHost（L88-L96）
fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
fn remove(&mut self, contribution_id: &str, cx: &mut App);

// 当前 IContributionRegistry（L117-L127）
fn add(&self, host: Box<dyn HostHandle>, cx: &mut App);           // ← cx 不需要
fn remove(&self, host_id: &str, cx: &mut App);                    // ← cx 不需要
fn register(&self, host_id: &str, ..., cx: &mut App);             // ← cx 不需要
fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool; // ← cx 不需要
```

`cx` 存在的唯一原因：`HostHandle::add` 需要调用 `entity.update(cx, ...)`。移除 `HostHandle` 后，`host.add()` 直接操作 `ObservableVec`（内部可变性），不需要 `cx`。

### 问题 3：缺少 ObservableVec

当前 host 的 `add` 是 `&mut self` + `cx`，无法在不持有 `cx` 时修改数据。`ObservableVec` 提供 `&self` 安全的 `push`/`retain` 等操作，配合 channel 通知机制实现自动 UI 更新。

***

## Phase A：`ObservableVec<T>` 核心类型

### 新建文件

**`crates/core/src/observable.rs`** —— 基于 `RwLock` + `AtomicU64` + `flume` channel 的可观察集合

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use flume::Sender;

/// 版本号驱动的可观察集合。
/// `&self` 安全变更（内部 RwLock），mutation 自动 bump version + 发送 channel 通知。
/// host Entity 在 on_loaded 中 spawn 后台任务接收通知并调用 cx.notify()。
pub struct ObservableVec<T> {
    inner: RwLock<Vec<T>>,
    version: AtomicU64,
    notify: Option<Sender<()>>,
}

impl<T> ObservableVec<T> {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            version: AtomicU64::new(0),
            notify: None,
        }
    }

    /// 创建带通知 channel 的 ObservableVec。
    /// push/insert/remove 等 mutation 方法会发送 `()` 到 channel。
    pub fn with_notifier(notify: Sender<()>) -> Self {
        Self {
            inner: RwLock::new(Vec::new()),
            version: AtomicU64::new(0),
            notify: Some(notify),
        }
    }

    fn bump(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
        if let Some(tx) = &self.notify {
            let _ = tx.send(());
        }
    }

    // —— mutation 方法（&self，内部 RwLock 写锁）——
    pub fn push(&self, value: T) {
        self.inner.write().unwrap().push(value);
        self.bump();
    }

    pub fn insert(&self, index: usize, value: T) {
        self.inner.write().unwrap().insert(index, value);
        self.bump();
    }

    pub fn remove(&self, index: usize) -> T {
        let v = self.inner.write().unwrap().remove(index);
        self.bump();
        v
    }

    pub fn swap(&self, a: usize, b: usize) {
        let mut guard = self.inner.write().unwrap();
        guard.swap(a, b);
        drop(guard);
        self.bump();
    }

    pub fn clear(&self) {
        self.inner.write().unwrap().clear();
        self.bump();
    }

    pub fn retain<F>(&self, f: F)
    where
        F: FnMut(&T) -> bool,
    {
        let mut guard = self.inner.write().unwrap();
        let before = guard.len();
        guard.retain(f);
        let after = guard.len();
        drop(guard);
        if before != after {
            self.bump();
        }
    }

    pub fn sort_by_mut<F>(&self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.inner.write().unwrap().sort_by(compare);
        self.bump();
    }

    // —— 只读方法（&self，内部 RwLock 读锁或 lock-free）——
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<T>> {
        self.inner.read().unwrap()
    }

    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    pub fn iter(&self) -> impl Iterator<Item = std::sync::Arc<T>> {
        // 返回 Arc<T> 迭代器避免 RwLockReadGuard 生命周期问题
        // 简化方案：collect 后迭代
        self.inner.read().unwrap().iter().cloned().collect::<Vec<_>>().into_iter()
    }
}

impl<T> Default for ObservableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

// 注意：不实现 DerefMut —— 强制通过 mutation 方法修改，确保 version bump
```

### 修改文件

| 文件                              | 操作                                                              |
| ------------------------------- | --------------------------------------------------------------- |
| `crates/core/src/observable.rs` | **新建**：`ObservableVec<T>` 实现                                    |
| `crates/core/src/lib.rs`        | 添加 `pub mod observable;` + `pub use observable::ObservableVec;` |
| `crates/core/src/prelude.rs`    | 导出 `ObservableVec`                                              |
| `crates/core/Cargo.toml`        | 添加 `flume = "0.11"` 依赖                                          |

### 设计要点

* **`RwLock<Vec<T>>`**：`&self` 安全的 mutation，多读单写

* **`AtomicU64`** **version**：lock-free 读取，供 `#[computed]` 缓存键使用

* **`Option<Sender<()>>`**：可选的通知 channel。`None` 时（如 `Default`）不发送通知，仅 bump version。host 在 `on_loaded` 中通过 `with_notifier(tx)` 创建带通知的实例

* **不实现** **`DerefMut`**：强制通过 `push`/`retain` 等方法修改，确保 version bump

* **`read()`** **返回** **`RwLockReadGuard`**：调用方在 guard 生命周期内只读访问数据

***

## Phase B：Core trait 重构

### 修改文件

**`crates/core/src/contribution.rs`** —— 移除 `HostHandle`，重构 trait 签名

```rust
/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
/// host 使用 ObservableVec 等内部可变性结构存储数据，add/remove 为 &self。
pub trait IContributionHost: Send + Sync + 'static {
    const ID: &'static str;

    /// 运行时获取 host ID（供 trait 对象访问 const ID）。
    fn id(&self) -> &'static str {
        Self::ID
    }

    /// 受理代码：接收并处置贡献。host 按 options.slot/group 分发到自有 ObservableVec。
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions);

    /// 移除贡献。host 自行清理对应数据。
    fn remove(&self, contribution_id: &str);
}

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add 方法。
/// 所有方法 &self + 无 cx —— 内部 RwLock 可变性，host.add 直接调用。
pub trait IContributionRegistry: Send + Sync {
    /// 注册 host（host Entity 在 on_loaded 时调用，传入 Arc<dyn IContributionHost>）。
    fn add(&self, host: Arc<dyn IContributionHost>);

    /// 注销 host。
    fn remove(&self, host_id: &str);

    /// 向 host 注册贡献（#[contribute] 宏生成代码调用）。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    );

    /// 从 host 注销贡献。
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
```

**移除**：

* `HostHandle` trait（L98-L104）

* `HostHandle` 在 prelude 中的导出

**保留不变**：

* `IContribution` trait 签名（`Send + Sync + Any`，`id`/`name`/`description`/`icon`）

* `IVisualContribution` trait 签名（`render(&self, window: &mut Window, cx: &mut App) -> AnyElement`）

* `ContributionOptions` 结构体

* `VisualExtractor` 类型别名

### 修改 prelude

**`crates/core/src/prelude.rs`**：

```rust
pub use crate::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
    IVisualContribution,
};
// 移除 HostHandle 导出
pub use crate::observable::ObservableVec;
```

***

## Phase C：App 层重构

### C1：`crates/app/src/contribution/registry.rs` 重写

```rust
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IContributionRegistry,
};

static VISUAL_EXTRACTORS: OnceLock<RwLock<HashMap<TypeId, rml_core::contribution::VisualExtractor>>> =
    OnceLock::new();

fn visual_extractors() -> &'static RwLock<HashMap<TypeId, rml_core::contribution::VisualExtractor>> {
    VISUAL_EXTRACTORS.get_or_init(|| RwLock::new(HashMap::new()))
}

#[doc(hidden)]
pub fn register_visual_extractor(type_id: TypeId, extractor: rml_core::contribution::VisualExtractor) {
    visual_extractors().write().unwrap().insert(type_id, extractor);
}

pub fn extract_visual(
    contribution: &Arc<dyn IContribution>,
) -> Option<Arc<dyn rml_core::contribution::IVisualContribution>> {
    let type_id = (**contribution).type_id();
    let extractors = visual_extractors().read().unwrap();
    extractors.get(&type_id).and_then(|f| f(contribution))
}

/// 框架内部实现：桥接 contribute → host
pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, Arc<dyn IContributionHost>>>,
    pending: RwLock<HashMap<String, Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}

impl ContributionRegistry {
    pub fn new() -> Self {
        Self {
            hosts: RwLock::new(HashMap::new()),
            pending: RwLock::new(HashMap::new()),
        }
    }

    pub fn has_host(&self, host_id: &str) -> bool {
        self.hosts.read().unwrap().contains_key(host_id)
    }
}

impl Default for ContributionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl IContributionRegistry for ContributionRegistry {
    fn add(&self, host: Arc<dyn IContributionHost>) {
        let id = host.id().to_string();
        {
            let mut hosts = self.hosts.write().unwrap();
            hosts.insert(id.clone(), host);
        }

        // 重放 pending 队列 —— 直接调 host.add()，无需 cx
        let queue = {
            let mut pending = self.pending.write().unwrap();
            pending.remove(&id).unwrap_or_default()
        };

        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(&id) {
            for (contribution, options) in queue {
                host.add(contribution, options);
            }
        }
    }

    fn remove(&self, host_id: &str) {
        self.hosts.write().unwrap().remove(host_id);
    }

    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        } else {
            drop(hosts);
            self.pending
                .write()
                .unwrap()
                .entry(host_id.to_string())
                .or_default()
                .push((contribution, options));
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

### C2：`crates/app/src/contribution/global.rs` 重写

移除 `EntityHostHandleBox`、`HostHandle` impl、`register_host` 函数。保留 `ContributionRegistryExt`、`bootstrap_contributions` 等。

```rust
use std::sync::{Mutex, OnceLock};

use gpui::App;
use rml_core::contribution::IContributionRegistry;

use super::registry::ContributionRegistry;

static CONTRIBUTION_BOOTSTRAP: Mutex<Option<fn(&mut App)>> = Mutex::new(None);

static REGISTRY: OnceLock<ContributionRegistry> = OnceLock::new();

fn registry() -> &'static ContributionRegistry {
    REGISTRY.get_or_init(ContributionRegistry::new)
}

pub fn install_contribution_bootstrap(f: fn(&mut App)) {
    *CONTRIBUTION_BOOTSTRAP.lock().unwrap() = Some(f);
}

pub fn bootstrap_contributions(cx: &mut App) {
    if let Some(f) = CONTRIBUTION_BOOTSTRAP.lock().unwrap().as_ref() {
        f(cx);
    }
}

pub fn ensure_contribution_registry(_cx: &mut App) {
    let _ = registry();
}

/// App 扩展：获取 IContributionRegistry 接口。
/// 返回 &'static 引用——不借用 App，所有方法 &self + 内部 RwLock 可变性。
pub trait ContributionRegistryExt {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry;
}

impl ContributionRegistryExt for App {
    fn get_contribution_registry(&self) -> &'static dyn IContributionRegistry {
        registry()
    }
}
```

**移除**：

* `EntityHostHandleBox<T>` 结构体

* `HostHandle for EntityHostHandleBox<T>` impl

* `register_host<T>(cx)` 函数

* `cx.defer` 嵌套借用规避代码

### C3：`crates/app/src/contribution/mod.rs` 更新

```rust
mod global;
mod registry;

pub use global::{
    bootstrap_contributions, ensure_contribution_registry, install_contribution_bootstrap,
    ContributionRegistryExt,
};
pub use registry::extract_visual;

#[doc(hidden)]
pub use registry::{register_visual_extractor, ContributionRegistry};
```

**移除**：`register_host`、`EntityHostHandleBox` 导出

### C4：`crates/app/src/lib.rs` 更新

```rust
pub use contribution::{
    bootstrap_contributions, ensure_contribution_registry, extract_visual,
    ContributionRegistryExt,
};
// 移除 register_host 导出
```

***

## Phase D：宏调整

### D1：`crates/macros/src/contribute.rs`

**修改点**：生成的 `__rml_register_<name>(cx)` 函数中，移除 `register()` 调用的 `cx` 参数。

```rust
// 当前（L319-L332）：
pub fn #register_fn(cx: &mut gpui::App) {
    use rml_app::contribution::ContributionRegistryExt;
    cx.get_contribution_registry().register(
        #host_id,
        std::sync::Arc::new(#struct_name::default()),
        rml_core::contribution::ContributionOptions::new()
            #slot #parent_id #order #group #align,
        cx,  // ← 移除此行
    );
}

// 修改后：
pub fn #register_fn(cx: &mut gpui::App) {
    use rml_app::contribution::ContributionRegistryExt;
    cx.get_contribution_registry().register(
        #host_id,
        std::sync::Arc::new(#struct_name::default()),
        rml_core::contribution::ContributionOptions::new()
            #slot #parent_id #order #group #align,
    );
}
```

**注意**：`cx: &mut gpui::App` 参数保留——`get_contribution_registry(&self)` 需要 `&App`，且 `bootstrap_contributions(cx)` 调用链需要 `&mut App`。仅移除传给 `register()` 的 `cx`。

### D2：`crates/macros/src/contributehost.rs`

**修改点**：更新文档注释（L8），移除"host 在 on\_loaded 调 register\_host(cx) 注册自身"的描述。

```rust
// 当前：
//! 宏**不**生成注册函数——host 在 `on_loaded` 调 `register_host(cx)` 注册自身。

// 修改后：
//! 宏**不**生成注册函数——host 实现方在 Entity 的 `on_loaded` 中创建 `Arc<Host>`
//! 并调用 `cx.get_contribution_registry().add(host)` 注册自身。
```

无结构性变化。

### D3：`crates/engine/src/build/contribution_generator.rs`

**无需修改**。生成的 `register_rml_contributions(cx: &mut App)` 函数签名不变（`cx` 仍传递给 `__rml_register_<name>(cx)`），内部调用的 `register()` 不再传 `cx`。

***

## Phase E：Demo 重构（main\_window\.rml.rs）

### 核心架构：分离 Host 数据与 Entity

```
MainWindowHost（IContributionHost 实现，Arc 共享）
  ├── menu_entries: ObservableVec<MenuEntry>
  ├── status_entries: ObservableVec<StatusEntry>
  ├── activity_entries: ObservableVec<ActivityEntry>
  └── case_entries: ObservableVec<CaseEntry>

MainWindow（Entity，UI 组件）
  ├── host: Arc<MainWindowHost>  ← 在 on_loaded 中创建
  ├── open_tabs: Vec<OpenTab>
  ├── selected_tab: usize
  └── ... 其他 UI 状态
```

### E1：`demo/src/shell/main_window.rml.rs` 重写

```rust
use std::sync::Arc;
use flume;
use gpui::{Window, IntoElement, AnyElement};
use rml::prelude::*;
use rml_core::contribution::{IContribution, IContributionHost, ContributionOptions};
use rml_core::i18n::I18nExt;

/// 贡献 host 数据——Arc 共享，registry 与 MainWindow 各持一份。
/// add/remove 为 &self，使用 ObservableVec 内部可变性。
#[contributehost(id = "demo.shell")]
pub struct MainWindowHost {
    menu_entries: ObservableVec<MenuEntry>,
    status_entries: ObservableVec<StatusEntry>,
    activity_entries: ObservableVec<ActivityEntry>,
    case_entries: ObservableVec<CaseEntry>,
}

impl MainWindowHost {
    /// 创建带 channel 通知的 host。
    /// 返回 (host, rx) —— rx 由 Entity 的 on_loaded 消费。
    pub fn new(notify: flume::Sender<()>) -> Self {
        Self {
            menu_entries: ObservableVec::with_notifier(notify.clone()),
            status_entries: ObservableVec::with_notifier(notify.clone()),
            activity_entries: ObservableVec::with_notifier(notify.clone()),
            case_entries: ObservableVec::with_notifier(notify),
        }
    }
}

impl IContributionHost for MainWindowHost {
    const ID: &'static str = "demo.shell";

    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions) {
        match options.effective_slot() {
            Some("menu") => self.menu_entries.push(MenuEntry::from(&contribution, &options)),
            Some("status") => self.status_entries.push(StatusEntry::from(&contribution, &options)),
            Some("activity") => self.activity_entries.push(ActivityEntry::from(&contribution, &options)),
            Some("case") => self.case_entries.push(CaseEntry::from(&contribution, &options)),
            _ => {}
        }
        // ObservableVec::push 已 bump version + 发送 channel 通知
        // Entity 的后台任务接收通知 → cx.notify() → 自动重渲
    }

    fn remove(&self, contribution_id: &str) {
        self.menu_entries.retain(|e| e.id != contribution_id);
        self.status_entries.retain(|e| e.id != contribution_id);
        self.activity_entries.retain(|e| e.id != contribution_id);
        self.case_entries.retain(|e| e.id != contribution_id);
    }
}

#[window]
#[derive(Default)]
pub struct MainWindow {
    host: Option<Arc<MainWindowHost>>,
    open_tabs: Vec<OpenTab>,
    selected_tab: usize,
    active_case_id: String,
    show_chrome: bool,
    i18n_version: u32,
    menu_commands: HashMap<String, Arc<dyn ICommand>>,
    slot_left_size: gpui::Pixels,
}

impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 初始化 UI 状态
        if self.open_tabs.is_empty() {
            self.open_tabs.push(OpenTab {
                id: "welcome".to_string(),
                title: cx.t("shell.welcome").to_string(),
            });
            self.selected_tab = 0;
            self.active_case_id = "welcome".to_string();
        }
        self.show_chrome = true;
        self.slot_left_size = gpui::px(260.);

        // 创建 host + channel 通知
        let (tx, rx) = flume::unbounded::<()>();
        let host = Arc::new(MainWindowHost::new(tx));
        self.host = Some(host.clone());

        // spawn 后台任务：接收 ObservableVec 变更通知 → cx.notify()
        cx.spawn(async move |this, cx| {
            while rx.recv_async().await.is_ok() {
                let _ = this.update(cx, |_, cx| cx.notify());
            }
        }).detach();

        // 注册 host 到 registry → 重放 pending 贡献 → host.add 逐条调用
        cx.get_contribution_registry().add(host);

        // menu_commands 初始化（与当前相同）
        // ...

        // 无需 subscribe_host_changes —— ObservableVec channel 通知已替代
        // 无需 refresh_shell_chrome —— #[computed] 从 ObservableVec 自动计算
    }
}

impl MainWindow {
    #[computed]
    pub fn menu_items(&self) -> MenuItems {
        let host = self.host.as_ref().unwrap();
        let entries = host.menu_entries.read();
        build_menu_tree(&entries, &self.menu_commands)
    }

    #[computed]
    pub fn status_items(&self) -> StatusBarItems {
        let host = self.host.as_ref().unwrap();
        let entries = host.status_entries.read();
        build_status_items(&entries)
    }

    #[computed]
    pub fn activity_panels(&self) -> ActivityPanels {
        let host = self.host.as_ref().unwrap();
        let entries = host.activity_entries.read();
        build_activity_panels(&entries)
    }

    #[computed]
    pub fn case_tree_items(&self) -> Vec<TreeItem> {
        let host = self.host.as_ref().unwrap();
        let entries = host.case_entries.read();
        build_case_tree(&entries)
    }

    /// 渲染当前激活的视觉贡献。
    pub fn active_case_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        use rml_app::contribution::extract_visual;
        let host = match &self.host {
            Some(h) => h,
            None => return gpui::div().into_any_element(),
        };
        let entries = host.case_entries.read();
        if let Some(entry) = entries.iter().find(|e| e.id == self.active_case_id) {
            if let Some(visual) = extract_visual(&entry.contribution) {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }
}
```

### E2：host 内部存储格式（业务自定义类型）

在 `demo/src/shell/main_window.rml.rs` 或单独文件中定义：

```rust
struct MenuEntry {
    id: String,
    name: SharedString,
    order: i32,
    parent_id: Option<String>,
    contribution: Arc<dyn IContribution>,
}

impl MenuEntry {
    fn from(contribution: &Arc<dyn IContribution>, options: &ContributionOptions) -> Self {
        Self {
            id: contribution.id().to_string(),
            name: contribution.name(),
            order: options.order,
            parent_id: options.parent_id.as_deref().map(String::from),
            contribution: contribution.clone(),
        }
    }
}
// 类似 StatusEntry, ActivityEntry, CaseEntry ...
```

### E3：移除的旧 API 引用

`main_window.rml.rs` 当前使用的以下 API 将被移除/替代：

| 旧 API                                                    | 替代方案                                                    |
| -------------------------------------------------------- | ------------------------------------------------------- |
| `subscribe_host_changes(Self::ID, cx, \|this, cx\| ...)` | `ObservableVec` channel 通知 → `cx.spawn` → `cx.notify()` |
| `ContributionRegistryGlobal`                             | 不再使用 Global，改用 `OnceLock` 静态实例                          |
| `ComponentEntityCache`                                   | 移除（已删除）                                                 |
| `contribution_entries(Self::ID, cx)`                     | `host.menu_entries.read()` 等                            |
| `render_contribution_visual(&visual, window, cx)`        | `extract_visual(&contrib).render(window, cx)`           |
| `refresh_shell_chrome(cx)`                               | `#[computed]` 从 ObservableVec 自动计算                      |
| `register_host(cx)`                                      | `cx.get_contribution_registry().add(Arc::new(host))`    |

### E4：`activity_panel.rml.rs` 调整

移除 `#[contribute]`，成为固定 shell 组件。`ActivityPanel` 通过 `DemoShellHost` WeakEntity observe `MainWindow` Entity 变化。

***

## 关键文件清单

| 文件                                        | Phase | 操作                                          |
| ----------------------------------------- | ----- | ------------------------------------------- |
| `crates/core/src/observable.rs`           | A     | **新建**                                      |
| `crates/core/Cargo.toml`                  | A     | 添加 `flume` 依赖                               |
| `crates/core/src/lib.rs`                  | A, B  | 添加 `pub mod observable`                     |
| `crates/core/src/prelude.rs`              | A, B  | 导出 `ObservableVec`，移除 `HostHandle`          |
| `crates/core/src/contribution.rs`         | B     | 重写 trait 签名，移除 `HostHandle`                 |
| `crates/app/src/contribution/registry.rs` | C     | 重写：`Arc<dyn IContributionHost>` + 无 cx      |
| `crates/app/src/contribution/global.rs`   | C     | 重写：移除 `EntityHostHandleBox`/`register_host` |
| `crates/app/src/contribution/mod.rs`      | C     | 更新导出                                        |
| `crates/app/src/lib.rs`                   | C     | 移除 `register_host` 导出                       |
| `crates/macros/src/contribute.rs`         | D     | 移除 `register()` 的 `cx` 参数                   |
| `crates/macros/src/contributehost.rs`     | D     | 更新文档注释                                      |
| `demo/src/shell/main_window.rml.rs`       | E     | 重写：分离 Host + Entity                         |

***

## 验证步骤

### Phase A 验证

```bash
cargo build -p rust-rml-core
cargo test -p rust-rml-core -- observable
```

验证：

1. `ObservableVec::new()` 创建无通知实例
2. `ObservableVec::with_notifier(tx)` 创建带通知实例
3. `push`/`insert`/`remove`/`retain`/`clear` 后 `version()` 递增
4. `with_notifier` 的实例 mutation 后 channel 收到 `()`
5. `read()` 返回 `RwLockReadGuard`，可只读访问

### Phase B + C 验证

```bash
cargo build -p rust-rml-core -p rust-rml-app
cargo test -p rust-rml-app -- contribution
```

验证：

1. `IContributionHost::add`/`remove` 签名为 `&self`，无 `cx`
2. `IContributionRegistry` 所有方法无 `cx`
3. `HostHandle` trait 完全移除，编译无引用
4. `EntityHostHandleBox` 完全移除，编译无引用
5. `register_host` 函数移除，编译无引用
6. `ContributionRegistry` 存储 `Arc<dyn IContributionHost>`
7. pending 队列重放：`add` 后 pending 中的贡献被逐条投递到 `host.add()`

### Phase D 验证

```bash
cargo build -p rust-rml-macros -p rust-rml-engine
```

验证：

1. `#[contribute]` 生成的 `register()` 调用不传 `cx`
2. `#[contributehost]` 生成的 `const ID` + 断言不变

### Phase E 验证

```bash
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

验证：

1. Demo 启动后 menu/status/activity 面板正确显示
2. 通过菜单打开 case → tab 新增 → UI 自动更新（无 `refresh_shell_chrome`）
3. `ObservableVec` mutation → channel 通知 → `cx.notify()` → 重渲
4. **无** **`contribution_entries`** **/** **`subscribe_host_changes`** **/** **`HostHandle`** **引用出现在 demo 代码中**

***

## 假设与设计决策

### 假设

1. **Host 与 Entity 分离**：`IContributionHost` 实现方是独立的 `Arc` 共享数据对象，不是 Entity 内部类型。Entity 持有 `Arc<Host>` 引用，在 `render` 中只读访问 host 的 `ObservableVec` 字段。
2. **channel 通知**：`ObservableVec::push` 等 mutation 方法通过 `flume::Sender<()>` 发送通知。Entity 在 `on_loaded` 中 spawn 后台任务消费 `flume::Receiver` 并调用 `cx.notify()`。
3. **pending 队列重放无需 cx**：`add` 重放 pending 时直接调 `host.add(contribution, options)`，host 内部 `ObservableVec::push` 发送 channel 通知，后台任务统一触发 `cx.notify()`。
4. **`flume`** **依赖**：`flume 0.11` 已在 Cargo.lock（gpui 传递依赖），需添加为 `rust-rml-core` 直接依赖。

### 设计决策

1. **`IContributionHost::add`/`remove`** **为** **`&self`**：使用 `ObservableVec` 的 `RwLock` 内部可变性，允许 `&self` 安全 mutation。这是移除 `cx` 的前提——host 不需要 `entity.update(cx, ...)` 即可修改数据。
2. **`fn id(&self) -> &'static str`** **默认方法**：`const ID` 无法通过 `dyn IContributionHost` trait 对象访问。添加默认方法 `fn id(&self) -> &'static str { Self::ID }` 让 registry 在运行时通过 `host.id()` 获取 ID。这不违反"IContribution/IVisualContribution 签名禁止修改"的约束——`IContributionHost` 不在禁止修改范围内。
3. **`ObservableVec::notify`** **为** **`Option<Sender>`**：`None` 时（如 `Default::default()`）不发送通知。host 通过 `with_notifier(tx)` 创建带通知的实例。这允许 `ObservableVec` 在非 host 场景下使用（如纯数据结构）。
4. **不实现** **`DerefMut`**：强制通过 `push`/`retain` 等 mutation 方法修改，确保 version bump。`Deref` 到 `&[T]` 可选实现用于只读访问。
5. **`MainWindowHost`** **不实现** **`Render`**：host 是纯数据对象，不渲染 UI。Entity（`MainWindow`）实现 `Render`，在 `render` 中读取 host 数据。
6. **`cx: &mut App`** **保留在** **`__rml_register_<name>`** **生成函数签名中**：`get_contribution_registry(&self)` 需要 `&App`。仅移除传给 `register()` 的 `cx` 参数，函数签名本身不变。
7. **移除** **`register_host<T>(cx)`** **函数**：不再需要框架提供的 host 注册辅助函数。业务代码在 `on_loaded` 中直接 `cx.get_contribution_registry().add(Arc::new(host))`。

### 风险

1. **channel 通知时序**：`ObservableVec::push` 在 `host.add()` 中调用，channel 通知异步发送。后台任务 `cx.spawn` 接收后调用 `cx.notify()`。如果 host 在 Entity `on_loaded` 之前被注册（pending 重放），此时后台任务可能还未 spawn。**缓解**：`on_loaded` 中先创建 channel + spawn 任务，再 `registry.add(host)` 重放 pending。
2. **`flume`** **async 与 GPUI executor**：`cx.spawn(async move |this, cx| { rx.recv_async().await })` 需要 GPUI 的 async executor 支持 `flume` 的 async API。`flume` 基于 `futures`，与 GPUI 的 `Task` 兼容。
3. **`ObservableVec::read()`** **返回 Guard**：`RwLockReadGuard` 不能跨 `.await` 持有。`#[computed]` 方法中 `host.menu_entries.read()` 的 guard 在方法返回前释放，不跨 await。`active_case_view` 中需要先 clone 数据再释放 guard（如果调用 `visual.render()` 跨 await）。

