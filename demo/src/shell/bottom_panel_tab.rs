//! Bottom 面板 Tab 视图模型 —— 解包 `IVisualContribution` 为 TabItem 数据。
//!
//! 供 `MainWindow.bottom_panel_tabs` 集合持有，
//! `render_bottom_panel` 按 order 排序后迭代生成 `TabItem`。

use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};

/// Bottom 面板 Tab 视图模型：解包 `kind = "bottom_tab"` 的视觉贡献。
#[derive(Clone)]
pub struct BottomPanelTabViewModel {
    pub name: SharedString,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
}

impl BottomPanelTabViewModel {
    /// 从贡献条目构造；非 `bottom_tab` 槽位或非视觉贡献返回 `None`。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("bottom_tab") {
            return None;
        }
        c.as_visual()?;
        Some(Self {
            name: c.name(),
            order: opts.order,
            contribution: c,
        })
    }

    /// 渲染 Tab body（委托给底层 `IVisualContribution`）。
    pub fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        self.contribution
            .as_visual()
            .expect("BottomPanelTabViewModel requires IVisualContribution")
            .render(window, cx)
    }
}

/// 从贡献条目列表构建 `BottomPanelTabViewModel` 列表（按 order 排序）。
pub fn build_bottom_panel_tabs(
    entries: &[(Arc<dyn IContribution>, ContributionOptions)],
) -> Vec<BottomPanelTabViewModel> {
    let mut tabs: Vec<_> = entries
        .iter()
        .filter_map(|(c, o)| BottomPanelTabViewModel::from_contribution(c.clone(), o.clone()))
        .collect();
    tabs.sort_by_key(|t| t.order);
    tabs
}
