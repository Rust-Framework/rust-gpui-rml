use rml::prelude::*;

/// 卡片组件 —— 演示自定义组件插槽（`#[component(slots = [...])]`）。
///
/// 声明三个插槽：`header` / `default` / `footer`。
/// 模板 `card.rml` 用 `<slot>` 占位符声明渲染位置，父视图用 `<template slot="...">` 填充。
#[component(slots = ["header", "default", "footer"])]
#[derive(Default)]
pub struct Card {}

impl ILifecycle for Card {}

impl Card {
    pub fn new() -> Self {
        Self::default()
    }
}
