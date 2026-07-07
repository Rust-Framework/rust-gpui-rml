//! Sourcemap 端到端链路测试
//!
//! 验证 compile() 返回的 CompileOutput.source_map 包含 codegen 透传的子元素 span，
//! 且可序列化为 JSON 供 dap crate 的 LineAccurateMapper 消费。

use rust_rml_engine::compiler::{compile, CodegenCtx};

fn minimal_ctx() -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        ..Default::default()
    }
}

const SAMPLE_RML: &str = r#"
<window title="Test">
    <div>
        <label text="Hello" />
        <button label="Click" on:click="handle_click" />
    </div>
</window>
"#;

#[test]
fn compile_returns_non_empty_sourcemap() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    // codegen 应记录子元素（<div>/<label>/<button>）的 span
    assert!(
        !output.source_map.entries.is_empty(),
        "sourcemap should have entries after compile"
    );
}

#[test]
fn sourcemap_records_child_elements_not_just_root() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    // 根元素 <window> 的 span 覆盖整个文档，子元素 span 在其内部
    // 验证 sourcemap 包含多个不同 span（至少有 <div>/<label>/<button> 三个子元素）
    let distinct_spans: std::collections::HashSet<_> = output
        .source_map
        .entries
        .iter()
        .map(|e| (e.rml_span.start, e.rml_span.end))
        .collect();
    assert!(
        distinct_spans.len() >= 3,
        "sourcemap should contain at least 3 distinct child element spans, got {}: {:?}",
        distinct_spans.len(),
        distinct_spans
    );
}

#[test]
fn sourcemap_spans_reference_actual_rml_positions() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    // <label> 在 SAMPLE_RML 中的字节起始偏移
    let label_start = SAMPLE_RML
        .find("<label")
        .expect("<label> should be present in sample");
    // <button> 在 SAMPLE_RML 中的字节起始偏移
    let button_start = SAMPLE_RML
        .find("<button")
        .expect("<button> should be present in sample");

    // sourcemap 应包含覆盖 <label> 起始偏移的 entry
    let has_label = output.source_map.entries.iter().any(|e| {
        e.rml_span.start <= label_start && label_start < e.rml_span.end
    });
    assert!(
        has_label,
        "sourcemap should contain an entry covering <label> at byte {}",
        label_start
    );

    // sourcemap 应包含覆盖 <button> 起始偏移的 entry
    let has_button = output.source_map.entries.iter().any(|e| {
        e.rml_span.start <= button_start && button_start < e.rml_span.end
    });
    assert!(
        has_button,
        "sourcemap should contain an entry covering <button> at byte {}",
        button_start
    );
}

#[test]
fn generated_code_has_no_sourcemap_markers() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    // 后处理应已删除所有 /*__rml_sm:S:E*/ 标记
    assert!(
        !output.code.contains("__rml_sm:"),
        "generated code should not contain sourcemap markers after postprocess"
    );
}

#[test]
fn sourcemap_is_json_serializable() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    let json = output.source_map.to_json().expect("serialize should succeed");
    assert!(!json.is_empty(), "JSON output should be non-empty");

    // 反序列化回 SourceMap，验证字段完整
    let restored = rust_rml_engine::compiler::source_map::SourceMap::from_json(&json)
        .expect("deserialize should succeed");
    assert_eq!(restored.entries.len(), output.source_map.entries.len());
}

#[test]
fn sourcemap_rust_lines_fall_within_generated_code() {
    let ctx = minimal_ctx();
    let output = compile(SAMPLE_RML, &ctx).expect("compile failed");

    let code_line_count = output.code.lines().count() as u32;
    for entry in &output.source_map.entries {
        assert!(
            entry.rust_line >= 1 && entry.rust_line <= code_line_count + 1,
            "entry rust_line {} should be within code lines [1, {}]",
            entry.rust_line,
            code_line_count + 1
        );
    }
}
