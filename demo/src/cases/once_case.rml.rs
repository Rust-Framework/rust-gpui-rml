use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.once",
    kind = "case",
    group = "framework",
    order = 51,
)]
#[component]
#[derive(Default)]
pub struct OnceCase {
    pub counter: u32,
    pub frozen_counter: Option<u32>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for OnceCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.once.title")
    }
}

impl ILifecycle for OnceCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.counter = 0;
        self.frozen_counter = Some(0);
        let (cols, rows) = build_api_table(&[
            ("once", "指令", "标记元素仅首次渲染求值，后续渲染复用首次快照"),
            ("适用场景", "说明", "静态内容、配置信息、避免重复计算"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl OnceCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("once_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("once_case.rml.rs").to_string()
    }

    /// 模拟 once 指令的快照行为：首次渲染后冻结 counter 值。
    /// 之所以用字段而非 once 指令，是因为 once 指令在 `<template slot>` 闭包内
    /// 需要 &mut self，而 slot 闭包只能拿到 `&self`，无法编译。
    #[computed]
    pub fn once_counter(&self) -> u32 {
        self.frozen_counter.unwrap_or(self.counter)
    }

    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.counter = self.counter.saturating_add(1);
    }
}
