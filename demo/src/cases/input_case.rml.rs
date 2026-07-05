use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::InputState;

#[contribute(
    host_id = "demo.shell",
    id = "components.input",
    kind = "case",
    group = "components",
    order = 35,
)]
#[component]
#[derive(Default)]
pub struct InputCase {
    pub input_state: Option<gpui::Entity<InputState>>,
}

impl IContribution for InputCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.input.title")
    }
}

impl ILifecycle for InputCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.input_state = Some(cx.new(|cx| {
            InputState::new(_window, cx)
                .placeholder("请输入内容")
                .default_value("Hello RML")
        }));
    }
}

impl InputCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Input placeholder="请输入内容" />"#.to_string()
    }
}
