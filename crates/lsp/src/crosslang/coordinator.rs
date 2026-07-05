//! 跨语言协调器：桥接 .rml 语义层与 .rml.rs 符号层
//!
//! 职责：
//! - 将 .rml 绑定表达式解析为 Rust 符号信息（类型/文档），供 hover 使用
//! - 校验 .rml 自定义组件标签对应的 Rust struct 存在性，供 goto/校验使用
//! - .rml 绑定路径 → .rml.rs 字段定义位置（Phase 4：基于 RA HIR）
//!
//! 设计原则：
//! - 协调器无状态，所有查询通过 `&ProjectIndex` / `&dyn RustSemanticQuery` 完成
//! - 复用 StructMetadata（field_types / computed_returns），避免重复实现 HIR 字段查询
//! - rust_query 不可用时优雅降级（返回 None）
//!
//! 数据流：
//! ```text
//! .rml 绑定 {user.name}
//!   ↓ resolver::parse_binding_path
//! BindingPath { root: "user", members: ["name"] }
//!   ↓ coordinator::resolve_binding
//! ProjectIndex.metadata_for(rml_uri) → StructMetadata.field_types["user"] → "User"
//!   ↓
//! SymbolInfo { name: "user", kind: Field, type_str: Some("User") }
//! ```

use lsp_types::Url;

use crate::crosslang::resolver::parse_binding_path;
use crate::rust::query::{RustSemanticQuery, SymbolInfo, SymbolKind, SymbolLocation};
use crate::workspace::project_index::ProjectIndex;

/// 解析 .rml 绑定表达式 → Rust 符号信息（类型字符串）
///
/// Phase 3：仅根标识符级解析，使用 `StructMetadata.field_types` / `computed_returns`。
/// members 链式访问（如 `user.address.city`）的中间类型推导留给 Phase 4。
///
/// 返回 None 的情形：
/// - 表达式无法解析出根标识符
/// - 根标识符是 builtin（cx/_window/true/false/self/Self）
/// - metadata 未加载或字段不存在
pub fn resolve_binding(
    rml_uri: &Url,
    binding_expr: &str,
    index: &ProjectIndex,
) -> Option<SymbolInfo> {
    let path = parse_binding_path(binding_expr)?;
    if is_builtin_ident(&path.root) {
        return None;
    }
    let metadata_map = index.metadata_for(rml_uri)?;
    // MVP：取第一个 struct 的 metadata（与 semantics::binder 一致）
    let meta = metadata_map.values().next()?;

    if let Some(type_str) = meta.field_types.get(&path.root) {
        return Some(SymbolInfo {
            name: path.root,
            kind: SymbolKind::Field,
            type_str: Some(type_str.clone()),
            doc: None,
            location: None,
        });
    }
    if meta.computed_methods.contains(&path.root) {
        let return_type = meta.computed_returns.get(&path.root).cloned();
        return Some(SymbolInfo {
            name: path.root,
            kind: SymbolKind::Method,
            type_str: return_type,
            doc: None,
            location: None,
        });
    }
    None
}

/// 校验 .rml 自定义组件标签 → 返回 struct 定义位置
///
/// 用于 `<MyComponent>` 的 goto definition / hover。
/// 内部委托 `rust_query.find_struct`（RA workspace 符号搜索）。
pub fn find_component(
    tag_name: &str,
    rust_query: &dyn RustSemanticQuery,
) -> Option<SymbolLocation> {
    rust_query.find_struct(tag_name)
}

