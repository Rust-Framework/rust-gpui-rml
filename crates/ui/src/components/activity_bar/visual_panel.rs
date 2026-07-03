//! `VisualActivityPanel` —— 视觉贡献 → IActivityPanel 通用适配器

use std::sync::Arc;

use gpui::{AnyElement, App, SharedString, Window};
use gpui_component::IconName;
use rml_core::contribution::{IContribution, VisualAbilityExt};

use super::traits::IActivityPanel;

/// 通用视觉贡献 → `IActivityPanel` 适配器。
///
/// 包装 `Arc<dyn IContribution>`，`id`/`icon`/`title` 从 `IContribution` 元数据提取，
/// `panel()` 经 `as_visual()` 委托给 `IVisualContribution::render`（经框架实体缓存复用 Entity）。
pub struct VisualActivityPanel {
    contrib: Arc<dyn IContribution>,
    id: SharedString,
    icon_name: IconName,
    title: SharedString,
}

impl VisualActivityPanel {
    /// 从贡献创建适配器（贡献需实现 `IVisualContribution`，否则 `panel()` 返回 `None`）。
    ///
    /// `icon` 字符串经 `parse_icon_name` 解析，未匹配时 fallback `PanelLeft`。
    pub fn new(contrib: Arc<dyn IContribution>) -> Option<Self> {
        let id: SharedString = contrib.id().to_string().into();
        let title = contrib.name();
        let icon_name = contrib
            .icon()
            .and_then(|s| parse_icon_name(&s))
            .unwrap_or(IconName::PanelLeft);
        Some(Self {
            contrib,
            id,
            icon_name,
            title,
        })
    }
}

impl IActivityPanel for VisualActivityPanel {
    fn id(&self) -> SharedString {
        self.id.clone()
    }
    fn icon(&self) -> IconName {
        self.icon_name.clone()
    }
    fn title(&self) -> SharedString {
        self.title.clone()
    }
    fn panel(&self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        Some(self.contrib.as_visual()?.render(window, cx))
    }
}

/// 解析图标名字符串 → `IconName`。
///
/// `IconName` 由 `icon_named!` 宏生成，未实现 `FromStr`。
/// 此处映射常用图标子集；未匹配时返回 `None`，调用方使用 fallback。
fn parse_icon_name(s: &str) -> Option<IconName> {
    match s {
        "BookOpen" => Some(IconName::BookOpen),
        "PanelLeft" => Some(IconName::PanelLeft),
        "PanelRight" => Some(IconName::PanelRight),
        "PanelBottom" => Some(IconName::PanelBottom),
        "Settings" => Some(IconName::Settings),
        "Search" => Some(IconName::Search),
        "Folder" => Some(IconName::Folder),
        "File" => Some(IconName::File),
        "Menu" => Some(IconName::Menu),
        "LayoutDashboard" => Some(IconName::LayoutDashboard),
        "SquareTerminal" => Some(IconName::SquareTerminal),
        "Github" => Some(IconName::Github),
        "Bell" => Some(IconName::Bell),
        "User" => Some(IconName::User),
        "Star" => Some(IconName::Star),
        "Info" => Some(IconName::Info),
        _ => None,
    }
}
