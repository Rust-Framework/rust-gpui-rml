use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar",
    kind = "case",
    group = "components",
    order = 13,
)]
#[component]
#[derive(Default)]
pub struct AvatarCase {
    pub name: String,
    pub size_index: u8,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for AvatarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar.title")
    }
}

impl ILifecycle for AvatarCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.name = "Jason Lee".into();
        let (cols, rows) = build_api_table(&[
            ("src", "URL 字符串/绑定", "图片地址"),
            ("name", "字符串/绑定", "取首字母显示"),
            ("placeholder", "IconName 枚举名", "占位图标"),
            ("size", "xsmall/small/medium/large", "尺寸"),
            ("on-click", "事件", "点击回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AvatarCase {
    #[computed]
    pub fn size_label(&self) -> &'static str {
        match self.size_index % 3 {
            0 => "small",
            1 => "medium",
            _ => "large",
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- avatar_case.rml：声明式 UI，三种内容模式 + 尺寸 + 动态绑定 -->
<component>
    <!-- src 图片源 -->
    <Avatar src="https://..." size="large" />

    <!-- name 首字母（"Jason Lee" → "JL"） -->
    <Avatar name="Jason Lee" />

    <!-- placeholder 占位图标（IconName 枚举名） -->
    <Avatar placeholder="UserCircle" />

    <!-- 4 种尺寸 -->
    <Avatar name="XS" size="xsmall" />
    <Avatar name="M" size="medium" />

    <!-- 绑定：name={field} -->
    <Avatar name={user_name} size="large" />

    <!-- AvatarGroup 分组 -->
    <AvatarGroup limit="3" ellipsis="">
        <Avatar name="Alice" />
        <Avatar name="Bob" />
    </AvatarGroup>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// avatar_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct AvatarCase {
    pub name: String,           // model 双向绑定的字段
    pub size_index: u8,         // 状态字段
}

impl ILifecycle for AvatarCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.name = "Jason Lee".into();   // 初始化默认值
    }
}

impl AvatarCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 引用
    #[computed]
    pub fn size_label(&self) -> &'static str {
        match self.size_index % 3 {
            0 => "small",
            1 => "medium",
            _ => "large",
        }
    }

    // #[command] 标注的方法可被 on-click={on_cycle_size} 调用
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.size_index = self.size_index.wrapping_add(1);
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.size_index = self.size_index.wrapping_add(1);
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
