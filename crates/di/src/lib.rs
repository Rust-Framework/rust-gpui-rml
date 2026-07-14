//! RML DI 适配层 —— ServiceCollection + ServiceProvider + configure 模式
//!
//! 对标 ASP.NET Core DI：`RmlApplication::new().configure(|s| { ... }).run::<L>()`
//! - `ServiceCollection`：注册容器（自维护 factory map）
//! - `ServiceProvider`：解析容器（impl `IServiceProvider`，自维护 cache）
//! - `RmlApplicationExt::configure`：链式 API，接收闭包直接操作 ServiceCollection，
//!   内部创建 collection → 自动注册 → 闭包配置 → build provider → `set` 进 RmlApplication
//!
//! 零污染：不暴露 rust-dix 细节，无 "dix" 字眼。core/app 不依赖本 crate。

extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;

mod auto_register;
mod collection;
mod configure;
mod provider;

pub mod prelude;

pub use auto_register::{apply_auto_registrations, auto_register};
pub use collection::ServiceCollection;
pub use configure::RmlApplicationExt;
pub use provider::ServiceProvider;
