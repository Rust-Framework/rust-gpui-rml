use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

/// 案例组件 —— 演示 ui crate Card 组件（Ant Design 风格）。
///
/// 使用 `<Card title={...} hoverable="">` 标准卡片 API：
/// - `title` 绑定到 i18n 文本
/// - `hoverable` 启用悬浮提升
/// - body 子节点直接作为卡片内容
#[contribute(
    host_id = "demo.shell",
    id = "components.slot",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct SlotCase {}

impl IContribution for SlotCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.slot.title").into()
    }
}

impl SlotCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<Card title="卡片标题" hoverable="">
    <p>卡片内容</p>
    <Button label="操作" primary="" />
</Card>"#
            .to_string()
    }
}
