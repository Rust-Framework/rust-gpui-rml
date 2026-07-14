//! ChatWorkbench ViewModel —— IWorkbench + IWorkbenchComponentHost,纯壳。
//!
//! 聊天工作台按 `chat://` URI 路由,管理单一 `ChatComponent`(内部复用现成
//! `rml_ui::ChatPanel`)。ChatWorkbench 仅负责:
//! 1. 资源会话管理(IWorkbench):uri/close/activate/closable + preview
//! 2. 组件宿主管理(IWorkbenchComponentHost):枚举/激活/切换 + 共享文档/状态
//! 3. Body 容器:经条件分支渲染激活的 IWorkbenchComponent
//!
//! `#[component]` 生成 `impl IModel + IViewModel + IComponent + Render`。
//! 手动 `impl IContribution + IVisual + ILifecycle + IWorkbench + IWorkbenchComponentHost`
//! 补充元数据 + 渲染入口 + 初始化 + 资源会话 + 组件宿主。

use std::any::Any;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Once};

use gpui::{AnyElement, App, Entity, SharedString, Window};
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{IconSpec, register_contribution_ability, register_visual_ability};
use rml_core::context::ServiceProviderExt;
use rml_core::workbench::{IWorkbench, Uri, register_workbench_ability};
use studio_core::ability_ext::register_workbench_component_host_ability;
use studio_core::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use studio_core::document::{WorkbenchDocument, WorkbenchState};

use crate::chat_component::ChatComponent;

/// 聊天工作台 —— IWorkbench + IWorkbenchComponentHost,纯壳。
///
/// `#[component]` 生成 RML 框架契约(IModel/IViewModel/IComponent/Render),
/// 经 `include!` 引入编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 impl:
/// - `IContribution + IVisual` —— 元数据 + 渲染入口(委托 Render)
/// - `ILifecycle` —— on_loaded 初始化共享 document/state + chat_component
/// - `IWorkbench` —— 资源会话管理(uri/close/activate/preview)
/// - `IWorkbenchComponentHost` —— 组件枚举/激活/切换 + 共享文档/状态
#[component]
#[derive(Default)]
pub struct ChatWorkbench {
    /// chatter URI(`chat://{provider_id}/{chatter_id}`)。
    uri: SharedString,
    /// 从 IChatManager 解析的 chatter 名称(供 ChatComponent 读取并 set_title)。
    pub chatter_name: SharedString,
    /// 共享文档模型 —— URI 传递媒介(MVP content 为空,无聊天历史持久化)。
    document: Option<Entity<WorkbenchDocument>>,
    /// 共享工作台状态。
    state: Option<Entity<WorkbenchState>>,
    /// 当前激活的 IWorkbenchComponent id(默认 "chat")。
    active_component_id: SharedString,
    /// 聊天交互子组件 —— 经 RML `<ChatComponent if={is_chat_active} />` 引用。
    chat_component: Option<Entity<ChatComponent>>,
    /// 预览模式标记(VSCode 风格 Tab:italic 标题,双击升级为正式)。
    preview: Arc<AtomicBool>,
}

impl IContribution for ChatWorkbench {
    fn id(&self) -> &str {
        &self.uri
    }
    fn name(&self) -> SharedString {
        if self.chatter_name.is_empty() {
            "Chat".into()
        } else {
            self.chatter_name.clone()
        }
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("MessageCircle"))
    }
}

impl IVisual for ChatWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<ChatWorkbench>(cx);
        // get_or_create_entity 按 TypeId 缓存,所有 ChatWorkbench 实例共享同一 Entity。
        // 当 URI 变化时(打开不同 chatter),需重新加载:解析 chatter_name + document.reload。
        let uri = self.uri.clone();
        entity.update(cx, |this, ctx| {
            let uri_changed = this.uri != uri;
            this.uri = uri;
            if uri_changed {
                this.reload(ctx);
            }
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for ChatWorkbench {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 初始化共享 document + state(IWorkbenchComponent 间数据同步媒介)
        self.document = Some(cx.new(|_| WorkbenchDocument::default()));
        self.state = Some(cx.new(|_| WorkbenchState::default()));
        // 初始化 chat_component 子组件(RML 模板经 <ChatComponent /> 引用)
        self.chat_component = Some(cx.new(|_| ChatComponent::default()));
    }
}

impl IWorkbench for ChatWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }

    fn close(&self) {
        // 聊天工作台关闭时无特殊资源释放(ChatPanel Entity drop 自动释放)
    }

    fn activate(&self) {
        // 聊天工作台获得焦点
    }

    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}

    fn closable(&self) -> bool {
        true
    }

    fn preview(&self) -> bool {
        self.preview.load(Ordering::Relaxed)
    }

    fn set_preview(&self, preview: bool) {
        self.preview.store(preview, Ordering::Relaxed);
    }
}

impl IWorkbenchComponentHost for ChatWorkbench {
    fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>> {
        // MVP: 单一 ChatComponent,直接构造。
        // 不从 get_workbench_components() 获取 —— 避免 CodeComponent(matches=all) 出现。
        vec![Arc::new(ChatComponent::default()) as Arc<dyn IWorkbenchComponent>]
    }

    fn active_component_id(&self) -> SharedString {
        self.active_component_id.clone()
    }

    fn switch_component(&self, id: &str, cx: &mut App) {
        let entity = get_or_create_entity::<ChatWorkbench>(cx);
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

impl ChatWorkbench {
    /// 当前激活的组件是否为 chat(ChatComponent)。
    ///
    /// RML 模板经 `<ChatComponent if={is_chat_active} />` 条件渲染。
    #[computed]
    pub fn is_chat_active(&self) -> bool {
        self.active_component_id == "chat" || self.active_component_id.is_empty()
    }

    /// 设置 chatter URI(由 ChatWorkbenchProvider 在构造后调用)。
    pub fn set_uri(&mut self, uri: SharedString) {
        self.uri = uri;
    }

    /// 重新加载:从 IChatManager 解析 chatter_name → document.reload → 默认激活 "chat"。
    ///
    /// 在 `IVisual::render` 中检测 `uri_changed` 时调用。Tab 切换时共享 Entity 不重建,
    /// 经此方法同步新 chatter 的元数据到 document,ChatComponent observe 变化重新初始化。
    fn reload(&mut self, cx: &mut Context<Self>) {
        // 默认激活 chat 组件(切换 Tab 时仅首次激活,后续保留用户选择)
        if self.active_component_id.is_empty() {
            self.active_component_id = "chat".into();
        }

        // 从 IChatManager 解析 chatter_name(按 uri 查找 IChatter)
        if let Some(mgr) = cx.get_trait::<dyn studio_core::chat::IChatManager>() {
            if let Some(chatter) = mgr.find_chatter(&self.uri) {
                self.chatter_name = chatter.name();
            }
        }

        // document.reload(触发 ChatComponent 同步 URI 变化)
        if let Some(doc) = self.document.as_ref() {
            doc.update(cx, |d, _| {
                d.reload(self.uri.clone(), SharedString::default(), "chat");
            });
        }

        cx.notify();
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:ChatWorkbench 需注册 IContribution + IVisual + IWorkbench
//  + IWorkbenchComponentHost 能力 cast,使 MainWindow 的 as_visual() /
//  as_workbench() / as_workbench_component_host() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_chat_workbench_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<ChatWorkbench>();
        register_visual_ability::<ChatWorkbench>();
        register_workbench_ability::<ChatWorkbench>();
        register_workbench_component_host_ability::<ChatWorkbench>();
    });
}
