# 9.7 贡献点架构（Contribution System）

> 贡献点是**扩展注册表**，不是 Shell 框架。`rml_app` 只提供 registry / host / bootstrap；UI 映射与业务桥接在应用层。

## 框架提供什么

| API | 作用 |
|-----|------|
| `#[contributehost]` 宏 | 生成 `pub const ID: &'static str`（host_id 单一来源）|
| `ContributionStorage` | `Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>` —— Host 共享存储类型别名 |
| `IContributionHost`（默认 impl）| 框架为 `RwLock<Vec<...>>` 提供 impl，`entries.clone()` 经 unsized coercion 转为 `Arc<dyn IContributionHost>` |
| `IContributionRegistry::add / remove` | 注册/注销 Host（`Arc<dyn IContributionHost>`） |
| `IContributionRegistry::register / unregister` | 按 host_id 调 `host.add(c, opts)` / `host.remove(id)` 路由 |
| `IAppContextExt::register_host(id, host)` | 注册 Host 到 registry（`host: Arc<dyn IContributionHost>`） |
| `IAppContextExt::get_contribution_registry()` | 从 `App` / `Context` 获取 registry |
| `bootstrap_host_contributions(cx, host_id)` | 触发 build.rs 生成的 `#[contribute]` 批量注册 |

**不提供**：ActivityBar 映射、案例激活、菜单构建、树形 UI 适配、变更通知订阅。

## 应用层负责什么（demo 参考）

| 模块 | 职责 |
|------|------|
| `demo/shell/main_window.rml.rs` | `on_loaded` 中注册 host、投影到 ViewModel |
| `demo/shell/case_view_model.rs` | `IVisualContribution` → `CaseViewModel` 解包 |
| `demo/shell/menu_view_model.rs` | `ICommand` → `MenuViewModel` 解包 |
| `demo/shell/status_view_model.rs` | `IVisualContribution` → `StatusViewModel` 解包 |

## Host 注册流程

Host 持有 `entries: Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>` 共享存储字段。
框架为 `RwLock<Vec<...>>` impl `IContributionHost`（默认受理：`add` push 到 vec、`remove` 按 id 过滤），
故 `entries.clone()`（`Arc<RwLock<Vec<...>>>`）经 unsized coercion 转为 `Arc<dyn IContributionHost>` 注册到 registry。
Registry 调 `host.add(c, opts)` 路由贡献，不经 Entity 系统，避免 `on_loaded` 中的重入 panic。
需要自定义受理逻辑时，业务代码可为自身类型 impl `IContributionHost` 并注册 `Arc<dyn IContributionHost>`。

```rust
#[contributehost(id = "app.db")]
#[window]
pub struct DbProviderHost {
    entries: Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}

// 在 on_loaded 中注册：
cx.register_host(Self::ID, self.entries.clone()); // unsized coercion → Arc<dyn IContributionHost>
bootstrap_host_contributions(cx, DbProviderHost::ID); // 触发 #[contribute] 批量注册
```

## 带 ViewModel 投影的 Host（MainWindow）

```rust
#[contributehost(id = "demo.shell")]
#[window]
pub struct MainWindow { ... }
```

`on_loaded` 中：
1. `cx.register_host(Self::ID, self.entries.clone())` —— registry 存 `Arc<dyn IContributionHost>`
2. `bootstrap_host_contributions(cx, Self::ID)` 触发 `#[contribute]` 注册（`host.add` 写入 `entries`）
3. 从 `entries` 投影到 `cases / menus / status / activities` ViewModel 集合
4. RML 数据绑定自动驱动 UI

**投影什么、如何映射到 RML 字段， entirely 应用代码。**

## `#[contribute]`

```rust
#[contribute(host = MainWindow, id = "x", name = "...", slot = "menu")]
```

`slot` 语义由应用定义；demo 约定 `menu` / `activity` / `status` / `case`。

## 设计要点

- **`IContributionHost` 是有意义的抽象**：框架为共享存储 `RwLock<Vec<...>>` 提供默认 impl，业务代码亦可自定义受理逻辑（依赖倒置、开闭原则）
- **无中间 handle 包装层、无 `WeakEntity` 闭包**：`entries.clone()` 经 unsized coercion 转为 `Arc<dyn IContributionHost>` 注册到 registry
- **无变更通知**：Host 在 `on_loaded` 中投影后自行决定是否触发刷新（如 `cx.notify()`）
- **无 `bindings` 参数**：`#[contributehost]` 仅接受 `id`，ViewModel 投影由 `on_loaded` 手写
- **`bootstrap_host_contributions`**：build.rs 生成 `register_rml_contributions_for(cx, host_id)`，按 host_id 路由调用所有 `#[contribute(host_id = "...")]` 的 `__rml_register_*` 函数

## 视觉贡献 Entity 生命周期管理

`#[contribute]` + `#[component]` 叠加时，宏自动实现 `IVisual::render`，内部调用 `rml_app::contribution::get_or_create_entity::<T>(cx)` 复用 Entity。

**为什么需要**：`IVisualContribution::render` 每次渲染都会被调用。若每次创建新 Entity，`on_loaded` 会重复触发、内部状态丢失。`VisualEntityCache` 以 `TypeId` 为键存储强引用 `Entity<T>`，确保同一视觉贡献类型在应用生命周期内复用同一 Entity。

**语义边界**：
- `IContributionHost` 管"有哪些贡献"（注册数据）—— 框架不存储
- `VisualEntityCache` 管"视觉贡献的 Entity 不被重建"（生命周期）—— 框架管理
- 两者职责正交，`VisualEntityCache` 不是贡献注册缓存

**存储位置**：`ServiceCollection`（通过 `IAppContext::set_service` 注册），与 i18n/theme 范式对齐。
