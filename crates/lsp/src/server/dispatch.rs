//! 请求/通知路由到 handlers
//!
//! 按 LSP method 字符串分派到对应 handler 模块。

use anyhow::Result;
use lsp_server::{Connection, Notification, Request, Response};

use crate::handlers;
use crate::server::connection::ServerState;

/// 处理 LSP 请求（有响应的方法）
pub fn handle_request(
    req: Request,
    state: &mut ServerState,
    conn: &Connection,
) -> Result<()> {
    let result: anyhow::Result<Option<serde_json::Value>> = match req.method.as_str() {
        "textDocument/completion" => {
            handlers::completion::handle_completion(req.params, state)
                .map(|v| v.and_then(|c| serde_json::to_value(c).ok()))
        }
        "textDocument/hover" => {
            handlers::hover::handle_hover(req.params, state)
                .map(|v| v.and_then(|h| serde_json::to_value(h).ok()))
        }
        "textDocument/definition" => {
            handlers::definition::handle_definition(req.params, state)
                .map(|v| v.and_then(|d| serde_json::to_value(d).ok()))
        }
        "textDocument/documentSymbol" => {
            handlers::document_symbol::handle_document_symbol(req.params, state)
                .map(|v| v.and_then(|s| serde_json::to_value(s).ok()))
        }
        "textDocument/foldingRange" => {
            handlers::folding_range::handle_folding_range(req.params, state)
                .map(|v| v.and_then(|f| serde_json::to_value(f).ok()))
        }
        "textDocument/references" => {
            handlers::references::handle_references(req.params, state)
                .map(|v| v.and_then(|r| serde_json::to_value(r).ok()))
        }
        "textDocument/formatting" => {
            handlers::formatting::handle_formatting(req.params, state)
                .map(|v| v.and_then(|e| serde_json::to_value(e).ok()))
        }
        "textDocument/signatureHelp" => {
            handlers::signature_help::handle_signature_help(req.params, state)
                .map(|v| v.and_then(|s| serde_json::to_value(s).ok()))
        }
        "textDocument/rename" => {
            handlers::rename::handle_rename(req.params, state)
                .map(|v| v.and_then(|e| serde_json::to_value(e).ok()))
        }
        "textDocument/semanticTokens/full" => {
            handlers::semantic_tokens::handle_full(req.params, state)
                .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
        }
        "textDocument/semanticTokens/range" => {
            handlers::semantic_tokens::handle_range(req.params, state)
                .map(|v| v.and_then(|t| serde_json::to_value(t).ok()))
        }
        _ => {
            log::debug!("unhandled request: {}", req.method);
            Ok(None)
        }
    };

    let response = match result {
        Ok(Some(value)) => Response {
            id: req.id,
            result: Some(value),
            error: None,
        },
        Ok(None) => Response {
            id: req.id,
            result: None,
            error: None,
        },
        Err(e) => Response {
            id: req.id,
            result: None,
            error: Some(lsp_server::ResponseError {
                code: 0,
                message: e.to_string(),
                data: None,
            }),
        },
    };
    conn.sender.send(response.into())?;
    Ok(())
}

/// 处理 LSP 通知（无响应的方法）
pub fn handle_notification(
    not: Notification,
    state: &mut ServerState,
    conn: &Connection,
) -> Result<()> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            handlers::sync::handle_did_open(not.params, state, conn)?;
        }
        "textDocument/didChange" => {
            handlers::sync::handle_did_change(not.params, state, conn)?;
        }
        "textDocument/didSave" => {
            handlers::sync::handle_did_save(not.params, state, conn)?;
        }
        "textDocument/didClose" => {
            handlers::sync::handle_did_close(not.params, state)?;
        }
        "initialized" => {
            log::debug!("client initialized");
            start_rust_backend(state);
            scan_workspace_assets(state);
        }
        _ => {
            log::debug!("unhandled notification: {}", not.method);
        }
    }
    Ok(())
}

/// 在后台线程加载 rust-analyzer workspace（不阻塞主循环）
#[cfg(feature = "rust-backend")]
fn start_rust_backend(state: &mut ServerState) {
    let root_path = match state.root_path.clone() {
        Some(p) => p,
        None => {
            log::warn!("root_path unavailable, rust-analyzer backend not started");
            return;
        }
    };
    let host = std::sync::Arc::clone(&state.ra_host);
    std::thread::spawn(move || {
        log::info!("loading rust-analyzer workspace at {:?}", root_path);
        match host.load(root_path) {
            Ok(()) => log::info!("rust-analyzer workspace loaded"),
            Err(e) => log::error!("rust-analyzer workspace load failed: {}", e),
        }
    });
}

#[cfg(not(feature = "rust-backend"))]
fn start_rust_backend(_state: &mut ServerState) {}

/// 扫描 workspace 下的 i18n JSON 与 CSS 文件,构建资源索引
fn scan_workspace_assets(state: &mut ServerState) {
    if let Some(root) = state.root_path.clone() {
        log::info!("scanning workspace assets at {:?}", root);
        state.i18n_index.scan(&root);
        state.css_index.scan(&root);
        log::info!(
            "asset scan done: {} i18n keys, {} css classes",
            state.i18n_index.len(),
            state.css_index.len()
        );
    } else {
        log::warn!("root_path unavailable, workspace assets not scanned");
    }
}

/// 发送诊断通知
pub fn send_diagnostics(
    uri: &lsp_types::Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
    conn: &Connection,
) -> Result<()> {
    let params = lsp_types::PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let not = lsp_server::Notification {
        method: "textDocument/publishDiagnostics".into(),
        params: serde_json::to_value(params)?,
    };
    conn.sender.send(not.into())?;
    Ok(())
}
