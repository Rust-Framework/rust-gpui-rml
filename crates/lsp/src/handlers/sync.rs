//! 文档同步：didOpen / didChange / didSave / didClose
//!
//! 收到文档变更 → 更新 Workspace → 触发重解析 + 语义重算 → 发布诊断。

use anyhow::Result;
use lsp_server::Connection;
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams,
};

use crate::handlers::diagnostics;
use crate::server::connection::ServerState;

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

    state.workspace.open_document(uri.clone(), &text, version);

    // 发布诊断
    let diags = diagnostics::collect(&uri, &state.workspace);
    crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;

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

    state.workspace.update_document(&uri, &text, version);

    let diags = diagnostics::collect(&uri, &state.workspace);
    crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;

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

    // 若提供了保存后的文本，更新 workspace
    if let Some(text) = params.text {
        state.workspace.update_document(&uri, &text, 0);
    }

    let diags = diagnostics::collect(&uri, &state.workspace);
    crate::server::dispatch::send_diagnostics(&uri, diags, conn)?;

    Ok(())
}

/// 处理 textDocument/didClose
pub fn handle_did_close(
    params: serde_json::Value,
    state: &mut ServerState,
) -> Result<()> {
    let params: DidCloseTextDocumentParams = serde_json::from_value(params)?;
    state.workspace.close_document(&params.text_document.uri);
    Ok(())
}
