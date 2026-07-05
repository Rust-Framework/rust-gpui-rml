use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "components.progress",
    kind = "case",
    group = "components",
    order = 26,
)]
#[component]
#[derive(Default)]
pub struct ProgressCase {
    pub current: f32,
}

impl IContribution for ProgressCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.progress.title")
    }
}

impl ILifecycle for ProgressCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.current = 60.0;
        cx.notify();
    }
}

impl ProgressCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Progress value={60} />
<Progress loading="" />
<Progress value={current} />"#
            .to_string()
    }
}
