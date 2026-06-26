//! Engine crate 自身的 build.rs
//!
//! 目的：计算 engine 源码哈希并写入 `OUT_DIR/rml_engine_hash.txt`，
//! 供下游 build.rs 通过 `engine_source_hash()` 读取，用于失效 RML 增量缓存。
//!
//! 工作流：
//! 1. 任何 src/**/*.rs 变化 → cargo 检测 `cargo:rerun-if-changed=src` 重新执行本脚本
//! 2. 本脚本重算 sha256 合并哈希 → 写入 OUT_DIR/rml_engine_hash.txt
//! 3. engine lib.rs 通过 include_str! 嵌入新哈希 → 重新编译
//! 4. 下游 build.rs 链接新 engine → 调用 engine_source_hash() 获取新哈希
//! 5. 下游 build.rs 发现哈希变化 → 失效所有 .rml 缓存条目 → 重新生成

use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

fn main() {
    // 让 cargo 在 src/ 下任何文件变化时重新执行本脚本
    println!("cargo:rerun-if-changed=src");
    // build.rs 自身变化时也重新执行
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR not set");
    let src_dir = PathBuf::from(&manifest_dir).join("src");

    // 递归收集 src/**/*.rs，排序以保证哈希稳定
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    files.sort();

    // 计算合并 sha256：每个文件路径 + 内容都参与哈希
    let mut hasher = Sha256::new();
    for path in &files {
        let rel = path.strip_prefix(&src_dir).unwrap_or(path);
        hasher.update(rel.to_string_lossy().as_bytes());
        hasher.update(b"\0");
        match fs::read(path) {
            Ok(content) => hasher.update(&content),
            Err(_) => hasher.update(b"<read-error>"),
        }
        hasher.update(b"\0");
    }
    let hash = format!("{:x}", hasher.finalize());

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = PathBuf::from(&out_dir).join("rml_engine_hash.txt");
    if let Err(e) = fs::write(&out_path, &hash) {
        panic!("failed to write {}: {}", out_path.display(), e);
    }

    // 调试用：在 cargo:warning 中可见
    // println!("cargo:warning=RML engine source hash: {}", hash);
}

fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
