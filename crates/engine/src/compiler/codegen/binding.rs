//! 双向绑定与字段校验代码生成
//!
//! - `gen_model_input`：`<input value={field}>` → `rml_ui::Input::new(&state)` + 双向同步
//! - `gen_field_*`：VM↔UI 反向赋值代码（含 parse、校验链、bump_version）

use crate::compiler::{CodegenCtx, CodegenError, ValidationRule, ValidationRuleSet};
use crate::compiler::state_bridge::StateBridgeSpec;
use crate::css;
use crate::parser::ast::{Attribute, Element};

use super::attribute::apply_css_styles;

/// 生成带 value 双向绑定的 Input 组件代码
///
/// `<input value={field} placeholder="..." />` 生成：
/// ```text
/// rml_ui::Input::new(&self.__rml_get_or_init_input_state("field", Some("..."), _window, cx))
///     .disabled(false)
/// ```
///
/// 正向绑定（VM→UI）和反向绑定（UI→VM）均由 `__rml_get_or_init_input_state` 内部处理。
/// placeholder 仅支持静态字符串（`InputState::placeholder()` 是 builder 方法，仅创建时可调）。
pub(crate) fn gen_model_input(
    elem: &Element,
    _ctx: &CodegenCtx,
    _id_counter: &mut usize,
    field: String,
    multi_line: bool,
    parents: &[css::ParentInfo],
) -> Result<String, CodegenError> {
    let placeholder = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "placeholder" { Some(value.clone()) } else { None }
        } else { None }
    });
    let placeholder_arg = match placeholder {
        Some(p) => format!("Some({:?})", p),
        None => "None".to_string(),
    };

    let mut input_code = format!(
        "rml_ui::Input::new(&self.__rml_get_or_init_input_state({:?}, {}, {}, _window, cx))",
        field, placeholder_arg, multi_line
    );

    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "disabled" {
                let disabled_val = if value.eq_ignore_ascii_case("true") || value == "1" || value.is_empty() {
                    "true"
                } else {
                    "false"
                };
                input_code.push_str(&format!(".disabled({})", disabled_val));
            }
        }
    }

    // 应用 CSS 样式（class / 父链选择器）。model input 提前返回，因此需要在这里
    // 手动应用全局样式表中匹配的样式，否则 `.demo-section input` 等规则会失效。
    if let Some(sheet) = &_ctx.stylesheet {
        let style_code = apply_css_styles(elem, &elem.tag, sheet, parents);
        input_code.push_str(&style_code);
    }

    let wrapper_id = format!("rml_input_err:{}", field);
    let self_prefix = crate::compiler::expr::current_self_alias().unwrap_or("self");
    let code = format!(
        r#"{{
            let __rml_input = {input_code};
            let __rml_err: Option<gpui::SharedString> = {self_prefix}.__rml_state.field_errors.get({field:?}).and_then(|e| e.clone());
            if let Some(__rml_err_msg) = __rml_err {{
                let __rml_input = __rml_input.border_color(gpui::rgb(0xff0000));
                gpui::div()
                    .id({wrapper_id:?})
                    .child(__rml_input)
                    .tooltip(move |window, cx| rml_ui::Tooltip::new(__rml_err_msg.clone()).build(window, cx))
                    .into_any_element()
            }} else {{
                __rml_input.into_any_element()
            }}
        }}"#,
        input_code = input_code,
        self_prefix = self_prefix,
        field = field,
        wrapper_id = wrapper_id
    );

    Ok(code)
}

/// 生成带 value 双向绑定的 StateBridge 组件代码（C4：通用 StateBridge 机制）
///
/// `<Slider value={field} />` 生成：
/// ```text
/// rml_ui::Slider::new(&self.__rml_get_or_init_slider_state("field", _window, cx))
///     .disabled(false)
/// ```
///
/// 正向绑定（VM→State）和反向绑定（State→VM）均由
/// `__rml_get_or_init_<suffix>_state` 内部处理。
/// 组件构造路径与方法后缀由 `spec` 提供，支持任意 StateBridge 组件。
pub(crate) fn gen_model_state_bridge(
    spec: &StateBridgeSpec,
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    field: String,
    parents: &[css::ParentInfo],
) -> Result<String, CodegenError> {
    let method_name = format!("__rml_get_or_init_{}_state", spec.state_method_suffix);
    let mut code = format!(
        "{}::new(&self.{}({:?}, _window, cx))",
        spec.ctor_path, method_name, field,
    );

    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "disabled" {
                let disabled_val = if value.eq_ignore_ascii_case("true") || value == "1" || value.is_empty() {
                    "true"
                } else {
                    "false"
                };
                code.push_str(&format!(".disabled({})", disabled_val));
            }
        }
    }

    // 应用 CSS 样式（class / 父链选择器）
    if let Some(sheet) = &ctx.stylesheet {
        let style_code = apply_css_styles(elem, &elem.tag, sheet, parents);
        code.push_str(&style_code);
    }

    Ok(code)
}

