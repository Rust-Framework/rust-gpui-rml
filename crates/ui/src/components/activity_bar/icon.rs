//! 活动栏图标渲染 —— `IContribution::icon` 返回的 `IconSpec` → 可渲染图标元素
//!
//! `IconSpec` 的 variant tag 直接决定渲染路径,无需字符串推断:
//! 1. `Named(s)` → 查 `parse_icon_name` 表得 `IconName`,用 `Icon::new(name)`;
//!    未匹配时 fallback `IconName::PanelLeft`
//! 2. `Path(s)` → `Icon::default().path(s)`,经 `CompositeAssets` 路由,
//!    同时支持 gpui-component 内置 `icons/**/*.svg` 与 RML 用户嵌入资源(`assets/logo.svg` 等)
//! 3. `Url(s)` → `gpui::img(s).size_5()`,加载外部/文件 URL 图片
//! 4. `None` → fallback `IconName::PanelLeft`
//!
//! `Icon` 实例使用默认尺寸(18px,未调用 `Sizable::small()`),与 36px 按钮容器比例协调。

use gpui::{AnyElement, IntoElement, Styled, Window, img};
use gpui_component::{Icon, IconName};
use rml_core::contribution::IconSpec;

/// 渲染贡献点 `IContribution::icon` 返回的 `IconSpec` 为可渲染图标元素。
///
/// 按 variant tag 直接分派,无需 `is_url`/`is_asset_path` 等字符串推断:
/// - `Named` → 内置 `IconName` 枚举(查表未命中走 fallback)
/// - `Path` → `Icon::default().path(s)`(经 `CompositeAssets` 支持嵌入资源)
/// - `Url` → `gpui::img`(外部图片,Icon 无法加载 URL)
/// - `None` → fallback `IconName::PanelLeft`
pub fn resolve_icon(spec: Option<IconSpec>, window: &Window) -> AnyElement {
    match spec {
        Some(IconSpec::Named(s)) => match parse_icon_name(&s) {
            Some(name) => Icon::new(name).into_any_element(),
            None => Icon::new(IconName::PanelLeft).into_any_element(),
        },
        Some(IconSpec::Path(s)) => Icon::default().path(s).into_any_element(),
        Some(IconSpec::Url(s)) => {
            let text_color = window.text_style().color;
            img(s)
                .flex_shrink_0()
                .size_5()
                .text_color(text_color)
                .into_any_element()
        }
        None => Icon::new(IconName::PanelLeft).into_any_element(),
    }
}

/// 解析图标名字符串 → `IconName`。
///
/// `IconName` 由 `icon_named!` 宏从 `gpui-component-assets` 的 SVG 文件名生成(kebab-case → PascalCase),
/// 未实现 `FromStr`。此处维护完整映射,覆盖 assets 目录下全部 SVG 图标。
///
/// 未匹配时返回 `None`,调用方走 fallback `IconName::PanelLeft`。
///
/// # 后续优化路径
///
/// 此映射表是 `IconSpec::Named(SharedString)` 设计选择的副作用——`rml_core` 不依赖
/// `gpui-component`(框架中立),故 `Named` 载荷为字符串而非 `IconName`。彻底消除此表
/// 的方案是给上游 `gpui-component` 的 `icon_named!` 宏加 `FromStr` impl 生成,一次性投入。
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
