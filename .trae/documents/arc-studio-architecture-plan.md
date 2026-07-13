# Arc Studio —— AI 原生 IDE 框架架构计划

## 1. 摘要

在项目根目录创建 `studio/` 独立 workspace，构建 Arc Studio AI 原生 IDE 框架。该框架基于 RML MVVM 声明式 UI + 贡献点扩展机制，打造面向 AI 时代的现代化工作台产品。核心设计原则：**AI 原生驱动、插件化扩展、深度交互探索、单一职责高内聚低耦合**。

## 2. 当前状态分析

### 2.1 RML 框架现有能力

| 能力          | 位置                                  | 说明                                                                                |
| ----------- | ----------------------------------- | --------------------------------------------------------------------------------- |
| MVVM 声明式 UI | engine + macros                     | `.rml` 模板 + `.rml.rs` ViewModel，`#[computed]`/`#[command]`                        |
| 贡献点体系       | core + app                          | `IContribution` → `IVisualContribution`，`#[contribute]` 宏 + build.rs 扫描 codegen   |
| 扩展自动发现      | engine/build/extension\_registry.rs | `[rml.metadata]` Cargo.toml 声明 → `cargo metadata` 扫描 → 动态注册                       |
| 工作台管理       | core/workbench.rs                   | `IWorkbenchManager` + `IWorkbenchProvider` + `IWorkbench`（Uri 路由）                 |
| 服务容器        | core/context.rs                     | `IAppContext`（IServiceProvider 风格），`ServiceCollection` 按 TypeId 索引                |
| 命令系统        | core/command.rs                     | `ICommand: IContribution`，`RelayCommand`（WPF 等价物），`CallContext`                   |
| 能力查询        | core/ability.rs                     | `VisualAbilityExt`/`CommandAbilityExt`/`ContributionAbilityExt` 等 trait upcast 扩展 |
| 应用启动器       | app/application.rs                  | `RmlApplication<W>` builder 模式，声明式 / 命令式双入口                                       |
| 现代窗口        | ui/window/modern\_window\.rs        | TitleBar + Menu + StatusBar + ActivityBar 组合                                      |
| 终端组件        | ui-term                             | 独立扩展 crate，`[rml.metadata]` 注册                                                    |
| Chat 组件     | ui/components/chat/                 | `IChatBackend` trait，流式响应，附件/工具调用                                                 |
| Tab 窗口      | ui/window/tab\_window\.rs           | `each` 指令 + `ObservableVec` 响应式 Tab 管理                                            |
| LSP 集成      | lsp crate                           | `LanguageClient` 统一启动，`ServerStatus` 通知                                           |

### 2.2 现有架构模式（Arc Studio 需遵循）

1. **扩展 crate 模式**：ui-term 是标准模板 —— 独立 crate，`[rml.metadata]` 声明组件，使用方只需添加依赖
2. **贡献注册流程**：`#[contribute(host_id, ...)]` 宏 → build.rs 扫描 → `register_rml_contributions_for(cx, host_id)` → `ContributionRegistry::register` → `IContributionHost::add`
3. **MVVM 铁律**：`.rml` 声明式模板 + `.rml.rs` ViewModel，禁止在 `.rml.rs` 中使用 Rust 链式 API
4. **单一职责**：一个 rs 文件 = 一个组件 / 一个职责，`mod.rs` 仅做 re-export
5. **命名规范**：接口 `I` 前缀，框架 crate 以 `rust-rml-*` 命名，产品 crate 在 `studio/` workspace 下以简洁名称命名（无冗余前缀）

## 3. 提案：Studio Workspace 结构

