//! textDocument/semanticTokens 处理
//!
//! 将 `Document.semantic.tokens`（绝对字节 span）转换为 LSP delta 编码的
//! `SemanticTokens { data: Vec<SemanticToken> }`。
//!
//! LSP 协议要求 token 按位置单调递增排序，每个 token 用与前一个的 delta 描述：
//! `delta_line`/`delta_start`/`length`/`token_type`/`token_modifiers_bitset`。

use anyhow::Result;
use lsp_types::{
    Range, SemanticToken, SemanticTokens, SemanticTokensParams, SemanticTokensRangeParams,
};

use crate::server::connection::ServerState;
use crate::server::conv::{position_to_byte_offset, span_to_range};
use crate::semantics::tokens::SpannedSemanticToken;

/// textDocument/semanticTokens/full
pub fn handle_full(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<SemanticTokens>> {
    let params: SemanticTokensParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;
    let Some(doc) = state.workspace.document(&uri) else {
        return Ok(None);
    };
    let source = doc.tree.text();
    let line_starts = &doc.tree.line_starts;
    let tokens = &doc.semantic.tokens;
    Ok(Some(encode_tokens(tokens, source, line_starts, None)))
}

/// textDocument/semanticTokens/range
pub fn handle_range(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<Option<SemanticTokens>> {
    let params: SemanticTokensRangeParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;
    let Some(doc) = state.workspace.document(&uri) else {
        return Ok(None);
    };
    let source = doc.tree.text();
    let line_starts = &doc.tree.line_starts;
    let tokens = &doc.semantic.tokens;
    let range = params.range;
    let start_byte = position_to_byte_offset(range.start, source, line_starts);
    let end_byte = position_to_byte_offset(range.end, source, line_starts);
    Ok(Some(encode_tokens(
        tokens,
        source,
        line_starts,
        Some(start_byte..end_byte),
    )))
}

/// 将 `SpannedSemanticToken` 列表转 LSP delta 编码
///
/// `byte_range_filter`：若提供，仅保留 token.span 与之相交的 token（range 请求）
fn encode_tokens(
    tokens: &[SpannedSemanticToken],
    source: &str,
    line_starts: &[u32],
    byte_range_filter: Option<std::ops::Range<usize>>,
) -> SemanticTokens {
    // 1. 过滤 + 转 LSP Range + 计算 UTF-16 length
    let mut ranged: Vec<(&SpannedSemanticToken, Range, u32)> = tokens
        .iter()
        .filter_map(|t| {
            if let Some(ref r) = byte_range_filter {
                // token.span 与 r 相交：!(t.span.end <= r.start || t.span.start >= r.end)
                if t.span.end <= r.start || t.span.start >= r.end {
                    return None;
                }
            }
            let range = span_to_range(t.span, source, line_starts);
            let span_text = &source[t.span.start.min(source.len())..t.span.end.min(source.len())];
            let length: u32 = span_text.chars().map(|c| c.len_utf16() as u32).sum();
            Some((t, range, length))
        })
        .collect();

    // 2. 按 range.start 排序（line 优先，character 次）
    ranged.sort_by_key(|(_, r, _)| (r.start.line, r.start.character));

    // 3. delta 编码
    let mut data = Vec::with_capacity(ranged.len());
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    for (tok, range, length) in ranged {
        let delta_line = range.start.line - prev_line;
        let delta_start = if delta_line == 0 {
            range.start.character - prev_char
        } else {
            range.start.character
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.token_modifiers,
        });
        prev_line = range.start.line;
        prev_char = range.start.character;
    }
    SemanticTokens { data, result_id: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rml_engine::parser::Span;

    fn make_token(start: usize, end: usize, tt: u32, mods: u32) -> SpannedSemanticToken {
        SpannedSemanticToken::new(Span::new(start, end), tt, mods)
    }

    #[test]
    fn encode_empty_returns_empty_data() {
        let source = "<div></div>";
        let line_starts = crate::server::conv::compute_line_starts(source);
        let tokens: Vec<SpannedSemanticToken> = Vec::new();
        let result = encode_tokens(&tokens, source, &line_starts, None);
        assert!(result.data.is_empty());
    }

    #[test]
    fn encode_single_token_delta_starts_from_zero() {
        // token "if" at bytes 5..7 in "<div if={x}>"
        let source = "<div if={x}>";
        let line_starts = crate::server::conv::compute_line_starts(source);
        let tokens = vec![make_token(5, 7, 0, 0)]; // "if" keyword
        let result = encode_tokens(&tokens, source, &line_starts, None);
        assert_eq!(result.data.len(), 1);
        let t = &result.data[0];
        assert_eq!(t.delta_line, 0);
        assert_eq!(t.delta_start, 5); // position 5 in line 0
        assert_eq!(t.length, 2);
        assert_eq!(t.token_type, 0);
    }

    #[test]
    fn encode_two_tokens_same_line_delta() {
        let source = "<div if={count}>";
        let line_starts = crate::server::conv::compute_line_starts(source);
        // "if" at 5..7, "count" at 9..14
        let tokens = vec![
            make_token(5, 7, 0, 0),   // keyword "if"
            make_token(9, 14, 5, 0),  // variable "count"
        ];
        let result = encode_tokens(&tokens, source, &line_starts, None);
        assert_eq!(result.data.len(), 2);
        assert_eq!(result.data[0].delta_start, 5);
        assert_eq!(result.data[1].delta_line, 0);
        assert_eq!(result.data[1].delta_start, 9 - 5); // delta from prev
    }

    #[test]
    fn byte_range_filter_excludes_outside_tokens() {
        let source = "<div if={count}>";
        let line_starts = crate::server::conv::compute_line_starts(source);
        let tokens = vec![
            make_token(5, 7, 0, 0),   // "if"
            make_token(9, 14, 5, 0),  // "count"
        ];
        // 仅取 "count" 区间 [9..14]
        let result = encode_tokens(&tokens, source, &line_starts, Some(9..14));
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].delta_start, 9);
    }
}
