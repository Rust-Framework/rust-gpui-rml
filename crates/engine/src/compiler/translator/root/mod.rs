//! 根节点 translator
//!
//! 处理 RML 顶层节点：`<window>`、`<modern-window>`、`<tab-window>`、`<dialog>`、`<component>`。
//! 每个根节点对应一个 translator，通过 `TranslatorRegistry` 统一注册与路由。

pub mod component_root;
pub mod dialog;
pub mod modern_window;
pub mod tab_window;
pub mod window;

/// 注册所有根节点 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    window::register_all(registry);
    modern_window::register_all(registry);
    tab_window::register_all(registry);
    dialog::register_all(registry);
    component_root::register_all(registry);
}
