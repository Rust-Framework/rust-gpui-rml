//! 扩展组件代码生成（gpui-component 路由）
//!
//! 生成 `rml_ui::<Type>::new(...).<method>(...).on_click(cx.listener(...))` 形式调用。
//! 详见开发规划 §2.5 Layer 5 + §三 Phase B-3。
//!
//! ## 与原生元素 codegen 的区别
//!
//! | 维度 | 原生 div/h1 等 | 扩展 Button/Input 等 |
//! |------|---------------|---------------------|
//! | 构造 | `gpui::div()` | `rml_ui::Button::new(id)` |
//! | 属性 | `.child(text)` / `.when(cond, \|el\| el)` | `.label(...)` / `.primary()` |
//! | 事件 | `apply_event`（2 参闭包） | `component_event_setter`（3 参闭包 + cx.listener） |
//! | ID | 按需 `.id(...)` | 构造时必须传入 ElementId |

use crate::compiler::expr;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use crate::tags;

/// 生成扩展组件构造代码
///
/// 支持的语法：
/// - 静态属性：`label="Click me"` → `.label("Click me")`
/// - 布尔属性：`primary=""` / `primary="true"` → `.primary()`
/// - 绑定属性：`value={count}` → `.value(self.count.clone())`（仅对支持此方法的组件）
/// - 事件绑定：`onclick={increment}` → `.on_click(cx.listener(move |this, _ev, _window, cx| { ... }))`
/// - 文本子节点：`<Button>Click me</Button>` → `.label("Click me")`
pub fn gen_component(
    elem: &Element,
    ctx: &CodegenCtx,
    _depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let tag = &elem.tag;
    let component = tags::component_lookup(tag).ok_or_else(|| CodegenError {
        message: format!(
            "unknown extension component: <{}> (not in gpui-component routing table)",
            tag
        ),
    })?;

    // 1. 构造器
    //    若元素有 ref="name" 指令，使用稳定 ID `("rml_ref", "name")`，
    //    否则使用自增计数器生成 ID。
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref(name) => Some(name.as_str()),
        _ => None,
    });

    let id_val = *id_counter;
    *id_counter += 1;
    let mut code = match component.kind {
        tags::ComponentKind::Stateless => {
            if let Some(name) = ref_name {
                // ref 指令：用 "rml_ref:<name>" 稳定字符串作为 ElementId
                format!("{}::new({:?})", component.ctor_path, format!("rml_ref:{}", name))
            } else {
                format!("{}::new((\"rml_el\", {}usize))", component.ctor_path, id_val)
            }
        }
        tags::ComponentKind::StatelessNoId => {
            // 无参构造：TitleBar::new() / StatusBar::new()
            format!("{}::new()", component.ctor_path)
        }
        tags::ComponentKind::Stateful { state_field } if tag == "Tree" => format!(
            "{}::new(self.{}.as_ref().expect(\"init TreeState in on_loaded\"))",
            component.ctor_path, state_field
        ),
        tags::ComponentKind::Stateful { state_field } => format!(
            "{}::new(&self.{})",
            component.ctor_path, state_field
        ),
    };

    // 将 loop_vars 转换为 &[&str] 供绑定表达式使用
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();

    // 2. 静态属性 → builder 方法
    let mut label_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(setter) = component_static_setter(name, value, tag) {
                    code.push_str(&setter);
                    if name == "label" {
                        label_set_by_attr = true;
                    }
                }
            }
            Attribute::Bind { name, expr } => {
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, tag) {
                    code.push_str(&setter);
                }
            }
            Attribute::Event { name, handler } => {
                if let Some(setter) = component_event_setter(name, handler, tag) {
                    code.push_str(&setter);
                }
            }
        }
    }

    // 3. 子节点处理
    //
    // StatelessNoId 容器组件（TitleBar/StatusBar）实现 `ParentElement`，
    // 接收 element 子节点作为业务内容，用 `.child(...)` / `.children(...)` 传入。
    //
    // 其他组件（Button/Input 等）仅支持单个文本子节点作为 label（与显式 `label=` 互斥）。
    //
    // 注：ModernWindowShell 不经此路径处理——它由 codegen 根元素处理路径
    // （gen_modern_window_wrapper）直接生成，不通过 component_lookup 路由表。
    // ActivityBar 作为容器，子节点渲染到活动面板区域。
    let is_container =
        matches!(component.kind, tags::ComponentKind::StatelessNoId) || tag == "ActivityBar";

    if is_container {
        // 容器组件：所有 element/文本子节点作为 children
        let mut child_codes: Vec<String> = Vec::new();
        for child in &elem.children {
            let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
            if is_iter {
                // each 指令生成的迭代器：用 .children() 包裹
                child_codes.push(format!(".children({})", child_code));
            } else {
                child_codes.push(format!(".child({})", child_code));
            }
        }
        for child_code in child_codes {
            code.push_str(&format!("\n            {}", child_code));
        }
    } else if !label_set_by_attr {
        // 非容器组件：仅支持单个文本子节点作为 label
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".label({:?})", text));
                break;
            }
        }
    }

    Ok(code)
}

