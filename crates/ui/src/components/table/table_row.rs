//! TableRow 数据结构 —— 表格行数据（含合并列支持）
//!
//! 每行存储 `cells: HashMap<key, value>`（按 TableColumn.key 索引），
//! 以及可选的 `col_spans`/`row_spans`（按 key 指定合并跨度）。
//!
//! 构造示例：
//! ```ignore
//! TableRow::new()
//!     .cell("name", "John")
//!     .cell("email", "john@example.com")
//!     .col_span("name", 2)  // name 占 2 列
//! ```

use std::collections::HashMap;

use gpui::SharedString;

/// 表格行数据
#[derive(Debug, Clone, Default)]
pub struct TableRow {
    /// 单元格数据：`{ column_key => value }`
    pub cells: HashMap<SharedString, SharedString>,
    /// 合并列跨度：`{ column_key => span }`（该单元格横向跨 span 列）
    pub col_spans: HashMap<SharedString, usize>,
    /// 合并行跨度：`{ column_key => span }`（该单元格纵向跨 span 行）
    pub row_spans: HashMap<SharedString, usize>,
}

impl TableRow {
    /// 创建空行。
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加单元格数据。`key` 对应 `TableColumn.key`，`value` 为显示文本。
    pub fn cell(
        mut self,
        key: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        self.cells.insert(key.into(), value.into());
        self
    }

    /// 设置某列的合并列跨度（横向跨 span 列）。
    pub fn col_span(mut self, key: impl Into<SharedString>, span: usize) -> Self {
        self.col_spans.insert(key.into(), span);
        self
    }

    /// 设置某列的合并行跨度（纵向跨 span 行）。
    pub fn row_span(mut self, key: impl Into<SharedString>, span: usize) -> Self {
        self.row_spans.insert(key.into(), span);
        self
    }

    /// 按 key 获取单元格值，不存在返回空字符串。
    pub fn get(&self, key: &SharedString) -> SharedString {
        self.cells.get(key).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_row() {
        let row = TableRow::new();
        assert!(row.cells.is_empty());
        assert!(row.col_spans.is_empty());
        assert!(row.row_spans.is_empty());
    }

    #[test]
    fn cell_adds_entry() {
        let row = TableRow::new().cell("name", "John").cell("age", "30");
        assert_eq!(row.cells.get("name"), Some(&SharedString::from("John")));
        assert_eq!(row.cells.get("age"), Some(&SharedString::from("30")));
    }

    #[test]
    fn col_span_sets_span() {
        let row = TableRow::new().cell("name", "John").col_span("name", 2);
        assert_eq!(row.col_spans.get("name"), Some(&2));
    }

    #[test]
    fn row_span_sets_span() {
        let row = TableRow::new().cell("name", "John").row_span("name", 3);
        assert_eq!(row.row_spans.get("name"), Some(&3));
    }

    #[test]
    fn get_returns_value_or_empty() {
        let row = TableRow::new().cell("name", "John");
        assert_eq!(row.get(&SharedString::from("name")), "John");
        assert_eq!(row.get(&SharedString::from("missing")), "");
    }

    #[test]
    fn default_is_empty() {
        let row = TableRow::default();
        assert!(row.cells.is_empty());
    }

    #[test]
    fn clone_preserves_data() {
        let row = TableRow::new()
            .cell("name", "John")
            .col_span("name", 2);
        let cloned = row.clone();
        assert_eq!(cloned.cells.get("name"), Some(&SharedString::from("John")));
        assert_eq!(cloned.col_spans.get("name"), Some(&2));
    }
}
