//! RML 构建集成
//!
//! 在用户 `build.rs` 中调用，扫描 `.rml`、调用编译器、输出到 `OUT_DIR`。
//! 详见文档 §10.4 构建流程。

pub mod cache;
pub mod scanner;

use crate::compiler::{compile, CodegenCtx};
use crate::css;
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
    style_paths: Vec<PathBuf>,
}

/// 入口：创建一个新的 Builder。
///
/// ```rust,ignore
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
            style_paths: Vec::new(),
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

    /// 注册一个 CSS 样式表文件（可多次调用，按声明顺序合并，后者优先级更高）。
    ///
    /// ```rust,ignore
    /// // build.rs
    /// rml::build()
    ///     .scan_dir("src")
    ///     .with_style("styles/main.css")
    ///     .output_dir(std::env::var("OUT_DIR").unwrap())
    ///     .build()
    ///     .expect("RML build failed");
    /// ```
    pub fn with_style(mut self, path: impl Into<PathBuf>) -> Self {
        self.style_paths.push(path.into());
        self
    }

    /// 执行编译主流程。
    pub fn build(self) -> Result<(), BuildError> {
        // 所有 &self 借用必须在 self.output_dir 移动前完成
        let stylesheet = self.load_stylesheets()?;
        let rml_files = scanner::scan(&self.scan_dirs);
        for f in &rml_files {
            println!("cargo:rerun-if-changed={}", f.display());
        }
        let computed_methods = self.scan_computed_methods(&rml_files);

        // 现在可以移动 output_dir
        let output_dir = self.output_dir.ok_or_else(|| BuildError {
            message: "output_dir not set (use .output_dir(std::env::var(\"OUT_DIR\").unwrap()))".into(),
        })?;

        let generated_dir = output_dir.join("rml_generated");
        fs::create_dir_all(&generated_dir).map_err(|e| BuildError {
            message: format!("failed to create {}: {}", generated_dir.display(), e),
        })?;

        // 2. 加载缓存，并校验 engine 源码哈希
        //    engine 任何 src/**/*.rs 变化会让 engine_source_hash() 返回不同值，
        //    此时缓存中的旧 entries 全部失效，强制重新生成所有 .rml。
        let cache_path = output_dir.join("rml_cache.json");
        println!("cargo:rerun-if-changed={}", cache_path.display());
        let mut cache = cache::Cache::load(&cache_path);
        let current_engine_hash = crate::engine_source_hash().to_string();
        if !cache.is_valid_for_engine(&current_engine_hash) {
            // engine 源码已变化或旧版缓存：失效所有条目，重新生成
            cache.invalidate_all();
            cache.stamp_engine(current_engine_hash.clone());
        }

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
                stylesheet: stylesheet.clone(),
                computed_methods: computed_methods.clone(),
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

        // 4. 写回缓存（包含最新 engine_hash）
        if let Err(e) = cache.save(&cache_path) {
            println!("cargo:warning=RML: failed to write cache {}: {}", cache_path.display(), e);
        }

        Ok(())
    }

    /// 加载所有注册的 CSS 文件，合并为一个全局 StyleSheet。
    ///
    /// 按声明顺序合并：后注册的文件规则追加在末尾（优先级更高）。
    /// `:root` 变量跨文件共享。
    fn load_stylesheets(&self) -> Result<Option<css::StyleSheet>, BuildError> {
        if self.style_paths.is_empty() {
            return Ok(None);
        }
        let mut merged = css::StyleSheet::default();
        for path in &self.style_paths {
            println!("cargo:rerun-if-changed={}", path.display());
            let source = fs::read_to_string(path).map_err(|e| BuildError {
                message: format!("read css {}: {}", path.display(), e),
            })?;
            match css::parse(&source) {
                Ok(sheet) => {
                    // 合并规则（后者追加）
                    merged.rules.extend(sheet.rules);
                    // 合并变量（后者覆盖）
                    merged.variables.extend(sheet.variables);
                }
                Err(e) => {
                    let msg = format!("parse css {}: {}", path.display(), e);
                    println!("cargo:warning=RML: {}", msg);
                    return Err(BuildError { message: msg });
                }
            }
        }
        Ok(Some(merged))
    }

    /// 扫描 `.rml.rs` code-behind 文件，收集 `#[computed]` 标注的方法名。
    ///
    /// 使用正则匹配 `#[computed]` 后的 `fn <name>` 模式。
    /// 这些方法名传给 codegen，使 `{name}` 生成 `self.name()` 而非 `self.name`。
    fn scan_computed_methods(&self, rml_files: &[PathBuf]) -> Vec<String> {
        let mut methods = Vec::new();
        for rml_path in rml_files {
            // .rml → .rml.rs（在路径后追加 .rs）
            let rml_rs: PathBuf = format!("{}.rs", rml_path.display()).into();
            if !rml_rs.exists() {
                continue;
            }
            println!("cargo:rerun-if-changed={}", rml_rs.display());
            let source = match fs::read_to_string(&rml_rs) {
                Ok(s) => s,
                Err(_) => continue,
            };
            // 简化扫描：查找 #[computed] 后面紧跟的 fn name
            let lines: Vec<&str> = source.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                if line.trim().contains("#[computed]") {
                    // 下一行或同行后面应该有 fn <name>
                    let search_in = if i + 1 < lines.len() {
                        format!("{} {}", line, lines[i + 1])
                    } else {
                        line.to_string()
                    };
                    if let Some(name) = extract_fn_name(&search_in) {
                        if !methods.contains(&name) {
                            methods.push(name);
                        }
                    }
                }
            }
        }
        methods
    }
}

/// 从源码行中提取 `fn <name>` 的方法名
fn extract_fn_name(source: &str) -> Option<String> {
    let after_computed = source.split("#[computed]").nth(1)?;
    let after_fn = after_computed.split("fn").nth(1)?;
    let name_part = after_fn.trim_start();
    let name: String = name_part
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    if name.is_empty() {
        None
    } else {
        Some(name)
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
