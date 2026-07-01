//! 贡献点契约 —— `IContribution` / `IVisualContribution` / Host / Registry
//!
//! 面向应用开发者的插件化扩展 API。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{App, SharedString};

use crate::contribution_cache::ComponentEntityCacheImpl;

/// 框架内部视觉渲染回调类型（对开发者透明）
pub type VisualRenderer = Arc<
    dyn Fn(&mut ContributionRenderContext<'_>, &mut ComponentEntityCacheImpl) -> gpui::AnyElement
        + Send
        + Sync,
>;

/// 贡献在 UI 中的呈现角色（元数据，供 ViewModel 映射到控件数据）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualMode {
    /// 图标/紧凑控件（侧栏图标栏等）
    #[default]
    Chrome,
    /// 可切换的面板内容
    Panel,
    /// 内联条带（状态栏、工具条等）
    Inline,
    /// 预留：浮动层
    Overlay,
}

/// 内联条带内左右放置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VisualPlacement {
    #[default]
    Left,
    Right,
}

/// 贡献注册选项
#[derive(Debug, Clone, Default)]
pub struct ContributionOptions {
    pub order: i32,
    /// 父贡献 id（层级 host 用 `parent_id` 挂载子节点）
    pub parent_id: Option<SharedString>,
    pub group: Option<SharedString>,
    /// 呈现角色元数据（如活动栏面板、状态栏内联项）；由 ViewModel 读取后绑定到 UI 控件
    pub visual_mode: Option<VisualMode>,
    pub placement: Option<VisualPlacement>,
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

    pub fn visual_mode(mut self, mode: VisualMode) -> Self {
        self.visual_mode = Some(mode);
        self
    }

    pub fn placement(mut self, placement: VisualPlacement) -> Self {
        self.placement = Some(placement);
        self
    }

    pub fn property(mut self, key: impl Into<SharedString>, value: impl Into<SharedString>) -> Self {
        self.properties.insert(key.into(), value.into());
        self
    }
}

/// 所有贡献的公共契约：元数据 + 可选能力钩子
pub trait IContribution: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> SharedString;
    fn description(&self) -> SharedString;
    /// 图标名（如 gpui-component `IconName` 的 Debug 字符串），无图标返回 None
    fn icon(&self) -> Option<SharedString>;

    fn on_register(&self, _host_id: &str, _cx: &mut App) {}
    fn on_unregister(&self, _host_id: &str, _cx: &mut App) {}
}

/// 可视化贡献：显式关联 RML `#[component]` 类型
pub trait IVisualContribution: IContribution {
    type View: crate::component::IComponent + Default + Send + Sync + 'static;

    /// 返回组件实例；UI 在 `View` 的 `.rml` 中声明式定义
    fn render(&self) -> Self::View;
}

/// 贡献点主机：管理某 `host_id` 下的贡献集合与变更通知。
///
/// **不是 UI 组件。** Host 维护贡献元数据列表；应用通过 ViewModel 将 `entries()`
/// 映射为控件数据（如 `ActivityPanels`、`StatusBarItems`），在 RML 中声明式绑定。
pub trait IContributionHost: Send + Sync {
    fn host_id(&self) -> &str;
    fn add(&mut self, entry: ContributedEntry, cx: &mut App);
    fn remove(&mut self, contribution_id: &str, cx: &mut App) -> bool;
    fn entries(&self) -> &[ContributedEntry];
    fn version(&self) -> u64;
    fn set_on_changed(&mut self, callback: Box<dyn Fn(&mut App) + Send + Sync>);
}

/// 已注册贡献条目
pub struct ContributedEntry {
    pub contribution: Arc<dyn IContribution>,
    /// 保留扩展点；标准 MVVM 路径下为 `None`，由 ViewModel 消费元数据
    pub visual: Option<VisualRenderer>,
    pub options: ContributionOptions,
}

/// 贡献渲染上下文（`IVisualContribution` 高级路径使用）
pub struct ContributionRenderContext<'a> {
    pub window: &'a mut gpui::Window,
    pub cx: &'a mut App,
    pub active: bool,
    pub mode: VisualMode,
    pub placement: VisualPlacement,
}

/// 组件 Entity 缓存
pub trait ComponentEntityCache {
    fn render_view<V>(
        &mut self,
        contribution_id: &str,
        view: V,
        ctx: &mut ContributionRenderContext<'_>,
    ) -> gpui::AnyElement
    where
        V: gpui::Render + Send + Sync + 'static;

    fn clear(&mut self, contribution_id: &str);
    fn clear_all(&mut self);
}

/// 统一注册器契约
pub trait IContributionRegistry {
    fn add_host(&mut self, host: Box<dyn IContributionHost>);
    fn host(&self, host_id: &str) -> Option<&dyn IContributionHost>;

    fn register(
        &mut self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
        cx: &mut App,
    );

    fn unregister(&mut self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
