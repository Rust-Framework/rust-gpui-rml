//! `#[rml::main]` 属性宏
//!
//! 在用户 `fn main` 之前注入 `rml::embed_assets!()` 宏调用,
//! 等价于 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))`。
//!
//! 生成文件内含 `#[ctor::ctor]` 自动注册函数,在 `main` 之前完成
//! `rml_core::assets::init(...)` 调用,因此 main.rs 无需手写资源相关代码。
//!
//! ```rust,ignore
//! // main.rs
//! #[rml::main]
//! fn main() {
//!     rml_app::RmlApplication::new()
//!         .main_window::<MainWindow>()
//!         .run::<Startup>();
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

pub fn expand(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(item as ItemFn);

    let expanded = quote! {
        // 在 fn main 之前注入资源嵌入：
        // - 嵌入模式：include_bytes! + #[ctor::ctor] 自动 init
        // - 文件系统模式：仅 #[ctor::ctor] 自动 init（路径根）
        rml::embed_assets!();

        #item_fn
    };

    expanded.into()
}
