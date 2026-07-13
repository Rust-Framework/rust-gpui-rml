# 迁移 Terminal 终端组件与 AI Chat 聊天组件到 RML 框架

## 概述

将 `D:\GitCode\RF\rust-agent-ide` 中的两个核心组件迁移到 `d:\GitCode\RF\rust-gpui-rml` RML 框架：

* **Terminal 终端组件** → `crates/ui-term`（包名 `rust-rml-ui-term`）

* **AI Chat 聊天组件** → `crates/ui-chat`（包名 `rust-rml-ui-chat`），目标是**通用聊天组件**，支持快速定制为 IM 聊天和 AI 聊天

### 用户决策

| 决策点              | 选择                             |
| ---------------- | ------------------------------ |
| SDK 依赖（sdk-core） | **完全移除，自建类型**                  |
| Markdown 渲染      | **改用 RML Markdown**（移除 merman） |
| Chat 后端协议        | **简化为 trait 抽象**（移除 ACP）       |

### 架构设计原则

**RML 核心（engine crate）只负责组件注册与 codegen**：

* `tags.rs` → 组件标签路由

* `props_registry.rs` → 属性注册表

* `setters.rs` → setter 分支

* `compiler/components/<name>/` → 专属 codegen 逻辑

**独立 crate 负责全部领域实现**：

* `rust-rml-ui-term`：终端全部领域代码（PTY、alacritty、渲染、输入）

* `rust-rml-ui-chat`：聊天全部领域代码（类型、trait、UI、后端抽象）

**Chat 通用性设计**：

* 泛型消息模型（role-based，兼容 IM 的 sender/receiver 与 AI 的 user/assistant/system）

* 泛型后端 trait（同时支持 IM 的同步收发与 AI 的流式响应）

* 可插拔渲染器（默认纯文本，AI 场景启用 Markdown via RML Markdown 组件）

***

## 实施进度跟踪

> 本节跟踪实际执行进度，便于中断后续接。执行者应先阅读本节了解从何处续接。

| # | 任务                 | 状态     | 说明                                                                                                                                               |
| - | ------------------ | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1 | 创建 ui-term crate   | ✅ 完成   | Cargo.toml ✅；13 源文件 ✅；lib.rs 模块导出 ✅；pty.rs 修复（`crate::view::TerminalView`、TERM\_PROGRAM="RML"）✅；view\.rs 新增 `spawn_default(cx)` ✅                |
| 2 | 创建 ui-chat crate   | 🔄 进行中 | Cargo.toml ✅；model.rs ✅；backend.rs ✅；event.rs ✅；renderer.rs ✅；message\_bubble.rs ✅；message\_list.rs ✅；**input.rs ❌**；**panel.rs ❌**；**lib.rs ❌** |
| 3 | 工作区集成              | ✅ 完成   | 根 Cargo.toml 已添加 `crates/ui-term`、`crates/ui-chat` 到 members + workspace.dependencies                                                            |
| 4 | Engine 注册 Terminal | ⬜ 未开始  | tags.rs + props\_registry.rs + compiler/components/terminal/（**改用 EntityRef**，见下方决策）                                                             |
| 5 | Engine 注册 Chat     | ⬜ 未开始  | tags.rs + props\_registry.rs + compiler/components/chat/（EntityRef）                                                                              |
| 6 | 创建 Demo 案例         | ⬜ 未开始  | terminal\_case + chat\_case 的 .rml + .rml.rs                                                                                                     |
| 7 | 验证                 | ⬜ 未开始  | 编译 + 测试 + 运行                                                                                                                                     |

### 续接清单（按顺序执行）

#### Task #2 续接：完成 ui-chat 剩余 3 文件

1. **创建** **`crates/ui-chat/src/input.rs`** — `ChatInput` 组件

   * 参考：`rust-agent-ide/crates/agent/src/chat/chat_input.rs`（27KB）

   * **移除**：`sdk::icon`、`sdk::l10n`、`sdk::IAIVoiceProvider`、`sdk::AudioRecorder`、语音录制相关、模型选择菜单（`available_models`/`active_model_id`/`on_select_model`）、`add_popup_menu`（@image/@file/@plan 快捷菜单）

   * **保留**：`gpui_component::input::{Input, InputState, InputEvent}` 文本输入、回车发送、Shift+Enter 换行、auto\_grow、`is_streaming`/`set_streaming` 状态、`ChatInputEvent::{Send(String), Stop}` 事件

   * **简化 API**：

     ```rust
     pub struct ChatInput {
         input_state: Entity<InputState>,
         input_has_text: bool,
         is_streaming: bool,
         _input_sub: Option<Subscription>,
     }
     impl ChatInput {
         pub fn new(placeholder: &str, window, cx) -> Self;
         pub fn set_streaming(&mut self, streaming: bool, cx);
         pub fn set_placeholder(&mut self, text: &str, window, cx);
     }
     impl EventEmitter<ChatInputEvent> for ChatInput {}
     impl Render for ChatInput { ... }  // Input + 发送/停止按钮
     ```

   * 发送按钮：`is_streaming` 时显示停止按钮（点击 emit `Stop`），否则显示发送按钮（点击从 input\_state 取值 emit `Send(text)`）

   * 不依赖 `rml_ui`（仅 `gpui` + `gpui_component`）

