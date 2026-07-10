//! 补全功能：按光标位置上下文分派
//!
//! - `<` 后 → 标签补全
//! - 标签内属性名位置 → 属性补全（区分 static/bind/event）
//! - `{...}` / `value={...}` 内 → 绑定路径补全
//! - `onclick="..."` 内 → 命令补全

use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, InsertTextFormat};

use crate::features::source::{CompletionKind, CompletionSource};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::Workspace;

/// 补全上下文（根据光标位置推断）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionContext {
    /// `<` 后：标签名补全（携带已输入前缀，用于 RA 动态组件查询）
    TagName { prefix: String },
    /// 标签内属性名位置
    AttributeName { tag: Option<&'static str> },
    /// `{...}` 绑定表达式内
    BindingExpr,
    /// `onclick="..."` 命令名内
    CommandName,
    /// 无法确定上下文
    Unknown,
}

/// 执行补全
pub fn complete(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
) -> Option<CompletionResponse> {
    let doc = workspace.document(uri)?;
    let source = doc.tree.text();
    let line_starts = &doc.tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let ctx = infer_context(source, byte_offset);
    let source_provider = CompletionSource::new(workspace.index());

    let items: Vec<CompletionItem> = match ctx {
        CompletionContext::TagName { prefix } => {
            collect_tag_completions(&source_provider, rust_query, &prefix)
        }
        CompletionContext::AttributeName { tag } => {
            collect_attr_completions(&source_provider, tag)
        }
        CompletionContext::BindingExpr => {
            collect_binding_completions(&source_provider, uri)
        }
        CompletionContext::CommandName => {
            collect_command_completions(&source_provider, uri)
        }
        CompletionContext::Unknown => Vec::new(),
    };

    if items.is_empty() {
        None
    } else {
        Some(CompletionResponse::Array(items))
    }
}

/// 推断光标位置的补全上下文
fn infer_context(source: &str, offset: usize) -> CompletionContext {
    // 取光标前的文本（从行首或最近的关键字符开始）
    let before = &source[..offset.min(source.len())];

    // 检查是否在 `onclick="..."` 等事件属性的值内
    if let Some(ctx) = check_command_context(before) {
        return ctx;
    }

    // 检查是否在 `{...}` 绑定表达式内
    if let Some(ctx) = check_binding_context(before) {
        return ctx;
    }

    // 检查是否在 `<tag` 后的属性位置
    if let Some(ctx) = check_attribute_context(before) {
        return ctx;
    }

    // 检查是否在 `<` 后（标签名位置）
    if let Some(ctx) = check_tag_name_context(before) {
        return ctx;
    }

    CompletionContext::Unknown
}

/// `<tag` 或 `<` 后 → 标签名补全
fn check_tag_name_context(before: &str) -> Option<CompletionContext> {
    // 找最后一个 `<`，且其后无空格（紧跟光标）
    let last_lt = before.rfind('<')?;
    let after_lt = &before[last_lt + 1..];
    // 若 `<` 后紧跟字母（标签名进行中）或为空（刚输入 `<`）
    if after_lt.is_empty() || after_lt.chars().all(|c| c.is_alphanumeric() || c == '_') {
        // 排除 `</`（闭合标签不补全）
        if before[last_lt..].starts_with("</") {
            return None;
        }
        return Some(CompletionContext::TagName {
            prefix: after_lt.to_string(),
        });
    }
    None
}

/// `<tag attr...` 后 → 属性名补全
fn check_attribute_context(before: &str) -> Option<CompletionContext> {
    // 找最后一个未闭合的 `<tag`，提取 tag 名
    let last_lt = before.rfind('<')?;
    let after_lt = &before[last_lt + 1..];

    // 闭合标签 `</tag>` 不补全属性
    if after_lt.starts_with('/') {
        return None;
    }

    // 提取 tag 名（第一个空白前的字符序列）
    let tag_end = after_lt
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after_lt.len());
    let tag = &after_lt[..tag_end];

    // 如果 `>` 已出现（标签闭合），不补全属性
    if after_lt.contains('>') {
        return None;
    }

    // tag 为空（只有 `<`）→ 标签名补全，不是属性补全
    if tag.is_empty() {
        return None;
    }

    // 将 tag 名静态化（泄漏为 'static，仅用于 match 查找）
    // 这里用 leak 是 MVP 简化，避免每次分配；补全是短生命周期操作
    let tag_static: &'static str = Box::leak(tag.to_string().into_boxed_str());
    Some(CompletionContext::AttributeName { tag: Some(tag_static) })
}

