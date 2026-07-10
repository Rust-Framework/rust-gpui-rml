//! Settings 专用 translator —— 4 层嵌套设置面板
//!
//! Settings 是多层嵌套组件，层级结构：
//! ```text
//! Settings (Settings::new(id))
//!   └── SettingPage (.page(SettingPage::new(title)))
//!         └── SettingGroup (.group(SettingGroup::new()))
//!               └── SettingItem (.item(SettingItem::new(title, field)))
//! ```
//!
//! ## 核心要点
//! - Settings 不实现 Styled，仅 RenderOnce
//! - SettingGroup 实现 Styled，支持 CSS 样式
//! - SettingItem 的 field 通过 `field-type` 属性选择构造器：
//!   - `switch` → SettingField::switch(getter, setter) — bool
//!   - `checkbox` → SettingField::checkbox(getter, setter) — bool
//!   - `input` → SettingField::input(getter, setter) — SharedString
//!   - `dropdown` → SettingField::dropdown(options, getter, setter) — SharedString
//!   - `number-input` → SettingField::number_input(options, getter, setter) — f64
//! - `value={field}` bind → getter 闭包（读取 ViewModel 字段）
//! - `on-change={handler}` event → setter 闭包（调用 ViewModel 方法）
//! - 闭包使用 `cx.weak_entity()` 模式访问 ViewModel
//!
//! ## 生成代码示例
//!
//! ```ignore
//! rml_ui::Settings::new(("rml_el", 0usize))
//!     .sidebar_width(gpui::px(280.))
//!     .page(rml_ui::SettingPage::new("通用")
//!         .icon(rml_ui::Icon::new(rml_ui::IconName::Settings))
//!         .group(rml_ui::SettingGroup::new()
//!             .title("外观")
//!             .item(rml_ui::SettingItem::new("暗色主题", rml_ui::SettingField::switch(
//!                 { let weak = cx.weak_entity(); move |app: &gpui::App| {
//!                     weak.upgrade().map(|e| e.read(app).is_dark).unwrap_or(false) } },
//!                 { let weak = cx.weak_entity(); move |val: bool, app: &mut gpui::App| {
//!                     if let Some(e) = weak.upgrade() {
//!                         e.update(app, |this, cx| { this.on_dark_change(val, cx); }); } } }
//!             )))))
//! ```

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::{component_bind_rust_expr, component_event_setter, component_static_setter};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, EventHandler, Node};
use crate::tags;

// ──────────────────────────────────────────────────────────────────────────
//  SettingsTranslator：<settings> 设置面板
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct SettingsTranslator;

impl IRmlTranslator for SettingsTranslator {
    fn tag(&self) -> &'static str {
        "Settings"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Settings"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let id_val = *id_counter;
        *id_counter += 1;

        // 1. 构造器：Settings::new(id)
        let mut code = format!("rml_ui::Settings::new((\"rml_el\", {}usize))", id_val);

