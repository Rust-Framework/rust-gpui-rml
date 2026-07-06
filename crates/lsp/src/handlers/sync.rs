//! 文档同步：didOpen / didChange / didSave / didClose
//!
//! 收到文档变更 → 更新 Workspace（`.rml`）或 rust_query（`.rml.rs`）→ 触发重解析 → 发布诊断。

use anyhow::Result;
use lsp_server::Connection;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
};

use crate::handlers::diagnostics;
use crate::server::connection::ServerState;
use crate::server::doctype;

/// 处理 textDocument/didOpen
pub fn handle_did_open(
    params: serde_json::Value,
    state: &mut ServerState,
    conn: &Connection,
) -> Result<()> {
    let params: DidOpenTextDocumentParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri.clone();
    let text = params.text_document.text.clone();
    let version = params.text_document.version;

    if doctype::is_rust_file(&uri) {
        if doctype::is_rust_codebehind(&uri) {
            state.workspace.refresh_codebehind(&uri, &text);
        }
        state.rust_query.open_document(&uri, &text);
        let diags = diagnostics::collect_rust(&uri, state);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
        if doctype::is_rust_codebehind(&uri) {
            // .rml.rs 变更可能影响配对 .rml 的语义诊断，触发重诊断
            refresh_paired_rml(&uri, state, conn);
        }
    } else if doctype::is_rml_markup(&uri) {
        state.workspace.auto_pair(&uri);
        state.workspace.open_document(uri.clone(), &text, version);
        let diags = diagnostics::collect(&uri, &state.workspace);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
    }

    Ok(())
}

/// 处理 textDocument/didChange
pub fn handle_did_change(
    params: serde_json::Value,
    state: &mut ServerState,
    conn: &Connection,
) -> Result<()> {
    let params: DidChangeTextDocumentParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri.clone();
    let version = params.text_document.version;

    // FULL 同步模式：取最后一个变更事件的完整文本
    let text = params
        .content_changes
        .into_iter()
        .next()
        .map(|e| e.text)
        .unwrap_or_default();

    if doctype::is_rust_file(&uri) {
        if doctype::is_rust_codebehind(&uri) {
            state.workspace.refresh_codebehind(&uri, &text);
        }
        state.rust_query.apply_change(&uri, &text);
        let diags = diagnostics::collect_rust(&uri, state);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
        if doctype::is_rust_codebehind(&uri) {
            refresh_paired_rml(&uri, state, conn);
        }
    } else if doctype::is_rml_markup(&uri) {
        state.workspace.update_document(&uri, &text, version);
        let diags = diagnostics::collect(&uri, &state.workspace);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
    }

    Ok(())
}

/// 处理 textDocument/didSave
pub fn handle_did_save(
    params: serde_json::Value,
    state: &mut ServerState,
    conn: &Connection,
) -> Result<()> {
    let params: DidSaveTextDocumentParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri.clone();

    if doctype::is_rust_file(&uri) {
        if doctype::is_rust_codebehind(&uri) {
            if let Some(text) = params.text {
                state.workspace.refresh_codebehind(&uri, &text);
            }
        }
        if let Some(text) = params.text {
            state.rust_query.apply_change(&uri, &text);
        }
        let diags = diagnostics::collect_rust(&uri, state);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
        if doctype::is_rust_codebehind(&uri) {
            refresh_paired_rml(&uri, state, conn);
        }
    } else if doctype::is_rml_markup(&uri) {
        if let Some(text) = params.text {
            state.workspace.update_document(&uri, &text, 0);
        }
        let diags = diagnostics::collect(&uri, &state.workspace);
        crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;
    }

    Ok(())
}

/// 处理 textDocument/didClose
pub fn handle_did_close(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<()> {
    let params: DidCloseTextDocumentParams = serde_json::from_value(params)?;
    let uri = params.text_document.uri;
    if doctype::is_rust_file(&uri) {
        state.rust_query.close_document(&uri);
    } else {
        state.workspace.close_document(&uri);
    }
    Ok(())
}

/// .rml.rs 变更时，查找配对的 .rml 文件并重新发布诊断
///
/// StructMetadata 变更会影响 .rml 的绑定路径校验、命令校验等语义诊断。
fn refresh_paired_rml(rml_rs_uri: &lsp_types::Url, state: &mut ServerState, conn: &Connection) {
    // 遍历所有打开的 .rml 文档，查找配对为当前 rml_rs_uri 的
    let paired_rmls: Vec<lsp_types::Url> = state
        .workspace
        .index()
        .find_rml_for_codebehind(rml_rs_uri);
    for rml_uri in paired_rmls {
        let diags = diagnostics::collect(&rml_uri, &state.workspace);
        let _ = crate::server::dispatch::send_diagnostics(&rml_uri, diags, conn);
    }
}

