//! 贡献点契约 —— 能力贡献 / 视觉能力 / 可视化贡献 / 受理方 / 桥接注册表
//!
//! 框架不存储贡献数据——host 主动受理（`add`/`remove`），registry 仅按 `host_id` 路由。
//! 能力查询（`ICommand`/`IVisual`/`IContribution`）经 `*AbilityExt`
//! extension trait 实现，核心 trait 不枚举贡献类型。
//!
//! Trait 关系：
//! - `IContribution: IValue`——贡献是值对象的特化（仅元数据）
//! - `IVisual: IValue`——视觉能力（任何可渲染为 UI 元素的值对象）
//! - `IVisualContribution: IContribution + IVisual`——可视化贡献（标记 trait,blanket impl）
//!
//! UI 组件依赖 `IValue` 空接口,通过 `as_contribution()`/`as_visual()` 能力查询按需提取
//! 元数据与视图。`IVisual` 与 `IContribution` 解耦——非贡献的视觉对象也可实现 `IVisual`。

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use gpui::{AnyElement, App, SharedString, Window};

use crate::value::IValue;

/// 贡献注册选项（纯数据，builder 模式）
///
/// `slot` 字段已移除——挂载点统一走 `properties["kind"]`。
#[derive(Debug, Clone, Default)]
pub struct ContributionOptions {
    pub order: i32,
    /// 父贡献 id（树形菜单、案例分类等）
    pub parent_id: Option<SharedString>,
    pub group: Option<SharedString>,
    /// 扩展元数据（如 `kind=menu`、`align=right`）
    pub properties: HashMap<SharedString, SharedString>,
}

