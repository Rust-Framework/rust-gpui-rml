//! RML 代码折叠：基于缩进扫描
//!
//! 语言无关的缩进策略，与 VSCode `indentation` strategy 行为一致。
//! `adapter.rs` 中的 `indent_folding_ranges` 仅在 `rust-backend` feature 下编译，
//! 本模块在无 feature 时为 RML 文件提供相同能力。

use lsp_types::{FoldingRange, FoldingRangeKind, Url};

use crate::workspace::Workspace;

/// 为 RML 文档生成折叠区域
pub fn fold_ranges(uri: &Url, workspace: &Workspace) -> Vec<FoldingRange> {
    let Some(doc) = workspace.document(uri) else {
        return Vec::new();
    };
    indent_folding_ranges(doc.tree.text())
}

/// 基于缩进扫描生成折叠区域
///
/// 算法：维护缩进栈，遇到更深缩进时，前一行成为折叠起点，栈顶缩进回退时折叠结束。
/// 仅生成跨 ≥ 2 行的区域（gpui-component FoldMap 要求 MIN_FOLD_LINES = 2）。
fn indent_folding_ranges(text: &str) -> Vec<FoldingRange> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 3 {
        return Vec::new();
    }

    let indent_of = |line: &str| -> usize {
        line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
    };

    let mut ranges = Vec::new();
    let mut stack: Vec<(usize, usize)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let cur_indent = indent_of(line);

        while let Some(&(top_indent, start_line)) = stack.last() {
            if top_indent >= cur_indent {
                stack.pop();
                let end_line = idx.saturating_sub(1);
                if end_line > start_line + 1 {
                    ranges.push(FoldingRange {
                        start_line: start_line as u32,
                        end_line: end_line as u32,
                        start_character: None,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Region),
                        collapsed_text: None,
                    });
                }
            } else {
                break;
            }
        }

        let next_indent = lines[idx + 1..]
            .iter()
            .find(|l| !l.trim().is_empty())
            .map(indent_of)
            .unwrap_or(0);
        if next_indent > cur_indent {
            stack.push((cur_indent, idx));
        }
    }

    let last_line = lines.len() - 1;
    while let Some((_, start_line)) = stack.pop() {
        if last_line > start_line + 1 {
            ranges.push(FoldingRange {
                start_line: start_line as u32,
                end_line: last_line as u32,
                start_character: None,
                end_character: None,
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            });
        }
    }

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_returns_empty() {
        assert!(indent_folding_ranges("").is_empty());
    }

    #[test]
    fn flat_text_returns_empty() {
        let text = "line1\nline2\nline3";
        assert!(indent_folding_ranges(text).is_empty());
    }

    #[test]
    fn indented_block_produces_range() {
        let text = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let ranges = indent_folding_ranges(text);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 3);
    }

    #[test]
    fn nested_indent_produces_multiple_ranges() {
        let text = "a\n  b\n    c\n  d\ne\n";
        let ranges = indent_folding_ranges(text);
        assert!(ranges.len() >= 2);
    }
}
