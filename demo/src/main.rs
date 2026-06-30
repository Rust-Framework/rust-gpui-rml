// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

mod app;
mod cases;
mod login;
mod shell;

// 一键启动:内部完成资源嵌入、资源注册、应用启动
rml::main!(app::Startup);
