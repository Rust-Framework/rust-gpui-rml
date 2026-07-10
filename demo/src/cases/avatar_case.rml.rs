use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    /// value 双向绑定的姓名字段，配合 input + Avatar name={name} 实时联动
    pub name: String,
    /// 尺寸循环索引（0/1/2 → small/medium/large），配合 if 指令演示条件渲染
    pub size_index: u8,
    pub avatar_api_columns: Vec<TableColumn>,
    pub avatar_api_rows: Vec<TableRow>,
    pub group_api_columns: Vec<TableColumn>,
    pub group_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
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
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.name = "Jason Lee".into();

        let (cols, rows) = build_api_table(&[
            ("src", "URL 字符串/绑定", "图片地址（最高优先级，加载失败回退到 name/placeholder）"),
            ("name", "字符串/绑定", "取首字母显示（如 Jason Lee → JL）"),
            ("placeholder", "IconName 枚举名", "占位图标（无 src/name 时使用）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("on-click", "事件", "点击回调"),
        ]);
        self.avatar_api_columns = cols;
        self.avatar_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("limit", "数字/绑定", "限制显示的 Avatar 数量"),
            ("ellipsis", "布尔标志", "溢出折叠（显示 +N 提示）"),
        ]);
        self.group_api_columns = cols;
        self.group_api_rows = rows;
    }
}

impl AvatarCase {
    /// 根据 size_index % 3 返回当前尺寸标签，配合 if 指令演示条件渲染
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
        include_str!("avatar_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("avatar_case.rml.rs").to_string()
    }

    /// 循环切换尺寸索引：0 → 1 → 2 → 0 ...
    /// wrapping_add 避免 u8 溢出，配合 size_index % 3 实现循环
    #[command]
    pub fn on_cycle_size(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.size_index = self.size_index.wrapping_add(1);
    }
}
