//! 语义模型（Roslyn SemanticModel 等价物）
//!
//! 惰性解析绑定路径/命令名 → 产出语义诊断 + 语义 tokens。

use std::sync::Arc;

use crate::semantics::binder;
use crate::semantics::diagnostics::SemanticDiagnostic;
use crate::semantics::tokens::SpannedSemanticToken;
use crate::syntax::tree::SyntaxTree;
use crate::workspace::project_index::ProjectIndex;

/// 语义模型
pub struct SemanticModel {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub tokens: Vec<SpannedSemanticToken>,
}

impl SemanticModel {
    /// 带 ProjectIndex 上下文的语义分析（由 workspace 调用，传入 URI 查 metadata）
    pub fn analyze_with_uri(
        tree: &Arc<SyntaxTree>,
        index: &ProjectIndex,
        uri: &lsp_types::Url,
    ) -> Arc<Self> {
        let metadata_map = index.metadata_for(uri);
        let result = match &tree.root {
            Some(root) => binder::bind(root, tree.text(), metadata_map),
            None => binder::BindingResult::default(),
        };
        Arc::new(Self {
            diagnostics: result.diagnostics,
            tokens: result.tokens,
        })
    }

    /// 空模型（解析失败时用）
    pub fn empty() -> Arc<Self> {
        Arc::new(Self {
            diagnostics: Vec::new(),
            tokens: Vec::new(),
        })
    }
}
