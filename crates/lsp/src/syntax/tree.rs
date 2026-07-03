//! 不可变语法树快照（Roslyn SyntaxTree 等价物）
//!
//! `Arc<SyntaxTree>` 共享 → 多查询零拷贝；编辑后整体替换为新 Arc。
//! MVP 阶段编辑时整文件重解析，数据结构为后续增量解析预留。

use std::sync::Arc;

use rust_rml_engine::parser::ast::Node;
use rust_rml_engine::parser::ParseError;

/// 不可变语法树快照
pub struct SyntaxTree {
    /// 源码快照（Arc<str> 零拷贝切片）
    pub source: Arc<str>,
    /// 解析成功时的根节点
    pub root: Option<Node>,
    /// 语法错误（来自 engine::parser）
    pub errors: Vec<ParseError>,
    /// 每行起始字节偏移（字节→行列换算用，预计算）
    pub line_starts: Vec<u32>,
}

impl SyntaxTree {
    pub fn new(source: Arc<str>, root: Option<Node>, errors: Vec<ParseError>) -> Self {
        let line_starts = crate::server::conv::compute_line_starts(&source);
        Self {
            source,
            root,
            errors,
            line_starts,
        }
    }

    /// 源码文本
    pub fn text(&self) -> &str {
        &self.source
    }
}
