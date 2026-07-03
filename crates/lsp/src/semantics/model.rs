//! 语义模型（Roslyn SemanticModel 等价物）
//!
//! 惰性解析绑定路径/命令名 → 产出语义诊断。

use std::sync::Arc;

use crate::semantics::binder;
use crate::semantics::diagnostics::SemanticDiagnostic;
use crate::syntax::tree::SyntaxTree;
use crate::workspace::project_index::ProjectIndex;

/// 语义模型
pub struct SemanticModel {
    pub diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticModel {
    /// 带 ProjectIndex 上下文的语义分析（由 workspace 调用，传入 URI 查 metadata）
    pub fn analyze_with_uri(
        tree: &Arc<SyntaxTree>,
        index: &ProjectIndex,
        uri: &lsp_types::Url,
    ) -> Arc<Self> {
        let metadata_map = index.metadata_for(uri);
        let diagnostics = match &tree.root {
            Some(root) => binder::bind(root, metadata_map),
            None => Vec::new(),
        };
        Arc::new(Self { diagnostics })
    }

    /// 空模型（解析失败时用）
    pub fn empty() -> Arc<Self> {
        Arc::new(Self { diagnostics: Vec::new() })
    }
}
