//! 贡献点契约 —— 能力贡献 / 可视化贡献 / 受理方 / 桥接注册表
//!
//! 框架不存储贡献数据——host 主动受理（`add`/`remove`），registry 仅按 `host_id` 路由。
//! 能力查询（`ICommand`/`IVisualContribution`/`IContribution`）经 `*AbilityExt`
//! extension trait 实现，核心 trait 不枚举贡献类型。
//!
//! `IContribution: IValue`——贡献是值对象的特化。UI 组件依赖 `IValue` 空接口，
//! 通过 `as_contribution()`/`as_visual()` 能力查询按需提取元数据与视图。

use std::collections::HashMap;
use std::sync::Arc;

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

/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
///
/// host 直接实现此 trait（不再经由 `IHostEntity` 钩子）。
/// `add`/`remove` 均为 `&self`——host 使用 `RwLock`/`ObservableVec` 等内部可变性结构。
/// 默认空实现：host 按业务需要 override `add`。
///
/// host 可通过 `c.as_command()`/`c.as_visual()` 查询贡献能力并分类存储。
///
/// `#[contributehost]` 宏生成 `pub const ID: &'static str`（inherent impl），
/// 实现 trait 时 `fn id()` 返回 `Self::ID` 即可。
pub trait IContributionHost: Send + Sync + 'static {
    /// 运行时获取 host ID。
    fn id(&self) -> &'static str;

    /// 受理贡献（统一入口）。host 自行决定如何存储/分发。
    /// `options` 为 `None` 时表示无元数据（order/group/kind 等），host 可按 `ContributionOptions::default()` 处理。
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: Option<ContributionOptions>) {}

    /// 移除贡献。默认空实现。
    fn remove(&self, _contribution_id: &str) {}
}

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add 方法。
/// 所有方法 `&self` + 无 `cx` —— 内部 `RwLock` 可变性，`host.add` 直接调用。
///
/// Registry 仅存储 `IContributionHost`，不存储贡献本身。host 未注册时 `register` 直接 drop 贡献
/// （warn 日志）。Host 必须在 `on_loaded` 中先经 `__rml_install_host` 注册自身，再触发该 host_id
/// 的贡献注册。
pub trait IContributionRegistry: Send + Sync {
    /// 注册 host（Entity 在 `on_loaded` 时通过 `__rml_install_host` 调用）。
    fn add_host(&self, host: Arc<dyn IContributionHost>);

    /// 注销 host。
    fn remove_host(&self, host_id: &str);

    /// 向 host 注册贡献（`#[contribute]` 宏生成代码调用）。
    /// `options` 为 `None` 时表示无元数据。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
    );

    /// 从 host 注销贡献。
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