/// 静态属性 → builder 方法映射
///
/// - `label="..."` → `.label("...")`
/// - `placeholder="..."` → `.placeholder("...")`（Input 支持）
/// - `primary`/`secondary`/`danger`/`success`/`warning`/`info`/`ghost` → `.primary()` 等
/// - `disabled="true"` → `.disabled(true)`
/// - `selected`/`compact`/`loading` → 对应方法
/// - `xsmall`/`small`/`large` → Sizable 尺寸方法
/// - `font_bold`/`font_semibold` 等 → StyledExt 字体权重
/// - `h_flex`/`v_flex` → StyledExt 布局快捷方法
pub fn component_static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match name {
        "label" => Some(format!(".label({:?})", value)),
        "placeholder" => Some(format!(".placeholder({:?})", value)),
        "tooltip" => Some(format!(".tooltip({:?})", value)),
        // Button variant 属性（值为空或 "true" 时启用变体）
        "primary" | "secondary" | "danger" | "success" | "warning" | "info" | "ghost" | "link"
        | "text" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        // Sizable 尺寸方法（small 在 variant 和 size 中都适用，保持兼容）
        "small" | "xsmall" | "large" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        "compact" | "loading" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        // StyledExt 字体权重（值为空或 "true" 时启用）
        "font_thin" | "font_extralight" | "font_light" | "font_normal" | "font_medium"
        | "font_semibold" | "font_bold" | "font_extrabold" | "font_black" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        // StyledExt 布局快捷方法
        "h_flex" | "v_flex" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        "disabled" => Some(format!(".disabled({})", parse_bool(value))),
        "selected" => Some(format!(".selected({})", parse_bool(value))),
        // 通用样式属性（仅 div 等支持，组件侧通过 Styled trait 也支持部分）
        "class" | "id" | "style" | "src" | "href" | "type" | "value" => None,
        // ref 属性已在构造器中处理（生成稳定 ID），此处跳过
        "ref" => None,
        _ => {
            // 未知属性：保留为字符串属性供调试（不报错以兼容演进）
            let _ = tag;
            None
        }
    }
}

/// 绑定属性 → builder 方法映射
///
/// 利用表达式解析器支持复杂表达式：
/// - `value={count}` → `.value(self.count.clone())`
/// - `value={count + 1}` → `.value((self.count + 1).clone())`
/// - `label={user.name}` → `.label(self.user.name.clone())`
///
/// 对于无法解析的表达式，回退到简单的 `self.<expr>` 引用。
///
/// `tag` 参数保留用于未来组件专用 setter 扩展，当前无组件特定分支
/// （ModernWindowShell 的 menu/status_bar/title setter 已移至 codegen 根元素处理路径）。
pub fn component_bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let rust_expr = if let Some(code) = crate::compiler::codegen::try_gen_i18n_call(expr_str, loop_vars, computed) {
        code
    } else {
        match expr::parse(expr_str) {
        Ok(expr::Expr::Field(name)) if computed.iter().any(|c| *c == name.as_str()) => {
            if loop_vars.iter().any(|v| *v == name) {
                format!("{}()", name)
            } else {
                format!("self.{}()", name)
            }
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            let trimmed = expr_str.trim();
            if loop_vars.iter().any(|v| *v == trimmed) {
                trimmed.to_string()
            } else if computed.iter().any(|c| *c == trimmed) {
                format!("self.{}()", trimmed)
            } else {
                format!("self.{}", trimmed)
            }
        }
        }
    };
    let _tag = tag;
    match name {
        "value" => Some(format!(".value({}.clone())", rust_expr)),
        "disabled" => Some(format!(".disabled({})", rust_expr)),
        "selected" => Some(format!(".selected({})", rust_expr)),
        "checked" => Some(format!(".selected({})", rust_expr)),
        "label" => Some(format!(".label({}.clone())", rust_expr)),
        "panels" if tag == "ActivityBar" => Some(format!(".panels({}.clone())", rust_expr)),
        "actions" if tag == "ActivityBar" => Some(format!(".actions({}.clone())", rust_expr)),
        _ => None,
    }
}

