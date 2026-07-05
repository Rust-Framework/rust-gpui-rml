//! LSP 端到端测试：验证 JSON-RPC → dispatch → handler → features → 响应 完整链路
//!
//! 这是唯一能证明"LSP 协议层真的能工作"的测试类型。单元测试只验证各 features 模块
//! 的纯函数逻辑，无法捕获 dispatch 路由错误、handler 参数反序列化错误、capability
//! 声明遗漏等问题。

use crossbeam_channel::{unbounded, Receiver, Sender};
use lsp_server::{Message, Notification, Request, Response};
use lsp_types::Url;
use rml_lsp::server::connection::ServerState;
use rml_lsp::server::dispatch::{handle_notification, handle_request};

/// 构造测试用 Connection（只填充 sender，receiver 用 dummy）
fn make_conn() -> (lsp_server::Connection, Receiver<Message>) {
    let (tx, rx): (Sender<Message>, Receiver<Message>) = unbounded();
    let (_, dummy_rx) = unbounded::<Message>();
    let conn = lsp_server::Connection {
        sender: tx,
        receiver: dummy_rx,
    };
    (conn, rx)
}

/// 构造带文档的 ServerState
fn make_state(uri: &Url, source: &str) -> ServerState {
    let mut state = ServerState::new();
    state.workspace.open_document(uri.clone(), source, 1);
    state
}

/// 发送请求并接收响应
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

fn rml_uri() -> Url {
    Url::parse("file:///test.rml").unwrap()
}

fn make_position(line: u32, character: u32) -> serde_json::Value {
    serde_json::json!({ "line": line, "character": character })
}

fn text_document_params(uri: &Url) -> serde_json::Value {
    serde_json::json!({ "uri": uri.as_str() })
}

// ============================================================
// 6 个 LSP 路由的端到端测试
// ============================================================

#[test]
fn e2e_definition_returns_none_for_unknown_tag() {
    // definition 路由：对未知标签发起 definition 请求，应返回 None（无跳转目标）
    let uri = rml_uri();
    let source = "<component><div></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "position": make_position(0, 11),
    });
    let resp = send_request("textDocument/definition", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    // definition 对未识别符号返回 None（result 字段为 None）
    assert!(resp.result.is_none(), "expected None result for unknown tag, got {:?}", resp.result);
}

#[test]
fn e2e_references_returns_locations_for_tag() {
    // references 路由：对标签发起引用查找，应返回 Location 数组
    let uri = rml_uri();
    let source = "<component><div></div><div></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "position": make_position(0, 12),
        "context": { "includeDeclaration": false },
    });
    let resp = send_request("textDocument/references", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("references should be array");
    assert!(!arr.is_empty(), "should find tag references");
}

#[test]
fn e2e_document_symbol_returns_nested_tree() {
    // documentSymbol 路由：应返回嵌套的 DocumentSymbol 树
    let uri = rml_uri();
    let source = "<component><div><span></span></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
    });
    let resp = send_request(
        "textDocument/documentSymbol",
        params,
        &mut state,
        &conn,
        &rx,
    );
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("documentSymbol should be array");
    assert_eq!(arr.len(), 1, "should have one root symbol");
    let root = &arr[0];
    assert_eq!(root["name"], "component");
    let children = root["children"].as_array().expect("root should have children");
    assert_eq!(children.len(), 1, "root should have one child");
    assert_eq!(children[0]["name"], "div");
}

#[test]
fn e2e_formatting_returns_text_edits() {
    // formatting 路由：应返回 TextEdit 数组
    let uri = rml_uri();
    let source = "<component><div class=\"x\"></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "options": { "tabSize": 2, "insertSpaces": true },
    });
    let resp = send_request("textDocument/formatting", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    let result = resp.result.expect("result should exist");
    let arr = result.as_array().expect("formatting should be array");
    assert_eq!(arr.len(), 1, "should return single full-document edit");
    let new_text = arr[0]["newText"].as_str().expect("newText should be string");
    assert!(new_text.contains("<component>"), "newText should contain component tag");
}