```
studio/                              # 独立 workspace
├── Cargo.toml                       # workspace 定义
├── README.md
├── crates/
│   ├── core/                        # 核心类型 + IDE 贡献点契约
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs               # re-export 聚合
│   │       ├── contribution.rs      # IEditorContribution, IProjectProvider, IDiagnosticProvider 等
│   │       ├── document.rs          # ITextDocument, ITextBuffer, TextModel trait
│   │       ├── project.rs           # IProject, IWorkspace, IFileSystem 接口
│   │       ├── ai.rs                # IAIProvider, IAIAssistant, IAICompletion trait
│   │       └── command.rs           # IDE 通用命令 ID 常量（command palette 用）
│   │
│   ├── shell/                       # IDE 外壳：工作台布局 + 面板管理
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── arc_workspace.rs     # ArcWorkspace ViewModel（主窗口，host_id="studio.workspace"）
│   │       ├── arc_workspace.rml     # 主窗口布局模板
│   │       ├── panels/
│   │       │   ├── mod.rs
│   │       │   ├── explorer_panel.rs      # 文件浏览器面板
│   │       │   ├── explorer_panel.rml
│   │       │   ├── outline_panel.rs       # 大纲/符号面板
│   │       │   ├── outline_panel.rml
│   │       │   ├── search_panel.rs        # 全局搜索面板
│   │       │   ├── search_panel.rml
│   │       │   ├── terminal_panel.rs      # 集成终端面板
│   │       │   └── terminal_panel.rml
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── file_commands.rs       # 文件操作命令（打开/保存/关闭）
│   │       │   ├── edit_commands.rs       # 编辑操作命令（撤销/重做/剪切/复制）
│   │       │   └── view_commands.rs       # 视图操作命令（切换面板/缩放）
│   │       └── layout.rs           # 面板布局引擎（拖拽分割、flex 比例持久化）
│   │
│   ├── editor/                      # 代码编辑器 + LSP 诊断
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── code_editor.rs       # 增强 CodeEditor 组件（断点、行号、diff 标记）
│   │       ├── code_editor.rml
│   │       ├── lsp_client.rs        # LSP 客户端（复用 rust-rml-lsp）
│   │       ├── diagnostics.rs       # 诊断收集器 + 行内展示
│   │       ├── hover.rs             # 悬停提示（类型信息、文档）
│   │       └── completion.rs        # 补全提供者（LSP + AI 混合）
│   │
│   ├── ai/                          # AI 功能模块
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chat/
│   │       │   ├── mod.rs
│   │       │   ├── chat_panel.rs        # AI 对话面板（复用 Chat 组件）
│   │       │   ├── chat_panel.rml
│   │       │   └── chat_context.rs      # 对话上下文管理（代码片段、文件引用）
│   │       ├── copilot/
│   │       │   ├── mod.rs
│   │       │   ├── inline_completion.rs # 行内代码补全（Ghost Text）
│   │       │   └── suggestion.rs        # 建议 UI 组件
│   │       ├── agent/
│   │       │   ├── mod.rs
│   │       │   ├── agent_runner.rs      # AI Agent 执行引擎
│   │       │   └── tool_provider.rs     # Agent 工具注册（文件操作、终端命令等）
│   │       └── provider/
│   │           ├── mod.rs
│   │           ├── ai_provider.rs       # IAIProvider trait 实现（OpenAI/Anthropic/本地）
│   │           └── config.rs           # AI 配置管理
│   │
│   ├── explorer/                    # 文件系统 + 项目浏览器
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── file_explorer.rs     # 文件树组件
│   │       ├── file_explorer.rml
│   │       ├── file_system.rs       # 虚拟文件系统抽象
│   │       └── project_model.rs     # 项目模型（Cargo.toml / package.json 解析）
│   │
│   ├── terminal/                    # 集成终端（封装 ui-term）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       └── terminal.rs          # 终端面板组件（多 Tab 终端管理）
│   │
│   ├── debug/                       # 调试器集成（封装 dap）
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── debug_panel.rs       # 调试面板（变量、调用栈、断点）
│   │       └── debug_session.rs     # 调试会话管理
│   │
│   ├── scm/                         # 版本控制集成
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── git_provider.rs      # Git 操作提供者
│   │       └── scm_panel.rs         # 源代码管理面板（变更列表、diff）
│   │
│   └── app/                         # 应用入口
│       ├── Cargo.toml
│       ├── build.rs
│       └── src/
│           ├── main.rs              # 启动入口
│           └── lib.rs               # 启动逻辑 + 扩展注册
```

## 4. 详细设计

### 4.1 Studio Workspace 独立化

**决策**：`studio/` 为独立 workspace，与 RML 框架 workspace 平级。

**理由**：

* 职责分离：RML 是通用 UI 框架，Arc Studio 是具体产品

* 独立版本：Studio 可独立发版，不受框架版本约束

* 清晰边界：框架 crate 以 `rust-rml-*` 命名，产品 crate 在 `studio/` workspace 下以简洁名称命名（无冗余前缀）

