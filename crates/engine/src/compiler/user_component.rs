//! 用户自定义组件 codegen —— `#[component]` 标注的 struct 嵌入与 slot 注入。
//!
//! 由 `component::gen_component` 在 `component_lookup` 未命中时调用。
//! 处理两种场景：
//! - 无 slot：直接 clone entity
//! - 有 slot：clone entity 后通过 `entity.update(cx, ...)` 注入 slot 渲染闭包

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError, UserComponentInfo};
use crate::parser::ast::Element;

/// 生成用户自定义组件嵌入代码
///
/// 无 slot 子节点时：直接 clone entity
/// ```text
/// self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()
/// ```
///
/// 有 slot 子节点时：clone entity 后通过 `entity.update(cx, |this, _cx| { ... })` 注入 slot 内容
/// ```text
/// {
///     let __rml_entity = self.card.as_ref().expect("init Card in on_loaded").clone();
///     __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_header(...); });
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

    // 组件未声明任何插槽：保持原行为（直接 clone entity）
    if info.slots.is_empty() {
        return Ok(entity_expr);
    }

    // 分离 slot 子节点与 default 子节点
    let (slot_children, default_children) = partition_user_component_children(elem);

    // 父视图未提供任何 slot 内容：保持原行为
    if slot_children.is_empty() && default_children.is_empty() {
        return Ok(entity_expr);
    }

    let mut code = String::new();
    code.push_str("{\n");
    code.push_str(&format!("    let __rml_entity = {};\n", entity_expr));

    // 为每个具名 slot 生成渲染闭包 + 注入
    //
    // slot 字段类型为 `Option<SlotRenderer>`（`Box<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync>`）。
    // 把 slot 内容表达式包装为闭包：
    //   - 闭包内 cx 是参数（&mut App），不捕获外部 cx，避免借用冲突
    //   - 闭包是 `move`，捕获 slot 内容中引用的外部变量（应为字面量或 Send + Sync 数据）
    //   - 闭包不捕获父视图的 `self`（生命周期不允许，slot 内容应通过子组件 props 传数据）
    //
    // 在 `update(cx, ...)` 闭包外构造闭包，再传入 setter，避免 cx 借用冲突。
    for (slot_name, slot_nodes) in &slot_children {
        let slot_code = gen_slot_content(slot_nodes, ctx, id_counter, loop_vars)?;
        let binding = format!("__rml_slot_{}_value", slot_name);
        code.push_str(&format!(
            "    let {}: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement {{ ({}).into_any_element() }});\n",
            binding, slot_code
        ));
        code.push_str(&format!(
            "    __rml_entity.update(cx, |this, _cx| {{ this.__rml_set_slot_{}({}); }});\n",
            slot_name, binding
        ));
    }

    // default 插槽（无 slot 属性的子节点）
    if !default_children.is_empty() && info.slots.iter().any(|s| s == "default") {
        let default_code = gen_slot_content(&default_children, ctx, id_counter, loop_vars)?;
        code.push_str("    let __rml_slot_default_value: rml_core::slot::SlotRenderer = Box::new(move |_window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement { (");
        code.push_str(&default_code);
        code.push_str(").into_any_element() });\n");
        code.push_str(
            "    __rml_entity.update(cx, |this, _cx| { this.__rml_set_slot_default(__rml_slot_default_value); });\n",
        );
    }

    code.push_str("    __rml_entity\n");
    code.push('}');
    Ok(code)
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
