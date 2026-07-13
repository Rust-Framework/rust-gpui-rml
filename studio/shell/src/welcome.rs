//! WelcomeWorkbench —— Arc Studio 启动欢迎页。
//!
//! `rml://welcome` URI 的工作台,渲染静态欢迎信息。
//! 经 `WelcomeProvider`(schema="rml")由 `ArcShellManager::open` 路由构造。

use std::any::Any;
use std::sync::{Arc, Once};

use gpui::{AnyElement, App, IntoElement, ParentElement, SharedString, Styled, Window};
use rml_core::contribution::{
    IContribution, IVisual, register_contribution_ability, register_visual_ability,
};
use rml_core::workbench::{IWorkbench, IWorkbenchProvider, Uri, register_workbench_ability};

/// `rml://welcome` 的工作台 —— 常驻 Tab,不可关闭。
pub struct WelcomeWorkbench {
    uri: SharedString,
}

impl WelcomeWorkbench {
    pub fn new(uri: SharedString) -> Self {
        Self { uri }
    }
}

impl IContribution for WelcomeWorkbench {
    fn id(&self) -> &str {
        &self.uri
    }
    fn name(&self) -> SharedString {
        "Welcome".into()
    }
}

impl IVisual for WelcomeWorkbench {
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        gpui::div()
            .flex()
            .items_center()
            .justify_center()
            .size_full()
            .child(gpui::div().text_xl().child("Welcome to Arc Studio"))
            .into_any_element()
    }
}

impl IWorkbench for WelcomeWorkbench {
    fn uri(&self) -> &str {
        &self.uri
    }
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn Any + Send + Sync>) {}

    /// 欢迎页常驻,不显示关闭按钮。
    fn closable(&self) -> bool {
        false
    }
}

/// `rml://` URI 的工作台工厂 —— 构造 WelcomeWorkbench。
pub struct WelcomeProvider;

impl IContribution for WelcomeProvider {
    fn id(&self) -> &str {
        "welcome-provider"
    }
    fn name(&self) -> SharedString {
        "Welcome Provider".into()
    }
}

impl IWorkbenchProvider for WelcomeProvider {
    fn schema(&self) -> SharedString {
        "rml".into()
    }
    fn render(&self, uri: &Uri) -> Arc<dyn IWorkbench> {
        Arc::new(WelcomeWorkbench::new(uri.as_str().into()))
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:WelcomeWorkbench 需注册 IContribution + IVisual + IWorkbench
//  能力 cast,使 MainWindow 的 as_visual() / as_workbench() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub(crate) fn register_welcome_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<WelcomeWorkbench>();
        register_visual_ability::<WelcomeWorkbench>();
        register_workbench_ability::<WelcomeWorkbench>();
    });
}
