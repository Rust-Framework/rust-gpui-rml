# Route B：能力扩展 trait 重构方案

## 摘要

将贡献点系统的"三入口"设计（`add`/`add_visual`/`add_command`、`register`/`register_visual`/`register_command`）收敛为**单一入口**：核心 trait 仅 `add(Arc<dyn IContribution>, Option<ContributionOptions>)`，所有能力查询（`ICommand`/`IVisualContribution`）通过 **extension trait + 全局能力注册表** 实现。框架内置 `CommandAbilityExt`/`VisualAbilityExt`；业务自定义能力 trait 时，自行编写等价 extension trait。

核心动机：开发者会定义成千上万个业务派生贡献点，框架不应在核心 trait 中枚举贡献类型。

## 设计澄清（用户反馈）

### 澄清 1：不需要 `as_any()` 方法 —— 直接用 trait upcast

**用户提问**：`fn as_any::<T>() -> &dyn T` 是否更简单？

**结论**：用户直觉正确（应简化），但具体形式不可行；有更简单的方案 —— **完全不需要** **`as_any()`** **方法**。

**分析**：

1. `fn as_any::<T>() -> &dyn T` 是泛型方法，**不能通过** **`dyn IContribution`** **调用** —— trait object 的 vtable 必须固定，泛型方法要求每个 monomorphization 对应一个 vtable 条目，违反 object safety。Rust 类型系统根本禁止此模式。

2. 原 plan 中的 `fn as_any(&self) -> &dyn Any { self }` 是 mopa 模式的传统写法（在 trait upcasting 稳定前的 workaround）。本项目已使用 trait upcasting（见 `crates/core/src/command.rs` 测试 `relay_command_as_arc_dyn_icontribution_via_upcast`，`Arc<dyn ICommand>` → `Arc<dyn IContribution>` 工作正常）。

3. `IContribution: Send + Sync + Any` —— `Any` 已是显式 supertrait。trait upcasting 允许 `&dyn IContribution` 直接 coerce 到 `&dyn Any`，无需任何方法。

**最终方案**：

* `IContribution` trait **不添加** **`as_any()`** **方法**（保持现状）

* 宏生成的 cast\_fn 内部直接 trait upcast：

```rust
|c: &dyn IContribution| {
    let any: &dyn Any = c;  // trait upcast（IContribution: Any）
    any.downcast_ref::<Self>().map(|s| {
        let cmd: &dyn ICommand = s;
        unsafe { rml_core::ability::erase(cmd) }
    })
}
```

比原 plan 的 `c.as_any().downcast_ref::<Self>()` 更简单，且不污染 `IContribution` trait API。

### 澄清 2：`ContributionOptions` 改为 `Option<ContributionOptions>`

**用户指示**：`add` 中的 `ContributionOptions` 应该是可选的。

**方案**：所有 `add`/`register` 签名改为 `Option<ContributionOptions>`。

* 宏生成的 `__rml_register_*` 始终传 `Some(opts)`（宏从属性构建 options，总有值）

* 编程式调用（host 直接 `add`、测试代码）可传 `None` 表示"无元数据"

* Host 实现中 `options.unwrap_or_default()` 还原为 `ContributionOptions`

**影响范围**：`IContributionHost::add`、`IContributionRegistry::register`、`HostOp::Add`、宏生成代码、所有 host 实现。

## 当前状态分析

### 三入口分布（待消除）

| 文件                                           | 三入口痕迹                                                                                                                      |
| -------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `crates/core/src/contribution.rs`            | `IContributionHost::{add, add_visual, add_command}`、`IContributionRegistry::{register, register_visual, register_command}` |
| `crates/app/src/contribution/host_handle.rs` | `HostOp::{Add, AddVisual, AddCommand}` 三变体                                                                                 |
| `crates/app/src/contribution/registry.rs`    | `ContributionRegistry` 实现 3 个 register 方法                                                                                  |
| `crates/macros/src/contribute.rs`            | 宏按 `command`/`visual` flag 路由到 3 个不同 register 调用（L262-298）                                                                 |
| `demo/src/shell/main_window.rml.rs`          | 3 个存储桶：`entries`/`visual_entries`/`command_entries`；3 个 add 重载                                                             |
| `demo/src/shell/activity_panel.rml.rs`       | 仅 override `add_visual`（selective override 模式）                                                                             |
| `demo/src/shell/shell_chrome.rs`             | `map_menu_items(entries, commands)` 双入参；3 个类型别名                                                                            |
| `crates/ui/src/components/menu.rs`           | `IMenuItem::command() -> Option<Arc<dyn ICommand>>`                                                                        |

