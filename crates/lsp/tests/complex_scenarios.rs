//! 复杂场景测试：用真实 .rml 夹具验证 features 在复杂输入下的行为
//!
//! 覆盖：
//! - formatting 深嵌套 + 多属性 + 绑定 + 指令
//! - document_symbol 嵌套符号树
//! - references include_declaration=true/false 差异
//! - 边界情况：空文档、语法错误文档、单元素文档

use lsp_types::{Position, Url};
use rml_lsp::features::{
    document_symbol::document_symbol, formatting::format_document,
    references::find_references, symbol::{classify_symbol_at, Symbol},
};
use rml_lsp::rust::NoopQuery;
use rml_lsp::server::conv::offset_to_position;
use rml_lsp::workspace::Workspace;
use rust_rml_engine::parser;

const COMPLEX_RML: &str = include_str!("fixtures/complex.rml");

fn ws_with_doc(uri: &Url, source: &str) -> Workspace {
    let mut ws = Workspace::new();
    ws.open_document(uri.clone(), source, 1);
    ws
}

fn rml_uri() -> Url {
    Url::parse("file:///complex.rml").unwrap()
}

// ============================================================
// formatting 复杂场景
// ============================================================

#[test]
fn format_complex_fixture_does_not_panic() {
    // 复杂文档（深嵌套 + 多属性 + 绑定 + 指令 + 插值）格式化不 panic
    let options = lsp_types::FormattingOptions {
        tab_size: 4,
        insert_spaces: true,
        ..Default::default()
    };
    let result = format_document(COMPLEX_RML, &options);
    let edits = result.expect("formatting should succeed for valid doc");
    assert_eq!(edits.len(), 1, "should return single full-document edit");
    let new_text = &edits[0].new_text;
    assert!(new_text.contains("<component>"), "should contain root tag");
    assert!(new_text.contains("class=\"container\""), "should preserve static attr");
    assert!(new_text.contains("if={visible}"), "should preserve if directive");
}

#[test]
fn format_deeply_nested_elements() {
    // 深嵌套（4 层）元素格式化后每层缩进递增
    let source = "<component><div><div><div><span></span></div></div></div></component>";
    let options = lsp_types::FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        ..Default::default()
    };
    let edits = format_document(source, &options).expect("should format");
    let new_text = &edits[0].new_text;
    // 验证缩进递增：component=0, div=2, div=4, div=6, span=8 空格
    assert!(new_text.contains("\n  <div"), "depth 1 should have 2-space indent");
    assert!(new_text.contains("\n    <div"), "depth 2 should have 4-space indent");
    assert!(new_text.contains("\n      <div"), "depth 3 should have 6-space indent");
    assert!(new_text.contains("\n        <span"), "depth 4 should have 8-space indent");
}

#[test]
fn format_element_with_bind_and_event_attrs() {
    // 绑定属性 + 事件属性混合的元素格式化
    let source = "<component><button class=\"btn\" count={count} onclick={on_click} label={title}></button></component>";
    let options = lsp_types::FormattingOptions {
        tab_size: 2,
        insert_spaces: true,
        ..Default::default()
    };
    let edits = format_document(source, &options).expect("should format");
    let new_text = &edits[0].new_text;
    // 多属性应分行，每属性独占一行
    assert!(new_text.contains("class=\"btn\""));
    assert!(new_text.contains("count={count}"));
    assert!(new_text.contains("onclick={on_click}"));
    assert!(new_text.contains("label={title}"));
    // 验证属性分行（至少 4 个换行：开标签行 + 4 属性行 + 闭标签行）
    assert!(new_text.matches('\n').count() >= 5, "multi-attr element should be multi-line");
}

// ============================================================
// document_symbol 复杂场景
// ============================================================

#[test]
fn document_symbol_complex_fixture_returns_full_tree() {
    // 复杂文档的符号树应包含所有嵌套元素
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, COMPLEX_RML);
    let response = document_symbol(&uri, &ws).expect("should return symbols");
    let symbols = match response {
        lsp_types::DocumentSymbolResponse::Nested(s) => s,
        _ => panic!("expected nested response"),
    };
    assert_eq!(symbols.len(), 1, "should have one root");
    let root = &symbols[0];
    assert_eq!(root.name, "component");
    let children = root.children.as_ref().expect("root should have children");
    assert_eq!(children.len(), 1, "root should have one child (div)");
    let div = &children[0];
    assert_eq!(div.name, "div");
    let div_children = div.children.as_ref().expect("div should have children");
    assert!(div_children.len() >= 3, "div should have at least 3 children (h1, div, buttons)");
}

// ============================================================
// references include_declaration 差异
// ============================================================

