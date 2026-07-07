//! 案例共享工具模块
//!
//! 提供 API 文档表格的构建工具，统一案例的 API 表格渲染。
//! 所有案例使用 `<Table columns={api_columns} rows={api_rows} bordered="" stripe="" />`
//! 渲染 API 文档。

#[path = "case_doc_page.rml.rs"]
mod case_doc_page;

pub use case_doc_page::CaseDocPage;

use rml_ui::{TableColumn, TableRow};

const COL_PROP_WIDTH: f32 = 220.0;
const COL_TYPE_WIDTH: f32 = 120.0;

/// 构建 API 文档表格的列定义和行数据。
///
/// 列结构固定为：属性 / 类型 / 说明（三列）。
/// `props` 是 `(属性名, 类型, 说明)` 三元组切片。
///
/// # 示例
///
/// ```ignore
/// let (cols, rows) = build_api_table(&[
///     ("value", "f32", "进度值 0-100"),
///     ("loading", "布尔标志", "加载中状态"),
/// ]);
/// self.api_columns = cols;
/// self.api_rows = rows;
/// ```
pub fn build_api_table(props: &[(&str, &str, &str)]) -> (Vec<TableColumn>, Vec<TableRow>) {
    let columns = vec![
        TableColumn::new("prop", "属性").width(gpui::px(COL_PROP_WIDTH)),
        TableColumn::new("type", "类型").width(gpui::px(COL_TYPE_WIDTH)),
        TableColumn::new("desc", "说明"),
    ];
    let rows = props
        .iter()
        .map(|(p, t, d)| {
            TableRow::new()
                .cell("prop", *p)
                .cell("type", *t)
                .cell("desc", *d)
        })
        .collect();
    (columns, rows)
}
