//! 全局注册表 —— `ctor` + 全局工厂列表
//!
//! 三个独立注册表:
//! - **工作空间 opener** —— `register_workspace_opener` / `open_workspace`
//! - **工作台组件** —— `register_workbench_component` / `get_workbench_components`
//! - **ChatProvider** —— `register_chat_provider` / `get_chat_providers`
//!
//! 扩展 crate 经 `#[ctor::ctor]` 调用注册函数,Shell 在运行时枚举注入 DI 容器。
//! `Fn`（非 `FnOnce`）+ 非 drain 式读取,支持多次调用。
//!
//! DI 服务（IWorkbenchManager / IWorkspaceManager / IChatManager / IWorkbenchProvider 等）
//! 经 `studio_core::di::auto_register` 注册,不在此模块。

use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use crate::chat::IChatProvider;
use crate::component::IWorkbenchComponent;
use crate::workspace::IWorkspace;

// ──────────────────────────────────────────────────────────────────────────
//  工作空间 opener 注册表
// ──────────────────────────────────────────────────────────────────────────

type WorkspaceOpener = Box<dyn Fn(&Path) -> Option<Arc<dyn IWorkspace>> + Send + Sync>;

static WORKSPACE_OPENERS: OnceLock<Mutex<Vec<WorkspaceOpener>>> = OnceLock::new();

/// 注册工作空间 opener。通常在 `#[ctor::ctor]` 函数中调用。
///
/// opener 接收路径,返回 `Some(Arc<dyn IWorkspace>)` 表示此路径可被识别为工作空间,
/// 返回 `None` 表示此 opener 无法处理（Shell 将尝试下一个 opener）。
pub fn register_workspace_opener(
    f: impl Fn(&Path) -> Option<Arc<dyn IWorkspace>> + Send + Sync + 'static,
) {
    WORKSPACE_OPENERS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 尝试打开指定路径为工作空间 —— 依次尝试所有已注册 opener,返回首个成功结果。
///
/// 未经任何 opener 注册时返回 `None`。
pub fn open_workspace(path: &Path) -> Option<Arc<dyn IWorkspace>> {
    let registry = WORKSPACE_OPENERS.get()?;
    for opener in registry.lock().unwrap().iter() {
        if let Some(ws) = opener(path) {
            return Some(ws);
        }
    }
    None
}

// ──────────────────────────────────────────────────────────────────────────
//  工作台组件注册表
// ──────────────────────────────────────────────────────────────────────────

type WorkbenchComponentFactory = Box<dyn Fn() -> Arc<dyn IWorkbenchComponent> + Send + Sync>;

static WORKBENCH_COMPONENTS: OnceLock<Mutex<Vec<WorkbenchComponentFactory>>> = OnceLock::new();

/// 注册工作台组件工厂。通常在 `#[ctor::ctor]` 函数中调用。
///
/// 工厂返回 `Arc<dyn IWorkbenchComponent>`,Workbench 经 `IWorkbenchComponentHost::components()`
/// 取得后直接调用 `matches(uri)` 过滤,无需经能力查询中转。
pub fn register_workbench_component(
    f: impl Fn() -> Arc<dyn IWorkbenchComponent> + Send + Sync + 'static,
) {
    WORKBENCH_COMPONENTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 枚举所有已注册的工作台组件(经工厂构造)。
///
/// 返回 `Vec<Arc<dyn IWorkbenchComponent>>`,调用方按 `matches(uri)` 过滤。
/// 未经注册时返回空 Vec。
pub fn get_workbench_components() -> Vec<Arc<dyn IWorkbenchComponent>> {
    match WORKBENCH_COMPONENTS.get() {
        Some(registry) => registry.lock().unwrap().iter().map(|f| f()).collect(),
        None => Vec::new(),
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  ChatProvider 注册表
// ──────────────────────────────────────────────────────────────────────────

type ChatProviderFactory = Box<dyn Fn() -> Arc<dyn IChatProvider> + Send + Sync>;

static CHAT_PROVIDERS: OnceLock<Mutex<Vec<ChatProviderFactory>>> = OnceLock::new();

/// 注册聊天提供程序工厂。通常在 `#[ctor::ctor]` 函数中调用。
///
/// 工厂返回 `Arc<dyn IChatProvider>`,`ChatManager` 经 `get_chat_providers()`
/// 取得后聚合,提供统一的 `IChatter` 查询。
pub fn register_chat_provider(
    f: impl Fn() -> Arc<dyn IChatProvider> + Send + Sync + 'static,
) {
    CHAT_PROVIDERS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 枚举所有已注册的聊天提供程序(经工厂构造)。
///
/// 返回 `Vec<Arc<dyn IChatProvider>>`,`ChatManager` 聚合后提供 `chatters()` 查询。
/// 未经注册时返回空 Vec。
pub fn get_chat_providers() -> Vec<Arc<dyn IChatProvider>> {
    match CHAT_PROVIDERS.get() {
        Some(registry) => registry.lock().unwrap().iter().map(|f| f()).collect(),
        None => Vec::new(),
    }
}
