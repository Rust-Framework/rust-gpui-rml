//! 二进制入口：`rml-lsp --stdio`

use rml_lsp::run_server;

fn main() -> anyhow::Result<()> {
    // 简单参数校验：仅支持 --stdio（默认模式）
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] != "--stdio" {
        eprintln!("rml-lsp: unsupported argument '{}', only --stdio is supported", args[1]);
        std::process::exit(1);
    }
    run_server()
}
