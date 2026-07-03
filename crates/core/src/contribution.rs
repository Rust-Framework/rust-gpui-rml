//! 贡献点契约 —— 能力贡献 / 可视化贡献 / 受理方 / 桥接注册表
//!
//! 框架不存储贡献数据——host 主动受理（`add`/`add_visual`/`remove`），registry 仅按 `host_id` 路由。
//! 视觉贡献通过独立的 `register_visual` 路径直达 host 的 `add_visual`，无需提取器转换。

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use gpui::{AnyElement, App, SharedString, Window};

/// 贡献注册选项（纯数据，builder 模式）
#[derive(Debug, Clone, Default)]
pub struct ContributionOptions {
    pub order: i32,
    /// 父贡献 id（树形菜单、案例分类等）
    pub parent_id: Option<SharedString>,
    pub group: Option<SharedString>,
    /// Shell 挂载点（开放字符串，语义由应用定义）
    pub slot: Option<SharedString>,
    /// 扩展元数据（如 `align=right`）
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

    pub fn slot(mut self, slot: impl Into<SharedString>) -> Self {
        self.slot = Some(slot.into());
        self
    }

    pub fn property(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }

    /// 有效挂载点：`slot` 优先，兼容旧 `properties["kind"]`
    pub fn effective_slot(&self) -> Option<&str> {
        self.slot
            .as_deref()
            .or_else(|| self.properties.get("kind").map(|s| s.as_ref()))
    }
}

/// 能力贡献点：仅元数据，不渲染。
/// 业务贡献（菜单项、状态栏项、案例树节点等）实现此 trait。
/// 添加 `Any` supertrait——使 `dyn IContribution` 支持 trait upcasting 到 `dyn Any`。
pub trait IContribution: Send + Sync + Any {
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

/// 贡献点主机：主动受理方。host 自行决定如何存储/映射贡献。
///
/// host 直接实现此 trait（不再经由 `IHostEntity` 钩子）。
/// `add`/`add_visual`/`remove` 均为 `&self`——host 使用 `RwLock`/`ObservableVec` 等内部可变性结构。
/// 默认空实现：host 按业务需要 override `add`（能力贡献）或 `add_visual`（视觉贡献）。
///
/// `#[contributehost]` 宏生成 `pub const ID: &'static str`（inherent impl），
/// 实现 trait 时 `fn id()` 返回 `Self::ID` 即可。
pub trait IContributionHost: Send + Sync + 'static {
    /// 运行时获取 host ID。
    fn id(&self) -> &'static str;

    /// 受理能力贡献（非视觉）。默认空实现，host 按需 override。
    fn add(&self, _contribution: Arc<dyn IContribution>, _options: ContributionOptions) {}

    /// 受理视觉贡献。默认空实现，视觉 host override。
    fn add_visual(&self, _contribution: Arc<dyn IVisualContribution>, _options: ContributionOptions) {}

    /// 移除贡献。默认空实现。
    fn remove(&self, _contribution_id: &str) {}
}

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add/add_visual 方法。
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

    /// 向 host 注册能力贡献（`#[contribute]` 宏生成代码调用）。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    );

    /// 向 host 注册视觉贡献（`#[contribute]` + `#[component]` 叠加时由宏生成代码调用）。
    /// 视觉贡献直达 host 的 `add_visual`，无需 `VisualExtractor` 转换。
    fn register_visual(
        &self,
        host_id: &str,
        contribution: Arc<dyn IVisualContribution>,
        options: ContributionOptions,
    );

    /// 从 host 注销贡献。
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;
}
