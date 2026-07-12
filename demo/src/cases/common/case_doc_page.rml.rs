use gpui::SharedString;
use rml::prelude::*;

/// 案例页共享模板组件
///
/// 统一所有案例页的四段式布局：标题区 + 演示区 + 代码区 + API 区。
/// 通过 `#[component(slots = ["demo", "api"])]` 声明具名插槽，
/// 父视图用 `<template slot="demo">...</template>` 注入内容。
#[component(slots = ["demo", "api"])]
#[derive(Default)]
pub struct CaseDocPage {
    /// 案例标题
    pub title: SharedString,
    /// 案例描述
    pub description: SharedString,
    /// .rml 源码
    pub code_rml: String,
    /// .rs 源码
    pub code_rust: String,
    /// 代码 Tab 当前索引（0=RML, 1=Rust）
    pub code_tab: usize,
}

impl CaseDocPage {
    /// 代码行高约 20px（13px 字号 × 1.5 行高）；上下留白 16px。
    const CODE_LINE_HEIGHT_PX: f32 = 20.0;
    const CODE_EDITOR_PADDING_PX: f32 = 16.0;
    /// 与 `.rml-code-editor { min-height: 12rem }` 对齐。
    const CODE_EDITOR_MIN_PX: f32 = 192.0;
    /// 避免在 TabWindow 滚动区内撑满视口；超出部分在编辑器内滚动。
    const CODE_EDITOR_MAX_PX: f32 = 480.0;

    /// .rml 源码（computed 桥接，避免 String 字段在绑定中 move 出 &self）
    #[computed]
    pub fn rml_code(&self) -> String {
        self.code_rml.clone()
    }

    /// .rml.rs 源码
    #[computed]
    pub fn rust_code(&self) -> String {
        self.code_rust.clone()
    }

    /// 按当前源码行数自适应 CodeEditor 高度（有界、可参与文档流，不依赖视口 h_full）。
    #[computed]
    pub fn code_editor_height(&self) -> f32 {
        let lines = self
            .code_rml
            .lines()
            .count()
            .max(self.code_rust.lines().count())
            .max(6);
        (lines as f32 * Self::CODE_LINE_HEIGHT_PX + Self::CODE_EDITOR_PADDING_PX)
            .clamp(Self::CODE_EDITOR_MIN_PX, Self::CODE_EDITOR_MAX_PX)
    }

    /// 切换代码 Tab
    ///
    /// TabBar on_click 事件签名：`fn(&mut self, idx: usize, &mut Context<Self>)`
    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, cx: &mut gpui::Context<Self>) {
        self.code_tab = idx;
        cx.notify();
    }
}
