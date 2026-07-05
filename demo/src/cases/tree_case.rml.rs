use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TreeItem, TreeState};

#[contribute(
    host_id = "demo.shell",
    id = "components.tree",
    kind = "case",
    group = "components",
    order = 36,
)]
#[component]
#[derive(Default)]
pub struct TreeCase {
    pub tree_state: Option<gpui::Entity<TreeState>>,
    pub last_activated: SharedString,
}

impl IContribution for TreeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.tree.title")
    }
}

impl ILifecycle for TreeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        let items = vec![
            TreeItem::new("group1", "分组一")
                .expanded(true)
                .child(TreeItem::new("item1", "选项 1"))
                .child(TreeItem::new("item2", "选项 2")),
            TreeItem::new("group2", "分组二")
                .child(TreeItem::new("item3", "选项 3"))
                .child(TreeItem::new("item4", "选项 4")),
            TreeItem::new("item5", "独立项"),
        ];
        self.tree_state = Some(cx.new(|cx| TreeState::new(cx).items(items)));
    }
}

impl TreeCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_activated.is_empty() {
            "请点击树节点".to_string()
        } else {
            format!("已激活：{}", self.last_activated)
        }
    }

    #[command]
    pub fn on_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        self.last_activated = item_id.clone();
        cx.notify();
    }
}
