//! Table 组件模块（WPF DataGrid 风格声明式表格）

#[allow(clippy::module_inception)]
mod table;
mod table_column;
mod table_delegate;
mod table_row;
mod table_template;

pub use table::Table;
pub use table_column::TableColumn;
pub use table_delegate::{DefaultTableDelegate, TableDelegate};
pub use table_row::TableRow;
pub use table_template::{CellTemplate, FooterTemplate, HeaderTemplate};
