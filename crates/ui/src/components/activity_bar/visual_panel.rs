//! `VisualActivityPanel` —— 视觉贡献 → IActivityPanel 通用适配器

use std::sync::Arc;

use gpui::{AnyElement, App, SharedString, Window};
use rml_core::contribution::{IContribution, IVisualContribution, VisualAbilityExt};

use super::traits::IActivityPanel;

/// 通用视觉贡献 → `IActivityPanel` 适配器。
///
/// 包装 `Arc<dyn IContribution>`，`IContribution` 元数据与 `IVisualContribution::render`
/// 全部委托给底层贡献。图标通过 `IContribution::icon` 字符串提供，由 `icon::resolve_icon`
/// 在渲染时解析（URL 文件地址 → `Svg::external_path`；非 URL → 内置 `IconName`）。
pub struct VisualActivityPanel {
    contrib: Arc<dyn IContribution>,
}

impl VisualActivityPanel {
    /// 从贡献创建适配器。贡献需实现 `IVisualContribution`，否则返回 `None`。
    pub fn new(contrib: Arc<dyn IContribution>) -> Option<Self> {
        contrib.as_visual()?;
        Some(Self { contrib })
    }
}

impl IContribution for VisualActivityPanel {
    fn id(&self) -> &str {
        self.contrib.id()
    }
    fn name(&self) -> SharedString {
        self.contrib.name()
    }
    fn description(&self) -> SharedString {
        self.contrib.description()
    }
    fn icon(&self) -> Option<SharedString> {
        self.contrib.icon()
    }
}

impl IVisualContribution for VisualActivityPanel {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        self.contrib
            .as_visual()
            .expect("VisualActivityPanel requires IVisualContribution")
            .render(window, cx)
    }
}

impl IActivityPanel for VisualActivityPanel {}