        // 2. 属性（sidebar-width / size / group-variant / default-selected-page）
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    match name.as_str() {
                        "sidebar_width" => {
                            if let Some(px_code) = parse_px_value(value) {
                                code.push_str(&format!(".sidebar_width({})", px_code));
                            }
                        }
                        "group_variant" => {
                            if let Some(variant) = parse_group_variant(value) {
                                code.push_str(&format!(".with_group_variant({})", variant));
                            }
                        }
                        "default_selected_page" => {
                            if let Ok(page_ix) = value.parse::<usize>() {
                                code.push_str(&format!(
                                    ".default_selected_index(rml_ui::SelectIndex {{ page_ix: {}usize, group_ix: None }})",
                                    page_ix
                                ));
                            }
                        }
                        "size" => {
                            if let Some(size_code) = parse_size(value) {
                                code.push_str(&format!(".with_size({})", size_code));
                            }
                        }
                        _ => {
                            if let Some(setter) = component_static_setter(name, value, "Settings") {
                                code.push_str(&setter);
                            }
                        }
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if let Some(setter) =
                        crate::compiler::setters::component_bind_setter(name, expr, &lv, &computed, "Settings")
                    {
                        code.push_str(&setter);
                    }
                }
                Attribute::Event { name, handler, .. } => {
                    if let Some(setter) = component_event_setter(name, handler, "Settings") {
                        code.push_str(&setter);
                    }
                }
            }
        }

        // 3. 子节点 → .page(SettingPage::new(...)...)
        for child in &elem.children {
            match child {
                Node::Element(child_elem)
                    if tags::canonical_tag(&child_elem.tag) == "SettingPage" =>
                {
                    let page_code =
                        gen_setting_page(child_elem, ctx, id_counter, loop_vars)?;
                    code.push_str(&format!("\n            .page({})", page_code));
                }
                Node::Text(text) => {
                    eprintln!(
                        "[rml warning] <Settings> 不支持文本子节点 {:?}，已忽略",
                        text
                    );
                }
                Node::Element(child_elem) => {
                    return Err(CodegenError {
                        message: format!(
                            "<Settings> 仅支持 <SettingPage> 或 <setting-page> 子节点，得到 <{}>",
                            child_elem.tag
                        ),
                        span: Some(elem.span),
                    });
                }
                _ => {}
            }
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Settings", "Settings", ComponentCategory::Container).container(true)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  嵌套层级 codegen
// ──────────────────────────────────────────────────────────────────────────

/// 生成 SettingPage 构造代码
///
/// 生成形如：`rml_ui::SettingPage::new("通用").icon(...).group(SettingGroup::new()...).group(...)`
fn gen_setting_page(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // title 属性（必填）→ SettingPage::new(title)
    let title = extract_static_attr(elem, "title").ok_or_else(|| CodegenError {
        message: format!("<SettingPage> 缺少必需的 title 属性"),
        span: Some(elem.span),
    })?;

    let mut code = format!("rml_ui::SettingPage::new({:?})", title);

    // 属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                match name.as_str() {
                    "title" => { /* 已用于构造器 */ }
                    "icon" => {
                        code.push_str(&format!(
                            ".icon(rml_ui::Icon::new(rml_ui::IconName::{}))",
                            value
                        ));
                    }
                    "description" => {
                        code.push_str(&format!(".description({:?})", value));
                    }
                    "default_open" => {
                        code.push_str(&format!(".default_open({})", parse_bool(value)));
                    }
                    "resettable" => {
                        code.push_str(&format!(".resettable({})", parse_bool(value)));
                    }
                    _ => {
                        if let Some(setter) =
                            component_static_setter(name, value, "SettingPage")
                        {
                            code.push_str(&setter);
                        }
                    }
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(setter) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "SettingPage",
                ) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(setter) = component_event_setter(name, handler, "SettingPage") {
                    code.push_str(&setter);
                }
            }
        }
    }

    // 子节点 → .group(SettingGroup::new()...)
    for child in &elem.children {
        match child {
            Node::Element(child_elem)
                if tags::canonical_tag(&child_elem.tag) == "SettingGroup" =>
            {
                let group_code =
                    gen_setting_group(child_elem, ctx, id_counter, loop_vars, &[])?;
                code.push_str(&format!("\n            .group({})", group_code));
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <SettingPage> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<SettingPage> 仅支持 <SettingGroup> 或 <setting-group> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
                });
            }
            _ => {}
        }
    }

    Ok(code)
}

/// 生成 SettingGroup 构造代码
///
/// 生成形如：`rml_ui::SettingGroup::new().title("外观").item(SettingItem::new(...))...`
fn gen_setting_group(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::SettingGroup::new()");

    // CSS class 样式（SettingGroup 实现 Styled）
    append_css_class_styles(&mut code, elem, "SettingGroup", ctx.stylesheet.as_ref(), parents);

    // 属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => match name.as_str() {
                "title" => {
                    code.push_str(&format!(".title({:?})", value));
                }
                "description" => {
                    code.push_str(&format!(".description({:?})", value));
                }
                _ => {
                    if let Some(setter) =
                        component_static_setter(name, value, "SettingGroup")
                    {
                        code.push_str(&setter);
                    }
                }
            },
            Attribute::Bind { name, expr, .. } => {
                if let Some(setter) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "SettingGroup",
                ) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(setter) = component_event_setter(name, handler, "SettingGroup") {
                    code.push_str(&setter);
                }
            }
        }
    }

    // 子节点 → .item(SettingItem::new(title, field)...)
    for child in &elem.children {
        match child {
            Node::Element(child_elem)
                if tags::canonical_tag(&child_elem.tag) == "SettingItem" =>
            {
                let item_code =
                    gen_setting_item(child_elem, ctx, id_counter, loop_vars)?;
                code.push_str(&format!("\n            .item({})", item_code));
            }
            Node::Text(text) => {
                eprintln!(
                    "[rml warning] <SettingGroup> 不支持文本子节点 {:?}，已忽略",
                    text
                );
            }
            Node::Element(child_elem) => {
                return Err(CodegenError {
                    message: format!(
                        "<SettingGroup> 仅支持 <SettingItem> 或 <setting-item> 子节点，得到 <{}>",
                        child_elem.tag
                    ),
                    span: Some(elem.span),
                });
            }
            _ => {}
        }
    }

    Ok(code)
}

