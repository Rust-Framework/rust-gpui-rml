// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
// `rml` 别名是 `#[rust_rml_engine::main]` 宏展开后生成代码的约定（`rml::embed_assets!()` 等）
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate studio_shell as studio_shell;

use gpui::App;
use rml_app::{IAppLifecycle, RmlApplication};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

/// 应用启动引导 —— 配置应用级资源(样式 / i18n / 主题)。
#[derive(Default)]
struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");
    }
}

// `#[rust_rml_engine::main]` 自动注入 `rml::embed_assets!()` + `rml::embed_contributions!()`
// (include build.rs 生成的 rml_assets.rs / rml_contributions.rs)
#[rust_rml_engine::main]
fn main() {
    // Program.cs 风格：显式 builder 链，框架自动管理主窗口创建与生命周期
    RmlApplication::new()
        .main_window::<studio_shell::MainWindow>()
        .run::<Startup>();
}