### 关键约束（已确认）

* **核心 trait 不可枚举贡献类型**：`IContributionHost`/`IContributionRegistry` 仅暴露 `add`/`register` 单方法

* **能力扩展经 extension trait**：内置 `ICommand`/`IVisualContribution` 由框架提供 `as_command()`/`as_visual()`，业务自定义能力自行扩展

* **Menu 改为存** **`Arc<dyn IContribution>`**（用户已确认）：`IMenuItem::command()` → `contribution()`，on\_click 内 `c.as_command().execute(...)`

* **接受 unsafe transmute，封装在 core 内部**（用户已确认）：`#[allow(unsafe_code)]` 局部放开

* **不需要** **`as_any()`** **方法**（澄清 1）：直接用 trait upcast

* **`ContributionOptions`** **可选**（澄清 2）：`Option<ContributionOptions>`

### Rust 类型系统约束（已研究）

* `Any::downcast_ref::<T>` 要求 `T: Sized`，**不能 downcast 到** **`dyn Trait`** —— 这是 `switch(c) { case dyn ICommand }` 直接实现的根本障碍

* `TypeId::of::<dyn Trait>()` 在 `Trait: 'static` 时合法 —— 每个 trait 的 `dyn Trait` 有唯一 TypeId，可用作能力注册表 key

* `&dyn Trait` 是 fat pointer（data + vtable），可 `transmute` 为 `[*const (); 2]` 再还原 —— mopa 模式的核心机制

* trait upcasting（Rust 1.86+ 稳定，本项目 nightly）：`&dyn IContribution` → `&dyn Any` 直接 coerce（因 `IContribution: Any`）

* `impl CommandAbilityExt for dyn IContribution` 合法 Rust —— 可对 trait object 添加扩展方法

## 提议变更

### 1. 新增 `crates/core/src/ability.rs`（能力查询基础设施）

**职责**：全局能力注册表 + fat pointer 还原。封装所有 unsafe，对外提供安全 API。

**核心 API**：

```rust
//! 能力查询基础设施 —— mopa 模式实现 trait object 间 downcast。
//!
//! 核心思路：
//! - trait upcasting：`&dyn IContribution` 直接 coerce 到 `&dyn Any`（因 `IContribution: Any`）
//! - `downcast_ref::<T>()` 还原具体类型，再 trait upcast 到 `&dyn Ability`
//! - 全局 `HashMap<(TypeId, TypeId), CastFn>` 按 (concrete_type_id, ability_trait_id) 索引 cast 函数
//! - cast 函数 transmute `&dyn Ability` 为 `ErasedAbility`（擦除 fat pointer）
//! - `restore::<A>` 将 `ErasedAbility` 还原为 `&dyn A`
//!
//! unsafe 仅存在于 `erase`/`restore` 两个函数，封装在本模块内。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::RwLock;

use crate::contribution::IContribution;

/// 擦除后的能力 fat pointer（data + vtable）。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ErasedAbility {
    data: *const (),
    vtable: *const (),
}

/// cast 函数类型：`&dyn IContribution` → `Option<ErasedAbility>`。
pub type CastFn = fn(&dyn IContribution) -> Option<ErasedAbility>;

static ABILITY_REGISTRY: RwLock<HashMap<(TypeId, TypeId), CastFn>> =
    RwLock::new(HashMap::new());

/// 注册能力 cast 函数（幂等，重复注册同一 key 等价于覆盖）。
///
/// 由 `#[contribute]` 宏在 `__rml_register_*` 中调用。
pub fn register<T: 'static, A: ?Sized + 'static>(cast_fn: CastFn) {
    let key = (TypeId::of::<T>(), TypeId::of::<A>());
    ABILITY_REGISTRY.write().unwrap().insert(key, cast_fn);
}

