//! 用户自定义组件 codegen —— `#[component]` 标注的 struct 嵌入、属性传参与 slot 注入。
//!
//! 由 `component::gen_component` 在 `component_lookup` 未命中时调用。
//! 处理场景：
//! - 无属性无 slot：直接 clone entity
//! - 有属性：clone entity 后通过 `entity.update(cx, ...)` 注入属性值
//! - 有 slot：clone entity 后通过 `entity.update(cx, ...)` 注入 slot 渲染闭包
//!   slot 闭包通过 `cx.entity()` 捕获父视图 Entity<Self>，闭包内用
//!   `__rml_self_ref = entity.read(cx)` 获取父视图引用，使 slot 内容可引用
//!   父视图字段（self.items 等）。Entity<Self>: Send + Sync + 'static，可被 move 捕获。

use crate::compiler::codegen::gen_node;
use crate::compiler::component::{component_bind_rust_expr, parse_bool};
use crate::compiler::expr;
use crate::compiler::tabs::tab::extract_state_refs;
use crate::compiler::{CodegenCtx, CodegenError, UserComponentInfo};
use crate::parser::ast::{Attribute, Element};

/// 生成用户自定义组件嵌入代码
///
/// 无属性无 slot 时：直接 clone entity
/// ```text
/// self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()
/// ```
///
/// 有属性或 slot 时：clone entity 后通过 `entity.update(cx, ...)` 注入
/// ```text
/// {
///     let __rml_entity = self.case_doc_page.as_ref().expect("init CaseDocPage in on_loaded").clone();
///     __rml_entity.update(cx, |this, _cx| { this.title = "...".into(); });
///     __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_demo(...); });
///     __rml_entity
/// }
/// ```
///
/// 返回 `Entity<T>`，因 `T: Render`（由 `#[component]` 生成），
/// `Entity<T: Render>: IntoElement`，可直接作为子元素。
pub fn gen_user_component(
    info: &UserComponentInfo,
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let entity_expr = format!(
        "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
        info.entity_field, info.struct_name
    );

    // 生成属性赋值代码（Phase 1.3：用户组件属性传参）
    let mut prop_assigns: Vec<String> = Vec::new();
    for attr in &elem.attributes {
        if let Some(code) = gen_prop_assign(info, attr, ctx, loop_vars)? {
            prop_assigns.push(code);
        }
    }

    // 分离 slot 子节点与 default 子节点
    let (slot_children, default_children) = partition_user_component_children(elem);

    // 无属性赋值且无 slot 内容：直接 clone entity（保持原行为）
    if prop_assigns.is_empty() && slot_children.is_empty() && default_children.is_empty() {
        return Ok(entity_expr);
    }

    let has_slots = !slot_children.is_empty()
        || (!default_children.is_empty() && info.slots.iter().any(|s| s == "default"));

    let mut code = String::new();
    code.push_str("{\n");
    code.push_str(&format!("    let __rml_entity = {};\n", entity_expr));

    // 有 slot 内容时，捕获父视图 Entity，让 slot 闭包可通过 __rml_self_ref 引用父视图数据。
    // Entity<Self>: Send + Sync + 'static（不依赖 T 的 Send/Sync），可被 move 闭包捕获。
    if has_slots {
        code.push_str("    let __rml_self_entity = cx.entity();\n");
    }

    // 属性注入（在 slot 处理前）
    for assign in &prop_assigns {
        code.push_str(&format!("    {}\n", assign));
    }

    // 为每个具名 slot 生成渲染闭包 + 注入
    //
    // slot 字段类型为 `Option<SlotRenderer>`（`Box<dyn Fn(&dyn ISlotScope, &mut Window, &mut App) -> AnyElement + Send + Sync>`）。
    // 闭包通过 `cx.entity()` 捕获父视图 Entity<Self>，闭包内用
    // `__rml_self_ref = __rml_self_entity.read(cx)` 获取父视图引用，
    // 使 slot 内容可引用父视图字段（self.items 等）。
    //
    // 生成 slot 内容时用 `with_self_alias("__rml_self_ref", ...)` 设置 thread-local 别名，
    // 使 `to_rust_code_with_ctx` / `gen_expr_code` / `component_bind_rust_expr` 把
    // `self.xxx` 替换为 `__rml_self_ref.xxx`，绕过 slot 闭包的生命周期限制。
    //
    // 在 `update(cx, ...)` 闭包外构造闭包，再传入 setter，避免 cx 借用冲突。
    //
    // 闭包首参 `_scope: &dyn ISlotScope` 由插槽宿主构造传入，自定义组件默认传
    // `NullSlotScope`，不写 `scope={...}` 时以 `_scope` 忽略，向后兼容。
    for (slot_name, slot_nodes) in &slot_children {
        let slot_code = expr::with_self_alias("__rml_self_ref", || {
            gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)
        })?;
        // 提取 self.__rml_state.get_or_init_ref(...) 到 prelude（render 作用域），
        // 使 slot 闭包（Fn）不捕获 &mut self，而是 move 捕获提取的 Entity 变量。
        // 变量名带 slot_name 前缀避免多 slot 场景冲突。
        let var_prefix = format!("__rml_slot_{}_entity_", slot_name);
        let (prelude, slot_code_replaced) = extract_state_refs(&slot_code, &var_prefix);
        let binding = format!("__rml_slot_{}_value", slot_name);
        // 先发射 prelude（render 作用域，self 是 &mut Self）
        if !prelude.is_empty() {
            code.push_str(&format!("    {}\n", prelude));
        }
        // 每个 slot 闭包前 clone __rml_self_entity，避免被 move 后无法用于其他 slot 闭包。
        // 闭包内通过 `__rml_self_entity.update(_app, |this, cx| { ... })` 进入 &mut Context<Self>，
        // 使 slot 内容的 `cx.listener(...)` / `cx.t(...)` 等调用可用（与 wrap_shell_slot 同模式）。
        code.push_str(&format!(
            "    let {}: rml_core::slot::SlotRenderer = Box::new({{ let __rml_self_entity = __rml_self_entity.clone(); move |_scope: &dyn rml_core::slot::ISlotScope, _window: &mut gpui::Window, _app: &mut gpui::App| -> gpui::AnyElement {{ __rml_self_entity.update(_app, |this, cx| {{ let __rml_self_ref: &Self = this; ({}).into_any_element() }}) }} }});\n",
            binding, slot_code_replaced
        ));
        code.push_str(&format!(
            "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_{}({}); }});\n",
            slot_name, binding
        ));
    }

    // default 插槽（无 slot 属性的子节点）
    if !default_children.is_empty() && info.slots.iter().any(|s| s == "default") {
        let default_code = expr::with_self_alias("__rml_self_ref", || {
            gen_slot_content(&default_children, ctx, id_counter, loop_vars)
        })?;
        let (prelude, default_code_replaced) =
            extract_state_refs(&default_code, "__rml_slot_default_entity_");
        if !prelude.is_empty() {
            code.push_str(&format!("    {}\n", prelude));
        }
        code.push_str("    let __rml_slot_default_value: rml_core::slot::SlotRenderer = Box::new({ let __rml_self_entity = __rml_self_entity.clone(); move |_scope: &dyn rml_core::slot::ISlotScope, _window: &mut gpui::Window, _app: &mut gpui::App| -> gpui::AnyElement { __rml_self_entity.update(_app, |this, cx| { let __rml_self_ref: &Self = this; (");
        code.push_str(&default_code_replaced);
        code.push_str(").into_any_element() }) } });\n");
        code.push_str(
            "    __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_default(__rml_slot_default_value); });\n",
        );
    }

    code.push_str("    __rml_entity\n");
    code.push('}');
    Ok(code)
}

