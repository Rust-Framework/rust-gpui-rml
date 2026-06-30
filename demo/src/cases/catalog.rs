//! 案例目录 —— Tree 数据与元信息

use gpui::AppContext;
use rml_core::i18n::I18nExt;
use rml_ui::{TreeItem, TreeState};

/// 已打开的 Tab 页签
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenTab {
    pub id: String,
    pub title: String,
}

/// 案例标题 i18n key
pub fn case_title_key(id: &str) -> &'static str {
    match id {
        "binding.counter" => "case.counter.title",
        "binding.two-way" => "case.two_way.title",
        "components.button" => "case.button.title",
        "i18n.basic" => "case.i18n.title",
        _ => "shell.case_default",
    }
}

/// 构建按分类组织的案例树（标签通过 `cx.t` 本地化）
pub fn tree_items<C>(cx: &gpui::Context<C>) -> Vec<TreeItem> {
    vec![
        TreeItem::new("cat.binding", cx.t("tree.cat.binding"))
            .expanded(true)
            .child(TreeItem::new("binding.counter", cx.t("tree.case.counter")))
            .child(TreeItem::new("binding.two-way", cx.t("tree.case.two_way"))),
        TreeItem::new("cat.components", cx.t("tree.cat.components"))
            .expanded(true)
            .child(TreeItem::new("components.button", cx.t("tree.case.button"))),
        TreeItem::new("cat.i18n", cx.t("tree.cat.i18n"))
            .expanded(true)
            .child(TreeItem::new("i18n.basic", cx.t("tree.case.i18n"))),
    ]
}

/// 在 `on_loaded` 中初始化案例树状态
pub fn init_tree_state<C>(cx: &mut gpui::Context<C>) -> gpui::Entity<TreeState> {
    cx.new(|cx| TreeState::new(cx).items(tree_items(cx)))
}

/// 切换语言后刷新案例树
pub fn refresh_tree_state<C>(state: &gpui::Entity<TreeState>, cx: &mut gpui::Context<C>) {
    state.update(cx, |tree, cx| {
        tree.set_items(tree_items(cx), cx);
    });
}
