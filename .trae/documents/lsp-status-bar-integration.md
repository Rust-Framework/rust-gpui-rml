# LSP 加载状态对接状态栏

## 背景

RA workspace 加载需 30s+，期间前端无任何感知。用户 hover .rml.rs 时只看到 "Loading..." 但不知道还要等多久、是否在加载、是否出错。需要打通 server → client → 前端的状态通知链路，在状态栏显示 RA 加载状态。

## 当前架构

- **Server**：`start_rust_backend`（[dispatch.rs:126](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/dispatch.rs#L126)）在后台线程调用 `host.load()`，完成后 `is_ready()` 返回 true，但**不通知 client**
- **Client**：`LspClient`（[lsp_client.rs:88-125](file:///d:/GitCode/RF/rust-gpui-rml/crates/rml/src/lsp_client.rs#L88-L125)）Reader 线程接收 notification，但**只 log，不分发**
- **前端**：`LspStatusState` Entity（[lsp_status.rs:16](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_status.rs#L16)）已存在，有 `message` 字段和 `set_message` 方法，`LspStatusItem` 在状态栏显示，`MainWindow` observe Entity → 重渲。但只存储"命令摘要"，没有 RA 加载状态

## 方案：自定义 LSP notification `rml/serverStatus`

LSP 标准支持自定义 notification。server 在 `load` 各阶段推送状态，client 解析后分发到前端 Entity。

### 变更 1：Server 端发送状态通知

**文件**：`crates/lsp/src/server/dispatch.rs`

`start_rust_backend` 签名改为接受 `conn_sender: Sender<Message>`：

```rust
fn start_rust_backend(state: &mut ServerState, conn_sender: Sender<Message>) {
    let root_path = ...;
    let host = Arc::clone(&state.ra_host);
    std::thread::spawn(move || {
        send_server_status(&conn_sender, "loading", "Loading rust-analyzer workspace...");
        match host.load(root_path) {
            Ok(()) => send_server_status(&conn_sender, "ready", "rust-analyzer ready"),
            Err(e) => send_server_status(&conn_sender, "error", &format!("{e}")),
        }
    });
}

fn send_server_status(sender: &Sender<Message>, status: &str, message: &str) {
    let params = serde_json::json!({ "status": status, "message": message });
    let not = lsp_server::Notification { method: "rml/serverStatus".into(), params };
    let _ = sender.send(not.into());
}
```

`handle_notification` 的 `"initialized"` 分支调用改为 `start_rust_backend(state, conn.sender.clone())`。

### 变更 2：Client 端接收并分发状态

**文件**：`crates/rml/src/lsp_client.rs`

定义 `ServerStatus` 类型：
```rust
#[derive(Clone, Debug, PartialEq)]
pub enum ServerStatus {
    Loading,
    Ready,
    Error(String),
}
```

`LspClient` 添加 `status_tx: Sender<ServerStatus>` 字段，在 `spawn` 中创建。

Reader 线程的 `Message::Notification` 分支新增 `rml/serverStatus` 处理：
```rust
Message::Notification(not) => {
    if not.method == "rml/serverStatus" {
        if let Some(status) = parse_server_status(&not.params) {
            let _ = status_tx.send(status);
        }
    } else {
        log::debug!("LSP notification: {}", not.method);
    }
}
```

新增 `pub fn status_receiver(&self) -> Receiver<ServerStatus>`。

**文件**：`crates/rml/src/language_client.rs`

暴露 `pub fn status_receiver(&self) -> Receiver<ServerStatus>`（委托 `self.lsp.status_receiver()`）。

### 变更 3：前端订阅状态更新

**文件**：`demo/src/lsp/lsp_status.rs`

`LspStatusState` 添加 `status` 字段：
```rust
pub struct LspStatusState {
    message: Option<String>,  // 保留：命令摘要
    server_status: ServerStatus,  // 新增：RA 加载状态
}

impl LspStatusState {
    pub fn new() -> Self {
        Self { message: None, server_status: ServerStatus::Loading }
    }

    pub fn server_status(&self) -> &ServerStatus { &self.server_status }

    pub fn set_server_status(&mut self, status: ServerStatus, cx: &mut Context<Self>) {
        self.server_status = status;
        cx.notify();
    }
}
```

`LspStatusItem::render` 根据 `server_status` 显示：
- `Loading` → "RA: Loading..."（灰色）
- `Ready` → "RA: Ready"（绿色，或空不显示）
- `Error(msg)` → "RA: Error"（红色，tooltip 显示 msg）

**文件**：`demo/src/shell/main_window.rml.rs`

`init_lsp` 签名改为 `fn init_lsp(&mut self, cx: &mut Context<Self>)`：

```rust
fn init_lsp(&mut self, cx: &mut Context<Self>) {
    if let Ok(workspace_root) = std::env::current_dir() {
        match LanguageClient::unified(&workspace_root) {
            Ok(client) => {
                let rx = client.status_receiver();
                self.language_client = Some(Arc::new(client));
                self.spawn_status_listener(rx, cx);
            }
            Err(e) => log::warn!("Failed to start language server: {e}"),
        }
    }
}

fn spawn_status_listener(&self, rx: Receiver<ServerStatus>, cx: &mut Context<Self>) {
    let Some(lsp_status_ref) = cx.get_service::<LspStatusStateRef>() else { return; };
    let weak = lsp_status_ref.0.clone();
    cx.spawn(|_, mut cx| async move {
        while let Ok(status) = rx.recv() {
            if let Some(entity) = weak.upgrade() {
                let _ = entity.update(&mut cx, |this, cx| {
                    this.set_server_status(status, cx);
                });
            }
        }
    }).detach();
}
```

`on_loaded` 中 `self.init_lsp()` 改为 `self.init_lsp(cx)`。

## 假设与决策

1. **自定义 notification 而非轮询**：LSP 标准支持自定义 notification，状态变化时主动推送，比 client 轮询更高效、更及时。
2. **`ServerStatus` 定义在 client crate**：server 端直接构造 JSON（不依赖类型），client 端解析为 enum。避免 server/client crate 间的类型共享依赖。
3. **保留 `message` 字段**：`LspStatusState.message` 仍存储命令摘要（format/rename 等操作结果），`server_status` 是新增字段，两者共存。
4. **`cx.spawn` 异步轮询**：gpui 的 `cx.spawn` 在 foreground executor 上运行，可通过 `weak.update(&mut cx, ...)` 安全更新 Entity。
5. **状态栏显示策略**：Ready 状态可选择性显示（如只在 hover 时显示），Loading/Error 必须显示以提供感知。

## 验证步骤

1. **编译验证**：
   - `cargo build -p rust-rml-lsp --features rust-backend --lib`
   - `cargo build -p rust-rml-client`
   - `cargo build -p rust-rml-demo`

2. **单元测试**：
   - `cargo test -p rust-rml-lsp --features rust-backend --lib`
   - `cargo test -p rust-rml-client`
   - 为 `parse_server_status` 添加 JSON 解析测试

3. **集成验证（手动）**：
   - 启动 demo lsp 案例
   - 状态栏立即显示 "RA: Loading..."
   - 30s+ 后状态栏变为 "RA: Ready"（或消失）
   - hover .rml.rs 文件 → 应显示 RA quickinfo（而非 "Loading..."）
   - 若 RA 加载失败，状态栏显示 "RA: Error"

4. **回归验证**：
   - 确认命令摘要（format/rename 等）仍正常显示
   - 确认 .rml hover 不受影响
