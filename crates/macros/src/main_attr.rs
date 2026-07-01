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
        // 在 fn main 之前注入 build.rs 生成代码：
        // - embed_assets!：资源 #[ctor::ctor] 自动 init
        // - embed_contributions!：register_rml_contributions(cx) 供 on_launch 调用
        rml::embed_assets!();
        rml::embed_contributions!();

        #item_fn
    };

    expanded.into()
}
