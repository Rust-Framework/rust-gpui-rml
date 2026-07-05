//! `RustSemanticQuery` 的 rust-analyzer 实现
//!
//! 本文件是隔离层的关键：所有 `ra_ap_*` 类型的转换都在此完成，
//! 上层（handlers / crosslang / features）只依赖 `RustSemanticQuery` 中性接口。

use std::sync::Arc;

use lsp_types::{CompletionItemKind, DiagnosticSeverity, Position, Range, Url};
// 注意：`triomphe::Arc` 是 RA 内部使用的 Arc 实现，与 `std::sync::Arc` 不兼容。
// `analysis.file_text()` 返回 `triomphe::Arc<str>`，需通过 `to_string()` 转换为 `String`。

use super::host::RaHost;
use super::query::{
    ComponentInfo, CompletionEntry, HoverInfo, RustDiagnostic, RustSemanticQuery, SymbolInfo,
    SymbolKind, SymbolLocation,
};

/// RA 适配器：桥接 `RustSemanticQuery` 到 `ra_ap_ide::Analysis`
pub struct RaAdapter {
    host: Arc<RaHost>,
}

impl RaAdapter {
    pub fn new(host: Arc<RaHost>) -> Self {
        Self { host }
    }
}

// ──────────────────────────────────────────────────────────────────────────
// Url ↔ FileId 转换
// ──────────────────────────────────────────────────────────────────────────

/// Url → FileId（在 Vfs 中查找匹配路径）
fn url_to_file_id(host: &RaHost, uri: &Url) -> Option<ra_ap_ide::FileId> {
    if uri.scheme() != "file" {
        return None;
    }
    let path = uri.to_file_path().ok()?;
    let abs = ra_ap_vfs::AbsPathBuf::assert_utf8(path);
    let vfs_path = ra_ap_vfs::VfsPath::from(abs);
    host.with_vfs(|vfs| vfs.file_id(&vfs_path).map(|(id, _)| id))
        .flatten()
}

/// FileId → Url
fn file_id_to_url(host: &RaHost, file_id: ra_ap_ide::FileId) -> Option<Url> {
    let path = host.with_vfs(|vfs| vfs.file_path(file_id).as_path().map(|p| p.to_path_buf()))??;
    Url::from_file_path(&path).ok()
}

// ──────────────────────────────────────────────────────────────────────────
// Position ↔ TextSize 转换（基于 LineIndex）
// ──────────────────────────────────────────────────────────────────────────

/// 获取文件文本 + LineIndex
fn file_text_and_index(
    host: &RaHost,
    file_id: ra_ap_ide::FileId,
) -> Option<(String, ra_ap_ide::LineIndex)> {
    let analysis = host.analysis()?;
    let text = analysis.file_text(file_id).ok()?.to_string();
    let line_index = ra_ap_ide::LineIndex::new(&text);
    Some((text, line_index))
}

/// LSP Position → TextSize
fn position_to_offset(line_index: &ra_ap_ide::LineIndex, pos: Position) -> ra_ap_ide::TextSize {
    let line_col = ra_ap_ide::LineCol {
        line: pos.line,
        col: pos.character,
    };
    line_index
        .offset(line_col)
        .unwrap_or_else(|| ra_ap_ide::TextSize::from(u32::MAX))
}

/// TextSize → LSP Position
fn offset_to_position(line_index: &ra_ap_ide::LineIndex, offset: ra_ap_ide::TextSize) -> Position {
    let lc = line_index.line_col(offset);
    Position {
        line: lc.line,
        character: lc.col,
    }
}

/// TextRange → LSP Range
fn text_range_to_range(
    line_index: &ra_ap_ide::LineIndex,
    range: ra_ap_ide::TextRange,
) -> Range {
    Range {
        start: offset_to_position(line_index, range.start()),
        end: offset_to_position(line_index, range.end()),
    }
}

