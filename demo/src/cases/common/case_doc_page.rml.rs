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

    /// 切换代码 Tab
    ///
    /// TabBar on_click 事件签名：`fn(&mut self, idx: usize, &mut Context<Self>)`
    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, cx: &mut gpui::Context<Self>) {
        self.code_tab = idx;
        cx.notify();
    }
}
