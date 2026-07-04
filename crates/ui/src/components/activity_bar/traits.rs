//! ActivityBar 扩展接口 —— 视觉贡献 / 命令贡献的活动栏特化
//!
//! 两个 trait 均为空扩展：图标沿用 `IContribution::icon`（字符串），由 `icon::resolve_icon`
//! 在渲染时解析为 `AnyElement`（URL 文件地址 → `Svg::external_path`；非 URL → 内置 `IconName`）。

use rml_core::command::ICommand;
use rml_core::contribution::IVisualContribution;

/// 活动栏面板扩展接口。
///
/// `IActivityPanel: IVisualContribution`——面板本身是视觉贡献(`IContribution + IVisual`)：
/// - `IContribution::id` / `name` 提供元数据（`name` 作按钮 tooltip）
/// - `IContribution::icon` 提供图标字符串（URL 文件地址或内置图标名）
/// - `IVisual::render` 提供面板内容
pub trait IActivityPanel: IVisualContribution {}

/// 活动栏底部动作扩展接口。
///
/// `IActivityAct: ICommand`——动作本身是命令：
/// - `IContribution::id` / `name` 提供元数据（`name` 作按钮 tooltip）
/// - `IContribution::icon` 提供图标字符串（URL 文件地址或内置图标名）
/// - `ICommand::execute` 提供点击行为
pub trait IActivityAct: ICommand {}
