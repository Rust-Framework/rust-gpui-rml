//! `ICommand` trait —— 命令系统契约
//!
//! `#[command]` 标记的方法可被 `.rml` 中的 `on*` 事件绑定调用。
//! 命令方法签名：`fn(&mut self, ev: &Event, cx: &mut Context<Self>)`
//! 或带参数：`fn(&mut self, param: T, ev: &Event, cx: &mut Context<Self>)`

/// 命令参数元信息
///
/// 由 `#[command]` 宏在编译期生成，供绑定引擎校验参数类型与顺序。
#[derive(Debug, Clone)]
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
}
