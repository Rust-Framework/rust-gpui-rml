//! EditorWorkbench ViewModel —— IWorkbench + IWorkbenchComponentHost,纯壳。
//!
//! Phase 2b+3 改造:不再直接持有 `editor_state`/`language_client` —— 代码编辑逻辑
//! 由 `CodeComponent` 接管。EditorWorkbench 仅负责:
//! 1. 资源会话管理(IWorkbench):uri/close/activate/closable
//! 2. 组件宿主管理(IWorkbenchComponentHost):枚举/激活/切换 + 共享文档/状态
//! 3. Header 渲染:面包屑 + 视图切换按钮
//! 4. Body 容器:经条件分支渲染激活的 IWorkbenchComponent
//!
//! `#[component]` 生成 `impl IModel + IViewModel + IComponent + Render`。
//! 手动 `impl IContribution + IVisual + ILifecycle + IWorkbench + IWorkbenchComponentHost`
//! 补充元数据 + 渲染入口 + 初始化 + 资源会话 + 组件宿主。

use std::any::Any;
use std::path::PathBuf;
use std::sync::{Arc, Once};

use gpui::{AnyElement, App, Entity, Window};
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{
    IconSpec, register_contribution_ability, register_visual_ability,
};
use rml_core::workbench::{IWorkbench, Uri, register_workbench_ability};
use studio_core::ability_ext::register_workbench_component_host_ability;
use studio_core::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use studio_core::document::{WorkbenchDocument, WorkbenchState, document_kind};
use studio_core::get_workbench_components;

use crate::code_component::CodeComponent;

/// 代码编辑器工作台 —— IWorkbench + IWorkbenchComponentHost,纯壳。
///
/// `#[component]` 生成 RML 框架契约(IModel/IViewModel/IComponent/Render),
/// 经 `include!` 引入编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 impl:
/// - `IContribution + IVisual` —— 元数据 + 渲染入口(委托 Render)
/// - `ILifecycle` —— on_loaded 初始化共享 document/state + code_component + observe
/// - `IWorkbench` —— 资源会话管理(uri/close/activate)
/// - `IWorkbenchComponentHost` —— 组件枚举/激活/切换 + 共享文档/状态
#[component]
#[derive(Default)]
pub struct EditorWorkbench {
    uri: SharedString,
    file_path: PathBuf,
    /// 匹配当前 URI 的视图组件名称列表(each 指令要求字段而非方法)。
    /// 在 `reload` 中经 `compute_view_names()` 填充。
    view_names: Vec<SharedString>,
    /// 共享文档模型 —— IWorkbenchComponent 间数据同步的单一真相源。
    document: Option<Entity<WorkbenchDocument>>,
    /// 共享工作台状态 —— 跨组件统一管理 dirty/saving 等。
    ///
    /// observe `document` 变化 → 更新 `dirty` → Tab 标题联动。
    state: Option<Entity<WorkbenchState>>,
    /// 当前激活的 IWorkbenchComponent id(空串表示未激活)。
    ///
    /// 在 `reload` 中默认激活首个匹配组件;Header 视图切换按钮经
    /// `switch_component` 更新此字段。
    active_component_id: SharedString,
    /// 代码编辑子组件 —— 经 RML `<CodeComponent if={is_code_active} />` 引用。
    ///
    /// on_loaded 中初始化,经 `get_or_create_entity` 全局单例缓存。
    /// 切 Tab 时 Entity 不重建,CodeComponent 内部经 observe(document) 同步。
    code_component: Option<Entity<CodeComponent>>,
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
        // 当 URI 变化时(打开不同文件),需重新加载文件内容到共享 document。
        let uri = self.uri.clone();
        let file_path = self.file_path.clone();
        let view_names = self.compute_view_names();
        entity.update(cx, |this, ctx| {
            let uri_changed = this.uri != uri;
            this.uri = uri;
            this.file_path = file_path;
            this.view_names = view_names;
            if uri_changed {
                this.reload(ctx);
            }
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for EditorWorkbench {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 初始化共享 document + state(IWorkbenchComponent 间数据同步媒介)
        self.document = Some(cx.new(|_| WorkbenchDocument::default()));
        self.state = Some(cx.new(|_| WorkbenchState::default()));
        // 初始化 code_component 子组件(RML 模板经 <CodeComponent /> 引用)
        self.code_component = Some(cx.new(|_| CodeComponent::default()));

        // observe document → state.set_dirty
        // 注册一次即可,后续 document.reload / set_content 均触发此回调
        if let (Some(doc), Some(state)) = (self.document.as_ref(), self.state.as_ref()) {
            let state_clone = state.clone();
            cx.observe(doc, move |_: &mut Self, doc, cx| {
                let dirty = doc.read(cx).is_dirty();
                state_clone.update(cx, |s, _| s.set_dirty(dirty));
            })
            .detach();
        }

        // 首次加载无需调用 reload(IVisual::render 检测 uri_changed 会调用),
        // 但需保证 code_component 的 on_loaded 能读到 document —— 顺序由 RML 渲染保证:
        // EditorWorkbench.on_loaded 先于 .rml 模板渲染 → 模板中 <CodeComponent /> 触发
        // CodeComponent.on_loaded → 此时 document 已初始化。
        let _ = window;
    }
}

impl IWorkbench for EditorWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn close(&self) {
        // 编辑器关闭时释放资源(LSP 由 CodeComponent 持有,Entity drop 自动释放)
    }

    fn activate(&self) {
        // 编辑器获得焦点
    }

    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}

