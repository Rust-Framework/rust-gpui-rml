use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_core::contribution::IconSpec;
use rml_core::element_ref::ElementRef;
use rml_core::i18n::t_static;
use rml_ui::{TreeData, TreeState};

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
    tree_state: ElementRef<TreeState>,
    tree_items: Vec<TreeData>,
}

impl IContribution for LspExplorerPanel {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.lsp_explorer")
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("FileCode"))
    }
}

impl ILifecycle for LspExplorerPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.tree_items = crate::lsp::file_tree::build_source_tree();
        cx.notify();
    }
}

impl LspExplorerPanel {
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
