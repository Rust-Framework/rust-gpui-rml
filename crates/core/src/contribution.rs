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
/// host 使用 `ObservableVec` 等内部可变性结构存储数据，`add`/`remove` 为 `&self`。
///
/// `#[contributehost]` 宏生成 `pub const ID: &'static str`（inherent impl），
/// 实现 trait 时 `fn id()` 返回 `Self::ID` 即可。
pub trait IContributionHost: Send + Sync + 'static {
    /// 运行时获取 host ID。
    fn id(&self) -> &'static str;

    /// 受理代码：接收并处置贡献。host 按 `options.slot`/`group` 分发到自有 `ObservableVec`。
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions);

    /// 移除贡献。host 自行清理对应数据。
    fn remove(&self, contribution_id: &str);
}

/// 贡献条目：host 受理的贡献 + 注册选项。
/// 由 `#[contributehost]` 宏注入的 `entries: ObservableVec<ContributionEntry>` 字段使用。
#[derive(Clone)]
pub struct ContributionEntry {
    pub contribution: Arc<dyn IContribution>,
    pub options: ContributionOptions,
}

/// Host Entity 钩子：业务代码实现此 trait 提供 host 特有逻辑。
///
/// 框架生成的 `ILifecycle::on_loaded` 在完成标准 setup（channel/spawn/take_pending/i18n observe）
/// 后调用 `host_on_loaded`；locale 变更时调用 `on_locale_changed`（框架已在外层
/// bump `i18n_version` + `cx.notify`，业务代码只需处理额外刷新逻辑）。
pub trait IHostEntity {
    /// 框架标准 setup 完成后调用。业务代码在此执行 host 特有初始化。
    fn host_on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>)
    where
        Self: Sized,
    {
    }

    /// locale 变更时调用。默认为空（框架已 bump `i18n_version` + `cx.notify`）。
    fn on_locale_changed(&mut self, _cx: &mut gpui::Context<Self>)
    where
        Self: Sized,
    {
    }
}

/// 视觉提取器函数类型：从 `Arc<dyn IContribution>` 提取 `Arc<dyn IVisualContribution>`。
/// 由 `#[contribute]` 宏为视觉贡献生成，注册到 `rml_app::contribution` 进程级静态表。
/// 利用 `Any` supertrait + trait upcasting coercion（Rust 1.86+）：
///   `Arc<dyn IContribution>` → `Arc<dyn Any + Send + Sync>` → `Arc::downcast::<T>()` → `Arc<T> as Arc<dyn IVisualContribution>`
#[doc(hidden)]
pub type VisualExtractor = fn(&Arc<dyn IContribution>) -> Option<Arc<dyn IVisualContribution>>;

/// 贡献注册表接口：桥接 contribute → host。
/// 框架内实现，按 host_id 路由 register 调用到对应 host 的 add 方法。
/// 所有方法 `&self` + 无 `cx` —— 内部 `RwLock` 可变性，`host.add` 直接调用。
pub trait IContributionRegistry: Send + Sync {
    /// 注册 host（Entity 在 `on_loaded` 时调用，传入 `Arc<dyn IContributionHost>`）。
    fn add(&self, host: Arc<dyn IContributionHost>);

    /// 注销 host。
    fn remove(&self, host_id: &str);

    /// 向 host 注册贡献（`#[contribute]` 宏生成代码调用）。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: ContributionOptions,
    );

    /// 从 host 注销贡献。
    fn unregister(&self, host_id: &str, contribution_id: &str) -> bool;

    /// Entity host 在 `on_loaded` 中调用：取出 pending 贡献，自行 `add` 受理。
    /// 取出后 pending 队列清空。后续 `register` 调用仍入 pending（Entity host 不注册 Arc）。
    fn take_pending(&self, host_id: &str) -> Vec<(Arc<dyn IContribution>, ContributionOptions)>;
}
