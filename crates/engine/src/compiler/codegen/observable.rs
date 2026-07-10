//! Observable 字段版本管理 + 计算属性缓存 + InputState 双向同步代码生成
//!
//! 所有运行时状态由 `__rml_state: rml_ui::RmlState` 单一字段承载，
//! 替代旧的 7+ 类散落 `__rml_*` 字段。

use crate::compiler::CodegenCtx;
use super::binding::{gen_field_assign_expr, gen_field_value_expr};

/// 生成 oninput/onchange handler 调用代码（Phase B-3）
///
/// 在 `cx.subscribe` 回调的 `InputEvent::Change` 分支内、model 反向同步之后、`cx.notify()` 之前调用。
/// 无 handler 时返回空字符串。
fn gen_input_handler_call(field: &str, ctx: &CodegenCtx) -> String {
    let handlers = match ctx.model_input_handlers.get(field) {
        Some(h) => h,
        None => return String::new(),
    };
    let mut calls = Vec::new();
    if let Some(method) = &handlers.on_input {
        calls.push(format!(
            "let __rml_input_ev = rml::runtime::event_flow::convert::input(value.clone(), gpui::SharedString::default());\n                    this.{}(&__rml_input_ev, cx);",
            method
        ));
    }
    if let Some(method) = &handlers.on_change {
        calls.push(format!(
            "let __rml_change_ev = rml::runtime::event_flow::convert::change(value.clone());\n                    this.{}(&__rml_change_ev, cx);",
            method
        ));
    }
    calls.join("\n                    ")
}

/// 生成 observable 字段版本管理方法 + 计算属性依赖版本方法
///
/// 生成一个 `impl <View> { ... }` 块，包含四个方法：
/// - `__rml_bump_version(&mut self, field: &str)` — 委托 `RmlState::bump_version`
/// - `__rml_get_version(&self, field: &str) -> u64` — 委托 `RmlState::get_version`
/// - `__rml_computed_deps_version(&self, computed: &str) -> u64` — 求和依赖字段版本
/// - `__rml_changed_fields(&self) -> &'static [&'static str]` — 静态字段名列表
///
/// `bump_version` 取 `&mut self` 以支持 `HashMap::entry` 惰性插入；
/// 所有调用点（`#[command]` 包装、双向绑定反向同步）均持有 `&mut self`。
pub(super) fn gen_observable_impl(ctx: &CodegenCtx) -> String {
    let view_name = &ctx.view_struct_name;

    let mut deps_arms = String::new();
    for method in &ctx.computed_methods {
        let deps = ctx.computed_deps.get(method).cloned().unwrap_or_default();
        if deps.is_empty() {
            deps_arms.push_str(&format!(
                "            \"{}\" => 0,\n",
                method
            ));
        } else {
            let sum_expr = deps
                .iter()
                .map(|d| format!("self.__rml_get_version(\"{}\")", d))
                .collect::<Vec<_>>()
                .join(" + ");
            deps_arms.push_str(&format!(
                "            \"{}\" => {},\n",
                method, sum_expr
            ));
        }
    }

    // ObservableVec<T> 字段：版本路由到 self.field.version()，而非 __rml_state.get_version
    // 通过 field_types 中类型字符串包含 "ObservableVec" 识别
    let mut version_route_arms = String::new();
    for (field, ty) in &ctx.field_types {
        if ty.contains("ObservableVec") {
            version_route_arms.push_str(&format!(
                "        \"{}\" => self.{}.version(),\n",
                field, field
            ));
        }
    }

    let field_names: Vec<String> = ctx
        .observable_fields
        .iter()
        .map(|f| format!("\"{}\"", f))
        .collect();
    let changed_fields_array = if field_names.is_empty() {
        "&[]".to_string()
    } else {
        format!("&[{}]", field_names.join(", "))
    };

    format!(
        r#"#[allow(dead_code, non_snake_case)]
impl {view_name} {{
    /// 将指定字段的版本号 +1（由 #[command] 宏注入）
    fn __rml_bump_version(&mut self, field: &str) {{
        self.__rml_state.bump_version(field);
    }}

    /// 读取字段当前版本号
    ///
    /// ObservableVec<T> 字段路由到 `self.field.version()`（内部 AtomicU64），
    /// 其他字段走 `__rml_state.get_version(field)`。
    fn __rml_get_version(&self, field: &str) -> u64 {{
        match field {{
{version_route_arms}            _ => self.__rml_state.get_version(field),
        }}
    }}

    /// 返回计算属性依赖字段版本号之和，作为缓存键
    fn __rml_computed_deps_version(&self, computed: &str) -> u64 {{
        match computed {{
{deps_arms}            _ => 0,
        }}
    }}

    /// 返回所有 observable 字段名列表（供 #[command(no_notify)] 场景手动判断变更字段）
    fn __rml_changed_fields(&self) -> &'static [&'static str] {{
        {changed_fields_array}
    }}
}}"#,
        view_name = view_name,
        version_route_arms = version_route_arms,
        deps_arms = deps_arms,
        changed_fields_array = changed_fields_array,
    )
}