/// FileId + TextRange → SymbolLocation
fn range_to_location(
    host: &RaHost,
    file_id: ra_ap_ide::FileId,
    range: ra_ap_ide::TextRange,
) -> Option<SymbolLocation> {
    let uri = file_id_to_url(host, file_id)?;
    let (_, line_index) = file_text_and_index(host, file_id)?;
    Some(SymbolLocation {
        uri,
        range: text_range_to_range(&line_index, range),
    })
}

// ──────────────────────────────────────────────────────────────────────────
// RustSemanticQuery 实现
// ──────────────────────────────────────────────────────────────────────────

impl RustSemanticQuery for RaAdapter {
    fn open_document(&mut self, _uri: &Url, _text: &str) {
        // .rml.rs 文件由 RA 的 ProjectModel 自行加载与监控，
        // 此处仅用于 future：若用户打开了未在 workspace 中的 .rml.rs，
        // 需手动注入到 Vfs。当前 MVP 阶段忽略。
    }

    fn apply_change(&mut self, _uri: &Url, _text: &str) {
        // TODO: 增量同步到 Vfs + AnalysisHost
    }

    fn close_document(&mut self, _uri: &Url) {}

    fn goto_definition(&self, uri: &Url, pos: Position) -> Vec<SymbolLocation> {
        let analysis = match self.host.analysis() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let file_id = match url_to_file_id(&self.host, uri) {
            Some(f) => f,
            None => return Vec::new(),
        };
        let (_, line_index) = match file_text_and_index(&self.host, file_id) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let offset = position_to_offset(&line_index, pos);
        let config = ra_ap_ide::GotoDefinitionConfig {
            ra_fixture: ra_ap_ide::RaFixtureConfig::default(),
        };
        let result = match analysis.goto_definition(
            ra_ap_ide::FilePosition { file_id, offset },
            &config,
        ) {
            Ok(it) => it,
            Err(_) => return Vec::new(),
        };
        let navs = match result {
            Some(range_info) => range_info.info,
            None => return Vec::new(),
        };
        navs.into_iter()
            .filter_map(|n| {
                let range = n.focus_or_full_range();
                range_to_location(&self.host, n.file_id, range)
            })
            .collect()
    }

    fn hover(&self, uri: &Url, pos: Position) -> Option<HoverInfo> {
        let analysis = self.host.analysis()?;
        let file_id = url_to_file_id(&self.host, uri)?;
        let (_, line_index) = file_text_and_index(&self.host, file_id)?;
        let offset = position_to_offset(&line_index, pos);
        let config = ra_ap_ide::HoverConfig {
            links_in_hover: false,
            memory_layout: None,
            documentation: true,
            keywords: true,
            format: ra_ap_ide::HoverDocFormat::Markdown,
            max_trait_assoc_items_count: None,
            max_fields_count: None,
            max_enum_variants_count: None,
            max_subst_ty_len: ra_ap_ide::SubstTyLen::Unlimited,
            show_drop_glue: false,
            ra_fixture: ra_ap_ide::RaFixtureConfig::default(),
        };
        // hover 接收 FileRange：以光标位置构造 0-length range
        let frange = ra_ap_ide::FileRange {
            file_id,
            range: ra_ap_ide::TextRange::empty(offset),
        };
        let result = analysis.hover(&config, frange).ok()??;
        let content = result.info.markup.as_str().to_string();
        let range = Some(text_range_to_range(&line_index, result.range));
        Some(HoverInfo { content, range })
    }