2. **创建** **`crates/ui-chat/src/panel.rs`** — `ChatPanel` 主 GPUI View

   * 参考：`rust-agent-ide/crates/agent/src/chat/chat_panel.rs`（54KB，ACP 耦合严重）

   * **移除**：`AiSessionService`、`AiPanelAcpBackend`、ACP 协议（`AcpPromptJob`/`AcpPromptStage`）、`sdk::EventBus`、`sdk::ICommandManager`、`sdk::ServiceLocator`、`sdk::AgentLaunchStore`、`ResourceOpener`、`ChatWindowProvider`、`embedded_snapshot`、`workbench_visible`、所有 ACP worker/watchdog/reconnect 逻辑

   * **保留**：`ChatPanel` 主 View 结构、`input_area` + `message_list` 子组件组合、`Render` 实现、焦点管理

   * **简化 API**：

     ```rust
     pub struct ChatPanel {
         conversation: Conversation,
         backend: Option<Arc<dyn ChatBackend>>,
         render_mode: RenderMode,
         input: Option<Entity<ChatInput>>,
         _input_sub: Option<Subscription>,
         message_list: Option<Entity<MessageListView>>,
         _message_list_sub: Option<Subscription>,
         focus_handle: FocusHandle,
         next_message_id: u64,
         is_streaming: bool,
     }
     impl ChatPanel {
         pub fn new(render_mode: RenderMode, cx) -> Self;  // 无 backend，由 set_backend 注入
         pub fn set_backend(&mut self, backend: Arc<dyn ChatBackend>, cx);
         pub fn send_message(&mut self, content: String, cx);  // 调用 backend.stream_message
         pub fn cancel(&mut self, cx);
         pub fn messages(&self) -> &[Message];
     }
     impl EventEmitter<ChatEvent> for ChatPanel {}
     impl Render for ChatPanel { ... }  // 顶部消息列表 + 底部输入区
     ```

   * **send\_message 流程**：追加 user message → 设置 `is_streaming=true` → 创建空 assistant message（streaming=true）→ spawn 异步任务调用 `backend.stream_message(conv, content, on_chunk)` → `on_chunk` 中 `entity.update(cx, |this, cx| this.message_list.update(...).update_last_message(chunk))` → 完成后 `is_streaming=false` + emit `ChatEvent::MessageReceived`

3. **创建** **`crates/ui-chat/src/lib.rs`** — 模块导出

   ```rust
   pub mod backend;
   pub mod event;
   pub mod input;
   pub mod message_bubble;
   pub mod message_list;
   pub mod model;
   pub mod panel;
   pub mod renderer;

   pub use backend::{ChatBackend, ChatError};
   pub use event::{ChatEvent, ChatInputEvent};
   pub use input::ChatInput;
   pub use message_bubble::ChatBubble;
   pub use message_list::{MessageListView, MessageListEvent};
   pub use model::{Attachment, Conversation, Message, MessageMetadata, MessageRole, ToolCall};
   pub use panel::ChatPanel;
   pub use renderer::{render_content, RenderMode};
   ```

#### Task #4：Engine 注册 Terminal 组件（**改用 EntityRef**）

> **决策修正**：原计划使用 `Stateful`，但 `TerminalView::new()` 签名为 `new<W, R>(stdin_writer, stdout_reader, config, cx)`，不接受 `&Entity<TerminalView>`。Stateful codegen 会生成 `ctor_path::new(&Entity<T>)`，与现有构造器不兼容。`TerminalView::spawn_default(cx: &mut Context<Self>) -> Self` 也要求在 Entity 上下文中调用。因此改用 `EntityRef`，用户在 ViewModel `on_loaded` 中通过 `cx.new(|cx| TerminalView::spawn_default(cx))` 创建 `Entity<TerminalView>`。

1. **修改** **`crates/engine/src/tags.rs`** — 在 `component_lookup()` 末尾添加：

   ```rust
   "Terminal" | "terminal" => Some(ComponentTag {
       ctor_path: "rml_ui_term::TerminalView",
       kind: ComponentKind::EntityRef,
       container: false,
   }),
   ```

2. **修改** **`crates/engine/src/compiler/props_registry.rs`** — `COMPONENT_PROPS` 添加空条目（EntityRef 组件无 RML 属性直接绑定，配置通过 ViewModel 中获取 `ElementRef<TerminalView>` 后调用 API）：

   ```rust
   ("Terminal", &[]),
   ```

3. **不创建** `compiler/components/terminal/`（EntityRef 组件走通用 translator，无需专属 codegen）

#### Task #5：Engine 注册 Chat 组件（EntityRef）

1. **修改** **`crates/engine/src/tags.rs`** — 添加：

   ```rust
   "Chat" | "chat" => Some(ComponentTag {
       ctor_path: "rml_ui_chat::ChatPanel",
       kind: ComponentKind::EntityRef,
       container: false,
   }),
   ```

2. **修改** **`crates/engine/src/compiler/props_registry.rs`** — 添加空条目：

   ```rust
   ("Chat", &[]),
   ```

3. **不创建** `compiler/components/chat/`

#### Task #6：创建 Demo 案例

1. **`demo/src/cases/terminal_case.rml`** + **`terminal_case.rml.rs`**

   ```rml
   <component>
       <CaseDocPage title="Terminal" description="嵌入式终端组件">
           <template slot="demo">
               <Terminal ref="term" />
           </template>
       </CaseDocPage>
   </component>
   ```

   * ViewModel `TerminalCase`：`ElementRef<TerminalView>` 字段 `term`

   * `on_loaded` 中：`let term = cx.new(|cx| rml_ui_term::TerminalView::spawn_default(cx)); self.term.set(term);`

