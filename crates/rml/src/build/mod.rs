//! RML 构建集成
//!
//! 在用户 `build.rs` 中调用，扫描 `.rml`、调用编译器、输出到 `OUT_DIR`。
//! 详见文档 §10.4 构建流程。

pub mod cache;
pub mod scanner;

use crate::compiler::{compile, CodegenCtx};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::PathBuf;

/// 构建错误
#[derive(Debug)]
pub struct BuildError {
    pub message: String,
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RML build error: {}", self.message)
    }
}

impl std::error::Error for BuildError {}

/// Builder 配置器
pub struct Builder {
    scan_dirs: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    namespace: Option<String>,
    strict: bool,
    hot_reload: bool,
    public: bool,
}

/// 入口：创建一个新的 Builder。
///
/// ```rust
/// // build.rs
/// fn main() {
///     rml::build()
///         .scan_dir("src")
///         .output_dir(std::env::var("OUT_DIR").unwrap())
///         .build()
///         .expect("RML build failed");
/// }
/// ```
pub fn build() -> Builder {
    Builder::new()
}

impl Builder {
    pub fn new() -> Self {
        Self {
            scan_dirs: vec![PathBuf::from("src")],
            output_dir: None,
            namespace: None,
            strict: true,
            hot_reload: false,
            public: false,
        }
    }

    /// 添加扫描目录（可多次调用）。默认 `src`。
    pub fn scan_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.scan_dirs.push(dir.into());
        self
    }

    /// 设置输出目录（通常为 `std::env::var("OUT_DIR")`）。
    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.output_dir = Some(dir.into());
        self
    }

    /// 设置命名空间（Phase A 仅记录，不生效）。
    pub fn namespace(mut self, ns: impl Into<String>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    /// 严格模式：把警告升级为错误（默认 true）。
    pub fn strict(mut self, on: bool) -> Self {
        self.strict = on;
        self
    }

    /// 启用热重载（Phase A 仅记录，不生效）。
    pub fn hot_reload(mut self, on: bool) -> Self {
        self.hot_reload = on;
        self
    }

    /// 生成的代码标记为 pub，供下游 crate 使用（Phase A 仅记录，不生效）。
    pub fn public(mut self, on: bool) -> Self {
        self.public = on;
        self
    }

    /// 执行编译主流程。
    pub fn build(self) -> Result<(), BuildError> {
        let output_dir = self.output_dir.ok_or_else(|| BuildError {
            message: "output_dir not set (use .output_dir(std::env::var(\"OUT_DIR\").unwrap()))".into(),
        })?;

        let generated_dir = output_dir.join("rml_generated");
        fs::create_dir_all(&generated_dir).map_err(|e| BuildError {
            message: format!("failed to create {}: {}", generated_dir.display(), e),
        })?;

        // 1. 扫描 .rml 文件
        let rml_files = scanner::scan(&self.scan_dirs);
        for f in &rml_files {
            println!("cargo:rerun-if-changed={}", f.display());
        }

        // 2. 加载缓存
        let cache_path = output_dir.join("rml_cache.json");
        println!("cargo:rerun-if-changed={}", cache_path.display());
        let mut cache = cache::Cache::load(&cache_path);

        // 3. 逐个编译
        for rml_path in &rml_files {
            let source = match fs::read_to_string(rml_path) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("read {}: {}", rml_path.display(), e);
                    println!("cargo:warning=RML error in {}: {}", rml_path.display(), msg);
                    return Err(BuildError { message: msg });
                }
            };

            // 计算哈希，命中缓存则跳过
            let hash = hash_str(&source);
            let key = rml_path.to_string_lossy().to_string();
            if cache.entries.get(&key) == Some(&hash) {
                continue;
            }

            // 文件名 → 视图结构名
            let stem = rml_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("view");
            let view_struct_name = to_pascal_case(stem);
            let snake = to_snake_case(stem);
            let out_file = generated_dir.join(format!("{}.rs", snake));

            let ctx = CodegenCtx {
                view_struct_name: view_struct_name.clone(),
                view_module_path: self.namespace.clone().unwrap_or_default(),
            };

            match compile(&source, &ctx) {
                Ok(code) => {
                    if let Err(e) = fs::write(&out_file, code) {
                        let msg = format!("write {}: {}", out_file.display(), e);
                        println!("cargo:warning=RML error in {}: {}", rml_path.display(), msg);
                        return Err(BuildError { message: msg });
                    }
                    cache.entries.insert(key, hash);
                }
                Err(e) => {
                    println!(
                        "cargo:warning=RML error in {}: {}",
                        rml_path.display(),
                        e
                    );
                    return Err(BuildError {
                        message: format!("compile {}: {}", rml_path.display(), e),
                    });
                }
            }
        }

        // 4. 写回缓存
        if let Err(e) = cache.save(&cache_path) {
            println!("cargo:warning=RML: failed to write cache {}: {}", cache_path.display(), e);
        }

        Ok(())
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

fn hash_str(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// snake_case / kebab-case → PascalCase（如 "counter" → "Counter"，"my_view" → "MyView"）
fn to_pascal_case(s: &str) -> String {
    s.split(|c: char| c == '_' || c == '-')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut chars = p.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// 已是 snake_case 时原样返回；PascalCase → snake_case（与 macros::derive_model 对齐）
fn to_snake_case(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
