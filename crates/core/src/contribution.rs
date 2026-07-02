//! 贡献点契约 —— 能力贡献 / 可视化贡献 / 受理方 / 桥接注册表
//!
//! 框架不存储贡献数据——host 主动受理（`add`/`remove`），registry 仅按 `host_id` 路由。
//! 视觉贡献向下转型通过 `Any` supertrait + 宏生成 `VisualExtractor` 函数实现。

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
/// 添加 `Any` supertrait——使 `dyn IContribution` 支持 trait upcasting 到 `dyn Any`，
/// 配合宏生成的视觉提取器实现 `Arc<dyn IVisualContribution>` 向下转型。
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
/// host 可使用 Vec/HashMap/任何自定义结构，甚至不存储——框架不限定。
pub trait IContributionHost: Send + Sync + 'static {
    const ID: &'static str;

    /// 受理代码：接收并处置贡献。host 按 options.slot/group 等分发到自有数据结构。
    fn add(&mut self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);

    /// 移除贡献。host 自行清理对应数据。
    fn remove(&mut self, contribution_id: &str, cx: &mut App);
}

/// 内部桥接 trait：类型擦除的 host 句柄，包装 WeakEntity<T>。
#[doc(hidden)]
pub trait HostHandle: Send + Sync {
    fn id(&self) -> &str;
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut App);
    fn remove(&self, contribution_id: &str, cx: &mut App);
}

/// 视觉提取器函数类型：从 `Arc<dyn IContribution>` 提取 `Arc<dyn IVisualContribution>`。
/// 由 `#[contribute]` 宏为视觉贡献生成，注册到 `rml_app::contribution` 进程级静态表。
/// 利用 `Any` supertrait + trait upcasting coercion（Rust 1.86+）：
///   `Arc<dyn IContribution>` → `Arc<dyn Any + Send + Sync>` → `Arc::downcast::<T>()` → `Arc<T> as Arc<dyn IVisualContribution>`
#[doc(hidden)]
pub type VisualExtractor = fn(&Arc<dyn IContribution>) -> Option<Arc<dyn IVisualContribution>>;

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add 方法。
/// trait 仅含 4 个用户决策方法——视觉提取器注册/查找为 `#[doc(hidden)]` 自由函数，
/// 由 `#[ctor::ctor]` 在进程启动期写入进程级静态表，host 通过 `rml_app::contribution::extract_visual` 查找。
pub trait IContributionRegistry: Send + Sync {
    fn add(&self, host: Box<dyn HostHandle>, cx: &mut App);
    fn remove(&self, host_id: &str, cx: &mut App);
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
        cx: &mut App,
    );
    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