/// .rml 绑定路径 → .rml.rs 字段定义位置
///
/// 解析绑定表达式，在配对的 .rml.rs 文件中查找根标识符对应的字段/方法定义位置。
/// Phase 4：仅根标识符级跳转（如 `{count}` → count 字段）。
/// 链式访问（如 `{user.name}` → User 的 name 字段）需多步类型推导，留给后续迭代。
pub fn goto_def_for_binding(
    rml_uri: &Url,
    binding_expr: &str,
    index: &ProjectIndex,
    rust_query: &dyn RustSemanticQuery,
) -> Option<SymbolLocation> {
    let path = parse_binding_path(binding_expr)?;
    if is_builtin_ident(&path.root) {
        return None;
    }
    let rml_rs_uri = index.codebehind_uri(rml_uri)?;
    let metadata_map = index.metadata_for(rml_uri)?;

    // 在 code-behind 的所有 struct 中查找包含该字段/方法的一个
    for (struct_name, meta) in metadata_map {
        let has_member = meta.field_types.contains_key(&path.root)
            || meta.computed_methods.contains(&path.root);
        if !has_member {
            continue;
        }
        // 通过 RA HIR 解析字段/方法的精确定义位置
        let symbol_info = rust_query.resolve_member(rml_rs_uri, struct_name, &path.root)?;
        return symbol_info.location;
    }
    None
}