impl ContributionOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    pub fn parent_id(mut self, parent_id: impl Into<SharedString>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    pub fn group(mut self, group: impl Into<SharedString>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn property(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// 有效挂载点：读 `properties["kind"]`
    pub fn effective_slot(&self) -> Option<&str> {
        self.properties.get("kind").map(|s| s.as_ref())
    }
}

/// 图标规格 —— `IContribution::icon` 的返回类型,描述"用什么图标"的元数据。
///
/// 设计为封闭 enum(而非 `Any`):图标种类是有限集合,variant tag 让框架无需
/// 猜测字符串语义,编译期强制穷举匹配。`IconSpec` 本身仍是 metadata,
/// 不携带 `&Window`/`&App`,保持 `IContribution` 元数据 trait 语义。
///
/// 渲染由 `rml_ui::resolve_icon` 统一处理:variant tag 直接决定渲染路径,
/// 无需 `is_url`/`is_asset_path` 等字符串推断。
///
/// # Variants
///
/// - `Named(SharedString)` — 内置命名图标(如 `"BookOpen"`),由 `IconName` 枚举解析。
///   字符串→`IconName` 映射在 ui 层维护(gpui-component 的 `IconName` 未实现 `FromStr`)。
/// - `Path(SharedString)` — SVG 资产路径(如 `"icons/foo.svg"`、`"logo.svg"`)。
///   经 `CompositeAssets` 路由:同时支持 gpui-component 内置图标 `icons/**/*.svg`
///   与 RML 用户嵌入资源(`assets/logo.svg` 等,由 `rml_core::assets::load` 管理)。
/// - `Url(SharedString)` — 外部 URL(`http:`/`https:`/`file:` 等),通过 `gpui::img` 加载。
///
/// # 与嵌入资源系统的集成
///
/// RML 框架的 `CompositeAssets`(在 `rml_app::assets`)已将 gpui-component-assets
/// 与 `rml_core::assets::load` 桥接为统一 `AssetSource`。因此 `IconSpec::Path("logo.svg")`
/// 会自动解析到用户在 `assets/logo.svg` 嵌入的资源,无需额外配置或新 variant。
#[derive(Debug, Clone)]
pub enum IconSpec {
    /// 内置命名图标(字符串对应 `IconName` 变体名,如 `"BookOpen"`)。
    Named(SharedString),
    /// SVG 资产路径(同时支持内置 `icons/**/*.svg` 与用户嵌入资源)。
    Path(SharedString),
    /// 外部 URL(`http:`/`https:`/`file:` 等)。
    Url(SharedString),
}

impl IconSpec {
    /// 构造命名图标规格。`s` 应为 `IconName` 变体名(如 `"BookOpen"`)。
    pub fn named(s: impl Into<SharedString>) -> Self {
        Self::Named(s.into())
    }

    /// 构造 SVG 资产路径规格。路径相对资产根(如 `"icons/foo.svg"`、`"logo.svg"`)。
    pub fn path(s: impl Into<SharedString>) -> Self {
        Self::Path(s.into())
    }

    /// 构造外部 URL 图标规格(如 `"https://example.com/logo.png"`)。
    pub fn url(s: impl Into<SharedString>) -> Self {
        Self::Url(s.into())
    }
}

/// 能力贡献点：仅元数据，不渲染。
/// 业务贡献（菜单项、状态栏项、案例树节点等）实现此 trait。
///
/// `IContribution: IValue`——贡献是值对象的特化，`IValue: Send + Sync + Any`
/// 提供 trait upcasting 到 `dyn Any`，供能力查询获取具体 `TypeId`。
pub trait IContribution: IValue {
    fn id(&self) -> &str;
    fn name(&self) -> SharedString;
    fn description(&self) -> SharedString {
        SharedString::default()
    }
    /// 贡献的图标规格。返回 `None` 时由框架走 fallback(`IconName::PanelLeft`)。
    ///
    /// 返回 `IconSpec` 而非 `SharedString`——variant tag 显式声明图标种类,
    /// 框架无需字符串推断。详见 [`IconSpec`]。
    fn icon(&self) -> Option<IconSpec> {
        None
    }
}

/// 视觉能力 trait —— 任何可渲染为 UI 元素的值对象实现此 trait。
///
/// 与 `IContribution` 解耦:非贡献的视觉对象(如纯视图模型)也可实现 `IVisual`。
/// 业务视觉贡献(如 `ActivityPanel`)同时实现 `IContribution + IVisual`,
/// 经 blanket impl 自动获得 `IVisualContribution` 标记。
///
/// `#[contribute]` + `#[component]` 宏自动 impl `IVisual`(生成 `render` 方法)。
pub trait IVisual: IValue {
    /// 渲染视图。host 调用此方法获取 `AnyElement`，自行决定是否缓存结果。
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}

/// 可视化贡献点 —— 标记 trait,纯组合语义(`IContribution + IVisual`)。
///
/// 业务视觉贡献(如 `ActivityPanel`)同时实现 `IContribution + IVisual` 即自动满足。
/// 框架提供 blanket impl,业务代码无需手动 impl 此 trait。
///
/// 能力查询:经 `as_visual()` 获取 `&dyn IVisual`(含 `render`),经 `as_contribution()`
/// 获取 `&dyn IContribution`(含元数据)。两者组合等价于旧的 `IVisualContribution` 查询。
pub trait IVisualContribution: IContribution + IVisual {}

/// Blanket impl —— 任何 `IContribution + IVisual` 自动获得 `IVisualContribution` 标记。
impl<T: IContribution + IVisual> IVisualContribution for T {}

/// 视觉能力扩展 trait —— 让 `dyn IValue` 可查询 `IVisual` 能力。
///
/// 框架内置:`#[contribute]` + `#[component]` 叠加时宏自动注册能力 cast 函数。
/// 业务自定义视觉类型(无 `#[contribute]`)时,手动调用 `register_visual_ability::<T>()`。
pub trait VisualAbilityExt {
    /// 若此值实现了 `IVisual`,返回视觉引用;否则 `None`。
    fn as_visual(&self) -> Option<&dyn IVisual>;
}

#[allow(unsafe_code)]
impl VisualAbilityExt for dyn IValue {
    fn as_visual(&self) -> Option<&dyn IVisual> {
        let erased = crate::ability::query::<dyn IVisual>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IVisual>(erased) })
    }
}

/// `dyn IContribution` 薄委托——trait upcast 到 `&dyn IValue` 后调用主 impl,
/// 使现有 `&dyn IContribution` 调用点无需修改。
impl VisualAbilityExt for dyn IContribution {
    fn as_visual(&self) -> Option<&dyn IVisual> {
        let iv: &dyn IValue = self;
        iv.as_visual()
    }
}

