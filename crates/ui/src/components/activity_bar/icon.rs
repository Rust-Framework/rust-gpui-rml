//! 活动栏图标解析 —— `IContribution::icon` 字符串 → 可渲染图标元素
//!
//! 解析规则：
//! 1. URL 格式（含 `file:` 开头的本地文件地址）→ 文件地址，通过 `gpui::img` 加载（支持 SVG/PNG/JPG 等）
//! 2. 非 URL 字符串 → 内置 `IconName`

use gpui::{AnyElement, IntoElement, SharedString, Styled, Window, img};
use gpui_component::{Icon, IconName};

/// 解析贡献点 `IContribution::icon` 字符串为可渲染图标元素。
///
/// - `None` → fallback `IconName::PanelLeft`
/// - URL 格式（`file:`/`http:`/`https:` 等开头）→ 通过 `gpui::img` 加载（支持 SVG/PNG/JPG 等）
/// - 非 URL 字符串 → 解析为内置 `IconName`；未匹配则使用 fallback
pub fn resolve_icon(icon: Option<SharedString>, window: &Window) -> AnyElement {
    match icon.as_deref() {
        Some(s) if is_url(s) => {
            let text_color = window.text_style().color;
            img(s)
                .flex_shrink_0()
                .size_4()
                .text_color(text_color)
                .into_any_element()
        }
        Some(s) => match parse_icon_name(s) {
            Some(name) => Icon::new(name).into_any_element(),
            None => Icon::new(IconName::PanelLeft).into_any_element(),
        },
        None => Icon::new(IconName::PanelLeft).into_any_element(),
    }
}

/// 判断字符串是否为 URL 格式（含 `file:`/`http:`/`https:` 等协议前缀）。
fn is_url(s: &str) -> bool {
    s.starts_with("file:")
        || s.starts_with("http:")
        || s.starts_with("https:")
        || s.contains("://")
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
