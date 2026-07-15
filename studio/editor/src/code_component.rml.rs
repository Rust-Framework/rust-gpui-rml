//! CodeComponent ViewModel —— 默认代码编辑视图组件(IWorkbenchComponent)。
//!
//! 从 EditorWorkbench 接管代码编辑逻辑(InputState + LanguageClient)。
//! 经 `observe(input_state)` 写回 host 共享 `WorkbenchDocument`,
//! 经 `ILifecycle::before_render` 中检测 `document.uri` 变化同步到 `InputState`。
//!
//! # 同步链路
//!
//! 1. 用户编辑 → `InputState` 变更 → `observe` 回调 → `document.set_content`
//! 2. Tab 切换 → `EditorWorkbench.reload` → `document.reload`(新 uri+content)
//!    → `CodeComponent::before_render` 检测 `document.uri` 变化 → `init_editor`
//!    (创建新 InputState + LSP,匹配新文件)
//! 3. `document.content` 变化(非自身编辑引起)→ `before_render` 中 `set_value` 同步到 InputState
//!
//! # 循环防护
//!
//! `last_synced_content` 字段记录上次同步内容,`observe(input_state)` 与
//! `before_render(document)` 双向同步均比对,内容相同则跳过 update。

use std::sync::Arc;

use gpui::{Entity, SharedString, Window};
use gpui_component::input::InputState;
use rml::prelude::*;
use rml_app::contribution::get_active_entity;
use rml_core::contribution::{IconSpec, IContribution};
use rml_core::workbench::Uri;
use rust_rml_client::{file_path_to_uri, LanguageClient};
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use studio_core::document::WorkbenchDocument;
use studio_core::register_workbench_component;

use crate::editor_workbench::EditorWorkbench;

/// 默认代码编辑视图组件。
///
/// `matches(uri)` 使用默认实现(返回 `true`)—— 所有资源均可使用代码视图。
/// 实际代码编辑经 `<CodeEditor>` RML 标签自动绑定到 `editor_state` 字段。
#[component]
#[derive(Default)]
pub struct CodeComponent {
    /// 代码编辑器状态(`<CodeEditor>` 经 tags.rs 自动绑定此字段)。
    editor_state: Option<Entity<InputState>>,
    /// LSP 客户端(按工作区根目录缓存)。
    language_client: Option<Arc<LanguageClient>>,
    /// 上次同步到 InputState 的 document.uri —— 变化时重新 init_editor。
    last_synced_uri: SharedString,
    /// 上次同步到 InputState 的 document.content —— 防止循环同步。
    last_synced_content: SharedString,
}

impl IContribution for CodeComponent {
    fn id(&self) -> &str {
        "code"
    }
    fn name(&self) -> SharedString {
        "Code".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("FileCode"))
    }
}

impl ILifecycle for CodeComponent {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 首次加载:从 host document 读初始内容,创建 InputState + LSP。
        self.init_editor(window, cx);
    }

    fn before_render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 首次渲染 editor_state 为 None,由 on_loaded 负责初始化(避免重复 init)。
        // 后续渲染检测 host document 变化(Tab 切换/其他组件编辑)同步到 InputState。
        if self.editor_state.is_some() {
            self.sync_from_host(window, cx);
        }
    }
}

impl IWorkbenchComponent for CodeComponent {
    // matches(uri) 使用默认实现(返回 true)—— CodeComponent 是默认文本视图
}

impl CodeComponent {
    /// 从 host 取共享 document Entity。
    ///
    /// CodeComponent 经 `get_active_entity::<EditorWorkbench>` 取当前渲染的 host
    /// (最近一次 `get_or_create_entity_by_uri` 设置的活跃 Entity,即当前 Tab 的 URI 对应 Entity),
    /// 直接调用 `IWorkbenchComponentHost::document()`(EditorWorkbench impl 此 trait)。
    /// 时序保证:EditorWorkbench IVisual::render → set_active → 子组件 before_render → 此处读取。
    fn host_document(&self, cx: &mut Context<Self>) -> Option<Entity<WorkbenchDocument>> {
        let host = get_active_entity::<EditorWorkbench>(cx)?;
        let host_ref = host.read(cx);
        Some(IWorkbenchComponentHost::document(host_ref))
    }

    /// 初始化编辑器:从 host document 读内容 → 创建 InputState → 安装 LSP → observe 写回。
    ///
    /// 在 `on_loaded` 与 `sync_from_host`(检测 uri 变化)中调用。
    /// 每次调用都重新创建 InputState(丢失撤销栈,匹配现有 Tab 切换行为)。
    fn init_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.host_document(cx) else {
            return;
        };
        let (content, uri) = {
            let doc_read = doc.read(cx);
            (doc_read.content(), doc_read.uri())
        };