/// 生成 `#[computed]` 方法的缓存包装层
pub(super) fn gen_computed_wrappers(ctx: &CodegenCtx) -> String {
    if ctx.computed_methods.is_empty() {
        return String::new();
    }

    let view_name = &ctx.view_struct_name;
    let mut methods = String::new();
    for method in &ctx.computed_methods {
        let ret_type = ctx
            .computed_returns
            .get(method)
            .cloned()
            .unwrap_or_else(|| panic!("missing return type for #[computed] method `{}`", method));
        methods.push_str(&format!(
            r#"
    #[allow(dead_code, non_snake_case)]
    pub fn {method}(&self) -> {ret_type} {{
        let __v = self.__rml_computed_deps_version("{method}");
        self.__rml_state.computed_cache.get_or_compute::<{ret_type}>("{method}", __v, || self.__rml_computed_{method}())
    }}
"#,
            method = method,
            ret_type = ret_type,
        ));
    }

    format!(
        r#"#[allow(dead_code, non_snake_case, unused_variables)]
impl {view_name} {{{methods}}}
"#,
        view_name = view_name,
        methods = methods,
    )
}

/// 生成 InputState 惰性初始化 + 双向同步方法（Phase B-3：双向绑定）
pub(super) fn gen_input_state_impl(ctx: &CodegenCtx) -> String {
    let view_name = &ctx.view_struct_name;
    let input_fields: Vec<String> = ctx.model_fields.clone();

    let mut forward_arms = String::new();
    for field in &input_fields {
        let ty = ctx.field_types.get(field).cloned().unwrap_or_default();
        let converter = ctx.model_converters.get(field).map(|s| s.as_str());
        let expr = gen_field_value_expr(field, &ty, converter);
        forward_arms.push_str(&format!("            \"{}\" => {},\n", field, expr));
    }

    let mut reverse_arms = String::new();
    for field in &input_fields {
        let ty = ctx.field_types.get(field).cloned().unwrap_or_default();
        let validation = ctx.field_validations.get(field);
        let converter = ctx.model_converters.get(field).map(|s| s.as_str());
        let assign = gen_field_assign_expr(field, &ty, validation, converter);
        // Phase B-3：model 反向同步后追加 oninput/onchange handler 调用（cx.notify 之前）
        let handler_call = gen_input_handler_call(field, ctx);
        reverse_arms.push_str(&format!(
            "                \"{}\" => {{\n                    {}\n                    {}\n                }}\n",
            field, assign, handler_call
        ));
    }

    let mut out = String::new();
    out.push_str("#[allow(dead_code, non_snake_case, unused_variables)]\n");
    out.push_str(&format!("impl {} {{\n", view_name));
    out.push_str("    fn __rml_get_or_init_input_state(\n");
    out.push_str("        &mut self,\n");
    out.push_str("        field: &'static str,\n");
    out.push_str("        placeholder: Option<&'static str>,\n");
    out.push_str("        window: &mut gpui::Window,\n");
    out.push_str("        cx: &mut gpui::Context<Self>,\n");
    out.push_str("    ) -> gpui::Entity<rml_ui::InputState> {\n");
    out.push_str("        if !self.__rml_state.input_states.contains_key(field) {\n");
    out.push_str("            let entity = match placeholder {\n");
    out.push_str("                Some(p) => cx.new(|cx| rml_ui::InputState::new(window, cx).placeholder(p)),\n");
    out.push_str("                None => cx.new(|cx| rml_ui::InputState::new(window, cx)),\n");
    out.push_str("            };\n");
    out.push_str("            let initial_value: gpui::SharedString = match field {\n");
    out.push_str(&forward_arms);
    out.push_str("                _ => gpui::SharedString::default(),\n");
    out.push_str("            };\n");
    out.push_str("            entity.update(cx, |state, cx| state.set_value(initial_value, window, cx));\n");
    out.push_str("            cx.subscribe(&entity, move |this, input_entity, event, cx| {\n");
    out.push_str("                match event {\n");
    out.push_str("                    rml_ui::InputEvent::Change => {\n");
    out.push_str("                        let value = input_entity.read(cx).value();\n");
    out.push_str("                        match field {\n");
    out.push_str(&reverse_arms);
    out.push_str("                            _ => {}\n");
    out.push_str("                        }\n");
    out.push_str("                        let v = this.__rml_get_version(field);\n");
    out.push_str("                        this.__rml_state.input_state_versions.insert(field.to_string(), v);\n");
    out.push_str("                        cx.notify();\n");
    out.push_str("                    }\n");
    out.push_str("                    _ => {}\n");
    out.push_str("                }\n");
    out.push_str("            }).detach();\n");
    out.push_str("            let v = self.__rml_get_version(field);\n");
    out.push_str("            self.__rml_state.input_state_versions.insert(field.to_string(), v);\n");
    out.push_str("            self.__rml_state.input_states.insert(field.to_string(), entity);\n");
    out.push_str("        }\n");
    out.push_str("        let entity = self.__rml_state.input_states.get(field).unwrap().clone();\n");
    out.push_str("        let current_version = self.__rml_get_version(field);\n");
    out.push_str("        let last_synced = self.__rml_state.input_state_versions.get(field).copied().unwrap_or(0);\n");
    out.push_str("        if current_version != last_synced {\n");
    out.push_str("            let value: gpui::SharedString = match field {\n");
    out.push_str(&forward_arms);
    out.push_str("                _ => gpui::SharedString::default(),\n");
    out.push_str("            };\n");
    out.push_str("            entity.update(cx, |state, cx| state.set_value(value, window, cx));\n");
    out.push_str("            self.__rml_state.field_errors.insert(field.to_string(), None);\n");
    out.push_str("            self.__rml_state.input_state_versions.insert(field.to_string(), current_version);\n");
    out.push_str("        }\n");
    out.push_str("        entity\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

/// 生成 SliderState 惰性初始化 + 双向同步方法（C3：Slider StateBridge）
///
/// 生成 `__rml_get_or_init_slider_state(field, window, cx) -> Entity<SliderState>`：
/// - 首次调用：创建 SliderState，设置初始值，订阅 SliderEvent::Change 反向回写
/// - 后续调用：版本检查 → 正向同步（VM 字段变更 → SliderState.set_value）
///
/// 与 InputState 的差异：
/// - 值类型为 f32（经 `as f32` 转换），非 SharedString
/// - 事件为 `SliderEvent::Change(SliderValue)`，需从 SliderValue 提取 f32
/// - 无校验、无 on_input/on_change handler
pub(super) fn gen_slider_state_impl(ctx: &CodegenCtx) -> String {
    let view_name = &ctx.view_struct_name;
    let slider_fields: Vec<String> = ctx.slider_fields.clone();

    if slider_fields.is_empty() {
        return String::new();
    }

    // 正向同步臂：field → f32 值
    let mut forward_arms = String::new();
    for field in &slider_fields {
        let ty = ctx.field_types.get(field).map(|s| s.as_str()).unwrap_or("f32");
        let expr = numeric_forward_expr(field, ty);
        forward_arms.push_str(&format!("            \"{}\" => {},\n", field, expr));
    }

    // 反向同步臂：SliderValue → field 赋值
    let mut reverse_arms = String::new();
    for field in &slider_fields {
        let ty = ctx.field_types.get(field).map(|s| s.as_str()).unwrap_or("f32");
        let cast = numeric_cast_expr(ty);
        reverse_arms.push_str(&format!(
            "                \"{}\" => {{ this.{} = v{}; this.__rml_bump_version({:?}); }}\n",
            field, field, cast, field,
        ));
    }

    let mut out = String::new();
    out.push_str("#[allow(dead_code, non_snake_case, unused_variables)]\n");
    out.push_str(&format!("impl {} {{\n", view_name));
    out.push_str("    fn __rml_get_or_init_slider_state(\n");
    out.push_str("        &mut self,\n");
    out.push_str("        field: &'static str,\n");
    out.push_str("        window: &mut gpui::Window,\n");
    out.push_str("        cx: &mut gpui::Context<Self>,\n");
    out.push_str("    ) -> gpui::Entity<rml_ui::SliderState> {\n");
    out.push_str("        if !self.__rml_state.slider_states.contains_key(field) {\n");
    out.push_str("            let entity = cx.new(|_cx| rml_ui::SliderState::new());\n");
    out.push_str("            let initial_value: f32 = match field {\n");
    out.push_str(&forward_arms);
    out.push_str("                _ => 0.0,\n");
    out.push_str("            };\n");
    out.push_str("            entity.update(cx, |state, cx| state.set_value(rml_ui::SliderValue::Single(initial_value), window, cx));\n");
    out.push_str("            cx.subscribe(&entity, move |this, _state_entity, event, cx| {\n");
    out.push_str("                match event {\n");
    out.push_str("                    rml_ui::SliderEvent::Change(value) => {\n");
    out.push_str("                        let v = match value {\n");
    out.push_str("                            rml_ui::SliderValue::Single(v) => *v,\n");
    out.push_str("                            _ => return,\n");
    out.push_str("                        };\n");
    out.push_str("                        match field {\n");
    out.push_str(&reverse_arms);
    out.push_str("                            _ => {}\n");
    out.push_str("                        }\n");
    out.push_str("                        let __rml_ver = this.__rml_get_version(field);\n");
    out.push_str("                        this.__rml_state.slider_state_versions.insert(field.to_string(), __rml_ver);\n");
    out.push_str("                        cx.notify();\n");
    out.push_str("                    }\n");
    out.push_str("                    _ => {}\n");
    out.push_str("                }\n");
    out.push_str("            }).detach();\n");
    out.push_str("            let __rml_ver = self.__rml_get_version(field);\n");
    out.push_str("            self.__rml_state.slider_state_versions.insert(field.to_string(), __rml_ver);\n");
    out.push_str("            self.__rml_state.slider_states.insert(field.to_string(), entity);\n");
    out.push_str("        }\n");
    out.push_str("        let entity = self.__rml_state.slider_states.get(field).unwrap().clone();\n");
    out.push_str("        let current_version = self.__rml_get_version(field);\n");
    out.push_str("        let last_synced = self.__rml_state.slider_state_versions.get(field).copied().unwrap_or(0);\n");
    out.push_str("        if current_version != last_synced {\n");
    out.push_str("            let value: f32 = match field {\n");
    out.push_str(&forward_arms);
    out.push_str("                _ => 0.0,\n");
    out.push_str("            };\n");
    out.push_str("            entity.update(cx, |state, cx| state.set_value(rml_ui::SliderValue::Single(value), window, cx));\n");
    out.push_str("            self.__rml_state.slider_state_versions.insert(field.to_string(), current_version);\n");
    out.push_str("        }\n");
    out.push_str("        entity\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

/// 生成数值字段的正向同步表达式：`self.field as f32`
fn numeric_forward_expr(field: &str, ty: &str) -> String {
    match ty {
        "f32" | "f64" => format!("self.{} as f32", field),
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" => format!("self.{} as f32", field),
        _ => format!("self.{} as f32", field),
    }
}

/// 生成数值类型转换后缀：` as i32` / ` as f32` / 空（f32 无需转换）
fn numeric_cast_expr(ty: &str) -> String {
    match ty {
        "f32" => String::new(),
        _ => format!(" as {}", ty),
    }
}
