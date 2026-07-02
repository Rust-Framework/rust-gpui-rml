//! 贡献点契约 —— 扩展点标识 + 条目元数据
//!
//! 运行时注册表由 `rml_app::ContributionExt` 提供。框架不包含 Shell/UI 映射或业务桥接。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;

/// 框架内部视觉渲染回调类型（对开发者透明）
pub type VisualRenderer = Arc<
    dyn Fn(&mut RenderContext<'_>) -> gpui::AnyElement + Send + Sync,
>;

/// 贡献注册选项
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

/// 所有贡献的公共契约：元数据
pub trait IContribution: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> SharedString;
    fn description(&self) -> SharedString;
    /// 图标名（如 gpui-component `IconName` 的 Debug 字符串），无图标返回 None
    fn icon(&self) -> Option<SharedString>;
}

/// 视觉贡献契约：具备渲染能力的贡献点。
///
/// 由 `#[contribute]` + `#[component]` 叠加时自动实现。
/// IVisualContribution 共享用于：
/// 1. 构造 TreeItems（案例树/活动面板树）—— 从元数据（id/name/icon/parent_id/order）构建
/// 2. 渲染到 tab body —— 选中后调用 render() 直接渲染
///
/// 开发者无需关心 Entity 缓存，框架内部处理。
pub trait IVisualContribution: IContribution {
    /// 渲染贡献视图。框架内部负责 Entity 缓存与复用。
    fn render(&self, ctx: &mut RenderContext<'_>) -> gpui::AnyElement;
}

/// 贡献点主机标识（扩展点命名空间）。
///
/// 由 `#[contributehost(id = "...")]` 实现。运行时条目存在 `ContributionRegistry` 中；
/// 消费者通过 `contribution_entries` / `subscribe_host_changes` 读取，自行决定如何消费。
pub trait IContributionHost {
    const ID: &'static str;
}

/// 已注册贡献条目
pub struct ContributedEntry {
    pub contribution: Arc<dyn IContribution>,
    pub visual: Option<VisualRenderer>,
    pub options: ContributionOptions,
}

/// 渲染上下文（视觉贡献渲染时使用）
pub struct RenderContext<'a> {
    pub window: &'a mut gpui::Window,
    pub cx: &'a mut gpui::App,
    pub active: bool,
}

/// 组件 Entity 缓存（框架内部，对开发者透明）
#[doc(hidden)]
pub trait ComponentEntityCache {
    fn render_view<V>(
        &mut self,
        contribution_id: &str,
        view: V,
        ctx: &mut RenderContext<'_>,
    ) -> gpui::AnyElement
    where
        V: gpui::Render + Send + Sync + 'static;

    /// 预注册已创建的 Entity，使后续 `render_view` 直接返回该 Entity 而非延迟创建。
    /// 用于需要 `Context::new()` 创建子 Entity（有父级链接）的场景。
    fn pre_register<T: gpui::Render + Send + Sync + 'static>(
        &mut self,
        contribution_id: &str,
        entity: gpui::Entity<T>,
    );

    fn clear(&mut self, contribution_id: &str);
    fn clear_all(&mut self);
}
