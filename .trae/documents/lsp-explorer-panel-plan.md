# LspExplorerPanel 实现方案

## Context

用户需要在 demo 中新增活动栏贡献 `LspExplorerPanel`，实现：

1. 活动栏面板加载 demo 源码目录，树形显示项目结构
2. 点击源码文件打开 Tab，基于 CodeEditor 集成 LSP 语法服务

用户已确认：**子进程 + LSP client** 方式，**.rml + .rml.rs 全覆盖**，使用成熟的 Rust LSP crate（不自行实现协议）。

## 架构设计

```
demo/src/lsp/
├── mod.rs                        # 模块声明 + re-export（仅 type exports）
├── lsp_client.rs                 # LspClient: 子进程管理 + LSP 协议（lsp_server::Connection）
├── file_tree.rs                  # build_source_tree() 扫描 demo/src 构建文件树
├── lsp_explorer_panel.rml.rs     # LspExplorerPanel: 活动栏贡献（文件树面板）
├── lsp_explorer_panel.rml        # Tree 模板
├── code_editor_tab.rs            # CodeEditorTab: Entity（InputState code_editor 模式 + LSP providers）
├── completion_provider.rs        # RmlCompletionProvider: impl CompletionProvider
├── hover_provider.rs             # RmlHoverProvider: impl HoverProvider
└── definition_provider.rs        # RmlDefinitionProvider: impl DefinitionProvider
```

### 核心数据流

```
LspExplorerPanel (文件树)
  └─ 点击文件 → DemoShellHost → MainWindow::open_lsp_file(path)
       └─ 创建 CodeEditorTab Entity（含 InputState + LSP providers）
            └─ LSP providers → LspClient → rml-lsp 子进程（stdio）
```

## 实现步骤

### 1. Cargo.toml 依赖

`demo/Cargo.toml` 新增：

```toml
lsp-server = "0.7"
lsp-types = "0.97"        # 匹配 gpui-component 的版本（provider trait 来自 gpui-component）
ropey = "=2.0.0-beta.1"   # 匹配 gpui-component 的版本
crossbeam-channel = "0.5"  # 请求/响应关联
```

**版本说明**：

* `rml-lsp` crate 内部用 `lsp-types = "0.95"`，demo 用 `0.97`。两者通过子进程 JSON 通信，不产生 crate 版本冲突。LSP 3.17 协议在 0.95/0.97 间无破坏性变化。

* demo 不依赖 `rust-rml-lsp` crate，只通过子进程调用 `rml-lsp` 二进制。

* 用户需先 `cargo build -p rust-rml-lsp --features rust-backend --bin rml-lsp` 构建二进制。

### 2. LspClient（`lsp_client.rs`）

核心职责：spawn `rml-lsp --stdio` 子进程，管理 LSP 协议通信。

**关键设计**：

* `lsp_server::Connection` 的 `sender`/`receiver` 字段是 `pub` 的，可以手动从子进程的 stdout/stdin 构造

* 自建 I/O 线程（仿 `lsp_server` 内部的 `stdio_transport`）读取子进程 stdout → 写入 `crossbeam_channel`，反之亦然

* 请求/响应关联：`AtomicU64` 生成请求 ID，`Arc<Mutex<HashMap<u64, Sender<Result<Value>>>>>` 存储待响应通道

* 后台接收线程：从 `conn.receiver` 读消息，按类型分发（Response → 匹配 pending 通道；Notification → 诊断等）

**公开 API**：

```rust
pub struct LspClient {
    sender: Sender<Message>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, Sender<Result<serde_json::Value>>>>>,
    _child: Child,
    _receiver_thread: JoinHandle<()>,
}

impl LspClient {
    pub fn spawn(workspace_root: &Path) -> Result<Self>;
    pub fn open_document(&self, uri: &Url, text: &str, language_id: &str);
    pub fn change_document(&self, uri: &Url, text: &str);
    pub fn close_document(&self, uri: &Url);
    pub fn completion(&self, uri: &Url, position: Position) -> Receiver<Result<Value>>;
    pub fn hover(&self, uri: &Url, position: Position) -> Receiver<Result<Value>>;
    pub fn definition(&self, uri: &Url, position: Position) -> Receiver<Result<Value>>;
    pub fn shutdown(&self);
}
```

