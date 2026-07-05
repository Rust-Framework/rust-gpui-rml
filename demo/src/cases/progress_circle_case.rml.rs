use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.progress_circle",
    kind = "case",
    group = "components",
    order = 27,
)]
#[component]
#[derive(Default)]
pub struct ProgressCircleCase {
    pub current: f32,
}

impl IContribution for ProgressCircleCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.progress_circle.title")
    }
}

impl ILifecycle for ProgressCircleCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.current = 75.0;
        cx.notify();
    }
}

impl ProgressCircleCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<ProgressCircle value={75} />
<ProgressCircle loading="" />"#
            .to_string()
    }
}
