//! 双向绑定注册表 —— (组件, 属性) → 反向同步方式
//!
//! 自动推断原则：属性具备双向能力则自动双向，开发者无需声明 mode=twoway。
//! - Checkbox/Switch/Radio + `checked={field}` → 自动双向（on_click &bool 回写）
//! - Rating + `value={field}` → 自动双向（on_click &usize 回写）
//! - RadioGroup/Stepper + `selected_index={field}` → 自动双向（on_click &usize 回写）
//!
//! ## 工作机制
//!
//! Stateless 表单组件无 State Entity，值通过 `.checked()` / `.value()` / `.selected_index()`
//! 传入（正向同步）。反向同步通过在 `on_click` 事件回调中直接回写 ViewModel 字段实现。
//!
//! 当用户同时声明 `on-click={handler}` 时，框架生成合并回调：
//! 1. 自动回写：`this.field = *payload; this.__rml_bump_version("field");`
//! 2. 用户回调：`this.handler(payload, cx);`
//! 3. `cx.notify();`

use crate::parser::ast::EventHandler;
use crate::tags;

/// 事件载荷类型
#[derive(Clone, Copy)]
pub enum PayloadType {
    /// Checkbox/Switch/Radio: on_click(&bool)
    Bool,
    /// Rating/RadioGroup/Stepper: on_click(&usize)
    Usize,
}

impl PayloadType {
    fn rust_type(self) -> &'static str {
        match self {
            PayloadType::Bool => "bool",
            PayloadType::Usize => "usize",
        }
    }
}

/// 双向绑定规格
pub struct TwoWayBindingSpec {
    /// 绑定属性名（"checked" / "value" / "selected_index"）
    pub bind_property: &'static str,
    /// 事件回调中的载荷变量名（与组件 event_setter 保持一致）
    pub payload_var: &'static str,
    /// 载荷类型
    pub payload_type: PayloadType,
}

/// 双向绑定注册表 —— 单一信源
static TWOWAY_BINDING_REGISTRY: &[(&str, TwoWayBindingSpec)] = &[
    ("Checkbox", TwoWayBindingSpec { bind_property: "checked", payload_var: "checked", payload_type: PayloadType::Bool }),
    ("Switch", TwoWayBindingSpec { bind_property: "checked", payload_var: "checked", payload_type: PayloadType::Bool }),
    ("Radio", TwoWayBindingSpec { bind_property: "checked", payload_var: "checked", payload_type: PayloadType::Bool }),
    ("Rating", TwoWayBindingSpec { bind_property: "value", payload_var: "value", payload_type: PayloadType::Usize }),
    ("RadioGroup", TwoWayBindingSpec { bind_property: "selected_index", payload_var: "idx", payload_type: PayloadType::Usize }),
    ("Stepper", TwoWayBindingSpec { bind_property: "selected_index", payload_var: "idx", payload_type: PayloadType::Usize }),
];

/// 查询组件的指定属性是否支持双向绑定
pub fn lookup_twoway_binding(tag: &str, bind_property: &str) -> Option<&'static TwoWayBindingSpec> {
    let canonical = tags::canonical_tag(tag);
    TWOWAY_BINDING_REGISTRY
        .iter()
        .find(|(t, spec)| *t == canonical.as_str() && spec.bind_property == bind_property)
        .map(|(_, spec)| spec)
}

/// 从 bind 表达式中提取 field 名（复用 codegen::extract_field_converter）
pub fn extract_bind_field(expr: &str) -> String {
    crate::compiler::codegen::extract_field_converter(expr).0
}

/// 检测元素是否存在双向绑定，返回 (field, spec, user_on_click_handler)
pub fn detect_twoway_binding<'a>(
    elem: &'a crate::parser::ast::Element,
    tag: &str,
) -> Option<(String, &'static TwoWayBindingSpec, Option<&'a EventHandler>)> {
    let canonical = tags::canonical_tag(tag);

    // 查找双向绑定属性
    let twoway_spec = TWOWAY_BINDING_REGISTRY
        .iter()
        .find(|(t, _)| *t == canonical.as_str());

    let twoway_spec = twoway_spec?;

    let (_, spec) = twoway_spec;

    // 查找 bind 属性
    let bind_expr = elem.attributes.iter().find_map(|attr| {
        if let crate::parser::ast::Attribute::Bind { name, expr, .. } = attr {
            if name == spec.bind_property {
                return Some(expr.clone());
            }
        }
        None
    })?;

    let field = extract_bind_field(&bind_expr);

    // 字面量表达式（checked={true}、value={5} 等）不参与双向绑定：
    // 双向绑定需要可变字段引用作为回写目标，字面量无法回写
    if field == "true" || field == "false" || field.parse::<f64>().is_ok() {
        return None;
    }

    // 查找用户 on_click handler（如有）
    let user_on_click = elem.attributes.iter().find_map(|attr| {
        if let crate::parser::ast::Attribute::Event { name, handler, .. } = attr {
            if name == "on_click" {
                return Some(handler);
            }
        }
        None
    });

    Some((field, spec, user_on_click))
}

/// 生成双向绑定的 on_click 回调代码
///
/// 合并自动回写 + 用户回调（如有）。
/// ClosureField handler 不支持合并，返回 None（调用方降级为仅正向绑定）。
pub fn gen_twoway_on_click(
    spec: &TwoWayBindingSpec,
    field: &str,
    user_handler: Option<&EventHandler>,
) -> Option<String> {
    let payload_var = spec.payload_var;
    let payload_ty = spec.payload_type.rust_type();

    let auto_sync = format!(
        "this.{field} = *{pv};\n    this.__rml_bump_version({field:?});",
        field = field,
        pv = payload_var
    );

    let user_call = match user_handler {
        Some(EventHandler::Ident(m)) | Some(EventHandler::MethodName(m)) => {
            format!("\n    this.{}({}, cx);", m, payload_var)
        }
        Some(EventHandler::WithArgs(m, args)) if args.is_empty() => {
            format!("\n    this.{}({}, cx);", m, payload_var)
        }
        Some(EventHandler::WithArgs(m, args)) => {
            let arg = &args[0];
            format!(
                "\n    let p0 = {}.clone();\n    this.{}(p0, {}, cx);",
                arg, m, payload_var
            )
        }
        _ => String::new(),
    };

    if matches!(user_handler, Some(EventHandler::ClosureField(_))) {
        return None;
    }

    Some(format!(
        ".on_click(cx.listener(move |this, {pv}: &{ty}, _window, cx| {{\n    \
         {sync}{user}\n    \
         cx.notify();\n}}))",
        pv = payload_var,
        ty = payload_ty,
        sync = auto_sync,
        user = user_call,
    ))
}