**子进程查找顺序**：

1. `RML_LSP_PATH` 环境变量
2. `{workspace_root}/target/debug/rml-lsp`
3. `{workspace_root}/target/release/rml-lsp`
4. PATH 中的 `rml-lsp`

**客户端 initialize 握手**（手动，因为 `Connection::initialize` 是服务端用的）：

1. 发送 `initialize` Request（含 `root_uri` = workspace\_root）
2. 接收 Response（含 server capabilities）
3. 发送 `initialized` Notification

### 3. LSP Providers（`completion_provider.rs` / `hover_provider.rs` / `definition_provider.rs`）

每个 provider 持有 `Arc<LspClient>` + `Url`（文档 URI），实现 gpui-component 的 trait。

**CompletionProvider**（`gpui_component::input::lsp::CompletionProvider`）：

```rust
pub struct RmlCompletionProvider {
    client: Arc<LspClient>,
    uri: Url,
}

impl CompletionProvider for RmlCompletionProvider {
    fn completions(&self, text: &Rope, offset: usize, trigger: CompletionContext,
                   _window: &mut Window, cx: &mut Context<InputState>) -> Task<Result<CompletionResponse>> {
        let position = text.offset_to_position(offset);
        let rx = self.client.completion(&self.uri, position);
        cx.background_executor().spawn(async move {
            let resp = rx.recv()??;
            let result: CompletionResponse = serde_json::from_value(resp)?;
            Ok(result)
        })
    }

    fn is_completion_trigger(&self, _offset: usize, new_text: &str, _cx: &mut Context<InputState>) -> bool {
        // 触发条件：输入字母、.、<、空格等
        new_text.chars().any(|c| c.is_alphanumeric() || c == '.' || c == '<' || c == ' ')
    }
}
```

**HoverProvider** 和 **DefinitionProvider** 类似，分别返回 `Option<lsp_types::Hover>` 和 `Vec<lsp_types::LocationLink>`。

**关键转换**：`RopeExt::offset_to_position(offset) -> lsp_types::Position`（gpui-component 已提供，`use gpui_component::RopeExt`）。

### 4. file\_tree.rs

扫描 `demo/src/` 目录递归构建 `Vec<rml_ui::TreeItem>`：

* 文件夹 → `TreeItem::new(id, label).children(scan_children)`

* 文件 → `TreeItem::new(id, label)`

* 排序：文件夹优先，然后按名称

* 过滤：跳过 `target/`、`.git/` 等目录

* id 用文件相对路径（如 `cases/button_case.rml.rs`），用于后续打开

**TreeItem 结构**（来自 `gpui_component::tree::TreeItem`）：

```rust
pub struct TreeItem {
    pub id: SharedString,
    pub label: SharedString,
    pub children: Vec<TreeItem>,
    // state 字段由 TreeItem::new 初始化
}
```

**函数签名**（无 cx 参数，纯路径扫描）：

```rust
pub fn build_source_tree() -> Vec<TreeItem> {
    // workspace_root = std::env::current_dir().unwrap()
    // src_dir = workspace_root.join("src")
    scan_dir(&src_dir, "")
}
```

参考 `gpui-component` 的 `tree_story.rs:79-111` 的 `build_file_items` 模式。

### 5. LspExplorerPanel（`lsp_explorer_panel.rml.rs` + `.rml`）

活动栏面板贡献，仿 `activity_panel.rml.rs` 模式（但不做 host）：

```rust
#[contribute(host_id = "demo.shell", id = "lsp_explorer", kind = "activity", order = 10)]
#[component]
#[derive(Default)]
pub struct LspExplorerPanel {
    tree_state: Option<gpui::Entity<TreeState>>,
}

impl IContribution for LspExplorerPanel {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("shell.lsp_explorer").into() }
    fn icon(&self) -> Option<SharedString> { Some("Search".into()) }  // 或 "FileCode"
}

impl ILifecycle for LspExplorerPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let items = crate::lsp::file_tree::build_source_tree();
        self.set_tree_items(items, cx);
        cx.notify();
    }
}

impl LspExplorerPanel {
    #[command]
    pub fn on_file_activate(&mut self, item_id: &gpui::SharedString, cx: &mut Context<Self>) {
        // 仅文件可激活（文件夹由 Tree 自行展开/折叠）
        let path = item_id.to_string();
        if path.ends_with(".rs") || path.ends_with(".rml") {
            if let Some(host) = cx.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
                host.update(cx, |main, cx| {
                    main.open_lsp_file(path, cx);
                });
            }
        }
    }
}
```