/// 查询能力：返回擦除后的 fat pointer，由 `restore::<A>` 还原。
pub fn query<A: ?Sized + 'static>(c: &dyn IContribution) -> Option<ErasedAbility> {
    // trait upcast：&dyn IContribution → &dyn Any（因 IContribution: Any）
    let any: &dyn Any = c;
    let concrete_id = any.type_id();
    let ability_id = TypeId::of::<A>();
    let registry = ABILITY_REGISTRY.read().unwrap();
    let cast_fn = registry.get(&(concrete_id, ability_id))?;
    cast_fn(c)
}

/// 擦除：`&dyn A` → `ErasedAbility`。
///
/// # Safety
/// `a` 必须是合法的 `&dyn A` fat pointer。`A: 'static` 保证 vtable 静态有效。
#[allow(unsafe_code)]
pub unsafe fn erase<A: ?Sized + 'static>(a: &A) -> ErasedAbility {
    let ptr: *const A = a;
    // fat pointer (data + vtable) 与 ErasedAbility 布局一致
    unsafe { std::mem::transmute_copy::<*const A, ErasedAbility>(&ptr) }
}

/// 还原：`ErasedAbility` → `&dyn A`。
///
/// # Safety
/// `erased` 必须由 `erase::<A>` 产生，且 `A` 与还原目标一致。
#[allow(unsafe_code)]
pub unsafe fn restore<'a, A: ?Sized + 'static>(erased: ErasedAbility) -> &'a A {
    let ptr: *const A = unsafe { std::mem::transmute_copy::<ErasedAbility, *const A>(&erased) };
    unsafe { &*ptr }
}
```

**注意**：`IContribution` trait **不添加** **`as_any()`** **方法**（见澄清 1）。`query` 函数内部直接用 trait upcast `let any: &dyn Any = c;`。

### 2. 新增 `crates/core/src/command.rs` 中的 `CommandAbilityExt`

**职责**：为 `dyn IContribution` 添加 `as_command()` 方法，封装 `query::<dyn ICommand>` + `restore`。

```rust
/// 命令能力扩展 trait —— 让 `dyn IContribution` 可查询 `ICommand` 能力。
///
/// 框架内置：`#[contribute(command, ...)]` 宏自动注册能力 cast 函数。
/// 业务自定义能力 trait 时，参考此模式编写等价 extension trait。
pub trait CommandAbilityExt {
    /// 若此贡献实现了 `ICommand`，返回命令引用；否则 `None`。
    fn as_command(&self) -> Option<&dyn ICommand>;
}

impl CommandAbilityExt for dyn IContribution {
    fn as_command(&self) -> Option<&dyn ICommand> {
        let erased = crate::ability::query::<dyn ICommand>(self)?;
        unsafe { crate::ability::restore::<dyn ICommand>(erased) }
    }
}
```

### 3. 新增 `crates/core/src/contribution.rs` 中的 `VisualAbilityExt`

**职责**：为 `dyn IContribution` 添加 `as_visual()` 方法。

```rust
/// 视觉能力扩展 trait —— 让 `dyn IContribution` 可查询 `IVisualContribution` 能力。
pub trait VisualAbilityExt {
    fn as_visual(&self) -> Option<&dyn IVisualContribution>;
}

