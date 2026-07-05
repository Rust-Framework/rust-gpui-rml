//! rename 跨语言测试：用 MockQuery 验证 rename_member/rename_struct 路径
//!
//! features/rename.rs 的单元测试全用 NoopQuery，跨语言路径（rename_member/rename_struct）
//! 从未触发。本测试用 MockQuery 返回真实 TextEdit，验证 .rml + .rml.rs 双向同步改名。

use std::collections::HashMap;

use lsp_types::{Position, Range, TextEdit, Url};
use rml_lsp::features::rename::rename;
use rml_lsp::features::symbol::Symbol;
use rml_lsp::features::{ast_util, symbol};
use rml_lsp::rust::{
    CompletionEntry, ComponentInfo, HoverInfo, RustDiagnostic, RustSemanticQuery, SymbolInfo,
    SymbolLocation,
};
use rml_lsp::server::conv::offset_to_position;
use rml_lsp::workspace::Workspace;
use rust_rml_engine::parser::ast::Attribute;

/// MockQuery：可配置返回值的 RustSemanticQuery mock
struct MockQuery {
    /// rename_member 返回的 TextEdit（按 (struct_name, member) 键索引）
    rename_member_edits: HashMap<(String, String), Vec<TextEdit>>,
    /// rename_struct 返回的 TextEdit（按 old_name 键索引）
    rename_struct_edits: HashMap<String, HashMap<Url, Vec<TextEdit>>>,
}

impl MockQuery {
    fn new() -> Self {
        Self {
            rename_member_edits: HashMap::new(),
            rename_struct_edits: HashMap::new(),
        }
    }

    fn with_member_rename(mut self, struct_name: &str, member: &str, edits: Vec<TextEdit>) -> Self {
        self.rename_member_edits
            .insert((struct_name.to_string(), member.to_string()), edits);
        self
    }

    fn with_struct_rename(
        mut self,
        old_name: &str,
        edits: HashMap<Url, Vec<TextEdit>>,
    ) -> Self {
        self.rename_struct_edits.insert(old_name.to_string(), edits);
        self
    }
}

impl RustSemanticQuery for MockQuery {
    fn open_document(&mut self, _uri: &Url, _text: &str) {}
    fn apply_change(&mut self, _uri: &Url, _text: &str) {}
    fn close_document(&mut self, _uri: &Url) {}
    fn goto_definition(&self, _uri: &Url, _pos: Position) -> Vec<SymbolLocation> {
        Vec::new()
    }
    fn hover(&self, _uri: &Url, _pos: Position) -> Option<HoverInfo> {
        None
    }
    fn completion(&self, _uri: &Url, _pos: Position) -> Vec<CompletionEntry> {
        Vec::new()
    }
    fn diagnostics(&self, _uri: &Url) -> Vec<RustDiagnostic> {
        Vec::new()
    }
    fn resolve_member(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _member: &str,
    ) -> Option<SymbolInfo> {
        None
    }
    fn find_struct(&self, _struct_name: &str) -> Option<SymbolLocation> {
        None
    }
    fn struct_slots(&self, _rml_rs_uri: &Url, _struct_name: &str) -> Vec<String> {
        Vec::new()
    }
    fn command_signature(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _method: &str,
    ) -> Option<SymbolInfo> {
        None
    }
    fn list_components(&self, _prefix: &str) -> Vec<ComponentInfo> {
        Vec::new()
    }
    fn is_ready(&self) -> bool {
        true
    }
    fn find_references(
        &self,
        _uri: &Url,
        _pos: Position,
        _include_declaration: bool,
    ) -> Vec<SymbolLocation> {
        Vec::new()
    }
    fn rename_member(
        &self,
        _rml_rs_uri: &Url,
        struct_name: &str,
        member: &str,
        _new_name: &str,
    ) -> Vec<TextEdit> {
        self.rename_member_edits
            .get(&(struct_name.to_string(), member.to_string()))
            .cloned()
            .unwrap_or_default()
    }
    fn rename_struct(
        &self,
        old_name: &str,
        _new_name: &str,
    ) -> HashMap<Url, Vec<TextEdit>> {
        self.rename_struct_edits
            .get(old_name)
            .cloned()
            .unwrap_or_default()
    }
}

/// 构造带 codebehind 配对的 workspace
fn make_paired_workspace(rml_uri: &Url, rml_source: &str, rml_rs_uri: &Url, rml_rs_source: &str) -> Workspace {
    let mut ws = Workspace::new();
    ws.open_document(rml_uri.clone(), rml_source, 1);
    ws.refresh_codebehind(rml_rs_uri, rml_rs_source);
    ws.register_pair(rml_uri.clone(), rml_rs_uri.clone());
    ws
}

