//! RML 编译器入口
//!
//! 串起 parse → validate → codegen，输出 Rust 源码字符串。

pub mod codegen;
pub mod expr;
pub mod validator;

use crate::parser;
use std::fmt;

/// 代码生成上下文
#[derive(Debug, Clone)]
pub struct CodegenCtx {
    /// 视图结构体名（如 "Counter"）
    pub view_struct_name: String,
    /// 视图模块路径（如 "my_app::views::counter"）
    pub view_module_path: String,
}

/// 编译错误
#[derive(Debug)]
pub enum CompileError {
    Parse(parser::ParseError),
    Validate(validator::ValidationError),
    Codegen(codegen::CodegenError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Parse(e) => write!(f, "{}", e),
            CompileError::Validate(e) => write!(f, "{}", e),
            CompileError::Codegen(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<parser::ParseError> for CompileError {
    fn from(e: parser::ParseError) -> Self {
        CompileError::Parse(e)
    }
}
impl From<validator::ValidationError> for CompileError {
    fn from(e: validator::ValidationError) -> Self {
        CompileError::Validate(e)
    }
}
impl From<codegen::CodegenError> for CompileError {
    fn from(e: codegen::CodegenError) -> Self {
        CompileError::Codegen(e)
    }
}

/// 编译 `.rml` 源码为 Rust 源码字符串
///
/// # 参数
/// - `source`: `.rml` 文件内容
/// - `ctx`: 代码生成上下文（含视图结构名）
///
/// # 返回
/// 生成的 `impl Render for <View>` 代码块字符串
pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<String, CompileError> {
    let root = parser::parse(source)?;
    validator::validate(&root)?;
    let code = codegen::codegen(&root, ctx)?;
    Ok(code)
}
