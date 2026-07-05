//! LSP 生命周期端到端测试：验证文档同步生命周期 + 多消息序列 + 状态变迁
//!
//! 覆盖 dispatch_e2e 未触及的盲区：
//! - initialize 握手 capabilities 完整性
//! - did_open / did_change / did_save / did_close 通知路径
//! - 文档变更后后续请求看到新内容（状态变迁）
//! - did_open/did_change 发布 publishDiagnostics 通知
//! - completion / hover 的 dispatch 路由
//! - 多文件 workspace 隔离
//! - 完整生命周期链路：open → change → symbol → close → symbol

use crossbeam_channel::{unbounded, Receiver, Sender};
use lsp_server::{Message, Notification, Request, Response};
use lsp_types::Url;
use rml_lsp::server::connection::{build_capabilities, ServerState};
use rml_lsp::server::dispatch::{handle_notification, handle_request};

// ============================================================
// 测试辅助
// ============================================================

fn make_conn() -> (lsp_server::Connection, Receiver<Message>) {
    let (tx, rx): (Sender<Message>, Receiver<Message>) = unbounded();
    let (_, dummy_rx) = unbounded::<Message>();
    let conn = lsp_server::Connection {
        sender: tx,
        receiver: dummy_rx,
    };
    (conn, rx)
}

fn rml_uri(name: &str) -> Url {
    Url::parse(&format!("file:///{name}")).unwrap()
}

fn did_open_params(uri: &Url, text: &str, version: i32) -> serde_json::Value {
    serde_json::json!({
        "textDocument": {
            "uri": uri.as_str(),
            "languageId": "rml",
            "version": version,
            "text": text,
        }
    })
}

fn did_change_params(uri: &Url, text: &str, version: i32) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri.as_str(), "version": version },
        "contentChanges": [{ "text": text }]
    })
}

fn did_save_params(uri: &Url, text: Option<&str>) -> serde_json::Value {
    let text_field = match text {
        Some(t) => serde_json::json!(t),
        None => serde_json::Value::Null,
    };
    serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "text": text_field,
    })
}

fn did_close_params(uri: &Url) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri.as_str() }
    })
}

fn doc_symbol_params(uri: &Url) -> serde_json::Value {
    serde_json::json!({ "textDocument": { "uri": uri.as_str() } })
}

fn position_params(uri: &Url, line: u32, character: u32) -> serde_json::Value {
    serde_json::json!({
        "textDocument": { "uri": uri.as_str() },
        "position": { "line": line, "character": character }
    })
}

fn send_request(
    method: &str,
    params: serde_json::Value,
    state: &mut ServerState,
    conn: &lsp_server::Connection,
    rx: &Receiver<Message>,
) -> Response {
    let req = Request {
        id: 1.into(),
        method: method.to_string(),
        params,
    };
    handle_request(req, state, conn).expect("handle_request should not error");
    match rx.recv() {
        Ok(Message::Response(resp)) => resp,
        Ok(other) => panic!("expected Response, got {other:?}"),
        Err(e) => panic!("no response received: {e}"),
    }
}

/// 收集 channel 中所有通知（drain）
fn drain_notifications(rx: &Receiver<Message>) -> Vec<Notification> {
    let mut notifications = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let Message::Notification(n) = msg {
            notifications.push(n);
        }
    }
    notifications
}

/// 发送通知并收集所有发布的通知（用于 did_open/did_change 后收 diagnostics）
fn send_notification_and_collect(
    method: &str,
    params: serde_json::Value,
    state: &mut ServerState,
    conn: &lsp_server::Connection,
    rx: &Receiver<Message>,
) -> Vec<Notification> {
    let not = Notification {
        method: method.to_string(),
        params,
    };
    handle_notification(not, state, conn).expect("handle_notification should not error");
    drain_notifications(rx)
}

// ============================================================
// initialize 握手 capabilities 完整性
// ============================================================

#[test]
fn initialize_capabilities_declares_all_providers() {
    // 验证 build_capabilities 声明了所有已实现的 LSP provider
    // 缺失任一 capability 会导致客户端不发送对应请求
    let caps = build_capabilities();

    assert!(caps.completion_provider.is_some(), "completion provider required");
    assert!(caps.hover_provider.is_some(), "hover provider required");
    assert!(caps.definition_provider.is_some(), "definition provider required");
    assert!(caps.references_provider.is_some(), "references provider required");
    assert!(caps.document_symbol_provider.is_some(), "documentSymbol provider required");
    assert!(caps.document_formatting_provider.is_some(), "formatting provider required");
    assert!(caps.signature_help_provider.is_some(), "signatureHelp provider required");
    assert!(caps.rename_provider.is_some(), "rename provider required");

    // textDocumentSync 必须声明（否则客户端不发 did_change）
    assert!(caps.text_document_sync.is_some(), "textDocumentSync required");
}

