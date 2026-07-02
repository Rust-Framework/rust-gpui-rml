use gpui::Window;
use rml::prelude::*;
use rml_ui::TreeState;

use crate::shell::shell_chrome::map_case_tree_items;
use crate::shell::DemoShellHost;

/// ActivityPanel 双重角色：
/// - 视觉贡献（`#[contribute]`）：为 MainWindow 贡献活动栏面板
/// - Host（`#[contributehost]`）：接收案例贡献（kind="case"）
///
/// `#[contribute]` + `#[contributehost]` + `#[component]` 叠加使用：
/// Entity 由框架实体缓存复用（`get_or_create_entity`），状态持久。
#[contribute(
    host_id = "demo.shell",
    id = "samples",
    name = "shell.samples",
    icon = IconName::BookOpen,
    kind = "activity",
    order = 0
)]
#[contributehost(id = "demo.activity")]
#[component]
#[derive(Default)]
pub struct ActivityPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
    // entries, i18n_version ← #[contributehost] 自动注入
}

// 无 impl IContributionHost —— #[contributehost] 宏自动生成
// 无 impl ILifecycle —— #[contributehost] 宏自动生成

impl IHostEntity for ActivityPanel {
    fn host_on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
    }

    fn on_locale_changed(&mut self, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
    }
}

impl ActivityPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let items = {
            let entries = self.entries.read();
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