/// 生成 SettingItem 构造代码
///
/// 生成形如：`rml_ui::SettingItem::new("暗色主题", SettingField::switch(getter, setter))`
fn gen_setting_item(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // title 属性（必填）→ SettingItem::new(title, field)
    let title = extract_static_attr(elem, "title").ok_or_else(|| CodegenError {
        message: format!("<SettingItem> 缺少必需的 title 属性"),
        span: Some(elem.span),
    })?;

    // field-type 属性（必填）→ 选择 SettingField 构造器
    let field_type =
        extract_static_attr(elem, "field_type").ok_or_else(|| CodegenError {
            message: format!("<SettingItem> 缺少必需的 field-type 属性（如 field-type=\"switch\"）"),
            span: Some(elem.span),
        })?;

    // value bind 属性 → getter 闭包（读取 ViewModel 字段）
    let value_expr = extract_bind_attr(elem, "value").ok_or_else(|| CodegenError {
        message: format!("<SettingItem> 缺少必需的 value 绑定属性（如 value={{is_dark}}）"),
        span: Some(elem.span),
    })?;

    // on-change event 属性 → setter 闭包（调用 ViewModel 方法）
    let on_change_handler = extract_event_handler(elem, "on_change").ok_or_else(|| CodegenError {
        message: format!("<SettingItem> 缺少必需的 on-change 事件属性（如 on-change={{on_dark_change}}）"),
        span: Some(elem.span),
    })?;

    // 生成 field 构造代码
    let field_code = gen_setting_field(
        &field_type,
        &value_expr,
        &on_change_handler,
        elem,
        &lv,
        &computed,
    )?;

    let mut code = format!(
        "rml_ui::SettingItem::new({:?}, {})",
        title, field_code
    );

    // 额外属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => match name.as_str() {
                "title" | "field_type" | "value" => { /* 已用于构造器 */ }
                "description" => {
                    code.push_str(&format!(".description({:?})", value));
                }
                "disabled" => {
                    code.push_str(&format!(".disabled({})", parse_bool(value)));
                }
                "layout" => {
                    if let Some(layout) = parse_layout(value) {
                        code.push_str(&format!(".layout({})", layout));
                    }
                }
                "keywords" => {
                    code.push_str(&format!(".keywords({})", parse_keywords(value)));
                }
                _ => {
                    if let Some(setter) =
                        component_static_setter(name, value, "SettingItem")
                    {
                        code.push_str(&setter);
                    }
                }
            },
            Attribute::Bind { name, expr, .. } => {
                if name == "value" {
                    continue; // 已用于 field getter
                }
                if let Some(setter) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "SettingItem",
                ) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if name == "on_change" {
                    continue; // 已用于 field setter
                }
                if let Some(setter) = component_event_setter(name, handler, "SettingItem") {
                    code.push_str(&setter);
                }
            }
        }
    }

    Ok(code)
}

// ──────────────────────────────────────────────────────────────────────────
//  SettingField 构造
// ──────────────────────────────────────────────────────────────────────────