fn dummy_range() -> Range {
    Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 0, character: 5 },
    }
}

#[test]
fn rename_field_with_mock_query_returns_cross_lang_edits() {
    // .rml 含 {count} 绑定，.rml.rs 含 struct MyView { pub count: i32 }
    // rename count → total 应同时产生 .rml 编辑和 .rml.rs 编辑（来自 MockQuery）
    let rml_uri = Url::parse("file:///view.rml").unwrap();
    let rml_rs_uri = Url::parse("file:///view.rml.rs").unwrap();
    let rml_source = "<component><div count={count}></div></component>";
    let rml_rs_source = r#"#[window]
pub struct MyView {
    pub count: i32,
}
"#;
    let ws = make_paired_workspace(&rml_uri, rml_source, &rml_rs_uri, rml_rs_source);

    // 验证 metadata 正确解析
    let meta = ws.index().metadata_for(&rml_uri).expect("metadata should exist");
    assert!(meta.contains_key("MyView"), "MyView struct should be in metadata");
    assert!(meta["MyView"].observable_fields.contains(&"count".to_string()));

    // MockQuery: rename_member 返回 1 个 .rml.rs 编辑
    let rs_edit = TextEdit {
        range: dummy_range(),
        new_text: "total".to_string(),
    };
    let q = MockQuery::new().with_member_rename("MyView", "count", vec![rs_edit]);

    // 定位光标到 {count} 的 count 标识符上
    let doc = ws.document(&rml_uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    let elem = ast_util::find_element_at_offset(root, 20).expect("should find element");
    let bind_span = elem
        .attributes
        .iter()
        .find_map(|a| match a {
            Attribute::Bind { span, .. } => Some(*span),
            _ => None,
        })
        .expect("should find bind attr");
    let mid = (bind_span.start + bind_span.end) / 2;
    let pos = offset_to_position(mid, rml_source, &doc.tree.line_starts);

    // 验证符号识别
    assert_eq!(
        symbol::classify_symbol_at(root, rml_source, mid),
        Some(Symbol::Field("count".to_string()))
    );

    let edit = rename(&rml_uri, pos, "total", &ws, &q).expect("rename should return edit");
    let changes = edit.changes.expect("should have changes");

    // 应包含 .rml 编辑（count 引用）
    let rml_edits = changes.get(&rml_uri).expect("should have .rml edits");
    assert!(!rml_edits.is_empty(), ".rml edits should not be empty");
    assert!(rml_edits.iter().all(|e| e.new_text == "total"));

    // 应包含 .rml.rs 编辑（来自 MockQuery）
    let rs_edits = changes.get(&rml_rs_uri).expect("should have .rml.rs edits from MockQuery");
    assert_eq!(rs_edits.len(), 1, "MockQuery should return 1 .rml.rs edit");
    assert_eq!(rs_edits[0].new_text, "total");
}

#[test]
fn rename_command_with_mock_query_returns_method_rename() {
    // .rml 含 onclick={on_click}，.rml.rs 含 impl MyView { #[command] fn on_click }
    // rename on_click → on_click_v2 应同时产生 .rml 和 .rml.rs 编辑
    let rml_uri = Url::parse("file:///cmd.rml").unwrap();
    let rml_rs_uri = Url::parse("file:///cmd.rml.rs").unwrap();
    let rml_source = "<component><button onclick={on_click}></button></component>";
    let rml_rs_source = r#"#[window]
pub struct MyView {}

impl MyView {
    #[command]
    pub fn on_click(&mut self) {}
}
"#;
    let ws = make_paired_workspace(&rml_uri, rml_source, &rml_rs_uri, rml_rs_source);

    let meta = ws.index().metadata_for(&rml_uri).expect("metadata should exist");
    assert!(meta["MyView"].commands.contains(&"on_click".to_string()));

    let rs_edit = TextEdit {
        range: dummy_range(),
        new_text: "on_click_v2".to_string(),
    };
    let q = MockQuery::new().with_member_rename("MyView", "on_click", vec![rs_edit]);

    let doc = ws.document(&rml_uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    let elem = ast_util::find_element_at_offset(root, 20).expect("should find element");
    let event_span = elem
        .attributes
        .iter()
        .find_map(|a| match a {
            Attribute::Event { span, .. } => Some(*span),
            _ => None,
        })
        .expect("should find event attr");
    let mid = (event_span.start + event_span.end) / 2;
    let pos = offset_to_position(mid, rml_source, &doc.tree.line_starts);

    assert_eq!(
        symbol::classify_symbol_at(root, rml_source, mid),
        Some(Symbol::Command("on_click".to_string()))
    );

    let edit = rename(&rml_uri, pos, "on_click_v2", &ws, &q).expect("rename should return edit");
    let changes = edit.changes.expect("should have changes");

    let rml_edits = changes.get(&rml_uri).expect("should have .rml edits");
    assert!(rml_edits.iter().all(|e| e.new_text == "on_click_v2"));

    let rs_edits = changes
        .get(&rml_rs_uri)
        .expect("should have .rml.rs edits from MockQuery");
    assert_eq!(rs_edits.len(), 1);
    assert_eq!(rs_edits[0].new_text, "on_click_v2");
}

#[test]
fn rename_tag_with_mock_query_returns_struct_rename_across_files() {
    // .rml 含 <MyComponent>，MockQuery.rename_struct 返回跨文件编辑
    let rml_uri = Url::parse("file:///comp.rml").unwrap();
    let rml_source = "<component><MyComponent></MyComponent></component>";
    let ws = {
        let mut ws = Workspace::new();
        ws.open_document(rml_uri.clone(), rml_source, 1);
        ws
    };

    // MockQuery: rename_struct 返回 2 个文件的编辑
    let rs1_uri = Url::parse("file:///a.rml.rs").unwrap();
    let rs2_uri = Url::parse("file:///b.rml.rs").unwrap();
    let mut struct_edits = HashMap::new();
    struct_edits.insert(
        rs1_uri.clone(),
        vec![TextEdit {
            range: dummy_range(),
            new_text: "NewComponent".to_string(),
        }],
    );
    struct_edits.insert(
        rs2_uri.clone(),
        vec![TextEdit {
            range: dummy_range(),
            new_text: "NewComponent".to_string(),
        }],
    );
    let q = MockQuery::new().with_struct_rename("MyComponent", struct_edits);

    // 光标在 MyComponent 标签名上
    let doc = ws.document(&rml_uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    // <component> 前缀长度 = 11，<MyComponent 起始 = 11，标签名起始 = 12
    let pos = offset_to_position(12, rml_source, &doc.tree.line_starts);

    assert_eq!(
        symbol::classify_symbol_at(root, rml_source, 12),
        Some(Symbol::Tag("MyComponent".to_string()))
    );

    let edit = rename(&rml_uri, pos, "NewComponent", &ws, &q).expect("rename should return edit");
    let changes = edit.changes.expect("should have changes");

    // .rml 编辑（标签名）
    let rml_edits = changes.get(&rml_uri).expect("should have .rml edits");
    assert!(rml_edits.iter().any(|e| e.new_text == "NewComponent"));

    // 跨文件编辑（来自 MockQuery）
    let rs1_edits = changes.get(&rs1_uri).expect("should have a.rml.rs edits");
    assert_eq!(rs1_edits.len(), 1);
    let rs2_edits = changes.get(&rs2_uri).expect("should have b.rml.rs edits");
    assert_eq!(rs2_edits.len(), 1);
}

#[test]
fn rename_field_without_codebehind_pair_skips_rust_edits() {
    // 无 codebehind 配对时，rename field 应只产生 .rml 编辑，不调 rename_member
    let rml_uri = Url::parse("file:///solo.rml").unwrap();
    let rml_source = "<component><div count={count}></div></component>";
    let mut ws = Workspace::new();
    ws.open_document(rml_uri.clone(), rml_source, 1);
    // 不调用 register_pair / refresh_codebehind

    let q = MockQuery::new(); // 无配置，rename_member 返回空

    let doc = ws.document(&rml_uri).unwrap();
    let root = doc.tree.root.as_ref().unwrap();
    let elem = ast_util::find_element_at_offset(root, 20).expect("should find element");
    let bind_span = elem
        .attributes
        .iter()
        .find_map(|a| match a {
            Attribute::Bind { span, .. } => Some(*span),
            _ => None,
        })
        .expect("should find bind attr");
    let mid = (bind_span.start + bind_span.end) / 2;
    let pos = offset_to_position(mid, rml_source, &doc.tree.line_starts);

    let edit = rename(&rml_uri, pos, "total", &ws, &q).expect("rename should return edit");
    let changes = edit.changes.expect("should have changes");

    // 只有 .rml 编辑，无 .rml.rs 编辑
    assert!(changes.contains_key(&rml_uri));
    assert_eq!(changes.len(), 1, "should only have .rml edits, no .rml.rs");
}
