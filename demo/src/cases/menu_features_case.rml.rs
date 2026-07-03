use std::sync::Arc;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::command::RelayCommand;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.menu.features",
    kind = "case",
    group = "menu",
    order = 19,
)]
#[component]
#[derive(Default)]
pub struct MenuFeaturesCase {
    pub is_checked: bool,
    pub last_action: String,
    /// B-1 demo：声明式命令绑定字段。
    /// RML `<menu-item command={save_command} />` 据此生成 clone-Arc-out 闭包，
    /// 点击时经 `ICommand::execute` 调度（区别于 `onclick={method}` 的强类型直接调用）。
    /// 类型为 `Arc<RelayCommand>`（具体类型）而非 `Arc<dyn ICommand>`，以便
    /// `#[derive(Default)]` 生效——框架已为 `RelayCommand` 实现 `Default`（no-op 空对象）。
    pub save_command: Arc<RelayCommand>,
}

impl IContribution for MenuFeaturesCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.menu.features.title").into()
    }
}

impl ILifecycle for MenuFeaturesCase {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 初始化 save_command：点击时设置 last_action（复用现有 status 展示链路）
        self.save_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.last_action = "Save command executed".to_string();
            cx.notify();
        }));
    }
}

impl MenuFeaturesCase {
    #[computed]
    pub fn features_status(&self) -> String {
        self.last_action.clone()
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<dropdown-menu scrollable="" max_h="280">
    <Button label="Features" ghost="" />
    <menu-item label="Available" onclick={on_available} />
    <menu-item label="Disabled" disabled="" onclick={on_disabled} />
    <menu-item label="Checkable" checked={is_checked} onclick={on_toggle_check} />
    <menu-separator />
    <menu-item label="Docs" href="https://..." icon="Info" />
    <menu-item label="Submenu">
        <menu-item label="Item A" onclick={on_nested_a} />
    </menu-item>
</dropdown-menu>"#
            .to_string()
    }

    #[command]
    pub fn on_available(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Available".to_string();
    }

    #[command]
    pub fn on_disabled(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Disabled (should not fire)".to_string();
    }

    #[command]
    pub fn on_toggle_check(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.is_checked = !self.is_checked;
        self.last_action = format!("Checked: {}", self.is_checked);
    }

    #[command]
    pub fn on_nested_a(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested A".to_string();
    }

    #[command]
    pub fn on_nested_b(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.last_action = "Nested B".to_string();
    }
}