/// 生成正向值表达式：`self.field` → `gpui::SharedString`
///
/// - 有 converter 时：`Converter.convert(&self.field).into()`（`convert` 返回 `Target`，
///   通常为 `String`，`SharedString: From<String>` 生效）
/// - 无 converter：数字类型 `to_string().into()`；String/SharedString 等 `.clone().into()`
pub(super) fn gen_field_value_expr(field: &str, ty: &str, converter: Option<&str>) -> String {
    if let Some(conv) = converter {
        return format!("{}.convert(&self.{}).into()", conv, field);
    }
    match ty {
        "bool" => format!("self.{}.to_string().into()", field),
        "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "usize" | "isize" => {
            format!("self.{}.to_string().into()", field)
        }
        _ => format!("self.{}.clone().into()", field),
    }
}

/// 生成反向赋值代码块：`value: SharedString` → `this.field = ...`
///
/// 返回完整代码块（含 parse + 赋值 + 错误处理 + bump_version），调用方不再追加 bump_version。
///
/// 当 `converter` 为 `Some` 时，使用 `ConverterName.convert_back(&value.to_string())` 替代裸 `parse`。
pub(super) fn gen_field_assign_expr(
    field: &str,
    ty: &str,
    validation: Option<&ValidationRuleSet>,
    converter: Option<&str>,
) -> String {
    if let Some(conv) = converter {
        return gen_field_assign_with_converter(field, conv);
    }
    if let Some(v) = validation {
        if let Some(validator_type) = &v.validator_type {
            return gen_field_assign_with_validator(field, ty, validator_type);
        }
    }
    let rules = match validation {
        Some(v) if !v.rules.is_empty() => &v.rules,
        _ => return gen_field_assign_expr_default(field, ty),
    };
    let custom_msg = validation.and_then(|v| v.custom_message.as_deref());

    match ty {
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" | "f32" | "f64" => {
            gen_numeric_field_assign_with_validation(field, ty, rules, custom_msg)
        }
        "bool" => gen_field_assign_expr_default(field, ty),
        _ => gen_string_field_assign_with_validation(field, rules, custom_msg),
    }
}

/// Converter 反向转换生成（Phase B-2：`value={field | Converter}`）
///
/// 生成 `ConverterName.convert_back(&value.to_string())` 调用：
/// ```text
/// match ConverterName.convert_back(&value.to_string()) {
///     Some(v) => {
///         this.field = v;
///         this.__rml_state.field_errors.insert("field".to_string(), None);
///         this.__rml_bump_version("field");
///     }
///     None => {
///         this.__rml_state.field_errors.insert("field".to_string(), Some("转换失败".into()));
///     }
/// }
/// ```
fn gen_field_assign_with_converter(field: &str, converter: &str) -> String {
    format!(
        r#"match {converter}.convert_back(&value.to_string()) {{
    Some(v) => {{
        this.{field} = v;
        this.__rml_state.field_errors.insert({field:?}.to_string(), None);
        this.__rml_bump_version({field:?});
    }}
    None => {{
        this.__rml_state.field_errors.insert({field:?}.to_string(), Some("转换失败".into()));
    }}
}}"#,
        converter = converter,
        field = field,
    )
}

