//! 解析文档：调用 engine::parser，产 SyntaxTree + 语法诊断
//!
//! CaaS 单一解析入口——不重复实现 parser，直接复用 engine。

use std::sync::Arc;

use rust_rml_engine::parser;

use crate::syntax::tree::SyntaxTree;

/// 解析 .rml 源码，产出不可变 SyntaxTree
///
/// 解析失败时 root 为 None，errors 填入语法错误（供诊断发布）。
pub fn parse_document(source: &str) -> Arc<SyntaxTree> {
    let source_arc: Arc<str> = Arc::from(source);
    match parser::parse(source) {
        Ok(root) => {
            // parser::parse 成功时 errors 为空
            Arc::new(SyntaxTree::new(source_arc, Some(root), Vec::new()))
        }
        Err(err) => {
            // 解析失败：无根节点，但保留错误供 LSP 诊断
            Arc::new(SyntaxTree::new(source_arc, None, vec![err]))
        }
    }
}