impl VisualAbilityExt for dyn IContribution {
    fn as_visual(&self) -> Option<&dyn IVisualContribution> {
        let erased = crate::ability::query::<dyn IVisualContribution>(self)?;
        unsafe { crate::ability::restore::<dyn IVisualContribution>(erased) }
    }
}
```

### 4. 重构 `crates/core/src/contribution.rs`：收敛 `IContributionHost`/`IContributionRegistry`

**`IContribution`**：保持现状（不添加 `as_any()`）。

**`IContributionHost`**（移除 `add_visual`/`add_command`，options 改为 `Option`）：

```rust
pub trait IContributionHost: Send + Sync + 'static {
    fn id(&self) -> &'static str;
    /// 受理贡献（统一入口）。host 自行决定如何存储/分发。
    /// host 可通过 `c.as_command()`/`c.as_visual()` 查询贡献能力并分类存储。
    /// `options` 为 `None` 时表示无元数据（order/group/kind 等），host 可按 `ContributionOptions::default()` 处理。
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: Option<ContributionOptions>) {}
    fn remove(&self, _contribution_id: &str) {}
}
```

**`IContributionRegistry`**（移除 `register_visual`/`register_command`，options 改为 `Option`）：

```rust
pub trait IContributionRegistry: Send + Sync {
    fn add_host(&self, host: Arc<dyn IContributionHost>);
    fn remove_host(&self, host_id: &str);
    /// 注册贡献（统一入口）。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    );
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
```

### 5. 重构 `crates/app/src/contribution/host_handle.rs`：`HostOp` 收敛

```rust
pub enum HostOp {
    Add(Arc<dyn IContribution>, Option<ContributionOptions>),
    Remove(String),
}

impl<T: 'static> IContributionHost for EntityHostHandle<T> {
    fn id(&self) -> &'static str { self.id }
    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let _ = self.tx.send(HostOp::Add(contribution, options));
    }
    fn remove(&self, contribution_id: &str) {
        let _ = self.tx.send(HostOp::Remove(contribution_id.to_string()));
    }
}

pub fn drain_host_ops<T: IContributionHost>(rx: &flume::Receiver<HostOp>, host: &T) {
    for op in rx.try_iter() {
        match op {
            HostOp::Add(c, o) => host.add(c, o),
            HostOp::Remove(id) => host.remove(&id),
        }
    }
}
```

**移除 import**：`ICommand`/`IVisualContribution` 不再需要。

### 6. 重构 `crates/app/src/contribution/registry.rs`：单 `register`

```rust
impl IContributionRegistry for ContributionRegistry {
    fn add_host(&self, host: Arc<dyn IContributionHost>) { /* unchanged */ }
    fn remove_host(&self, host_id: &str) { /* unchanged */ }

    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    ) {
        let hosts = self.hosts.read().unwrap();
        if let Some(host) = hosts.get(host_id) {
            host.add(contribution, options);
        } else {
            let _ = (host_id, contribution, options);
        }
    }

    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool { /* unchanged */ }
}
```

**移除 import**：`ICommand`/`IVisualContribution`。

### 7. 重构 `crates/macros/src/contribute.rs`：统一 `register` + 生成 `register_ability`

**关键变更**：

* `register_call` 始终调用 `register`（移除三分支路由），options 包裹 `Some(...)`

* 在 `__rml_register_*` 函数体顶部生成 `register_ability` 调用（按 flag）：

  * `command` flag → 注册到 `dyn ICommand` 能力

  * `visual` flag（或 `#[component]` 叠加）→ 注册到 `dyn IVisualContribution` 能力

* cast\_fn 内部用 trait upcast（不调 `as_any()`）

**生成代码示例**（`MenuFileNew`，`command` flag）：

```rust
pub fn __rml_register_menufilenew(cx: &mut gpui::App) {
    use rml_app::contribution::ContributionRegistryExt;
    // 1. 注册能力（幂等）—— trait upcast + downcast_ref，无需 as_any()
    rml_core::ability::register::<MenuFileNew, dyn rml_core::command::ICommand>(
        |c| {
            let any: &dyn std::any::Any = c;  // trait upcast
            any.downcast_ref::<MenuFileNew>().map(|s| {
                let cmd: &dyn rml_core::command::ICommand = s;
                unsafe { rml_core::ability::erase(cmd) }
            })
        },
    );
    // 2. 注册贡献（统一入口，options 包裹 Some）
    cx.get_contribution_registry().register(
        "demo.shell",
        std::sync::Arc::new(MenuFileNew::default()),
        Some(
            rml_core::contribution::ContributionOptions::new()
                .parent_id("menu.file").order(0).property("kind", "menu"),
        ),
    );
}
```

**视觉贡献示例**（`StatusBarCase`，`visual` flag via `#[component]`）：

```rust
rml_core::ability::register::<StatusBarCase, dyn rml_core::contribution::IVisualContribution>(
    |c| {
        let any: &dyn std::any::Any = c;
        any.downcast_ref::<StatusBarCase>().map(|s| {
            let v: &dyn rml_core::contribution::IVisualContribution = s;
            unsafe { rml_core::ability::erase(v) }
        })
    },
);
```

