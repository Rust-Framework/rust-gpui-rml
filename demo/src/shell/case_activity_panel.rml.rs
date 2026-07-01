use rml::prelude::*;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::i18n::I18nState;
use rml_ui::TreeState;

use crate::shell::contributions;

#[contribute(
    host = "demo.shell",
    id = "samples",
    name = "shell.samples",
    icon = IconName::BookOpen,
    mode = Panel,
    kind = "activity",
    order = 0,
)]
#[component]
#[derive(Default)]
pub struct CaseActivityPanel {
    case_tree_state: Option<gpui::Entity<TreeState>>,
}

impl ILifecycle for CaseActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
        cx.observe_global::<ContributionRegistryGlobal>(|this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        })
        .detach();
        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        })
        .detach();
    }
}

impl CaseActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = contributions::build_case_tree_items(cx);
        if let Some(state) = self.case_tree_state.as_ref() {
            state.update(cx, |s, cx| {
                s.set_items(items, cx);
            });
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.case_tree_state = Some(state);
        }
        cx.notify();
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        crate::shell::main_window::activate_case(item_id.to_string(), cx);
    }
}
