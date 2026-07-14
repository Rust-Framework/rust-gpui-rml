//! 状态栏视图模型 —— 解包 `(IVisualContribution, ContributionOptions)` 为类型化结构。
//!
//! 供 MainWindow.status 集合持有，RML
//! `<component each={s in status_left} content={s.render(_window, cx)} />` 直接消费。

use std::sync::Arc;

use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};
use rml_ui::StatusBarAlign;

/// 贡献条目类型别名
pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);

#[derive(Clone)]
pub struct StatusViewModel {
    pub align: StatusBarAlign,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
}

impl StatusViewModel {
    /// 从贡献条目构造；非 status 槽位或非视觉贡献返回 `None`。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("status") {
            return None;
        }
        c.as_visual()?;
        let align = match opts.properties.get("align").map(|s| s.as_ref()) {
            Some("right") => StatusBarAlign::Right,
            Some("center") => StatusBarAlign::Center,
            _ => StatusBarAlign::Left,
        };
        Some(Self {
            align,
            order: opts.order,
            contribution: c,
        })
    }

    /// 渲染状态栏项（委托给底层 `IVisualContribution`）。
    pub fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        self.contribution
            .as_visual()
            .expect("StatusViewModel requires IVisualContribution")
            .render(window, cx)
    }
}

/// 从贡献条目列表构建 `StatusViewModel` 列表（按 order 排序）。
pub fn build_status_view_models(entries: &[ContribEntry]) -> Vec<StatusViewModel> {
    let mut items: Vec<StatusViewModel> = entries
        .iter()
        .filter_map(|(c, o)| StatusViewModel::from_contribution(c.clone(), o.clone()))
        .collect();
    items.sort_by_key(|s| s.order);
    items
}
