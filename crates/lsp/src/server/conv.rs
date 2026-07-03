//! `engine::Span` ↔ `lsp_types::Range` 换算
//!
//! engine 的 `Span` 基于字节偏移；LSP `Position.character` 基于 UTF-16 码元。
//! 用预计算的 `line_starts`（每行起始字节偏移）做二分查找换算。

use lsp_types::{Position, Range};
use rust_rml_engine::parser::Span;

/// 预计算每行的起始字节偏移（含 0 作为第一行起点）
///
/// `line_starts[i]` = 第 i 行（0-based）起始的字节偏移。
/// 例 `"ab\ncd"` → `[0, 3]`
pub fn compute_line_starts(source: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (offset, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            starts.push((offset + 1) as u32);
        }
    }
    starts
}

/// 字节偏移 → LSP Position（UTF-16 码元）
///
/// 用 `line_starts` 二分查找所在行，再遍历该行字符计算 UTF-16 列。
pub fn offset_to_position(byte_offset: usize, source: &str, line_starts: &[u32]) -> Position {
    let offset = byte_offset.min(source.len()) as u32;

    // 二分查找：找到最大的 line_starts[i] <= offset
    let line = match line_starts.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => i.saturating_sub(1),
    };

    let line_start = line_starts[line] as usize;
    let line_text = &source[line_start..offset as usize];

    // UTF-16 码元计数（BMP 字符 = 1，辅助平面字符 = 2）
    let character = line_text.chars().map(|c| c.len_utf16() as u32).sum();

    Position {
        line: line as u32,
        character,
    }
}

/// engine::Span → lsp_types::Range
pub fn span_to_range(span: Span, source: &str, line_starts: &[u32]) -> Range {
    Range {
        start: offset_to_position(span.start, source, line_starts),
        end: offset_to_position(span.end, source, line_starts),
    }
}

/// 空区间（零长度，定位用）
pub fn empty_range_at(span: Span, source: &str, line_starts: &[u32]) -> Range {
    let pos = offset_to_position(span.start, source, line_starts);
    Range { start: pos, end: pos }
}

/// LSP Position → 字节偏移（补全/悬停查询反向换算）
pub fn position_to_byte_offset(pos: Position, source: &str, line_starts: &[u32]) -> usize {
    let line = (pos.line as usize).min(line_starts.len().saturating_sub(1));
    let line_start = line_starts[line] as usize;

    // 从行首开始，按 UTF-16 码元推进到目标列
    let mut byte_offset = line_start;
    let mut utf16_remaining = pos.character as usize;
    for (offset, ch) in source[line_start..].char_indices() {
        if utf16_remaining == 0 {
            break;
        }
        let utf16_len = ch.len_utf16();
        if utf16_remaining < utf16_len {
            // 字符中间：定位到该字符起始
            byte_offset = line_start + offset;
            break;
        }
        utf16_remaining -= utf16_len;
        byte_offset = line_start + offset + ch.len_utf8();
    }
    byte_offset
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_starts_simple() {
        assert_eq!(compute_line_starts("ab\ncd"), vec![0, 3]);
        assert_eq!(compute_line_starts("hello"), vec![0]);
        assert_eq!(compute_line_starts("a\nb\nc"), vec![0, 2, 4]);
    }

    #[test]
    fn offset_to_pos_basic() {
        let source = "ab\ncd";
        let starts = compute_line_starts(source);
        assert_eq!(offset_to_position(0, source, &starts), Position { line: 0, character: 0 });
        assert_eq!(offset_to_position(2, source, &starts), Position { line: 0, character: 2 });
        assert_eq!(offset_to_position(3, source, &starts), Position { line: 1, character: 0 });
        assert_eq!(offset_to_position(5, source, &starts), Position { line: 1, character: 2 });
    }

    #[test]
    fn offset_to_pos_multibyte() {
        // 中文：每个字符 3 字节 UTF-8，1 个 UTF-16 码元
        let source = "你好";
        let starts = compute_line_starts(source);
        assert_eq!(offset_to_position(0, source, &starts), Position { line: 0, character: 0 });
        assert_eq!(offset_to_position(3, source, &starts), Position { line: 0, character: 1 });
        assert_eq!(offset_to_position(6, source, &starts), Position { line: 0, character: 2 });
    }

    #[test]
    fn roundtrip_pos_to_offset() {
        let source = "ab\ncd";
        let starts = compute_line_starts(source);
        let pos = Position { line: 1, character: 1 };
        assert_eq!(position_to_byte_offset(pos, source, &starts), 4);
    }
}
