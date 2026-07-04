//! 贡献点契约 —— 能力贡献 / 可视化贡献 / 受理方 / 桥接注册表
//!
//! 框架不存储贡献数据——host 主动受理（`add`/`remove`），registry 仅按 `host_id` 路由。
//! 能力查询（`ICommand`/`IVisualContribution`/`IContribution`）经 `*AbilityExt`
//! extension trait 实现，核心 trait 不枚举贡献类型。
//!
//! `IContribution: IValue`——贡献是值对象的特化。UI 组件依赖 `IValue` 空接口，
//! 通过 `as_contribution()`/`as_visual()` 能力查询按需提取元数据与视图。

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
    fn icon(&self) -> Option<SharedString> {
        None
    }
}

/// 可视化贡献点：能渲染 UI 元素的贡献。
/// `IVisualContribution: IContribution`——视觉贡献同时是能力贡献（含元数据）。
/// 业务视觉贡献（如 `ActivityPanel`）实现此 trait，由 `#[contribute]` + `#[component]` 宏自动生成。
pub trait IVisualContribution: IContribution {
    /// 渲染贡献视图。host 调用此方法获取 `AnyElement`，自行决定是否缓存结果。
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}

/// 视觉能力扩展 trait —— 让 `dyn IValue` 可查询 `IVisualContribution` 能力。
///
/// 框架内置：`#[contribute]` + `#[component]` 叠加时宏自动注册能力 cast 函数。
/// 业务自定义能力 trait 时，参考此模式编写等价 extension trait。
pub trait VisualAbilityExt {
    /// 若此值实现了 `IVisualContribution`，返回视觉引用；否则 `None`。
    fn as_visual(&self) -> Option<&dyn IVisualContribution>;
}

#[allow(unsafe_code)]
impl VisualAbilityExt for dyn IValue {
    fn as_visual(&self) -> Option<&dyn IVisualContribution> {
        let erased = crate::ability::query::<dyn IVisualContribution>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IVisualContribution>(erased) })
    }
}

/// `dyn IContribution` 薄委托——trait upcast 到 `&dyn IValue` 后调用主 impl，
/// 使现有 `&dyn IContribution` 调用点无需修改。
impl VisualAbilityExt for dyn IContribution {
    fn as_visual(&self) -> Option<&dyn IVisualContribution> {
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

/// 为实现 `IVisualContribution` 但未使用 `#[contribute]` + `#[component]` 组合的类型注册视觉能力 cast。
///
/// `#[contribute]` + `#[component]` 会自动注册视觉能力；仅有 `#[contribute]` 的贡献
/// 需手动调用此函数，使 `as_visual()` 查询生效。
#[allow(unsafe_code)]
pub fn register_visual_ability<T: IVisualContribution + 'static>() {
    crate::ability::register::<T, dyn IVisualContribution>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let visual: &dyn IVisualContribution = s;
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
