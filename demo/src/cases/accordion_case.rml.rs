use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.accordion",
    kind = "case",
    group = "components",
    order = 10,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    /// Section 1：基础用法（单展开模式）
    /// open-indices 绑定 Vec<usize>，初始 vec![0] 表示第一项展开
    pub basic_open: Vec<usize>,

    /// Section 2：multiple 多展开
    /// 初始 vec![0, 1]，两项同时展开
    pub multiple_open: Vec<usize>,

    /// Section 3：size 尺寸（small / large）
    pub sizes_small_open: Vec<usize>,
    pub sizes_large_open: Vec<usize>,

    /// Section 4：icon + disabled
    pub with_icon_open: Vec<usize>,

    /// Section 5：嵌套 accordion
    pub nested_open: Vec<usize>,
    pub nested_child_open: Vec<usize>,

    /// 记录上次展开项索引（on-toggle-click 回调更新）
    pub last_open: String,

    pub accordion_api_columns: Vec<TableColumn>,
    pub accordion_api_rows: Vec<TableRow>,
    pub item_api_columns: Vec<TableColumn>,
    pub item_api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for AccordionCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.accordion.title")
    }
}

impl ILifecycle for AccordionCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        // 受控模式下初始展开态由状态字段决定，避免在 item 上硬编码 open 导致点击无法收起
        self.basic_open = vec![0];
        self.multiple_open = vec![0, 1];

        let (cols, rows) = build_api_table(&[
            ("bordered", "布尔标志", "显示边框（bordered=\"\" 为 true）"),
            ("multiple", "布尔标志", "允许多项同时展开（multiple=\"\" 为 true）"),
            ("size", "small/medium/large", "尺寸变体（Sizable trait 通用属性）"),
            ("open-indices", "Vec<usize> 绑定", "展开项索引列表（受控模式核心属性）"),
            ("on-toggle-click", "事件", "展开状态变化回调（参数：&[usize]）"),
        ]);
        self.accordion_api_columns = cols;
        self.accordion_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("title", "字符串/绑定", "面板标题"),
            ("icon", "IconName 枚举值", "标题图标（如 Settings/Bell）"),
            ("disabled", "布尔", "禁用面板（disabled=\"true\" 禁用）"),
        ]);
        self.item_api_columns = cols;
        self.item_api_rows = rows;
    }
}

impl AccordionCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("accordion_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("accordion_case.rml.rs").to_string()
    }

    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    /// on-toggle-click 回调签名：(&[usize], &mut Context<Self>)
    /// 展开状态变化时触发，open_indices 为当前展开项索引列表
    #[command]
    pub fn on_toggle(&mut self, open_indices: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_indices);
        cx.notify();
    }
}
