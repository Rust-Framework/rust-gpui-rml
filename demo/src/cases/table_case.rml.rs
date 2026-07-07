use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.table",
    kind = "case",
    group = "components",
    order = 15,
)]
#[component]
#[derive(Default)]
pub struct TableCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub column_api_columns: Vec<TableColumn>,
    pub column_api_rows: Vec<TableRow>,
    pub slot_api_columns: Vec<TableColumn>,
    pub slot_api_rows: Vec<TableRow>,
    pub user_rows: Vec<TableRow>,
    pub merged_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TableCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.table.title")
    }
}

impl ILifecycle for TableCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("columns", "Vec<TableColumn> 绑定", "数据绑定式列定义（与 Column 子标签二选一）"),
            ("rows", "Vec<TableRow> 绑定", "行数据绑定"),
            ("bordered", "布尔标志", "显示边框"),
            ("stripe", "布尔标志", "斑马纹样式"),
            ("delegate", "Rc<dyn TableDelegate>", "模板委托（自定义渲染，高级用法）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("key", "字符串", "列字段标识（对应 TableRow::cell(key, value) 的 key）"),
            ("title", "字符串", "列标题（显示在列头）"),
            ("width", "数字字符串", "列宽（像素，如 width=\"120\"）"),
            ("align", "left/center/right", "列对齐方式（默认 left）"),
        ]);
        self.column_api_columns = cols;
        self.column_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("template slot=\"header\"", "slot", "自定义列头模板（替换默认列头渲染）"),
            ("template slot=\"footer\"", "slot", "表格底部插槽（如统计信息、分页等）"),
            ("template slot=\"cell\" field=\"name\"", "scoped slot", "单元格模板（field 指定列，模板内可引用 row_idx 闭包参数）"),
        ]);
        self.slot_api_columns = cols;
        self.slot_api_rows = rows;

        self.user_rows = vec![
            TableRow::new()
                .cell("name", "张三")
                .cell("age", "28")
                .cell("email", "zhangsan@example.com"),
            TableRow::new()
                .cell("name", "李四")
                .cell("age", "34")
                .cell("email", "lisi@example.com"),
            TableRow::new()
                .cell("name", "王五")
                .cell("age", "22")
                .cell("email", "wangwu@example.com"),
        ];

        self.merged_rows = vec![
            TableRow::new()
                .cell("category", "水果")
                .cell("name", "苹果")
                .cell("value", "5")
                .row_span("category", 2),
            TableRow::new()
                .cell("name", "香蕉")
                .cell("value", "3"),
            TableRow::new()
                .cell("category", "蔬菜")
                .cell("name", "胡萝卜")
                .cell("value", "8")
                .row_span("category", 2),
            TableRow::new()
                .cell("name", "菠菜")
                .cell("value", "4"),
        ];
    }
}

impl TableCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("table_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("table_case.rml.rs").to_string()
    }
}
