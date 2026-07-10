//! StateBridge 注册表 —— Stateful 组件的双向绑定规格
//!
//! StateBridge 用于有独立 State Entity 的 Stateful 表单组件（Slider 及未来的
//! ColorPicker/Select 等）。与 `twoway.rs` 的 Stateless EventClick 机制互补：
//!
//! - **Stateless EventClick**（twoway.rs）：无 State Entity，on_click 直接回写
//! - **Stateful StateBridge**（本模块）：有 State Entity，通过 subscribe 事件回写
//!
//! ## 工作机制
//!
//! 1. 字段收集器扫描 `<Component bind_property={field}>`，按 bridge_key 分组
//! 2. `gen_state_bridge_impl()` 为每个注册的 spec 生成 `__rml_get_or_init_<suffix>_state` 方法
//! 3. `gen_model_state_bridge()` 生成 `Component::new(&state)` 元素代码
//! 4. RmlState 用类型擦除的 `state_bridge_entities` 存储各 State Entity
//!
//! ## 扩展新组件
//!
//! 添加新 StateBridge 组件只需：
//! 1. 在 `STATE_BRIDGE_REGISTRY` 中添加 `StateBridgeSpec` 条目
//! 2. 从 `rml_ui` crate re-export 对应的 State/Event 类型
//! 3. 在 `tags.rs` 中注册组件（`ComponentKind::Stateful`）

use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;
use std::collections::HashMap;

/// State 值类型 —— 决定正向/反向同步的代码生成方式
#[derive(Clone, Copy)]
pub enum ValueKind {
    /// 数值类型（f32 为中间表示）：forward `self.field as f32`，reverse `v as <type>`
    Numeric,
}

/// StateBridge 规格 —— 描述一个 Stateful 组件的双向绑定方式
///
/// 所有字段为 `&'static str`，作为代码生成模板注入到生成的 Rust 代码中。
#[derive(Clone)]
pub struct StateBridgeSpec {
    /// 规范化的组件标签名（"Slider"）
    pub component_tag: &'static str,
    /// 双向绑定属性名（"value"）
    pub bind_property: &'static str,
    /// State 类型完全限定路径（"rml_ui::SliderState"）
    pub state_type: &'static str,
    /// 生成方法名后缀（"slider" → `__rml_get_or_init_slider_state`）
    pub state_method_suffix: &'static str,
    /// RmlState 中的 bridge 键（"slider"）
    pub bridge_key: &'static str,
    /// State 构造表达式（"rml_ui::SliderState::new()"）
    pub state_ctor: &'static str,
    /// 组件构造路径（"rml_ui::Slider"）
    pub ctor_path: &'static str,
    /// 事件匹配臂（"rml_ui::SliderEvent::Change(value)"）
    pub event_match: &'static str,
    /// 事件载荷提取代码（从事件中提取 f32/String 值，赋给变量 `v`）
    pub event_payload_extract: &'static str,
    /// 正向 set_value 调用（`state.set_value(rml_ui::SliderValue::Single(value), window, cx)`）
    pub value_set_call: &'static str,
    /// 值类型（决定 forward/reverse 代码生成）
    pub value_kind: ValueKind,
}

/// StateBridge 注册表 —— 单一信源
static STATE_BRIDGE_REGISTRY: &[StateBridgeSpec] = &[
    StateBridgeSpec {
        component_tag: "Slider",
        bind_property: "value",
        state_type: "rml_ui::SliderState",
        state_method_suffix: "slider",
        bridge_key: "slider",
        state_ctor: "rml_ui::SliderState::new()",
        ctor_path: "rml_ui::Slider",
        event_match: "rml_ui::SliderEvent::Change(value)",
        event_payload_extract: "let v = match value { rml_ui::SliderValue::Single(v) => *v, _ => return };",
        value_set_call: "state.set_value(rml_ui::SliderValue::Single(value), window, cx)",
        value_kind: ValueKind::Numeric,
    },
];

/// 查询组件的指定属性是否支持 StateBridge 双向绑定
pub fn lookup_state_bridge(tag: &str, bind_property: &str) -> Option<&'static StateBridgeSpec> {
    let canonical = tags::canonical_tag(tag);
    STATE_BRIDGE_REGISTRY.iter().find(|spec| {
        spec.component_tag == canonical.as_str() && spec.bind_property == bind_property
    })
}

/// 返回所有注册的 StateBridge spec（供 codegen 遍历）
pub fn all_specs() -> impl Iterator<Item = &'static StateBridgeSpec> {
    STATE_BRIDGE_REGISTRY.iter()
}

/// 查询组件是否有任意属性的 StateBridge 注册
pub fn lookup_state_bridge_for_tag(tag: &str) -> Option<&'static StateBridgeSpec> {
    let canonical = tags::canonical_tag(tag);
    STATE_BRIDGE_REGISTRY
        .iter()
        .find(|spec| spec.component_tag == canonical.as_str())
}

/// 收集 RML 中所有 StateBridge 双向绑定字段
///
/// 返回 `HashMap<bridge_key, Vec<field_name>>`，按 bridge_key 分组。
/// 每个 bridge_key 对应一种 State 类型（如 "slider" → SliderState 字段列表）。
pub fn collect_state_bridge_fields(root: &Node) -> HashMap<&'static str, Vec<String>> {
    let mut fields: HashMap<&'static str, Vec<String>> = HashMap::new();
    if let Node::Element(elem) = root {
        collect_state_bridge_fields_recursive(elem, &mut fields);
    }
    // 排序去重
    for field_list in fields.values_mut() {
        field_list.sort();
        field_list.dedup();
    }
    fields
}

fn collect_state_bridge_fields_recursive(
    elem: &Element,
    fields: &mut HashMap<&'static str, Vec<String>>,
) {
    let canonical = tags::canonical_tag(&elem.tag);
    if let Some(spec) = STATE_BRIDGE_REGISTRY
        .iter()
        .find(|s| s.component_tag == canonical.as_str())
    {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == spec.bind_property {
                    let (field, _) = crate::compiler::codegen::extract_field_converter(expr);
                    fields.entry(spec.bridge_key).or_default().push(field);
                }
            }
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_state_bridge_fields_recursive(child_elem, fields);
        }
    }
}
