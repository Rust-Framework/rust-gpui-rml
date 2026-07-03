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
    CompletionEntry, HoverInfo, RustDiagnostic, RustSemanticQuery, SymbolInfo, SymbolLocation,
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
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _member: &str,
    ) -> Option<SymbolInfo> {
        // TODO Phase 4：基于 HIR 查询 struct 的 field/method 类型 + 定义位置
        // Phase 3 由 coordinator 使用 StructMetadata.field_types 替代
        None
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

    fn struct_slots(&self, _rml_rs_uri: &Url, _struct_name: &str) -> Vec<String> {
        // Phase 3 由 coordinator 使用 StructMetadata.slots 替代
        Vec::new()
    }

    fn command_signature(
        &self,
        _rml_rs_uri: &Url,
        _struct_name: &str,
        _method: &str,
    ) -> Option<SymbolInfo> {
        // TODO Phase 4：基于 HIR 查询 #[command] 方法签名
        None
    }

    fn is_ready(&self) -> bool {
        self.host.is_ready()
    }
}

// ──────────────────────────────────────────────────────────────────────────
// 类型映射辅助
// ──────────────────────────────────────────────────────────────────────────

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
