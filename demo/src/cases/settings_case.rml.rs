use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.settings",
    kind = "case",
    group = "components",
    order = 75,
)]
#[component]
#[derive(Default)]
pub struct SettingsCase {
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub is_dark: bool,
    pub enable_notifications: bool,
    pub language: SharedString,
    pub language_options: Vec<(SharedString, SharedString)>,
    pub notify_email: SharedString,
    pub font_size: f64,
    pub auto_save: bool,
    pub username: SharedString,
}

impl IContribution for SettingsCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.settings.title")
    }
}

impl ILifecycle for SettingsCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        self.is_dark = false;
        self.enable_notifications = true;
        self.language = "zh-CN".into();
        self.language_options = vec![
            ("中文".into(), "zh-CN".into()),
            ("English".into(), "en-US".into()),
            ("日本語".into(), "ja-JP".into()),
        ];
        self.notify_email = "user@example.com".into();
        self.font_size = 14.0;
        self.auto_save = true;
        self.username = "anonymous".into();

        let (cols, rows) = build_api_table(&[
            ("sidebar-width", "px", "侧边栏宽度（默认 250px）"),
            ("group-variant", "normal/fill/outline", "分组框样式变体"),
            ("default-selected-page", "usize", "默认选中页面索引"),
            ("setting-page", "子节点", "设置页面，title/icon/description/default-open/resettable"),
            ("setting-group", "子节点", "设置分组，实现 Styled，title/description"),
            ("setting-item", "子节点", "设置项，title/field-type/value/on-change"),
            ("item.field-type", "switch/checkbox/input/dropdown/number-input", "字段类型选择 SettingField 构造器"),
            ("item.value", "绑定", "getter 闭包，读取 ViewModel 字段"),
            ("item.on-change", "事件", "setter 闭包，调用 ViewModel 方法"),
            ("item.options", "绑定", "dropdown 选项 Vec<(SharedString, SharedString)>"),
            ("item.default-value", "静态", "重置默认值"),
            ("item.min/max/step", "f64", "number-input 数值范围与步进"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl SettingsCase {
    fn on_dark_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.is_dark = val;
        cx.notify();
    }

    fn on_notifications_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.enable_notifications = val;
        cx.notify();
    }

    fn on_language_change(&mut self, val: SharedString, cx: &mut Context<Self>) {
        self.language = val;
        cx.notify();
    }

    fn on_email_change(&mut self, val: SharedString, cx: &mut Context<Self>) {
        self.notify_email = val;
        cx.notify();
    }

    fn on_font_size_change(&mut self, val: f64, cx: &mut Context<Self>) {
        self.font_size = val;
        cx.notify();
    }

    fn on_auto_save_change(&mut self, val: bool, cx: &mut Context<Self>) {
        self.auto_save = val;
        cx.notify();
    }

    fn on_username_change(&mut self, val: SharedString, cx: &mut Context<Self>) {
        self.username = val;
        cx.notify();
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("settings_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("settings_case.rml.rs").to_string()
    }
}
