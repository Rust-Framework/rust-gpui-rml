//! PreviewComponent ViewModel —— 只读预览视图组件(IWorkbenchComponent)。
//!
//! Phase 4 落地:经 `matches(uri)` 仅匹配 `.md`/`.markdown`/`.html`,
//! 在 EditorWorkbench 的视图切换器中作为可选视图呈现。
//!
//! # 数据同步链路
//!
//! ```text
//! CodeComponent 编辑 → InputState → observe → document.set_content
//!   ↓
//! PreviewComponent::IVisual::render → sync_from_document
//!   → 读 document.content + kind → 更新 content/kind 字段 → cx.notify
//!   → RML 模板重新渲染 <Markdown content={content} />
//! ```
//!
//! # 渲染分支
//!
//! - `kind == document_kind::MARKDOWN` → `<Markdown content={content} />`(GFM 富文本)
//! - `kind == document_kind::HTML` → `<pre>{content}</pre>`(源码,降级策略)
//! - 其他 → `<pre>{content}</pre>`(纯文本)
//!
//! HTML 降级为源码展示的原因:GPUI 暂无 HTML 渲染引擎,iframe 沙箱方案
//! 需引入 webview 依赖,与原生渲染目标冲突。后续可经插件扩展 HTML 渲染器。

use std::sync::Arc;

use gpui::{AnyElement, App, SharedString, Window};
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{IconSpec, IContribution, IVisual};
use rml_core::workbench::Uri;
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use studio_core::document::document_kind;
use studio_core::register_workbench_component;

use crate::editor_workbench::EditorWorkbench;

/// 只读预览视图组件。
///
/// `matches(uri)` 仅匹配 `.md`/`.markdown`/`.html` —— 其他文件不显示 Preview 按钮。
/// 经 `IVisual::render` 中 `sync_from_document` 同步 host document 内容到字段,
/// RML 模板按 `kind` 条件分支渲染。
#[component]
#[derive(Default)]
pub struct PreviewComponent {
    /// 缓存的文档内容(从 host document 同步)。
    content: SharedString,
    /// 缓存的文档类型(开放字符串,从 host document 同步)。
    kind: SharedString,
    /// 上次同步的内容,避免重复 cx.notify。
    last_synced_content: SharedString,
    /// 面包屑文本(从 host document.uri 解析文件名)。
    breadcrumb_text: SharedString,
}

impl IContribution for PreviewComponent {
    fn id(&self) -> &str {
        "preview"
    }
    fn name(&self) -> SharedString {
        "Preview".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("FileText"))
    }
}

impl IVisual for PreviewComponent {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<PreviewComponent>(cx);
        entity.update(cx, |this, ctx| {
            // 每帧同步 host document 变化(CodeComponent 编辑 → document.set_content → 此处刷新)
            this.sync_from_document(ctx);
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for PreviewComponent {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 首次加载同步初始内容(IVisual::render 也会调用,但首次 render 前先初始化避免空闪)
        self.sync_from_document(cx);
    }
}

impl IWorkbenchComponent for PreviewComponent {
    fn matches(&self, uri: &Uri) -> bool {
        // 经 Path 解析扩展名,避免 url::Url path() 中 query/fragment 干扰
        let path = std::path::Path::new(uri.path());
        match path.extension().and_then(|e| e.to_str()) {
            Some("md") | Some("markdown") | Some("html") => true,
            _ => false,
        }
    }
}

impl PreviewComponent {
    /// 从 host document 同步 content + kind + breadcrumb_text。
    ///
    /// 在 `IVisual::render` 中每帧调用,经 `last_synced_content` 比对避免重复更新。
    /// CodeComponent 编辑 → document.set_content → 下次 render 检测 content 变化 → 刷新。
    fn sync_from_document(&mut self, cx: &mut Context<Self>) {
        let host = get_or_create_entity::<EditorWorkbench>(cx);
        let (content, kind, uri) = {
            let host_ref = host.read(cx);
            let doc = IWorkbenchComponentHost::document(host_ref);
            let doc_read = doc.read(cx);
            (doc_read.content(), doc_read.kind(), doc_read.uri())
        };

        if self.last_synced_content != content {
            self.content = content.clone();
            self.kind = kind;
            self.last_synced_content = content;
            // 从 uri 解析文件名作为面包屑(末段 path component)
            self.breadcrumb_text = extract_filename(&uri);
            cx.notify();
        }
    }

    /// 经开放字符串常量比较,支持插件自定义类型扩展。
    #[computed]
    pub fn is_markdown(&self) -> bool {
        self.kind == document_kind::MARKDOWN
    }

    /// HTML 文件降级为源码展示(`<pre>`)。
    #[computed]
    pub fn is_html(&self) -> bool {
        self.kind == document_kind::HTML
    }

    /// 纯文本(无扩展名或 .txt 等)。
    #[computed]
    pub fn is_text(&self) -> bool {
        !self.is_markdown() && !self.is_html()
    }

    /// 文档内容(computed 桥接,避免 SharedString 非 Copy 在绑定中 move 出 &self)。
    #[computed]
    pub fn content_text(&self) -> SharedString {
        self.content.clone()
    }
}

/// 从 uri 字符串解析文件名(末段 path component)。
///
/// `file:///e:/foo/bar.md` → `bar.md`;解析失败返回 "preview"。
fn extract_filename(uri: &str) -> SharedString {
    let path = std::path::Path::new(uri);
    path.file_name()
        .and_then(|n| n.to_str())
        .map(SharedString::from)
        .unwrap_or_else(|| "preview".into())
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:PreviewComponent 需注册 IWorkbenchComponent 能力 cast + 工厂。
// ──────────────────────────────────────────────────────────────────────────

/// 注册 PreviewComponent 能力 cast + 工厂。
///
/// 在 `#[ctor::ctor]` 中调用:
/// 1. `register_workbench_component_ability::<PreviewComponent>()` —— 注册能力 cast
/// 2. `register_workbench_component(factory)` —— 注册工厂到全局注册表
pub fn register_preview_component() {
    register_workbench_component_ability::<PreviewComponent>();
    register_workbench_component(|| {
        Arc::new(PreviewComponent::default()) as Arc<dyn IWorkbenchComponent>
    });
}
