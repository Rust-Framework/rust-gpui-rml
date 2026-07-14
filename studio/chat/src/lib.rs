//! Arc Studio IDE 聊天模块。
//!
//! 此 crate 实现:
//! - [`chat_manager`] —— `ChatManager`(`IChatManager` 实现,聚合 IChatProvider)
//! - [`chat_provider`] —— `DefaultChatProvider`/`DefaultChatter`/`ChatWorkbenchProvider`
//! - [`chat_panel`] —— `ChatPanel`(微信风格活动栏面板,IContribution + IVisual)
//! - [`chat_workbench`] —— `ChatWorkbench`(IWorkbench + IWorkbenchComponentHost,纯壳)
//! - [`chat_component`] —— `ChatComponent`(IWorkbenchComponent,聊天交互视图)
//!
//! 服务自注册: `#[ctor::ctor]` 自动注册 ChatManager/ChatWorkbenchProvider 到 DI,
//! ChatPanel 到 ActivityBar,ChatComponent 到工作台组件注册表。

extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;
extern crate studio_core as studio_core;

#[path = "chat_panel.rml.rs"]
pub mod chat_panel;
#[path = "chat_workbench.rml.rs"]
pub mod chat_workbench;
#[path = "chat_component.rml.rs"]
pub mod chat_component;
pub mod chat_manager;
pub mod chat_provider;

/// 自动注册 —— `#[ctor::ctor]` 在 `main` 之前执行:
/// 1. ChatManager → DI singleton(`dyn IChatManager`)
/// 2. ChatWorkbenchProvider → DI keyed singleton(`dyn IWorkbenchProvider("chat")`)
/// 3. DefaultChatProvider → 全局工厂 + 能力 cast
/// 4. ChatWorkbench 能力 cast(IContribution + IVisual + IWorkbench + IWorkbenchComponentHost)
/// 5. ChatComponent 能力 cast + 工厂
/// 6. ChatPanel 能力 cast + ActivityBar 面板工厂
#[rml_core::ctor::ctor]
fn register_chat_services() {
    use std::sync::Arc;
    use rml_core::contribution::IContribution;
    use rml_core::workbench::IWorkbenchProvider;
    use rml_ui::register_activity_panel;
    use rust_rml_di::{auto_register, ServiceCollection};

    // 1. ChatManager + ChatWorkbenchProvider → DI
    auto_register(|s: &mut ServiceCollection| {
        s.add_singleton::<dyn studio_core::chat::IChatManager>(|_| {
            Arc::new(crate::chat_manager::ChatManager::new())
                as Arc<dyn studio_core::chat::IChatManager>
        });
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("chat", |_| {
            Arc::new(crate::chat_provider::ChatWorkbenchProvider) as Arc<dyn IWorkbenchProvider>
        });
    });

    // 2. DefaultChatProvider → 全局工厂 + 能力 cast
    crate::chat_provider::register_default_chat_provider();

    // 3. ChatWorkbench 能力 cast
    crate::chat_workbench::register_chat_workbench_abilities();

    // 4. ChatComponent 能力 cast + 工厂
    crate::chat_component::register_chat_component();

    // 5. ChatPanel 能力 cast + ActivityBar 面板工厂
    crate::chat_panel::register_chat_panel_abilities();
    register_activity_panel(|| {
        Arc::new(crate::chat_panel::ChatPanel::default()) as Arc<dyn IContribution>
    });
}
