// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

mod app;
mod cases;
mod shell;

// 嵌入 assets/ 资源到二进制（由 build.rs 生成 RML_ASSETS 注册表）
rml::embed_assets!();

fn main() {
    // Program.cs 风格：显式 builder 链，框架自动管理主窗口创建与生命周期
    rml_app::RmlApplication::new()
        .assets(RML_ASSETS)
        .main_window::<shell::MainWindow>()
        .run::<app::Startup>();
}
