use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.avatar_group",
    kind = "case",
    group = "components",
    order = 29,
)]
#[component]
#[derive(Default)]
pub struct AvatarGroupCase {
    pub avatar_count: u8,
    pub code_tab: usize,
    pub group_api_columns: Vec<TableColumn>,
    pub group_api_rows: Vec<TableRow>,
    pub avatar_api_columns: Vec<TableColumn>,
    pub avatar_api_rows: Vec<TableRow>,
}

impl IContribution for AvatarGroupCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar_group.title")
    }
}

impl ILifecycle for AvatarGroupCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.avatar_count = 3;
        let (cols, rows) = build_api_table(&[
            ("limit", "数字", "最大显示数量"),
            ("ellipsis", "布尔标志", "溢出显示 +N"),
        ]);
        self.group_api_columns = cols;
        self.group_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("src", "字符串", "图片源 URL"),
            ("name", "字符串", "首字母 fallback"),
            ("placeholder", "图标名", "占位图标"),
        ]);
        self.avatar_api_columns = cols;
        self.avatar_api_rows = rows;
    }
}

impl AvatarGroupCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- avatar_group_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：包裹多个 Avatar 子节点 -->
    <AvatarGroup>
        <Avatar name="Alice" />
        <Avatar name="Bob" />
        <Avatar name="Charlie" />
    </AvatarGroup>

    <!-- 数量限制 + 溢出折叠：limit + ellipsis -->
    <AvatarGroup limit="2" ellipsis="">
        <Avatar name="Alice" />
        <Avatar name="Bob" />
        <Avatar name="Charlie" />
        <Avatar name="Dave" />
    </AvatarGroup>

    <!-- 动态 if 条件渲染：根据 avatar_count 增减头像 -->
    <AvatarGroup limit="5" ellipsis="">
        <Avatar name="Alice" if={avatar_count >= 1} />
        <Avatar name="Bob" if={avatar_count >= 2} />
        <Avatar name="Charlie" if={avatar_count >= 3} />
    </AvatarGroup>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// avatar_group_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct AvatarGroupCase {
    pub avatar_count: u8,
}

impl ILifecycle for AvatarGroupCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.avatar_count = 3;
    }
}

impl AvatarGroupCase {
    #[command]
    pub fn on_add_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count < 5 {
            self.avatar_count += 1;
        }
    }

    #[command]
    pub fn on_remove_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count > 1 {
            self.avatar_count -= 1;
        }
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_add_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count < 5 {
            self.avatar_count += 1;
        }
    }

    #[command]
    pub fn on_remove_avatar(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        if self.avatar_count > 1 {
            self.avatar_count -= 1;
        }
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
