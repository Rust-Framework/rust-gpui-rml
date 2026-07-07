//! 单个 `<Tab>` 子节点 codegen —— 生成 `rml_ui::TabItem::new()...` 表达式。
//!
//! ## 设计
//!
//! `<tab>` 标签底层编译为 `rml_ui::TabItem`（WPF TabItem 模式：title + body）。
//! `TabItem` 同时承载 title (header) 与 body (选中时渲染的内容)，由 `TabBar::render`
//! 自动垂直堆叠 header + body。
//!
//! ## 子节点分流规则
//!
//! - `label` 属性 / 纯文本子节点 → `.title("...")`（互斥，属性优先）
//! - `icon` 属性 → `.title_icon(IconName::...)`
//! - `<template slot="header">` 子节点 → `.title_child(<element>)`（可多次，header 自定义插槽）
//! - 其余 element 子节点 → `.body(closure)`（body 内容，仅选中 tab 渲染）
//!
//! ## each 指令
//!
//! `each={tab in tabs}` 生成 `self.tabs.iter().map(|tab| { let tab = tab.clone(); ... })`，
//! loop 变量 clone 为 owned 以满足 body 闭包 `'static` 约束。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, Node};

/// 扫描 body 代码中的 self/cx 引用，在闭包外预提取为 owned 变量，
/// 闭包内替换为该变量，使 body 闭包满足 `Send + Sync + 'static` 约束
/// （`TabItem::body` 闭包签名不接收 `Context`，无法访问 self/cx）。
///
/// ## 替换规则（按优先级）
///
/// 1. **`self.__rml_state.get_or_init_ref(...)`**（ref-based 组件 lazy init）
///    → 闭包外 `let __entity_N = self.__rml_state.get_or_init_ref(...);`
///    → 闭包内用 `__entity_N` 替代原表达式
///    （Entity<T> 是 Send + Sync + Clone，可被闭包 move 捕获）
///
/// 2. **`self.xxx()`**（computed 方法调用）
///    → 闭包外 `let xxx = self.xxx();`
///    → 闭包内用 `xxx` 替代
///
/// 3. **`self.xxx`**（字段访问）
///    → 闭包外 `let xxx = self.xxx.clone();`
///    → 闭包内用 `xxx` 替代
///
/// 4. **`cx.theme()`**（Context 上的 theme 访问）
///    → 闭包内替换为 `app.theme()`（body 闭包有 `app: &mut App`，`ThemeExt for App`）
///
/// ## 安全性
///
/// - 仅替换 `self.` 开头的简单标识符访问，不影响 `self.tabs.iter()` 等链式调用
/// - `get_or_init_ref` 用括号匹配找到完整调用（处理嵌套闭包参数）
/// - `_window` 在 render 作用域和 body 闭包都可用，无需替换
fn extract_body_deps(
    body_code: &str,
    computed_methods: &[String],
    loop_vars: &[String],
) -> (String, String) {
    use std::collections::BTreeMap;

    let computed: std::collections::HashSet<&str> =
        computed_methods.iter().map(|s| s.as_str()).collect();

    let mut prelude = String::new();
    let mut working = body_code.to_string();

    // ── 步骤 0：提取 block 表达式内的 let 语句（如 let __code = ...;）到 prelude ──
    // CodeEditor 生成 { let __code = self.xxx(); Input::new(...)... }，
    // get_or_init_ref 的 ctor 闭包引用 __code，必须在 prelude 中定义。
    // 匹配 let <ident> = <expr>; 语句（非 let mut 等其他形式）
    let let_re = regex::Regex::new(r"let\s+(__\w+)\s*=\s*([^;]+);\s*").unwrap();
    for cap in let_re.captures_iter(&working.clone()) {
        let var_name = cap.get(1).unwrap().as_str();
        let expr = cap.get(2).unwrap().as_str().trim();
        prelude.push_str(&format!("let {} = {};\n            ", var_name, expr));
    }
    // 从 body 中移除这些 let 语句
    working = let_re.replace_all(&working, "").to_string();

    // ── 步骤 1：提取 self.__rml_state.get_or_init_ref(...) 到 prelude ──
    // 用括号匹配找到完整调用（参数含嵌套闭包 move |w, c| ...）
    let mut entity_counter: usize = 0;
    loop {
        let needle = "self.__rml_state.get_or_init_ref(";
        let start = match working.find(needle) {
            Some(i) => i,
            None => break,
        };
        // 从 needle 末尾的 '(' 开始括号匹配
        let paren_open = start + needle.len() - 1;
        let paren_close = match find_matching_paren(&working, paren_open) {
            Some(i) => i,
            None => break,
        };
        // 完整调用：working[start..=paren_close]
        let full_call = &working[start..=paren_close];
        let var_name = format!("__rml_entity_{}", entity_counter);
        entity_counter += 1;
        prelude.push_str(&format!("let {} = {};\n            ", var_name, full_call));
        // 替换 body 中的完整调用为变量名
        working.replace_range(start..=paren_close, &var_name);
    }

    // ── 步骤 2 & 3：提取 self.xxx / self.xxx() 到 prelude ──
    // 匹配 self.xxx 或 self.xxx() 的简单字段访问（不匹配链式调用如 self.tabs.iter()）
    let re = regex::Regex::new(r"self\.([a-z_][a-z0-9_]*)(\(\))?").unwrap();

    // 收集所有匹配项，去重（同名字段只 clone 一次）
    let mut refs: BTreeMap<String, bool> = BTreeMap::new(); // name -> is_computed_call
    for cap in re.captures_iter(&working) {
        let name = cap.get(1).unwrap().as_str().to_string();
        // 跳过 loop_vars（each 模式下已 clone）
        if loop_vars.iter().any(|v| v == &name) {
            continue;
        }
        // 跳过 __rml_state（已在步骤 1 处理）
        if name == "__rml_state" {
            continue;
        }
        let is_call = cap.get(2).is_some();
        let is_computed = is_call && computed.contains(name.as_str());
        refs.insert(name, is_computed);
    }

    // 生成 prelude：let xxx = self.xxx(); 或 let xxx = self.xxx.clone();
    for (name, is_computed) in &refs {
        if *is_computed {
            prelude.push_str(&format!("let {} = self.{}();\n            ", name, name));
        } else {
            prelude.push_str(&format!("let {} = self.{}.clone();\n            ", name, name));
        }
    }

    // 替换 body 代码：self.xxx() → xxx，self.xxx → xxx
    working = re.replace_all(&working, |cap: &regex::Captures| {
        let name = cap.get(1).unwrap().as_str();
        if loop_vars.iter().any(|v| v == name) {
            cap.get(0).unwrap().as_str().to_string()
        } else if name == "__rml_state" {
            // __rml_state 引用应已在步骤 1 消除；保留原样以防遗漏
            cap.get(0).unwrap().as_str().to_string()
        } else {
            name.to_string()
        }
    }).to_string();

    // ── 步骤 4：cx.theme() → app.theme() ──
    // body 闭包签名 move |_window, app|，ThemeExt for App 使 app.theme() 可用
    working = working.replace("cx.theme()", "app.theme()");
    // cx.mono_font_family / cx.mono_font_size 等也替换为 app（ThemeExt 已覆盖）
    // 但仅替换与 theme 相关的 cx 调用，避免误伤 cx.subscribe 等
    working = working.replace("cx.theme().", "app.theme().");

    (prelude, working)
}

