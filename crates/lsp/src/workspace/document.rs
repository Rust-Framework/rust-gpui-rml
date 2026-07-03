//! 单个打开的文档（Roslyn Document 等价物）
//!
//! 持有文本快照 + 不可变语法树 + 语义模型。

use std::sync::Arc;

use lsp_types::Url;

use crate::semantics::model::SemanticModel;
use crate::syntax::tree::SyntaxTree;

/// 单个打开的文档
pub struct Document {
    /// 文档 URI
    pub uri: Url,
    /// 版本号（来自 didOpen/didChange）
    pub version: i32,
    /// 不可变语法树快照
    pub tree: Arc<SyntaxTree>,
    /// 语义模型（绑定路径/命令名解析结果）
    pub semantic: Arc<SemanticModel>,
}

impl Document {
    pub fn new(uri: Url, version: i32, tree: Arc<SyntaxTree>, semantic: Arc<SemanticModel>) -> Self {
        Self { uri, version, tree, semantic }
    }
}