**纯元数据贡献**（如 `MenuFileRoot`，无 flag）：仅生成 `register` 调用，无 `register_ability`。

**实现要点**：

```rust
let ability_registrations = if use_command {
    quote! {
        rml_core::ability::register::<#struct_name, dyn rml_core::command::ICommand>(
            |c| {
                let any: &dyn std::any::Any = c;
                any.downcast_ref::<#struct_name>().map(|s| {
                    let cmd: &dyn rml_core::command::ICommand = s;
                    unsafe { rml_core::ability::erase(cmd) }
                })
            },
        );
    }
} else { quote!{} };

let visual_ability_registration = if use_visual {
    quote! {
        rml_core::ability::register::<#struct_name, dyn rml_core::contribution::IVisualContribution>(
            |c| {
                let any: &dyn std::any::Any = c;
                any.downcast_ref::<#struct_name>().map(|s| {
                    let v: &dyn rml_core::contribution::IVisualContribution = s;
                    unsafe { rml_core::ability::erase(v) }
                })
            },
        );
    }
} else { quote!{} };

let register_call = quote! {
    cx.get_contribution_registry().register(
        #host_id,
        std::sync::Arc::new(#struct_name::default()),
        Some(
            rml_core::contribution::ContributionOptions::new()
                #parent_id #order #group #properties_tokens,
        ),
    );
};

quote! {
    #(#items)*
    impl #struct_name { pub const CONTRIBUTION_ID: &'static str = #id; }
    const _: () = { /* assert IContribution */ };
    #command_assert
    #visual_impl

    pub fn #register_fn(cx: &mut gpui::App) {
        use rml_app::contribution::ContributionRegistryExt;
        #ability_registrations
        #visual_ability_registration
        #register_call
    }
}
```

**注意**：`command` 与 `visual` 可同时为 true（一个贡献既是命令又是视觉，罕见但允许）—— 故分开两个 registration 块而非 `else if`。

### 8. 重构 `demo/src/shell/main_window.rml.rs`：单一存储桶

```rust
pub struct MainWindow {
    // ...其他字段不变
    // 单一存储桶：所有贡献（菜单根/叶子命令/状态栏/案例/活动栏）
    entries: std::sync::RwLock<Vec<ContribEntry>>,
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl IContributionHost for MainWindow {
    fn id(&self) -> &'static str { Self::ID }
    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let opts = options.unwrap_or_default();
        self.entries.write().unwrap().push((contribution, opts));
    }
    fn remove(&self, contribution_id: &str) {
        self.entries.write().unwrap().retain(|(c, _)| c.id() != contribution_id);
    }
}

impl MainWindow {
    fn refresh_shell_chrome(&mut self) {
        let entries = self.entries.read().unwrap();
        self.status_items = map_status_items(&entries);
        self.menu_items = map_menu_items(&entries);
    }

    pub fn active_case_view(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
        let entries = self.entries.read().unwrap();
        if let Some((c, _)) = entries.iter().find(|(c, _)| c.id() == self.active_case_id) {
            if let Some(visual) = c.as_visual() {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }
}
```

`on_loaded` 中构建 ActivityBar 改用统一 entries：

```rust
let panels = {
    let entries = self.entries.read().unwrap();
    build_activity_panels_from(&entries)
};
```

**移除 import**：`CommandEntry`/`VisualEntry` 不再需要；引入 `ContribEntry`。

### 9. 重构 `demo/src/shell/activity_panel.rml.rs`：单一存储桶

```rust
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    case_entries: std::sync::RwLock<Vec<ContribEntry>>,  // 改为 ContribEntry
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl IContributionHost for ActivityPanel {
    fn id(&self) -> &'static str { Self::ID }
    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let opts = options.unwrap_or_default();
        self.case_entries.write().unwrap().push((contribution, opts));
    }
    fn remove(&self, contribution_id: &str) {
        self.case_entries.write().unwrap().retain(|(c, _)| c.id() != contribution_id);
    }
}
```

