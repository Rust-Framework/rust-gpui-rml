//! 案例目录 —— Tab 元信息

use gpui::SharedString;
use rml_core::contribution::IContribution;

/// 已打开的 Tab 页签（实现 IContribution，由 TabWindowShell 直接消费）
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OpenTab {
    pub id: String,
    pub title: String,
}

impl IContribution for OpenTab {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.title.clone().into()
    }
}

/// 案例标题 i18n key
pub fn case_title_key(id: &str) -> &'static str {
    match id {
        "welcome" => "shell.welcome",
        "binding.counter" => "case.counter.title",
        "binding.two-way" => "case.two_way.title",
        "components.button" => "case.button.title",
        "components.accordion" => "case.accordion.title",
        "components.tab_bar" => "case.tab_bar.title",
        "components.avatar" => "case.avatar.title",
        "components.slot" => "case.slot.title",
        "components.table" => "case.table.title",
        "components.menu.context" => "case.menu.context.title",
        "components.menu.dropdown" => "case.menu.dropdown.title",
        "components.menu.editor" => "case.menu.editor.title",
        "components.menu.features" => "case.menu.features.title",
        "components.menu.custom" => "case.menu.custom.title",
        "components.status_bar" => "case.status_bar.title",
        "i18n.basic" => "case.i18n.title",
        _ => "shell.case_default",
    }
}
