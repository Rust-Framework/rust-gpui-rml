//! 源码区间标注（LSP 与未来增量解析的基础设施）
//!
//! 字节偏移基准，line/column 由独立工具函数按需换算。
//! 半开区间 [start, end)，与 `str` 切片边界对齐。

use serde::{Deserialize, Serialize};

/// 半开字节区间 [start, end)
///
/// `start` 为起始字节偏移（含），`end` 为结束字节偏移（不含）。
/// 所有偏移基于 UTF-8 字节，可直接用于 `&str[span.start..span.end]` 切片。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Span {
    /// 起始字节偏移（含）
    pub start: usize,
    /// 结束字节偏移（不含）
    pub end: usize,
}

impl Span {
    /// 构造半开区间 [start, end)
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// 空区间（零长度，多用于占位）
    pub fn empty() -> Self {
        Self { start: 0, end: 0 }
    }

    /// 区间字节长度
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// 是否为空区间
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// 字节偏移是否落在区间内 [start, end)
    pub fn contains(&self, offset: usize) -> bool {
        self.start <= offset && offset < self.end
    }
}

impl Default for Span {
    fn default() -> Self {
        Self::empty()
    }
}