2. **`demo/src/cases/chat_case.rml`** + **`chat_case.rml.rs`**

   ```rml
   <component>
       <CaseDocPage title="Chat" description="通用聊天组件">
           <template slot="demo">
               <Chat ref="chat" />
           </template>
       </CaseDocPage>
   </component>
   ```

   * ViewModel `ChatCase`：`ElementRef<ChatPanel>` 字段 `chat`

   * `on_loaded` 中：创建 `Entity<ChatPanel>` 并注入 `MockBackend`（echo 响应）

3. **修改** **`demo/Cargo.toml`** — 添加 `rust-rml-ui-term` 和 `rust-rml-ui-chat` 依赖

#### Task #7：验证

1. `cargo check -p rust-rml-ui-term`
2. `cargo check -p rust-rml-ui-chat`
3. `cargo check -p rust-rml-engine`
4. `cargo test -p rust-rml-engine --lib props_registry::tests`（Terminal/Chat 注册一致性）
5. `cargo check -p demo`
6. `cargo build -p demo`

***

## 当前状态分析

### 源项目 Terminal（rust-agent-ide/crates/terminal）

**Crate 名**：`terminal`（需重命名为 `rust-rml-ui-term`）

**依赖**：

* `alacritty_terminal = "0.26"` — 终端模拟器核心

* `portable-pty = "0.9"` — 跨平台 PTY

* `flume = "0.12"` — 异步通道

* `parking_lot = "0.12"` — 锁

* `arboard = "3"` — 剪贴板

* `smol = "2"` — 异步运行时

* `sdk (sdk-core)` — **需移除**

* `gpui`, `gpui-component` — 保留

**源文件（18 个 .rs）**：

```
src/
├── lib.rs                    # 模块导出 + TerminalModule（SDK 耦合，需移除）
├── colors.rs                 # 颜色工具
├── events.rs                 # 事件类型
├── panel.rs                  # 终端面板（IDE 耦合，需移除/重写）
├── pty_process.rs            # PTY 进程管理（spawn_terminal）
├── tab.rs                    # 终端标签页（IDE 耦合，需移除/重写）
├── workbench_provider.rs     # 工作台提供者（SDK 耦合，需移除）
└── emulator/
    ├── mod.rs                # 模块入口
    ├── terminal.rs           # TerminalState（alacritty 集成）
    ├── terminal_scroll.rs    # 滚动历史
    ├── view.rs               # TerminalView（GPUI View，~1396 行）
    ├── render.rs             # TerminalRenderer
    ├── input.rs              # 键盘输入
    ├── mouse.rs              # 鼠标事件
    ├── interaction.rs        # TerminalLayout（几何/命中测试）
    ├── event.rs              # TerminalEvent, GpuiEventProxy
    ├── clipboard.rs          # 剪贴板
    └── colors.rs             # ColorPalette
```

**关键 API**：

* `TerminalView` — 主 GPUI View，实现 `Render` + `Focusable`

  * `TerminalView::new(stdin_writer, stdout_reader, config, cx)` — 创建终端

  * `.with_resize_callback()` / `.with_exit_callback()` / `.with_key_handler()` / `.with_bell_callback()` / `.with_title_callback()` / `.with_clipboard_store_callback()` — 回调构建器

* `TerminalState` — 终端状态（grid、cursor、VTE 解析器）

  * `TerminalState::new(cols, rows, event_proxy)` / `.process_bytes()` / `.resize()`

* `TerminalConfig` — 配置（cols/rows/font\_family/font\_size/scrollback/colors/padding）

* `ColorPalette` — 颜色调色板

* `TerminalEvent` — 终端事件枚举

* `GpuiEventProxy` — alacritty 事件代理

* `spawn_terminal(shell, cwd, env, cols, rows)` — PTY 进程创建

**SDK 耦合点**：

1. `lib.rs` — `TerminalModule` 实现 `sdk::Module` trait，`sdk::declare_module!()`，`sdk::l10n_auto!()`
2. `workbench_provider.rs` — `WorkbenchProvider` / `TerminalWorkbenchProvider`（IDE 工作台集成）
3. `panel.rs` / `tab.rs` — 使用 `sdk::EventBus` 等服务
4. 所有 `sdk::l10n::t()` 调用

### 源项目 AI Chat（rust-agent-ide/crates/agent/src/chat）

**Crate 名**：`agent`（chat 是其中的模块，需提取为独立 crate `rust-rml-ui-chat`）

**依赖**：

* `agent-client-protocol = "0.13"` — **需移除**

* `merman = "0.6"` — **需移除**（改用 RML Markdown）

* `tokio` + `tokio-util` — 异步运行时（保留）

* `reqwest = "0.13"` — HTTP 客户端（保留，用于后端实现）

* `serde` + `serde_json` — 序列化（保留）

* `uuid = "1"` — ID 生成（保留）

* `sdk (sdk-core)` — **需移除**

* `gpui`, `gpui-component` — 保留

* `inventory` — **需移除**（RML 用不同注册机制）

**源文件（20 个 .rs）**：

