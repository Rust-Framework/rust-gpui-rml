//! Printer 上下文
//!
//! 控制 AST → RML 源码输出的格式细节。

use super::TranslatorRegistry;

/// RML 源码打印上下文
#[derive(Debug, Clone)]
pub struct PrinterCtx {
    /// 当前缩进层级
    pub indent_level: usize,
    /// 每层缩进的空格数
    pub indent_size: usize,
    /// 一行最大字符数（超出则换行属性）
    pub max_line_length: usize,
    /// 是否自闭合空元素
    pub self_closing: bool,
    /// 当前行是否已写入内容
    pub line_started: bool,
    /// translator 注册表，供递归打印子元素时查询对应 `to_rml` 实现
    pub registry: TranslatorRegistry,
}

impl Default for PrinterCtx {
    fn default() -> Self {
        Self {
            indent_level: 0,
            indent_size: 2,
            max_line_length: 100,
            self_closing: true,
            line_started: false,
            registry: TranslatorRegistry::empty(),
        }
    }
}

impl PrinterCtx {
    /// 创建默认 printer 上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 创建带注册表的 printer 上下文
    pub fn with_registry(registry: TranslatorRegistry) -> Self {
        Self {
            registry,
            ..Self::default()
        }
    }

    /// 增加缩进层级
    pub fn indent(&self) -> Self {
        let mut next = self.clone();
        next.indent_level += 1;
        next
    }

    /// 当前缩进字符串
    pub fn indent_str(&self) -> String {
        " ".repeat(self.indent_level * self.indent_size)
    }

    /// 新行前缀（缩进）
    pub fn newline_indent(&self) -> String {
        format!("\n{}", self.indent_str())
    }
}
