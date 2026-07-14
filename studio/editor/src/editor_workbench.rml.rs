//! EditorWorkbench ViewModel —— RML 声明式编辑器组件。
//!
//! `#[component]` 生成 `impl IModel + IViewModel + IComponent + Render`。
//! 手动 `impl IContribution + IVisual + ILifecycle` 补充元数据 + 渲染入口 + 初始化。
//! 手动 `impl IWorkbench` 提供 URI/关闭/激活/状态管理。

use std::any::Any;
use std::path::PathBuf;
use std::sync::{Arc, Once};

use gpui::{AnyElement, App, Entity, Window};
use gpui_component::input::InputState;
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{
    IconSpec, register_contribution_ability, register_visual_ability,
};
use rml_core::workbench::{IWorkbench, Uri, register_workbench_ability};
use rust_rml_client::{file_path_to_uri, LanguageClient};
use studio_core::ability_ext::WorkbenchComponentAbilityExt;
use studio_core::get_workbench_components;

/// 代码编辑器工作台 —— IWorkbench 实现，承载文件编辑 + LSP 集成。
///
/// `#[component]` 生成 RML 框架契约（IModel/IViewModel/IComponent/Render），
/// 经 `include!` 引入编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 `impl IWorkbench + IContribution + IVisual + ILifecycle`：
/// - `IVisual::render` → 创建 Entity 并委托 `Render::render`
/// - `ILifecycle::on_loaded` → 打开文件 + 创建 InputState + 安装 LSP providers
/// - `IWorkbench` → URI/关闭/激活/状态管理
#[component]
#[derive(Default)]
pub struct EditorWorkbench {
    editor_state: Option<Entity<InputState>>,
    language_client: Option<Arc<LanguageClient>>,
    uri: SharedString,
    file_path: PathBuf,
    /// 匹配当前 URI 的视图组件名称列表(each 指令要求字段而非方法)。
    /// 在 `init_editor` 中经 `compute_view_names()` 填充。
    view_names: Vec<SharedString>,
}

impl IContribution for EditorWorkbench {
    fn id(&self) -> &str {
        &self.uri
    }
    fn name(&self) -> SharedString {
        self.file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("File"))
    }
}

impl IVisual for EditorWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<EditorWorkbench>(cx);
        // get_or_create_entity 按 TypeId 缓存,所有 EditorWorkbench 实例共享同一 Entity。
        // 当 URI 变化时(打开不同文件),需重新初始化编辑器以加载新文件内容。
        let uri = self.uri.clone();
        let file_path = self.file_path.clone();
        let view_names = self.compute_view_names();
        entity.update(cx, |this, ctx| {
            let uri_changed = this.uri != uri;
            this.uri = uri;
            this.file_path = file_path;
            this.view_names = view_names;
            if uri_changed {
                this.init_editor(window, ctx);
            }
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for EditorWorkbench {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.init_editor(window, cx);
    }
}

impl IWorkbench for EditorWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn close(&self) {
        // 编辑器关闭时释放 LSP 资源
    }

    fn activate(&self) {
        // 编辑器获得焦点
    }

    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}

    fn closable(&self) -> bool {
        true
    }
}

impl EditorWorkbench {
    /// 面包屑导航文本 —— 显示最后 3 个路径段,用 › 分隔。
    #[computed]
    pub fn breadcrumb_text(&self) -> SharedString {
        if self.file_path.as_os_str().is_empty() {
            return "untitled".into();
        }
        let segments: Vec<&std::ffi::OsStr> = self.file_path.iter().rev().take(3).collect();
        segments
            .into_iter()
            .rev()
            .filter_map(|s| s.to_str())
            .collect::<Vec<_>>()
            .join(" › ")
            .into()
    }

    /// 查询匹配当前 URI 的视图组件名称列表。
    ///
    /// 查询全局 IWorkbenchComponent 注册表,按 `matches(uri)` 过滤。
    /// 仅当多个组件匹配时,Header 显示视图切换按钮。
    ///
    /// 注:`each` 指令 codegen 生成字段访问 `self.view_names.iter()`,
    /// 因此 `view_names` 必须是字段而非 `#[computed]` 方法。
    /// 字段在 `init_editor` / `IVisual::render` 中经此方法填充。
    fn compute_view_names(&self) -> Vec<SharedString> {
        let Ok(uri) = self.uri.parse::<Uri>() else {
            return Vec::new();
        };
        get_workbench_components()
            .iter()
            .filter_map(|c| {
                c.as_workbench_component()
                    .filter(|wc| wc.matches(&uri))
                    .map(|wc| wc.name())
            })
            .collect()
    }

    /// 是否显示视图切换按钮 —— 仅当多个视图组件匹配时显示。
    #[computed]
    pub fn show_view_switcher(&self) -> bool {
        self.view_names.len() > 1
    }

    /// 设置文件路径和 URI（由 EditorProvider 在构造后调用）。
    pub fn set_file(&mut self, uri: SharedString, file_path: PathBuf) {
        self.uri = uri;
        self.file_path = file_path;
    }

    /// 初始化编辑器：读取文件内容 → 创建 InputState → 安装 LSP providers。
    ///
    /// 无论是否成功解析 URI 或读取文件,都必须创建 `editor_state`,
    /// 否则生成代码 `.expect("init editor_state in on_loaded")` 会 panic。
    fn init_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 填充视图组件名称列表(Header 视图切换按钮数据源)
        self.view_names = self.compute_view_names();

        if self.file_path.as_os_str().is_empty() {
            self.editor_state =
                Some(cx.new(|cx| InputState::new(window, cx).multi_line(true)));
            return;
        }

        let text = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        let language = detect_language(&self.file_path);

        // 创建 LanguageClient（按工作区根目录缓存，避免重复启动 LSP server）
        let client = self.get_or_create_language_client(&self.file_path);

        // URI 解析失败时跳过 LSP 集成,但仍创建可用的编辑器
        let uri = file_path_to_uri(&self.file_path).ok();

        if let (Some(ref client), Some(ref uri)) = (&client, &uri) {
            client.open_document(uri, &text);
        }

        let editor_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .multi_line(true)
                .default_value(&text);
            if let (Some(ref client), Some(ref uri)) = (&client, &uri) {
                client.install_providers(&mut state, uri.clone());
            }
            state
        });

        // 文档变更同步到 LSP server
        if let (Some(ref client), Some(ref uri)) = (&client, &uri) {
            let uri_clone = uri.clone();
            let client_clone = client.clone();
            cx.observe(&editor_state, move |_: &mut Self, state, obs_cx| {
                let text = state.read(obs_cx).text().to_string();
                client_clone.change_document(&uri_clone, &text);
            })
            .detach();
        }

        self.editor_state = Some(editor_state);
        self.language_client = client;
        cx.notify();
    }

    /// 从工作区根目录获取或创建 LanguageClient（同一工作区共享一个 LSP server）。
    fn get_or_create_language_client(
        &self,
        _file_path: &std::path::Path,
    ) -> Option<Arc<LanguageClient>> {
        // MVP: 使用 unified profile（rust+rml 一体化），
        // 工作区根目录取当前工作目录。后续迭代支持多工作区 + 多语言。
        let workspace_root = std::env::current_dir().ok()?;
        LanguageClient::unified(&workspace_root).ok().map(Arc::new)
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

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:EditorWorkbench 需注册 IContribution + IVisual + IWorkbench
//  能力 cast,使 MainWindow 的 as_visual() / as_workbench() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_editor_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<EditorWorkbench>();
        register_visual_ability::<EditorWorkbench>();
        register_workbench_ability::<EditorWorkbench>();
    });
}