//! Semantic token types & modifiers（LSP `textDocument/semanticTokens` legend）
//!
//! 单一信源：`build_capabilities` 声明 server legend，demo 从 initialize 响应透传。
//! token_type 索引必须与 `RML_TOKEN_TYPES` 数组顺序一致。

use lsp_types::{SemanticTokenModifier, SemanticTokenType};
use rust_rml_engine::parser::Span;

/// Token 类型（按索引引用）
pub const RML_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,             // 0: 指令名 if/each/model/...
    SemanticTokenType::new("tag"),          // 1: HTML 标签名（自定义类型，与 tree-sitter @tag 对齐）
    SemanticTokenType::TYPE,                // 2: 组件标签（PascalCase）
    SemanticTokenType::new("attribute"),    // 3: 属性名（自定义类型，与 tree-sitter @attribute 对齐）
    SemanticTokenType::STRING,              // 4: 静态属性值
    SemanticTokenType::VARIABLE,            // 5: 已解析绑定字段
    SemanticTokenType::PROPERTY,            // 6: 未解析绑定字段
    SemanticTokenType::FUNCTION,            // 7: 事件处理器/命令
    SemanticTokenType::COMMENT,             // 8: 注释
];

/// Token 修饰符（按 bit 位引用）
pub const RML_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,   // bit 0: ref 目标 / each 迭代变量声明
    SemanticTokenModifier::DEFINITION,    // bit 1: 已解析绑定
    SemanticTokenModifier::DEPRECATED,    // bit 2: 未解析绑定（划线提示）
    SemanticTokenModifier::MODIFICATION,  // bit 3: model 双向绑定
];

/// Token 类型索引常量（避免魔法数字）
pub mod token_type {
    pub const KEYWORD: u32 = 0;
    pub const TAG: u32 = 1;
    pub const TYPE: u32 = 2;
    pub const ATTRIBUTE: u32 = 3;
    pub const STRING: u32 = 4;
    pub const VARIABLE: u32 = 5;
    pub const PROPERTY: u32 = 6;
    pub const FUNCTION: u32 = 7;
    pub const COMMENT: u32 = 8;
}

/// Token 修饰符 bit 位常量
pub mod token_modifier {
    pub const DECLARATION: u32 = 1 << 0;
    pub const DEFINITION: u32 = 1 << 1;
    pub const DEPRECATED: u32 = 1 << 2;
    pub const MODIFICATION: u32 = 1 << 3;
}

/// 带字节区间的语义 token（binder 发射，handler 转换为 LSP delta 编码）
#[derive(Debug, Clone)]
pub struct SpannedSemanticToken {
    pub span: Span,
    pub token_type: u32,
    pub token_modifiers: u32,
}

impl SpannedSemanticToken {
    pub fn new(span: Span, token_type: u32, token_modifiers: u32) -> Self {
        Self {
            span,
            token_type,
            token_modifiers,
        }
    }
}
