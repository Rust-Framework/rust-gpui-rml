//! `ICommand` trait —— 命令系统契约
//!
//! `#[command]` 标记的方法可被 `.rml` 中的 `on*` 事件绑定调用。
//! 命令方法签名：`fn(&mut self, ev: &Event, cx: &mut Context<Self>)`
//! 或带参数：`fn(&mut self, param: T, ev: &Event, cx: &mut Context<Self>)`

/// 命令参数元信息
///
/// 由 `#[command]` 宏在编译期生成，供绑定引擎校验参数类型与顺序。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamMeta {
    /// 参数名（来自方法签名）
    pub name: &'static str,
    /// 参数类型名（如 "i32"、"SharedString"）
    pub ty: &'static str,
}

/// 命令基础 trait。
///
/// 命令是 ViewModel 中唯一允许修改视图状态的方法。
/// 命令执行后必须调用 `cx.notify()` 触发重渲染。
///
/// `#[command]` 宏在 Phase A 为 pass-through（不强制实现此 trait），
/// Phase B 会自动生成 `ICommand` 实现并填充元信息。
///
/// 注：由于 `#[command]` 作用于方法而非结构体，宏无法获取结构体名，
/// 因此 `impl ICommand` 的自动生成需要 `#[view]` 宏配合或在 build.rs 中扫描。
/// 当前 Phase B-2 采用 pass-through + 编译期元信息提取策略。
pub trait ICommand {
    /// 命令名称（方法名），供绑定引擎校验
    fn rml_command_name() -> &'static str;

    /// 事件对象类型名（编译期生成，如 "ClickEvent"）
    fn rml_event_type() -> &'static str {
        ""
    }

    /// 参数描述（编译期生成，由 `#[command]` 宏填充）
    fn rml_params() -> &'static [ParamMeta] {
        &[]
    }

    /// 命令是否可执行（用于禁用按钮等）。
    ///
    /// codegen 在 `disabled={expr}` 绑定时调用此方法，
    /// 返回 false 时按钮渲染为 disabled 状态。
    fn can_execute(&self) -> bool {
        true
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // 测试用的 mock 命令实现
    struct AlwaysEnabled;
    struct AlwaysDisabled;

    impl ICommand for AlwaysEnabled {
        fn rml_command_name() -> &'static str {
            "increment"
        }
    }

    impl ICommand for AlwaysDisabled {
        fn rml_command_name() -> &'static str {
            "decrement"
        }

        fn rml_event_type() -> &'static str {
            "ClickEvent"
        }

        fn rml_params() -> &'static [ParamMeta] {
            &[
                ParamMeta { name: "amount", ty: "i32" },
            ]
        }

        fn can_execute(&self) -> bool {
            false
        }
    }

    // ─── ParamMeta ───

    #[test]
    fn param_meta_construction() {
        let p = ParamMeta { name: "count", ty: "i32" };
        assert_eq!(p.name, "count");
        assert_eq!(p.ty, "i32");
    }

    #[test]
    fn param_meta_clone() {
        let p1 = ParamMeta { name: "value", ty: "SharedString" };
        let p2 = p1.clone();
        assert_eq!(p1.name, p2.name);
        assert_eq!(p1.ty, p2.ty);
    }

    #[test]
    fn param_meta_debug_format() {
        let p = ParamMeta { name: "x", ty: "i32" };
        let debug_str = format!("{:?}", p);
        assert!(debug_str.contains("ParamMeta"));
        assert!(debug_str.contains("x"));
        assert!(debug_str.contains("i32"));
    }

    // ─── ICommand 默认实现 ───

    #[test]
    fn default_event_type_is_empty() {
        assert_eq!(AlwaysEnabled::rml_event_type(), "");
    }

    #[test]
    fn default_params_is_empty() {
        assert_eq!(AlwaysEnabled::rml_params(), &[] as &[ParamMeta]);
    }

    #[test]
    fn default_can_execute_is_true() {
        let cmd = AlwaysEnabled;
        assert!(cmd.can_execute());
    }

    // ─── ICommand 自定义实现 ───

    #[test]
    fn custom_command_name() {
        assert_eq!(AlwaysEnabled::rml_command_name(), "increment");
        assert_eq!(AlwaysDisabled::rml_command_name(), "decrement");
    }

    #[test]
    fn custom_event_type() {
        assert_eq!(AlwaysDisabled::rml_event_type(), "ClickEvent");
    }

    #[test]
    fn custom_params() {
        let params = AlwaysDisabled::rml_params();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "amount");
        assert_eq!(params[0].ty, "i32");
    }

    #[test]
    fn custom_can_execute_false() {
        let cmd = AlwaysDisabled;
        assert!(!cmd.can_execute());
    }

    // ─── ParamMeta 集合使用场景 ───

    #[test]
    fn param_meta_slice_operations() {
        let params: &[ParamMeta] = &[
            ParamMeta { name: "a", ty: "i32" },
            ParamMeta { name: "b", ty: "String" },
            ParamMeta { name: "c", ty: "bool" },
        ];
        assert_eq!(params.len(), 3);
        assert_eq!(params[0].name, "a");
        assert_eq!(params[2].ty, "bool");

        // 验证可以迭代
        let names: Vec<&str> = params.iter().map(|p| p.name).collect();
        assert_eq!(names, vec!["a", "b", "c"]);
    }
}