/// 事件属性 → 组件事件方法映射
///
/// 与原生 div 的事件不同，gpui-component 组件的 on_* 方法接受 3 参闭包
/// `Fn(&ClickEvent, &mut Window, &mut App)`。通过 `cx.listener` 包装后可访问 `this`。
///
/// 支持的事件：
/// - `onclick`：所有组件通用，回调接收 `&gpui::ClickEvent`
/// - `onchange`：Input/TextInput 专用，回调接收 `&rml_ui::InputState`
pub fn component_event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    match name {
        "onclick" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            match handler {
                EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                    ".on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                     let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                     this.{}(&rml_ev, cx);\n                }}))",
                    method
                )),
                EventHandler::WithArgs(_, args) => {
                    if args.is_empty() {
                        Some(format!(
                            ".on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                             let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                             this.{}(&rml_ev, cx);\n                }}))",
                            method
                        ))
                    } else {
                        let arg = &args[0];
                        Some(format!(
                            ".on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                             let p0 = {}.clone();\n                    \
                             let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                             this.{}(p0, &rml_ev, cx);\n                }}))",
                            arg, method
                        ))
                    }
                }
            }
        }
        "onchange" if tag == "Input" || tag == "TextInput" => {
            // Input 组件的 on_change 回调接收 &InputState，而非标准事件
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| {{\n                    \
                 this.{}(state, cx);\n                }}))",
                method
            ))
        }
        "on_panel_change" if tag == "ActivityBar" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_panel_change(cx.listener(move |this, panel_id: &gpui::SharedString, _window, cx| {{\n                    \
                 this.{}(panel_id, cx);\n                }}))",
                method
            ))
        }
        "on_activate" if tag == "Tree" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
            };
            Some(format!(
                ".on_activate_rc(std::rc::Rc::new({{\n                    \
                 let weak = cx.weak_entity();\n                    \
                 move |item: rml_ui::TreeItem, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                 if let Some(entity) = weak.upgrade() {{\n                            \
                 entity.update(app, |this, cx| {{ this.{}(&item.id, cx); }});\n                        \
                 }}\n                    }}\n                }}))",
                method
            ))
        }
        _ => None,
    }
}

