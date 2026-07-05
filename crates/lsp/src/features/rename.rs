//! 重命名：在 .rml 与 .rml.rs 内对 Tag/Field/Command 符号执行重命名
//!
//! 流程：
//! 1. 校验 `new_name` 为合法 Rust 标识符
//! 2. `classify_symbol_at` 识别光标处符号
//! 3. `features::references::find_references` 收集 .rml 内引用点 → 转 TextEdit
//! 4. 跨语言部分：
//!    - Field/Command：调 `rust_query.rename_member` 改 .rml.rs 内对应字段/方法
//!    - Tag：调 `rust_query.rename_struct` 改 workspace 内所有 `#[component] struct <Tag>`
//! 5. 拼装 WorkspaceEdit.changes 返回

use std::collections::HashMap;

use lsp_types::{Position, TextEdit, Url, WorkspaceEdit};

use crate::features::references::find_references;
use crate::features::symbol::{classify_symbol_at, Symbol};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::Workspace;

/// 合法标识符正则：`^[A-Za-z_][A-Za-z0-9_]*$`
fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// 执行重命名
///
/// 返回 None 表示：
/// - 文档未打开
/// - 光标处无可识别符号
/// - `new_name` 不合法
pub fn rename(
    uri: &Url,
    position: Position,
    new_name: &str,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<WorkspaceEdit> {
    if !is_valid_ident(new_name) {
        return None;
    }

    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;
    let symbol = classify_symbol_at(root, source, byte_offset)?;

    // 收集 .rml 内引用点
    let locations = find_references(uri, position, true, workspace, rust_query);
    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    let mut rml_edits: Vec<TextEdit> = locations
        .into_iter()
        .filter(|loc| loc.uri == *uri)
        .map(|loc| TextEdit {
            range: loc.range,
            new_text: new_name.to_string(),
        })
        .collect();
    // 去重（include_declaration 时定义点可能与引用点重合）
    rml_edits.dedup_by(|a, b| a.range == b.range);
    if !rml_edits.is_empty() {
        changes.insert(uri.clone(), rml_edits);
    }

    // 跨语言：.rml.rs 内同步改名
    match &symbol {
        Symbol::Field(name) | Symbol::Command(name) => {
            if let Some(rs_uri) = workspace.codebehind_uri(uri) {
                // 在 metadata 中找包含该 member 的 struct
                let struct_name = workspace
                    .index()
                    .metadata_for(uri)
                    .and_then(|m| {
                        m.iter().find_map(|(sname, meta)| {
                            let hit = match &symbol {
                                Symbol::Field(_) => {
                                    meta.observable_fields.contains(name)
                                        || meta.computed_methods.contains(name)
                                }
                                Symbol::Command(_) => meta.commands.contains(name),
                                _ => false,
                            };
                            if hit {
                                Some(sname.clone())
                            } else {
                                None
                            }
                        })
                    });
                if let Some(struct_name) = struct_name {
                    let rs_edits =
                        rust_query.rename_member(rs_uri, &struct_name, name, new_name);
                    if !rs_edits.is_empty() {
                        changes.entry(rs_uri.clone()).or_default().extend(rs_edits);
                    }
                }
            }
        }
        Symbol::Tag(old_name) => {
            let rs_changes = rust_query.rename_struct(old_name, new_name);
            for (file_uri, edits) in rs_changes {
                if !edits.is_empty() {
                    changes.entry(file_uri).or_default().extend(edits);
                }
            }
        }
    }

    if changes.is_empty() {
        return None;
    }

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::ast_util::find_element_at_offset;
    use crate::features::symbol::Symbol;
    use crate::rust::NoopQuery;
    use crate::server::conv::offset_to_position;
    use crate::workspace::Workspace;
    use rust_rml_engine::parser::ast::Attribute;

    fn ws_with_doc(rml_uri: &Url, source: &str) -> Workspace {
        let mut ws = Workspace::new();
        ws.open_document(rml_uri.clone(), source, 0);
        ws
    }

    #[test]
    fn invalid_ident_returns_none() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let ws = ws_with_doc(&rml, "<component><div></div></component>");
        let q = NoopQuery;
        let pos = Position { line: 0, character: 12 };
        assert!(rename(&rml, pos, "1bad", &ws, &q).is_none());
        assert!(rename(&rml, pos, "", &ws, &q).is_none());
        assert!(rename(&rml, pos, "has-dash", &ws, &q).is_none());
    }

    #[test]
    fn rename_tag_returns_rml_edits() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><div><div></div></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;
        // 光标在第一个 div 标签名上
        let pos = Position { line: 0, character: 12 };
        let edit = rename(&rml, pos, "section", &ws, &q).expect("should rename");
        let changes = edit.changes.expect("changes present");
        let rml_edits = changes.get(&rml).expect("rml edits");
        assert_eq!(rml_edits.len(), 2);
        assert!(rml_edits.iter().all(|e| e.new_text == "section"));
    }

    #[test]
    fn rename_field_returns_rml_edits() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><div count={count}></div><div count={count}></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;

        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        let elem = find_element_at_offset(root, 15).unwrap();
        let bind_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Bind { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (bind_attr.start + bind_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        assert_eq!(
            classify_symbol_at(root, source, mid),
            Some(Symbol::Field("count".to_string()))
        );

        let edit = rename(&rml, pos, "total", &ws, &q).expect("should rename");
        let rml_edits = edit.changes.unwrap().get(&rml).unwrap().clone();
        assert_eq!(rml_edits.len(), 2);
        assert!(rml_edits.iter().all(|e| e.new_text == "total"));
    }

    #[test]
    fn rename_command_returns_rml_edits() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><button onclick={on_click}></button><button onclick={on_click}></button></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;

        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        let elem = find_element_at_offset(root, 15).unwrap();
        let event_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Event { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (event_attr.start + event_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        assert_eq!(
            classify_symbol_at(root, source, mid),
            Some(Symbol::Command("on_click".to_string()))
        );

        let edit = rename(&rml, pos, "on_click_v2", &ws, &q).expect("should rename");
        let rml_edits = edit.changes.unwrap().get(&rml).unwrap().clone();
        assert_eq!(rml_edits.len(), 2);
        assert!(rml_edits.iter().all(|e| e.new_text == "on_click_v2"));
    }

    #[test]
    fn rename_returns_none_when_no_symbol() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let source = "<component><div class=\"x\"></div></component>";
        let ws = ws_with_doc(&rml, source);
        let q = NoopQuery;

        let doc = ws.document(&rml).unwrap();
        let root = doc.tree.root.as_ref().unwrap();
        let elem = find_element_at_offset(root, 15).unwrap();
        let static_attr = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { span, .. } => Some(*span),
                _ => None,
            })
            .unwrap();
        let mid = (static_attr.start + static_attr.end) / 2;
        let pos = offset_to_position(mid, source, &doc.tree.line_starts);
        assert!(rename(&rml, pos, "new_name", &ws, &q).is_none());
    }

    #[test]
    fn valid_ident_check() {
        assert!(is_valid_ident("count"));
        assert!(is_valid_ident("_count"));
        assert!(is_valid_ident("count_v2"));
        assert!(!is_valid_ident(""));
        assert!(!is_valid_ident("1bad"));
        assert!(!is_valid_ident("has-dash"));
        assert!(!is_valid_ident("has space"));
    }
}