**Cargo.toml 结构**：

```toml
# studio/Cargo.toml
[workspace]
members = [
    "crates/core",
    "crates/shell",
    "crates/editor",
    "crates/ai",
    "crates/explorer",
    "crates/terminal",
    "crates/debug",
    "crates/scm",
    "crates/app",
]
resolver = "2"

[workspace.dependencies]
# RML 框架依赖（path 指向父 workspace）
rust-rml-core = { path = "../crates/core" }
rust-rml-engine = { path = "../crates/engine" }
rust-rml-ui = { path = "../crates/ui" }
rust-rml-ui-term = { path = "../crates/ui-term" }
rust-rml-app = { path = "../crates/app" }
rust-rml-lsp = { path = "../crates/lsp" }
rust-rml-dap = { path = "../crates/dap" }
rust-rml-client = { path = "../crates/rml" }
# ... 其他依赖
```

### 4.2 core：IDE 贡献点契约

**职责**：定义 Arc Studio 专属的 trait 接口和类型，不包含任何实现。

**关键接口**：

```rust
// crates/core/src/contribution.rs

/// 编辑器贡献点 —— 扩展编辑器功能（如断点、行内提示、代码镜头）
pub trait IEditorContribution: IContribution {
    fn decorate_line(&self, line: usize) -> Option<LineDecoration>;
    fn provide_code_lens(&self, document: &dyn ITextDocument) -> Vec<CodeLens>;
}

/// 诊断提供者 —— 注册诊断源（LSP、linter、AI 分析）
pub trait IDiagnosticProvider: IContribution {
    fn analyze(&self, document: &dyn ITextDocument) -> Vec<Diagnostic>;
    fn clear(&self, uri: &Uri);
}

/// 项目提供者 —— 注册项目类型（Rust、Python、Node 等）
pub trait IProjectProvider: IContribution {
    fn detect(&self, root: &Path) -> bool;
    fn create_project(&self, root: &Path) -> Arc<dyn IProject>;
}

/// 视图容器 —— 可停靠面板（类似 VS Code ViewContainer）
pub trait IViewContainer: IVisualContribution {
    fn container_id(&self) -> &str;  // "explorer" | "scm" | "debug" | "ai" | ...
    fn badge(&self) -> Option<usize>;  // 角标数字
}
```

**AI 接口**：

```rust
// crates/core/src/ai.rs

/// AI 提供者 —— 大模型后端抽象
pub trait IAIProvider: Send + Sync {
    fn chat(&self, request: &AIChatRequest) -> Result<AIChatResponse, AIError>;
    fn stream_chat(&self, request: &AIChatRequest, on_chunk: &dyn Fn(&AIStreamChunk)) -> Result<AIChatResponse, AIError>;
    fn completion(&self, request: &AICompletionRequest) -> Result<AICompletionResponse, AIError>;
    fn cancel(&self) -> Result<(), AIError>;
}

/// AI 助手 —— 对话式 AI 交互
pub trait IAIAssistant: IContribution {
    fn send_message(&self, message: &str, context: &AIContext);
    fn stream_message(&self, message: &str, context: &AIContext, on_event: &dyn Fn(&AIAssistantEvent));
    fn cancel(&self);
}

/// AI 工具 —— Agent 可调用的工具
pub trait IAITool: IContribution {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> serde_json::Value;
    fn execute(&self, params: serde_json::Value) -> Result<String, AIError>;
}
```

### 4.3 shell：工作台主窗口

**职责**：IDE 主窗口布局、面板管理、命令调度。

**设计要点**：

* `ArcWorkspace` 作为 `#[contributehost(id = "studio.workspace")]` 主 ViewModel

* 继承 ModernWindow 的 TitleBar/Menu/StatusBar 布局

* 面板区域采用经典 IDE 布局：\[ActivityBar] \[SideBar] \[Editor] \[SideBar/辅助面板]

* 面板通过 `IViewContainer` 贡献点注册，动态加载

* 命令面板（Ctrl+Shift+P）遍历所有 `ICommand` 贡献

**文件**：

* `arc_workspace.rs`：ViewModel，管理面板可见性、活动面板、布局比例

* `arc_workspace.rml`：声明式布局模板（each 指令迭代面板列表）

* `panels/`：各内置面板（Explorer、Search、Outline、Terminal）

