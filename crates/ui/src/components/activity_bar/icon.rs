//! 活动栏图标解析 —— `IContribution::icon` 字符串 → 可渲染图标元素
//!
//! 解析规则（按优先级）：
//! 1. URL 格式（`file:`/`http:`/`https:` 等开头）→ 通过 `gpui::img` 加载（支持 SVG/PNG/JPG 等）
//! 2. 命名图标（匹配 `IconName` 变体名，如 `"BookOpen"`）→ `Icon::new(name).small()`
//! 3. SVG 资产路径（非 URL、含 `/` 或以 `.svg` 结尾、未匹配命名）→ `Icon::default().path(s).small()`
//!    利用 Icon 组件的 `path()` 能力加载自定义 SVG 资产（如 `"icons/custom.svg"`）
//! 4. 其他未匹配字符串 → fallback `IconName::PanelLeft`
//! 5. `None` → fallback `IconName::PanelLeft`
//!
//! 所有 `Icon` 实例统一应用 `.small()`（Sizable trait），与 TabWindow 标题栏图标尺寸一致。

use gpui::{AnyElement, IntoElement, SharedString, Styled, Window, img};
use gpui_component::{Icon, IconName, Sizable as _};

/// 解析贡献点 `IContribution::icon` 字符串为可渲染图标元素。
///
/// 充分利用 `Icon` 组件能力处理不同数据类型：
/// - URL → `img`（外部图片，Icon 无法加载 URL）
/// - 命名图标 → `Icon::new(name)`（内置 `IconName` 枚举）
/// - SVG 资产路径 → `Icon::default().path(s)`（自定义 SVG 资产）
/// - 其他 → fallback `IconName::PanelLeft`
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
            Some(name) => Icon::new(name).small().into_any_element(),
            None if is_asset_path(s) => {
                Icon::default().path(s).small().into_any_element()
            }
            None => Icon::new(IconName::PanelLeft).small().into_any_element(),
        },
        None => Icon::new(IconName::PanelLeft).small().into_any_element(),
    }
}

/// 判断字符串是否为 URL 格式（含 `file:`/`http:`/`https:` 等协议前缀）。
fn is_url(s: &str) -> bool {
    s.starts_with("file:")
        || s.starts_with("http:")
        || s.starts_with("https:")
        || s.contains("://")
}

/// 判断字符串是否像 SVG 资产路径（非 URL、含路径分隔符或 `.svg` 后缀）。
///
/// 用于在 `parse_icon_name` 未匹配时，将字符串交给 `Icon::default().path(s)`
/// 作为自定义 SVG 资产路径加载（如 `"icons/custom.svg"`、`"my-icon.svg"`）。
fn is_asset_path(s: &str) -> bool {
    s.contains('/') || s.ends_with(".svg")
}

