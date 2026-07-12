//! 扩展组件 setter 工具集（gpui-component 路由）
//!
//! 本模块不含组件生成入口；每个扩展组件独占一个 translator 文件
//!（位于 `compiler/translator/component/`），通过本模块的 setter 函数
//! 复用静态属性 / 绑定属性 / 事件属性的统一映射逻辑。

use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法映射
///
/// - `label="..."` → `.label("...")`
/// - `placeholder="..."` → `.placeholder("...")`（NumberInput/Select/Combobox/DatePicker 等组件支持；Input/TextInput 由 InputTranslator 注入 state_ctor）
/// - `primary=""`/`danger=""` → `.primary()` / `.danger()`（Button 专用布尔属性）
/// - `disabled="true"` → `.disabled(true)`
/// - `selected`/`compact`/`loading` → 对应方法
/// - `size` → Sizable 尺寸方法（`size="small"` → `.with_size(rml_ui::Size::Small)`）
/// - `font_bold`/`font_semibold` 等 → StyledExt 字体权重
/// - `h_flex`/`v_flex` → StyledExt 布局快捷方法
pub fn component_static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    // 组件专用 static setter 委托（Avatar/AvatarGroup 的 src/name/placeholder/limit/ellipsis）
    if let Some(s) = super::components::avatar::static_setter(name, value, tag) {
        return Some(s);
    }
    // Badge 的 count/max/dot/icon（Number/Dot/Icon 三种 variant）
    if let Some(s) = super::components::badge::static_setter(name, value, tag) {
        return Some(s);
    }
    // Card 的 title/bordered/borderless/hoverable
    if let Some(s) = super::components::card::static_setter(name, value, tag) {
        return Some(s);
    }
    // Table 的 bordered/borderless/stripe + Column 的 width/align
    if let Some(s) = super::components::table::setters::static_setter(name, value, tag) {
        return Some(s);
    }
    // DescriptionList 的 vertical/horizontal/bordered/columns/label_width + DescriptionItem 的 value/span
    if let Some(s) = super::components::description_list::setters::static_setter(name, value, tag) {
        return Some(s);
    }
    // Tooltip 通用属性（Button/Checkbox/Clipboard/DropdownButton/Toggle/Radio/Switch 的 .tooltip()）
    if let Some(s) = super::tooltip::static_setter(name, value, tag) {
        return Some(s);
    }
    // Rating: color="red" → .color(cx.theme().red)，设置星标激活色
    // 必须在 apply_style_attr 之前处理，避免被 CSS color 属性拦截生成 .text_color()
    if tag == "Rating" && name == "color" {
        return Some(format!(".color(cx.theme().{})", value));
    }
    // OtpInput: groups="2" → .groups(2usize)
    // length/masked/default_value 由 OtpInputTranslator 注入 state_ctor，不生成 setter
    if let Some(s) = super::components::otp_input::setters::static_setter(name, value, tag) {
        return Some(s);
    }
    // 归一化样式属性：对所有扩展组件生效（gpui-component 实现 Styled trait）
    // 复用 css::mapper 单一映射源，避免双轨制
    if let Some(s) = crate::compiler::codegen::style_attr::apply_style_attr(name, value) {
        return Some(s);
    }
    match name {
        "label" => Some(format!(".label({:?})", value)),
        "placeholder" => Some(format!(".placeholder({:?})", value)),
        // ── Phase 1 组件专用 static setter ──
        // Skeleton: secondary="" → .secondary()
        "secondary" if tag == "Skeleton" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".secondary()".to_string())
            } else {
                None
            }
        }
        // Link: href="url" → .href("url")
        "href" if tag == "Link" => Some(format!(".href({:?})", value)),
        // Spinner / ColorPicker: icon="Loader" → .icon(rml_ui::Icon::new(rml_ui::IconName::Loader))
        "icon" if tag == "Spinner" || tag == "ColorPicker" => {
            Some(format!(".icon(rml_ui::Icon::new(rml_ui::IconName::{}))", value))
        }
        // Collapsible: open="true" → .open(true)
        "open" if tag == "Collapsible" => Some(format!(".open({})", parse_bool(value))),
        // GroupBox: title="..." → .title("...")
        "title" if tag == "GroupBox" => Some(format!(".title({:?})", value)),
        // GroupBox variant 布尔属性: fill → .fill(), outline → .outline(), normal → .normal()
        // 各 variant 为独立布尔属性，可与其他属性自由组合
        "fill" | "normal" | "outline" if tag == "GroupBox" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        }
        // Button variant 布尔属性: primary → .primary(), ghost → .ghost() 等
        // 各 variant 为独立布尔属性，可与其他属性自由组合（如 <Button primary compact />）
        // secondary 为默认值（ButtonVariant::Secondary 为 #[default]）
        "primary" | "secondary" | "danger" | "success" | "warning" | "info" | "ghost"
        | "link" | "text" if tag == "Button" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".{}()", name))
            } else {
                None
            }
        },
        // Pagination 数值属性
        "current_page" | "total_pages" | "visible_pages" if tag == "Pagination" => {
            Some(format!(".{}({})", name, value))
        }
        // RadioGroup: selected_index="2" → .selected_index(Some(2usize))
        "selected_index" if tag == "RadioGroup" => {
            Some(format!(".selected_index(Some({}usize))", value))
        }
        // Radio tab_index/tab_stop
        "tab_index" if tag == "Radio" => Some(format!(".tab_index({})", value)),
        "tab_stop" if tag == "Radio" => Some(format!(".tab_stop({})", parse_bool(value))),
        // Rating: value="3" → .value(3usize), max="5" → .max(5usize)
        "value" if tag == "Rating" => Some(format!(".value({}usize)", value)),
        "max" if tag == "Rating" => Some(format!(".max({}usize)", value)),
        // NumberInput: appearance 默认 true，仅在 false 时显式设置
        "appearance" if tag == "NumberInput" => {
            if value.eq_ignore_ascii_case("false") {
                Some(".appearance(false)".to_string())
            } else {
                Some(String::new())
            }
        }
        // DatePicker: appearance 默认 true，仅在 false 时显式设置（同 NumberInput）
        "appearance" if tag == "DatePicker" => {
            if value.eq_ignore_ascii_case("false") {
                Some(".appearance(false)".to_string())
            } else {
                Some(String::new())
            }
        }
        // DatePicker: cleanable 默认 false，仅在 true（或空属性）时显式设置
        "cleanable" if tag == "DatePicker" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".cleanable(true)".to_string())
            } else {
                Some(String::new())
            }
        }
        // DatePicker: number_of_months="2" → .number_of_months(2usize)
        "number_of_months" if tag == "DatePicker" => {
            Some(format!(".number_of_months({}usize)", value))
        }
        // Select/Combobox: cleanable 默认 false，仅在 true（或空属性）时显式设置
        "cleanable" if tag == "Select" || tag == "Combobox" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".cleanable(true)".to_string())
            } else {
                Some(String::new())
            }
        }
        // Select/Combobox: appearance 默认 true，仅在 false 时显式设置（同 NumberInput/DatePicker）
        "appearance" if tag == "Select" || tag == "Combobox" => {
            if value.eq_ignore_ascii_case("false") {
                Some(".appearance(false)".to_string())
            } else {
                Some(String::new())
            }
        }
        // Select/Combobox: menu_width="200px" → .menu_width(gpui::px(200.0))
        // 仅支持 px 数值，复杂 Length 类型用户可在 code-behind 中命令式设置
        "menu_width" if tag == "Select" || tag == "Combobox" => {
            if let Some(px_val) = parse_px(value) {
                Some(format!(".menu_width(gpui::px({}.))", px_val))
            } else {
                None
            }
        }
        // Select/Combobox: menu_max_h="300px" → .menu_max_h(gpui::px(300.0))
        "menu_max_h" if tag == "Select" || tag == "Combobox" => {
            if let Some(px_val) = parse_px(value) {
                Some(format!(".menu_max_h(gpui::px({}.))", px_val))
            } else {
                None
            }
        }
        // Combobox: search_placeholder="搜索..." → .search_placeholder("...")
        "search_placeholder" if tag == "Combobox" => {
            Some(format!(".search_placeholder({:?})", value))
        }
        // Sizable 尺寸：size="xsmall" / size="small" / size="large"
        // medium/default 为组件原生默认（Size::Medium 由 #[default] 指定），
        // 遵循原生写法不生成 .with_size() 调用，避免冗余加工。
        // 不写 size 属性 = size="medium" = size="default" = 无调用
        // 返回 Some("") 而非 None，避免 check_missing_mapping 误报"无映射"。
        "size" => {
            let size = match value {
                "xsmall" => "rml_ui::Size::XSmall",
                "small" => "rml_ui::Size::Small",
                "large" => "rml_ui::Size::Large",
                // medium/default 是原生默认，返回空字符串（no-op）
                "medium" | "default" => return Some(String::new()),
                _ => return None,
            };
            Some(format!(".with_size({})", size))
        }
        // compact 是无参方法（Button/ButtonGroup.compact()）
        "compact" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(format!(".compact()"))
            } else {
                None
            }
        }
        // loading 需要 bool 参数（Button/Progress/ProgressCircle.loading(bool)）
        // HTML 布尔属性语义：空值或 "true" 为 true，其他为 false
        "loading" => {
            let b = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                "true"
            } else {
                "false"
            };
            Some(format!(".loading({})", b))
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
        "disabled" => Some(format!(".disabled({})", parse_bool(value))),
        "selected" => Some(format!(".selected({})", parse_bool(value))),
        // style 属性：内联 CSS 字符串，对所有组件生效（gpui-component 实现 Styled trait）
        "style" => {
            let code = crate::compiler::codegen::attribute::apply_inline_style(value);
            if code.is_empty() { None } else { Some(code) }
        }
        // class/id 由 apply_css_styles 处理，src/type/value 由专属逻辑处理
        "class" | "id" | "src" | "type" | "value" => None,
        // ref 属性已在构造器中处理（生成稳定 ID），此处跳过
        "ref" => None,
        _ => None,
    }
}