    fn completion(&self, uri: &Url, pos: Position) -> Vec<CompletionEntry> {
        let analysis = match self.host.analysis() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let file_id = match url_to_file_id(&self.host, uri) {
            Some(f) => f,
            None => return Vec::new(),
        };
        let (_, line_index) = match file_text_and_index(&self.host, file_id) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let offset = position_to_offset(&line_index, pos);
        let config = ra_ap_ide::CompletionConfig {
            enable_postfix_completions: true,
            enable_imports_on_the_fly: false,
            enable_self_on_the_fly: false,
            enable_auto_iter: false,
            enable_auto_await: false,
            enable_private_editable: false,
            enable_term_search: false,
            term_search_fuel: 0,
            full_function_signatures: false,
            callable: Some(ra_ap_ide::CallableSnippets::FillArguments),
            add_colons_to_module: true,
            add_semicolon_to_unit: true,
            snippet_cap: None,
            insert_use: ra_ap_ide_db::imports::insert_use::InsertUseConfig {
                granularity: ra_ap_ide_db::imports::insert_use::ImportGranularity::Crate,
                enforce_granularity: true,
                prefix_kind: ra_ap_hir::PrefixKind::Plain,
                group: true,
                skip_glob_imports: true,
            },
            prefer_no_std: false,
            prefer_prelude: true,
            prefer_absolute: false,
            snippets: Vec::new(),
            limit: None,
            fields_to_resolve: ra_ap_ide::CompletionFieldsToResolve::empty(),
            exclude_flyimport: Vec::new(),
            exclude_traits: &[],
            ra_fixture: ra_ap_ide::RaFixtureConfig::default(),
        };
        let items = match analysis.completions(
            &config,
            ra_ap_ide::FilePosition { file_id, offset },
            None,
        ) {
            Ok(Some(it)) => it,
            _ => return Vec::new(),
        };
        items
            .into_iter()
            .map(|i| {
                let label = i.label.primary.to_string();
                let kind = map_completion_kind(i.kind);
                let insert_text = i.lookup().to_string();
                CompletionEntry {
                    label,
                    kind,
                    detail: i.detail,
                    insert_text: Some(insert_text),
                }
            })
            .collect()
    }

    fn diagnostics(&self, uri: &Url) -> Vec<RustDiagnostic> {
        let analysis = match self.host.analysis() {
            Some(a) => a,
            None => return Vec::new(),
        };
        let file_id = match url_to_file_id(&self.host, uri) {
            Some(f) => f,
            None => return Vec::new(),
        };
        let (_, line_index) = match file_text_and_index(&self.host, file_id) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let diags = match analysis.full_diagnostics(
            &ra_ap_ide::DiagnosticsConfig::test_sample(),
            ra_ap_ide::AssistResolveStrategy::None,
            file_id,
        ) {
            Ok(it) => it,
            Err(_) => return Vec::new(),
        };
        diags
            .into_iter()
            .map(|d| RustDiagnostic {
                range: text_range_to_range(&line_index, d.range.range),
                severity: map_severity(d.severity),
                message: d.message,
                code: Some(d.code.as_str().to_string()),
            })
            .collect()
    }

    fn resolve_member(
        &self,
        rml_rs_uri: &Url,
        struct_name: &str,
        member: &str,
    ) -> Option<SymbolInfo> {
        let file_id = url_to_file_id(&self.host, rml_rs_uri)?;
        let analysis = self.host.analysis()?;
        let source_file = analysis.parse(file_id).ok()?;

        // 阶段 1：在 with_db 内执行 HIR 查询，收集原始数据
        // （不能在 with_db 内调用 range_to_location，否则会重入锁死）
        let member_data: Option<(SymbolKind, String, ra_ap_ide::FileId, ra_ap_ide::TextRange)> =
            self.host.with_db(|db| {
                use ra_ap_hir::{AssocItem, HasSource, HirDisplay, Semantics};
                use ra_ap_syntax::AstNode;

                let sema = Semantics::new(db);

                // 在文件语法树中查找名称匹配的 ast::Struct，转为 hir::Struct
                let hir_struct = source_file
                    .syntax()
                    .descendants()
                    .filter_map(ra_ap_syntax::ast::Struct::cast)
                    .find_map(|s| {
                        let h = sema.to_struct_def(&s)?;
                        (h.name(db).as_str() == struct_name).then_some(h)
                    })?;

                let krate = hir_struct.module(db).krate(db);
                let display_target = krate.to_display_target(db);

                // 优先查字段
                for field in hir_struct.fields(db) {
                    if field.name(db).as_str() == member {
                        let type_str = field.ty(db).display(db, display_target).to_string();
                        let src = field.source(db)?;
                        let fid = src.file_id.original_file(db).file_id(db);
                        let range = src.value.syntax().text_range();
                        return Some((SymbolKind::Field, type_str, fid, range));
                    }
                }

                // 字段未命中，查 impl 块中的方法
                let struct_ty = hir_struct.ty(db);
                for impl_ in ra_ap_hir::Impl::all_for_type(db, struct_ty) {
                    for item in impl_.items(db) {
                        if let AssocItem::Function(f) = item {
                            if f.name(db).as_str() == member {
                                let type_str = f.display(db, display_target).to_string();
                                let src = f.source(db)?;
                                let fid = src.file_id.original_file(db).file_id(db);
                                let range = src.value.syntax().text_range();
                                return Some((SymbolKind::Method, type_str, fid, range));
                            }
                        }
                    }
                }

                None
            })?;

        // 阶段 2：在 with_db 外将 (FileId, TextRange) 转为 SymbolLocation
        let (kind, type_str, fid, range) = member_data?;
        let location = range_to_location(&self.host, fid, range);
        Some(SymbolInfo {
            name: member.to_string(),
            kind,
            type_str: Some(type_str),
            doc: None,
            location,
        })
    }