/// 内置标识符白名单：不视为 ViewModel 字段
fn is_builtin_ident(s: &str) -> bool {
    matches!(s, "cx" | "_window" | "true" | "false" | "self" | "Self")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_binding_field_type() {
        let source = r#"
            #[component]
            struct Counter {
                pub count: i32,
                pub name: String,
            }
        "#;
        let rml = Url::parse("file:///foo/counter.rml").unwrap();
        let rs = Url::parse("file:///foo/counter.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, source);
        idx.register_pair(rml.clone(), rs);
        let info = resolve_binding(&rml, "count", &idx).unwrap();
        assert_eq!(info.name, "count");
        assert_eq!(info.kind, SymbolKind::Field);
        assert_eq!(info.type_str.as_deref(), Some("i32"));
    }

    #[test]
    fn resolve_binding_computed_method() {
        let source = r#"
            #[component]
            struct Counter {
                pub count: i32,
            }
            impl Counter {
                #[computed]
                fn double_count(&self) -> i32 { self.count * 2 }
            }
        "#;
        let rml = Url::parse("file:///foo/counter.rml").unwrap();
        let rs = Url::parse("file:///foo/counter.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, source);
        idx.register_pair(rml.clone(), rs);
        let info = resolve_binding(&rml, "double_count", &idx).unwrap();
        assert_eq!(info.name, "double_count");
        assert_eq!(info.kind, SymbolKind::Method);
        assert_eq!(info.type_str.as_deref(), Some("i32"));
    }

    #[test]
    fn resolve_binding_builtin_returns_none() {
        let rml = Url::parse("file:///foo/x.rml").unwrap();
        let rs = Url::parse("file:///foo/x.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, "#[component] struct X { pub v: i32 }");
        idx.register_pair(rml.clone(), rs);
        assert!(resolve_binding(&rml, "cx", &idx).is_none());
        assert!(resolve_binding(&rml, "_window", &idx).is_none());
        assert!(resolve_binding(&rml, "self", &idx).is_none());
    }

    #[test]
    fn resolve_binding_unknown_field_returns_none() {
        let rml = Url::parse("file:///foo/x.rml").unwrap();
        let rs = Url::parse("file:///foo/x.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, "#[component] struct X { pub v: i32 }");
        idx.register_pair(rml.clone(), rs);
        assert!(resolve_binding(&rml, "nonexistent", &idx).is_none());
    }

    #[test]
    fn resolve_binding_path_access_returns_root_type() {
        let source = r#"
            #[component]
            struct App {
                pub user: User,
            }
        "#;
        let rml = Url::parse("file:///foo/app.rml").unwrap();
        let rs = Url::parse("file:///foo/app.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, source);
        idx.register_pair(rml.clone(), rs);
        let info = resolve_binding(&rml, "user.name", &idx).unwrap();
        assert_eq!(info.name, "user");
        assert_eq!(info.type_str.as_deref(), Some("User"));
    }

    #[test]
    fn resolve_binding_unpaired_rml_returns_none() {
        let rml = Url::parse("file:///foo/unpaired.rml").unwrap();
        let idx = ProjectIndex::new();
        assert!(resolve_binding(&rml, "count", &idx).is_none());
    }

    #[test]
    fn resolve_binding_invalid_expr_returns_none() {
        let rml = Url::parse("file:///foo/x.rml").unwrap();
        let rs = Url::parse("file:///foo/x.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, "#[component] struct X { pub v: i32 }");
        idx.register_pair(rml.clone(), rs);
        assert!(resolve_binding(&rml, "", &idx).is_none());
        assert!(resolve_binding(&rml, "   ", &idx).is_none());
        assert!(resolve_binding(&rml, "123", &idx).is_none());
    }

    struct NoopQuery;
    impl RustSemanticQuery for NoopQuery {
        fn open_document(&mut self, _: &Url, _: &str) {}
        fn apply_change(&mut self, _: &Url, _: &str) {}
        fn close_document(&mut self, _: &Url) {}
        fn goto_definition(&self, _: &Url, _: lsp_types::Position) -> Vec<SymbolLocation> {
            Vec::new()
        }
        fn hover(&self, _: &Url, _: lsp_types::Position) -> Option<crate::rust::query::HoverInfo> {
            None
        }
        fn completion(
            &self,
            _: &Url,
            _: lsp_types::Position,
        ) -> Vec<crate::rust::query::CompletionEntry> {
            Vec::new()
        }
        fn diagnostics(&self, _: &Url) -> Vec<crate::rust::query::RustDiagnostic> {
            Vec::new()
        }
        fn resolve_member(&self, _: &Url, _: &str, _: &str) -> Option<SymbolInfo> {
            None
        }
        fn find_struct(&self, _: &str) -> Option<SymbolLocation> {
            None
        }
        fn struct_slots(&self, _: &Url, _: &str) -> Vec<String> {
            Vec::new()
        }
        fn command_signature(&self, _: &Url, _: &str, _: &str) -> Option<SymbolInfo> {
            None
        }
        fn list_components(&self, _: &str) -> Vec<crate::rust::query::ComponentInfo> {
            Vec::new()
        }
        fn is_ready(&self) -> bool {
            false
        }
        fn find_references(
            &self,
            _uri: &Url,
            _pos: lsp_types::Position,
            _include_declaration: bool,
        ) -> Vec<SymbolLocation> {
            Vec::new()
        }
        fn rename_member(
            &self,
            _rml_rs_uri: &Url,
            _struct_name: &str,
            _member: &str,
            _new_name: &str,
        ) -> Vec<lsp_types::TextEdit> {
            Vec::new()
        }
        fn rename_struct(
            &self,
            _old_name: &str,
            _new_name: &str,
        ) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>> {
            std::collections::HashMap::new()
        }
    }

    #[test]
    fn find_component_delegates_to_rust_query() {
        let q = NoopQuery;
        assert!(find_component("MyComponent", &q).is_none());
    }

    #[test]
    fn goto_def_for_binding_returns_none_when_rust_query_unavailable() {
        let rml = Url::parse("file:///foo/x.rml").unwrap();
        let rs = Url::parse("file:///foo/x.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, "#[component] struct X { pub v: i32 }");
        idx.register_pair(rml.clone(), rs);
        let q = NoopQuery;
        // resolve_member 返回 None（RA 未就绪），goto_def 应优雅降级
        assert!(goto_def_for_binding(&rml, "v", &idx, &q).is_none());
    }

    #[test]
    fn goto_def_for_binding_skips_builtin_ident() {
        let rml = Url::parse("file:///foo/x.rml").unwrap();
        let rs = Url::parse("file:///foo/x.rml.rs").unwrap();
        let mut idx = ProjectIndex::new();
        idx.refresh_codebehind(&rs, "#[component] struct X { pub v: i32 }");
        idx.register_pair(rml.clone(), rs);
        let q = NoopQuery;
        assert!(goto_def_for_binding(&rml, "cx", &idx, &q).is_none());
    }

    #[test]
    fn goto_def_for_binding_unpaired_rml_returns_none() {
        let rml = Url::parse("file:///foo/unpaired.rml").unwrap();
        let idx = ProjectIndex::new();
        let q = NoopQuery;
        assert!(goto_def_for_binding(&rml, "v", &idx, &q).is_none());
    }
}