/// 绑定表达式 → Rust 代码（供 setter 复用）
pub fn component_bind_rust_expr(
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    if let Some(code) = crate::compiler::codegen::try_gen_i18n_call(expr_str, loop_vars, computed) {
        return code;
    }
    let prefix = expr::current_self_alias().unwrap_or("self");
    match expr::parse(expr_str) {
        Ok(expr::Expr::Field(name)) if computed.contains(&name.as_str()) => {
            if loop_vars.iter().any(|v| *v == name) {
                format!("{}()", name)
            } else {
                format!("{}.{}()", prefix, name)
            }
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            let trimmed = expr_str.trim();
            if loop_vars.contains(&trimmed) {
                trimmed.to_string()
            } else if computed.contains(&trimmed) {
                format!("{}.{}()", prefix, trimmed)
            } else {
                format!("{}.{}", prefix, trimmed)
            }
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
pub fn component_bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    // 组件专用 bind setter 委托（menu/MenuBar/status-bar 的 items 属性）
    if let Some(s) = super::components::menu::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // Avatar/AvatarGroup 的 src/name/placeholder/limit 属性
    if let Some(s) = super::components::avatar::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // Badge 的 count/max 绑定（Number variant 动态计数）
    if let Some(s) = super::components::badge::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // Card 的 title/extra/cover/footer/bordered/hoverable 属性
    if let Some(s) = super::components::card::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // Table 的 columns/rows/delegate/bordered/stripe + Column 的 width/align
    if let Some(s) = super::components::table::setters::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // DescriptionList 的 bordered/columns/label_width + DescriptionItem 的 value/span
    if let Some(s) =
        super::components::description_list::setters::bind_setter(name, expr_str, loop_vars, computed, tag)
    {
        return Some(s);
    }
    // OtpInput: groups={count} → .groups(self.count)
    if let Some(s) = super::components::otp_input::setters::bind_setter(
        name,
        expr_str,
        loop_vars,
        computed,
        tag,
    ) {
        return Some(s);
    }
    // Breadcrumb 的 items 属性 → .items(Vec<BreadcrumbItem>.clone())
    if crate::tags::canonical_tag(tag) == "Breadcrumb" && name == "items" {
        let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
        return Some(format!(".items({}.clone())", rust_expr));
    }
    // Tooltip 通用属性绑定（Button/Checkbox/Clipboard/DropdownButton/Toggle/Radio/Switch）
    if super::tooltip::supports_tooltip(tag) && name == "tooltip" {
        let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
        if let Some(s) = super::tooltip::bind_setter(name, &rust_expr, tag) {
            return Some(s);
        }
    }

    match name {
        // content={expr}：通过 IntoContent trait 统一转换为 child
        // 支持 IntoElement（String/SharedString/AnyElement）、ToString（i32/bool 等）、IVisual（&dyn IVisual 等）
        // 表达式经 component_bind_rust_expr 处理：slot 上下文中 self. 替换为 __rml_self_ref.，
        // _window/cx 作为 scope_vars 识别为 render 方法作用域变量（不加 self. 前缀）
        "content" => {
            let mut scope_vars: Vec<&str> = loop_vars.iter().copied().collect();
            for v in ["_window", "cx"] {
                if !scope_vars.contains(&v) {
                    scope_vars.push(v);
                }
            }
            let code = component_bind_rust_expr(expr_str, &scope_vars, computed);
            let final_code = if crate::compiler::codegen::attribute::needs_borrow_for_content(
                &code,
                &scope_vars,
            ) {
                format!("&{}", code)
            } else {
                code
            };
            Some(format!(".child(rml_core::content::into_content({}, _window, cx))", final_code))
        }
        "value" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".value({}.clone())", rust_expr))
        }
        "disabled" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".disabled({})", rust_expr))
        }
        "selected" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".selected({})", rust_expr))
        }
        "checked" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            // Switch 有 .checked() 但无 Selectable trait；其他组件（Checkbox）通过 .selected() 设置
            if tag == "Switch" {
                Some(format!(".checked({})", rust_expr))
            } else {
                Some(format!(".selected({})", rust_expr))
            }
        }
        "label" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".label({}.clone())", rust_expr))
        }
        // Sizable 尺寸绑定：size={size_value} → .with_size(self.size_value)
        "size" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".with_size({})", rust_expr))
        }
        // loading={bool_expr} → .loading(self.is_loading)
        "loading" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".loading({})", rust_expr))
        }
        // ── Phase 1 组件专用 bind setter ──
        // Link: href={url} → .href(self.url.clone())
        "href" if tag == "Link" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".href({}.clone())", rust_expr))
        }
        // Collapsible: open={is_open} → .open(self.is_open)
        "open" if tag == "Collapsible" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".open({})", rust_expr))
        }
        // GroupBox: title={title_text} → .title(self.title_text.clone())
        "title" if tag == "GroupBox" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".title({}.clone())", rust_expr))
        }
        // Pagination: current_page/total_pages/visible_pages={usize_field}
        "current_page" | "total_pages" | "visible_pages" if tag == "Pagination" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        // RadioGroup: selected_index={idx} → .selected_index(Some(self.idx))
        // 字段类型约定为 usize（API 接受 Option<usize>，框架自动包 Some）
        "selected_index" if tag == "RadioGroup" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".selected_index(Some({}))", rust_expr))
        }
        // 动态高度/宽度：height={px} → .h(gpui::px(<expr>))（Styled 组件通用）
        "height" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".h(gpui::px({}))", rust_expr))
        }
        "width" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".w(gpui::px({}))", rust_expr))
        }
        _ => None,
    }
}

