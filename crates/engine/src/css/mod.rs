//! CSS 子集解析器与样式映射
//!
//! 将 `.css` 文件解析为 `StyleSheet`，再将 CSS 声明映射为 GPUI 样式方法调用。
//! 详见文档 §7.2 CSS 子集与扩展。

pub mod ast;
pub mod mapper;
pub mod matcher;
pub mod parser;

pub use ast::*;
pub use mapper::map_declarations;
pub use matcher::{generate_styles, matches_selector, styles_for_class, ElementContext};
pub use parser::{parse, ParseError};
