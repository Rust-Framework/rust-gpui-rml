//! TableColumn 数据结构 —— 表格列定义
//!
//! 描述一列的元信息：字段 key（用于从 TableRow.cells 取值）、列头标题、
//! 列宽（可选）、对齐方式（可选）。纯数据结构，非 IntoElement 组件。
//!
//! RML `<Column key="..." title="..." width="..." align="..." />` 编译为
//! `rml_ui::TableColumn::new("key", "title").width(...).align(...)`。

use gpui::{Pixels, SharedString, TextAlign};

/// 表格列定义
#[derive(Debug, Clone)]
pub struct TableColumn {
    /// 字段 key（用于从 `TableRow.cells` 取值，作为 HashMap key）
    pub key: SharedString,
    /// 列头标题文本
    pub title: SharedString,
    /// 列宽（px），None 表示自动分配
    pub width: Option<Pixels>,
    /// 对齐方式，None 表示默认左对齐
    pub align: Option<TextAlign>,
    /// 是否可编辑（标记此列的单元格支持行内编辑）
    pub editable: bool,
}

impl TableColumn {
    /// 创建列定义。`key` 用于从 `TableRow.cells` 取值，`title` 为列头文本。
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width: None,
            align: None,
            editable: false,
        }
    }

    /// 设置列宽（px）。
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.width = Some(width.into());
        self
    }

    /// 设置对齐方式。
    pub fn align(mut self, align: TextAlign) -> Self {
        self.align = Some(align);
        self
    }

    /// 标记此列为可编辑（单元格支持行内编辑）。
    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_key_and_title() {
        let col = TableColumn::new("name", "Name");
        assert_eq!(col.key, "name");
        assert_eq!(col.title, "Name");
        assert!(col.width.is_none());
        assert!(col.align.is_none());
    }

    #[test]
    fn builder_methods_set_fields() {
        let col = TableColumn::new("age", "Age")
            .width(gpui::px(120.))
            .align(TextAlign::Center);
        assert_eq!(col.width, Some(gpui::px(120.)));
        assert_eq!(col.align, Some(TextAlign::Center));
    }

    #[test]
    fn clone_works() {
        let col = TableColumn::new("email", "Email").width(gpui::px(200.));
        let cloned = col.clone();
        assert_eq!(cloned.key, "email");
        assert_eq!(cloned.title, "Email");
        assert_eq!(cloned.width, Some(gpui::px(200.)));
    }
}
