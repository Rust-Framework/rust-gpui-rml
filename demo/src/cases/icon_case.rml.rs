use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{IconName, TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.icon",
    kind = "case",
    group = "components",
    order = 39,
)]
#[component]
#[derive(Default)]
pub struct IconCase {
    pub icon_index: u32,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for IconCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.icon.title")
    }
}

impl ILifecycle for IconCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.icon_index = 0;
        let (cols, rows) = build_api_table(&[
            ("name", "IconName 枚举名", "图标名称（如 Settings/Bell/User），生成 Icon::new(IconName::Settings)"),
            ("path", "字符串", "自定义图标路径（如 icons/foo.svg），生成 Icon::empty().path(...)"),
            ("size", "枚举", "Sizable 尺寸：xsmall/small/medium/large"),
            ("text_color", "颜色", "来自 Styled trait，设置图标颜色"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl IconCase {
    #[computed]
    pub fn current_icon(&self) -> IconName {
        match self.icon_index % 3 {
            0 => IconName::Settings,
            1 => IconName::Bell,
            _ => IconName::User,
        }
    }

    #[computed]
    pub fn current_icon_name(&self) -> &'static str {
        match self.icon_index % 3 {
            0 => "Settings",
            1 => "Bell",
            _ => "User",
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- icon_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：name="Settings" → Icon::new(IconName::Settings) -->
    <Icon name="Settings" />
    <Icon name="Bell" />
    <Icon name="User" />

    <!-- 尺寸 size（走通用 Sizable setter） -->
    <Icon name="Settings" size="xsmall" />
    <Icon name="Settings" size="small" />
    <Icon name="Settings" size="medium" />
    <Icon name="Settings" size="large" />

    <!-- 动态绑定：name={current_icon} 绑定 computed 返回的 IconName 枚举 -->
    <Icon name={current_icon} size="large" />

    <!-- 自定义路径：path="icons/custom.svg" → Icon::empty().path(...) -->
    <Icon path="icons/custom.svg" size="medium" />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// icon_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;
use rml_ui::IconName;

#[component]
#[derive(Default)]
pub struct IconCase {
    pub icon_index: u32,
}

impl IconCase {
    // computed 方法返回 IconName 枚举，供 name={current_icon} 绑定
    #[computed]
    pub fn current_icon(&self) -> IconName {
        match self.icon_index % 3 {
            0 => IconName::Settings,
            1 => IconName::Bell,
            _ => IconName::User,
        }
    }

    #[command]
    pub fn on_rotate_icon(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.icon_index = self.icon_index.saturating_add(1);
        cx.notify();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_rotate_icon(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.icon_index = self.icon_index.saturating_add(1);
        cx.notify();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