模板 `lsp_explorer_panel.rml`：

```html
<component>
    <Tree on_activate="on_file_activate" />
</component>
```

### 6. CodeEditorTab（`code_editor_tab.rs`）

**非贡献**，纯 Entity，由 MainWindow 直接管理。

```rust
pub struct CodeEditorTab {
    editor_state: gpui::Entity<InputState>,
    file_path: String,
    uri: Url,
    lsp_client: Arc<LspClient>,
}

impl CodeEditorTab {
    pub fn new(
        file_path: &str,
        full_path: &Path,
        lsp_client: Arc<LspClient>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        let text = std::fs::read_to_string(full_path).unwrap_or_default();
        let uri = Url::from_file_path(full_path).unwrap();
        let language = if file_path.ends_with(".rml.rs") || file_path.ends_with(".rs") {
            "rust"
        } else {
            "rml"
        };

        // 通知 LSP 服务器打开文档
        lsp_client.open_document(&uri, &text, language);

        let editor_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .tab_size(TabSize { tab_size: 4, ..Default::default() })
                .default_value(&text);
            // 设置 LSP providers
            state.lsp.completion_provider = Some(Rc::new(RmlCompletionProvider::new(lsp_client.clone(), uri.clone())));
            state.lsp.hover_provider = Some(Rc::new(RmlHoverProvider::new(lsp_client.clone(), uri.clone())));
            state.lsp.definition_provider = Some(Rc::new(RmlDefinitionProvider::new(lsp_client.clone(), uri.clone())));
            state
        });

        cx.new(|cx| {
            // observe 编辑器变化 → 同步到 LSP（全量同步）
            let uri_clone = uri.clone();
            let client_clone = lsp_client.clone();
            cx.observe(&editor_state, move |_, state, _cx| {
                let text = state.read(_cx).text().to_string();
                client_clone.change_document(&uri_clone, &text);
            }).detach();

            Self { editor_state, file_path: file_path.to_string(), uri, lsp_client }
        })
    }
}

impl Render for CodeEditorTab {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        Input::new(&self.editor_state)
            .font_family(cx.theme().mono_font_family.clone())
            .text_size(cx.theme().mono_font_size)
            .size_full()
    }
}
```

**文档同步**：`cx.observe(&editor_state, ...)` 在编辑器变化时发送 `textDocument/didChange`（全量同步）。uri 存储为字段，避免重建。

### 7. MainWindow 改造（`demo/src/shell/main_window.rml.rs` + `.rml`）

**新增字段**：

```rust
pub struct MainWindow {
    // ... 现有字段 ...
    lsp_client: Option<Arc<LspClient>>,
    lsp_tabs: HashMap<String, Entity<CodeEditorTab>>,  // key = "lsp://<relative_path>"
}
```

**`on_loaded`** **改造**：

* 在现有初始化末尾追加：spawn LSP 子进程

```rust
// 6. 启动 LSP 子进程
if let Ok(workspace_root) = std::env::current_dir() {
    if let Ok(client) = LspClient::spawn(&workspace_root) {
        self.lsp_client = Some(Arc::new(client));
    }
}
```

* observe LspExplorerPanel Entity（框架缓存），与 ActivityPanel 模式一致

**新增命令**：

```rust
#[command]
pub fn open_lsp_file(&mut self, relative_path: String, cx: &mut Context<Self>) {
    let tab_id = format!("lsp://{}", relative_path);
    if !self.open_tabs.iter().any(|tab| tab.id == tab_id) {
        let title = std::path::Path::new(&relative_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&relative_path)
            .to_string();
        self.open_tabs.push(OpenTab { id: tab_id.clone(), title });
    }
    self.selected_tab = self.open_tabs.iter().position(|tab| tab.id == tab_id).unwrap_or(0);
    self.active_case_id = tab_id.clone();

    // 懒加载 CodeEditorTab
    if !self.lsp_tabs.contains_key(&tab_id) {
        if let Some(client) = &self.lsp_client {
            let full_path = std::env::current_dir().unwrap().join("src").join(&relative_path);
            let tab = CodeEditorTab::new(&relative_path, &full_path, client.clone(), window, cx);
            self.lsp_tabs.insert(tab_id.clone(), tab);
        }
    }
    cx.notify();
}
```

