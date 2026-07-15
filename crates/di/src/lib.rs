//! RML DI 适配层 —— 基于 rust-dix 的薄包装
//!
//! 对标 ASP.NET Core DI：`RmlApplication::new().configure(|s| { ... }).run::<L>()`
//! - `ServiceCollection`：注册容器（内部委托 `rust_dix::ServiceCollection`）
//! - `ServiceProvider`：解析容器（impl `IServiceProvider`，委托 rust-dix）
//! - `ServiceProviderWrapper`：子主容器桥接（child-first 分层解析）
//! - `RmlApplicationExt::configure`：链式 API
//! - `auto_register`：`#[ctor::ctor]` 全局注册表（向后兼容）
//!
//! ## 自动注册（推荐）
//!
//! 使用 `#[inject]` 宏标记服务，自动注册到容器：
//! ```rust,ignore
//! use rust_rml_di::inject;
//!
//! #[inject]
//! struct MyService;
//! ```
//! `ServiceCollection::new()` 自动收集所有 `#[inject]` 标记的服务。
//!
//! ## 手动注册（向后兼容）
//!
//! ```rust,ignore
//! s.add_singleton::<dyn IFoo>(|_| Arc::new(FooImpl));
//! ```
//!
//! 零污染：不强制暴露 rust-dix 细节。core/app 不依赖本 crate。
//! studio 层可通过 re-export 直接使用 rust-dix 原生 API。

extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;

mod auto_register;
mod collection;
mod configure;
mod provider;
mod wrapper;

pub mod prelude;

pub use auto_register::{apply_auto_registrations, auto_register};
pub use collection::ServiceCollection;
pub use configure::RmlApplicationExt;
pub use provider::ServiceProvider;
pub use wrapper::ServiceProviderWrapper;

// ── rust-dix re-exports（供 studio 层直接使用原生 API）──
pub use rust_dix::{
    inject, module, register, Inject, IServiceResolver, IProvider, ServiceLifetime,
    ServiceRegistration, ServiceProvider as RdiServiceProvider,
};