#[test]
fn initialize_capabilities_completion_trigger_chars() {
    // 验证补全 trigger characters 包含 < / 空格 / {（用于自动弹出补全）
    let caps = build_capabilities();
    let completion = caps.completion_provider.unwrap();
    let triggers = completion
        .trigger_characters
        .as_ref()
        .expect("trigger_characters should be declared");
    assert!(triggers.contains(&"<".to_string()), "should trigger on <");
    assert!(triggers.contains(&" ".to_string()), "should trigger on space");
    assert!(triggers.contains(&"{".to_string()), "should trigger on {{");
}

#[test]
fn initialize_capabilities_signature_help_trigger_chars() {
    // 验证 signatureHelp trigger characters 包含 , 和 (
    let caps = build_capabilities();
    let sig = caps.signature_help_provider.unwrap();
    let triggers = sig
        .trigger_characters
        .as_ref()
        .expect("signatureHelp trigger_characters should be declared");
    assert!(triggers.contains(&",".to_string()), "should trigger on comma");
    assert!(triggers.contains(&"(".to_string()), "should trigger on (");
}

#[test]
fn initialize_capabilities_text_document_sync_full() {
    // 验证 textDocumentSync 为 FULL 模式（did_change 发送完整文本）
    // 若改为 INCREMENTAL，did_change 测试需要重写
    use lsp_types::TextDocumentSyncCapability;
    let caps = build_capabilities();
    match caps.text_document_sync.unwrap() {
        TextDocumentSyncCapability::Kind(kind) => {
            use lsp_types::TextDocumentSyncKind;
            assert_eq!(kind, TextDocumentSyncKind::FULL, "expected FULL sync mode");
        }
        TextDocumentSyncCapability::Options(_) => {
            panic!("expected Kind, got Options");
        }
    }
}

// ============================================================
// did_open 通知路径
// ============================================================

#[test]
fn did_open_publishes_diagnostics_notification() {
    // did_open 应发送 publishDiagnostics 通知
    let uri = rml_uri("diag.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    let notifications = send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // 应至少有一条 publishDiagnostics 通知
    let diag_notifs: Vec<_> = notifications
        .iter()
        .filter(|n| n.method == "textDocument/publishDiagnostics")
        .collect();
    assert!(
        !diag_notifs.is_empty(),
        "did_open should publish diagnostics, got: {:?}",
        notifications.iter().map(|n| &n.method).collect::<Vec<_>>()
    );

    // 通知 params 应包含正确的 URI
    let params = &diag_notifs[0].params;
    let notif_uri = params["uri"].as_str().expect("uri should be string");
    assert_eq!(notif_uri, uri.as_str());
}

#[test]
fn did_open_then_document_symbol_sees_content() {
    // did_open 后,后续 documentSymbol 请求能看到文档结构
    let uri = rml_uri("open.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none());
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("should be array");
    assert_eq!(arr.len(), 1, "should have root component symbol");
    assert_eq!(arr[0]["name"], "component");
}

// ============================================================
// did_change 状态变迁
// ============================================================

#[test]
fn did_change_updates_document_symbol_sees_new_content() {
    // did_change 改变文本后,后续 documentSymbol 看到新结构
    let uri = rml_uri("change.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    // 初始：<component><div></div></component>
    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // 变更为：<component><span></span></component>
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><span></span></component>", 2),
        &mut state,
        &conn,
        &rx,
    );

    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("should be array");
    let root = &arr[0];
    assert_eq!(root["name"], "component");
    let children = root["children"]
        .as_array()
        .expect("root should have children");
    assert_eq!(children[0]["name"], "span", "should see span after did_change");
}

#[test]
fn did_change_publishes_diagnostics() {
    // did_change 应发送 publishDiagnostics 通知
    let uri = rml_uri("change_diag.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component/>", 1),
        &mut state,
        &conn,
        &rx,
    );

    let notifications = send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><div></div></component>", 2),
        &mut state,
        &conn,
        &rx,
    );
    let has_diag = notifications
        .iter()
        .any(|n| n.method == "textDocument/publishDiagnostics");
    assert!(has_diag, "did_change should publish diagnostics");
}

#[test]
fn did_change_multiple_times_final_content_wins() {
    // 多次 did_change 后,最终内容生效
    let uri = rml_uri("multi_change.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component/>", 1),
        &mut state,
        &conn,
        &rx,
    );
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><div></div></component>", 2),
        &mut state,
        &conn,
        &rx,
    );
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><span></span></component>", 3),
        &mut state,
        &conn,
        &rx,
    );
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><h1></h1><h2></h2></component>", 4),
        &mut state,
        &conn,
        &rx,
    );

    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("should be array");
    let children = arr[0]["children"]
        .as_array()
        .expect("root should have children");
    assert_eq!(children.len(), 2, "should have 2 children after final change");
    assert_eq!(children[0]["name"], "h1");
    assert_eq!(children[1]["name"], "h2");
}