/// 贡献能力扩展 trait —— 让 `dyn IValue` 可查询 `IContribution` 能力。
///
/// UI 组件持有 `Vec<Arc<dyn IValue>>` 时，通过 `as_contribution()?.name()` 获取标题。
/// `#[contribute]` 宏为每个贡献结构体注册 `dyn IContribution` 能力 cast 函数。
pub trait ContributionAbilityExt {
    /// 若此值实现了 `IContribution`，返回贡献引用；否则 `None`。
    fn as_contribution(&self) -> Option<&dyn IContribution>;
}

#[allow(unsafe_code)]
impl ContributionAbilityExt for dyn IValue {
    fn as_contribution(&self) -> Option<&dyn IContribution> {
        let erased = crate::ability::query::<dyn IContribution>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IContribution>(erased) })
    }
}

/// 为实现 `IContribution` 但未使用 `#[contribute]` 的类型注册能力 cast 函数。
///
/// 用于简单数据项（非 UI 贡献），使 `Arc<T>` 存储为 `Arc<dyn IValue>` 后
/// 可通过 `as_contribution()` 查询到 `IContribution` 能力。
#[allow(unsafe_code)]
pub fn register_contribution_ability<T: IContribution + 'static>() {
    crate::ability::register::<T, dyn IContribution>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let contrib: &dyn IContribution = s;
            unsafe { crate::ability::erase(contrib) }
        })
    });
}

/// 为实现 `IVisual` 但未使用 `#[contribute]` + `#[component]` 组合的类型注册视觉能力 cast。
///
/// `#[contribute]` + `#[component]` 会自动注册视觉能力;仅有 `#[contribute]` 的贡献
/// 或纯视觉类型(非贡献)需手动调用此函数,使 `as_visual()` 查询生效。
#[allow(unsafe_code)]
pub fn register_visual_ability<T: IVisual + 'static>() {
    crate::ability::register::<T, dyn IVisual>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let visual: &dyn IVisual = s;
            unsafe { crate::ability::erase(visual) }
        })
    });
}

/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
///
/// `add`/`remove` 均为 `&self`——host 使用 `RwLock`/`Arc<RwLock<Vec<...>>>` 等内部可变性结构。
/// 默认空实现：host 按业务需要 override `add`/`remove`。
///
/// 框架为共享存储类型 `RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>` 提供默认 impl，
/// 业务代码持有 `entries: Arc<RwLock<Vec<...>>>` 字段后，`entries.clone()` 经 unsized coercion
/// 转为 `Arc<dyn IContributionHost>` 即可注册。需要自定义受理逻辑时，为自身类型 impl 本 trait。
///
/// host 可通过 `c.as_command()`/`c.as_visual()` 查询贡献能力并分类存储。
pub trait IContributionHost: Send + Sync + 'static {
    /// 运行时获取 host ID。默认 `""`——共享存储 host 无固有 ID，由 `register_host(id, ...)` 外部传入。
    fn id(&self) -> &'static str { "" }

    /// 受理贡献（统一入口）。host 自行决定如何存储/分发。
    /// `options` 为 `None` 时表示无元数据（order/group/kind 等），host 可按 `ContributionOptions::default()` 处理。
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: Option<ContributionOptions>) {}

    /// 移除贡献。默认空实现。
    fn remove(&self, _contribution_id: &str) {}
}

/// 共享存储默认 host 实现：`Arc<RwLock<Vec<...>>>` 经 unsized coercion 转为 `Arc<dyn IContributionHost>`。
/// 业务代码无需自定义 host 类型即可使用 `register_host(id, storage.clone())`。
impl IContributionHost for RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>> {
    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        self.write().unwrap().push((contribution, options.unwrap_or_default()));
    }

    fn remove(&self, contribution_id: &str) {
        self.write().unwrap().retain(|(c, _)| c.id() != contribution_id);
    }
}