* `commands/`：内置命令实现（文件、编辑、视图操作）

### 4.4 editor：代码编辑器

**职责**：增强 CodeEditor 组件，集成 LSP 诊断、补全、悬停提示。

**设计要点**：

* 复用 `rust-rml-ui` 的 CodeEditor 组件

* 通过 `#[contribute(visual, host_id = "studio.editor")]` 注册编辑器扩展

* LSP 客户端复用 `rust-rml-lsp`，增加诊断收集器

* 补全提供者支持 LSP + AI 双源混合

### 4.5 ai：AI 核心模块

**职责**：AI 对话、行内补全、Agent 执行引擎。

**设计要点**：

* 复用 `rust-rml-ui` 的 Chat 组件（`IChatBackend` trait）

* `IAIProvider` 抽象多个 AI 后端（OpenAI、Anthropic、本地模型）

* 行内补全采用 Ghost Text 模式（类似 GitHub Copilot）

* Agent 框架：`IAITool` 贡献点注册工具 → Agent 根据上下文选择工具 → 执行 → 反馈

### 4.6 扩展注册模式

每个 studio crate 作为 RML 扩展，通过 `[rml.metadata]` 声明组件：

```toml
# studio/crates/shell/Cargo.toml
[package.metadata.rml]
components = [
    { tag = "ArcWorkspace", ctor_path = "shell::ArcWorkspace", kind = "EntityRef", container = false },
]
```

贡献点注册通过 `#[contribute]` 宏：

```rust
// studio/crates/shell/src/panels/explorer_panel.rs
#[contribute(host_id = "studio.workspace", id = "explorer", order = 0, kind = "activity")]
#[component]
#[derive(Default)]
pub struct ExplorerPanel;

impl IContribution for ExplorerPanel {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { "Explorer".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("Files")) }
}
```

### 4.7 构建系统

每个 crate 的 `build.rs` 遵循 RML 标准模式：

```rust
// studio/crates/app/build.rs
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .assets("assets", true)
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

## 5. 实施阶段

### Phase 1：基础设施搭建（本次计划）

* [ ] 创建 `studio/` 目录和 workspace Cargo.toml

* [ ] 创建 `core` crate（IDE 贡献点契约 + 核心类型）

* [ ] 创建 `shell` crate（ArcWorkspace 主窗口骨架）

* [ ] 创建 `app` crate（入口 + build.rs）

* [ ] 验证 workspace 编译通过

### Phase 2：核心功能

* [ ] `editor`：增强代码编辑器 + LSP 集成

* [ ] `explorer`：文件浏览器 + 项目模型

* [ ] `terminal`：集成终端封装

### Phase 3：AI 集成

* [ ] `ai`：AI 对话 + 行内补全 + Agent 框架

* [ ] AI Provider 抽象 + 首个后端实现

### Phase 4：高级功能

* [ ] `debug`：调试器集成

* [ ] `scm`：版本控制面板

* [ ] 扩展市场 / 插件管理器

## 6. 关键决策

| 决策点           | 选择                                 | 理由                                                    |
| ------------- | ---------------------------------- | ----------------------------------------------------- |
| Workspace 独立性 | **独立 workspace**                   | 框架与产品职责分离，独立版本管理                                      |
| 命名规范          | **无冗余前缀**                          | 在 `studio/` 下直接使用简洁名称（`core`/`shell`/`editor`），符合用户要求 |
| 依赖方式          | `path` 指向父 workspace               | 开发阶段紧密耦合，后续可改为 git 依赖                                 |
| 扩展注册          | `[rml.metadata]` + `#[contribute]` | 复用现有成熟机制，无需新建                                         |
| MVVM 模式       | 严格遵循 `.rml` + `.rml.rs`            | 项目铁律，保证一致性                                            |
| AI 接口         | `IAIProvider` trait 抽象             | 支持多后端切换，面向接口编程                                        |
| 面板系统          | `IViewContainer` 贡献点               | 插件化注册，动态加载                                            |

## 7. 验证步骤

1. `cd studio && cargo check` —— workspace 编译通过
2. `cargo build -p app` —— 应用入口编译通过
3. `cargo test -p core` —— 核心接口测试通过
4. 验证 `#[contribute]` 宏扫描正确生成 `rml_contributions.rs`
5. 验证 `[rml.metadata]` 扩展组件自动发现注册