```
chat/
├── mod.rs                    # 模块入口 + ChatMessageExt/ChatSessionExt（SDK 类型扩展）
├── backend.rs                # AiPanelAcpBackend trait（ACP 耦合，需重写）
├── chat_panel.rs             # ChatPanel（主 GPUI View）
├── chat_input.rs             # ChatInput
├── chat_bubble.rs            # ChatBubble（消息气泡）
├── message_list_view.rs      # MessageListView
├── context.rs                # 聊天上下文
├── events.rs                 # AiSessionChanged 等事件
├── service.rs                # AiSessionService
└── renderers/
    ├── mod.rs                # 渲染器入口
    ├── pipeline.rs           # 渲染管线
    ├── code_block.rs         # 代码块渲染（改用 RML Markdown）
    ├── callout.rs            # 标注块（移除）
    ├── diff.rs               # 差异渲染（移除）
    ├── footnote.rs           # 脚注（移除）
    ├── math.rs               # 数学公式（移除）
    ├── mermaid.rs            # Mermaid 图表（移除）
    ├── ocr_image.rs          # OCR 图片（移除）
    ├── thinking.rs           # 思考过程（保留为消息元数据渲染）
    └── tool_call.rs          # 工具调用（保留为消息元数据渲染）
```

**关键 API**：

* `ChatPanel` — 主 GPUI View，实现 `Render`

  * `ChatPanel::new(session, backend, service)` — 创建聊天面板

* `ChatInput` — 输入组件

* `ChatBubble` — 消息气泡

* `MessageListView` — 消息列表

* `AiSessionService` — 会话服务

* `AiPanelAcpBackend` — ACP 后端 trait（需替换为通用 `ChatBackend`）

* `ChatMessage` — 消息（来自 `sdk::event_types`，需自建）

* `ChatSession` — 会话（来自 `sdk::event_types`，需自建）

* `MessageRole` — 消息角色（来自 `sdk::event_types`，需自建）

**SDK 耦合点**：

1. `mod.rs` — `pub use sdk::event_types::{ChatMessage, ChatSession, MessageRole}`
2. `mod.rs` — `AiPanelServiceHandle` 实现 `sdk::IAIPanelAdapter`
3. `mod.rs` — `sdk::l10n::t()` 调用
4. `backend.rs` — `AiPanelAcpBackend` 依赖 ACP 协议
5. `service.rs` — 使用 `sdk` 类型和服务

### 目标框架 RML（rust-gpui-rml）

**工作区成员**：`core`, `macros`, `engine`, `ui`, `app`, `rml`, `lsp`, `demo`

**组件注册三件套**（位于 engine crate）：

1. `crates/engine/src/tags.rs` → `component_lookup()` 函数：tag → `ComponentTag { ctor_path, kind, container }`
2. `crates/engine/src/compiler/props_registry.rs` → `COMPONENT_PROPS`：组件名 → 专用属性列表
3. `crates/engine/src/compiler/setters.rs` → `component_static_setter` / `component_bind_setter` / `component_event_setter`

**复杂组件 codegen 模式**（以 CodeEditor 为参考）：

```
crates/engine/src/compiler/components/code_editor/
├── mod.rs    # 模块入口，re-export gen
└── gen.rs    # 生成逻辑，声明 pub const HANDLED_PROPS: &[&str]
```

**MVVM 模式**：

* `.rml` 模板声明 UI 结构与数据绑定

* `.rml.rs` ViewModel 包含 `pub` 字段、`#[computed]`、`#[command]`、`ILifecycle`、`IContribution`

**命名约定**：所有 crate 使用 `rust-rml-*` 前缀

**约束**（来自项目记忆）：

* mod.rs 仅 re-export，禁止业务代码

* 一个 rs 文件 = 一个组件/职责

* COMPONENT\_PROPS 与 component\_lookup 必须一致（有测试强制校验）

* 内联处理的属性需声明 `HANDLED_PROPS` 并在 `COMPONENT_PROPS` 登记

* 所有 demo 遵循 `.rml` + `.rml.rs` MVVM 模式

***

## 提议变更

### 第一部分：创建 `crates/ui-term`（rust-rml-ui-term）

#### 1.1 创建 Cargo.toml

**文件**：`crates/ui-term/Cargo.toml`

```toml
[package]
name = "rust-rml-ui-term"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "RML 终端组件：基于 alacritty_terminal + portable-pty 的嵌入式终端"

[dependencies]
rust-rml-core = { workspace = true, features = ["gpui-component"] }
gpui = { workspace = true }
gpui-component = { workspace = true }
alacritty_terminal = "0.26"
portable-pty = "0.9"
flume = "0.12"
parking_lot = "0.12"
arboard = "3"
smol = "2"
anyhow = { workspace = true }
tracing = "0.4"

[features]
default = []
```

#### 1.2 创建源文件结构

从源项目迁移，移除 SDK 耦合，遵循"一个 rs 文件 = 一个职责"原则：

**文件**：`crates/ui-term/src/lib.rs`（模块导出 + init 函数）

* 导出 `TerminalView`, `TerminalState`, `TerminalConfig`, `ColorPalette`, `TerminalEvent`, `GpuiEventProxy`

* 移除 `TerminalModule`, `WorkbenchProvider`, `sdk::declare_module!`

* 提供 `pub fn init(cx: &mut App)` 注册函数（如需要）

**文件**：`crates/ui-term/src/pty.rs`（PTY 进程管理）

* 从 `pty_process.rs` 迁移

* 保留 `spawn_terminal(shell, cwd, env, cols, rows) -> (Box<dyn Write + Send>, Box<dyn Read + Send>, PtyChild)`

* 移除任何 SDK 依赖

**文件**：`crates/ui-term/src/config.rs`（终端配置）

* 合并源 `emulator/colors.rs` 的 `ColorPalette` + `TerminalConfig`

* 字段：cols, rows, font\_family, font\_size, scrollback, line\_height\_multiplier, padding, colors

**文件**：`crates/ui-term/src/event.rs`（终端事件）

* 从 `emulator/event.rs` 迁移

