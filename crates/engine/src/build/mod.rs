//! RML 构建集成
//!
//! 在用户 `build.rs` 中调用，扫描 `.rml`、调用编译器、输出到 `OUT_DIR`。
//! 详见文档 §10.4 构建流程。

pub mod assets_processor;
pub mod cache;
pub mod contribution_generator;
pub mod i18n_extractor;
pub mod scanner;

pub use assets_processor::{AssetMode, AssetsProcessor};
pub use i18n_extractor::I18nExtractor;

use crate::compiler::{compile, CodegenCtx, UserComponentInfo};
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
    i18n_extract: Option<PathBuf>,
    assets_dir: Option<PathBuf>,
    assets_mode: AssetMode,
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
            i18n_extract: None,
            assets_dir: None,
            assets_mode: AssetMode::Filesystem,
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

    /// 扫描 `.rml` 中的 `t("key")` 并合并写入 i18n JSON（缺失 key 以 key 为默认值）
    pub fn extract_i18n(mut self, path: impl Into<PathBuf>) -> Self {
        self.i18n_extract = Some(path.into());
        self
    }

    /// 注册资源根目录并指定是否嵌入二进制
    ///
    /// - `embed=true`：所有文件经 `include_bytes!` 编译期嵌入二进制（无资源泄露,
    ///   二进制较大）
    /// - `embed=false`：运行期按需从磁盘读取,首次读取后 `Box::leak` 缓存到 `'static`
    ///   （二进制小,符合方案 2「不关心资源泄露」）
    ///
    /// 两种模式运行时 API 一致,均通过 `rml_core::assets::load(path)` 查询,
    /// 路径以相对 `assets/` 的正斜杠形式(如 `"themes/dark.css"`、`"i18n/zh-CN.json"`)。
    /// 资源注册由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动完成,
    /// main.rs 中无需调用 `embed_assets!()` 或 `RmlApplication::assets()`。
    ///
    /// ```rust,ignore
    /// // build.rs (嵌入模式)
    /// rml::build()
    ///     .scan_dir("src")
    ///     .assets("assets", true)
    ///     .output_dir(std::env::var("OUT_DIR").unwrap())
    ///     .build()
    ///
    /// // build.rs (文件系统模式)
    /// rml::build()
    ///     .scan_dir("src")
    ///     .assets("assets", false)
    ///     .output_dir(std::env::var("OUT_DIR").unwrap())
    ///     .build()
    /// ```
    pub fn assets(mut self, dir: impl Into<PathBuf>, embed: bool) -> Self {
        self.assets_dir = Some(dir.into());
        self.assets_mode = if embed {
            AssetMode::Embedded
        } else {
            AssetMode::Filesystem
        };
        self
    }

    /// 与 [`extract_i18n`] 相同，提供文档中的 `I18nExtractor` 命名
    pub fn plugin(self, extractor: I18nExtractor) -> Self {
        self.extract_i18n(extractor.path().to_path_buf())
    }

    /// 执行编译主流程。
    pub fn build(self) -> Result<(), BuildError> {
        // 所有 &self 借用必须在 self.output_dir 移动前完成
        let stylesheet = self.load_stylesheets()?;
        let rml_files = scanner::scan(&self.scan_dirs);
        for f in &rml_files {
            println!("cargo:rerun-if-changed={}", f.display());
        }

        // Phase B-2：syn 扫描 .rml.rs code-behind，提取每个 struct 的元信息
        // 元信息按 struct_name 索引，供每个 .rml 文件按 view_struct_name 查询
        let struct_metas = self.scan_struct_metas(&rml_files);

        // 收集用户自定义组件注册表（所有 #[component] 标注的 struct）
        // 供 codegen 在 component_lookup 未命中时生成 self.<field>.as_ref().expect(...).clone()
        let user_components: std::collections::HashMap<String, UserComponentInfo> = struct_metas
            .iter()
            .filter(|(_, m)| m.is_component)
            .map(|(name, meta)| {
                (
                    name.clone(),
                    UserComponentInfo {
                        struct_name: name.clone(),
                        entity_field: to_snake_case(name),
                        slots: meta.slots.clone(),
                    },
                )
            })
            .collect();

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
        let mut rml_sources: Vec<String> = Vec::new();
        for rml_path in &rml_files {
            let source = match fs::read_to_string(rml_path) {
                Ok(s) => s,
                Err(e) => {
                    let msg = format!("read {}: {}", rml_path.display(), e);
                    println!("cargo:warning=RML error in {}: {}", rml_path.display(), msg);
                    return Err(BuildError { message: msg });
                }
            };

            rml_sources.push(source.clone());

            // 计算哈希
            let hash = hash_str(&source);
            let key = rml_path.to_string_lossy().to_string();

            // 计算 .rml.rs code-behind 哈希（若存在）
            let rml_rs_path: PathBuf = format!("{}.rs", rml_path.display()).into();
            let current_cb_hash = if rml_rs_path.exists() {
                match fs::read_to_string(&rml_rs_path) {
                    Ok(s) => Some(hash_str(&s)),
                    Err(_) => None,
                }
            } else {
                None
            };

            // 缓存命中条件：.rml 源哈希匹配 AND code-behind 哈希匹配
            // （任一不匹配则需重新生成，确保 computed_methods 等上下文变化生效）
            let rml_unchanged = cache.entries.get(&key) == Some(&hash);
            let cb_unchanged = match &current_cb_hash {
                Some(h) => cache.is_codebehind_unchanged(&key, h),
                None => cache.is_codebehind_unchanged(&key, ""), // .rml.rs 不存在时检查缓存中是否也无
            };
            if rml_unchanged && cb_unchanged {
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

            // 查找当前 struct 的元信息（按 view_struct_name 索引）
            // .rml.rs 不存在或无对应 struct 时使用空元信息
            let struct_meta = struct_metas
                .get(&view_struct_name)
                .cloned()
                .unwrap_or_default();

            let ctx = CodegenCtx {
                view_struct_name: view_struct_name.clone(),
                view_module_path: self.namespace.clone().unwrap_or_default(),
                stylesheet: stylesheet.clone(),
                computed_methods: struct_meta.computed_methods.clone(),
                observable_fields: struct_meta.observable_fields.clone(),
                version_fields: struct_meta.version_fields.clone(),
                computed_deps: struct_meta.computed_deps.clone(),
                computed_returns: struct_meta.computed_returns.clone(),
                field_types: struct_meta.field_types.clone(),
                field_validations: struct_meta.field_validations.clone(),
                model_fields: Vec::new(),
                user_components: user_components.clone(),
                is_contributehost: struct_meta.is_contributehost,
                contribution_bindings: struct_meta.contribution_bindings,
            };

            match compile(&source, &ctx) {
                Ok(code) => {
                    if let Err(e) = fs::write(&out_file, code) {
                        let msg = format!("write {}: {}", out_file.display(), e);
                        println!("cargo:warning=RML error in {}: {}", rml_path.display(), msg);
                        return Err(BuildError { message: msg });
                    }
                    cache.entries.insert(key.clone(), hash);
                    if let Some(h) = current_cb_hash {
                        cache.stamp_codebehind(key, h);
                    } else {
                        // .rml.rs 不存在：标记为空字符串，避免下次因 cache miss 反复重新生成
                        cache.stamp_codebehind(key, String::new());
                    }
                }
                Err(e) => {
                    println!(
                        "cargo:warning=RML error in {}: {}",
                        rml_path.display(), e
                    );
                    return Err(BuildError {
                        message: format!("compile {}: {}", rml_path.display(), e),
                    });
                }
            }
        }

        if let Some(i18n_path) = &self.i18n_extract {
            println!("cargo:rerun-if-changed={}", i18n_path.display());
            let extractor = I18nExtractor::new(i18n_path.clone());
            let refs: Vec<&str> = rml_sources.iter().map(|s| s.as_str()).collect();
            if let Err(e) = extractor.extract_from_sources(&refs) {
                return Err(BuildError { message: e });
            }
        }

        // 4. 写回缓存（包含最新 engine_hash）
        if let Err(e) = cache.save(&cache_path) {
            println!("cargo:warning=RML: failed to write cache {}: {}", cache_path.display(), e);
        }

        // 5. 生成 assets 资源注册代码（按 mode 决定嵌入或文件系统模式）
        let assets_dir = self
            .assets_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("assets"));
        if assets_dir.exists() {
            println!("cargo:rerun-if-changed={}", assets_dir.display());
        }
        let processor = AssetsProcessor::new(&assets_dir, self.assets_mode);
        if let Err(e) = processor.generate(&output_dir) {
            return Err(e);
        }

        // 6. 扫描 `#[contributehost]` / `#[contribute]` 并生成统一注册函数
        let (hosts, contributions) =
            contribution_generator::scan_contribution_registrars(&self.scan_dirs);
        if let Err(e) = contribution_generator::generate(&hosts, &contributions, &output_dir) {
            return Err(e);
        }

        Ok(())
    }

    /// 加载所有注册的 CSS 文件，合并为一个全局 StyleSheet。
    ///
    /// 按声明顺序合并：后注册的文件规则追加在末尾（优先级更高）。
    /// `:root` 变量跨文件共享。
    ///
    /// 除了通过 `.with_style()` 显式注册的文件外,还会自动扫描 `assets_dir` 根目录下的
    /// `.css` 文件(不递归子目录,避免误加载 `themes/` 等主题文件)。
    fn load_stylesheets(&self) -> Result<Option<css::StyleSheet>, BuildError> {
        let mut all_paths = self.style_paths.clone();

        // 自动发现 assets/ 根目录下的 CSS 文件(不递归,排除 themes/ 等子目录)
        let assets_dir = self
            .assets_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("assets"));
        if assets_dir.exists() {
            if let Ok(entries) = fs::read_dir(&assets_dir) {
                let mut auto_css: Vec<PathBuf> = entries
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| {
                        p.is_file() && p.extension().map(|e| e == "css").unwrap_or(false)
                    })
                    .collect();
                auto_css.sort();
                all_paths.extend(auto_css);
            }
        }

        if all_paths.is_empty() {
            return Ok(None);
        }
        let mut merged = css::StyleSheet::default();
        for path in &all_paths {
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

    /// 扫描所有 `.rml.rs` code-behind 文件，提取每个 struct 的元信息。
    ///
    /// 返回扁平化 map：`HashMap<struct_name, StructMetadata>`。
    /// 多个 .rml.rs 文件中同名 struct 的元信息会被后者覆盖（不应出现）。
    fn scan_struct_metas(
        &self,
        rml_files: &[PathBuf],
    ) -> std::collections::HashMap<String, scanner::StructMetadata> {
        let mut all_metas = std::collections::HashMap::new();
        for rml_path in rml_files {
            let rml_rs: PathBuf = format!("{}.rs", rml_path.display()).into();
            if !rml_rs.exists() {
                continue;
            }
            println!("cargo:rerun-if-changed={}", rml_rs.display());
            let metas = scanner::scan_struct_metadata(&rml_rs);
            all_metas.extend(metas);
        }
        all_metas
    }
}

/// 从源码行中提取 `fn <name>` 的方法名
#[allow(dead_code)]
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
