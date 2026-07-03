//! 工作台管理器契约 —— 资源打开/关闭/激活的抽象接口
//!
//! 框架仅定义接口，由业务实现。manager 经 `rml_app::WorkbenchManagerExt` 安装到 App，
//! 通过 OnceLock 进程级槽位访问。所有方法 `&self` + 内部可变性，UI 相关工作由业务
//! 自行延迟到具备 `&mut App` 的时机处理（镜像 `IContributionHost` 模式）。
//!
//! 三接口分工：
//! - `IWorkbenchManager`：调度者。资源生命周期与激活态入口（`open`/`close`/`get`…）。
//! - `IWorkbench`：状态载体。单个已打开资源的会话句柄（`close`/`activate`/`set`）。
//! - `IWorkbenchProvider`：视图工厂。按 Uri schema 注册，`render(uri)` 把资源构造成 `IWorkbench`。
//!
//! 流程：`Manager.open(uri)` → `uri.scheme()` → 业务自持 map 查 `Provider` →
//! `Provider.render(uri)` → `IWorkbench`。

use std::any::Any;
use std::sync::Arc;

use crate::contribution::IContribution;
use gpui::SharedString;

/// Uri 类型：复用 `url::Url`。
pub use url::Url as Uri;

/// 工作台：一个已打开资源的会话句柄。
///
/// 业务实现此 trait；实例由 `IWorkbenchManager::open` 返回。
/// `close`/`activate`/`set` 均为 `&self`——业务使用内部可变性，
/// 并将 cx 相关 UI 工作延迟到具备 `&mut App` 的时机（如宿主实体的 `on_loaded`/observe 回调）。
/// 加 `Any` supertrait——与 `IContribution` 一致，便于业务按需 downcast 到具体工作台类型。
pub trait IWorkbench: Send + Sync + Any {
    /// 关闭此工作台。
    fn close(&self);

    /// 激活此工作台。
    fn activate(&self);

    /// 向此工作台设置数据（类型擦除值，业务按 key 自行 downcast）。
    fn set(&self, key: SharedString, value: Box<dyn Any + Send + Sync>);
}

/// 工作台管理器：资源的打开/关闭/查询。
///
/// 业务实现并经 `rml_app::WorkbenchManagerExt::set_workbench_manager` 安装。
/// 所有方法 `&self`——业务用 `RwLock`/channel 等内部可变性，UI 工作延迟处理。
pub trait IWorkbenchManager: Send + Sync + 'static {
    /// 打开资源；若已打开则激活现有工作台并返回。
    fn open(&self, uri: &Uri) -> Arc<dyn IWorkbench>;

    /// 关闭资源对应的工作台。
    fn close(&self, uri: &Uri);

    /// 当前所有已打开的工作台。
    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>>;

    /// 当前激活的工作台。
    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>>;

    /// 按 Uri 获取工作台。
    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>>;
}

/// 工作台提供程序：按 Uri schema 注册的资源→工作台工厂。
///
/// 继承 `IContribution`（具备 `id`/`name` 元数据，可纳入贡献体系），但不继承
/// `IVisualContribution`——此处的 `render` 不产出 UI 元素，而是**工厂方法**：
/// 给定资源 Uri，构造并返回对应的 `IWorkbench` 实例。
///
/// 框架不提供 schema 路由注册表——业务在 `IWorkbenchManager` 实现中自行维护
/// `schema -> Arc<dyn IWorkbenchProvider>` 映射，按 `uri.scheme()` 查表后调用 `render`。
/// 这与 `IContributionHost` 的「业务自受理」范式一致。
///
/// 流程：`Manager.open(uri)` → `uri.scheme()` → 业务 map 查 `Provider` →
/// `Provider.render(uri)` → `IWorkbench`。
pub trait IWorkbenchProvider: IContribution {
    /// 此提供程序处理的 Uri schema（如 `"file"`、`"lsp"`）。
    fn schema(&self) -> SharedString;

    /// 渲染资源为工作台：给定 Uri，构造并返回对应的 `IWorkbench` 实例。
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench>;
}
