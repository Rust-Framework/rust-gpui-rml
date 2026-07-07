//! RML 源映射数据模型
//!
//! codegen 生成 `.rml.rs` 时同步产出 `SourceMap`，记录 AST 节点的 `.rml` 字节区间
//! 到生成代码的 `(line, column)` 的映射。持久化为 `.rml.map` 文件后，由
//! `rust-rml-dap` 的 `LineAccurateMapper` 加载消费。
//!
//! ## 位置基准
//!
//! - `rml_span`：字节偏移（与 `parser::Span` 一致，半开区间 `[start, end)`）
//! - `rust_line` / `rust_column`：1-based（与 DAP `StackFrame.line/column` 一致）
//!
//! ## 查询语义
//!
//! - 正向 `rml_to_rust(span)`：找到覆盖 `span` 的最小 entry，返回其 rust 位置
//! - 反向 `rust_to_rml(line, col)`：找到覆盖 `(line, col)` 的最小 entry，返回其 rml span

use crate::parser::Span;
use serde::{Deserialize, Serialize};

/// 单条源映射记录
///
/// 一条 entry 表示「`rml_span` 这个区间对应的源码片段，在生成的 `.rml.rs` 中
/// 起始于 `(rust_line, rust_column)`」。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapEntry {
    /// `.rml` 源码字节区间（半开）
    pub rml_span: Span,
    /// 生成代码起始行（1-based）
    pub rust_line: u32,
    /// 生成代码起始列（1-based）
    pub rust_column: u32,
}

/// 源映射表
///
/// 由 codegen 在生成代码过程中逐步收集，最终序列化为 `.rml.map` JSON 文件。
/// entries 按 `rust_line` 升序排列（便于二分反查）；同一行按 `rust_column` 升序。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SourceMap {
    /// 映射条目列表
    pub entries: Vec<SourceMapEntry>,
}

impl SourceMap {
    /// 创建空映射
    pub fn new() -> Self {
        Self::default()
    }

    /// 记录一条映射：`.rml` 字节区间 → 生成代码位置
    ///
    /// `rust_line`/`rust_column` 为 1-based。调用方应在 codegen 生成关键代码片段
    /// （如元素构造调用、属性 setter、事件绑定）时调用此方法。
    pub fn record(&mut self, rml_span: Span, rust_line: u32, rust_column: u32) {
        self.entries.push(SourceMapEntry {
            rml_span,
            rust_line,
            rust_column,
        });
    }

    /// 按 (rust_line, rust_column) 升序排序 entries
    ///
    /// 在所有 record 调用完成后、序列化前调用一次，保证反查使用二分查找。
    pub fn sort_by_rust(&mut self) {
        self.entries.sort_by_key(|e| (e.rust_line, e.rust_column));
    }

    /// 按 rml_span.start 升序排序 entries
    ///
    /// 正向查询（rml → rust）使用，保证二分查找生效。
    pub fn sort_by_rml(&mut self) {
        self.entries.sort_by_key(|e| e.rml_span.start);
    }

    /// 正向查询：`.rml` 字节偏移 → 生成代码位置
    ///
    /// 找到覆盖 `byte_offset` 的最小 entry（即 `rml_span.start <= offset < rml_span.end`
    /// 中 start 最大的那个），返回其 rust 位置。无匹配返回 None。
    pub fn rml_to_rust(&self, byte_offset: usize) -> Option<(u32, u32)> {
        // 线性查找：entries 数量级与 AST 节点数相同（数百到数千），可接受
        // 若未来需要二分，可保持 sort_by_rml + 二分查找
        let mut best: Option<&SourceMapEntry> = None;
        for entry in &self.entries {
            if entry.rml_span.contains(byte_offset) {
                match best {
                    None => best = Some(entry),
                    Some(cur) if entry.rml_span.start >= cur.rml_span.start => best = Some(entry),
                    _ => {}
                }
            }
        }
        best.map(|e| (e.rust_line, e.rust_column))
    }

    /// 反向查询：生成代码 (line, column) → `.rml` 字节区间
    ///
    /// 找到 `(line, column)` 落入的最小 entry（即 line 匹配、column <= entry.col
    /// 中 column 最大的那个），返回其 rml_span。无匹配返回 None。
    pub fn rust_to_rml(&self, line: u32, column: u32) -> Option<Span> {
        let mut best: Option<&SourceMapEntry> = None;
        for entry in &self.entries {
            if entry.rust_line == line && entry.rust_column <= column {
                match best {
                    None => best = Some(entry),
                    Some(cur) if entry.rust_column >= cur.rust_column => best = Some(entry),
                    _ => {}
                }
            }
        }
        best.map(|e| e.rml_span)
    }

    /// 序列化为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_query_roundtrip() {
        let mut map = SourceMap::new();
        // 模拟两条映射：rml 字节 [0,10) → rust 5:1，rml 字节 [20,30) → rust 8:5
        map.record(Span::new(0, 10), 5, 1);
        map.record(Span::new(20, 30), 8, 5);

        // 正向查询：偏移 5 落入第一条
        assert_eq!(map.rml_to_rust(5), Some((5, 1)));
        // 偏移 25 落入第二条
        assert_eq!(map.rml_to_rust(25), Some((8, 5)));
        // 偏移 15 不在任何区间
        assert_eq!(map.rml_to_rust(15), None);
    }

    #[test]
    fn nested_span_prefers_innermost() {
        let mut map = SourceMap::new();
        // 外层 [0, 100) → 1:1，内层 [40, 50) → 3:5
        map.record(Span::new(0, 100), 1, 1);
        map.record(Span::new(40, 50), 3, 5);

        // 偏移 45 同时被两条 entry 覆盖，应返回 start 更大的内层
        assert_eq!(map.rml_to_rust(45), Some((3, 5)));
        // 偏移 20 仅被外层覆盖
        assert_eq!(map.rml_to_rust(20), Some((1, 1)));
    }

    #[test]
    fn reverse_query_finds_best_column() {
        let mut map = SourceMap::new();
        // 同一行 5:1 与 5:10
        map.record(Span::new(0, 10), 5, 1);
        map.record(Span::new(20, 30), 5, 10);

        // 列 5 落入 5:1 之后、5:10 之前，应返回 5:1
        assert_eq!(map.rust_to_rml(5, 5), Some(Span::new(0, 10)));
        // 列 15 落入 5:10 之后，应返回 5:10
        assert_eq!(map.rust_to_rml(5, 15), Some(Span::new(20, 30)));
        // 行 6 无 entry
        assert_eq!(map.rust_to_rml(6, 1), None);
    }

    #[test]
    fn json_roundtrip_preserves_entries() {
        let mut map = SourceMap::new();
        map.record(Span::new(0, 10), 5, 1);
        map.record(Span::new(20, 30), 8, 5);

        let json = map.to_json().unwrap();
        let restored = SourceMap::from_json(&json).unwrap();
        assert_eq!(restored.entries.len(), 2);
        assert_eq!(restored.entries[0].rml_span, Span::new(0, 10));
        assert_eq!(restored.entries[0].rust_line, 5);
        assert_eq!(restored.entries[0].rust_column, 1);
    }
}
