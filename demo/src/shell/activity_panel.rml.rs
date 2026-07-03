use std::sync::Arc;

use gpui::{SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::TreeState;

use crate::shell::shell_chrome::{map_case_tree_items, VisualEntry};
use crate::shell::DemoShellHost;

/// ActivityPanel 双重角色：
/// - 视觉贡献（`#[contribute]`）：为 MainWindow 贡献活动栏面板
/// - Host（`#[contributehost]`）：接收案例贡献（kind="case"，视觉贡献）
///
/// `#[contribute]` + `#[contributehost]` + `#[component]` 叠加使用：
/// Entity 由框架实体缓存复用（`get_or_create_entity`），状态持久。
#[contribute(
    host_id = "demo.shell",
    id = "samples",
    kind = "activity",
    order = 0
)]
#[contributehost(id = "demo.activity")]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    // 视觉贡献存储：case 贡献（视觉，由 add_visual 受理）
    case_entries: std::sync::RwLock<Vec<VisualEntry>>,
    // host handle receiver
    host_rx: Option<rml_core::flume::Receiver<rml_app::contribution::HostOp>>,
}

impl IContribution for ActivityPanel {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.samples").into()
    }
    fn icon(&self) -> Option<SharedString> {
        Some("BookOpen".into())
    }
}

impl IContributionHost for ActivityPanel {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn add_visual(
        &self,
        contribution: Arc<dyn IVisualContribution>,
        options: ContributionOptions,
    ) {
        self.case_entries
            .write()
            .unwrap()
            .push((contribution, options));
    }

    fn remove(&self, contribution_id: &str) {
        let mut entries = self.case_entries.write().unwrap();
        entries.retain(|(c, _)| c.id() != contribution_id);
    }
}

impl ILifecycle for ActivityPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 1. 注册 host handle + 触发 demo.activity 的所有案例贡献注册（同步入队）
        let rx = Self::__rml_install_host(cx.entity(), cx);
        self.host_rx = Some(rx);

        // 2. drain 队列中的 HostOp → 调用自身 IContributionHost::add_visual
        if let Some(rx) = &self.host_rx {
            rml_app::contribution::drain_host_ops(rx, self);
        }

        // 3. 从 case_entries 构建树
        self.refresh_tree(cx);
        cx.notify();
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = {
            let entries = self.case_entries.read().unwrap();
            map_case_tree_items(&entries)
        };
        self.set_tree_items(items, cx);
    }

    fn set_tree_items(&mut self, items: Vec<rml_ui::TreeItem>, cx: &mut Context<Self>) {
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
