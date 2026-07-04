use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TreeItem, TreeState};

use crate::shell::MainWindowRef;

/// LspExplorerPanel：活动栏贡献，加载 demo 源码目录树形显示。
///
/// 点击文件 → IAppContext::get_service::<MainWindowRef>() → MainWindow::open_lsp_file。
#[contribute(
    host_id = "demo.shell",
    id = "lsp_explorer",
    kind = "activity",
    order = 10
)]
#[component]
#[derive(Default)]
pub struct LspExplorerPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
}

impl IContribution for LspExplorerPanel {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.lsp_explorer")
    }
    fn icon(&self) -> Option<SharedString> {
        Some("FileCode".into())
    }
}

impl ILifecycle for LspExplorerPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let items = crate::lsp::file_tree::build_source_tree();
        self.set_tree_items(items, cx);
        cx.notify();
    }
}

impl LspExplorerPanel {
    fn set_tree_items(&mut self, items: Vec<TreeItem>, cx: &mut Context<Self>) {
        if let Some(state) = self.tree_state.as_ref() {
            state.update(cx, |s, cx| {
                s.set_items(items, cx);
            });
        } else {
            let state = cx.new(|cx| TreeState::new(cx).items(items));
            self.tree_state = Some(state);
        }
    }

    #[command]
    pub fn on_file_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        let path = item_id.to_string();
        if path.ends_with(".rs") || path.ends_with(".rml") {
            if let Some(host) = cx
                .get_service::<MainWindowRef>()
                .and_then(|r| r.0.upgrade())
            {
                host.update(cx, |main, cx| {
                    main.open_lsp_file(path, cx);
                });
            }
        }
    }
}
