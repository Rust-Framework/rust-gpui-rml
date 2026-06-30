//! RML 解析引擎与编译器
//!
//! 将 `.rml` 模板编译为原生 GPUI 渲染代码。
//! 包含：词法分析、AST 构建、语义验证、代码生成、构建集成。

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
// pub extern crate 让别名对下游可见（rml::rml_core::... 可用）
pub extern crate rust_rml_core as rml_core;
pub extern crate rust_rml_macros as rml_macros;

// Phase B-3.2：re-export regex crate，供 codegen 生成的校验代码使用（rml::regex::Regex::new(...)）
pub use regex;

pub mod build;
pub mod compiler;
pub mod css;
pub mod parser;
pub mod runtime;
pub mod tags;

pub mod prelude;

/// 重导出 core 的资源模块,供用户 crate 通过 `rml::assets::load` 调用
pub use rml_core::assets;

/// 重导出 core 的 i18n 模块,供 codegen 生成代码通过 `rml::i18n` 访问
pub use rml_core::i18n;

/// 重导出 core 的 theme 模块,供 codegen 生成代码通过 `rml::theme::color` 访问
pub use rml_core::theme;

/// 构建入口：在用户 `build.rs` 中调用，扫描 `.rml`、调用编译器、输出到 `OUT_DIR`。
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
pub use build::build;

/// 嵌入资源注册表宏
///
/// 在用户 crate 根(通常是 `main.rs` 或 `lib.rs`)调用,注入 `RML_ASSETS` 常量。
/// 配合 `rml::assets::init(RML_ASSETS)` 在启动时注册到运行时查询表。
///
/// ```rust,ignore
/// // main.rs
/// rml::embed_assets!();
///
/// fn main() {
///     rml::assets::init(RML_ASSETS);
///     // ... 启动应用
/// }
/// ```
#[macro_export]
macro_rules! embed_assets {
    () => {
        include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"));
    };
}

/// 一键启动宏:内部完成资源嵌入、资源注册、应用启动。
///
/// 在用户 crate 根调用,替代手写 `embed_assets!()` + `fn main()`。
/// 宏内部生成 `fn main()`,调用者无需再写。
///
/// ```rust,ignore
/// // main.rs
/// extern crate rust_rml_engine as rml;
/// extern crate rust_rml_app as rml_app;
///
/// mod app;
///
/// rml::main!(app::AppBootstrap);
/// ```
#[macro_export]
macro_rules! main {
    ($app:path) => {
        $crate::embed_assets!();

        fn main() {
            $crate::assets::init(RML_ASSETS);
            ::rml_app::RmlApplication::new().run::<$app>();
        }
    };
}

/// 当前 engine crate 源码的 sha256 哈希（编译期嵌入）。
///
/// 当 engine 任何 `src/**/*.rs` 文件变化时，engine 的 build.rs 会重算哈希并写入 OUT_DIR，
/// 导致本常量更新。下游 build.rs 通过比较此哈希与缓存中的 `engine_hash` 字段判断是否需要
/// 失效所有 `.rml` 缓存条目（避免使用过期 codegen 输出）。
pub fn engine_source_hash() -> &'static str {
    include_str!(concat!(env!("OUT_DIR"), "/rml_engine_hash.txt"))
}