/// Host 共享存储类型别名 —— `Arc<RwLock<Vec<...>>>`。
/// 经 `register_host` 注册到 registry（unsized coercion 为 `Arc<dyn IContributionHost>`），
/// registry 调用 trait 方法写入/移除贡献，不经 Entity 系统，避免 `on_loaded` 中的重入 panic。
pub type ContributionStorage = Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>;

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 `IContributionHost::add`。
///
/// Registry 存储 `Arc<dyn IContributionHost>` trait object，经 trait 方法路由贡献，
/// 不依赖具体存储类型（依赖倒置）、不经 Entity 系统、不存 `WeakEntity` 闭包。
/// 这避免了 `on_loaded` 中 `weak.update` 的重入 panic，同时允许业务代码自定义受理逻辑。
///
/// Host 必须在 `on_loaded` 中先经 `cx.register_host(id, host)` 注册自身（或共享存储），
/// 再调用 `bootstrap_host_contributions(cx, id)` 触发该 host_id 的贡献注册。
pub trait IContributionRegistry: Send + Sync {
    /// 注册 host。host 经 `Arc<dyn IContributionHost>` 提供 `add`/`remove` 能力，
    /// registry 调用 trait 方法路由贡献，不依赖具体存储类型（依赖倒置）。
    fn add(&self, host_id: &str, host: Arc<dyn IContributionHost>);

    /// 注销 host。
    fn remove(&self, host_id: &str);

    /// 向 host 注册贡献（`#[contribute]` 宏生成代码调用）。
    /// registry 经 `host.add(c, opts)` 路由到具体 host 的受理逻辑。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    );

    /// 从 host 注销贡献。
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}

// ──────────────────────────────────────────────────────────────────────────
//  StatusBar 专属契约 —— WPF StatusBarItem + DockPanel.Dock 类比
// ──────────────────────────────────────────────────────────────────────────

/// 状态栏项对齐方式 —— WPF `DockPanel.Dock` 类比。
///
/// 框架提供此枚举是因为 `IStatusBarItem::align()` 返回类型需要在 core 定义。
/// `rml_ui::status_bar` 经 `pub use` re-export 保持兼容。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusBarAlign {
    #[default]
    Left,
    Right,
    Center,
}

/// 状态栏项 —— 状态栏容器的视觉贡献,额外提供对齐提示。
///
/// WPF `StatusBarItem` + `DockPanel.Dock` 类比:容器按 `align()` 决定布局位置,
/// 内容由 `IVisual::render` 提供。`order` 经 `ContributionOptions` 传入,
/// 命令由 `render` 自行处理(返回的 `AnyElement` 可携带 `.on_click` 等)。
///
/// 业务实现此 trait 后,经 `register_status_bar_item_ability::<T>()` 注册能力 cast,
/// host 即可通过 `as_status_bar_item()` 查询提取 `align()` 信息。
pub trait IStatusBarItem: IVisualContribution {
    /// 返回此状态栏项的对齐方式(容器据此决定布局位置)。
    fn align(&self) -> StatusBarAlign;
}

/// 状态栏项能力扩展 trait —— 让 `dyn IValue` 可查询 `IStatusBarItem` 能力。
///
/// 与 `VisualAbilityExt`/`ContributionAbilityExt` 模式一致。
/// 业务自定义状态栏项类型后,需调用 `register_status_bar_item_ability::<T>()` 注册。
pub trait StatusBarItemAbilityExt {
    /// 若此值实现了 `IStatusBarItem`,返回引用;否则 `None`。
    fn as_status_bar_item(&self) -> Option<&dyn IStatusBarItem>;
}

#[allow(unsafe_code)]
impl StatusBarItemAbilityExt for dyn IValue {
    fn as_status_bar_item(&self) -> Option<&dyn IStatusBarItem> {
        let erased = crate::ability::query::<dyn IStatusBarItem>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IStatusBarItem>(erased) })
    }
}

/// 为实现 `IStatusBarItem` 的类型注册能力 cast 函数。
///
/// `#[contribute]` 宏暂不自动识别 `kind="status"` 注册此能力,业务需手动调用。
/// 调用后,`as_status_bar_item()` 查询生效。
#[allow(unsafe_code)]
pub fn register_status_bar_item_ability<T: IStatusBarItem + 'static>() {
    crate::ability::register::<T, dyn IStatusBarItem>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let item: &dyn IStatusBarItem = s;
            unsafe { crate::ability::erase(item) }
        })
    });
}
