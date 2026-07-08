use std::sync::Arc;
use std::sync::Once;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// DescriptionList items 绑定的演示数据项。
/// name() → label，id() → value（通过 as_contribution() 能力查询提取）。
pub struct DescEntry {
    name: SharedString,
    id: String,
}

impl IContribution for DescEntry {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.name.clone()
    }
}

static DESC_ENTRY_REGISTERED: Once = Once::new();

fn ensure_desc_entry_registered() {
    DESC_ENTRY_REGISTERED.call_once(|| {
        register_contribution_ability::<DescEntry>();
    });
}

#[contribute(
    host_id = "demo.shell",
    id = "components.description_list",
    kind = "case",
    group = "components",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct DescriptionListCase {
    pub user_name: String,
    pub user_email: String,
    pub role: String,
    pub width: gpui::Pixels,
    pub is_vertical: bool,
    pub desitems: Vec<Arc<dyn IValue>>,
    pub list_api_columns: Vec<TableColumn>,
    pub list_api_rows: Vec<TableRow>,
    pub item_api_columns: Vec<TableColumn>,
    pub item_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for DescriptionListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.description_list.title")
    }
}

impl ILifecycle for DescriptionListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        ensure_desc_entry_registered();
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = gpui::px(120.0);
        self.is_vertical = true;
        let (cols, rows) = build_api_table(&[
            ("vertical", "布尔/绑定", "纵向布局（默认横向）"),
            ("bordered", "布尔标志", "显示边框"),
            ("columns", "数字", "列数"),
            ("label-width", "像素值", "标签列宽"),
            ("items", "绑定", "批量数据绑定（Vec<Arc<dyn IValue>>）"),
        ]);
        self.list_api_columns = cols;
        self.list_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "子项标签（必填）"),
            ("value", "字符串/绑定/元素", "子项值"),
            ("span", "数字", "跨列数"),
        ]);
        self.item_api_columns = cols;
        self.item_api_rows = rows;

        self.desitems = vec![
            Arc::new(DescEntry { name: "产品名称".into(), id: "RML 框架".into() }),
            Arc::new(DescEntry { name: "版本".into(), id: "1.0.0".into() }),
            Arc::new(DescEntry { name: "许可证".into(), id: "MIT".into() }),
            Arc::new(DescEntry { name: "作者".into(), id: "Rust 社区".into() }),
        ];
    }
}

impl DescriptionListCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("description_list_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("description_list_case.rml.rs").to_string()
    }
}