### 10. 重构 `demo/src/shell/shell_chrome.rs`：统一列表 + `as_command()`/`as_visual()` 过滤

**`ContribEntry`** **类型**（options 不再是 `Option`，host 已 unwrap）：

```rust
pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);
```

**移除**：`VisualEntry`/`CommandEntry` 类型别名。

**`map_menu_items`**（单入参，按 `as_command()` 区分叶子）：

```rust
pub fn map_menu_items(entries: &[ContribEntry]) -> MenuItems {
    let mut all: Vec<MenuNode> = Vec::new();
    for (c, o) in entries.iter().filter(|(_, o)| o.effective_slot() == Some("menu")) {
        all.push(MenuNode {
            id: c.id().to_string(),
            name: c.name(),
            order: o.order,
            parent_id: o.parent_id.as_ref().map(|s| s.to_string()),
            // 关键：通过 as_command() 判定是否为叶子命令
            contribution: c.as_command().map(|_| c.clone()),
        });
    }
    // ... 建树逻辑不变（MenuNode.command 字段改为 contribution: Option<Arc<dyn IContribution>>）
}
```

**`map_case_tree_items`**（单入参，按 `as_visual()` 过滤）：

```rust
pub fn map_case_tree_items(entries: &[ContribEntry]) -> Vec<TreeItem> {
    let entries: Vec<&ContribEntry> = entries.iter()
        .filter(|(c, o)| o.effective_slot() == Some("case") && c.as_visual().is_some())
        .collect();
    // ... 后续建树逻辑不变
}
```

**`build_activity_panels_from`**（单入参，按 `as_visual()` 过滤）：

```rust
pub fn build_activity_panels_from(entries: &[ContribEntry]) -> ActivityPanels {
    let mut panels: Vec<&ContribEntry> = entries.iter()
        .filter(|(c, o)| o.effective_slot() == Some("activity") && c.as_visual().is_some())
        .collect();
    panels.sort_by_key(|(_, o)| o.order);
    panels.into_iter()
        .filter_map(|(c, _)| {
            VisualActivityPanel::new(c.clone())  // 改为接收 Arc<dyn IContribution>
                .map(|p| Arc::new(p) as Arc<dyn rml_ui::IActivityPanel>)
        })
        .collect()
}
```

**`VisualActivityPanel::new`** **签名调整**：从 `Arc<dyn IVisualContribution>` 改为 `Arc<dyn IContribution>`。`Arc<dyn Trait>` 无法 downcast，故 `VisualActivityPanel` 内部存储 `Arc<dyn IContribution>`，`panel()` 实现中 `self.contrib.as_visual().render(...)`。

**`map_status_items`**（单入参，无变化，仍按 `kind="status"` 过滤）。

### 11. 重构 `crates/ui/src/components/menu.rs`：`command()` → `contribution()`

**`IMenuItem`** **trait**：

```rust
pub trait IMenuItem: Send + Sync + 'static {
    // ... 其他方法不变
    fn contribution(&self) -> Option<Arc<dyn rml_core::contribution::IContribution>> { None }
    // 移除 fn command(&self) -> Option<Arc<dyn ICommand>>;
}
```

**`MenuItem`** **struct**：

```rust
pub struct MenuItem {
    // ...
    contribution: Option<Arc<dyn rml_core::contribution::IContribution>>,
    // 移除 command 字段
}

impl MenuItem {
    pub fn contribution(mut self, c: Arc<dyn rml_core::contribution::IContribution>) -> Self {
        self.contribution = Some(c);
        self
    }
    // 移除 pub fn command(...)
}
```

**on\_click 处理**（两处：MenuBar L330、PopupMenu L412）：

```rust
if let Some(c) = contribution {
    btn = btn.on_click(move |_, _window, cx| {
        let contrib: &dyn rml_core::contribution::IContribution = &*c;
        if let Some(cmd) = contrib.as_command() {
            cmd.execute(&mut CallContext::new(_window, cx));
        }
    });
}
```

**需要引入**：`use rml_core::contribution::{IContribution, CommandAbilityExt};`（`CommandAbilityExt` 使 `as_command` 可用）。

**声明式 codegen 路径**（如 `compiler/menu/` 中生成 `MenuItem::command(...)` 调用的地方）：需同步改为 `.contribution(...)`。需 grep 确认所有调用点。

