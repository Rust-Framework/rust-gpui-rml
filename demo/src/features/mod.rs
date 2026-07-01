//! 功能模块 —— 自注册贡献点

mod case_tree;
pub mod navigation;
mod samples_panel;
mod status_text;

use gpui::App;

use crate::shell::hosts;

/// 注册所有功能模块贡献
pub fn register_all(cx: &mut App) {
    case_tree::register_case_tree(cx);
    samples_panel::__rml_register_samplespanel(cx);
    status_text::register_status_text(cx);
}

/// Demo 应用在启动时预创建需监听变更的 host（可选；首次 register 也会自动创建）
pub fn ensure_hosts(cx: &mut gpui::App) {
    use gpui::BorrowAppContext;
    use rml_app::contribution::ContributionRegistryGlobal;

    cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
        global.0.ensure_host(hosts::ACTIVITY_BAR);
        global.0.ensure_host(hosts::STATUS);
        global.0.ensure_host(hosts::CASE_TREE);
    });
}