    fn find_struct(&self, struct_name: &str) -> Option<SymbolLocation> {
        let analysis = self.host.analysis()?;
        let mut query = ra_ap_ide_db::symbol_index::Query::new(struct_name.to_string());
        query.exact();
        query.only_types();
        let results = analysis.symbol_search(query, 10).ok()?;
        let nav = results
            .into_iter()
            .find(|n| n.kind == Some(ra_ap_ide::SymbolKind::Struct))?;
        let range = nav.focus_or_full_range();
        range_to_location(&self.host, nav.file_id, range)
    }

    fn struct_slots(&self, rml_rs_uri: &Url, struct_name: &str) -> Vec<String> {
        let Some(file_id) = url_to_file_id(&self.host, rml_rs_uri) else {
            return Vec::new();
        };
        let Some(analysis) = self.host.analysis() else {
            return Vec::new();
        };
        let Ok(source_file) = analysis.parse(file_id) else {
            return Vec::new();
        };

        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let ast_struct = source_file
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == struct_name));

        let Some(ast_struct) = ast_struct else {
            return Vec::new();
        };

        parse_slots_from_attrs(&ast_struct)
    }

    fn command_signature(
        &self,
        rml_rs_uri: &Url,
        struct_name: &str,
        method: &str,
    ) -> Option<SymbolInfo> {
        // 委托 resolve_member：#[command] 过滤由调用方通过 StructMetadata.commands 完成
        self.resolve_member(rml_rs_uri, struct_name, method)
    }

    fn list_components(&self, prefix: &str) -> Vec<ComponentInfo> {
        let Some(analysis) = self.host.analysis() else {
            return Vec::new();
        };
        let mut query = ra_ap_ide_db::symbol_index::Query::new(prefix.to_string());
        query.only_types();
        // 不调 exact()，启用前缀/模糊匹配
        let Ok(results) = analysis.symbol_search(query, 50) else {
            return Vec::new();
        };

        results
            .into_iter()
            .filter(|n| n.kind == Some(ra_ap_ide::SymbolKind::Struct))
            .filter_map(|n| {
                let name = n.name.to_string();
                let Ok(source_file) = analysis.parse(n.file_id) else {
                    return None;
                };
                use ra_ap_syntax::ast::{AstNode, HasName, Struct};
                let has_component = source_file
                    .syntax()
                    .descendants()
                    .filter_map(Struct::cast)
                    .find(|s| s.name().is_some_and(|nm| nm.text() == name))
                    .is_some_and(has_component_attr);
                if !has_component {
                    return None;
                }
                let location = range_to_location(&self.host, n.file_id, n.focus_or_full_range());
                Some(ComponentInfo { name, location })
            })
            .collect()
    }

    fn is_ready(&self) -> bool {
        self.host.is_ready()
    }

    fn find_references(
        &self,
        _uri: &Url,
        _pos: Position,
        _include_declaration: bool,
    ) -> Vec<SymbolLocation> {
        // RA 后端实现待 ra_ap_* 依赖恢复后补齐
        Vec::new()
    }

    fn rename_member(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _member: &str,
        _new_name: &str,
    ) -> Vec<lsp_types::TextEdit> {
        // RA 后端实现待 ra_ap_* 依赖恢复后补齐
        Vec::new()
    }

    fn rename_struct(
        &self,
        _old_name: &str,
        _new_name: &str,
    ) -> std::collections::HashMap<Url, Vec<lsp_types::TextEdit>> {
        // RA 后端实现待 ra_ap_* 依赖恢复后补齐
        std::collections::HashMap::new()
    }
}

