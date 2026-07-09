//! CSS 三层样式优先级端到端测试
//!
//! 验证扩展组件 style 属性生效 + 优先级顺序：
//! 归一化属性 > 内联 style > 页面 CSS class > 全局 CSS class
//!
//! GPUI "last write wins" → 代码生成顺序：构造器 → CSS class → style + 归一化属性
//!
//! 本测试覆盖三类组件：
//! - Stateless（Button）— 验证 setters.rs style 修复 + stateless.rs 优先级修复
//! - Specialized（Icon/Label）— 验证 gen 函数内 append_css_class_styles 插入位置
//! - Native（div）— 回归验证原生元素优先级未被破坏

use rust_rml_engine::compiler::{compile, CodegenCtx};
use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::css::{self, StyleSheet};

/// 构造带内置 translator 注册表 + 可选样式表的 CodegenCtx
fn ctx_with_sheet(sheet: Option<StyleSheet>) -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        stylesheet: sheet,
        registry: TranslatorRegistry::builtin(),
        ..Default::default()
    }
}

fn parse_css(src: &str) -> StyleSheet {
    css::parse(src).expect("CSS parse should succeed")
}

/// 编译 RML 并返回生成代码，失败时 panic
fn compile_rml(rml: &str, ctx: &CodegenCtx) -> String {
    compile(rml, ctx).expect("compile should succeed").code
}

/// 在 code 中查找 pattern 的字节位置，找不到则 panic
fn find_pos(code: &str, pattern: &str, label: &str) -> usize {
    code.find(pattern).unwrap_or_else(|| {
        panic!(
            "expected to find `{}` ({}) in generated code, got:\n{}",
            pattern, label, code
        )
    })
}

// ════════════════════════════════════════════════════════════════════════════
// P2.2 验证：style 属性对扩展组件生效（不再被丢弃）
// ════════════════════════════════════════════════════════════════════════════

/// `<Button style="padding: 10px">` 的 style 不被丢弃，生成 `.p(gpui::px(10`
#[test]
fn style_on_button_not_dropped() {
    let rml = r#"<component>
        <Button style="padding: 10px">Click</Button>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(None));
    assert!(
        code.contains(".p(gpui::px(10"),
        "Button style should generate .p(gpui::px(10...), got:\n{}",
        code
    );
}

/// `<Icon style="padding: 10px" name="Bell" />` 的 style 不被丢弃
#[test]
fn style_on_icon_not_dropped() {
    let rml = r#"<component>
        <Icon name="Bell" style="padding: 10px" />
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(None));
    assert!(
        code.contains(".p(gpui::px(10"),
        "Icon style should generate .p(gpui::px(10...), got:\n{}",
        code
    );
}

/// `<Label style="padding: 10px">` 的 style 不被丢弃
#[test]
fn style_on_label_not_dropped() {
    let rml = r#"<component>
        <Label style="padding: 10px">Text</Label>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(None));
    assert!(
        code.contains(".p(gpui::px(10"),
        "Label style should generate .p(gpui::px(10...), got:\n{}",
        code
    );
}

// ════════════════════════════════════════════════════════════════════════════
// P2.4 验证：CSS class 在 setter 之前应用（优先级反转修复）
// ════════════════════════════════════════════════════════════════════════════

/// Button：CSS class（padding:5px）在 style（padding:10px）之前
/// → style 覆盖 CSS class（last write wins）
#[test]
fn button_css_class_before_inline_style() {
    let sheet = parse_css(".button { padding: 5px; }");
    let rml = r#"<component>
        <Button style="padding: 10px">Click</Button>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let style_pos = find_pos(&code, ".p(gpui::px(10", "inline style padding 10px");
    assert!(
        class_pos < style_pos,
        "CSS class (at {}) should come BEFORE inline style (at {}) for correct priority, got:\n{}",
        class_pos, style_pos, code
    );
}

/// Button：CSS class（padding:5px）在归一化属性 padding="10px" 之前
#[test]
fn button_css_class_before_normalized_attr() {
    let sheet = parse_css(".button { padding: 5px; }");
    let rml = r#"<component>
        <Button padding="10px">Click</Button>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let attr_pos = find_pos(&code, ".p(gpui::px(10", "normalized attr padding 10px");
    assert!(
        class_pos < attr_pos,
        "CSS class (at {}) should come BEFORE normalized attr (at {}), got:\n{}",
        class_pos, attr_pos, code
    );
}