/// 生成 SettingField 构造代码
///
/// 根据 field-type 选择对应的 SettingField 构造器，并生成 getter/setter 闭包。
fn gen_setting_field(
    field_type: &str,
    value_expr: &str,
    on_change_handler: &EventHandler,
    elem: &Element,
    loop_vars: &[&str],
    computed: &[&str],
) -> Result<String, CodegenError> {
    let getter = gen_field_getter(value_expr, field_type, loop_vars, computed);
    let setter = gen_field_setter(on_change_handler, field_type, loop_vars, computed);

    let default_value = extract_static_attr(elem, "default_value");

    let field_code = match field_type {
        "switch" => {
            format!("rml_ui::SettingField::switch({}, {})", getter, setter)
        }
        "checkbox" => {
            format!("rml_ui::SettingField::checkbox({}, {})", getter, setter)
        }
        "input" => {
            format!("rml_ui::SettingField::input({}, {})", getter, setter)
        }
        "dropdown" => {
            // options 属性：bind 引用 ViewModel 的 Vec<(SharedString, SharedString)> 字段
            // clone 避免从共享引用中 move
            let options_code = extract_bind_attr(elem, "options").unwrap_or_else(|| {
                // 回退到空 vec（用户应提供 options 绑定）
                "std::vec::Vec::new()".to_string()
            });
            let options_rust = component_bind_rust_expr(&options_code, loop_vars, computed);
            format!(
                "rml_ui::SettingField::dropdown({}.clone(), {}, {})",
                options_rust, getter, setter
            )
        }
        "scrollable-dropdown" => {
            let options_code = extract_bind_attr(elem, "options").unwrap_or_else(|| {
                "std::vec::Vec::new()".to_string()
            });
            let options_rust = component_bind_rust_expr(&options_code, loop_vars, computed);
            format!(
                "rml_ui::SettingField::scrollable_dropdown({}.clone(), {}, {})",
                options_rust, getter, setter
            )
        }
        "number-input" => {
            // NumberFieldOptions 可通过 min/max/step 属性配置
            let options_code = gen_number_field_options(elem);
            format!(
                "rml_ui::SettingField::number_input({}, {}, {})",
                options_code, getter, setter
            )
        }
        _ => {
            return Err(CodegenError {
                message: format!(
                    "不支持的 field-type: {}（支持: switch/checkbox/input/dropdown/scrollable-dropdown/number-input）",
                    field_type
                ),
                span: Some(elem.span),
            });
        }
    };

    // default_value 属性 → .default_value(...)
    let final_code = if let Some(dv) = default_value {
        let dv_code = match field_type {
            "switch" | "checkbox" => parse_bool(&dv).to_string(),
            "input" | "dropdown" | "scrollable-dropdown" => format!("\"{}\"", dv),
            "number-input" => format!("{}f64", dv),
            _ => dv.clone(),
        };
        format!("{}.default_value({})", field_code, dv_code)
    } else {
        field_code
    };

    Ok(final_code)
}

/// 生成 getter 闭包代码
///
/// ```ignore
/// { let weak = cx.weak_entity(); move |app: &gpui::App| {
///     weak.upgrade().map(|e| e.read(app).field).unwrap_or(default) } }
/// ```
fn gen_field_getter(
    value_expr: &str,
    field_type: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    // 将 bind 表达式转为 Rust 代码（如 is_dark → self.is_dark）
    let rust_expr = component_bind_rust_expr(value_expr, loop_vars, computed);
    // 替换 self. / __rml_self_ref. 前缀为 e.read(app).
    let prefix = crate::compiler::expr::current_self_alias().unwrap_or("self");
    let field_access = rust_expr.replace(&format!("{}.", prefix), "e.read(app).");

    let default = match field_type {
        "switch" | "checkbox" => "false",
        "input" | "dropdown" | "scrollable-dropdown" => "gpui::SharedString::default()",
        "number-input" => "0.0f64",
        _ => "Default::default()",
    };

    // bool / f64 实现 Copy，可直接返回；SharedString 需 clone 避免从引用中 move
    let access_suffix = match field_type {
        "switch" | "checkbox" | "number-input" => "",
        _ => ".clone()",
    };

    format!(
        "{{\n                        \
         let weak = cx.weak_entity();\n                        \
         move |app: &gpui::App| {{\n                            \
         weak.upgrade().map(|e| {}{}).unwrap_or({})\n                        \
         }}\n                    \
         }}",
        field_access, access_suffix, default
    )
}

/// 生成 setter 闭包代码
///
/// ```ignore
/// { let weak = cx.weak_entity(); move |val: T, app: &mut gpui::App| {
///     if let Some(e) = weak.upgrade() {
///         e.update(app, |this, cx| { this.handler(val, cx); });
///     } } }
/// ```
fn gen_field_setter(
    handler: &EventHandler,
    field_type: &str,
    _loop_vars: &[&str],
    _computed: &[&str],
) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };

    let val_type = match field_type {
        "switch" | "checkbox" => "bool",
        "input" | "dropdown" | "scrollable-dropdown" => "gpui::SharedString",
        "number-input" => "f64",
        _ => "_",
    };

    format!(
        "{{\n                        \
         let weak = cx.weak_entity();\n                        \
         move |val: {}, app: &mut gpui::App| {{\n                            \
         if let Some(e) = weak.upgrade() {{\n                                \
         e.update(app, |this, cx| {{ this.{}(val, cx); }});\n                            \
         }}\n                        \
         }}\n                    \
         }}",
        val_type, method
    )
}

