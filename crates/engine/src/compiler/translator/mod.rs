//! RML Translator 接口与基础设施
//!
//! 将 AST → Rust 代码、AST → RML 源码的转译逻辑从分散的硬编码模块抽象为
//! 统一的 `IRmlTranslator` trait。每个标签对应一个 translator，属性映射、
//! 构造逻辑、子节点处理、设计时元数据内聚在一处。

pub mod builtin;
pub mod component;
pub mod ctx;
pub mod menu;
pub mod metadata;
pub mod registry;
pub mod slot;
pub mod transparent;
pub mod user_component;
pub mod utils;

pub use ctx::PrinterCtx;
pub use metadata::{ComponentCategory, TranslatorMetadata};
pub use registry::TranslatorRegistry;

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;
use std::fmt;

/// Printer 错误
#[derive(Debug, Clone)]
pub struct PrintError {
    pub message: String,
}

impl fmt::Display for PrintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Print error: {}", self.message)
    }
}

impl std::error::Error for PrintError {}

impl PrintError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// RML 标签转译器接口
///
/// 每个实现对应一个 RML 标签（原生 HTML 标签、扩展组件、用户组件、根节点）。
/// `to_rust` 生成 GPUI 构建代码，`to_rml` 生成格式化 RML 源码，`metadata`
/// 提供设计时与校验所需信息。
pub trait IRmlTranslator: Send + Sync + fmt::Debug {
    /// 该 translator 处理的 canonical 标签名
    fn tag(&self) -> &'static str;

    /// 是否可处理此元素
    ///
    /// 默认按 `elem.tag == self.tag()` 精确匹配。需要模糊匹配（如用户组件
    /// wildcard）的 translator 可重载此方法。
    fn matches(&self, elem: &Element) -> bool {
        elem.tag == self.tag()
    }

    /// AST → Rust 代码
    ///
    /// 生成用于 `impl Render` 方法体中的 GPUI 元素构造代码片段。
    /// 返回 (代码, 是否迭代器)：当元素含 `each` 指令时返回迭代器表达式。
    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[crate::css::ParentInfo],
    ) -> Result<(String, bool), CodegenError>;

    /// AST → RML 源码
    ///
    /// 将元素序列化为 `.rml` 文件中的文本表示，供可视化设计器写回源码。
    fn to_rml(
        &self,
        elem: &Element,
        ctx: &PrinterCtx,
    ) -> Result<String, PrintError>;

    /// 设计时与校验元数据
    fn metadata(&self) -> TranslatorMetadata;
}