/// 解析图标名字符串 → `IconName`。
///
/// `IconName` 由 `icon_named!` 宏从 `gpui-component-assets` 的 SVG 文件名生成（kebab-case → PascalCase），
/// 未实现 `FromStr`。此处维护完整映射，覆盖 assets 目录下全部 SVG 图标。
///
/// 未匹配时返回 `None`，调用方按资产路径或 fallback 处理。
fn parse_icon_name(s: &str) -> Option<IconName> {
    match s {
        // A
        "ALargeSmall" => Some(IconName::ALargeSmall),
        // Arrow
        "ArrowDown" => Some(IconName::ArrowDown),
        "ArrowLeft" => Some(IconName::ArrowLeft),
        "ArrowRight" => Some(IconName::ArrowRight),
        "ArrowUp" => Some(IconName::ArrowUp),
        // B
        "Asterisk" => Some(IconName::Asterisk),
        "Battery" => Some(IconName::Battery),
        "BatteryCharging" => Some(IconName::BatteryCharging),
        "BatteryFull" => Some(IconName::BatteryFull),
        "BatteryLow" => Some(IconName::BatteryLow),
        "BatteryMedium" => Some(IconName::BatteryMedium),
        "BatteryWarning" => Some(IconName::BatteryWarning),
        "Bell" => Some(IconName::Bell),
        "BookOpen" => Some(IconName::BookOpen),
        "Bot" => Some(IconName::Bot),
        "Building2" => Some(IconName::Building2),
        // C
        "Calendar" => Some(IconName::Calendar),
        "CaseSensitive" => Some(IconName::CaseSensitive),
        "ChartPie" => Some(IconName::ChartPie),
        "Check" => Some(IconName::Check),
        "ChevronDown" => Some(IconName::ChevronDown),
        "ChevronLeft" => Some(IconName::ChevronLeft),
        "ChevronRight" => Some(IconName::ChevronRight),
        "ChevronUp" => Some(IconName::ChevronUp),
        "ChevronsUpDown" => Some(IconName::ChevronsUpDown),
        "CircleCheck" => Some(IconName::CircleCheck),
        "CircleUser" => Some(IconName::CircleUser),
        "CircleX" => Some(IconName::CircleX),
        "Close" => Some(IconName::Close),
        "Copy" => Some(IconName::Copy),
        "Cpu" => Some(IconName::Cpu),
        // D-E
        "Dash" => Some(IconName::Dash),
        "Delete" => Some(IconName::Delete),
        "Ellipsis" => Some(IconName::Ellipsis),
        "EllipsisVertical" => Some(IconName::EllipsisVertical),
        "ExternalLink" => Some(IconName::ExternalLink),
        "Eye" => Some(IconName::Eye),
        "EyeOff" => Some(IconName::EyeOff),
        // F-G
        "File" => Some(IconName::File),
        "Folder" => Some(IconName::Folder),
        "FolderClosed" => Some(IconName::FolderClosed),
        "FolderOpen" => Some(IconName::FolderOpen),
        "Frame" => Some(IconName::Frame),
        "GalleryVerticalEnd" => Some(IconName::GalleryVerticalEnd),
        "Github" => Some(IconName::Github),
        "Globe" => Some(IconName::Globe),
        // H-I
        "HardDrive" => Some(IconName::HardDrive),
        "Heart" => Some(IconName::Heart),
        "HeartOff" => Some(IconName::HeartOff),
        "Inbox" => Some(IconName::Inbox),
        "Info" => Some(IconName::Info),
        "Inspector" => Some(IconName::Inspector),
        // L-M
        "LayoutDashboard" => Some(IconName::LayoutDashboard),
        "Loader" => Some(IconName::Loader),
        "LoaderCircle" => Some(IconName::LoaderCircle),
        "Map" => Some(IconName::Map),
        "Maximize" => Some(IconName::Maximize),
        "MemoryStick" => Some(IconName::MemoryStick),
        "Menu" => Some(IconName::Menu),
        "Minimize" => Some(IconName::Minimize),
        "Minus" => Some(IconName::Minus),
        "Moon" => Some(IconName::Moon),
        // N-P
        "Network" => Some(IconName::Network),
        "Palette" => Some(IconName::Palette),
        "PanelBottom" => Some(IconName::PanelBottom),
        "PanelBottomOpen" => Some(IconName::PanelBottomOpen),
        "PanelLeft" => Some(IconName::PanelLeft),
        "PanelLeftClose" => Some(IconName::PanelLeftClose),
        "PanelLeftOpen" => Some(IconName::PanelLeftOpen),
        "PanelRight" => Some(IconName::PanelRight),
        "PanelRightClose" => Some(IconName::PanelRightClose),
        "PanelRightOpen" => Some(IconName::PanelRightOpen),
        "Pause" => Some(IconName::Pause),
        "Play" => Some(IconName::Play),
        "Plus" => Some(IconName::Plus),
        // R-S
        "Redo" => Some(IconName::Redo),
        "Redo2" => Some(IconName::Redo2),
        "Replace" => Some(IconName::Replace),
        "ResizeCorner" => Some(IconName::ResizeCorner),
        "Search" => Some(IconName::Search),
        "Settings" => Some(IconName::Settings),
        "Settings2" => Some(IconName::Settings2),
        "SortAscending" => Some(IconName::SortAscending),
        "SortDescending" => Some(IconName::SortDescending),
        "SquareTerminal" => Some(IconName::SquareTerminal),
        "Star" => Some(IconName::Star),
        "StarFill" => Some(IconName::StarFill),
        "StarOff" => Some(IconName::StarOff),
        "Sun" => Some(IconName::Sun),
        // T-W
        "ThumbsDown" => Some(IconName::ThumbsDown),
        "ThumbsUp" => Some(IconName::ThumbsUp),
        "TriangleAlert" => Some(IconName::TriangleAlert),
        "Undo" => Some(IconName::Undo),
        "Undo2" => Some(IconName::Undo2),
        "User" => Some(IconName::User),
        "WindowClose" => Some(IconName::WindowClose),
        "WindowMaximize" => Some(IconName::WindowMaximize),
        "WindowMinimize" => Some(IconName::WindowMinimize),
        "WindowRestore" => Some(IconName::WindowRestore),
        _ => None,
    }
}