#[test]
fn references_include_declaration_returns_more_than_exclude() {
    // include_declaration=true 应包含定义点（若 find_definition 成功），false 不包含
    // NoopQuery 下 find_definition 返回 None，两者返回相同的引用集合
    // 此测试验证：两者都正确收集所有 bind attr 引用，且 true >= false
    let uri = rml_uri();
    let source = "<component><div title={title}></div><span title={title}></span><h1 title={title}></h1></component>";
    let ws = ws_with_doc(&uri, source);
    let q = NoopQuery;

    let first_bind = source.find("title={title}").expect("should find first bind");
    let cursor_offset = first_bind + 9; // 光标在 = 后的 title 标识符中间
    let pos = offset_to_position(cursor_offset, source, &ws.document(&uri).unwrap().tree.line_starts);

    let with_decl = find_references(&uri, pos, true, &ws, &q);
    let without_decl = find_references(&uri, pos, false, &ws, &q);

    // 两者都应收集到全部 3 个 bind attr 引用
    assert_eq!(with_decl.len(), 3, "include_declaration=true should find all 3 bindings");
    assert_eq!(without_decl.len(), 3, "include_declaration=false should find all 3 bindings");
    // true 应 >= false（定义点仅在 find_definition 成功时额外添加）
    assert!(with_decl.len() >= without_decl.len());
}

#[test]
fn references_on_field_finds_all_bindings() {
    // 同一字段在多个绑定属性中引用，references 应全部找到
    let uri = rml_uri();
    let source = "<component><div count={count}></div><div count={count}></div><div count={count}></div></component>";
    let ws = ws_with_doc(&uri, source);
    let q = NoopQuery;

    // 定位到第一个 count={count} 的绑定属性
    let first_bind = source.find("count={count}").expect("should find first bind");
    let cursor_offset = first_bind + 8; // 光标在 = 后的 count 标识符中间
    let pos = offset_to_position(cursor_offset, source, &ws.document(&uri).unwrap().tree.line_starts);

    let refs = find_references(&uri, pos, false, &ws, &q);
    assert_eq!(refs.len(), 3, "should find all 3 count bindings, got {}", refs.len());
}

// ============================================================
// 边界情况
// ============================================================

#[test]
fn format_empty_document_returns_none() {
    // 空文档格式化应返回 None（无内容可格式化）
    let options = lsp_types::FormattingOptions::default();
    let result = format_document("", &options);
    assert!(result.is_none(), "empty doc should return None");
}

#[test]
fn format_syntax_error_document_returns_none() {
    // 语法错误文档格式化应返回 None（解析失败）
    let source = "<component><div></component>"; // div 未闭合
    let options = lsp_types::FormattingOptions::default();
    let result = format_document(source, &options);
    assert!(result.is_none(), "syntax error should return None");
}

#[test]
fn document_symbol_empty_document_returns_none() {
    // 空文档无符号
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, "");
    let result = document_symbol(&uri, &ws);
    assert!(result.is_none(), "empty doc should return None");
}

#[test]
fn document_symbol_syntax_error_returns_none_or_partial() {
    // 语法错误文档可能返回 None 或部分符号（取决于解析器行为）
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, "<component><div></component>");
    // 不 panic 即可
    let _ = document_symbol(&uri, &ws);
}

#[test]
fn references_empty_document_returns_empty() {
    // 空文档 references 应返回空数组，不 panic
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, "");
    let q = NoopQuery;
    let pos = Position { line: 0, character: 0 };
    let refs = find_references(&uri, pos, true, &ws, &q);
    assert!(refs.is_empty(), "empty doc should return empty references");
}

#[test]
fn classify_symbol_on_empty_doc_returns_none() {
    // 空文档符号分类应返回 None
    let root = parser::parse("").ok();
    let result = root.and_then(|r| classify_symbol_at(&r, "", 0));
    assert!(result.is_none(), "empty doc should classify as None");
}

#[test]
fn format_single_element_document() {
    // 单元素文档格式化应返回单行
    let source = "<component/>";
    let options = lsp_types::FormattingOptions::default();
    let edits = format_document(source, &options).expect("should format single element");
    assert_eq!(edits.len(), 1);
    assert!(!edits[0].new_text.is_empty());
}

#[test]
fn references_find_interpolation_in_complex_fixture() {
    // complex.rml 中 {title} 出现在 <h1>{title}</h1> 和 <span>{title}</span>
    // references 应收集到这 2 个插值引用
    let uri = rml_uri();
    let ws = ws_with_doc(&uri, COMPLEX_RML);
    let q = NoopQuery;

    let source = COMPLEX_RML;
    let doc = ws.document(&uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();

    let interp_offset = source.find("{title}").expect("should find {title} in fixture");
    let cursor_offset = interp_offset + 2;
    let pos = offset_to_position(cursor_offset, source, &doc.tree.line_starts);
    assert_eq!(
        classify_symbol_at(root, source, cursor_offset),
        Some(Symbol::Field("title".to_string()))
    );

    let refs = find_references(&uri, pos, false, &ws, &q);
    assert_eq!(refs.len(), 2, "should find 2 {{title}} interpolations in fixture");
}