/// IValidate 接口式校验生成（Phase B-3.3）
fn gen_field_assign_with_validator(field: &str, ty: &str, validator_type: &str) -> String {
    match ty {
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" | "f32" | "f64" => {
            let is_integer = matches!(ty, "i32" | "u32" | "i64" | "u64" | "isize" | "usize");
            let type_err = if is_integer { "请输入有效的整数" } else { "请输入有效的数字" };
            format!(
                r#"match value.parse::<{ty}>() {{
    Ok(v) => {{
        let __rml_validator = {validator_type}::default();
        let __rml_result = __rml_validator.valid_with_view(value.as_ref(), this as &dyn std::any::Any);
        if let Some(__rml_err_msg) = __rml_validator.message(&__rml_result) {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), Some(__rml_err_msg));
        }} else {{
            this.{field} = v;
            this.__rml_state.field_errors.insert({field:?}.to_string(), None);
            this.__rml_bump_version({field:?});
        }}
    }}
    Err(_) => {{
        this.__rml_state.field_errors.insert({field:?}.to_string(), Some({type_err:?}.into()));
    }}
}}"#,
                ty = ty,
                validator_type = validator_type,
                field = field,
                type_err = type_err,
            )
        }
        "bool" => gen_field_assign_expr_default(field, ty),
        _ => format!(
            r#"{{
    let __rml_value = value.to_string();
    let __rml_validator = {validator_type}::default();
    let __rml_result = __rml_validator.valid_with_view(&__rml_value, this as &dyn std::any::Any);
    if let Some(__rml_err_msg) = __rml_validator.message(&__rml_result) {{
        this.__rml_state.field_errors.insert({field:?}.to_string(), Some(__rml_err_msg));
    }} else {{
        this.{field} = __rml_value;
        this.__rml_state.field_errors.insert({field:?}.to_string(), None);
        this.__rml_bump_version({field:?});
    }}
}}"#,
            validator_type = validator_type,
            field = field,
        ),
    }
}

/// 无校验规则时的默认反向赋值逻辑
fn gen_field_assign_expr_default(field: &str, ty: &str) -> String {
    match ty {
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" => format!(
            r#"match value.parse::<{ty}>() {{
                Ok(v) => {{
                    this.{field} = v;
                    this.__rml_state.field_errors.insert({field:?}.to_string(), None);
                    this.__rml_bump_version({field:?});
                }}
                Err(_) => {{
                    this.__rml_state.field_errors.insert({field:?}.to_string(), Some("请输入有效的整数".into()));
                }}
            }}"#,
            field = field,
            ty = ty
        ),
        "f32" | "f64" => format!(
            r#"match value.parse::<{ty}>() {{
                Ok(v) => {{
                    this.{field} = v;
                    this.__rml_state.field_errors.insert({field:?}.to_string(), None);
                    this.__rml_bump_version({field:?});
                }}
                Err(_) => {{
                    this.__rml_state.field_errors.insert({field:?}.to_string(), Some("请输入有效的数字".into()));
                }}
            }}"#,
            field = field,
            ty = ty
        ),
        "bool" => format!(
            r#"this.{field} = !value.is_empty();
            this.__rml_state.field_errors.insert({field:?}.to_string(), None);
            this.__rml_bump_version({field:?});"#,
            field = field
        ),
        _ => format!(
            r#"this.{field} = value.to_string();
            this.__rml_state.field_errors.insert({field:?}.to_string(), None);
            this.__rml_bump_version({field:?});"#,
            field = field
        ),
    }
}

