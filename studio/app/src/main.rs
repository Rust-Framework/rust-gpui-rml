// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
// `rml` 别名是 `#[rml::main]` 宏展开后生成代码的约定（`rml::embed_assets!()` 等）
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
// 强制链接 feature crate —— 仅需其 `#[ctor::ctor]` 自注册副作用（Provider + abilities + 面板/工作空间工厂）
extern crate studio_editor as _;
extern crate studio_explorer as _;
extern crate studio_chat as _;

mod startup;

use rml_app::RmlApplication;

// `#[rml::main]` 自动注入 `rml::embed_assets!()` + `rml::embed_contributions!()`
// (include build.rs 生成的 rml_assets.rs / rml_contributions.rs)
#[rml::main]
fn main() {
    // Program.cs 风格：显式 builder 链，框架自动管理主窗口创建与生命周期
    // DI 容器由 MainWindow::on_loaded → di::build_runtime_provider 二阶段构建
    RmlApplication::new()
        .main_window::<studio_shell::MainWindow>()
        .run::<startup::Startup>();
}
