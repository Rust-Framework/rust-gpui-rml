//! 页面级 CSS 端到端测试
//!
//! 验证：
//! 1. `<style source="..."/>` 元素从 codegen 输出中过滤（不渲染为元素）
//! 2. 页面 CSS 规则覆盖全局 CSS 规则（优先级：页面 > 全局）

use rust_rml_engine::compiler::{compile, CodegenCtx};
use rust_rml_engine::css::{self, StyleSheet};

fn ctx_with_stylesheet(sheet: Option<StyleSheet>) -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        stylesheet: sheet,
        ..Default::default()
    }
}

/// 解析 CSS 文本为 StyleSheet
fn parse_css(src: &str) -> StyleSheet {
    css::parse(src).expect("CSS parse should succeed")
}

/// `<style source="..."/>` 元素不出现在生成的 Rust 代码中
#[test]
fn style_element_filtered_from_codegen_output() {
    let rml = r#"<component>
        <style source="page.css" />
        <div class="content">hello</div>
    </component>"#;
    let ctx = ctx_with_stylesheet(None);
    let output = compile(rml, &ctx).expect("compile should succeed");
    // 生成的代码不应包含 "style" 标签相关内容
    assert!(
        !output.code.contains("\"style\""),
        " <style> element should be filtered from codegen output, got: {}",
        output.code
    );
}

/// 页面 CSS 规则追加在全局 CSS 之后，同选择器同属性后者覆盖前者
#[test]
fn page_css_overrides_global_css() {
    let global = parse_css(".foo { color: red; }");
    let page = parse_css(".foo { color: blue; }");

    // 模拟 build.rs 的合并逻辑：页面规则追加在全局之后
    let mut merged = global.clone();
    merged.rules.extend(page.rules);
    merged.variables.extend(page.variables);

    let rml = r#"<component>
        <div class="foo">text</div>
    </component>"#;
    let ctx = ctx_with_stylesheet(Some(merged));
    let output = compile(rml, &ctx).expect("compile should succeed");

    // GPUI "last write wins"：两个 text_color 调用都生成，
    // 页面 CSS (blue) 在全局 CSS (red) 之后 → blue 覆盖 red
    let red_pos = output.code.find("0xff0000ff").or_else(|| output.code.find("0xFF0000FF"));
    let blue_pos = output.code.find("0x0000ffff").or_else(|| output.code.find("0x0000FFFF"));
    assert!(red_pos.is_some(), "global CSS (red) should be in output, got: {}", output.code);
    assert!(blue_pos.is_some(), "page CSS (blue) should be in output, got: {}", output.code);
    let (red_pos, blue_pos) = (red_pos.unwrap(), blue_pos.unwrap());
    assert!(
        blue_pos > red_pos,
        "page CSS (blue at {}) should come AFTER global CSS (red at {}) for last-write-wins, got: {}",
        blue_pos, red_pos, output.code
    );
}

/// 仅全局 CSS（无页面 CSS）时正常工作
#[test]
fn global_css_only_works() {
    let global = parse_css(".bar { padding: 10px; }");
    let rml = r#"<component>
        <div class="bar">text</div>
    </component>"#;
    let ctx = ctx_with_stylesheet(Some(global));
    let output = compile(rml, &ctx).expect("compile should succeed");
    assert!(
        output.code.contains("10.0") || output.code.contains("10"),
        "global CSS padding should be applied, got: {}",
        output.code
    );
}

/// `<style>` 在 `<window>` 根下也被正确过滤
#[test]
fn style_element_filtered_in_window_root() {
    let rml = r#"<window title="Test">
        <style source="window.css" />
        <div>content</div>
    </window>"#;
    let ctx = ctx_with_stylesheet(None);
    let output = compile(rml, &ctx).expect("compile should succeed");
    assert!(
        !output.code.contains("\"style\""),
        "<style> should be filtered from window root codegen, got: {}",
        output.code
    );
}

/// 嵌套的 `<style>`（非根直接子节点）触发校验错误
#[test]
fn nested_style_element_returns_validation_error() {
    let rml = r#"<component>
        <div>
            <style source="nested.css" />
        </div>
    </component>"#;
    let ctx = ctx_with_stylesheet(None);
    let result = compile(rml, &ctx);
    assert!(result.is_err(), "nested <style> should fail validation");
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("直接子节点"),
        "error should mention direct child requirement, got: {}",
        msg
    );
}

/// 缺少 source 属性的 `<style>` 触发校验错误
#[test]
fn style_without_source_returns_validation_error() {
    let rml = r#"<component>
        <style />
        <div>content</div>
    </component>"#;
    let ctx = ctx_with_stylesheet(None);
    let result = compile(rml, &ctx);
    assert!(result.is_err(), "<style> without source should fail validation");
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(
        msg.contains("source"),
        "error should mention source attribute, got: {}",
        msg
    );
}
