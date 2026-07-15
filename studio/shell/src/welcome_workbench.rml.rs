//! WelcomeWorkbench ViewModel —— `rml://welcome` 工作台,渲染欢迎页。
//!
//! `#[component(workbench)]` 生成 RML 框架契约 + URI 键缓存版 `impl IVisual`,
//! 在 `Render::render` 之前自动调用 `sync_from_external` 同步外部实例 URI。

use std::any::Any;
use std::sync::Once;

use gpui::{App, SharedString};
use rml::prelude::*;
use rml_app::contribution::evict_entity_by_uri;
use rml_core::contribution::{
    IContribution, register_contribution_ability, register_visual_ability,
};
use rml_core::workbench::{IWorkbench, register_workbench_ability};

/// `rml://welcome` 的工作台 —— 常驻 Tab,不可关闭。
///
/// `#[component(workbench)]` 生成 URI 键缓存版 IVisual + 自动 sync_from_external。
/// 手动 impl IContribution + ILifecycle(sync_from_external) + IWorkbench(on_closing)。
#[component(workbench)]
#[derive(Default)]
pub struct WelcomeWorkbench {
    uri: SharedString,
}

impl WelcomeWorkbench {
    /// 构造带 URI 的实例（由 WelcomeProvider 调用）。
    pub fn new(uri: SharedString) -> Self {
        let mut this = Self::default();
        this.uri = uri;
        this
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

impl ILifecycle for WelcomeWorkbench {
    fn sync_from_external(&mut self, external: &Self, _cx: &mut Context<Self>) {
        self.uri = external.uri.clone();
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

    fn on_closing(&self, cx: &mut App) {
        evict_entity_by_uri::<Self>(self.uri(), cx);
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:WelcomeWorkbench 需注册 IContribution + IVisual + IWorkbench
//  能力 cast,使 MainWindow 的 as_visual() / as_workbench() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_welcome_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<WelcomeWorkbench>();
        register_visual_ability::<WelcomeWorkbench>();
        register_workbench_ability::<WelcomeWorkbench>();
    });
}