* `TerminalEvent` 枚举 + `GpuiEventProxy`

**文件**：`crates/ui-term/src/state.rs`（终端状态）

* 从 `emulator/terminal.rs` 迁移 `TerminalState`

* 从 `emulator/terminal_scroll.rs` 迁移 `TerminalScrollHandle`

* alacritty 集成、VTE 解析、resize

**文件**：`crates/ui-term/src/view.rs`（终端视图，主组件）

* 从 `emulator/view.rs` 迁移 `TerminalView`

* 实现 `Render` + `Focusable`

* 保留所有 builder 方法：`with_resize_callback`, `with_exit_callback`, `with_key_handler`, `with_bell_callback`, `with_title_callback`, `with_clipboard_store_callback`

* **新增**：`spawn(shell, cwd, config, cx)` 便捷方法，内部调用 `spawn_terminal` 并创建视图

**文件**：`crates/ui-term/src/render.rs`（终端渲染器）

* 从 `emulator/render.rs` 迁移 `TerminalRenderer`

**文件**：`crates/ui-term/src/input.rs`（键盘输入）

* 从 `emulator/input.rs` 迁移

* `keystroke_to_bytes`, `is_paste_keystroke`, `normalize_paste_bytes`

**文件**：`crates/ui-term/src/mouse.rs`（鼠标事件）

* 从 `emulator/mouse.rs` 迁移

* `encode_modifiers`, `mouse_button_report`, `pixels_to_scroll_lines`, `scroll_report`, `selection_type_from_clicks`

**文件**：`crates/ui-term/src/clipboard.rs`（剪贴板）

* 从 `emulator/clipboard.rs` 迁移 `Clipboard` trait + arboard 实现

**文件**：`crates/ui-term/src/layout.rs`（布局与命中测试）

* 从 `emulator/interaction.rs` 迁移 `TerminalLayout`

**文件**：`crates/ui-term/src/colors.rs`（颜色工具）

* 从源 `colors.rs`（顶层）迁移

* 颜色转换工具函数

**移除的文件**（IDE 专用，不迁移）：

* `workbench_provider.rs` — SDK 工作台集成

* `panel.rs` — IDE 面板（由 RML 组件系统替代）

* `tab.rs` — IDE 标签页（由 RML 组件系统替代）

* `events.rs`（顶层）— 如果与 `emulator/event.rs` 重复则合并

#### 1.3 在 RML Engine 中注册 Terminal 组件

> ⚠️ **决策修正**：原计划使用 `Stateful`，但 `TerminalView::new()` 不接受 `&Entity<T>`。改用 `EntityRef`，详见上方"实施进度跟踪 / Task #4"。

**修改**：`crates/engine/src/tags.rs` — `component_lookup()` 添加：

```rust
"Terminal" | "terminal" => Some(ComponentTag {
    ctor_path: "rml_ui_term::TerminalView",
    kind: ComponentKind::EntityRef,
    container: false,
}),
```

**修改**：`crates/engine/src/compiler/props_registry.rs` — `COMPONENT_PROPS` 添加：

```rust
("Terminal", &[
    "cols", "rows", "shell", "working_dir",
    "font_family", "font_size", "scrollback",
    "on_exit", "on_title_change", "on_bell",
]),
```

**创建**：`crates/engine/src/compiler/components/terminal/mod.rs`

```rust
pub mod gen;
pub use gen::{gen_terminal, HANDLED_PROPS};
```

**创建**：`crates/engine/src/compiler/components/terminal/gen.rs`

* 声明 `pub const HANDLED_PROPS: &[&str] = &["cols", "rows", "shell", "working_dir", "font_family", "font_size", "scrollback", "on_exit", "on_title_change", "on_bell"];`

* 实现 `gen_terminal()` 函数，处理内联属性（cols/rows/shell/working\_dir 注入 state\_ctor，font\_\* 生成 setter 调用，on\_\* 生成事件订阅）

**修改**：`crates/engine/src/compiler/components/mod.rs` — 添加 `pub mod terminal;`

**修改**：`crates/engine/src/compiler/setters.rs` — 添加 Terminal setter 分支（如非全内联处理）

#### 1.4 工作区集成

**修改**：根 `Cargo.toml`

* `[workspace] members` 添加 `"crates/ui-term"`

* `[workspace.dependencies]` 添加 `rust-rml-ui-term = { path = "crates/ui-term" }`

***

### 第二部分：创建 `crates/ui-chat`（rust-rml-ui-chat）

#### 2.1 创建 Cargo.toml

**文件**：`crates/ui-chat/Cargo.toml`

```toml
[package]
name = "rust-rml-ui-chat"
version.workspace = true
edition.workspace = true
license.workspace = true
description = "RML 通用聊天组件：支持 IM 聊天与 AI 聊天快速定制"

[dependencies]
rust-rml-core = { workspace = true, features = ["gpui-component"] }
rust-rml-ui = { workspace = true }
gpui = { workspace = true }
gpui-component = { workspace = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4"] }
anyhow = { workspace = true }
tracing = "0.4"

[features]
default = []
# 启用流式响应支持（AI 聊天场景）
streaming = []
# 启用异步后端支持
async-backend = ["tokio"]
tokio = { version = "1", features = ["rt", "macros", "sync"], optional = true }
```

#### 2.2 创建源文件结构

**通用聊天架构设计**：

**文件**：`crates/ui-chat/src/lib.rs`（模块导出）

* 导出所有公共类型和组件

**文件**：`crates/ui-chat/src/model.rs`（通用消息模型，自建，无 SDK）

