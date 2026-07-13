# Arc Studio 实施计划

> 本文档是 Arc Studio IDE 的**待实施计划**,不是架构演进记录。所有内容面向实施,decision-complete。

## 1. 摘要

基于 rust-gpui-rml 框架构建 Arc Studio IDE。引入 [rust-dix](https://crates.io/crates/rust-dix) DI 容器,面向接口开发,公共接口注册进全局容器。

### 核心契约

| 契约 | 归属 | 职责 |
|------|------|------|
| `IWorkbenchComponent` | studio/core | 工作台内部呈现组件贡献点(编辑/预览/设计多态) |
| `IEditorCommand` | studio/core | 编辑器命令,扩展 `ICommand` 添加 `gesture()` |
| `TextPosition` / `TextSpan` | studio/core | 文本位置(点)与文本块(范围) |
| `IWorktree` | studio/core | 文件系统抽象,AI 并行编程依赖此能力 |
| `IWorkspace` / `IWorkspaceManager` | studio/core | IDE 工作空间(多根目录)管理 |
| `IEditorLanguage` | studio/editor | 统一语言服务接入 |

### 框架改动

**无**。`IWorkbench` trait 维持 `IContribution + IVisual`,框架层不强制 host 语义。工作台是否受理子贡献由**实现决定**——`EditorWorkbench` 直接 `impl IContributionHost` override `add()` 受理 `IWorkbenchComponent`,其他工作台(如 demo 的 `CaseWorkbench`/`LspWorkbench`)不 impl 即不受理。这与 `IContributionHost` 所有方法有默认空实现的设计一致。

## 2. 架构总览

### 2.1 DI 容器集成(rust-dix)

引入 rust-dix v0.6.0 作为全局 DI 容器。**公共接口注册进容器,面向接口开发**。

```
startup
  → ServiceCollection::new()
       .singleton::<dyn IWorkbenchManager>(factory)      —— ArcShellManager
       .singleton::<dyn IWorkspaceManager>(factory)      —— ArcShellManager(同一实例)
       .keyed_singleton::<dyn IWorkbenchProvider>("file", factory)
       .keyed_singleton::<dyn IEditorLanguage>("rust", factory)
       .keyed_singleton::<dyn IChatBackend>("openai", factory)
       .build() → ServiceProvider
  → cx.set_service(Arc<ServiceProvider>)
```

**解析方式**: `cx.get_service::<Arc<ServiceProvider>>()?.get::<dyn IWorkbenchManager>()`

### 2.2 逻辑/UI 分离

| 组件 | 类型 | 注册位置 | 说明 |
|------|------|----------|------|
| `ArcShellManager` | 纯逻辑 struct | DI singleton | impl IWorkbenchManager + IWorkspaceManager,无 GPUI 依赖 |
| `ArcShell` | GPUI Entity(`#[window]`) | GPUI Entity 系统 | 持有 `Arc<ArcShellManager>`,处理 UI 渲染 |
| `EditorWorkbenchProvider` | 纯逻辑 struct | DI keyed | impl IWorkbenchProvider,按 schema 路由 |
| `EditorWorkbench` | GPUI Entity | 由 Provider 创建 | impl IWorkbench + IContributionHost |
| `ExplorerPanel` | GPUI Entity | `#[contribute]` | impl IActivityPanel |
| `CodeEditorComponent` | GPUI Entity | 由 Provider 创建 | impl IWorkbenchComponent |
| `RustRmlLanguage` | 纯逻辑 struct | DI keyed | impl IEditorLanguage |
| `OpenAIBackend` | 纯逻辑 struct | DI keyed | impl IChatBackend |
| `LocalWorktree` | 纯逻辑 struct | DI singleton | impl IWorktree + IWorkspace |

### 2.3 概念层级

```
ArcShell（IDE 主窗口, GPUI #[window]）
  ├─ Arc<ArcShellManager>（DI singleton）
  │    ├─ impl IWorkbenchManager  —— 资源会话管理(Tab 打开/关闭/激活)
  │    │    └─ Vec<Arc<dyn IWorkbench>>  —— 已打开资源会话
  │    │         └─ EditorWorkbench      —— 编辑器资源会话
  │    │              ├─ impl IContributionHost  —— 受理 IWorkbenchComponent
  │    │              └─ Vec<Arc<dyn IContribution>>  —— 组件(Code/Design/Preview)
  │    └─ impl IWorkspaceManager  —— 工作空间管理(多根目录)
  │         └─ Vec<Arc<dyn IWorkspace>>  —— 已打开工作空间
  │              └─ impl IWorktree  —— 文件系统抽象
  │
  └─ UI 渲染(TabWindow + ActivityBar + StatusBar, 复刻 demo)
```

### 2.4 单一出入口

| 流程 | 唯一入口 | 经由 |
|------|----------|------|
| 打开资源(Tab) | `IWorkbenchManager::open(uri)` | DI 解析 provider → `IWorkbenchProvider::render(uri)` |
| 添加工作空间 | `IWorkspaceManager::add(ws)` | 创建 `LocalWorktree` |
| 文件枚举 | `IWorktree::entries(dir)` | `ExplorerPanel` 调用 |
| 文件读取 | `IWorktree::read(path)` | `EditorWorkbenchProvider` 调用 |
| 文件写入 | `IWorktree::write(path, content)` | AI / FormatCommand 调用 |
| 语言服务 | `IEditorLanguage::install(state, uri, text)` | `CodeEditorComponent` 调用 |
| AI 对话 | `IChatBackend::send()` / `stream()` | `ChatPanel` 调用 |
| 视图切换 | `EditorWorkbench::switch(component_id)` | 激活 `IWorkbenchComponent` |
| 编辑器命令 | `IEditorCommand::execute(ctx)` | gesture 触发或命令面板 |

## 3. Crate 结构

```
studio/
├── Cargo.toml
├── crates/
│   ├── core/                     # 纯契约(依赖 rml_core + std)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ability_ext.rs    # 集中:所有 *AbilityExt + register_*_ability
│   │       ├── component.rs      # IWorkbenchComponent + TextPosition + TextSpan + EditorContext
│   │       ├── command.rs        # IEditorCommand(gesture)
│   │       ├── worktree.rs       # IWorktree + WorktreeEntry + WorktreeChange + WorktreeStat
│   │       └── workspace.rs      # IWorkspace + IWorkspaceManager
│   ├── shell/                    # 主窗口(依赖 rust-dix + rml_ui)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── di.rs             # DI 容器构建(ServiceCollection → ServiceProvider)
│   │       ├── shell_manager.rs  # ArcShellManager(IWorkbenchManager + IWorkspaceManager)
│   │       ├── arc_shell.rs   # ArcShell(GPUI #[window], 持有 Arc<ArcShellManager>)
│   │       └── arc_shell.rml  # TabWindow 布局(复刻 demo)
│   ├── editor/                   # 编辑器(依赖 rml_ui + rust_rml_client)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── language.rs       # IEditorLanguage trait
│   │       ├── editor_workbench.rs    # EditorWorkbench(IWorkbench + IContributionHost)
│   │       ├── provider.rs       # EditorWorkbenchProvider(IWorkbenchProvider)
│   │       ├── components/
│   │       │   ├── mod.rs
│   │       │   ├── code_editor.rs     # CodeEditorComponent(IWorkbenchComponent)
│   │       │   └── rml_design.rs      # RmlDesignComponent(IWorkbenchComponent)
│   │       ├── languages/
│   │       │   └── rust_rml.rs        # RustRmlLanguage(IEditorLanguage)
│   │       └── commands/
│   │           └── format.rs          # FormatCommand(IEditorCommand)
│   ├── explorer/                 # 文件浏览器
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── local_worktree.rs      # LocalWorktree(IWorktree + IWorkspace)
│   │       └── explorer_panel.rs      # ExplorerPanel(IActivityPanel)
│   └── ai/                       # AI 模块
│       └── src/
│           ├── lib.rs
│           ├── provider/
│           │   └── openai.rs         # impl IChatBackend
│           ├── chat_panel.rs         # 复用 rml_ui::ChatPanel
│           ├── copilot.rs            # 行内补全
│           └── agent.rs              # Agent 工具注册
```

## 4. Core 契约(studio/crates/core)

### 4.1 集中能力扩展(ability_ext.rs)

**设计**: 所有 `*AbilityExt` trait 与 `register_*_ability` 函数集中在一个文件,避免分散。

```rust
//! 集中能力扩展 —— 所有 studio 子 trait 的能力查询与注册。
//!
//! 对齐 rml_core 模式:每个子 trait 配套 AbilityExt + register 函数,
//! 经 rml_core::ability::query/register/erase/restore 实现类型擦除能力查询。

use std::any::Any;
use rml_core::value::IValue;
use crate::component::IWorkbenchComponent;
use crate::command::IEditorCommand;
use crate::worktree::IWorktree;
use crate::workspace::IWorkspace;

// ── IWorkbenchComponent ──
pub trait WorkbenchComponentAbilityExt {
    fn as_workbench_component(&self) -> Option<&dyn IWorkbenchComponent>;
}
#[allow(unsafe_code)]
impl WorkbenchComponentAbilityExt for dyn IValue {
    fn as_workbench_component(&self) -> Option<&dyn IWorkbenchComponent> {
        let erased = rml_core::ability::query::<dyn IWorkbenchComponent>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkbenchComponent>(erased) })
    }
}
#[allow(unsafe_code)]
pub fn register_workbench_component_ability<T: IWorkbenchComponent + 'static>() {
    rml_core::ability::register::<T, dyn IWorkbenchComponent>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let v: &dyn IWorkbenchComponent = s;
            unsafe { rml_core::ability::erase(v) }
        })
    });
}

// ── IEditorCommand ──
pub trait EditorCommandAbilityExt {
    fn as_editor_command(&self) -> Option<&dyn IEditorCommand>;
}
#[allow(unsafe_code)]
impl EditorCommandAbilityExt for dyn IValue {
    fn as_editor_command(&self) -> Option<&dyn IEditorCommand> {
        let erased = rml_core::ability::query::<dyn IEditorCommand>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IEditorCommand>(erased) })
    }
}
#[allow(unsafe_code)]
pub fn register_editor_command_ability<T: IEditorCommand + 'static>() {
    rml_core::ability::register::<T, dyn IEditorCommand>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let cmd: &dyn IEditorCommand = s;
            unsafe { rml_core::ability::erase(cmd) }
        })
    });
}

// ── IWorktree ──
pub trait WorktreeAbilityExt {
    fn as_worktree(&self) -> Option<&dyn IWorktree>;
}
#[allow(unsafe_code)]
impl WorktreeAbilityExt for dyn IValue {
    fn as_worktree(&self) -> Option<&dyn IWorktree> {
        let erased = rml_core::ability::query::<dyn IWorktree>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorktree>(erased) })
    }
}
#[allow(unsafe_code)]
pub fn register_worktree_ability<T: IWorktree + 'static>() {
    rml_core::ability::register::<T, dyn IWorktree>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let wt: &dyn IWorktree = s;
            unsafe { rml_core::ability::erase(wt) }
        })
    });
}

// ── IWorkspace ──
pub trait WorkspaceAbilityExt {
    fn as_workspace(&self) -> Option<&dyn IWorkspace>;
}
#[allow(unsafe_code)]
impl WorkspaceAbilityExt for dyn IValue {
    fn as_workspace(&self) -> Option<&dyn IWorkspace> {
        let erased = rml_core::ability::query::<dyn IWorkspace>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkspace>(erased) })
    }
}
#[allow(unsafe_code)]
pub fn register_workspace_ability<T: IWorkspace + 'static>() {
    rml_core::ability::register::<T, dyn IWorkspace>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let ws: &dyn IWorkspace = s;
            unsafe { rml_core::ability::erase(ws) }
        })
    });
}
```

### 4.2 IWorkbenchComponent + 文本类型(component.rs)

```rust
//! 工作台组件契约 + 文本位置/块类型。

use gpui::SharedString;
use rml_core::contribution::IVisualContribution;
use rml_core::workbench::Uri;

/// 工作台呈现组件 —— IWorkbench 实现内部的贡献点。
///
/// 继承 `IVisualContribution`(具备 `id`/`name`/`description`/`icon` + `render`)。
/// 此 trait 为空标记 —— 仅用于能力查询区分"此视觉贡献是工作台组件"。
///
/// # 应用场景
///
/// `EditorWorkbench`(IWorkbench 实现 + IContributionHost)受理多个 IWorkbenchComponent:
/// - `CodeEditorComponent`(id="code")—— 代码编辑,默认内置
/// - `RmlDesignComponent`(id="design")—— RML 可视化设计器,仅 .rml
/// - `PreviewComponent`(id="preview")—— 只读预览,仅 Markdown/HTML
///
/// 用户在组件间切换,实现编辑/预览/设计多态呈现。
///
/// # 元数据来源(无冗余)
///
/// - `IContribution::id()` → 组件标识("code"/"design"/"preview")
/// - `IContribution::name()` → 切换按钮标签
/// - `IContribution::icon()` → 切换按钮图标
/// - `IVisual::render()` → 组件视图内容
pub trait IWorkbenchComponent: IVisualContribution {}

/// 文本位置 —— 行/列从 0 开始。
///
/// 用于 `EditorContext::cursor` 与 `TextSpan` 的起止点。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub character: usize,
}

/// 文本块 —— 位置范围,表示一段连续文本。
///
/// 用于 `EditorContext::selection`、LSP Range、AI 上下文选区。
/// `start` 必须 ≤ `end`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextSpan {
    pub start: TextPosition,
    pub end: TextPosition,
}

/// 编辑器命令上下文 —— 经 `CallContext::parameter` 传入 `ICommand::execute` / `can_execute`。
///
/// `can_execute` 据此判断命令可用性(替代 v1 的 `when()` 方法)。
#[derive(Debug, Clone)]
pub struct EditorContext {
    pub uri: Uri,
    pub cursor: Option<TextPosition>,
    pub selection: Option<TextSpan>,
}
```

### 4.3 IEditorCommand(command.rs)

```rust
//! 编辑器命令契约。

use gpui::SharedString;
use rml_core::command::ICommand;

/// 编辑器命令 —— 扩展 `ICommand`,仅添加手势(快捷键)声明。
///
/// `can_execute(ctx)` 已由 `ICommand` 提供,命令实现通过 `ctx.parameter` 的
/// `EditorContext` downcast 判断可用性,无需 `when()` 方法。
///
/// # 应用场景
///
/// - `FormatCommand` → gesture="Shift+Alt+F", can_execute 检查 EditorContext 存在
/// - `RenameCommand` → gesture="F2", can_execute 检查 cursor 存在
/// - `GoToDefinitionCommand` → gesture="F12", can_execute 检查 cursor 存在
pub trait IEditorCommand: ICommand {
    /// 手势绑定(键盘快捷键 "Shift+Alt+F" / "F12")。
    /// None = 无默认快捷键,仅经命令面板触发。
    /// 命名 gesture 而非 keybinding:未来可扩展鼠标/触摸手势。
    fn gesture(&self) -> Option<SharedString> { None }
}
```

### 4.4 IWorktree —— 文件系统抽象(worktree.rs)

**职责明确**: 文件枚举、路径解析、读写、元数据、变更监听。AI 并行编程依赖此能力。

```rust
//! 工作区文件系统抽象契约。

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use gpui::SharedString;
use rml_core::contribution::IContribution;
use rml_core::value::IValue;
use rml_core::workbench::Uri;

/// 文件树条目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

/// 文件树条目。
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// 相对于工作区根的路径。
    pub path: PathBuf,
    pub kind: EntryKind,
    pub name: SharedString,
}

/// 文件元数据。
#[derive(Debug, Clone)]
pub struct WorktreeStat {
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// 文件变更事件。
#[derive(Debug, Clone)]
pub enum WorktreeChange {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// 文件系统抽象 —— 文件枚举、路径解析、读写、元数据、变更监听。
///
/// 继承 `IContribution`(`id`/`name` 标识 fs 类型)。
/// 一个 `IWorkspace` 拥有一个 `IWorktree` 实例。
///
/// # 职责清单(AI 并行编程依赖)
///
/// | 方法 | UI 场景 | AI 场景 |
/// |------|---------|---------|
/// | `root()` | 文件树根节点 | 工作空间定位 |
/// | `entries(dir)` | ExplorerPanel 渲染文件树 | AI 扫描代码库结构 |
/// | `resolve(path)` | 点击文件 → Uri → 打开 Tab | AI 引用文件 |
/// | `relativize(uri)` | Tab 标题显示相对路径 | AI 输出相对路径 |
/// | `read(path)` | EditorWorkbench 打开文件 | AI 理解代码内容 |
/// | `write(path, content)` | FormatCommand 保存 | **AI 应用代码修改** |
/// | `stat(path)` | ExplorerPanel 显示大小/时间 | AI 检查文件大小 |
/// | `watch(on_change)` | ExplorerPanel 自动刷新 | AI 检测外部修改 |
pub trait IWorktree: IContribution {
    /// 工作区根路径。
    fn root(&self) -> &Path;

    /// 枚举指定目录下的条目(None = 根目录)。
    fn entries(&self, dir: Option<&Path>) -> Vec<WorktreeEntry>;

    /// 将相对路径解析为 Uri。
    fn resolve(&self, path: &Path) -> Uri;

    /// 将 Uri 还原为相对路径(`resolve` 的逆运算)。
    fn relativize(&self, uri: &Uri) -> Option<PathBuf>;

    /// 读取文件内容。
    fn read(&self, path: &Path) -> std::io::Result<String>;

    /// 写入文件内容(AI 应用修改、FormatCommand 保存)。
    fn write(&self, path: &Path, content: &str) -> std::io::Result<()>;

    /// 获取文件元数据。
    fn stat(&self, path: &Path) -> std::io::Result<WorktreeStat>;

    /// 订阅文件变更。返回取消订阅句柄(调用即停止监听)。
    fn watch(&self, on_change: Arc<dyn Fn(&WorktreeChange) + Send + Sync>) -> Box<dyn FnOnce()>;
}
```

### 4.5 IWorkspace + IWorkspaceManager(workspace.rs)

```rust
//! IDE 工作空间管理契约。

use std::any::Any;
use std::path::Path;
use std::sync::Arc;

use rml_core::value::IValue;

use crate::worktree::IWorktree;

/// IDE 工作空间 —— 一个已注册的根目录。
///
/// 继承 `IWorktree`(空标记 trait)—— 工作空间即是一个 worktree,
/// 额外标记"此 worktree 已被 IDE 注册为工作空间"。
///
/// # 应用场景
///
/// 用户在资源管理器中"打开文件夹" → 创建 `LocalWorktree` → 注册为 `IWorkspace` →
/// `ExplorerPanel` 经 `IWorkspaceManager::list()` 渲染多根文件树。
/// 每个根目录独立显示为文件树顶级节点(VSCode 多根工作空间模式)。
pub trait IWorkspace: IWorktree {}

/// 工作空间管理器 —— 管理多个已打开的工作空间。
///
/// 镜像 `IWorkbenchManager` 模式。`ArcShellManager` 直接 impl 此 trait。
/// 注册为 DI singleton,经 `provider.get::<dyn IWorkspaceManager>()` 解析。
pub trait IWorkspaceManager: Send + Sync + 'static {
    /// 添加工作空间;若根路径已存在则忽略。
    fn add(&self, workspace: Arc<dyn IWorkspace>);

    /// 移除工作空间(按根路径)。
    fn remove(&self, root: &Path);

    /// 当前所有工作空间。
    fn list(&self) -> Vec<Arc<dyn IWorkspace>>;

    /// 按根路径获取工作空间。
    fn get(&self, root: &Path) -> Option<Arc<dyn IWorkspace>>;
}
```

## 5. 框架改动(rml_core)

**无框架改动**。`IWorkbench` trait 维持 `IContribution + IVisual`,框架层不强制 host 语义。

工作台是否受理子贡献由**实现决定**——这与 `IContributionHost` 所有方法有默认空实现的设计一致:

- `EditorWorkbench`(Phase 4)直接 `impl IContributionHost for EditorWorkbench` override `add()` 受理 `IWorkbenchComponent`,实现编辑/预览/设计多态呈现。
- demo 的 `CaseWorkbench`/`LspWorkbench` 不 impl `IContributionHost`,不受理子组件。

框架层 `IWorkbench` 定义保持不变:

```rust
pub trait IWorkbench: IContribution + IVisual {
    fn uri(&self) -> &str;
    fn close(&self);
    fn activate(&self);
    fn set(&self, key: SharedString, value: Box<dyn Any + Send + Sync>);
    fn closable(&self) -> bool { true }
}
```

## 6. Shell(studio/crates/shell)

### 6.1 DI 容器构建(di.rs)

```rust
use std::sync::Arc;
use rust_dix::*;
use rml_core::workbench::{IWorkbenchManager, IWorkbenchProvider};
use studio_core::workspace::IWorkspaceManager;
use studio_editor::language::IEditorLanguage;
use rml_ui::components::chat::IChatBackend;

use crate::shell_manager::ArcShellManager;

/// 构建 DI 容器,注册所有公共接口。
pub fn build_provider() -> anyhow::Result<ServiceProvider> {
    let manager = Arc::new(ArcShellManager::new());

    let collection = ServiceCollection::new()
        // 管理器(singleton,同一实例实现两个 trait)
        .singleton::<dyn IWorkbenchManager>(move |_| {
            manager.clone() as Arc<dyn IWorkbenchManager>
        })
        .singleton::<dyn IWorkspaceManager>(move |_| {
            manager.clone() as Arc<dyn IWorkspaceManager>
        })
        // 工作台 provider(keyed by schema)
        .keyed_singleton::<dyn IWorkbenchProvider>("file", |_| {
            Arc::new(EditorWorkbenchProvider::new()) as Arc<dyn IWorkbenchProvider>
        })
        // 语言服务(keyed by language_id)
        .keyed_singleton::<dyn IEditorLanguage>("rust", |_| {
            Arc::new(RustRmlLanguage::new()?) as Arc<dyn IEditorLanguage>
        })
        // AI 后端(keyed by provider name)
        .keyed_singleton::<dyn IChatBackend>("openai", |_| {
            Arc::new(OpenAIBackend::from_env()?) as Arc<dyn IChatBackend>
        })
        .build()?;

    Ok(collection)
}
```

### 6.2 ArcShellManager —— 纯逻辑(shell_manager.rs)

```rust
use std::sync::{Arc, RwLock};
use rml_core::observable::ObservableVec;
use rml_core::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};
use studio_core::workspace::{IWorkspace, IWorkspaceManager};
use rust_dix::ServiceProvider;

/// 纯逻辑管理器 —— 无 GPUI 依赖,可注册进 DI 容器。
///
/// 同时 impl IWorkbenchManager + IWorkspaceManager。
/// ArcShell(GPUI Entity)持有 Arc<ArcShellManager> 用于 UI 渲染。
pub struct ArcShellManager {
    provider: Arc<ServiceProvider>,
    workbenches: ObservableVec<Arc<dyn IWorkbench>>,
    activated: RwLock<Option<Arc<dyn IWorkbench>>>,
    workspaces: RwLock<Vec<Arc<dyn IWorkspace>>>,
}

impl ArcShellManager {
    pub fn new(provider: Arc<ServiceProvider>) -> Self {
        Self {
            provider,
            workbenches: ObservableVec::new(),
            activated: RwLock::new(None),
            workspaces: RwLock::new(Vec::new()),
        }
    }
}

impl IWorkbenchManager for ArcShellManager {
    fn open(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        // 去重
        let uri_str = uri.as_str();
        if let Some(wb) = self.workbenches.snapshot().into_iter().find(|w| w.uri() == uri_str) {
            *self.activated.write().unwrap() = Some(wb.clone());
            return Some(wb);
        }
        // 路由:schema → DI keyed provider
        let schema = uri.scheme();
        let provider: Arc<dyn IWorkbenchProvider> = self.provider.get_keyed(schema).ok()?;
        let wb = provider.render(uri);
        self.workbenches.push(wb.clone());
        *self.activated.write().unwrap() = Some(wb.clone());
        Some(wb)
    }

    fn close(&self, uri: &Uri) {
        let uri_str = uri.as_str();
        self.workbenches.retain(|w| w.uri() != uri_str);
        let mut activated = self.activated.write().unwrap();
        if activated.as_ref().map(|w| w.uri() == uri_str).unwrap_or(false) {
            *activated = self.workbenches.snapshot().into_iter().next();
        }
    }

    fn get_all(&self) -> Vec<Arc<dyn IWorkbench>> { self.workbenches.snapshot() }
    fn get_activated(&self) -> Option<Arc<dyn IWorkbench>> { self.activated.read().unwrap().clone() }
    fn get(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
        let uri_str = uri.as_str();
        self.workbenches.snapshot().into_iter().find(|w| w.uri() == uri_str)
    }
}

impl IWorkspaceManager for ArcShellManager {
    fn add(&self, workspace: Arc<dyn IWorkspace>) {
        let root = workspace.root().to_path_buf();
        let mut ws = self.workspaces.write().unwrap();
        if !ws.iter().any(|w| w.root() == root) {
            ws.push(workspace);
        }
    }
    fn remove(&self, root: &Path) {
        self.workspaces.write().unwrap().retain(|w| w.root() != root);
    }
    fn list(&self) -> Vec<Arc<dyn IWorkspace>> { self.workspaces.read().unwrap().clone() }
    fn get(&self, root: &Path) -> Option<Arc<dyn IWorkspace>> {
        self.workspaces.read().unwrap().iter().find(|w| w.root() == root).cloned()
    }
}
```

### 6.3 ArcShell —— GPUI 主窗口(arc_shell.rs + .rml)

复刻 [demo main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) 模式,持有 `Arc<ArcShellManager>`。

```rust
#[window]
#[contributehost(id = "studio.shell")]
pub struct ArcShell {
    manager: Arc<ArcShellManager>,
    // 类型化集合(从贡献 entries 投影)
    menus: Vec<MenuViewModel>,
    status: Vec<StatusViewModel>,
    activities: Vec<Arc<dyn IActivityPanel>>,
    // Tab 状态
    show_chrome: bool,
    slot_left_size: gpui::Pixels,
    // 框架仪式
    activity_bar: Option<gpui::Entity<ActivityBar>>,
    entries: Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>,
}
```

**布局**(arc_shell.rml): 与 [demo main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) 一致 —— `<tab-window>` + slots(left/menu/bottom/footer)。

**初始化流程**(`on_loaded`):
1. 构建 DI 容器 → `cx.set_service(Arc<ServiceProvider>)`
2. 创建 `ArcShellManager` → 注册为 DI singleton
3. 通道桥接(ObservableVec → flume → cx.notify)
4. 注册 contributehost → 触发 `#[contribute]` 批量注册
5. 初始化默认工作空间(当前目录 → `LocalWorktree` → `IWorkspaceManager::add`)
6. 构建 ActivityBar
7. observe panels + i18n

## 7. Editor(studio/crates/editor)

### 7.1 IEditorLanguage(language.rs)

```rust
use std::sync::Arc;
use gpui::SharedString;
use gpui_component::input::InputState;
use lsp_types::Uri;
use rml_core::contribution::IContribution;
use rust_rml_client::LanguageClient;

/// 编辑器语言服务 —— 统一语言能力接入贡献点。
///
/// 语言能力(补全/悬停/定义/诊断/语义高亮/格式化)统一由底层
/// `LanguageClient` 驱动,经此 trait 暴露安装入口。
///
/// 注册为 DI keyed service,按 `language_id` 索引。
pub trait IEditorLanguage: IContribution {
    fn language_id(&self) -> SharedString;
    fn file_extensions(&self) -> Vec<SharedString>;
    /// 安装语言能力:open_document + install_providers(两步合一)。
    fn install(&self, state: &mut InputState, uri: &Uri, text: &str);
    /// 直访底层客户端(formatting/rename/references 等高级能力)。
    /// None = 无 LSP(纯本地解析语言)。
    fn client(&self) -> Option<&LanguageClient> { None }
}
```

### 7.2 EditorWorkbench —— IWorkbench + IContributionHost(editor_workbench.rs)

```rust
use std::sync::{Arc, RwLock};
use gpui::{AnyElement, App, SharedString, Window};
use rml_core::contribution::{
    ContributionOptions, IContribution, IContributionHost, IVisual,
    ContributionAbilityExt,
};
use rml_core::workbench::IWorkbench;
use rml_core::value::IValue;
use studio_core::ability_ext::WorkbenchComponentAbilityExt;
use studio_core::component::IWorkbenchComponent;

/// 编辑器资源会话 —— IWorkbench 实现 + IContributionHost(受理 IWorkbenchComponent)。
///
/// IWorkbench trait 本身不继承 IContributionHost(框架层不强制 host 语义)。
/// EditorWorkbench 作为**实现选择**直接 `impl IContributionHost`,override `add()`
/// 受理 IWorkbenchComponent,实现编辑/预览/设计多态呈现。其他工作台(如 demo 的
/// CaseWorkbench)不 impl IContributionHost 即不受理子组件。
pub struct EditorWorkbench {
    uri: Uri,
    title: SharedString,
    language: Option<Arc<dyn IEditorLanguage>>,
    components: RwLock<Vec<Arc<dyn IContribution>>>,
    active_component_id: RwLock<Option<SharedString>>,
}

impl EditorWorkbench {
    pub fn new(
        uri: Uri,
        title: SharedString,
        components: Vec<Arc<dyn IWorkbenchComponent>>,
        language: Option<Arc<dyn IEditorLanguage>>,
    ) -> Self {
        let first_id = components.first().map(|c| SharedString::from(c.id()));
        Self {
            uri, title, language,
            components: RwLock::new(components.into_iter().map(|c| c as Arc<dyn IContribution>).collect()),
            active_component_id: RwLock::new(first_id),
        }
    }

    /// 切换激活组件(按 IContribution::id 匹配)。
    pub fn switch(&self, component_id: &str) {
        *self.active_component_id.write().unwrap() = Some(component_id.into());
    }

    pub fn active_component(&self) -> Option<Arc<dyn IContribution>> {
        let components = self.components.read().unwrap();
        let active_id = self.active_component_id.read().unwrap().clone();
        components.into_iter()
            .find(|c| Some(SharedString::from(c.id())) == active_id)
            .or_else(|| components.first().cloned())
    }
}

impl IContribution for EditorWorkbench {
    fn id(&self) -> &str { self.uri.as_str() }
    fn name(&self) -> SharedString { self.title.clone() }
}

impl IVisual for EditorWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        if let Some(component) = self.active_component() {
            if let Some(visual) = component.as_visual() {
                return visual.render(window, cx);
            }
        }
        gpui::div().into_any_element()
    }
}

impl IWorkbench for EditorWorkbench {
    fn uri(&self) -> &str { self.uri.as_str() }
    fn close(&self) { /* 清理 LSP 文档 */ }
    fn activate(&self) { /* 聚焦编辑器 */ }
    fn set(&self, _key: SharedString, _value: Box<dyn std::any::Any + Send + Sync>) {}
}

/// IContributionHost override —— 仅受理 IWorkbenchComponent。
impl IContributionHost for EditorWorkbench {
    fn id(&self) -> &'static str { "studio.editor.workbench" }
    fn add(&self, contribution: Arc<dyn IContribution>, _options: Option<ContributionOptions>) {
        if contribution.as_workbench_component().is_some() {
            self.components.write().unwrap().push(contribution);
        }
    }
    fn remove(&self, contribution_id: &str) {
        self.components.write().unwrap().retain(|c| c.id() != contribution_id);
    }
}
```

### 7.3 EditorWorkbenchProvider(provider.rs)

```rust
use std::sync::Arc;
use gpui::SharedString;
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri};
use rust_dix::ServiceProvider;

use crate::editor_workbench::EditorWorkbench;
use crate::components::{code_editor::CodeEditorComponent, rml_design::RmlDesignComponent};
use studio_core::component::IWorkbenchComponent;

pub struct EditorWorkbenchProvider {
    provider: Arc<ServiceProvider>,
}

impl EditorWorkbenchProvider {
    pub fn new(provider: Arc<ServiceProvider>) -> Self {
        Self { provider }
    }

    fn match_language(&self, uri: &Uri) -> Option<Arc<dyn IEditorLanguage>> {
        let ext = uri.as_str().rsplit('.').next()?;
        // 遍历所有已注册的 IEditorLanguage,匹配扩展名
        for lang_id in ["rust", "rml"] {
            if let Ok(lang) = self.provider.get_keyed::<dyn IEditorLanguage>(lang_id) {
                if lang.file_extensions().iter().any(|e| e == ext) {
                    return Some(lang);
                }
            }
        }
        None
    }

    fn build_components(&self, uri: &Uri) -> Vec<Arc<dyn IWorkbenchComponent>> {
        let mut components: Vec<Arc<dyn IWorkbenchComponent>> = vec![
            Arc::new(CodeEditorComponent::new(uri.clone())),
        ];
        if uri.as_str().ends_with(".rml") {
            components.push(Arc::new(RmlDesignComponent::new(uri.clone())));
        }
        components
    }
}

impl IWorkbenchProvider for EditorWorkbenchProvider {
    fn schema(&self) -> SharedString { "file".into() }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        let language = self.match_language(uri);
        let components = self.build_components(uri);
        let title = uri.as_str().rsplit('/').next().unwrap_or("unknown").into();
        Arc::new(EditorWorkbench::new(uri.clone(), title, components, language))
    }
}
```

### 7.4 CodeEditorComponent —— 默认 IWorkbenchComponent

```rust
/// 代码编辑组件 —— IWorkbenchComponent 默认内置实现。
///
/// 复用 rml_ui::CodeEditor,经 IEditorLanguage::install 注入 LSP 能力。
/// 懒加载 CodeEditorTab Entity(首次 render 时创建)。
///
/// 元数据:id="code", name="Code", icon=IconSpec::Named("Code")
pub struct CodeEditorComponent {
    uri: Uri,
    tab: RwLock<Option<Entity<CodeEditorTab>>>,
}

impl CodeEditorComponent {
    pub fn new(uri: Uri) -> Self {
        Self { uri, tab: RwLock::new(None) }
    }
}

impl IContribution for CodeEditorComponent {
    fn id(&self) -> &str { "code" }
    fn name(&self) -> SharedString { "Code".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("Code")) }
}

impl IVisual for CodeEditorComponent {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        // 首次 render 时创建 CodeEditorTab + IEditorLanguage::install
        // 对齐 demo CodeEditorTab::new 模式
        todo!()
    }
}

impl IWorkbenchComponent for CodeEditorComponent {}
```

### 7.5 RmlDesignComponent —— 自定义 IWorkbenchComponent

```rust
/// RML 可视化设计组件 —— 针对 .rml 资源定制。
///
/// 元数据:id="design", name="Design", icon=IconSpec::Named("LayoutPanel")
pub struct RmlDesignComponent {
    uri: Uri,
}

impl IContribution for RmlDesignComponent {
    fn id(&self) -> &str { "design" }
    fn name(&self) -> SharedString { "Design".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("LayoutPanel")) }
}

impl IVisual for RmlDesignComponent {
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        gpui::div().child("RML Design View").into_any_element()
    }
}

impl IWorkbenchComponent for RmlDesignComponent {}
```

### 7.6 FormatCommand —— IEditorCommand

```rust
use std::any::Any;
use rml_core::command::{CallContext, ICommand};
use rml_core::contribution::IContribution;
use studio_core::command::IEditorCommand;
use studio_core::component::EditorContext;

/// 格式化文档命令。
///
/// 元数据:id="editor.format", name="Format Document"
/// gesture="Shift+Alt+F"
/// can_execute 检查 EditorContext 存在(替代 when())
pub struct FormatCommand;

impl IContribution for FormatCommand {
    fn id(&self) -> &str { "editor.format" }
    fn name(&self) -> SharedString { "Format Document".into() }
}

impl ICommand for FormatCommand {
    fn execute(&self, ctx: &mut CallContext) {
        if let Some(editor_ctx) = ctx.parameter.and_then(|p| p.downcast_ref::<EditorContext>()) {
            // 经 IEditorLanguage::client() 调用 LSP formatting
        }
    }
    fn can_execute(&self, ctx: &mut CallContext) -> bool {
        ctx.parameter.and_then(|p| p.downcast_ref::<EditorContext>()).is_some()
    }
}

impl IEditorCommand for FormatCommand {
    fn gesture(&self) -> Option<SharedString> { Some("Shift+Alt+F".into()) }
}
```

## 8. Explorer(studio/crates/explorer)

### 8.1 LocalWorktree —— IWorktree + IWorkspace

```rust
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use gpui::SharedString;
use rml_core::contribution::{IContribution, IconSpec};
use rml_core::workbench::Uri;
use studio_core::worktree::{EntryKind, IWorktree, WorktreeChange, WorktreeEntry, WorktreeStat};
use studio_core::workspace::IWorkspace;

/// 本地文件系统 Worktree + Workspace 实现。
///
/// 同时 impl IWorktree 与 IWorkspace(空标记)。
/// 注册为 DI singleton,经 IWorkspaceManager::add 注入。
pub struct LocalWorktree {
    root: PathBuf,
    watchers: RwLock<Vec<WatchHandle>>,
}

impl LocalWorktree {
    pub fn new(root: PathBuf) -> Self {
        Self { root, watchers: RwLock::new(Vec::new()) }
    }
}

impl IContribution for LocalWorktree {
    fn id(&self) -> &str { self.root.to_str().unwrap_or("local") }
    fn name(&self) -> SharedString {
        self.root.file_name().and_then(|n| n.to_str()).unwrap_or("Local").into()
    }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("Folder")) }
}

impl IWorktree for LocalWorktree {
    fn root(&self) -> &Path { &self.root }
    fn entries(&self, dir: Option<&Path>) -> Vec<WorktreeEntry> {
        let dir = dir.map(|d| self.root.join(d)).unwrap_or_else(|| self.root.clone());
        std::fs::read_dir(&dir).into_iter().flatten().filter_map(|e| e.ok()).map(|e| {
            let path = e.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").into();
            let kind = if path.is_dir() { EntryKind::Directory }
                else if path.is_symlink() { EntryKind::Symlink }
                else { EntryKind::File };
            WorktreeEntry {
                path: path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf(),
                kind, name,
            }
        }).collect()
    }
    fn resolve(&self, path: &Path) -> Uri {
        url::Url::from_file_path(self.root.join(path)).unwrap()
    }
    fn relativize(&self, uri: &Uri) -> Option<PathBuf> {
        uri.to_file_path().ok().and_then(|p| p.strip_prefix(&self.root).ok().map(|p| p.to_path_buf()))
    }
    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join(path))
    }
    fn write(&self, path: &Path, content: &str) -> std::io::Result<()> {
        std::fs::write(self.root.join(path), content)
    }
    fn stat(&self, path: &Path) -> std::io::Result<WorktreeStat> {
        let meta = std::fs::metadata(self.root.join(path))?;
        Ok(WorktreeStat {
            kind: if meta.is_dir() { EntryKind::Directory } else { EntryKind::File },
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }
    fn watch(&self, on_change: Arc<dyn Fn(&WorktreeChange) + Send + Sync>) -> Box<dyn FnOnce()> {
        // 使用 notify crate 监听文件变更
        Box::new(|| {})
    }
}

impl IWorkspace for LocalWorktree {}
```

### 8.2 ExplorerPanel —— IActivityPanel

```rust
/// 文件浏览器面板 —— IActivityPanel 实现。
///
/// 经 DI 解析 IWorkspaceManager::list(),渲染多根文件树。
/// 用户点击文件 → IWorkbenchManager::open(uri)。
#[contribute(host_id = "studio.shell", id = "explorer", kind = "activity", order = 10)]
#[component]
pub struct ExplorerPanel {
    tree_items: Vec<TreeData>,
}

impl ExplorerPanel {
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        // 经 DI 解析 IWorkspaceManager
        let provider = cx.get_service::<Arc<ServiceProvider>>();
        if let Some(provider) = provider {
            let manager: Arc<dyn IWorkspaceManager> = provider.get().ok()?;
            let workspaces = manager.list();
            self.tree_items = workspaces.iter()
                .flat_map(|ws| build_workspace_tree(ws))
                .collect();
        }
    }

    #[command]
    pub fn on_file_activate(&mut self, item_id: &SharedString, cx: &mut Context<Self>) {
        // 查找工作空间 → resolve → IWorkbenchManager::open
        let provider = cx.get_service::<Arc<ServiceProvider>>();
        if let Some(provider) = provider {
            let manager: Arc<dyn IWorkspaceManager> = provider.get().ok()?;
            let wb_manager: Arc<dyn IWorkbenchManager> = provider.get().ok()?;
            for ws in manager.list() {
                let path = Path::new(item_id.as_ref());
                if ws.stat(path).is_ok() {
                    let uri = ws.resolve(path);
                    wb_manager.open(&uri);
                    return;
                }
            }
        }
    }
}

impl IActivityPanel for ExplorerPanel {}
```

## 9. AI(studio/crates/ai)

### 9.1 复用 IChatBackend

删除原计划的 `IAIProvider`/`IAIAssistant`/`IAITool`。直接复用 [rml_ui::IChatBackend](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/chat/backend.rs)(`send`/`stream`/`cancel`)及模型类型。

### 9.2 OpenAIBackend —— DI keyed service

```rust
/// OpenAI 聊天后端 —— impl IChatBackend。
/// 注册为 DI keyed("openai"),经 ChatPanel 解析。
pub struct OpenAIBackend { api_key: String, model: String }

impl IChatBackend for OpenAIBackend {
    fn send(&self, conversation: &ChatConversation, request: &ChatRequest) -> Result<ChatMessage, ChatError> { todo!() }
    fn stream(&self, conversation: &ChatConversation, request: &ChatRequest, on_event: &dyn Fn(&ChatStreamEvent)) -> Result<ChatMessage, ChatError> { todo!() }
    fn cancel(&self) -> Result<(), ChatError> { todo!() }
}
```

### 9.3 Copilot + Agent

- **Copilot**: 行内补全(Ghost Text),经 `IEditorLanguage::client()` 直访 LSP completion
- **Agent**: 函数式工具注册,工具经 `ChatToolCall` 模型表达

## 10. 流转推导

### 10.1 打开工作空间

```
用户"打开文件夹"
  → LocalWorktree::new(path)
  → DI 解析 IWorkspaceManager
  → IWorkspaceManager::add(Arc<LocalWorktree>)
  → ExplorerPanel observe workspaces 变化 → refresh_tree
  → IWorktree::entries() 渲染文件树
```

### 10.2 点击文件 → 打开 Tab

```
ExplorerPanel::on_file_activate(relative_path)
  → DI 解析 IWorkspaceManager → list() 查找工作空间
  → IWorktree::resolve(path) → Uri
  → DI 解析 IWorkbenchManager → open(uri)
       │
       ▼
  IWorkbenchManager::open(uri)
  1. 去重:workbenches 中查找 uri,已打开则激活
  2. 路由:uri.scheme() → DI get_keyed::<dyn IWorkbenchProvider>(schema)
  3. 构造:provider.render(uri) → Arc<dyn IWorkbench>
     ├─ EditorWorkbenchProvider:
     │   ├─ match_language(uri) → DI get_keyed::<dyn IEditorLanguage>
     │   ├─ build_components(uri) → Vec<Arc<dyn IWorkbenchComponent>>
     │   │   ├─ CodeEditorComponent::new(uri)
     │   │   └─ RmlDesignComponent::new(uri)(仅 .rml)
     │   └─ EditorWorkbench::new(uri, title, components, language)
     │        └─ impl IContributionHost(受理 components)
  4. 入栈:workbenches.push(wb) → ObservableVec 版本递增
  5. 激活:activated = Some(wb)
  6. 通知:flume → cx.notify → RML 重渲 → 新增 Tab
```

### 10.3 视图切换

```
用户点击"Design"按钮
  → EditorWorkbench::switch("design")
  → active_component_id = "design"
  → IVisual::render() 委托给 RmlDesignComponent::render()
```

### 10.4 AI 并行编程流程

```
AI Agent 需要理解 + 修改代码
  → DI 解析 IWorktree(经 IWorkspaceManager::list()[0])
  → IWorktree::entries(dir) 扫描代码结构
  → IWorktree::read(path) 读取文件内容(理解)
  → IWorktree::write(path, content) 应用代码修改
  → IWorktree::watch() 监听外部变更(避免冲突)
```

### 10.5 编辑器命令

```
用户按 Shift+Alt+F
  → 查找 gesture="Shift+Alt+F" 的 IEditorCommand
  → 构造 CallContext, parameter = &EditorContext { uri, cursor, selection: TextSpan }
  → ICommand::can_execute(ctx) → 检查 EditorContext 存在
  → ICommand::execute(ctx) → IEditorLanguage::client() → LSP formatting
```

## 11. 实施阶段

### Phase 1: Core 契约

- [ ] 创建 `studio/crates/core/src/ability_ext.rs` — 集中 4 个 AbilityExt + register 函数
- [ ] 创建 `studio/crates/core/src/component.rs` — IWorkbenchComponent + TextPosition + TextSpan + EditorContext
- [ ] 创建 `studio/crates/core/src/command.rs` — IEditorCommand(gesture)
- [ ] 创建 `studio/crates/core/src/worktree.rs` — IWorktree(含 write/stat) + WorktreeEntry + WorktreeStat + WorktreeChange
- [ ] 创建 `studio/crates/core/src/workspace.rs` — IWorkspace + IWorkspaceManager
- [ ] **验证**: `cargo build -p studio-core` 编译通过

### Phase 2: DI 容器 + Shell 骨架

- [ ] 添加 `rust-dix = "0.6"` 依赖到 studio workspace
- [ ] 创建 `studio/crates/shell/src/di.rs` — build_provider(ServiceCollection)
- [ ] 创建 `studio/crates/shell/src/shell_manager.rs` — ArcShellManager(IWorkbenchManager + IWorkspaceManager)
- [ ] 创建 `studio/crates/shell/src/arc_shell.rs` + `.rml` — ArcShell(#[window], 复刻 demo)
- [ ] `on_loaded` 初始化:DI 容器 → cx.set_service → 通道桥接 → contributehost
- [ ] **验证**: 能打开 ArcShell 窗口,显示欢迎页 Tab

### Phase 3: Explorer + Worktree

- [ ] 创建 `studio/crates/explorer/src/local_worktree.rs` — LocalWorktree(IWorktree + IWorkspace,含 notify crate 监听)
- [ ] 创建 `studio/crates/explorer/src/explorer_panel.rs` — ExplorerPanel(IActivityPanel)
- [ ] ArcShell::init_workspaces — 默认打开当前目录
- [ ] **验证**: 多根文件树渲染 + 点击文件触发 open

### Phase 4: Editor + IEditorLanguage

- [ ] 创建 `studio/crates/editor/src/language.rs` — IEditorLanguage trait
- [ ] 创建 `studio/crates/editor/src/languages/rust_rml.rs` — RustRmlLanguage
- [ ] 创建 `studio/crates/editor/src/editor_workbench.rs` — EditorWorkbench(IWorkbench + IContributionHost)
- [ ] 创建 `studio/crates/editor/src/provider.rs` — EditorWorkbenchProvider(IWorkbenchProvider)
- [ ] DI 注册: EditorWorkbenchProvider(keyed "file") + RustRmlLanguage(keyed "rust")
- [ ] **验证**: 打开 .rs 文件 → LSP 补全/悬停/诊断生效

### Phase 5: IWorkbenchComponent + CodeEditorComponent

- [ ] 创建 `studio/crates/editor/src/components/code_editor.rs` — CodeEditorComponent(id="code")
- [ ] 创建 `studio/crates/editor/src/components/rml_design.rs` — RmlDesignComponent(id="design")
- [ ] EditorWorkbenchProvider::build_components — 显式创建组件注入
- [ ] EditorWorkbench 视图切换 UI(切换按钮 + switch(id))
- [ ] **验证**: 打开 .rml 文件 → Code/Design 视图切换

### Phase 6: IEditorCommand

- [ ] 创建 `studio/crates/editor/src/commands/format.rs` — FormatCommand
- [ ] gesture 注册(Shift+Alt+F → editor.format)
- [ ] 命令面板集成
- [ ] **验证**: 按 Shift+Alt+F 触发格式化

### Phase 7: AI 模块

- [ ] 创建 `studio/crates/ai/src/provider/openai.rs` — impl IChatBackend
- [ ] 创建 `studio/crates/ai/src/chat_panel.rs` — 复用 rml_ui::ChatPanel
- [ ] 创建 `studio/crates/ai/src/copilot.rs` — 行内补全
- [ ] 创建 `studio/crates/ai/src/agent.rs` — Agent 工具注册
- [ ] DI 注册: OpenAIBackend(keyed "openai")
- [ ] **验证**: AI 对话流式响应 + 工具调用

## 12. 旧文档清理

以下文档为架构演进记录或已完成工作记录,实施时删除:

- `arc-studio-implementation-plan.md`(v1,已替代)
- `arc-studio-implementation-plan-v2.md`(v2,已替代)
- `arc-studio-editor-architecture-revision.md`(架构记录)
- `arc-studio-workspace-flow-revision.md`(架构记录)
- `command-contribution-unification.md`(已完成)
- `mainwindow-direct-icontributionhost-plan.md`(已完成)
- `rml-contribution-refactor-plan.md`(已完成)
- `rml-contribution-architecture-refactor-plan.md`(已完成)
- `rml-content-control-and-reactivity-enhancement-plan.md`(已完成)
- `plan-shell-simplification-via-workbench-manager.md`(已完成)
- `route-b-ability-extension-trait.md`(已完成)
- `rml-remove-hosthandle-observablevec-plan.md`(已完成)
- `rml-two-host-direct-impl-plan.md`(已完成)

## 13. 验证清单

1. **DI 容器**: 所有公共接口(IWorkbenchManager/IWorkspaceManager/IWorkbenchProvider/IEditorLanguage/IChatBackend)注册进 rust-dix ServiceProvider,经 `provider.get()` / `provider.get_keyed()` 解析
2. **集中 Ext**: studio/crates/core/src/ability_ext.rs 包含所有 4 个 AbilityExt + register 函数,无分散
3. **契约最小化**: IWorkbenchComponent 是空标记 trait,IEditorCommand 仅添加 gesture()
4. **Host 由实现决定**: IWorkbench trait 不继承 IContributionHost,EditorWorkbench 直接 impl IContributionHost override add() 受理组件,demo 的 CaseWorkbench/LspWorkbench 不 impl 即不受理
5. **文本类型**: TextPosition(点) + TextSpan(块),EditorContext.selection 使用 TextSpan
6. **IWorktree 完整**: 含 write(写入) + stat(元数据) + watch(监听),支持 AI 并行编程
7. **逻辑/UI 分离**: ArcShellManager(纯逻辑,DI) vs ArcShell(GPUI Entity,UI)
8. **单一出入口**: 资源打开经 IWorkbenchManager,工作空间经 IWorkspaceManager,文件经 IWorktree
9. **无框架改动**: IWorkbench 维持 IContribution + IVisual,demo 无需修改即编译通过;EditorWorkbench impl IContributionHost 是实现选择,非 trait 强制
10. **流转完整**: 打开工作空间(4步) + 点击文件→Tab(6步) + 视图切换(3步) + AI编程(5步) + 命令(5步)链路完整
