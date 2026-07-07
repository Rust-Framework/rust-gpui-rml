# LSP 状态栏前端对接实施计划

## 背景

RA workspace 加载需 30s+，前端无感知。已实现自定义 LSP notification `rml/serverStatus` 的完整链路（Server 端发送 → Client 端接收分发），但前端尚未订阅状态更新，状态栏无法显示 RA 加载进度。

## 当前状态分析

### 已完成（Tasks 6, 7）

- **Server 端**（[dispatch.rs:127-163](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/dispatch.rs#L127-L163)）：`start_rust_backend` 在后台线程加载 RA workspace，通过 `send_server_status` 发送 `rml/serverStatus` 通知（loading/ready/error 三态）
- **Client 端**（[lsp_client.rs:26-35, 86, 126-133, 339-341](file:///d:/GitCode/RF/rust-gpui-rml/crates/rml/src/lsp_client.rs#L26-L35)）：`ServerStatus` enum 定义、`status_rx` 字段、Reader 线程解析通知、`status_receiver()` 方法、`parse_server_status()` 函数 + 5 个单元测试
- **LanguageClient**（[language_client.rs:117-122](file:///d:/GitCode/RF/rust-gpui-rml/crates/rml/src/language_client.rs#L117-L122)）：`status_receiver()` 委托方法

### 待完成（Task 8 + Task 9）

- **前端 `LspStatusState`**（[lsp_status.rs:16-33](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_status.rs#L16-L33)）：仅有 `message` 字段（命令摘要），无 `server_status` 字段
- **`LspStatusItem::render`**（[lsp_status.rs:59-70](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_status.rs#L59-L70)）：仅显示 `message`，不显示 RA 加载状态
- **`MainWindow::init_lsp`**（[main_window.rml.rs:139-146](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L139-L146)）：无 `cx` 参数，不获取 `status_receiver`，不 spawn 状态监听任务

## 变更方案

### 变更 1：re-export `ServerStatus` 类型

**文件**：`crates/rml/src/lib.rs`（[第 33 行](file:///d:/GitCode/RF/rust-gpui-rml/crates/rml/src/lib.rs#L33)）

当前 `pub use lsp_client::{file_path_to_uri, LspClient};` 未导出 `ServerStatus`，demo 侧需通过 `rust_rml_client::lsp_client::ServerStatus` 全路径访问。为简化导入，将 `ServerStatus` 加入 re-export：

```rust
pub use lsp_client::{file_path_to_uri, LspClient, ServerStatus};
```

### 变更 2：`LspStatusState` 添加 `server_status` 字段 + `LspStatusItem::render` 分态显示

**文件**：`demo/src/lsp/lsp_status.rs`

**2a. 导入 `ServerStatus`**

在 `use` 区添加：
```rust
use rust_rml_client::ServerStatus;
```

**2b. `LspStatusState` 结构体新增字段**

```rust
pub struct LspStatusState {
    message: Option<String>,       // 保留：命令摘要（format/rename 等结果）
    server_status: ServerStatus,   // 新增：RA 加载状态
}
```

**2c. `LspStatusState` impl 新增方法**

```rust
impl LspStatusState {
    pub fn new() -> Self {
        Self { message: None, server_status: ServerStatus::Loading }
    }

    // 保留 message() / set_message() 不变

    pub fn server_status(&self) -> &ServerStatus {
        &self.server_status
    }

    pub fn set_server_status(&mut self, status: ServerStatus, cx: &mut Context<Self>) {
        self.server_status = status;
        cx.notify();
    }
}
```

**2d. `LspStatusItem::render` 优先显示 server_status，回退显示 message**

渲染逻辑：
- `ServerStatus::Loading` → "RA: Loading..."，文字色 `cx.theme().muted_foreground`（灰）
- `ServerStatus::Ready` → "RA: Ready"，文字色 `cx.theme().success`（绿）
- `ServerStatus::Error(msg)` → "RA: Error"，文字色 `cx.theme().danger_foreground`（红），tooltip 显示 msg
- `ServerStatus::Ready` 且有 `message` → 显示 `message`（命令摘要覆盖，Ready 状态下用户更关心操作结果）

```rust
impl IVisual for LspStatusItem {
    fn render(&self, _window: &mut Window, cx: &mut App) -> AnyElement {
        let state = cx
            .get_service::<LspStatusStateRef>()
            .and_then(|r| r.0.upgrade());

        let Some(entity) = state else {
            return gpui::div().into_any_element();
        };
        let state = entity.read(cx);

        // Ready 且有命令摘要时，优先显示摘要（操作结果比 RA 状态更有价值）
        if matches!(state.server_status(), ServerStatus::Ready) {
            if let Some(msg) = state.message() {
                return gpui::div().text_xs().child(msg.to_string()).into_any_element();
            }
        }

        match state.server_status() {
            ServerStatus::Loading => gpui::div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("RA: Loading...")
                .into_any_element(),
            ServerStatus::Ready => gpui::div()
                .text_xs()
                .text_color(cx.theme().success)
                .child("RA: Ready")
                .into_any_element(),
            ServerStatus::Error(msg) => gpui::div()
                .text_xs()
                .text_color(cx.theme().danger_foreground)
                .child("RA: Error")
                .tooltip(move |window, cx| {
                    gpui_component::tooltip::Tooltip::new(msg.clone(), window, cx)
                })
                .into_any_element(),
        }
    }
}
```

**说明**：
- 使用 `gpui_component::tooltip::Tooltip` 显示错误详情（需确认 API，若不存在则用 `gpui::div().child(msg)` 简化）
- `Loading` 状态始终显示（提供加载感知）；`Ready` 且有命令摘要时切换为摘要显示
- `cx.theme()` 通过 `gpui_component::ActiveTheme` trait 提供，需 `use gpui_component::ActiveTheme as _`

### 变更 3：`MainWindow::init_lsp` 订阅状态更新

**文件**：`demo/src/shell/main_window.rml.rs`

**3a. 导入 `ServerStatus` 和 `Receiver`**

```rust
use crossbeam_channel::Receiver;
use rust_rml_client::ServerStatus;
```

**3b. `init_lsp` 签名改为接受 `cx`，获取 `status_receiver`，spawn 监听任务**

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
```

**3c. 新增 `spawn_status_listener` 方法**

关键设计：crossbeam `Receiver::recv()` 是阻塞调用，不能在 `cx.spawn`（foreground executor，UI 线程）中直接调用，否则冻结 UI。采用 `cx.spawn` + `cx.background_executor().spawn()` 嵌套模式：foreground 循环 await background 的单次 recv 结果，既不阻塞 UI 又能持续轮询。

```rust
fn spawn_status_listener(&self, rx: Receiver<ServerStatus>, cx: &mut Context<Self>) {
    let Some(lsp_status_ref) = cx.get_service::<LspStatusStateRef>() else {
        return;
    };
    let weak = lsp_status_ref.0.clone();

    cx.spawn(move |_this, cx: &mut gpui::AsyncApp| {
        let mut cx = cx.clone();
        async move {
            loop {
                let rx = rx.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { rx.recv() })
                    .await;
                match result {
                    Ok(status) => {
                        if let Some(entity) = weak.upgrade() {
                            let _ = entity.update(&mut cx, |this, cx| {
                                this.set_server_status(status, cx);
                            });
                        }
                    }
                    Err(_) => break, // channel closed (server 进程退出)
                }
            }
        }
    })
    .detach();
}
```

**3d. `on_loaded` 中调用改为传 `cx`**

[第 109 行](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L109)：
```rust
self.init_lsp();  →  self.init_lsp(cx);
```

## 假设与决策

1. **`ServerStatus` re-export 而非全路径导入**：在 `lib.rs` 添加 `ServerStatus` 到 re-export，demo 侧 `use rust_rml_client::ServerStatus` 更简洁，符合现有 `use rust_rml_client::LanguageClient` 模式
2. **`Ready` + `message` 优先显示 message**：RA 加载完成后（Ready），用户更关心命令操作结果（format/rename 摘要），此时状态栏切换回摘要显示；Loading/Error 期间强制显示 RA 状态提供感知
3. **异步模式：foreground spawn + background recv**：crossbeam `Receiver` 无 `recv_async()`，不能直接在 foreground executor 调用阻塞 `recv()`。采用 `cx.background_executor().spawn()` 执行单次 `recv()`（与 [semantic_tokens.rs:47](file:///d:/GitCode/RF/rust-gpui-rml/crates/rml/src/providers/semantic_tokens.rs#L47) 模式一致），foreground 循环 await 该 background task，既不阻塞 UI 又能持续轮询
4. **`crossbeam_channel::Receiver` 可 Clone**：`rx.clone()` 使每次循环 spawn 独立 background task，外层 `rx` 不被消耗
5. **主题色选择**：Loading 用 `muted_foreground`（灰），Ready 用 `success`（绿），Error 用 `danger_foreground`（红）—— 遵循 [theme-colors 规范](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/gpui-component/rules/theme-colors.md)，避免硬编码颜色值
6. **Tooltip API**：`ServerStatus::Error` 的 msg 显示依赖 `gpui_component::tooltip::Tooltip`，若 API 不匹配则降级为不显示 tooltip（状态栏空间有限，"RA: Error" 文本已提供基本感知）

## 验证步骤

1. **编译验证**：
   - `cargo build -p rust-rml-client --lib`
   - `cargo build -p rust-rml-demo`

2. **单元测试**：
   - `cargo test -p rust-rml-client`（确认 `parse_server_status` 5 个测试通过）

3. **集成验证（手动）**：
   - 启动 demo → 进入 LSP 案例
   - 状态栏立即显示灰色 "RA: Loading..."
   - 30s+ 后变为绿色 "RA: Ready"
   - hover .rml.rs 文件 → 应显示 RA quickinfo（而非 "Loading..."）
   - 执行 format/rename → 状态栏切换为命令摘要
   - 若 RA 加载失败 → 状态栏显示红色 "RA: Error"

4. **回归验证**：
   - 确认 .rml hover/completion/definition 不受影响
   - 确认状态栏其他贡献项（StatusReady 等）正常显示
