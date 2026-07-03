//! `impl ILifecycle` 自动联动代码生成（Phase B-3）
//!
//! 用户在 `.rml.rs` 中标注 `#[on_loaded]` / `#[on_unloaded]` 的方法，
//! 由 scanner 收集方法名，codegen 据此生成 `impl ILifecycle for <View>`，
//! 在 trait 方法中调用用户方法，避免用户手动 impl。
//!
//! ## 冲突处理
//!
//! 若用户已手动 `impl ILifecycle for <Type>` 且同时使用 `#[on_loaded]` 标注：
//! codegen 跳过自动生成并发出 `cargo:warning`，避免重复 impl 导致编译错误。
//! 用户应删除手动 impl 或移除 `#[on_loaded]` 标注。

use crate::compiler::CodegenCtx;

/// 生成 `impl ILifecycle for <View>` 代码块
///
/// 返回空字符串的情况：
/// - 存在手动 `impl ILifecycle` 且同时有标注（发出 warning，跳过生成避免冲突）
///
/// 无钩子且无手动 impl 时生成空 `impl ILifecycle for X {}`：
/// - `IViewModel: ILifecycle` 要求 ViewModel 必须实现 ILifecycle
/// - 空 impl 满足 trait 约束，避免用户手动写 16+ 处样板代码
pub(super) fn gen_lifecycle_impl(ctx: &CodegenCtx) -> String {
    // 冲突检测优先：手动 impl + 标注同时存在
    if ctx.has_manual_lifecycle_impl {
        if ctx.lifecycle_hooks.has_any() {
            println!(
                "cargo:warning=RML: {} 同时存在手动 `impl ILifecycle` 与 `#[on_loaded]`/`#[on_unloaded]` 标注，\
                 codegen 跳过自动生成。请删除手动 impl 或移除标注以避免歧义。",
                ctx.view_struct_name
            );
        }
        return String::new();
    }

    let view_name = &ctx.view_struct_name;

    // 无钩子且无手动 impl：生成空 impl（满足 IViewModel: ILifecycle 约束）
    if !ctx.lifecycle_hooks.has_any() {
        return format!(
            r#"#[allow(dead_code)]
impl rml_core::lifecycle::ILifecycle for {view_name} {{}}
"#,
            view_name = view_name,
        );
    }

    let on_loaded = ctx.lifecycle_hooks.on_loaded.as_deref();
    let on_unloaded = ctx.lifecycle_hooks.on_unloaded.as_deref();

    // 仅生成存在钩子的 trait 方法，避免生成空方法覆盖 trait 默认实现
    let mut methods = String::new();

    if let Some(method) = on_loaded {
        methods.push_str(&format!(
            "    fn on_loaded(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) where Self: Sized {{\n"
        ));
        methods.push_str(&format!("        self.{}(window, cx);\n", method));
        methods.push_str("    }\n");
    }

    if let Some(method) = on_unloaded {
        methods.push_str(&format!(
            "    fn on_unloaded(&mut self, cx: &mut gpui::Context<Self>) where Self: Sized {{\n"
        ));
        methods.push_str(&format!("        self.{}(cx);\n", method));
        methods.push_str("    }\n");
    }

    format!(
        r#"#[allow(dead_code, non_snake_case)]
impl rml_core::lifecycle::ILifecycle for {view_name} {{
{methods}}}
"#,
        view_name = view_name,
        methods = methods,
    )
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::scanner::LifecycleHooks;

    fn make_ctx(hooks: LifecycleHooks, has_manual: bool) -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".to_string(),
            lifecycle_hooks: hooks,
            has_manual_lifecycle_impl: has_manual,
            ..CodegenCtx::default()
        }
    }

    #[test]
    fn no_hooks_generates_empty_impl() {
        let ctx = make_ctx(LifecycleHooks::default(), false);
        let code = gen_lifecycle_impl(&ctx);
        assert!(
            code.contains("impl rml_core::lifecycle::ILifecycle for TestView"),
            "no-hooks should generate empty ILifecycle impl\n{}",
            code
        );
        assert!(
            !code.contains("fn on_loaded") && !code.contains("fn on_unloaded"),
            "no-hooks impl should not contain trait methods\n{}",
            code
        );
    }

    #[test]
    fn manual_impl_with_hooks_skips_and_warns() {
        let hooks = LifecycleHooks {
            on_loaded: Some("my_loaded".to_string()),
            on_unloaded: None,
        };
        let ctx = make_ctx(hooks, true);
        // 冲突时返回空字符串（避免重复 impl）
        assert_eq!(gen_lifecycle_impl(&ctx), "");
    }

    #[test]
    fn manual_impl_without_hooks_returns_empty() {
        let ctx = make_ctx(LifecycleHooks::default(), true);
        assert_eq!(gen_lifecycle_impl(&ctx), "");
    }

    #[test]
    fn on_loaded_only_generates_partial_impl() {
        let hooks = LifecycleHooks {
            on_loaded: Some("do_load".to_string()),
            on_unloaded: None,
        };
        let ctx = make_ctx(hooks, false);
        let code = gen_lifecycle_impl(&ctx);

        assert!(code.contains("impl rml_core::lifecycle::ILifecycle for TestView"));
        assert!(code.contains("fn on_loaded"));
        assert!(code.contains("self.do_load(window, cx);"));
        // 无 on_unloaded 方法（保留 trait 默认实现）
        assert!(!code.contains("fn on_unloaded"));
    }

    #[test]
    fn on_unloaded_only_generates_partial_impl() {
        let hooks = LifecycleHooks {
            on_loaded: None,
            on_unloaded: Some("do_unload".to_string()),
        };
        let ctx = make_ctx(hooks, false);
        let code = gen_lifecycle_impl(&ctx);

        assert!(code.contains("fn on_unloaded"));
        assert!(code.contains("self.do_unload(cx);"));
        assert!(!code.contains("fn on_loaded"));
    }

    #[test]
    fn both_hooks_generate_full_impl() {
        let hooks = LifecycleHooks {
            on_loaded: Some("load".to_string()),
            on_unloaded: Some("unload".to_string()),
        };
        let ctx = make_ctx(hooks, false);
        let code = gen_lifecycle_impl(&ctx);

        assert!(code.contains("fn on_loaded"));
        assert!(code.contains("self.load(window, cx);"));
        assert!(code.contains("fn on_unloaded"));
        assert!(code.contains("self.unload(cx);"));
    }
}
