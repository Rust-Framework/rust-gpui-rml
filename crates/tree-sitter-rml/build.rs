use std::path::PathBuf;

fn main() {
    let src_dir = PathBuf::from("src");

    cc::Build::new()
        .file(src_dir.join("parser.c"))
        .include(&src_dir)
        .warnings(false)
        .compile("tree-sitter-rml");
}
