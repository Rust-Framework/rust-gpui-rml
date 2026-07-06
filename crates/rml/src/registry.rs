//! 一行注册 RML 语言到 gpui-component LanguageRegistry（静态着色）。
//!
//! 在 `Startup::on_launch` 中调用 `rust_rml_client::register_rml_language()`
//! 即可启用 tree-sitter 静态着色层。

pub fn register_rml_language() {
    use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
    LanguageRegistry::singleton().register(
        "rml",
        &LanguageConfig::new(
            "rml",
            tree_sitter::Language::new(crate::grammar::language()),
            vec!["rust".into()],
            crate::grammar::HIGHLIGHTS_QUERY,
            crate::grammar::INJECTIONS_QUERY,
            "",
        ),
    );
}