// ──────────────────────────────────────────────────────────────────────────
// 类型映射辅助
// ──────────────────────────────────────────────────────────────────────────

/// 判断 struct 是否标注了 `#[component]` 属性
fn has_component_attr(s: ra_ap_syntax::ast::Struct) -> bool {
    use ra_ap_syntax::ast::{AstNode, HasAttrs};
    s.attrs().any(|attr| {
        attr.path()
            .is_some_and(|p| p.syntax().text() == "component")
    })
}

/// 从 `#[component(slots = ["header", "footer"])]` 属性解析 slot 名称列表
fn parse_slots_from_attrs(s: &ra_ap_syntax::ast::Struct) -> Vec<String> {
    use ra_ap_syntax::ast::{AstNode, HasAttrs};
    for attr in s.attrs() {
        let is_component = attr
            .path()
            .is_some_and(|p| p.syntax().text() == "component");
        if !is_component {
            continue;
        }
        // 取属性完整文本（如 `#[component(slots = ["header", "footer"])]`）
        let attr_text = attr.syntax().text().to_string();
        return extract_slot_names(&attr_text);
    }
    Vec::new()
}

/// 从属性参数字符串（如 `#[component(slots = ["header", "footer"])]`）提取 slot 名
fn extract_slot_names(attr_args: &str) -> Vec<String> {
    // 跳过 #[ 前缀，避免将属性语法的 [ 误识别为数组起始
    let search_start = if attr_args.starts_with("#[") { 2 } else { 0 };
    let Some(start) = attr_args[search_start..].find('[') else {
        return Vec::new();
    };
    let start = start + search_start;
    let Some(end) = attr_args[start..].find(']') else {
        return Vec::new();
    };
    let array_str = &attr_args[start + 1..start + end];
    array_str
        .split(',')
        .filter_map(|s| {
            let s = s.trim();
            (s.starts_with('"') && s.ends_with('"') && s.len() >= 2).then(|| {
                s[1..s.len() - 1].to_string()
            })
        })
        .collect()
}

/// RA Severity → LSP DiagnosticSeverity
fn map_severity(s: ra_ap_ide::Severity) -> DiagnosticSeverity {
    match s {
        ra_ap_ide::Severity::Error => DiagnosticSeverity::ERROR,
        ra_ap_ide::Severity::Warning => DiagnosticSeverity::WARNING,
        ra_ap_ide::Severity::WeakWarning => DiagnosticSeverity::HINT,
        ra_ap_ide::Severity::Allow => DiagnosticSeverity::HINT,
    }
}

