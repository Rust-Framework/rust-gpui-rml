// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;

fn main() {
    // 注册独立扩展组件（ui-term 不在 engine 依赖链中，由使用方注册）
    rml::register_extension_component("Terminal", rml::ComponentTag {
        ctor_path: "rml_ui_term::TerminalView",
        kind: rml::ComponentKind::EntityRef,
        container: false,
    });

    rml::build()
        .scan_dir("src")
        .assets("assets", true)   // true = 嵌入二进制;false = 文件系统模式
        .output_dir(std::env::var("OUT_DIR")
        .expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");

    // Windows 默认主线程栈 1MB。GPUI 深层元素树（多 Tab + 组件嵌套）在 debug 构建下
    // 栈帧较大，prepaint/paint 递归遍历可能溢出。通过 PE header 设置 8MB 栈预留。
    #[cfg(target_os = "windows")]
    println!("cargo:rustc-link-arg=/STACK:8388608");
}
