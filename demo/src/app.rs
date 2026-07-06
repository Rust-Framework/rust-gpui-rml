//! 应用启动引导 —— UI 初始化与贡献点注册由框架处理；此处仅配置应用级资源

use gpui::App;
use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use rml_app::IAppLifecycle;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use tree_sitter_rml::{language, HIGHLIGHTS_QUERY, INJECTIONS_QUERY};

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");

        LanguageRegistry::singleton().register(
            "rml",
            &LanguageConfig::new(
                "rml",
                tree_sitter::Language::new(language()),
                vec!["rust".into()],
                HIGHLIGHTS_QUERY,
                INJECTIONS_QUERY,
                "",
            ),
        );
    }
}
