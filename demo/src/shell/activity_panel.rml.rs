use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_app::IServiceProvider;
use rml_core::contribution::IconSpec;
use rml_core::element_ref::ElementRef;
use rml_core::i18n::t_static;
use rml_ui::{TreeData, TreeState};

use crate::shell::case_view_model::CaseViewModel;
use crate::shell::MainWindowRef;

/// ActivityPanel：纯视觉贡献，从 MainWindow.cases 集合构建 Tree。
///
/// 不再担任 host 角色（cases 现注册到 `demo.shell`，由 MainWindow 受理）。
/// 点击案例 → IAppContext::get_service::<MainWindowRef>() → MainWindow::open_case。
#[contribute(
    host_id = "demo.shell",
    id = "samples",
    kind = "activity",
    order = 0
)]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: ElementRef<TreeState>,
    tree_items: Vec<TreeData>,
    main: Option<gpui::WeakEntity<crate::shell::MainWindow>>,
}

impl IContribution for ActivityPanel {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.samples")
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("BookOpen"))
    }
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(host) = cx.get_service::<MainWindowRef>() {
            self.main = Some(host.0.clone());
        }
        self.refresh_tree(cx);
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = if let Some(main) = self.main.as_ref().and_then(|w| w.upgrade()) {
            let cases = main.read(cx).cases.clone();
            CaseViewModel::build_tree_items(&cases)
        } else {
            Vec::new()
        };
        self.tree_items = items;
        cx.notify();
    }

    #[command]
    pub fn on_case_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        if let Some(host) = cx
            .get_service::<MainWindowRef>()
            .and_then(|r| r.0.upgrade())
        {
            host.update(cx, |main, cx| {
                main.open_case(item_id.to_string(), cx);
            });
        }
    }
}