```rust
pub struct Message {
    pub id: u64,
    pub role: MessageRole,
    pub content: String,
    pub timestamp_ms: u64,
    pub metadata: MessageMetadata,  // 扩展字段（thinking, tool_call 等）
}

pub enum MessageRole {
    User,       // IM: 发送者 / AI: 用户
    Assistant,  // IM: 接收者 / AI: 助手
    System,     // 系统消息
    Custom(String), // 自定义角色
}

pub struct MessageMetadata {
    pub thinking: Option<String>,      // AI 思考过程
    pub tool_calls: Vec<ToolCall>,     // AI 工具调用
    pub attachments: Vec<Attachment>,  // 附件（IM/AI 通用）
    pub is_streaming: bool,            // 是否正在流式输出
}

pub struct Conversation {
    pub id: u64,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at_ms: u64,
}
```

**文件**：`crates/ui-chat/src/backend.rs`（通用后端 trait，替代 ACP）

```rust
/// 通用聊天后端 trait
/// - IM 场景：实现 send_message 返回同步响应
/// - AI 场景：实现 stream_message 返回流式响应（需 streaming feature）
pub trait ChatBackend: Send + Sync {
    /// 发送消息，返回完整响应
    fn send_message(&self, conversation: &Conversation, content: &str) -> Result<String, ChatError>;
    
    /// 流式发送消息（AI 场景），通过回调推送增量
    #[cfg(feature = "streaming")]
    fn stream_message(
        &self,
        conversation: &Conversation,
        content: &str,
        on_chunk: Box<dyn Fn(&str) + Send>,
    ) -> Result<(), ChatError>;
    
    /// 取消当前请求
    fn cancel(&self) -> Result<(), ChatError>;
}

pub enum ChatError {
    Network(String),
    Cancelled,
    Backend(String),
}
```

**文件**：`crates/ui-chat/src/event.rs`（聊天事件）

* `ChatEvent` 枚举（MessageReceived, MessageSent, StreamChunk, StreamEnd, Error, Cancelled）

* 移除 `AiSessionChanged`，替换为通用 `ChatEvent`

**文件**：`crates/ui-chat/src/panel.rs`（聊天面板，主 GPUI View）

* 从 `chat_panel.rs` 迁移 `ChatPanel`

* **关键变更**：

  * 移除 `AiPanelAcpBackend`，使用 `ChatBackend` trait

  * 移除 `AiSessionService`，简化为内部状态管理

  * 移除 `sdk::l10n::t()`，硬编码或用 RML i18n

  * 保留 `Render` 实现

  * **新增**：`ChatPanel::new(backend: Arc<dyn ChatBackend>)` 简化构造

**文件**：`crates/ui-chat/src/message_list.rs`（消息列表）

* 从 `message_list_view.rs` 迁移 `MessageListView`

* 渲染 `Vec<Message>` 为消息气泡列表

**文件**：`crates/ui-chat/src/message_bubble.rs`（消息气泡）

* 从 `chat_bubble.rs` 迁移 `ChatBubble`

* 根据 `MessageRole` 选择气泡样式（左/右对齐、颜色）

**文件**：`crates/ui-chat/src/input.rs`（聊天输入）

* 从 `chat_input.rs` 迁移 `ChatInput`

* 文本输入 + 发送按钮

* 支持 placeholder、disabled、loading 状态

**文件**：`crates/ui-chat/src/renderer.rs`（消息渲染器，使用 RML Markdown）

```rust
/// 消息内容渲染器 trait
pub trait MessageRenderer: Send + Sync {
    fn render(&self, content: &str, cx: &mut Context<ChatPanel>) -> impl IntoElement;
}

/// 纯文本渲染器（IM 默认）
pub struct PlainTextRenderer;

/// Markdown 渲染器（AI 默认），使用 RML 的 Markdown 组件
pub struct MarkdownRenderer;

/// 思考过程渲染器（AI 扩展）
pub struct ThinkingRenderer;
```

* **关键变更**：移除 `merman` 依赖，MarkdownRenderer 使用 `rml_ui::Markdown` 组件

* **移除的渲染器**：callout, diff, footnote, math, mermaid, ocr\_image（用户选择改用 RML Markdown）

* **保留的渲染器**：thinking, tool\_call（作为消息元数据的特殊渲染）

**文件**：`crates/ui-chat/src/service.rs`（会话管理，可选）

* 简化版的会话管理（创建、切换、删除会话）

* 移除 `AiSessionService` 的 ACP 耦合

**移除的文件/模块**：

* `renderers/pipeline.rs` — merman 渲染管线（移除）

* `renderers/callout.rs` — 移除

* `renderers/diff.rs` — 移除

* `renderers/footnote.rs` — 移除

* `renderers/math.rs` — 移除

* `renderers/mermaid.rs` — 移除

* `renderers/ocr_image.rs` — 移除

* `renderers/code_block.rs` — 移除（由 RML Markdown 处理）

* `context.rs` — 合并到 model.rs 或 panel.rs

* `mod.rs` 中的 `AiPanelServiceHandle` + `IAIPanelAdapter` 实现 — 移除

* `mod.rs` 中的 `ChatMessageExt` / `ChatSessionExt` — 合并到 model.rs 的 impl 块

#### 2.3 在 RML Engine 中注册 Chat 组件

**修改**：`crates/engine/src/tags.rs` — `component_lookup()` 添加：

```rust
"Chat" | "chat" => Some(ComponentTag {
    ctor_path: "rml_ui_chat::ChatPanel",
    kind: ComponentKind::EntityRef,
    container: false,
}),
```