#[test]
fn e2e_signature_help_returns_none_without_command_context() {
    // signatureHelp 路由：无 command 上下文时应返回 None（不 panic）
    let uri = rml_uri();
    let source = "<component><button onclick={on_click}></button></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "position": make_position(0, 30),
    });
    let resp = send_request("textDocument/signatureHelp", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    // 无 Rust codebehind metadata 时返回 None
    assert!(resp.result.is_none(), "expected None result without command context, got {:?}", resp.result);
}

#[test]
fn e2e_rename_returns_workspace_edit_for_tag() {
    // rename 路由：对标签发起 rename，应返回 WorkspaceEdit
    let uri = rml_uri();
    let source = "<component><div></div><div></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "position": make_position(0, 12),
        "newName": "section",
    });
    let resp = send_request("textDocument/rename", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    let result = resp.result.expect("result should exist");
    let changes = result["changes"]
        .as_object()
        .expect("WorkspaceEdit should have changes object");
    assert!(!changes.is_empty(), "should have changes");
    let rml_edits = changes
        .get(uri.as_str())
        .expect("should have edits for rml uri")
        .as_array()
        .unwrap();
    assert_eq!(rml_edits.len(), 2, "should rename both div tags");
    assert!(
        rml_edits
            .iter()
            .all(|e| e["newText"] == "section"),
        "all edits should use newName"
    );
}

// ============================================================
// 通知路径测试（did_open）
// ============================================================

#[test]
fn e2e_did_open_notification_opens_document() {
    // did_open 通知：应将文档加入 workspace，后续请求能查到
    let uri = rml_uri();
    let source = "<component><div></div></component>";
    let mut state = ServerState::new();
    let (conn, _rx) = make_conn();

    let not = Notification {
        method: "textDocument/didOpen".into(),
        params: serde_json::json!({
            "textDocument": {
                "uri": uri.as_str(),
                "languageId": "rml",
                "version": 1,
                "text": source,
            }
        }),
    };
    handle_notification(not, &mut state, &conn).expect("handle_notification should not error");

    // 验证文档已打开
    let doc = state.workspace.document(&uri).expect("document should be open");
    assert_eq!(doc.tree.text(), source);
}

#[test]
fn e2e_unknown_method_returns_null_result() {
    // 未知方法应返回 None（result 为 null），不 panic
    let uri = rml_uri();
    let mut state = make_state(&uri, "<component/>");
    let (conn, rx) = make_conn();

    let resp = send_request("textDocument/unknownMethod", serde_json::json!({}), &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "unknown method should not error");
    assert!(resp.result.is_none(), "unknown method should return null result");
}

#[test]
fn e2e_rename_invalid_ident_returns_null_result() {
    // rename 路由：非法新名应返回 None（result 为 null）
    let uri = rml_uri();
    let source = "<component><div></div></component>";
    let mut state = make_state(&uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&uri),
        "position": make_position(0, 12),
        "newName": "1invalid",
    });
    let resp = send_request("textDocument/rename", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "invalid ident should not error, should return null");
    assert!(resp.result.is_none(), "invalid ident should return None result, got {:?}", resp.result);
}

#[test]
fn e2e_references_on_rust_codebehind_returns_null() {
    // .rml.rs 文件 references 应返回 None（rust-analyzer 处理）
    let rs_uri = Url::parse("file:///test.rml.rs").unwrap();
    let source = "struct Foo;";
    let mut state = make_state(&rs_uri, source);
    let (conn, rx) = make_conn();

    let params = serde_json::json!({
        "textDocument": text_document_params(&rs_uri),
        "position": make_position(0, 7),
        "context": { "includeDeclaration": true },
    });
    let resp = send_request("textDocument/references", params, &mut state, &conn, &rx);
    assert!(resp.error.is_none(), "codebehind references should not error");
    assert!(resp.result.is_none(), "codebehind references should return None, got {:?}", resp.result);
}
