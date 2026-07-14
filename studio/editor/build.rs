extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}