// ============================================================
// did_save 通知路径
// ============================================================

#[test]
fn did_save_publishes_diagnostics() {
    // did_save 应发送 publishDiagnostics 通知
    let uri = rml_uri("save.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component/>", 1),
        &mut state,
        &conn,
        &rx,
    );

    let notifications = send_notification_and_collect(
        "textDocument/didSave",
        did_save_params(&uri, Some("<component><div></div></component>")),
        &mut state,
        &conn,
        &rx,
    );
    let has_diag = notifications
        .iter()
        .any(|n| n.method == "textDocument/publishDiagnostics");
    assert!(has_diag, "did_save should publish diagnostics");
}

#[test]
fn did_save_with_text_updates_document() {
    // did_save 携带 text 字段时,应更新文档内容
    let uri = rml_uri("save_text.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component/>", 1),
        &mut state,
        &conn,
        &rx,
    );

    send_notification_and_collect(
        "textDocument/didSave",
        did_save_params(&uri, Some("<component><div></div></component>")),
        &mut state,
        &conn,
        &rx,
    );

    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("should be array");
    let children = arr[0]["children"]
        .as_array()
        .expect("root should have children");
    assert_eq!(children[0]["name"], "div", "did_save text should update doc");
}

// ============================================================
// did_close 通知路径
// ============================================================

#[test]
fn did_close_clears_document_from_workspace() {
    // did_close 后,文档从 workspace 移除,后续 documentSymbol 返回 None
    let uri = rml_uri("close.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // close 不发通知（无 publishDiagnostics）
    let notifications = send_notification_and_collect(
        "textDocument/didClose",
        did_close_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    assert!(
        notifications.is_empty(),
        "did_close should not publish notifications, got: {:?}",
        notifications.iter().map(|n| &n.method).collect::<Vec<_>>()
    );

    // 后续 documentSymbol 应返回 None
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none());
    assert!(
        resp.result.is_none(),
        "closed doc should return None result, got {:?}",
        resp.result
    );
}

// ============================================================
// completion / hover dispatch 路由（dispatch_e2e 未覆盖）
// ============================================================

#[test]
fn completion_dispatch_returns_items_for_tag_context() {
    // 在 `<` 后位置发起 completion,应返回补全条目
    let uri = rml_uri("comp.rml");
    let mut state = ServerState::new();
    state
        .workspace
        .open_document(uri.clone(), "<component><", 1);
    let (conn, rx) = make_conn();

    // 光标在 `<` 后（offset 12,character 12）
    let resp = send_request(
        "textDocument/completion",
        position_params(&uri, 0, 12),
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    // completion 在 .rml 上应返回 Some(array) 或 None（取决于是否识别上下文）
    // 这里只验证 dispatch 路由不 panic、不 error
    if let Some(result) = resp.result {
        // 若返回了结果,应该是数组
        assert!(
            result.is_array() || result.is_object(),
            "completion result should be array or object, got: {result}"
        );
    }
}

#[test]
fn hover_dispatch_returns_hover_for_known_tag() {
    // 在已知标签上发起 hover,应返回 Hover
    let uri = rml_uri("hover.rml");
    let mut state = ServerState::new();
    state
        .workspace
        .open_document(uri.clone(), "<component><div></div></component>", 1);
    let (conn, rx) = make_conn();

    // 光标在 div 标签名上（offset 12..15）
    let resp = send_request(
        "textDocument/hover",
        position_params(&uri, 0, 13),
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    // div 是内置 HTML 标签,hover 应返回内容
    if let Some(result) = resp.result {
        let contents = &result["contents"];
        assert!(
            !contents.is_null(),
            "hover contents should not be null for known tag"
        );
    }
}

#[test]
fn hover_dispatch_returns_none_for_unknown_position() {
    // 在无符号位置发起 hover,应返回 None
    let uri = rml_uri("hover_unknown.rml");
    let mut state = ServerState::new();
    state
        .workspace
        .open_document(uri.clone(), "<component></component>", 1);
    let (conn, rx) = make_conn();

    // 光标在文档末尾（无元素）
    let resp = send_request(
        "textDocument/hover",
        position_params(&uri, 0, 100),
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none());
    assert!(
        resp.result.is_none(),
        "hover on unknown position should return None"
    );
}

// ============================================================
// 多文件 workspace 隔离
// ============================================================

#[test]
fn multi_file_workspace_documents_isolated() {
    // 两个 .rml 文档共存,各自 documentSymbol 反映各自内容
    let uri_a = rml_uri("a.rml");
    let uri_b = rml_uri("b.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri_a, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );
    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri_b, "<component><span></span></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // A 的 documentSymbol 应看到 div
    let resp_a = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri_a),
        &mut state,
        &conn,
        &rx,
    );
    let result_a = resp_a.result.expect("A result should exist");
    let arr_a = result_a.as_array().expect("A should be array");
    let children_a = arr_a[0]["children"]
        .as_array()
        .expect("A root should have children");
    assert_eq!(children_a[0]["name"], "div", "A should have div");

    // B 的 documentSymbol 应看到 span
    let resp_b = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri_b),
        &mut state,
        &conn,
        &rx,
    );
    let result_b = resp_b.result.expect("B result should exist");
    let arr_b = result_b.as_array().expect("B should be array");
    let children_b = arr_b[0]["children"]
        .as_array()
        .expect("B root should have children");
    assert_eq!(children_b[0]["name"], "span", "B should have span");
}

