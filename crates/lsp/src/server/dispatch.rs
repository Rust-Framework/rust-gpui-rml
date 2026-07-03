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
        }
        _ => {
            log::debug!("unhandled notification: {}", not.method);
        }
    }
    Ok(())
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