/// 生成 NumberFieldOptions 构造代码
///
/// 从 min/max/step 静态属性生成 `rml_ui::NumberFieldOptions { min: .., max: .., step: .. }`
fn gen_number_field_options(elem: &Element) -> String {
    let min = extract_static_attr(elem, "min")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(f64::MIN);
    let max = extract_static_attr(elem, "max")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(f64::MAX);
    let step = extract_static_attr(elem, "step")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0);

    format!(
        "rml_ui::NumberFieldOptions {{ min: {}f64, max: {}f64, step: {}f64 }}",
        min, max, step
    )
}

// ──────────────────────────────────────────────────────────────────────────
//  辅助函数
// ──────────────────────────────────────────────────────────────────────────

/// 从元素属性中提取 static 字符串值
fn extract_static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return Some(value.clone());
            }
        }
        None
    })
}

/// 从元素属性中提取 bind 表达式
fn extract_bind_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name: n, expr, .. } = attr {
            if n == name {
                return Some(expr.clone());
            }
        }
        None
    })
}

/// 从元素属性中提取 event handler
fn extract_event_handler(elem: &Element, name: &str) -> Option<EventHandler> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Event { name: n, handler, .. } = attr {
            if n == name {
                return Some(handler.clone());
            }
        }
        None
    })
}

/// 解析像素值字符串 → Rust 代码
/// "280px" → "gpui::px(280.)"
fn parse_px_value(value: &str) -> Option<String> {
    let v = value.trim().trim_end_matches("px").trim();
    let n: f64 = v.parse().ok()?;
    let n_str = if n.fract() == 0.0 {
        format!("{:.0}.", n)
    } else {
        format!("{}", n)
    };
    Some(format!("gpui::px({})", n_str))
}

/// 解析布尔值
fn parse_bool(value: &str) -> &'static str {
    if value.is_empty() || value.eq_ignore_ascii_case("true") {
        "true"
    } else {
        "false"
    }
}

/// 解析 GroupBoxVariant
/// "fill" → "rml_ui::GroupBoxVariant::Fill"
fn parse_group_variant(value: &str) -> Option<String> {
    match value.trim().to_lowercase().as_str() {
        "normal" => Some("rml_ui::GroupBoxVariant::Normal".to_string()),
        "fill" => Some("rml_ui::GroupBoxVariant::Fill".to_string()),
        "outline" => Some("rml_ui::GroupBoxVariant::Outline".to_string()),
        _ => None,
    }
}

/// 解析 Size 枚举
/// "small" → "rml_ui::Size::Small"
fn parse_size(value: &str) -> Option<String> {
    match value.trim().to_lowercase().as_str() {
        "small" => Some("rml_ui::Size::Small".to_string()),
        "medium" => Some("rml_ui::Size::Medium".to_string()),
        "large" => Some("rml_ui::Size::Large".to_string()),
        _ => None,
    }
}

/// 解析布局
/// "vertical" → "gpui::Axis::Vertical"
/// "horizontal" → "gpui::Axis::Horizontal"
fn parse_layout(value: &str) -> Option<String> {
    match value.trim().to_lowercase().as_str() {
        "vertical" | "v" => Some("gpui::Axis::Vertical".to_string()),
        "horizontal" | "h" => Some("gpui::Axis::Horizontal".to_string()),
        _ => None,
    }
}

/// 解析关键词列表
/// "mfa,auth" → `vec!["mfa".to_string(), "auth".to_string()]`
fn parse_keywords(value: &str) -> String {
    let items: Vec<String> = value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| format!("{:?}", s))
        .collect();
    if items.is_empty() {
        return "std::vec::Vec::new()".to_string();
    }
    format!("vec![{}]", items.join(", "))
}

