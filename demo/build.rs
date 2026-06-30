// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .assets_dir("assets")
        .output_dir(std::env::var("OUT_DIR")
        .expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