/// 处理「已注册但无映射」的属性：strict 模式下报 error，否则输出 warning。
///
/// 由各类别 component translator 在 setter 返回 None 后调用。若属性未在 props_registry 登记，
/// 视为合法跳过（validator 已校验未知属性，此处不重复报错）。
pub(crate) fn check_missing_mapping(
    ctx: &CodegenCtx,
    tag: &str,
    name: &str,
    kind: &str,
) -> Result<(), CodegenError> {
    if !crate::compiler::props_registry::is_prop_registered(tag, name) {
        return Ok(());
    }
    let msg = format!(
        "<{}> {} property `{}` is registered in props_registry but has no mapping in component_{}_setter; \
         property will be silently dropped. Add a match arm in crates/engine/src/compiler/setters.rs.",
        tag, kind, name, kind
    );
    if ctx.strict {
        Err(CodegenError {
            message: msg,
            span: None,
        })
    } else {
        eprintln!("[rml warning] {}", msg);
        Ok(())
    }
}

/// 事件属性 → 组件事件方法映射（dispatcher）
///
/// 与原生 div 的事件不同，gpui-component 组件的 on_* 方法接受 3 参闭包
/// `Fn(&ClickEvent, &mut Window, &mut App)`。通过 `cx.listener` 包装后可访问 `this`。
///
/// 组件专用事件（Input/TextInput 的 onchange、Tree 的 on_activate、Accordion 的 on_toggle_click）
/// 已迁移到各自的组件模块，本函数仅处理通用 `on_click` 事件并作为回退入口。
///
/// 声明式统一为 `on-click`（kebab-case），normalize 后内部 match `on_click`（snake_case）。
/// 旧的单词条形式 `onclick` 已移除（无兼容性设计）。
pub fn component_event_setter(name: &str, handler: &EventHandler, tag: &str) -> Option<String> {
    // Input 事件（on_change/on_enter/on_focus/on_blur）通过 EventEmitter + cx.subscribe 模式处理，
    // 不走 setter 链路——在 stateful translator 中统一生成 block 表达式包装构造器。
    // 此处返回 None 让属性循环跳过 setter 生成，事件由 stateful translator 收集后统一 subscribe。
    if super::components::input::is_input_event(name, tag) {
        return None;
    }

    // Breadcrumb 专用：on_select 同级选择回调
    // `<Breadcrumb on-select={on_breadcrumb_select} />` →
    // `.on_select_rc(Rc::new({ let weak = cx.weak_entity(); move |level, index, w, app| { ... } }))`
    // 用户方法签名约定：`fn on_breadcrumb_select(&mut self, level: usize, index: usize, cx: &mut Context<Self>)`
    if name == "on_select" && crate::tags::canonical_tag(tag) == "Breadcrumb" {
        let method = match handler {
            EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
            EventHandler::WithArgs(m, _) => m,
            EventHandler::ClosureField(_) => "",
        };
        return Some(format!(
            ".on_select_rc(std::rc::Rc::new({{\n                    \
             let weak = cx.weak_entity();\n                    \
             move |level: usize, index: usize, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
             if let Some(entity) = weak.upgrade() {{\n                            \
             entity.update(app, |this, cx| {{ this.{}(level, index, cx); }});\n                        \
             }}\n                    \
             }}\n                }}))",
            method
        ));
    }

    // Table 专用：on_cell_edit 单元格编辑提交回调
    // `<Table on-cell-edit={handle_edit} />` →
    // `.on_cell_edit(Rc::new({ let weak = cx.weak_entity(); move |row, col, new_value, app| { ... } }))`
    // 用户方法签名约定：`fn handle_edit(&mut self, row: usize, col: usize, new_value: SharedString, cx: &mut Context<Self>)`
    if name == "on_cell_edit" && crate::tags::canonical_tag(tag) == "Table" {
        let method = match handler {
            EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
            EventHandler::WithArgs(m, _) => m,
            EventHandler::ClosureField(_) => "",
        };
        return Some(format!(
            ".on_cell_edit(std::rc::Rc::new({{\n                    \
             let weak = cx.weak_entity();\n                    \
             move |row: usize, col: usize, new_value: gpui::SharedString, app: &mut gpui::App| {{\n                        \
             if let Some(entity) = weak.upgrade() {{\n                            \
             entity.update(app, |this, cx| {{ this.{}(row, col, new_value, cx); }});\n                        \
             }}\n                    \
             }}\n                }}))",
            method
        ));
    }

    match name {
        "on_click" => {
            let method = match handler {
                EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
                EventHandler::WithArgs(m, _) => m,
                EventHandler::ClosureField(_) => "",
            };

            // Pagination 的 on_click 闭包参数是新的页码（&usize），而非 ClickEvent。
            // 用户方法签名约定：`fn on_page_change(&mut self, page: &usize, cx: &mut Context<Self>)`
            if tag == "Pagination" {
                return match handler {
                    EventHandler::ClosureField(_) => None,
                    EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                        ".on_click(cx.listener(move |this, page: &usize, _window, cx| {{\n                    \
                         this.{}(page, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
                        ".on_click(cx.listener(move |this, page: &usize, _window, cx| {{\n                    \
                         this.{}(page, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) => {
                        let arg = &args[0];
                        Some(format!(
                            ".on_click(cx.listener(move |this, page: &usize, _window, cx| {{\n                    \
                             let p0 = {}.clone();\n                    \
                             this.{}(p0, page, cx);\n                }}))",
                            arg, method
                        ))
                    }
                };
            }

            // RadioGroup 的 on_click 闭包参数是新的选中索引（&usize），而非 ClickEvent。
            // 用户方法签名约定：`fn on_radio_change(&mut self, idx: &usize, cx: &mut Context<Self>)`
            if tag == "RadioGroup" {
                return match handler {
                    EventHandler::ClosureField(_) => None,
                    EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                        ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
                         this.{}(idx, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
                        ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
                         this.{}(idx, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) => {
                        let arg = &args[0];
                        Some(format!(
                            ".on_click(cx.listener(move |this, idx: &usize, _window, cx| {{\n                    \
                             let p0 = {}.clone();\n                    \
                             this.{}(p0, idx, cx);\n                }}))",
                            arg, method
                        ))
                    }
                };
            }

            // Rating 的 on_click 闭包参数是新的评分值（&usize），而非 ClickEvent。
            // 用户方法签名约定：`fn on_rating_change(&mut self, value: &usize, cx: &mut Context<Self>)`
            if tag == "Rating" {
                return match handler {
                    EventHandler::ClosureField(_) => None,
                    EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                        ".on_click(cx.listener(move |this, value: &usize, _window, cx| {{\n                    \
                         this.{}(value, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
                        ".on_click(cx.listener(move |this, value: &usize, _window, cx| {{\n                    \
                         this.{}(value, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) => {
                        let arg = &args[0];
                        Some(format!(
                            ".on_click(cx.listener(move |this, value: &usize, _window, cx| {{\n                    \
                             let p0 = {}.clone();\n                    \
                             this.{}(p0, value, cx);\n                }}))",
                            arg, method
                        ))
                    }
                };
            }

            // Checkbox / Switch / Radio 的 on_click 闭包参数是新的 checked 状态（&bool），
            // 而非 ClickEvent。
            let is_bool_event = matches!(tag, "Checkbox" | "Switch" | "Radio");

            if is_bool_event {
                match handler {
                    EventHandler::ClosureField(_) => None,
                    EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                        ".on_click(cx.listener(move |this, checked: &bool, _window, cx| {{\n                    \
                         this.{}(checked, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
                        ".on_click(cx.listener(move |this, checked: &bool, _window, cx| {{\n                    \
                         this.{}(checked, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) => {
                        let arg = &args[0];
                        Some(format!(
                            ".on_click(cx.listener(move |this, checked: &bool, _window, cx| {{\n                    \
                             let p0 = {}.clone();\n                    \
                             this.{}(p0, checked, cx);\n                }}))",
                            arg, method
                        ))
                    }
                }
            } else {
                match handler {
                    EventHandler::ClosureField(_) => None,
                    EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
                        ".on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                         let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                         this.{}(&rml_ev, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
                        ".on_click(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                         let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                         this.{}(&rml_ev, cx);\n                }}))",
                        method
                    )),
                    EventHandler::WithArgs(_, args) => {
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

/// 解析 RML 属性值中的像素数值（如 "200px" / "200" → 200.0）
fn parse_px(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    let stripped = trimmed.strip_suffix("px").unwrap_or(trimmed).trim();
    stripped.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_bind_setter_wraps_with_into_content() {
        let code = component_bind_setter("content", "self.title", &[], &[], "Button").unwrap();
        // 简单字段访问自动添加 & 前缀
        assert_eq!(
            code,
            ".child(rml_core::content::into_content(&self.title, _window, cx))"
        );
    }

    #[test]
    fn content_bind_setter_preserves_complex_expr() {
        let code = component_bind_setter("content", "self.counter + 1", &[], &[], "Button").unwrap();
        assert!(code.contains("into_content("));
        assert!(code.contains("self.counter + 1"));
        assert!(!code.contains("&self.counter"));
    }

    #[test]
    fn content_bind_setter_works_for_any_component() {
        // content 绑定对所有组件标签生效（非 Button 专用）
        let code = component_bind_setter("content", "self.name", &[], &[], "Tag").unwrap();
        assert!(code.contains("into_content(&self.name"));
    }

    #[test]
    fn content_bind_setter_no_borrow_for_method_call() {
        // 方法调用不加 & 前缀（返回 owned 值）
        let code = component_bind_setter("content", "self.make_badge()", &[], &[], "Button").unwrap();
        assert!(code.contains("into_content(self.make_badge()"));
        assert!(!code.contains("&self.make_badge"));
    }

    #[test]
    fn content_bind_setter_no_borrow_for_loop_var() {
        // 循环变量不加 & 前缀
        let code = component_bind_setter("content", "item", &["item"], &[], "Button").unwrap();
        assert!(code.contains("into_content(item,"));
        assert!(!code.contains("&item"));
    }
}
