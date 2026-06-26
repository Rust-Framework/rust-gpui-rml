// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;

use gpui::px;
use rml_app::RmlApplication;

#[path = "counter.rml.rs"]
mod counter;

fn main() {
    RmlApplication::new()
        .title("RML Counter Demo")
        .size(px(400.), px(300.))
        .run::<counter::Counter>();
}