/// 解析 RML 属性值中的布尔字面量
pub fn parse_bool(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        "true"
    } else {
        "false"
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            stylesheet: None,
            computed_methods: Vec::new(),
            observable_fields: Vec::new(),
            computed_deps: std::collections::HashMap::new(),
            computed_returns: std::collections::HashMap::new(),
            field_types: std::collections::HashMap::new(),
            field_validations: std::collections::HashMap::new(),
            model_fields: Vec::new(),
        }
    }

    fn make_element(tag: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
        }
    }

    fn make_element_with_directives(
        tag: &str,
        attrs: Vec<Attribute>,
        directives: Vec<Directive>,
        children: Vec<Node>,
    ) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives,
            children,
        }
    }

    // ─── parse_bool ───

    #[test]
    fn parse_bool_true_variants() {
        assert_eq!(parse_bool("true"), "true");
        assert_eq!(parse_bool("TRUE"), "true");
        assert_eq!(parse_bool("True"), "true");
        assert_eq!(parse_bool("1"), "true");
    }

    #[test]
    fn parse_bool_false_variants() {
        assert_eq!(parse_bool("false"), "false");
        assert_eq!(parse_bool(""), "false");
        assert_eq!(parse_bool("0"), "false");
        assert_eq!(parse_bool("anything"), "false");
    }

    // ─── component_static_setter ───

    #[test]
    fn static_setter_label() {
        let code = component_static_setter("label", "Click me", "Button").unwrap();
        assert_eq!(code, ".label(\"Click me\")");
    }

    #[test]
    fn static_setter_placeholder() {
        let code = component_static_setter("placeholder", "Enter name", "Input").unwrap();
        assert_eq!(code, ".placeholder(\"Enter name\")");
    }

    #[test]
    fn static_setter_tooltip() {
        let code = component_static_setter("tooltip", "Help text", "Button").unwrap();
        assert_eq!(code, ".tooltip(\"Help text\")");
    }

    #[test]
    fn static_setter_variant_empty() {
        // primary="" 启用变体
        let code = component_static_setter("primary", "", "Button").unwrap();
        assert_eq!(code, ".primary()");
    }

    #[test]
    fn static_setter_variant_true() {
        let code = component_static_setter("danger", "true", "Button").unwrap();
        assert_eq!(code, ".danger()");
    }

    #[test]
    fn static_setter_variant_case_insensitive() {
        let code = component_static_setter("secondary", "TRUE", "Button").unwrap();
        assert_eq!(code, ".secondary()");
    }

    #[test]
    fn static_setter_variant_false_returns_none() {
        // primary="false" 不启用变体
        assert!(component_static_setter("primary", "false", "Button").is_none());
        assert!(component_static_setter("danger", "0", "Button").is_none());
    }

    #[test]
    fn static_setter_all_variants() {
        for v in &[
            "primary", "secondary", "danger", "success", "warning", "info", "ghost", "link", "text",
        ] {
            let code = component_static_setter(v, "", "Button").unwrap();
            assert_eq!(code, format!(".{}()", v));
        }
    }

    #[test]
    fn static_setter_size_modifiers() {
        assert_eq!(
            component_static_setter("small", "", "Button").unwrap(),
            ".small()"
        );
        assert_eq!(
            component_static_setter("compact", "", "Button").unwrap(),
            ".compact()"
        );
        assert_eq!(
            component_static_setter("loading", "", "Button").unwrap(),
            ".loading()"
        );
    }

    #[test]
    fn static_setter_sizable_xsmall() {
        assert_eq!(
            component_static_setter("xsmall", "", "Button").unwrap(),
            ".xsmall()"
        );
    }

    #[test]
    fn static_setter_sizable_large() {
        assert_eq!(
            component_static_setter("large", "", "Button").unwrap(),
            ".large()"
        );
    }

    #[test]
    fn static_setter_font_weights() {
        assert_eq!(
            component_static_setter("font_bold", "", "Button").unwrap(),
            ".font_bold()"
        );
        assert_eq!(
            component_static_setter("font_semibold", "", "Label").unwrap(),
            ".font_semibold()"
        );
        assert_eq!(
            component_static_setter("font_thin", "", "Label").unwrap(),
            ".font_thin()"
        );
    }

    #[test]
    fn static_setter_font_weight_false_returns_none() {
        assert!(component_static_setter("font_bold", "false", "Button").is_none());
    }

    #[test]
    fn static_setter_layout_methods() {
        assert_eq!(
            component_static_setter("h_flex", "", "div").unwrap(),
            ".h_flex()"
        );
        assert_eq!(
            component_static_setter("v_flex", "", "div").unwrap(),
            ".v_flex()"
        );
    }

    #[test]
    fn static_setter_disabled() {
        assert_eq!(
            component_static_setter("disabled", "true", "Button").unwrap(),
            ".disabled(true)"
        );
        assert_eq!(
            component_static_setter("disabled", "false", "Button").unwrap(),
            ".disabled(false)"
        );
    }

    #[test]
    fn static_setter_selected() {
        assert_eq!(
            component_static_setter("selected", "true", "Button").unwrap(),
            ".selected(true)"
        );
    }

    #[test]
    fn static_setter_skipped_attrs_return_none() {
        // 这些属性在组件上下文中不直接映射到 builder 方法
        assert!(component_static_setter("class", "btn", "Button").is_none());
        assert!(component_static_setter("id", "my-id", "Button").is_none());
        assert!(component_static_setter("style", "color:red", "Button").is_none());
        assert!(component_static_setter("value", "42", "Button").is_none());
        assert!(component_static_setter("type", "submit", "Button").is_none());
    }

    #[test]
    fn static_setter_unknown_attr_returns_none() {
        assert!(component_static_setter("data-foo", "bar", "Button").is_none());
    }

    // ─── component_bind_setter ───

    #[test]
    fn bind_setter_value() {
        let code = component_bind_setter("value", "count", &[], &[], "Button").unwrap();
        assert_eq!(code, ".value(self.count.clone())");
    }

    #[test]
    fn bind_setter_value_with_expr() {
        // value={count + 1} → .value((self.count + 1).clone())
        let code = component_bind_setter("value", "count + 1", &[], &[], "Button").unwrap();
        assert_eq!(code, ".value((self.count + 1).clone())");
    }

    #[test]
    fn bind_setter_value_with_member_access() {
        // value={user.name} → .value(self.user.name.clone())
        let code = component_bind_setter("value", "user.name", &[], &[], "Button").unwrap();
        assert_eq!(code, ".value(self.user.name.clone())");
    }

    #[test]
    fn bind_setter_disabled_with_expr() {
        // disabled={count > 0} → .disabled((self.count > 0))
        let code = component_bind_setter("disabled", "count > 0", &[], &[], "Button").unwrap();
        assert_eq!(code, ".disabled((self.count > 0))");
    }

    #[test]
    fn bind_setter_label_with_expr() {
        // label={user.name} → .label(self.user.name.clone())
        let code = component_bind_setter("label", "user.name", &[], &[], "Button").unwrap();
        assert_eq!(code, ".label(self.user.name.clone())");
    }

    #[test]
    fn bind_setter_disabled() {
        let code = component_bind_setter("disabled", "is_locked", &[], &[], "Button").unwrap();
        assert_eq!(code, ".disabled(self.is_locked)");
    }

    #[test]
    fn bind_setter_selected() {
        let code = component_bind_setter("selected", "is_active", &[], &[], "Button").unwrap();
        assert_eq!(code, ".selected(self.is_active)");
    }

    #[test]
    fn bind_setter_checked_maps_to_selected() {
        // checked 绑定映射到 .selected()（GPUI Checkbox 使用 selected 状态）
        let code = component_bind_setter("checked", "flag", &[], &[], "Button").unwrap();
        assert_eq!(code, ".selected(self.flag)");
    }

    #[test]
    fn bind_setter_label() {
        let code = component_bind_setter("label", "title", &[], &[], "Button").unwrap();
        assert_eq!(code, ".label(self.title.clone())");
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(component_bind_setter("color", "theme", &[], &[], "Button").is_none());
        assert!(component_bind_setter("", "x", &[], &[], "Button").is_none());
    }

    #[test]
    fn bind_setter_loop_var() {
        // each={todo in todos} 内 value={todo.text} → .value(todo.text.clone())
        let code = component_bind_setter("value", "todo.text", &["todo"], &[], "Button").unwrap();
        assert_eq!(code, ".value(todo.text.clone())");
    }

    // ─── component_event_setter ───

    #[test]
    fn event_setter_onclick_ident() {
        let handler = EventHandler::Ident("increment".into());
        let code = component_event_setter("onclick", &handler, "Button").unwrap();
        assert!(code.starts_with(".on_click("));
        assert!(code.contains("gpui::ClickEvent"));
        assert!(code.contains("from_gpui_click(_ev)"));
        assert!(code.contains("this.increment"));
    }

    #[test]
    fn event_setter_onclick_method_name() {
        let handler = EventHandler::MethodName("handle_click".into());
        let code = component_event_setter("onclick", &handler, "Button").unwrap();
        assert!(code.contains("this.handle_click"));
    }

    #[test]
    fn event_setter_onclick_with_args_empty() {
        let handler = EventHandler::WithArgs("increment".into(), vec![]);
        let code = component_event_setter("onclick", &handler, "Button").unwrap();
        assert!(code.contains("this.increment"));
        assert!(!code.contains("p0"));
    }

    #[test]
    fn event_setter_onclick_with_args_single() {
        let handler = EventHandler::WithArgs("set_value".into(), vec!["42".into()]);
        let code = component_event_setter("onclick", &handler, "Button").unwrap();
        assert!(code.contains("let p0 = 42.clone();"));
        assert!(code.contains("this.set_value(p0,"));
    }

    #[test]
    fn event_setter_non_click_returns_none() {
        let handler = EventHandler::Ident("handler".into());
        // onchange 只在 Input/TextInput 上支持，Button 不支持
        assert!(component_event_setter("onchange", &handler, "Button").is_none());
        assert!(component_event_setter("oninput", &handler, "Input").is_none());
        assert!(component_event_setter("onhover", &handler, "Button").is_none());
        assert!(component_event_setter("oncustom", &handler, "Button").is_none());
    }

    #[test]
    fn event_setter_onchange_input_ident() {
        let handler = EventHandler::Ident("on_input_change".into());
        let code = component_event_setter("onchange", &handler, "Input").unwrap();
        assert!(code.starts_with(".on_change("));
        assert!(code.contains("rml_ui::InputState"));
        assert!(code.contains("this.on_input_change"));
        assert!(code.contains("state"));
    }

    #[test]
    fn event_setter_onchange_textinput() {
        let handler = EventHandler::Ident("on_text_change".into());
        let code = component_event_setter("onchange", &handler, "TextInput").unwrap();
        assert!(code.starts_with(".on_change("));
        assert!(code.contains("this.on_text_change"));
    }

    #[test]
    fn event_setter_onchange_method_name() {
        let handler = EventHandler::MethodName("handle_change".into());
        let code = component_event_setter("onchange", &handler, "Input").unwrap();
        assert!(code.contains("this.handle_change"));
    }

    #[test]
    fn event_setter_onchange_button_returns_none() {
        // onchange 不支持在 Button 上
        let handler = EventHandler::Ident("handler".into());
        assert!(component_event_setter("onchange", &handler, "Button").is_none());
    }

    // ─── gen_component ───

    #[test]
    fn gen_component_button_minimal() {
        let elem = make_element("Button", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Button::new"));
        assert!(code.contains("\"rml_el\""));
        assert_eq!(id, 1);
    }

    #[test]
    fn gen_component_button_with_label_attr() {
        let elem = make_element(
            "Button",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Submit".into(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"Submit\")"));
    }

    #[test]
    fn gen_component_button_with_text_child() {
        // <Button>Click me</Button> → .label("Click me")
        let elem = make_element("Button", vec![], vec![Node::Text("Click me".into())]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"Click me\")"));
    }

    #[test]
    fn gen_component_label_attr_overrides_text_child() {
        // 显式 label 属性优先于文本子节点
        let elem = make_element(
            "Button",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Explicit".into(),
            }],
            vec![Node::Text("Ignored".into())],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"Explicit\")"));
        assert!(!code.contains("Ignored"));
    }

    #[test]
    fn gen_component_button_with_variant() {
        let elem = make_element(
            "Button",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "Delete".into(),
                },
                Attribute::Static {
                    name: "danger".into(),
                    value: "".into(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"Delete\")"));
        assert!(code.contains(".danger()"));
    }

    #[test]
    fn gen_component_button_with_click_event() {
        let elem = make_element(
            "Button",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "+".into(),
                },
                Attribute::Event {
                    name: "onclick".into(),
                    handler: EventHandler::Ident("increment".into()),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"+\")"));
        assert!(code.contains(".on_click("));
        assert!(code.contains("this.increment"));
    }

    #[test]
    fn gen_component_button_with_value_bind() {
        let elem = make_element(
            "Button",
            vec![Attribute::Bind {
                name: "value".into(),
                expr: "count".into(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".value(self.count.clone())"));
    }

    #[test]
    fn gen_component_unknown_tag_errors() {
        let elem = make_element("NonExistent", vec![], vec![]);
        let mut id = 0;
        let result = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("unknown extension component"));
    }

    #[test]
    fn gen_component_increments_id_counter() {
        let elem = make_element("Button", vec![], vec![]);
        let mut id = 5;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("5usize"));
        assert_eq!(id, 6);
    }

    #[test]
    fn gen_component_input_with_onchange() {
        // <Input placeholder="Enter name" onchange={on_input_change} />
        let elem = make_element(
            "Input",
            vec![
                Attribute::Static {
                    name: "placeholder".into(),
                    value: "Enter name".into(),
                },
                Attribute::Event {
                    name: "onchange".into(),
                    handler: EventHandler::Ident("on_input_change".into()),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        // Stateful 组件构造为 Input::new(&self.input_state)
        assert!(code.contains("rml_ui::Input::new(&self.input_state)"));
        assert!(code.contains(".placeholder(\"Enter name\")"));
        assert!(code.contains(".on_change("));
        assert!(code.contains("rml_ui::InputState"));
        assert!(code.contains("this.on_input_change"));
    }

    #[test]
    fn gen_component_button_with_expr_bind() {
        // <Button value={count + 1} />
        let elem = make_element(
            "Button",
            vec![Attribute::Bind {
                name: "value".into(),
                expr: "count + 1".into(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        // 表达式解析器应生成 (self.count + 1).clone()
        assert!(code.contains(".value((self.count + 1).clone())"));
    }

    #[test]
    fn gen_component_button_with_sizable() {
        // <Button label="OK" large="" font_bold="" />
        let elem = make_element(
            "Button",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "OK".into(),
                },
                Attribute::Static {
                    name: "large".into(),
                    value: "".into(),
                },
                Attribute::Static {
                    name: "font_bold".into(),
                    value: "".into(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"OK\")"));
        assert!(code.contains(".large()"));
        assert!(code.contains(".font_bold()"));
    }

    // ─── ref 指令 ───

    #[test]
    fn gen_component_button_with_ref_uses_stable_id() {
        // <Button ref="submit_btn" /> → Button::new("rml_ref:submit_btn")
        let elem = make_element_with_directives(
            "Button",
            vec![],
            vec![Directive::Ref("submit_btn".into())],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Button::new(\"rml_ref:submit_btn\")"));
        // 不应使用计数器 ID
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_component_button_without_ref_uses_counter_id() {
        // 无 ref 时，使用计数器 ID
        let elem = make_element("Button", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Button::new((\"rml_el\", 0usize))"));
        assert_eq!(id, 1);
    }

    #[test]
    fn gen_component_ref_with_other_attrs() {
        // <Button ref="btn" label="OK" primary="" />
        let elem = make_element_with_directives(
            "Button",
            vec![
                Attribute::Static {
                    name: "label".into(),
                    value: "OK".into(),
                },
                Attribute::Static {
                    name: "primary".into(),
                    value: "".into(),
                },
            ],
            vec![Directive::Ref("btn".into())],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Button::new(\"rml_ref:btn\")"));
        assert!(code.contains(".label(\"OK\")"));
        assert!(code.contains(".primary()"));
    }

    #[test]
    fn static_setter_ref_returns_none() {
        // ref 是指令，不会作为静态属性出现，但防御性返回 None
        assert!(component_static_setter("ref", "name", "Button").is_none());
    }

    // ─── StatelessNoId 构造（TitleBar / StatusBar）───

    #[test]
    fn gen_component_titlebar_minimal() {
        // <TitleBar /> → rml_ui::TitleBar::new()
        let elem = make_element("TitleBar", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TitleBar::new()"));
        // StatelessNoId 不应生成 ElementId 参数
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_component_statusbar_minimal() {
        // <StatusBar /> → rml_ui::StatusBar::new()
        let elem = make_element("StatusBar", vec![], vec![]);
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::StatusBar::new()"));
    }

    #[test]
    fn gen_component_titlebar_ignores_ref_directive() {
        // StatelessNoId 组件不接受 ElementId，ref 指令应被忽略（不生成稳定 ID）
        let elem = make_element_with_directives(
            "TitleBar",
            vec![],
            vec![Directive::Ref("my_titlebar".into())],
            vec![],
        );
        let mut id = 0;
        let code = gen_component(&elem, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TitleBar::new()"));
        // 不应出现 ref: 前缀的 ID
        assert!(!code.contains("rml_ref"));
    }

    // ─── StatelessNoId 容器子节点 codegen ───
    // 注：ModernWindowShell 已从路由表移除，其子节点 codegen 由 codegen.rs 的
    // gen_modern_window_wrapper 处理。此处仅测试 TitleBar/StatusBar 的容器子节点行为。

    #[test]
    fn gen_component_button_does_not_use_child_for_element() {
        // Button（Stateless）不应将 element 子节点作为 .child() 传入
        // 仅文本子节点作为 .label()
        let button = make_element(
            "Button",
            vec![],
            vec![Node::Text("Click me".into())],
        );
        let mut id = 0;
        let code = gen_component(&button, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".label(\"Click me\")"));
        assert!(!code.contains(".child("));
    }

    #[test]
    fn gen_component_titlebar_with_child() {
        // TitleBar 也是 StatelessNoId 容器，应支持 .child(...)
        let title = make_element("h1", vec![], vec![Node::Text("My App".into())]);
        let bar = make_element("TitleBar", vec![], vec![Node::Element(title)]);
        let mut id = 0;
        let code = gen_component(&bar, &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::TitleBar::new()"));
        assert!(code.contains(".child("));
    }
}
