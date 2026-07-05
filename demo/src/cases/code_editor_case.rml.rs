use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::InputState;

const DEFAULT_CODE: &str = r#"// RML CodeEditor demo
// 基于 InputState.code_editor("rust").multi_line(true)
// 自动应用 mono 字体 + size_full

fn main() {
    println!("Hello, RML!");
}
"#;

#[contribute(
    host_id = "demo.shell",
    id = "components.code_editor",
    kind = "case",
    group = "components",
    order = 38,
)]
#[component]
#[derive(Default)]
pub struct CodeEditorCase {
    pub editor_state: Option<gpui::Entity<InputState>>,
}

impl IContribution for CodeEditorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.code_editor.title")
    }
}

impl ILifecycle for CodeEditorCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.editor_state = Some(cx.new(|cx| {
            InputState::new(_window, cx)
                .code_editor("rust")
                .multi_line(true)
                .default_value(DEFAULT_CODE)
        }));
    }
}

impl CodeEditorCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<CodeEditor ref="editor_state" />"#.to_string()
    }
}
