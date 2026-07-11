//! Grid / GridItem 专用属性 setter
//!
//! ## Grid 属性映射
//!
//! - `columns="3"` (static) → `.columns(3u16)`（等宽列数）
//! - `rows="2"` (static) → `.rows(2u16)`（等高行数）
//!
//! ## GridItem 属性映射
//!
//! - `col-span="2"` (static) → `.col_span(2u16)`
//! - `row-span="3"` (static) → `.row_span(3u16)`
//! - `col-start="1"` (static) → `.col_start(1i16)`
//! - `col-end="4"` (static) → `.col_end(4i16)`
//! - `row-start="2"` (static) → `.row_start(2i16)`
//! - `row-end="5"` (static) → `.row_end(5i16)`

/// Grid 专用静态属性 setter
pub fn grid_static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "columns" => {
            let n: u16 = value.parse().ok()?;
            Some(format!(".columns({}u16)", n))
        }
        "rows" => {
            let n: u16 = value.parse().ok()?;
            Some(format!(".rows({}u16)", n))
        }
        _ => None,
    }
}

/// GridItem 专用静态属性 setter
pub fn grid_item_static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "col_span" => {
            let n: u16 = value.parse().ok()?;
            Some(format!(".col_span({}u16)", n))
        }
        "row_span" => {
            let n: u16 = value.parse().ok()?;
            Some(format!(".row_span({}u16)", n))
        }
        "col_start" => {
            let n: i16 = value.parse().ok()?;
            Some(format!(".col_start({}i16)", n))
        }
        "col_end" => {
            let n: i16 = value.parse().ok()?;
            Some(format!(".col_end({}i16)", n))
        }
        "row_start" => {
            let n: i16 = value.parse().ok()?;
            Some(format!(".row_start({}i16)", n))
        }
        "row_end" => {
            let n: i16 = value.parse().ok()?;
            Some(format!(".row_end({}i16)", n))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_columns() {
        assert_eq!(
            grid_static_setter("columns", "3"),
            Some(".columns(3u16)".to_string())
        );
    }

    #[test]
    fn grid_rows() {
        assert_eq!(
            grid_static_setter("rows", "2"),
            Some(".rows(2u16)".to_string())
        );
    }

    #[test]
    fn grid_invalid_number() {
        assert!(grid_static_setter("columns", "abc").is_none());
    }

    #[test]
    fn grid_item_col_span() {
        assert_eq!(
            grid_item_static_setter("col_span", "2"),
            Some(".col_span(2u16)".to_string())
        );
    }

    #[test]
    fn grid_item_row_span() {
        assert_eq!(
            grid_item_static_setter("row_span", "3"),
            Some(".row_span(3u16)".to_string())
        );
    }

    #[test]
    fn grid_item_col_start() {
        assert_eq!(
            grid_item_static_setter("col_start", "1"),
            Some(".col_start(1i16)".to_string())
        );
    }

    #[test]
    fn grid_item_col_end() {
        assert_eq!(
            grid_item_static_setter("col_end", "4"),
            Some(".col_end(4i16)".to_string())
        );
    }

    #[test]
    fn grid_item_row_start() {
        assert_eq!(
            grid_item_static_setter("row_start", "2"),
            Some(".row_start(2i16)".to_string())
        );
    }

    #[test]
    fn grid_item_row_end() {
        assert_eq!(
            grid_item_static_setter("row_end", "5"),
            Some(".row_end(5i16)".to_string())
        );
    }

    #[test]
    fn grid_item_negative_start() {
        assert_eq!(
            grid_item_static_setter("col_start", "-1"),
            Some(".col_start(-1i16)".to_string())
        );
    }

    #[test]
    fn grid_item_unknown() {
        assert!(grid_item_static_setter("unknown", "1").is_none());
    }
}