**修改**：`crates/engine/src/compiler/props_registry.rs` — `COMPONENT_PROPS` 添加：

```rust
("Chat", &[
    "messages", "backend", "placeholder",
    "markdown", "variant", "loading",
    "on_send", "on_cancel", "on_message_click",
]),
```

**创建**：`crates/engine/src/compiler/components/chat/mod.rs`

```rust
pub mod gen;
pub use gen::{gen_chat, HANDLED_PROPS};
```

**创建**：`crates/engine/src/compiler/components/chat/gen.rs`

* 声明 `pub const HANDLED_PROPS`

* 实现 `gen_chat()` 函数

**修改**：`crates/engine/src/compiler/components/mod.rs` — 添加 `pub mod chat;`

#### 2.4 工作区集成

**修改**：根 `Cargo.toml`

* `[workspace] members` 添加 `"crates/ui-chat"`

* `[workspace.dependencies]` 添加 `rust-rml-ui-chat = { path = "crates/ui-chat" }`

***

### 第三部分：Demo 案例

#### 3.1 Terminal Demo

**创建**：`demo/src/cases/terminal_case.rml`

```rml
<component>
    <CaseDocPage title={t("case.terminal.title")} description="...">
        <template slot="demo">
            <Terminal ref="term" cols="80" rows="24" 
                      shell="powershell" font_size="14"
                      on_exit={handle_exit} on_title_change={handle_title} />
        </template>
    </CaseDocPage>
</component>
```

**创建**：`demo/src/cases/terminal_case.rml.rs`

* ViewModel：`TerminalCase` struct

* `#[contribute]` 注册到 demo shell

* `ElementRef<TerminalView>` 引用终端实例

* `#[command]` 处理 `handle_exit` / `handle_title`

**修改**：`demo/Cargo.toml` — 添加 `rust-rml-ui-term` 依赖

#### 3.2 Chat Demo

**创建**：`demo/src/cases/chat_case.rml`

```rml
<component>
    <CaseDocPage title={t("case.chat.title")} description="...">
        <template slot="demo">
            <Chat ref="chat" messages={messages}
                  markdown="" variant="ai"
                  placeholder="输入消息..."
                  on_send={handle_send} on_cancel={handle_cancel} />
        </template>
    </CaseDocPage>
</component>
```

**创建**：`demo/src/cases/chat_case.rml.rs`

* ViewModel：`ChatCase` struct

* `#[contribute]` 注册到 demo shell

* `pub messages: Vec<Message>` observable 状态

* `#[command]` 处理 `handle_send` / `handle_cancel`

* 实现一个简单的 `MockBackend`（echo 响应）用于演示

**修改**：`demo/Cargo.toml` — 添加 `rust-rml-ui-chat` 依赖

***

## 假设与决策

### 架构决策

1. **Terminal 组件 kind = EntityRef**（**已修正**）：原计划 Stateful，但 `TerminalView::new()` 签名 `(stdin_writer, stdout_reader, config, cx)` 不接受 `&Entity<TerminalView>`，与 Stateful codegen 生成的 `ctor_path::new(&Entity<T>)` 不兼容。`spawn_default(cx)` 返回 `Self` 且需在 `Context<Self>` 中调用。改用 EntityRef：用户在 ViewModel `on_loaded` 中通过 `cx.new(|cx| TerminalView::spawn_default(cx))` 创建 `Entity<TerminalView>`，RML 模板用 `ref="term"` 引用。

2. **Chat 组件 kind = EntityRef**：聊天面板作为 EntityRef 组件，用户通过 `ref` 指令获取 `Entity<ChatPanel>` 实例，在 ViewModel 中配置 backend。这比 Stateful 更灵活，因为 ChatBackend 实现需要在 Rust 代码中提供。

3. **Terminal 便捷构造**：在 `TerminalView` 上新增 `spawn_default(cx: &mut Context<Self>) -> Self` 方法（已实现），内部调用 `spawn_terminal(None, None, ...)` 使用系统默认 shell 与当前工作目录。用户在 ViewModel `on_loaded` 中通过 `cx.new(|cx| TerminalView::spawn_default(cx))` 创建 `Entity<TerminalView>`。

4. **Chat 后端注入方式**：ChatPanel 通过 `EntityRef` 模式，用户在 ViewModel 的 `on_loaded` 中获取 `ElementRef<ChatPanel>` 并调用 `.set_backend()` 注入后端实现。RML 层不直接绑定 backend（trait 对象无法序列化）。

5. **Markdown 渲染集成**：`rust-rml-ui-chat` 依赖 `rust-rml-ui` 以使用 `Markdown` 组件。`MarkdownRenderer` 内部调用 `rml_ui::Markdown::new().content(text)` 渲染消息内容。

6. **Chat 通用性设计**：

   * `MessageRole` 枚举支持 User/Assistant/System/Custom，兼容 IM（发送者/接收者）和 AI（用户/助手/系统）

   * `ChatBackend` trait 的 `send_message` 同步方法覆盖 IM 场景，`stream_message` 方法（streaming feature）覆盖 AI 场景

   * `MessageRenderer` trait 允许自定义渲染（纯文本/Markdown/富内容）

   * `variant` 属性（"ai"/"im"）控制 UI 风格（气泡样式、头像显示等）

7. **移除的 IDE 专用功能**：

   * Terminal: WorkbenchProvider、TerminalTab、TerminalPanel（IDE 集成层）

   * Chat: AiPanelServiceHandle、IAIPanelAdapter、ACP 协议、merman 渲染管线

   * 这些功能在 RML 框架中由用户通过 MVVM 模式自行组装

