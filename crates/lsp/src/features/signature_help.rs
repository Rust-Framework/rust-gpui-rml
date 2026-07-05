//! 签名帮助：基于 #[command] 方法签名构造 SignatureHelp
//!
//! 当光标位于事件属性 `onclick={cmd, args}` 内时，识别 cmd 名 → 查 metadata.commands
//! → 调 rust_query.command_signature 取 SymbolInfo.type_str → 构造 SignatureInformation。
//!
//! type_str 形如 `fn(Uuid, String) -> bool`，从中解析出参数列表与返回类型。

use lsp_types::{
    ParameterInformation, SignatureHelp, SignatureInformation, Url,
};

use rust_rml_engine::parser::ast::Attribute;

use crate::features::ast_util::{find_attribute_at_offset, find_element_at_offset};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::Workspace;

/// 构造签名帮助
pub fn signature_help(
    uri: &Url,
    position: lsp_types::Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<SignatureHelp> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;
    let elem = find_element_at_offset(root, byte_offset)?;
    let attr = find_attribute_at_offset(elem, byte_offset)?;

    let handler = match attr {
        Attribute::Event { handler, .. } => handler,
        _ => return None,
    };
    let cmd_name = match handler {
        rust_rml_engine::parser::ast::EventHandler::Ident(name)
        | rust_rml_engine::parser::ast::EventHandler::MethodName(name) => name.as_str(),
        rust_rml_engine::parser::ast::EventHandler::WithArgs(name, _) => name.as_str(),
    };

    let rml_rs_uri = workspace.codebehind_uri(uri)?;
    let metadata_map = workspace.index().metadata_for(uri)?;
    let mut struct_name = None;
    let mut found_cmd = false;
    for (sname, meta) in metadata_map {
        if meta.commands.iter().any(|c| c == cmd_name) {
            struct_name = Some(sname.clone());
            found_cmd = true;
            break;
        }
    }
    if !found_cmd {
        return None;
    }
    let struct_name = struct_name?;

    let sym = rust_query.command_signature(rml_rs_uri, struct_name.as_str(), cmd_name)?;
    let type_str = sym.type_str?;
    let (params, _return_type) = parse_signature(&type_str);

    let param_labels: Vec<String> = params.iter().map(|p| p.clone()).collect();
    let parameters: Vec<ParameterInformation> = param_labels
        .iter()
        .map(|label| ParameterInformation {
            label: lsp_types::ParameterLabel::Simple(label.clone()),
            documentation: None,
        })
        .collect();

    let sig_info = SignatureInformation {
        label: type_str.clone(),
        documentation: None,
        parameters: if parameters.is_empty() {
            None
        } else {
            Some(parameters)
        },
        active_parameter: Some(0),
    };

    Some(SignatureHelp {
        signatures: vec![sig_info],
        active_signature: Some(0),
        active_parameter: Some(0),
    })
}

