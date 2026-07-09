//! 诊断收集：合并语法错误 + validator 校验 + 语义诊断
//!
//! 三类诊断来源：
//! | 来源 | 类型 | 复用方式 |
//! |------|------|----------|
//! | 语法 | engine::parser::ParseError | SyntaxTree.errors 直接取 |
//! | 校验 | engine::validator::validate | 调 validate()，message 转诊断 |
//! | 语义 | semantics::diagnostics | SemanticModel.diagnostics |

use std::collections::HashMap;

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::compiler::validator;
use rust_rml_engine::compiler::UserComponentInfo;

use crate::server::connection::ServerState;
use crate::server::conv;
use crate::semantics::diagnostics::Severity;
use crate::workspace::Workspace;

/// 收集指定文档的所有诊断
pub fn collect(uri: &lsp_types::Url, workspace: &Workspace) -> Vec<Diagnostic> {
    let Some(doc) = workspace.document(uri) else {
        return Vec::new();
    };
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;

    let mut diagnostics = Vec::new();

    // 1. 语法错误（ParseError：有 line/column，无字节偏移）
    for err in &tree.errors {
        let range = Range {
            start: Position {
                line: (err.line.saturating_sub(1)) as u32,
                character: (err.column.saturating_sub(1)) as u32,
            },
            end: Position {
                line: (err.line.saturating_sub(1)) as u32,
                character: (err.column.saturating_sub(1) + 1) as u32,
            },
        };
        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            message: err.message.clone(),
            source: Some("rml".to_string()),
            ..Default::default()
        });
    }

    // 2. validator 校验错误（未知属性/slot/ref 重复）
    if let Some(root) = &tree.root {
        let user_components: HashMap<String, UserComponentInfo> = HashMap::new();
        let registry = TranslatorRegistry::default();
        if let Err(val_err) = validator::validate(root, &registry, &user_components) {
            // validator 无 span：用根元素的 span 回填
            let span = root_span(root);
            let range = conv::empty_range_at(span, source, line_starts);
            diagnostics.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::WARNING),
                message: val_err.message,
                source: Some("rml".to_string()),
                ..Default::default()
            });
        }
    }

    // 3. 语义诊断（绑定路径缺失 / 命令不存在）
    for sem_diag in &doc.semantic.diagnostics {
        let range = conv::span_to_range(sem_diag.span, source, line_starts);
        let severity = match sem_diag.severity {
            Severity::Error => DiagnosticSeverity::ERROR,
            Severity::Warning => DiagnosticSeverity::WARNING,
            Severity::Hint => DiagnosticSeverity::HINT,
        };
        diagnostics.push(Diagnostic {
            range,
            severity: Some(severity),
            message: sem_diag.message.clone(),
            source: Some("rml".to_string()),
            ..Default::default()
        });
    }

    diagnostics
}

/// 提取根节点的 span（用于 validator 诊断定位）
fn root_span(root: &rust_rml_engine::parser::ast::Node) -> rust_rml_engine::parser::Span {
    use rust_rml_engine::parser::ast::Node;
    match root {
        Node::Element(e) => e.span,
        _ => rust_rml_engine::parser::Span::empty(),
    }
}

/// 收集 `.rml.rs` 代码后置文件的诊断（来自 rust-analyzer 后端）
pub fn collect_rust(uri: &lsp_types::Url, state: &ServerState) -> Vec<Diagnostic> {
    state
        .rust_query
        .diagnostics(uri)
        .into_iter()
        .map(|d| Diagnostic {
            range: d.range,
            severity: Some(d.severity),
            message: d.message,
            code: d.code.map(lsp_types::NumberOrString::String),
            source: Some("rust-analyzer".to_string()),
            ..Default::default()
        })
        .collect()
}