### 12. 更新 `crates/core/src/lib.rs` 与 prelude

**`lib.rs`**：

```rust
pub mod ability;  // 新增
```

**`crates/core/src/prelude.rs`**：

```rust
pub use crate::ability::{erase, query, register, ErasedAbility};  // 按需
pub use crate::command::CommandAbilityExt;  // 新增
pub use crate::contribution::VisualAbilityExt;  // 新增
```

### 13. 文档注释更新

* `crates/core/src/contribution.rs` 顶部注释：删除"视觉贡献通过独立的 `register_visual` 路径"描述

* `crates/core/src/command.rs` 顶部注释：删除"命令贡献经 `register_command` 路由"

* `crates/macros/src/contributehost.rs` 顶部注释：删除"override `add`/`add_visual`/`remove`"改为"override `add`/`remove`"

* `crates/macros/src/contribute.rs` 顶部注释：删除"按 `command`/`visual` flag 路由到 `register`/`register_command`/`register_visual`"

## 假设与决策

1. **不添加** **`as_any()`** **方法**（澄清 1）：`IContribution: Any` 已是 supertrait，trait upcasting（Rust 1.86+，本项目 nightly 已用）允许 `&dyn IContribution` 直接 coerce 到 `&dyn Any`。宏 cast\_fn 内 `let any: &dyn Any = c;` + `any.downcast_ref::<Self>()`。比 mopa 传统 `as_any()` 更简洁，不污染 trait API。
2. **`Option<ContributionOptions>`**（澄清 2）：宏始终传 `Some(opts)`；编程式调用可传 `None`。Host impl 中 `options.unwrap_or_default()` 还原。`ContribEntry` 存储已 unwrap 的 `ContributionOptions`（host 入口处统一处理）。
3. **能力注册时机**：在 `__rml_register_*` 函数体顶部调用 `register_ability`，先于 `register`。`register` 触发 host `add`，host `add` 内可能立即 `as_command()` 查询 —— 此时能力必须已注册。此顺序保证正确性。
4. **能力注册幂等性**：`HashMap::insert` 同 key 同值覆盖，无副作用。
5. **`as_command()`** **返回** **`Option<&dyn ICommand>`**：生命周期与 `&self` 绑定。`Arc<dyn IContribution>` 持有时，先 deref 得到 `&dyn IContribution`，再调 `as_command()`，返回引用与 Arc 借用绑定。
6. **`VisualActivityPanel::new`** **签名调整**：从 `Arc<dyn IVisualContribution>` 改为 `Arc<dyn IContribution>`，内部按需 `as_visual()`。`Arc<dyn Trait>` 无法 downcast，这是必要调整。
7. **unsafe 边界**：`erase`/`restore` 是仅有的 unsafe 函数，封装在 `ability.rs` 内，对外通过 `CommandAbilityExt`/`VisualAbilityExt` 提供 safe API。`#[allow(unsafe_code)]` 局部放开，符合 `lib.rs` 的 `#![deny(unsafe_code)]` 策略（与 `ComputedCache` 一致）。
8. **不修改** **`crates/lsp`**：其编译错误为既有问题（lsp-server 0.7.9 Sender privacy），与本次重构无关。
9. **不修改** **`crates/app`** **的 doctest 失败**：Windows nightly toolchain 缺 rustdoc，环境问题。

## 验证步骤