/// RA CompletionItemKind → LSP CompletionItemKind
fn map_completion_kind(k: ra_ap_ide::CompletionItemKind) -> CompletionItemKind {
    use ra_ap_ide::CompletionItemKind as K;
    use ra_ap_ide::SymbolKind as SK;
    match k {
        K::Binding => CompletionItemKind::VARIABLE,
        K::BuiltinType => CompletionItemKind::STRUCT,
        K::InferredType => CompletionItemKind::VARIABLE,
        K::Keyword => CompletionItemKind::KEYWORD,
        K::Snippet => CompletionItemKind::SNIPPET,
        K::UnresolvedReference => CompletionItemKind::REFERENCE,
        K::Expression => CompletionItemKind::TEXT,
        K::SymbolKind(sk) => match sk {
            SK::Field | SK::SelfParam | SK::SelfType => CompletionItemKind::FIELD,
            SK::Function | SK::Method => CompletionItemKind::METHOD,
            SK::Struct => CompletionItemKind::STRUCT,
            SK::Enum => CompletionItemKind::ENUM,
            SK::Trait => CompletionItemKind::INTERFACE,
            SK::Module | SK::CrateRoot => CompletionItemKind::MODULE,
            SK::Local | SK::LifetimeParam | SK::ConstParam | SK::TypeParam => {
                CompletionItemKind::VARIABLE
            }
            SK::Const | SK::Static => CompletionItemKind::CONSTANT,
            SK::TypeAlias => CompletionItemKind::STRUCT,
            SK::Union => CompletionItemKind::STRUCT,
            SK::Macro | SK::ProcMacro => CompletionItemKind::FUNCTION,
            SK::Label => CompletionItemKind::REFERENCE,
            _ => CompletionItemKind::TEXT,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_slot_names_basic() {
        let names = extract_slot_names(r#"(slots = ["header", "footer"])"#);
        assert_eq!(names, vec!["header", "footer"]);
    }

    #[test]
    fn extract_slot_names_three_slots() {
        let names = extract_slot_names(r#"(slots = ["header", "footer", "default"])"#);
        assert_eq!(names, vec!["header", "footer", "default"]);
    }

    #[test]
    fn extract_slot_names_empty_array() {
        let names = extract_slot_names(r#"(slots = [])"#);
        assert!(names.is_empty());
    }

    #[test]
    fn extract_slot_names_no_array_returns_empty() {
        let names = extract_slot_names(r#"(other = true)"#);
        assert!(names.is_empty());
    }

    #[test]
    fn parse_slots_from_ast_struct() {
        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let source = r#"
            #[component(slots = ["header", "footer", "default"])]
            struct MyComponent { pub count: i32 }
        "#;
        let parse = ra_ap_syntax::SourceFile::parse(
            source,
            ra_ap_syntax::Edition::Edition2021,
        );
        let tree = parse.tree();
        let s = tree
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == "MyComponent"))
            .unwrap();
        let slots = parse_slots_from_attrs(&s);
        assert_eq!(slots, vec!["header", "footer", "default"]);
    }

    #[test]
    fn parse_slots_no_component_attr_returns_empty() {
        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let source = r#"struct Plain { pub v: i32 }"#;
        let parse = ra_ap_syntax::SourceFile::parse(
            source,
            ra_ap_syntax::Edition::Edition2021,
        );
        let tree = parse.tree();
        let s = tree
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == "Plain"))
            .unwrap();
        let slots = parse_slots_from_attrs(&s);
        assert!(slots.is_empty());
    }

    #[test]
    fn has_component_attr_detects_component() {
        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let source = r#"
            #[component(slots = ["header"])]
            struct MyWidget { pub count: i32 }
        "#;
        let parse = ra_ap_syntax::SourceFile::parse(
            source,
            ra_ap_syntax::Edition::Edition2021,
        );
        let tree = parse.tree();
        let s = tree
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == "MyWidget"))
            .unwrap();
        assert!(has_component_attr(s));
    }

    #[test]
    fn has_component_attr_detects_plain_struct() {
        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let source = r#"struct Plain { pub v: i32 }"#;
        let parse = ra_ap_syntax::SourceFile::parse(
            source,
            ra_ap_syntax::Edition::Edition2021,
        );
        let tree = parse.tree();
        let s = tree
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == "Plain"))
            .unwrap();
        assert!(!has_component_attr(s));
    }

    #[test]
    fn has_component_attr_ignores_other_attrs() {
        use ra_ap_syntax::ast::{AstNode, HasName, Struct};
        let source = r#"
            #[derive(Debug)]
            #[window(title = "Main")]
            struct WithOtherAttrs { pub v: i32 }
        "#;
        let parse = ra_ap_syntax::SourceFile::parse(
            source,
            ra_ap_syntax::Edition::Edition2021,
        );
        let tree = parse.tree();
        let s = tree
            .syntax()
            .descendants()
            .filter_map(Struct::cast)
            .find(|s| s.name().is_some_and(|n| n.text() == "WithOtherAttrs"))
            .unwrap();
        assert!(!has_component_attr(s));
    }
}
