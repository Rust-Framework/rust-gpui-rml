use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.tab_bar",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct TabBarCase {
    pub active_tab: usize,
}

impl IContribution for TabBarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tab_bar.title").into()
    }
}

impl ILifecycle for TabBarCase {}

impl TabBarCase {
    #[computed]
    pub fn status_text(&self) -> String {
        format!("当前选中索引：{}", self.active_tab)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<TabBar selected_index={active_tab} on_click={on_tab_select}>
    <Tab label="Account" />
    <Tab label="Profile" />
</TabBar>"#
            .to_string()
    }

    #[command]
    pub fn on_tab_select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.active_tab = index;
        cx.notify();
    }
}