        // 从 uri 解析本地路径(用于 LSP 与语言检测)
        let file_path = uri
            .parse::<Uri>()
            .ok()
            .and_then(|u| u.to_file_path().ok())
            .unwrap_or_default();
        let language = detect_language(&file_path);

        // 创建 LanguageClient(按工作区根目录缓存,同一工作区共享一个 LSP server)
        let client = get_or_create_language_client(&file_path);
        let lsp_uri = file_path_to_uri(&file_path).ok();

        if let (Some(ref client), Some(ref lsp_uri)) = (&client, &lsp_uri) {
            client.open_document(lsp_uri, &content);
        }

        let state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .default_value(&content);
            if let (Some(ref client), Some(ref lsp_uri)) = (&client, &lsp_uri) {
                client.install_providers(&mut state, lsp_uri.clone());
            }
            state
        });

        // observe input_state → 写回 document + 同步 LSP
        if let (Some(ref client), Some(ref lsp_uri)) = (&client, &lsp_uri) {
            let uri_clone = lsp_uri.clone();
            let client_clone = client.clone();
            cx.observe(&state, move |this: &mut Self, state, obs_cx| {
                let new_text = state.read(obs_cx).text().to_string();
                // LSP 同步(每次编辑都通知 server)
                client_clone.change_document(&uri_clone, &new_text);
                // document 同步(经 last_synced_content 防循环)
                if this.last_synced_content.as_ref() != new_text.as_str() {
                    this.last_synced_content = new_text.clone().into();
                    let doc = this.host_document(obs_cx);
                    if let Some(doc) = doc {
                        doc.update(obs_cx, |d, _| d.set_content(new_text.into()));
                    }
                }
            })
            .detach();
        } else {
            // 无 LSP 时仅 observe → document
            cx.observe(&state, move |this: &mut Self, state, obs_cx| {
                let new_text = state.read(obs_cx).text().to_string();
                if this.last_synced_content.as_ref() != new_text.as_str() {
                    this.last_synced_content = new_text.clone().into();
                    let doc = this.host_document(obs_cx);
                    if let Some(doc) = doc {
                        doc.update(obs_cx, |d, _| d.set_content(new_text.into()));
                    }
                }
            })
            .detach();
        }

        self.editor_state = Some(state);
        self.language_client = client;
        self.last_synced_uri = uri;
        self.last_synced_content = content;
        cx.notify();
    }

    /// 从 host document 同步到 InputState —— 在 ILifecycle::before_render 中每帧调用。
    ///
    /// - `document.uri` 变化(Tab 切换)→ `init_editor`(重建 InputState + LSP)
    /// - `document.content` 变化(其他组件编辑引起)→ MVP 阶段不处理(其他组件均只读)
    /// - 无变化 → no-op
    ///
    /// # 后续扩展
    ///
    /// Phase 4 落地 PreviewComponent(只读)后仍无需处理 content 变化。
    /// RmlDesignComponent 落地时,设计器编辑 AST → 写回 document.content,
    /// 此时需经 `cx.active_window()` + `handle.update` 获取 `&mut Window`
    /// 调用 `InputState::set_value`(签名:`set_value(&mut self, value, window, cx)`)。
    pub fn sync_from_host(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(doc) = self.host_document(cx) else {
            return;
        };
        let uri = doc.read(cx).uri();

        // uri 变化 → 重新 init_editor(新文件需新 InputState + LSP)
        if self.last_synced_uri != uri {
            self.init_editor(window, cx);
        }
    }
}

/// 从文件扩展名推断语言 ID。
fn detect_language(path: &std::path::Path) -> &str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("rml") => "rml",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("toml") | Some("lock") => "toml",
        Some("html") => "html",
        Some("css") => "css",
        _ => "plaintext",
    }
}

/// 从工作区根目录获取或创建 LanguageClient(同一工作区共享一个 LSP server)。
fn get_or_create_language_client(
    _file_path: &std::path::Path,
) -> Option<Arc<LanguageClient>> {
    // MVP: 使用 unified profile(rust+rml 一体化),
    // 工作区根目录取当前工作目录。后续迭代支持多工作区 + 多语言。
    let workspace_root = std::env::current_dir().ok()?;
    LanguageClient::unified(&workspace_root).ok().map(Arc::new)
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:CodeComponent 需注册 IWorkbenchComponent 能力 cast + 工厂。
// ──────────────────────────────────────────────────────────────────────────

/// 注册 CodeComponent 能力 cast + 工厂。
///
/// 在 `#[ctor::ctor]` 中调用:
/// 1. `register_workbench_component_ability::<CodeComponent>()` —— 注册能力 cast
/// 2. `register_workbench_component(factory)` —— 注册工厂到全局注册表
pub fn register_code_component() {
    register_workbench_component_ability::<CodeComponent>();
    register_workbench_component(|| {
        Arc::new(CodeComponent::default()) as Arc<dyn IWorkbenchComponent>
    });
}