### 验证 1：单元编译

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
cargo build -p rust-rml-core
cargo build -p rust-rml-app
cargo build -p rust-rml-macros
cargo build -p rust-rml-ui
cargo build -p rust-rml-demo
```

预期：全部成功（可能有 unused import 警告，需清理）。

### 验证 2：测试套件

```powershell
cargo test -p rust-rml-core
cargo test -p rust-rml-app
cargo test -p rust-rml-engine
cargo test -p rust-rml-demo
```

预期：所有既有测试通过（348+ 测试）。

### 验证 3：能力查询功能验证

新增 `crates/core/src/ability.rs` 单元测试：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CallContext, ICommand};
    use crate::contribution::IContribution;
    use crate::command::CommandAbilityExt;
    use gpui::SharedString;

    struct TestCmd;
    impl IContribution for TestCmd {
        fn id(&self) -> &str { "test.cmd" }
        fn name(&self) -> SharedString { "test".into() }
    }
    impl ICommand for TestCmd {
        fn execute(&self, _ctx: &mut CallContext) {}
    }

    #[test]
    fn ability_query_returns_some_when_registered() {
        register::<TestCmd, dyn ICommand>(|c| {
            let any: &dyn std::any::Any = c;
            any.downcast_ref::<TestCmd>().map(|s| {
                let cmd: &dyn ICommand = s;
                unsafe { erase(cmd) }
            })
        });
        let cmd = TestCmd;
        let c: &dyn IContribution = &cmd;
        assert!(c.as_command().is_some());
    }

    #[test]
    fn ability_query_returns_none_when_not_registered() {
        struct Unregistered;
        impl IContribution for Unregistered {
            fn id(&self) -> &str { "unreg" }
            fn name(&self) -> SharedString { "u".into() }
        }
        let u = Unregistered;
        let c: &dyn IContribution = &u;
        assert!(c.as_command().is_none());
    }
}
```

### 验证 4：demo 端到端运行

```powershell
cargo run -p rust-rml-demo
```

预期：窗口启动，菜单栏 File/View/Help 完整，点击叶子菜单项触发命令（如 File → New 打开 welcome tab），ActivityBar 显示案例树，点击案例切换视图。

### 验证 5：Clippy 无警告

```powershell
cargo clippy -p rust-rml-core -- -D warnings
cargo clippy -p rust-rml-demo -- -D warnings
```

## 实施顺序（建议 TodoList）

1. 新建 `crates/core/src/ability.rs`（基础设施 + 单测）
2. 修改 `crates/core/src/lib.rs`：`pub mod ability;`
3. 修改 `crates/core/src/contribution.rs`：加 `VisualAbilityExt`、收敛 `IContributionHost`/`IContributionRegistry`（`Option<ContributionOptions>`）
4. 修改 `crates/core/src/command.rs`：加 `CommandAbilityExt`、更新注释
5. 修改 `crates/core/src/prelude.rs`：导出扩展 trait
6. 修改 `crates/app/src/contribution/host_handle.rs`：`HostOp` 收敛 + `Option<ContributionOptions>`
7. 修改 `crates/app/src/contribution/registry.rs`：单 `register` + `Option<ContributionOptions>`
8. 修改 `crates/macros/src/contribute.rs`：统一 `register` + 生成 `register_ability`（trait upcast 版本）
9. 修改 `crates/macros/src/contributehost.rs`：更新注释
10. 修改 `crates/ui/src/components/menu.rs`：`command()` → `contribution()`
11. 修改 `demo/src/shell/shell_chrome.rs`：统一列表 + `as_command()`/`as_visual()` 过滤
12. 修改 `demo/src/shell/main_window.rml.rs`：单一存储桶 + `Option<ContributionOptions>` 处理
13. 修改 `demo/src/shell/activity_panel.rml.rs`：单一存储桶 + `Option<ContributionOptions>` 处理
14. 修改 `demo/src/shell/menu_shell_contribs.rs`：无变更（`#[contribute(command, ...)]` 不变），验证编译
15. Grep `compiler/menu/` 等声明式 codegen 路径，同步 `MenuItem::command` → `contribution` 调用
16. 运行验证 1-5

## 风险与回滚

* **风险点**：`as_command()` 在闭包中调用时，返回的 `&dyn ICommand` 借用自 `&*arc`，闭包捕获 `Arc` 后借用有效。需确认 GPUI `on_click` 闭包签名允许此模式。

* **风险点**：trait upcast `&dyn IContribution` → `&dyn Any` 在 nightly 上的可用性。已由 `relay_command_as_arc_dyn_icontribution_via_upcast` 测试间接验证（`Arc<dyn ICommand>` → `Arc<dyn IContribution>` 工作），`&dyn IContribution` → `&dyn Any` 同理。

* **回滚策略**：所有变更集中在 \~10 个文件，git 可单 commit 回滚。`ability.rs` 为新增文件，删除即可。

