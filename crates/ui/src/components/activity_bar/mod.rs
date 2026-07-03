//! ActivityBar 模块 —— VS Code 风格左侧活动栏（单 Entity 模型）
//!
//! 架构：
//! - 单 `ActivityBar` Entity：同时渲染图标栏 + 面板内容
//! - `set_active_id` 直接修改字段 + `cx.notify()` 触发自身重渲
//! - 无 EventEmitter、无 SidePanel、无 Shell
//!
//! RML 用法：`<ActivityBar ref="activity_bar" />`
//! Host 在 `on_loaded` 中 `cx.new(|_| ActivityBar::new(panels))` 创建并激活首项。

mod act;
mod bar;
mod icon;
mod panel;
mod traits;
mod visual_panel;

pub use act::ActivityAct;
pub use bar::ActivityBar;
pub use panel::ActivityPanel;
pub use traits::{IActivityAct, IActivityPanel};
pub use visual_panel::VisualActivityPanel;