**`active_case_view`** **改造**：

```rust
pub fn active_case_view(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
    if self.active_case_id.starts_with("lsp://") {
        if let Some(tab) = self.lsp_tabs.get(&self.active_case_id) {
            return tab.update(cx, |tab, cx| tab.render(window, cx).into_any_element());
        }
        return gpui::div().into_any_element();
    }
    // ... 现有 case 渲染逻辑 ...
}
```

**`on_tab_click`** **改造**：现有逻辑已兼容（只设置 `active_case_id`），无需修改。

### 8. mod.rs（`demo/src/lsp/mod.rs`）

```rust
pub mod lsp_client;
pub mod file_tree;
pub mod lsp_explorer_panel;
pub mod code_editor_tab;
pub mod completion_provider;
pub mod hover_provider;
pub mod definition_provider;

pub use lsp_client::LspClient;
pub use code_editor_tab::CodeEditorTab;
pub use completion_provider::RmlCompletionProvider;
pub use hover_provider::RmlHoverProvider;
pub use definition_provider::RmlDefinitionProvider;
```

### 9. main.rs 模块声明

`demo/src/main.rs` 新增 `mod lsp;`

### 10. i18n 字符串

`demo/assets/i18n/zh-CN.json` + `en-US.json` 新增：

```json
{
  "shell.lsp_explorer": "LSP 资源管理器" / "LSP Explorer"
}
```

### 11. ActivityPanel observe

MainWindow `on_loaded` 中需 observe LspExplorerPanel Entity（框架缓存），使其变化时触发 ActivityBar 重渲：

```rust
let lsp_panel_entity = rml_app::contribution::visual_entity::<LspExplorerPanel>(cx);
cx.observe(&lsp_panel_entity, |_, _, cx| cx.notify()).detach();
```

## 关键文件路径

需新增的文件：

* `demo/src/lsp/mod.rs`

* `demo/src/lsp/lsp_client.rs`

* `demo/src/lsp/file_tree.rs`

* `demo/src/lsp/lsp_explorer_panel.rml.rs`

* `demo/src/lsp/lsp_explorer_panel.rml`

* `demo/src/lsp/code_editor_tab.rs`

* `demo/src/lsp/completion_provider.rs`

* `demo/src/lsp/hover_provider.rs`

* `demo/src/lsp/definition_provider.rs`

需修改的文件：

* `demo/Cargo.toml` — 新增 lsp-server/lsp-types/ropey/crossbeam-channel 依赖

* `demo/src/main.rs` — 新增 `mod lsp;`

* `demo/src/shell/main_window.rml.rs` — 新增 lsp\_client/lsp\_tabs 字段 + open\_lsp\_file 命令 + active\_case\_view 分流 + on\_loaded 启动 LSP

* `demo/assets/i18n/zh-CN.json` — 新增 i18n key

* `demo/assets/i18n/en-US.json` — 新增 i18n key

## 验证步骤

1. **构建 LSP 二进制**：`cargo build -p rust-rml-lsp --features rust-backend --bin rml-lsp`
2. **构建 demo**：`cargo build -p rust-rml-demo`
3. **运行 demo**：`cargo run -p rust-rml-demo`
4. **验证文件树**：活动栏出现 LSP Explorer 图标，点击展开显示 demo/src 目录树
5. **验证 Tab 打开**：点击树中的 .rs 文件，打开 Tab 显示代码（语法高亮）
6. **验证 LSP 补全**：在编辑器中输入 `<` 或 `{` 后触发补全（.rml 文件）或 `.` 后触发补全（.rml.rs 文件）
7. **验证 LSP hover**：鼠标悬停（Ctrl+hover）显示类型信息
8. **验证 LSP 跳转**：Ctrl+click 跳转到定义

