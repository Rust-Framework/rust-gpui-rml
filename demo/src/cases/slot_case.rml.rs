use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use crate::components::card::Card;

/// 案例组件 —— 演示自定义组件插槽的填充。
///
/// 使用 `<Card>` 组件，通过 `<template slot="...">` 填充 header / footer，
/// 裸子节点填充 default 插槽。
#[contribute(
    host_id = "demo.activity",
    id = "components.slot",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct SlotCase {
    card: Option<gpui::Entity<Card>>,
}

impl IContribution for SlotCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slot.title").into()
    }
}

impl ILifecycle for SlotCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.card = Some(cx.new(|_| Card::new()));
    }
}
