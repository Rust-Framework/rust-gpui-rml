//! Table 组件模块（WPF DataGrid 风格声明式表格）

pub mod table;
pub mod table_column;
pub mod table_delegate;
pub mod table_row;
pub mod table_template;

pub use table::Table;
pub use table_column::TableColumn;
pub use table_delegate::{DefaultTableDelegate, TableDelegate};
pub use table_row::TableRow;
pub use table_template::{CellTemplate, FooterTemplate, HeaderTemplate};