/// 解析 type_str 中的参数列表
///
/// 支持 `fn(Uuid, String) -> bool` 与 `fn(&self, x: i32) -> String` 形式。
/// 返回 (参数列表, 可选返回类型)。
fn parse_signature(type_str: &str) -> (Vec<String>, Option<String>) {
    let trimmed = type_str.trim();
    let after_fn = trimmed.strip_prefix("fn ").unwrap_or(trimmed);
    let open = match after_fn.find('(') {
        Some(i) => i,
        None => return (Vec::new(), None),
    };
    let close = match after_fn.rfind(')') {
        Some(i) => i,
        None => return (Vec::new(), None),
    };
    if close <= open {
        return (Vec::new(), None);
    }
    let params_str = &after_fn[open + 1..close];
    let params: Vec<String> = if params_str.trim().is_empty() {
        Vec::new()
    } else {
        params_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };
    let return_type = after_fn[close + 1..]
        .trim()
        .strip_prefix("-> ")
        .map(|s| s.trim().to_string());
    (params, return_type)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_signature() {
        let (params, ret) = parse_signature("fn(Uuid, String) -> bool");
        assert_eq!(params, vec!["Uuid", "String"]);
        assert_eq!(ret, Some("bool".to_string()));
    }

    #[test]
    fn parse_no_args_signature() {
        let (params, ret) = parse_signature("fn() -> i32");
        assert!(params.is_empty());
        assert_eq!(ret, Some("i32".to_string()));
    }

    #[test]
    fn parse_self_signature() {
        let (params, ret) = parse_signature("fn(&self, x: i32) -> String");
        assert_eq!(params, vec!["&self", "x: i32"]);
        assert_eq!(ret, Some("String".to_string()));
    }

    #[test]
    fn parse_no_parens_returns_empty() {
        let (params, ret) = parse_signature("i32");
        assert!(params.is_empty());
        assert!(ret.is_none());
    }

    /// 测试用 Mock RustSemanticQuery
    struct MockQuery;
    impl RustSemanticQuery for MockQuery {
        fn open_document(&mut self, _: &Url, _: &str) {}
        fn apply_change(&mut self, _: &Url, _: &str) {}
        fn close_document(&mut self, _: &Url) {}
        fn goto_definition(
            &self,
            _: &Url,
            _: lsp_types::Position,
        ) -> Vec<crate::rust::SymbolLocation> {
            Vec::new()
        }
        fn hover(&self, _: &Url, _: lsp_types::Position) -> Option<crate::rust::HoverInfo> {
            None
        }
        fn completion(
            &self,
            _: &Url,
            _: lsp_types::Position,
        ) -> Vec<crate::rust::CompletionEntry> {
            Vec::new()
        }
        fn diagnostics(&self, _: &Url) -> Vec<crate::rust::RustDiagnostic> {
            Vec::new()
        }
        fn resolve_member(
            &self,
            _: &Url,
            _: &str,
            _: &str,
        ) -> Option<crate::rust::SymbolInfo> {
            None
        }
        fn find_struct(&self, _: &str) -> Option<crate::rust::SymbolLocation> {
            None
        }
        fn struct_slots(&self, _: &Url, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn command_signature(
            &self,
            _: &Url,
            _: &str,
            method: &str,
        ) -> Option<crate::rust::SymbolInfo> {
            if method == "on_click" {
                Some(crate::rust::SymbolInfo {
                    name: method.to_string(),
                    kind: crate::rust::SymbolKind::Method,
                    type_str: Some("fn(Uuid, String) -> bool".to_string()),
                    doc: None,
                    location: None,
                })
            } else {
                None
            }
        }
        fn list_components(&self, _: &str) -> Vec<crate::rust::ComponentInfo> {
            Vec::new()
        }
        fn is_ready(&self) -> bool {
            true
        }
        fn find_references(
            &self,
            _: &Url,
            _: lsp_types::Position,
            _: bool,
        ) -> Vec<crate::rust::SymbolLocation> {
            Vec::new()
        }
        fn rename_member(
            &self,
            _: &Url,
            _: &str,
            _: &str,
            _: &str,
        ) -> Vec<lsp_types::TextEdit> {
            Vec::new()
        }
        fn rename_struct(
            &self,
            _: &str,
            _: &str,
        ) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>> {
            std::collections::HashMap::new()
        }
    }

    #[test]
    fn signature_help_returns_signature_for_command() {
        let rml = Url::parse("file:///x.rml").unwrap();
        let rs = Url::parse("file:///x.rml.rs").unwrap();
        let rs_source = r#"
#[component]
struct Counter {
    pub count: i32,
}
impl Counter {
    #[command]
    fn on_click(&self, id: Uuid, name: String) -> bool { true }
}
"#;
        let rml_source = "<component><button onclick={on_click}>+</button></component>";

        let mut ws = crate::workspace::Workspace::new();
        ws.refresh_codebehind(&rs, rs_source);
        ws.register_pair(rml.clone(), rs.clone());
        ws.open_document(rml.clone(), rml_source, 0);

        // 定位光标到 onclick 属性中点
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
        let pos = conv::offset_to_position(mid, rml_source, &doc.tree.line_starts);

        let q = MockQuery;
        let help = signature_help(&rml, pos, &ws, &q).expect("should return signature");
        assert_eq!(help.signatures.len(), 1);
        let sig = &help.signatures[0];
        assert_eq!(sig.label, "fn(Uuid, String) -> bool");
        assert_eq!(sig.parameters.as_ref().unwrap().len(), 2);
    }
}