/// 从 `working[open]`（必须是 `(`）开始，找到匹配的右括号 `)`。
///
/// 处理嵌套括号和字符串字面量（简单版本：不处理转义和注释）。
fn find_matching_paren(working: &str, open: usize) -> Option<usize> {
    let bytes = working.as_bytes();
    if bytes.get(open) != Some(&b'(') {
        return None;
    }
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut string_char = b'\0';
    let mut i = open;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if c == string_char && bytes.get(i.wrapping_sub(1)) != Some(&b'\\') {
                in_string = false;
            }
        } else {
            match c {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                b'"' | b'\'' => {
                    in_string = true;
                    string_char = c;
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// 为 `<Tab>` 子节点生成 `rml_ui::TabItem::new()...` 表达式
///
/// 返回 `(代码, 是否迭代器)`：
/// - 无 `each` 指令：`(构造表达式, false)` → 父用 `.child(...)`
/// - 有 `each` 指令：`(iter().map(...), true)` → 父用 `.children(...)`
pub fn gen_tab_child(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<(String, bool), CodegenError> {
    let each_clause = elem.directives.iter().find_map(|d| match d {
        Directive::Each { clause: c, .. } => Some(c.clone()),
        _ => None,
    });

    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    if let Some(clause) = &each_clause {
        child_loop_vars.push(clause.item.clone());
        if let Some(idx) = &clause.index {
            child_loop_vars.push(idx.clone());
        }
    }

    let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::TabItem::new()");

    // 静态/绑定/事件属性 → 调 tab_bar 专用 setter（已映射 Tab→TabItem，如 label→title）
    let mut title_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "Tab") {
                    code.push_str(&s);
                    if name == "label" {
                        title_set_by_attr = true;
                    }
                } else if let Some(s) = super::super::component::component_static_setter(
                    name, value, "Tab",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Tab")
                {
                    code.push_str(&s);
                    if name == "label" {
                        title_set_by_attr = true;
                    }
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "Tab",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, "Tab") {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_event_setter(name, handler, "Tab")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 子节点分流：title 文本 / header slot / body
    // 收集 slot="header" 子节点和 body 子节点
    let mut header_slot_children: Vec<&Node> = Vec::new();
    let mut body_children: Vec<&Node> = Vec::new();

    for child in &elem.children {
        match child {
            Node::Text(_) => {
                // 文本子节点在下面单独处理为 .title(...)
            }
            Node::Element(child_elem) => {
                if let Some(slot) = child_elem.slot_name.as_deref() {
                    if slot == "header" {
                        header_slot_children.push(child);
                        continue;
                    }
                }
                // 非 header slot 的 element 子节点都视为 body 内容
                body_children.push(child);
            }
            _ => {}
        }
    }

    // 1. title 文本子节点 → .title("...")（仅当无 label 属性时）
    if !title_set_by_attr {
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".title({:?})", text));
                break;
            }
        }
    }

    // 2. header slot 子节点 → .title_child(<element>)
    // `<template slot="header">` 的子节点逐个 .title_child() 注入
    for slot_node in &header_slot_children {
        if let Node::Element(template_elem) = slot_node {
            // template slot="header" 的子节点逐个注入为 title_child
            for header_child in &template_elem.children {
                let (child_code, is_iter) =
                    gen_node(header_child, ctx, 0, id_counter, &child_loop_vars)?;
                if is_iter {
                    code.push_str(&format!(".title_child({}.into_any_element())", child_code));
                } else {
                    code.push_str(&format!(".title_child({})", child_code));
                }
            }
        }
    }

    // 3. body 子节点 → .body(closure)
    if !body_children.is_empty() {
        let body_code = if body_children.len() == 1 {
            let (child_code, _) = gen_node(body_children[0], ctx, 0, id_counter, &child_loop_vars)?;
            format!("({}).into_any_element()", child_code)
        } else {
            let mut div_code = String::from("gpui::div()");
            for child in &body_children {
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, &child_loop_vars)?;
                if is_iter {
                    div_code.push_str(&format!(".children({})", child_code));
                } else {
                    div_code.push_str(&format!(".child({})", child_code));
                }
            }
            format!("({}).into_any_element()", div_code)
        };

        // TabItem::body 闭包签名：Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static
        // 闭包不接收 Context，无法访问 self/cx。
        // 解决方案：extract_body_deps 扫描 body 代码中的 self/cx 引用，
        // 在闭包外预提取为 owned 变量（Entity/String），闭包 move 捕获，闭包内用 app.theme()。
        let (prelude, body_code_replaced) =
            extract_body_deps(&body_code, &ctx.computed_methods, &child_loop_vars);

        code.push_str(&format!(
            ".body({{\n            \
             {prelude}\
             move |_window: &mut gpui::Window, app: &mut gpui::App| -> gpui::AnyElement {{\n                \
             {body}\n            }}\n            }})",
            prelude = prelude,
            body = body_code_replaced
        ));
        // format 字符串解析：
        // .body({  ← block expression 开始（{{ 转义为字面 {）
        //     {prelude}  ← let 语句
        //     move |...| -> gpui::AnyElement {  ← 闘包体开始（{{ 转义为字面 {）
        //         {body}
        //     }  ← 闭包体结束（}} 转义为字面 }）
        // }  ← block expression 结束（}} 转义为字面 }）
        // )  ← .body() 方法调用结束
    }

    if let Some(clause) = each_clause {
        let iter_code = format!(
            "self.{}.iter().map(|{}| {{\n                \
             let {} = {}.clone();\n                \
             {}\n            }})",
            clause.iterable, clause.item, clause.item, clause.item, code
        );
        return Ok((iter_code, true));
    }

    Ok((code, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Attribute, Directive, EachClause, Element, Node};
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
            slot_name: None,
            ..Default::default()
        }
    }

    fn make_element_with_slot(
        tag: &str,
        attrs: Vec<Attribute>,
        slot: &str,
        children: Vec<Node>,
    ) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: Some(slot.into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_tab_minimal_label_attr() {
        // <tab label="A" /> → TabItem::new().title("A")
        let elem = make_element(
            "tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(!is_iter);
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"A\")"));
        // 无 body 子节点时不生成 .body()
        assert!(!code.contains(".body("));
    }

    #[test]
    fn gen_tab_text_child_as_title() {
        // <tab>Account</tab> → .title("Account")
        let elem = make_element("tab", vec![], vec![Node::Text("Account".into())]);
        let mut id = 0;
        let (code, _) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(\"Account\")"));
    }

    #[test]
    fn gen_tab_with_icon() {
        // <tab icon="User" label="Account" /> → .title_icon(...).title("Account")
        let elem = make_element(
            "tab",
            vec![
                Attribute::Static {
                    name: "icon".into(),
                    value: "User".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "label".into(),
                    value: "Account".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let (code, _) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title_icon(rml_ui::IconName::User)"));
        assert!(code.contains(".title(\"Account\")"));
    }

    #[test]
    fn gen_tab_with_body_child() {
        // <tab label="A"><div>body</div></tab> → .title("A").body(closure)
        let div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let elem = make_element(
            "tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "A".into(),
                span: Span::empty(),
            }],
            vec![Node::Element(div)],
        );
        let mut id = 0;
        let (code, _) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title(\"A\")"));
        assert!(code.contains(".body("));
        assert!(code.contains("move |_window"));
        assert!(code.contains("into_any_element()"));
    }

    /// CodeEditor 在 body 内：验证 extract_body_deps 正确处理 self.__rml_state.get_or_init_ref + cx.theme()
    #[test]
    fn gen_tab_with_code_editor_body_simulated() {
        // 模拟 CodeEditor 生成的 block 表达式作为 body
        // 结构: { let __code = self.rml_sample(); Input::new(&self.__rml_state.get_or_init_ref(...)).font_family(cx.theme()...) }
        let body_code = r#"{ let __code = self.rml_sample(); rml_ui::Input::new(&self.__rml_state.get_or_init_ref("rml_editor", _window, &mut *cx, move |w, c| rml_ui::InputState::new(w, c).code_editor("rust").multi_line(true).default_value(&__code))).font_family(cx.theme().mono_font_family.clone()).text_size(cx.theme().mono_font_size).w_full().h(gpui::px(360.)).focus_bordered(false) }"#;
        let (prelude, replaced) = extract_body_deps(body_code, &["rml_sample".to_string()], &[]);

        // prelude 应包含 let __code = self.rml_sample();
        assert!(prelude.contains("let __code = self.rml_sample();"), "prelude missing __code: {}", prelude);

        // prelude 应包含 let __rml_entity_0 = self.__rml_state.get_or_init_ref(...);
        assert!(prelude.contains("let __rml_entity_0 = self.__rml_state.get_or_init_ref("), "prelude missing entity: {}", prelude);

        // prelude 中的 get_or_init_ref 应保留完整调用（含 ctor 闭包）
        assert!(prelude.contains("move |w, c| rml_ui::InputState::new(w, c)"), "prelude missing ctor closure: {}", prelude);

        // replaced body 应移除 let __code 语句
        assert!(!replaced.contains("let __code ="), "body should not contain let __code: {}", replaced);

        // replaced body 应把 self.__rml_state.get_or_init_ref(...) 替换为 __rml_entity_0
        assert!(replaced.contains("&__rml_entity_0"), "body should reference __rml_entity_0: {}", replaced);
        assert!(!replaced.contains("self.__rml_state.get_or_init_ref"), "body should not contain get_or_init_ref call: {}", replaced);

        // replaced body 应把 cx.theme() 替换为 app.theme()
        assert!(replaced.contains("app.theme().mono_font_family"), "body should use app.theme(): {}", replaced);
        assert!(!replaced.contains("cx.theme()"), "body should not reference cx.theme(): {}", replaced);

        // replaced body 应保留 Input::new 构造
        assert!(replaced.contains("rml_ui::Input::new"), "body should contain Input::new: {}", replaced);
    }

    #[test]
    fn gen_tab_with_header_slot() {
        // <tab>
        //   <template slot="header"><Icon name="File" /><span>README</span></template>
        //   <div>body</div>
        // </tab>
        // → .title_child(Icon).title_child(span).body(closure div)
        let icon = make_element("Icon", vec![Attribute::Static {
            name: "name".into(),
            value: "File".into(),
            span: Span::empty(),
        }], vec![]);
        let span = make_element("span", vec![], vec![Node::Text("README".into())]);
        let template = make_element_with_slot("template", vec![], "header",
            vec![Node::Element(icon), Node::Element(span)]);
        let body_div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let elem = make_element("tab", vec![], vec![
            Node::Element(template),
            Node::Element(body_div),
        ]);
        let mut id = 0;
        let (code, _) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        // header slot 子节点 → .title_child(...)
        assert!(code.contains(".title_child("));
        // body 子节点 → .body(closure)
        assert!(code.contains(".body("));
    }

    #[test]
    fn gen_tab_with_each_no_body() {
        // <tab each={tab in tabs} label={tab.title} closable={tab.closable} />
        let elem = make_element_with_directives(
            "tab",
            vec![
                Attribute::Bind {
                    name: "label".into(),
                    expr: "tab.title".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "closable".into(),
                    expr: "tab.closable".into(),
                    span: Span::empty(),
                },
            ],
            vec![Directive::Each {
                clause: EachClause {
                    item: "tab".into(),
                    index: None,
                    iterable: "tabs".into(),
                },
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(is_iter);
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains("rml_ui::TabItem::new()"));
        assert!(code.contains(".title(tab.title.clone())"));
        assert!(code.contains(".closable(tab.closable)"));
        // 无 body 子节点时不生成 .body()
        assert!(!code.contains(".body("));
    }

    #[test]
    fn gen_tab_with_each_and_body() {
        // <tab each={tab in tabs} label={tab.title}><div>{tab.content}</div></tab>
        let div = make_element("div", vec![], vec![Node::Text("body".into())]);
        let elem = make_element_with_directives(
            "tab",
            vec![Attribute::Bind {
                name: "label".into(),
                expr: "tab.title".into(),
                span: Span::empty(),
            }],
            vec![Directive::Each {
                clause: EachClause {
                    item: "tab".into(),
                    index: None,
                    iterable: "tabs".into(),
                },
                span: Span::empty(),
            }],
            vec![Node::Element(div)],
        );
        let mut id = 0;
        let (code, is_iter) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(is_iter);
        assert!(code.contains("self.tabs.iter().map(|tab|"));
        assert!(code.contains("let tab = tab.clone();"));
        assert!(code.contains(".body("));
    }

    #[test]
    fn gen_tab_label_attr_priority_over_text() {
        // <tab label="Attr">Text</tab> → .title("Attr")（属性优先，忽略文本）
        let elem = make_element(
            "tab",
            vec![Attribute::Static {
                name: "label".into(),
                value: "Attr".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("Text".into())],
        );
        let mut id = 0;
        let (code, _) = gen_tab_child(&elem, &ctx(), &mut id, &[]).unwrap();
        assert!(code.contains(".title(\"Attr\")"));
        assert!(!code.contains(".title(\"Text\")"));
    }
}
