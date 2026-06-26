//! RML 编译器入口
//!
//! 串起 parse → validate → codegen，输出 Rust 源码字符串。

pub mod codegen;
pub mod component;
pub mod event;
pub mod expr;
pub mod validator;

use crate::css::StyleSheet;
use crate::parser;
use std::fmt;

/// 代码生成上下文
#[derive(Debug, Clone, Default)]
pub struct CodegenCtx {
    /// 视图结构体名（如 "Counter"）
    pub view_struct_name: String,
    /// 视图模块路径（如 "my_app::views::counter"）
    pub view_module_path: String,
    /// 全局样式表（由 build.rs 加载所有 `.css` 文件合并而成）
    ///
    /// codegen 在遇到 `class="..."` 属性时查询此样式表，
    /// 将匹配的 CSS 规则转换为 GPUI 样式方法调用。
    /// 为 None 时（如单元测试）class 属性不生成样式代码。
    pub stylesheet: Option<StyleSheet>,
    /// 计算属性方法名列表（由 build.rs 扫描 `.rml.rs` 文件中的 `#[computed]` 收集）
    ///
    /// 当插值 `{name}` 中的 `name` 在此列表中时，codegen 生成 `self.name()`（方法调用）
    /// 而非 `self.name`（字段访问）。
    pub computed_methods: Vec<String>,
}

/// 代码生成错误
///
/// 由 codegen / event / component 模块共用，定义在此避免循环依赖。
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codegen error: {}", self.message)
    }
}

impl std::error::Error for CodegenError {}

/// 编译错误
#[derive(Debug)]
pub enum CompileError {
    Parse(parser::ParseError),
    Validate(validator::ValidationError),
    Codegen(CodegenError),
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
impl From<CodegenError> for CompileError {
    fn from(e: CodegenError) -> Self {
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