/// 数字类型字段 + 校验规则：生成 parse + range/custom 校验链
fn gen_numeric_field_assign_with_validation(
    field: &str,
    ty: &str,
    rules: &[ValidationRule],
    custom_msg: Option<&str>,
) -> String {
    let is_integer = matches!(ty, "i32" | "u32" | "i64" | "u64" | "isize" | "usize");
    let type_err = if is_integer { "请输入有效的整数" } else { "请输入有效的数字" };

    let mut branches = String::new();
    for rule in rules {
        match rule {
            ValidationRule::Range { min, max } => {
                let condition = match (min, max) {
                    (Some(min), Some(max)) => format!("!({min}..={max}).contains(&v)"),
                    (Some(min), None) => format!("v < {min}"),
                    (None, Some(max)) => format!("v > {max}"),
                    (None, None) => continue,
                };
                let msg = custom_msg
                    .unwrap_or("值不在有效范围内")
                    .replace("{min}", &min.map(|v| v.to_string()).unwrap_or_default())
                    .replace("{max}", &max.map(|v| v.to_string()).unwrap_or_default());
                let final_msg = if custom_msg.is_some() {
                    msg
                } else {
                    match (min, max) {
                        (Some(min), Some(max)) => format!("值必须在 {}-{} 之间", *min as i64, *max as i64),
                        (Some(min), None) => format!("值必须 >= {}", *min as i64),
                        (None, Some(max)) => format!("值必须 <= {}", *max as i64),
                        (None, None) => "值不在有效范围内".to_string(),
                    }
                };
                branches.push_str(&format!(
                    r#"        if {condition} {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), Some({msg:?}.into()));
        }} else "#,
                    condition = condition,
                    field = field,
                    msg = final_msg
                ));
            }
            ValidationRule::Custom(fn_name) => {
                branches.push_str(&format!(
                    r#"if let Some(__rml_err) = Self::{fn_name}(value.as_ref()) {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), __rml_err);
        }} else "#,
                    fn_name = fn_name,
                    field = field
                ));
            }
            _ => continue,
        }
    }

    branches.push_str(&format!(
        r#"{{
            this.{field} = v;
            this.__rml_state.field_errors.insert({field:?}.to_string(), None);
            this.__rml_bump_version({field:?});
        }}"#,
        field = field
    ));

    format!(
        r#"match value.parse::<{ty}>() {{
            Ok(v) => {{
        {branches}
            }}
            Err(_) => {{
                this.__rml_state.field_errors.insert({field:?}.to_string(), Some({type_err:?}.into()));
            }}
        }}"#,
        ty = ty,
        branches = branches,
        field = field,
        type_err = type_err
    )
}

/// String 类型字段 + 校验规则：生成 required/length/regex/custom 校验链
fn gen_string_field_assign_with_validation(
    field: &str,
    rules: &[ValidationRule],
    custom_msg: Option<&str>,
) -> String {
    let mut branches = String::new();

    for rule in rules {
        match rule {
            ValidationRule::Required => {
                let msg = custom_msg.unwrap_or("此项为必填");
                branches.push_str(&format!(
                    r#"        if __rml_value.is_empty() {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), Some({msg:?}.into()));
        }} else "#,
                    field = field,
                    msg = msg
                ));
            }
            ValidationRule::Length { min, max } => {
                let condition = match (min, max) {
                    (Some(min), Some(max)) => format!("__rml_value.len() < {min} || __rml_value.len() > {max}"),
                    (Some(min), None) => format!("__rml_value.len() < {min}"),
                    (None, Some(max)) => format!("__rml_value.len() > {max}"),
                    (None, None) => continue,
                };
                let final_msg = if custom_msg.is_some() {
                    custom_msg.unwrap_or("长度不合法").to_string()
                } else {
                    match (min, max) {
                        (Some(min), Some(max)) => format!("长度必须在 {}-{} 之间", min, max),
                        (Some(min), None) => format!("长度必须 >= {}", min),
                        (None, Some(max)) => format!("长度必须 <= {}", max),
                        (None, None) => "长度不合法".to_string(),
                    }
                };
                branches.push_str(&format!(
                    r#"if {condition} {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), Some({msg:?}.into()));
        }} else "#,
                    condition = condition,
                    field = field,
                    msg = final_msg
                ));
            }
            ValidationRule::Regex(pattern) => {
                let msg = custom_msg.unwrap_or("格式不正确");
                branches.push_str(&format!(
                    r#"if !rml::regex::Regex::new({pattern:?}).unwrap().is_match(&__rml_value) {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), Some({msg:?}.into()));
        }} else "#,
                    pattern = pattern,
                    field = field,
                    msg = msg
                ));
            }
            ValidationRule::Custom(fn_name) => {
                branches.push_str(&format!(
                    r#"if let Some(__rml_err) = Self::{fn_name}(&__rml_value) {{
            this.__rml_state.field_errors.insert({field:?}.to_string(), __rml_err);
        }} else "#,
                    fn_name = fn_name,
                    field = field
                ));
            }
            _ => continue,
        }
    }

    branches.push_str(&format!(
        r#"{{
            this.{field} = __rml_value;
            this.__rml_state.field_errors.insert({field:?}.to_string(), None);
            this.__rml_bump_version({field:?});
        }}"#,
        field = field
    ));

    format!(
        r#"{{
            let __rml_value = value.to_string();
        {branches}
        }}"#,
        branches = branches
    )
}