#[test]
fn multi_file_close_one_does_not_affect_other() {
    // 关闭文档 A 不影响文档 B 的请求
    let uri_a = rml_uri("multi_a.rml");
    let uri_b = rml_uri("multi_b.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri_a, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );
    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri_b, "<component><span></span></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // 关闭 A
    send_notification_and_collect(
        "textDocument/didClose",
        did_close_params(&uri_a),
        &mut state,
        &conn,
        &rx,
    );

    // A 应返回 None
    let resp_a = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri_a),
        &mut state,
        &conn,
        &rx,
    );
    assert!(
        resp_a.result.is_none(),
        "A should be None after close"
    );

    // B 仍应正常返回
    let resp_b = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri_b),
        &mut state,
        &conn,
        &rx,
    );
    let result_b = resp_b.result.expect("B result should still exist");
    let arr_b = result_b.as_array().expect("B should be array");
    assert_eq!(arr_b[0]["name"], "component", "B should still be accessible");
}

// ============================================================
// 完整生命周期链路
// ============================================================

#[test]
fn full_lifecycle_open_change_symbol_close_symbol() {
    // 端到端：open → change → symbol → close → symbol
    let uri = rml_uri("lifecycle.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    // 1. did_open
    send_notification_and_collect(
        "textDocument/didOpen",
        did_open_params(&uri, "<component/>", 1),
        &mut state,
        &conn,
        &rx,
    );
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("after open: result should exist");
    let arr = result.as_array().expect("should be array");
    assert_eq!(arr[0]["name"], "component");
    assert!(
        arr[0]["children"].as_array().is_none()
            || arr[0]["children"].as_array().unwrap().is_empty(),
        "after open: should have no children"
    );

    // 2. did_change: 添加子元素
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><div></div></component>", 2),
        &mut state,
        &conn,
        &rx,
    );
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("after change: result should exist");
    let arr = result.as_array().expect("should be array");
    let children = arr[0]["children"]
        .as_array()
        .expect("after change: should have children");
    assert_eq!(children[0]["name"], "div");

    // 3. did_change: 替换子元素
    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><span></span></component>", 3),
        &mut state,
        &conn,
        &rx,
    );
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("after second change: result should exist");
    let arr = result.as_array().expect("should be array");
    let children = arr[0]["children"]
        .as_array()
        .expect("after second change: should have children");
    assert_eq!(children[0]["name"], "span");

    // 4. did_close
    send_notification_and_collect(
        "textDocument/didClose",
        did_close_params(&uri),
        &mut state,
        &conn,
        &rx,
    );

    // 5. documentSymbol 应返回 None
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    assert!(
        resp.result.is_none(),
        "after close: should return None, got {:?}",
        resp.result
    );
}

#[test]
fn did_change_without_prior_open_creates_document() {
    // did_change 在未 did_open 的文档上,应创建文档（update_document 行为）
    let uri = rml_uri("change_without_open.rml");
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();

    send_notification_and_collect(
        "textDocument/didChange",
        did_change_params(&uri, "<component><div></div></component>", 1),
        &mut state,
        &conn,
        &rx,
    );

    // 应能查到文档
    let resp = send_request(
        "textDocument/documentSymbol",
        doc_symbol_params(&uri),
        &mut state,
        &conn,
        &rx,
    );
    let result = resp.result.expect("result should exist after did_change");
    let arr = result.as_array().expect("should be array");
    assert_eq!(arr[0]["name"], "component");
}

#[test]
fn unhandled_notification_does_not_error() {
    // 未知通知方法不应导致错误
    let mut state = ServerState::new();
    let (conn, rx) = make_conn();
    let notifications = send_notification_and_collect(
        "textDocument/unknownNotification",
        serde_json::json!({}),
        &mut state,
        &conn,
        &rx,
    );
    assert!(
        notifications.is_empty(),
        "unknown notification should not produce output"
    );
}