### 文件迁移映射

| 源文件                                        | 目标文件                              | 变更                          |
| ------------------------------------------ | --------------------------------- | --------------------------- |
| `terminal/src/emulator/view.rs`            | `ui-term/src/view.rs`             | 移除 SDK 依赖，新增 spawn\_default |
| `terminal/src/emulator/terminal.rs`        | `ui-term/src/state.rs`            | 移除 SDK 依赖                   |
| `terminal/src/emulator/render.rs`          | `ui-term/src/render.rs`           | 无重大变更                       |
| `terminal/src/pty_process.rs`              | `ui-term/src/pty.rs`              | 移除 SDK 依赖                   |
| `terminal/src/emulator/input.rs`           | `ui-term/src/input.rs`            | 无变更                         |
| `terminal/src/emulator/mouse.rs`           | `ui-term/src/mouse.rs`            | 无变更                         |
| `terminal/src/emulator/event.rs`           | `ui-term/src/event.rs`            | 无变更                         |
| `terminal/src/emulator/clipboard.rs`       | `ui-term/src/clipboard.rs`        | 无变更                         |
| `terminal/src/emulator/interaction.rs`     | `ui-term/src/layout.rs`           | 无变更                         |
| `terminal/src/emulator/terminal_scroll.rs` | `ui-term/src/scroll.rs`           | 无变更                         |
| `terminal/src/emulator/colors.rs`          | `ui-term/src/config.rs`           | 合并 TerminalConfig           |
| `terminal/src/lib.rs`                      | `ui-term/src/lib.rs`              | 移除 Module/WorkbenchProvider |
| `terminal/src/workbench_provider.rs`       | —                                 | **移除**（IDE 专用）              |
| `terminal/src/panel.rs`                    | —                                 | **移除**（IDE 专用）              |
| `terminal/src/tab.rs`                      | —                                 | **移除**（IDE 专用）              |
| `agent/src/chat/mod.rs`                    | `ui-chat/src/lib.rs` + `model.rs` | 移除 SDK 类型，自建                |
| `agent/src/chat/chat_panel.rs`             | `ui-chat/src/panel.rs`            | 使用 ChatBackend trait        |
| `agent/src/chat/chat_input.rs`             | `ui-chat/src/input.rs`            | 无重大变更                       |
| `agent/src/chat/chat_bubble.rs`            | `ui-chat/src/message_bubble.rs`   | 无重大变更                       |
| `agent/src/chat/message_list_view.rs`      | `ui-chat/src/message_list.rs`     | 无重大变更                       |
| `agent/src/chat/backend.rs`                | `ui-chat/src/backend.rs`          | **重写**为 ChatBackend trait   |
| `agent/src/chat/events.rs`                 | `ui-chat/src/event.rs`            | 简化为 ChatEvent               |
| `agent/src/chat/service.rs`                | `ui-chat/src/service.rs`          | 移除 ACP 耦合                   |
| `agent/src/chat/context.rs`                | 合并到 `model.rs`                    | 移除                          |
| `agent/src/chat/renderers/*`               | `ui-chat/src/renderer.rs`         | **重写**，用 RML Markdown       |

***

## 验证步骤

### 编译验证

1. `cargo check -p rust-rml-ui-term` — 终端 crate 编译通过
2. `cargo check -p rust-rml-ui-chat` — 聊天 crate 编译通过
3. `cargo check -p rust-rml-engine` — engine crate 编译通过（含新注册）
4. `cargo check -p demo` — demo 编译通过

### 测试验证

1. `cargo test -p rust-rml-engine --lib props_registry::tests` — 组件注册一致性测试通过

   * `components_without_props_entry_audit` — Terminal/Chat 在 COMPONENT\_PROPS 中有条目

   * `inline_handled_props_are_registered` — HANDLED\_PROPS 反向校验

   * `registered_props_have_setter_or_inline_handling` — 正向校验
2. `cargo test -p rust-rml-ui-term` — 终端 crate 单元测试
3. `cargo test -p rust-rml-ui-chat` — 聊天 crate 单元测试

### 集成验证

1. `cargo build -p demo` — demo 构建成功
2. 运行 demo，验证 Terminal 案例可启动终端、输入命令、看到输出
3. 运行 demo，验证 Chat 案例可发送消息、收到 mock 响应

### 架构约束验证

1. 确认所有 mod.rs 仅 re-export，无业务代码
2. 确认每个 rs 文件只有一个独立组件/职责
3. 确认无 `sdk-core` 依赖残留
4. 确认无 `merman` 依赖残留
5. 确认无 `agent-client-protocol` 依赖残留
6. 确认 demo 案例遵循 `.rml` + `.rml.rs` MVVM 模式

***

## 实施顺序

1. **创建 ui-term crate**（Cargo.toml + 源文件迁移 + 移除 SDK）
2. **创建 ui-chat crate**（Cargo.toml + 源文件迁移 + 移除 SDK/ACP/merman + 自建类型）
3. **工作区集成**（根 Cargo.toml 添加 members + dependencies）
4. **Engine 注册 Terminal**（tags.rs + props\_registry.rs + compiler/components/terminal/）
5. **Engine 注册 Chat**（tags.rs + props\_registry.rs + compiler/components/chat/）
6. **创建 Demo 案例**（terminal\_case + chat\_case 的 .rml + .rml.rs）
7. **验证**（编译 + 测试 + 运行）

