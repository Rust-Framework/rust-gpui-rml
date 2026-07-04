// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

mod app;
mod cases;
mod lsp;
mod shell;

// `#[rml::main]` 自动注入 `rml::embed_assets!()`（include build.rs 生成的 rml_assets.rs）。
// 生成文件内的 `#[ctor::ctor]` 函数在 main 之前自动调用 `rml_core::assets::init(...)`,
// 因此此处无需手写资源初始化代码。模式（嵌入/文件系统）由 build.rs 的 `.assets(path, embed)` 决定。
#[rml::main]
fn main() {
    // Program.cs 风格：显式 builder 链，框架自动管理主窗口创建与生命周期
    rml_app::RmlApplication::new()
        .main_window::<shell::MainWindow>()
        .run::<app::Startup>();
}
