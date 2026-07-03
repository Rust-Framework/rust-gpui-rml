//! 语义诊断类型

use rust_rml_engine::parser::Span;

/// 语义诊断严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Hint,
}

/// 语义诊断（绑定路径缺失 / 命令不存在）
#[derive(Debug, Clone)]
pub struct SemanticDiagnostic {
    pub span: Span,
    pub message: String,
    pub severity: Severity,
}

impl SemanticDiagnostic {
    pub fn error(span: Span, message: impl Into<String>) -> Self {
        Self { span, message: message.into(), severity: Severity::Error }
    }

    pub fn warning(span: Span, message: impl Into<String>) -> Self {
        Self { span, message: message.into(), severity: Severity::Warning }
    }
}