    fn closable(&self) -> bool {
        true
    }
}

impl IWorkbenchComponentHost for EditorWorkbench {
    fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>> {
        let Ok(uri) = self.uri.parse::<Uri>() else {
            return Vec::new();
        };
        get_workbench_components()
            .into_iter()
            .filter(|c| c.matches(&uri))
            .collect()
    }

    fn active_component_id(&self) -> SharedString {
        self.active_component_id.clone()
    }

    fn switch_component(&self, id: &str, cx: &mut App) {
        // 经 `get_or_create_entity` 取 host Entity,update 内部可变性更新字段。
        // RML 模板经 active_component_id 字段访问触发条件分支重新渲染。
        let entity = get_or_create_entity::<EditorWorkbench>(cx);
        entity.update(cx, |this, _| {
            this.active_component_id = id.to_string().into();
        });
    }

    fn document(&self) -> Entity<WorkbenchDocument> {
        self.document
            .as_ref()
            .expect("document initialized in on_loaded")
            .clone()
    }

    fn state(&self) -> Entity<WorkbenchState> {
        self.state
            .as_ref()
            .expect("state initialized in on_loaded")
            .clone()
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
    /// 字段在 `reload` / `IVisual::render` 中经此方法填充。
    fn compute_view_names(&self) -> Vec<SharedString> {
        let Ok(uri) = self.uri.parse::<Uri>() else {
            return Vec::new();
        };
        get_workbench_components()
            .iter()
            .filter(|c| c.matches(&uri))
            .map(|c| c.name())
            .collect()
    }

    /// 是否显示视图切换按钮 —— 仅当多个视图组件匹配时显示。
    #[computed]
    pub fn show_view_switcher(&self) -> bool {
        self.view_names.len() > 1
    }

    /// 当前激活的组件是否为 code(CodeComponent)。
    ///
    /// RML 模板经 `<CodeComponent if={is_code_active} />` 条件渲染。
    #[computed]
    pub fn is_code_active(&self) -> bool {
        self.active_component_id == "code" || self.active_component_id.is_empty()
    }

    /// 设置文件路径和 URI(由 EditorProvider 在构造后调用)。
    pub fn set_file(&mut self, uri: SharedString, file_path: PathBuf) {
        self.uri = uri;
        self.file_path = file_path;
    }

    /// 重新加载:读文件 → document.reload → compute_view_names → 默认激活。
    ///
    /// 在 `IVisual::render` 中检测 `uri_changed` 时调用。Tab 切换时共享 Entity 不重建,
    /// 经此方法同步新文件内容到 document,CodeComponent observe document 变化重新初始化。
    fn reload(&mut self, cx: &mut Context<Self>) {
        // 填充视图组件名称列表(Header 视图切换按钮数据源)
        self.view_names = self.compute_view_names();

        // 默认激活首个匹配组件(切换 Tab 时仅首次激活,后续保留用户选择)
        if self.active_component_id.is_empty() {
            self.active_component_id = self
                .components()
                .first()
                .map(|c| c.id().to_string().into())
                .unwrap_or_default();
        }

        if self.file_path.as_os_str().is_empty() {
            return;
        }

        // 读文件 → document.reload(触发 CodeComponent 同步)
        let text = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        let kind = infer_kind(&self.file_path);
        if let Some(doc) = self.document.as_ref() {
            doc.update(cx, |d, _| {
                d.reload(self.uri.clone(), text.into(), kind);
            });
        }

        cx.notify();
    }
}

/// 从文件扩展名推断文档类型标识(开放字符串)。
///
/// 返回 [`studio_core::document::document_kind`] 模块的内置常量,
/// 插件可用自定义字符串扩展。组件经此 kind 判断渲染策略(如 PreviewComponent
/// 对 markdown 走 GFM 渲染,对 html 走源码展示)。
fn infer_kind(path: &std::path::Path) -> SharedString {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") | Some("markdown") => document_kind::MARKDOWN.into(),
        Some("html") | Some("htm") => document_kind::HTML.into(),
        Some("rml") => document_kind::RML.into(),
        _ => document_kind::TEXT.into(),
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:EditorWorkbench 需注册 IContribution + IVisual + IWorkbench
//  + IWorkbenchComponentHost 能力 cast,使 MainWindow 的 as_visual() /
//  as_workbench() / as_workbench_component_host() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_editor_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<EditorWorkbench>();
        register_visual_ability::<EditorWorkbench>();
        register_workbench_ability::<EditorWorkbench>();
        // 注册 IWorkbenchComponentHost 能力 cast,使组件经
        // `get_or_create_entity::<EditorWorkbench>` 取 host 后能调用其方法。
        register_workbench_component_host_ability::<EditorWorkbench>();
    });
}