/// 注册 Settings translator
pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SettingsTranslator);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Element, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_element(tag: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_settings_minimal() {
        let elem = make_element("Settings", vec![], vec![]);
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&elem, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains("rml_ui::Settings::new"));
        assert!(code.contains("\"rml_el\""));
    }

    #[test]
    fn gen_settings_with_sidebar_width() {
        let elem = make_element(
            "Settings",
            vec![Attribute::Static {
                name: "sidebar_width".into(),
                value: "280px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&elem, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains(".sidebar_width(gpui::px(280.))"));
    }

    #[test]
    fn gen_settings_with_switch_item() {
        let item = make_element(
            "SettingItem",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "暗色主题".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "field_type".into(),
                    value: "switch".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "value".into(),
                    expr: "is_dark".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_change".into(),
                    handler: EventHandler::Ident("on_dark_change".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let group = make_element(
            "SettingGroup",
            vec![Attribute::Static {
                name: "title".into(),
                value: "外观".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(item)],
        );
        let page = make_element(
            "SettingPage",
            vec![Attribute::Static {
                name: "title".into(),
                value: "通用".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(group)],
        );
        let settings = make_element("Settings", vec![], vec![Node::Element(page)]);
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&settings, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains(".page("));
        assert!(code.contains("rml_ui::SettingPage::new(\"通用\")"));
        assert!(code.contains(".group("));
        assert!(code.contains("rml_ui::SettingGroup::new()"));
        assert!(code.contains(".title(\"外观\")"));
        assert!(code.contains(".item("));
        assert!(code.contains("rml_ui::SettingItem::new(\"暗色主题\""));
        assert!(code.contains("rml_ui::SettingField::switch"));
        assert!(code.contains("cx.weak_entity()"));
        assert!(code.contains("e.read(app).is_dark"));
        assert!(code.contains("this.on_dark_change(val, cx)"));
    }

    #[test]
    fn gen_settings_rejects_non_page_child() {
        let div = make_element("div", vec![], vec![]);
        let settings = make_element("Settings", vec![], vec![Node::Element(div)]);
        let mut id = 0;
        let result = SettingsTranslator.to_rust(&settings, &ctx(), &mut id, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("仅支持 <SettingPage>"));
    }

    #[test]
    fn gen_settings_kebab_case_tag() {
        let elem = make_element("settings", vec![], vec![]);
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&elem, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains("rml_ui::Settings::new"));
    }

    #[test]
    fn gen_settings_dropdown_item() {
        let item = make_element(
            "SettingItem",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "主题".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "field_type".into(),
                    value: "dropdown".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "value".into(),
                    expr: "theme_name".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "options".into(),
                    expr: "theme_options".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_change".into(),
                    handler: EventHandler::Ident("on_theme_change".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let group = make_element("SettingGroup", vec![], vec![Node::Element(item)]);
        let page = make_element(
            "SettingPage",
            vec![Attribute::Static {
                name: "title".into(),
                value: "通用".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(group)],
        );
        let settings = make_element("Settings", vec![], vec![Node::Element(page)]);
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&settings, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains("rml_ui::SettingField::dropdown"));
        assert!(code.contains("self.theme_options"));
        assert!(code.contains("e.read(app).theme_name"));
        assert!(code.contains("this.on_theme_change(val, cx)"));
    }

    #[test]
    fn gen_settings_number_input_item() {
        let item = make_element(
            "SettingItem",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "字体大小".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "field_type".into(),
                    value: "number-input".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "min".into(),
                    value: "8".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "max".into(),
                    value: "32".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "step".into(),
                    value: "0.5".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "value".into(),
                    expr: "font_size".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_change".into(),
                    handler: EventHandler::Ident("on_font_size_change".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let group = make_element("SettingGroup", vec![], vec![Node::Element(item)]);
        let page = make_element(
            "SettingPage",
            vec![Attribute::Static {
                name: "title".into(),
                value: "通用".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(group)],
        );
        let settings = make_element("Settings", vec![], vec![Node::Element(page)]);
        let mut id = 0;
        let (code, _) = SettingsTranslator
            .to_rust(&settings, &ctx(), &mut id, &Vec::new(), &[])
            .unwrap();
        assert!(code.contains("rml_ui::SettingField::number_input"));
        assert!(code.contains("rml_ui::NumberFieldOptions"));
        assert!(code.contains("min: 8f64"));
        assert!(code.contains("max: 32f64"));
        assert!(code.contains("step: 0.5f64"));
        assert!(code.contains("e.read(app).font_size"));
        assert!(code.contains("this.on_font_size_change(val, cx)"));
    }
}