/// Icon（specialized）：CSS class 在 style 之前
#[test]
fn icon_css_class_before_inline_style() {
    let sheet = parse_css(".icon { padding: 5px; }");
    let rml = r#"<component>
        <Icon name="Bell" style="padding: 10px" />
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let style_pos = find_pos(&code, ".p(gpui::px(10", "inline style padding 10px");
    assert!(
        class_pos < style_pos,
        "Icon CSS class (at {}) should come BEFORE inline style (at {}), got:\n{}",
        class_pos, style_pos, code
    );
}

/// Label（specialized）：CSS class 在 style 之前
#[test]
fn label_css_class_before_inline_style() {
    let sheet = parse_css(".label { padding: 5px; }");
    let rml = r#"<component>
        <Label style="padding: 10px">Text</Label>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let style_pos = find_pos(&code, ".p(gpui::px(10", "inline style padding 10px");
    assert!(
        class_pos < style_pos,
        "Label CSS class (at {}) should come BEFORE inline style (at {}), got:\n{}",
        class_pos, style_pos, code
    );
}

/// Tag（specialized）：CSS class 在 style 之前
#[test]
fn tag_css_class_before_inline_style() {
    let sheet = parse_css(".tag { padding: 5px; }");
    let rml = r#"<component>
        <Tag style="padding: 10px">Tag</Tag>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let style_pos = find_pos(&code, ".p(gpui::px(10", "inline style padding 10px");
    assert!(
        class_pos < style_pos,
        "Tag CSS class (at {}) should come BEFORE inline style (at {}), got:\n{}",
        class_pos, style_pos, code
    );
}

// ════════════════════════════════════════════════════════════════════════════
// 回归验证：原生元素优先级未被破坏
// ════════════════════════════════════════════════════════════════════════════

/// div（原生元素）：CSS class 在 style 之前（回归验证）
#[test]
fn native_div_css_class_before_inline_style() {
    let sheet = parse_css(".foo { padding: 5px; }");
    let rml = r#"<component>
        <div class="foo" style="padding: 10px">text</div>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let style_pos = find_pos(&code, ".p(gpui::px(10", "inline style padding 10px");
    assert!(
        class_pos < style_pos,
        "div CSS class (at {}) should come BEFORE inline style (at {}), got:\n{}",
        class_pos, style_pos, code
    );
}

// ════════════════════════════════════════════════════════════════════════════
// 覆盖语义验证：last write wins
// ════════════════════════════════════════════════════════════════════════════

/// Button：inline style（color:blue）覆盖 CSS class（color:red）
/// 两者都生成，但 blue 在 red 之后 → blue 生效
#[test]
fn button_inline_style_overrides_css_class() {
    let sheet = parse_css(".button { color: red; }");
    let rml = r#"<component>
        <Button style="color: blue">Click</Button>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let red_pos = find_pos(&code, "0xff0000ff", "CSS class color red");
    let blue_pos = find_pos(&code, "0x0000ffff", "inline style color blue");
    assert!(
        blue_pos > red_pos,
        "inline style (blue at {}) should come AFTER CSS class (red at {}) to override, got:\n{}",
        blue_pos, red_pos, code
    );
}

/// Button：归一化属性 padding="20px" 覆盖 CSS class padding:5px
#[test]
fn button_normalized_attr_overrides_css_class() {
    let sheet = parse_css(".button { padding: 5px; }");
    let rml = r#"<component>
        <Button padding="20px">Click</Button>
    </component>"#;
    let code = compile_rml(rml, &ctx_with_sheet(Some(sheet)));

    let class_pos = find_pos(&code, ".p(gpui::px(5", "CSS class padding 5px");
    let attr_pos = find_pos(&code, ".p(gpui::px(20", "normalized attr padding 20px");
    assert!(
        attr_pos > class_pos,
        "normalized attr (20px at {}) should come AFTER CSS class (5px at {}) to override, got:\n{}",
        attr_pos, class_pos, code
    );
}
