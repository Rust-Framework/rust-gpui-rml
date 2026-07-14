//! 工作台管理器契约 —— 资源打开/关闭/激活的抽象接口
//!
//! 框架仅定义接口，由业务实现。manager 经 `rml_app::IAppContextExt` 安装到 App，
//! 通过 OnceLock 进程级槽位访问。所有方法 `&self` + 内部可变性，UI 相关工作由业务
//! 自行延迟到具备 `&mut App` 的时机处理（镜像 `IContributionHost` 模式）。
//!
//! 三接口分工：
//! - `IWorkbenchManager`：调度者。资源生命周期与激活态入口（`open`/`close`/`get`…）。
//! - `IWorkbench`：状态载体 + 元数据 + 视觉。继承 `IContribution + IVisual`
//!   （含 `id`/`name` 元数据 + `render` 视图),单个已打开资源的会话句柄。
//! - `IWorkbenchProvider`：视图工厂。按 Uri schema 注册，`render(uri)` 把资源构造成 `IWorkbench`。
//!
//! 流程：`Manager.open(uri)` → `uri.scheme()` → 业务自持 map 查 `Provider` →
//! `Provider.render(uri)` → `IWorkbench`。

use std::any::Any;
use std::sync::Arc;

use crate::contribution::{IContribution, IVisual};
use crate::value::IValue;
use gpui::SharedString;

/// Uri 类型：复用 `url::Url`。
pub use url::Url as Uri;

/// 工作台：一个已打开资源的会话句柄。
///
/// 继承 `IContribution + IVisual`——工作台同时是贡献(含 `id`/`name` 元数据)和视觉对象
/// (含 `render` 视图)。经 `as_visual()` 直接查询 `IVisual` 能力获取 render,
/// 无需再经 `as_contribution()` 中转。业务实现此 trait;实例由 `IWorkbenchManager::open` 返回。
/// `close`/`activate`/`set` 均为 `&self`——业务使用内部可变性,
/// 并将 cx 相关 UI 工作延迟到具备 `&mut App` 的时机（如宿主实体的 `on_loaded`/observe 回调）。
///
/// **设计理由**(WPF `ContentControl` 类比):workbench 本质是视觉的——已打开资源必然有视图。
/// 无视图的"后台任务"应实现 `IContribution` 而非 `IWorkbench`。
///
/// **Host 语义**:工作台是否受理子贡献(如 `IWorkbenchComponent`,实现编辑/预览/设计多态呈现)
/// 由**实现决定**——需要受理子组件的工作台(如 `EditorWorkbench`)直接 `impl IContributionHost`
/// 即可,无需在 trait 层强制。这与 `IContributionHost` 所有方法有默认空实现的设计一致。
pub trait IWorkbench: IContribution + IVisual {
    /// 此工作台的 Uri（唯一标识，用于去重与查找）。
    fn uri(&self) -> &str;

    /// 关闭此工作台。
    fn close(&self);

    /// 激活此工作台。
    fn activate(&self);

    /// 向此工作台设置数据（类型擦除值，业务按 key 自行 downcast）。
    fn set(&self, key: SharedString, value: Box<dyn Any + Send + Sync>);

    /// 此工作台对应的 Tab 是否允许关闭（显示关闭按钮）。
    /// 默认 `true`；欢迎页等常驻 Tab 可 override 返回 `false`。
    fn closable(&self) -> bool {
        true
    }

    /// 此工作台是否处于预览模式（VSCode 预览 Tab：italic 标题，双击升级为正式）。
    /// 默认 `false`；业务可经 [`Self::set_preview`] 切换。
    /// TabWindowShell 据此设置 TabItem.preview 渲染 italic 标题。
    fn preview(&self) -> bool {
        false
    }

    /// 切换预览模式状态。`&self` + 内部可变性（业务自行使用 `AtomicBool` 等）。
    /// 默认空实现；需要预览能力的业务 override。
    fn set_preview(&self, _preview: bool) {}
}

/// 工作台能力扩展 trait —— 让 `dyn IValue` 可查询 `IWorkbench` 能力。
///
/// 与 `VisualAbilityExt`/`ContributionAbilityExt` 模式一致。
/// 业务自定义工作台类型后，需调用 `register_workbench_ability::<T>()` 注册，
/// `as_workbench()` 查询即可生效，从而读取 `closable()` 等工作台专属信息。
pub trait WorkbenchAbilityExt {
    /// 若此值实现了 `IWorkbench`，返回引用；否则 `None`。
    fn as_workbench(&self) -> Option<&dyn IWorkbench>;
}

#[allow(unsafe_code)]
impl WorkbenchAbilityExt for dyn IValue {
    fn as_workbench(&self) -> Option<&dyn IWorkbench> {
        let erased = crate::ability::query::<dyn IWorkbench>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IWorkbench>(erased) })
    }
}

/// 为实现 `IWorkbench` 的类型注册能力 cast 函数。
///
/// `#[contribute]` 宏不自动注册此能力，业务需在初始化时手动调用。
/// 调用后，`as_workbench()` 查询生效，`TabWindowShell` 可据此读取 `closable()`。
#[allow(unsafe_code)]
pub fn register_workbench_ability<T: IWorkbench + 'static>() {
    crate::ability::register::<T, dyn IWorkbench>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let wb: &dyn IWorkbench = s;
            unsafe { crate::ability::erase(wb) }
        })
    });
}

/// 工作台管理器：资源的打开/关闭/查询。
///
/// 业务直接实现此 trait（如 `impl IWorkbenchManager for MainWindow`），
/// 用 `RwLock` 保护内部状态以支持 `&self` 方法。UI 刷新由调用方在 trait 方法返回后处理。
pub trait IWorkbenchManager: Send + Sync + 'static {
    /// 打开资源；若已打开则激活现有工作台。无法识别 URI 时返回 `None`。
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>>;

    /// 以预览模式打开资源（VSCode 风格：单击文件树预览，双击升级为正式）。
    ///
    /// 与 [`Self::open`] 的区别：
    /// - 若已有同资源的预览 Tab，复用之（不新建）
    /// - 新打开的工作台标记 `preview = true`（TabItem 显示 italic 标题）
    /// - 用户双击 Tab 或调用 [`Self::promote`] 升级为正式 Tab
    ///
    /// 默认实现退化为 [`Self::open`]（不区分预览/正式）。
    fn open_preview(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        self.open(uri)
    }

    /// 将预览 Tab 升级为正式 Tab（取消 preview 标记）。
    ///
    /// 默认空实现；需要预览能力的业务 override。
    fn promote(&self, _uri: &Uri) {}

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
/// 继承 `IContribution`(具备 `id`/`name` 元数据,可纳入贡献体系),但不继承 `IVisual`
/// ——此处的 `render` 不产出 UI 元素,而是**工厂方法**:给定资源 Uri,构造并返回对应的
/// `IWorkbench` 实例。`render` 方法名与 `IVisual::render` 同名但语义不同(工厂 vs 视觉),
/// 因 `IWorkbenchProvider` 不 impl `IVisual`,无 trait 方法冲突。
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