/// 为用户组件属性生成赋值代码（Phase 1.3）
///
/// 根据 `info.field_types` 生成类型转换代码，注入到子组件 entity。
///
/// - 静态属性 `title="..."` → `__rml_entity.update(cx, |this, _cx| { this.title = "...".into(); });`
/// - 绑定属性 `sample={sample}` → `{ let __rml_value_sample = self.sample(); __rml_entity.update(cx, |this, _cx| { this.sample = (__rml_value_sample).into(); }); }`
///   （在 update 闭包外计算表达式值，避免 cx.t(...) 等引用 cx 的表达式与 update(cx, ...) 借用冲突）
/// - 事件属性：跳过（Phase 1 不处理用户组件事件）
/// - 非组件属性（ref/class/id/style/slot）：跳过（由其他路径处理）
/// - 未在 field_types 中登记的属性：跳过（留待 Phase 4 编译期校验）
fn gen_prop_assign(
    info: &UserComponentInfo,
    attr: &Attribute,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Result<Option<String>, CodegenError> {
    let (name, attr_value): (&str, PropValue) = match attr {
        Attribute::Static { name, value, .. } => (name.as_str(), PropValue::Static(value)),
        Attribute::Bind { name, expr, .. } => (name.as_str(), PropValue::Bind(expr)),
        Attribute::Event { .. } => return Ok(None),
    };

    // 查询字段类型，未命中则跳过
    let field_type = match info.field_types.get(name) {
        Some(t) => t.as_str(),
        None => return Ok(None),
    };

    let loop_vars_slice: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed_slice: Vec<&str> = ctx
        .computed_methods
        .iter()
        .map(|s| s.as_str())
        .collect();

    match attr_value {
        PropValue::Static(value) => {
            let assign_expr = gen_static_assign(name, value, field_type)?;
            Ok(Some(format!(
                "__rml_entity.update(cx, |this, _cx| {{ {} }});",
                assign_expr
            )))
        }
        PropValue::Bind(expr) => {
            let rust_expr = component_bind_rust_expr(expr, &loop_vars_slice, &computed_slice);
            // 在 update 闭包外计算表达式值，避免 cx 借用冲突
            // （如 cx.t(...) 与 update(cx, ...) 冲突）
            let value_var = format!("__rml_value_{}", name);
            let assign_expr = gen_bind_assign(name, &value_var, field_type);
            Ok(Some(format!(
                "{{ let {} = {}; __rml_entity.update(cx, |this, _cx| {{ {} }}); }}",
                value_var, rust_expr, assign_expr
            )))
        }
    }
}

/// 静态属性值
enum PropValue<'a> {
    Static(&'a str),
    Bind(&'a str),
}

/// 为静态属性生成赋值表达式
fn gen_static_assign(
    field_name: &str,
    value: &str,
    field_type: &str,
) -> Result<String, CodegenError> {
    match field_type {
        "String" | "SharedString" | "gpui::SharedString" => {
            Ok(format!("this.{} = {:?}.into();", field_name, value))
        }
        "i32" | "u32" | "usize" | "i64" | "u64" | "f64" | "f32" => {
            Ok(format!(
                "this.{} = {:?}.parse().unwrap_or(0);",
                field_name, value
            ))
        }
        "bool" => Ok(format!("this.{} = {};", field_name, parse_bool(value))),
        _ => Ok(format!("this.{} = {:?}.into();", field_name, value)),
    }
}

/// 为绑定属性生成赋值表达式
fn gen_bind_assign(field_name: &str, rust_expr: &str, field_type: &str) -> String {
    match field_type {
        "String" | "SharedString" | "gpui::SharedString" => {
            format!("this.{} = ({}).into();", field_name, rust_expr)
        }
        "i32" | "u32" | "usize" | "i64" | "u64" | "f64" | "f32" | "bool" => {
            format!("this.{} = {};", field_name, rust_expr)
        }
        _ => format!("this.{} = ({}).clone();", field_name, rust_expr),
    }
}

/// 将用户组件的子节点分离为具名插槽内容与默认插槽内容
///
/// - `<template slot="name">...</template>` → slot_children[name]
/// - 其他子节点 → default_children
///
/// 返回 (slot_children: HashMap<slot_name, Vec<Node>>, default_children: Vec<Node>)
fn partition_user_component_children(
    elem: &Element,
) -> (
    std::collections::HashMap<String, Vec<crate::parser::ast::Node>>,
    Vec<crate::parser::ast::Node>,
) {
    let mut slot_children: std::collections::HashMap<String, Vec<crate::parser::ast::Node>> =
        std::collections::HashMap::new();
    let mut default_children: Vec<crate::parser::ast::Node> = Vec::new();

    for child in &elem.children {
        if let crate::parser::ast::Node::Element(child_elem) = child {
            if child_elem.tag == "template" {
                if let Some(slot_name) = &child_elem.slot_name {
                    slot_children
                        .entry(slot_name.clone())
                        .or_default()
                        .extend(child_elem.children.clone());
                    continue;
                }
            }
        }
        default_children.push(child.clone());
    }

    (slot_children, default_children)
}

/// 为 slot 内容子节点列表生成构建代码
///
/// - 空列表：返回 `gpui::Empty`（不渲染）
/// - 单节点：直接生成节点代码
/// - 多节点：包裹 `gpui::div().child(...).child(...)` 容器
fn gen_slot_content(
    nodes: &[crate::parser::ast::Node],
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    if nodes.is_empty() {
        return Ok("gpui::Empty".to_string());
    }
    if nodes.len() == 1 {
        let (code, _) = gen_node(&nodes[0], ctx, 0, id_counter, loop_vars)?;
        return Ok(code);
    }
    let mut code = String::from("gpui::div()");
    for node in nodes {
        let (node_code, is_iter) = gen_node(node, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", node_code));
        } else {
            code.push_str(&format!(".child({})", node_code));
        }
    }
    Ok(code)
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::UserComponentInfo;
    use crate::parser::ast::{Attribute, Element, EventHandler, Node};
    use crate::parser::Span;
    use std::collections::HashMap;

    fn make_info(name: &str, field_types: &[(&str, &str)]) -> UserComponentInfo {
        let mut ft = HashMap::new();
        for (k, v) in field_types {
            ft.insert((*k).to_string(), (*v).to_string());
        }
        let entity_field = to_snake_case_helper(name);
        UserComponentInfo {
            struct_name: name.into(),
            entity_field,
            slots: vec![],
            field_types: ft,
            computed_methods: vec![],
        }
    }

    fn to_snake_case_helper(s: &str) -> String {
        let mut out = String::new();
        for (i, c) in s.chars().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    out.push('_');
                }
                out.push(c.to_ascii_lowercase());
            } else {
                out.push(c);
            }
        }
        out
    }

    fn make_info_with_slots(
        name: &str,
        field_types: &[(&str, &str)],
        slots: &[&str],
    ) -> UserComponentInfo {
        let mut info = make_info(name, field_types);
        info.slots = slots.iter().map(|s| (*s).to_string()).collect();
        info
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

    fn static_attr(name: &str, value: &str) -> Attribute {
        Attribute::Static {
            name: name.into(),
            value: value.into(),
            span: Span::empty(),
        }
    }

    fn bind_attr(name: &str, expr: &str) -> Attribute {
        Attribute::Bind {
            name: name.into(),
            expr: expr.into(),
            span: Span::empty(),
        }
    }

    fn event_attr(name: &str, handler: &str) -> Attribute {
        Attribute::Event {
            name: name.into(),
            handler: EventHandler::MethodName(handler.into()),
            span: Span::empty(),
        }
    }

    fn ctx_with_computed(computed: &[&str]) -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            computed_methods: computed.iter().map(|s| (*s).to_string()).collect(),
            ..Default::default()
        }
    }

    fn gen(info: &UserComponentInfo, elem: &Element, ctx: &CodegenCtx) -> String {
        let mut id_counter = 0usize;
        let empty: Vec<String> = Vec::new();
        gen_user_component(info, elem, ctx, &mut id_counter, &empty).unwrap()
    }

    // ─── 静态属性 ───

    #[test]
    fn test_static_string_prop() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element("MyComp", vec![static_attr("title", "hello")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains(r#"this.title = "hello".into();"#),
            "expected title assignment, got: {}",
            code
        );
    }

    #[test]
    fn test_static_numeric_prop() {
        let info = make_info("MyComp", &[("count", "i32")]);
        let elem = make_element("MyComp", vec![static_attr("count", "42")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains(r#"this.count = "42".parse().unwrap_or(0);"#),
            "expected count parse, got: {}",
            code
        );
    }

    #[test]
    fn test_static_bool_prop() {
        let info = make_info("MyComp", &[("disabled", "bool")]);
        let elem = make_element("MyComp", vec![static_attr("disabled", "true")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("this.disabled = true;"),
            "expected disabled=true, got: {}",
            code
        );
    }

    // ─── 绑定属性 ───

    #[test]
    fn test_bind_field_prop() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element("MyComp", vec![bind_attr("title", "title")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("let __rml_value_title = self.title;"),
            "expected bind value pre-computation, got: {}",
            code
        );
        assert!(
            code.contains("this.title = (__rml_value_title).into();"),
            "expected bind field assignment via value var, got: {}",
            code
        );
    }

    #[test]
    fn test_bind_computed_prop() {
        let info = make_info("MyComp", &[("sample", "SharedString")]);
        let elem = make_element("MyComp", vec![bind_attr("sample", "sample")], vec![]);
        let ctx = ctx_with_computed(&["sample"]);
        let code = gen(&info, &elem, &ctx);
        assert!(
            code.contains("let __rml_value_sample = self.sample();"),
            "expected computed method pre-computation, got: {}",
            code
        );
        assert!(
            code.contains("this.sample = (__rml_value_sample).into();"),
            "expected computed assignment via value var, got: {}",
            code
        );
    }

    #[test]
    fn test_bind_numeric_field() {
        let info = make_info("MyComp", &[("count", "i32")]);
        let elem = make_element("MyComp", vec![bind_attr("count", "count")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("let __rml_value_count = self.count;"),
            "expected numeric field pre-computation, got: {}",
            code
        );
        assert!(
            code.contains("this.count = __rml_value_count;"),
            "expected numeric assignment via value var, got: {}",
            code
        );
    }

    #[test]
    fn test_bind_i18n_call_prop() {
        // title={t("case.table.title")} 应生成闭包外计算 + update 闭包内赋值，
        // 避免 cx.t(...) 与 update(cx, ...) 借用冲突。
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element(
            "MyComp",
            vec![bind_attr("title", "t(\"case.table.title\")")],
            vec![],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("let __rml_value_title = cx.t(\"case.table.title\");"),
            "expected i18n call pre-computation outside update closure, got: {}",
            code
        );
        assert!(
            code.contains("this.title = (__rml_value_title).into();"),
            "expected i18n value assignment via value var, got: {}",
            code
        );
        // 确保闭包内不再直接调用 cx.t(...)
        let update_closure_start = code.find("__rml_entity.update(cx,").unwrap();
        let update_closure_end = code[update_closure_start..].find("});").unwrap();
        let closure_body = &code[update_closure_start..update_closure_start + update_closure_end];
        assert!(
            !closure_body.contains("cx.t("),
            "cx.t(...) should be outside update closure, but found inside: {}",
            closure_body
        );
    }

    // ─── 跳过逻辑 ───

    #[test]
    fn test_skip_non_prop_attributes() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element(
            "MyComp",
            vec![static_attr("class", "foo"), static_attr("ref", "bar")],
            vec![],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        // class/ref 不在 field_types 中，应跳过；无任何属性赋值且无 slot → 直接返回 entity_expr
        assert!(
            !code.contains("__rml_entity.update"),
            "non-prop attributes should be skipped, got: {}",
            code
        );
    }

    #[test]
    fn test_skip_event_attributes() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element(
            "MyComp",
            vec![event_attr("onclick", "handle_click")],
            vec![],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            !code.contains("__rml_entity.update"),
            "event attributes should be skipped, got: {}",
            code
        );
    }

    // ─── 混合场景 ───

    #[test]
    fn test_mixed_props_and_slots() {
        let info = make_info_with_slots("MyComp", &[("title", "SharedString")], &["demo"]);
        let slot_child = Element {
            tag: "template".into(),
            slot_name: Some("demo".into()),
            children: vec![Node::Text("demo content".into())],
            ..Default::default()
        };
        let elem = make_element(
            "MyComp",
            vec![static_attr("title", "x")],
            vec![Node::Element(slot_child)],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains(r#"this.title = "x".into();"#),
            "expected title assignment in mixed scenario, got: {}",
            code
        );
        assert!(
            code.contains("__rml_set_slot_demo"),
            "expected slot demo injection in mixed scenario, got: {}",
            code
        );
    }

    #[test]
    fn test_no_props_no_slots() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element("MyComp", vec![], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            !code.starts_with("{"),
            "no props/slots should return bare entity_expr, got: {}",
            code
        );
        assert!(
            code.contains("self.my_comp.as_ref()"),
            "expected entity_expr, got: {}",
            code
        );
    }

    #[test]
    fn test_multiple_props() {
        let info = make_info(
            "MyComp",
            &[("title", "SharedString"), ("count", "i32"), ("active", "bool")],
        );
        let elem = make_element(
            "MyComp",
            vec![
                static_attr("title", "hello"),
                static_attr("count", "10"),
                bind_attr("active", "is_active"),
            ],
            vec![],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(code.contains(r#"this.title = "hello".into();"#), "title missing: {}", code);
        assert!(code.contains(r#"this.count = "10".parse().unwrap_or(0);"#), "count missing: {}", code);
        assert!(
            code.contains("let __rml_value_active = self.is_active;"),
            "active pre-computation missing: {}",
            code
        );
        assert!(
            code.contains("this.active = __rml_value_active;"),
            "active assignment via value var missing: {}",
            code
        );
    }

    // ─── Phase 2：slot 闭包捕获父视图数据 ───

    fn template_slot_node(slot_name: &str, children: Vec<Node>) -> Node {
        Node::Element(Element {
            tag: "template".into(),
            slot_name: Some(slot_name.into()),
            children,
            ..Default::default()
        })
    }

    fn interpolation_node(expr: &str) -> Node {
        Node::Interpolation {
            expr: expr.into(),
            span: Span::empty(),
        }
    }

    #[test]
    fn test_slot_closure_generates_self_entity_capture() {
        let info = make_info_with_slots("MyComp", &[], &["demo"]);
        let elem = make_element(
            "MyComp",
            vec![],
            vec![template_slot_node("demo", vec![Node::Text("content".into())])],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("let __rml_self_entity = cx.entity();"),
            "expected __rml_self_entity capture when slot present, got: {}",
            code
        );
        assert!(
            code.contains("let __rml_self_ref: &Self = this;"),
            "expected __rml_self_ref binding in slot closure, got: {}",
            code
        );
    }

    #[test]
    fn test_slot_closure_no_self_entity_without_slots() {
        let info = make_info("MyComp", &[("title", "SharedString")]);
        let elem = make_element("MyComp", vec![static_attr("title", "x")], vec![]);
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            !code.contains("__rml_self_entity"),
            "no slot should not generate __rml_self_entity, got: {}",
            code
        );
    }

    #[test]
    fn test_slot_closure_replaces_self_with_alias() {
        let info = make_info_with_slots("MyComp", &[], &["demo"]);
        let elem = make_element(
            "MyComp",
            vec![],
            vec![template_slot_node(
                "demo",
                vec![interpolation_node("items")],
            )],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("__rml_self_ref.items"),
            "expected self.items replaced with __rml_self_ref.items, got: {}",
            code
        );
    }

    #[test]
    fn test_slot_closure_computed_method() {
        let info = make_info_with_slots("MyComp", &[], &["demo"]);
        let ctx = ctx_with_computed(&["format_items"]);
        let elem = make_element(
            "MyComp",
            vec![],
            vec![template_slot_node(
                "demo",
                vec![interpolation_node("format_items")],
            )],
        );
        let code = gen(&info, &elem, &ctx);
        assert!(
            code.contains("__rml_self_ref.format_items()"),
            "expected computed method with alias, got: {}",
            code
        );
    }

    #[test]
    fn test_default_slot_closure_uses_alias() {
        let info = make_info_with_slots("MyComp", &[], &["default"]);
        let elem = make_element(
            "MyComp",
            vec![],
            vec![interpolation_node("data")],
        );
        let code = gen(&info, &elem, &CodegenCtx::default());
        assert!(
            code.contains("__rml_self_ref.data"),
            "expected default slot with alias, got: {}",
            code
        );
        assert!(
            code.contains("__rml_set_slot_default"),
            "expected default slot setter, got: {}",
            code
        );
    }
}