/// `{expr}` 内 → 绑定路径补全
fn check_binding_context(before: &str) -> Option<CompletionContext> {
    // 找最后一个未闭合的 `{`（排除 `{{` 插值，MVP 简化：单 `{` 即视为绑定）
    let last_brace = before.rfind('{')?;
    let after_brace = &before[last_brace + 1..];
    // 若 `}` 已出现则闭合，不在绑定内
    if after_brace.contains('}') {
        return None;
    }
    Some(CompletionContext::BindingExpr)
}

/// `onclick="..."` 内 → 命令补全
fn check_command_context(before: &str) -> Option<CompletionContext> {
    // 找最后一个 `="`，检查属性名是否以 on 开头
    let last_eq_quote = before.rfind("=\"")?;
    let after = &before[last_eq_quote + 2..];
    // 若 `"` 已闭合则不在值内
    if after.contains('"') {
        return None;
    }
    // 向前查找属性名
    let before_eq = &before[..last_eq_quote];
    let attr_start = before_eq
        .rfind(|c: char| c.is_whitespace())
        .map(|i| i + 1)
        .unwrap_or(0);
    let attr_name = &before_eq[attr_start..];
    if attr_name.starts_with("on") {
        Some(CompletionContext::CommandName)
    } else {
        None
    }
}

fn collect_tag_completions(
    source: &CompletionSource,
    rust_query: &dyn RustSemanticQuery,
    prefix: &str,
) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = source
        .tags()
        .into_iter()
        .map(|c| match c {
            CompletionKind::Tag { name, detail } => CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some(detail),
                insert_text: Some(name),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            },
            _ => CompletionItem::default(),
        })
        .collect();

    // 追加 RA 动态查询的 `#[component]` struct（去重：跳过静态列表已有的标签名）
    let static_names: std::collections::HashSet<String> =
        items.iter().map(|i| i.label.clone()).collect();
    for comp in rust_query.list_components(prefix) {
        if static_names.contains(&comp.name) {
            continue;
        }
        items.push(CompletionItem {
            label: comp.name.clone(),
            kind: Some(CompletionItemKind::STRUCT),
            detail: Some("user component".to_string()),
            insert_text: Some(comp.name),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        });
    }

    items
}

fn collect_attr_completions(source: &CompletionSource, tag: Option<&str>) -> Vec<CompletionItem> {
    let Some(tag) = tag else {
        return Vec::new();
    };
    let prop_set = source.props_for(tag);
    let mut items = Vec::new();

    for prop in prop_set.statics {
        items.push(CompletionItem {
            label: prop.clone(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("static".to_string()),
            insert_text: Some(format!("{}=\"\"", prop)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
    for prop in prop_set.binds {
        items.push(CompletionItem {
            label: prop.clone(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("bind".to_string()),
            insert_text: Some(format!("{}={{}}", prop)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
    for prop in prop_set.events {
        items.push(CompletionItem {
            label: prop.clone(),
            kind: Some(CompletionItemKind::EVENT),
            detail: Some("event".to_string()),
            insert_text: Some(format!("{}=\"\"", prop)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    items
}

fn collect_binding_completions(source: &CompletionSource, uri: &lsp_types::Url) -> Vec<CompletionItem> {
    source
        .binding_paths(uri)
        .into_iter()
        .map(|c| match c {
            CompletionKind::BindingPath { name, detail } => CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(detail),
                insert_text: Some(name),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            },
            _ => CompletionItem::default(),
        })
        .collect()
}

fn collect_command_completions(source: &CompletionSource, uri: &lsp_types::Url) -> Vec<CompletionItem> {
    source
        .commands(uri)
        .into_iter()
        .map(|c| match c {
            CompletionKind::Command { name } => CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some("#[command]".to_string()),
                insert_text: Some(name),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            },
            _ => CompletionItem::default(),
        })
        .collect()
}
