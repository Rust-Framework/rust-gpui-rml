use rml::prelude::*;
use rml_app::contribution::subscribe_host_changes;
use crate::shell::shell_chrome::map_case_tree_items;
use rml_core::i18n::I18nState;
use rml_ui::TreeState;

use crate::shell::{DemoShellHost, MainWindow};

#[contribute(
    host_id = "demo.shell",
    id = "samples",
    name = "shell.samples",
    icon = IconName::BookOpen,
    kind = "activity",
    order = 0,
)]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let _ = window;
        self.refresh_tree(cx);
        subscribe_host_changes(MainWindow::ID, cx, |this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        });
        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_tree(cx);
            cx.notify();
        })
        .detach();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = map_case_tree_items(MainWindow::ID, cx);
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| {
                s.set_items(items, cx);
            });
            cx.notify();
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        if let Some(host) = cx
            .try_global::<DemoShellHost>()
            .and_then(|h| h.0.upgrade())
        {
            host.update(cx, |main, cx| {
                main.open_case(item_id.to_string(), cx);
            });
        }
    }
}
