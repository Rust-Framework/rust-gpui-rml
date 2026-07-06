use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.card",
    kind = "case",
    group = "components",
    order = 30,
)]
#[component]
#[derive(Default)]
pub struct CardCase {
    pub card_title: String,
    pub card_body: String,
    pub hoverable: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CardCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.card.title")
    }
}

impl ILifecycle for CardCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.card_title = "动态卡片".into();
        self.card_body = "这是通过 model 双向绑定控制的卡片内容。".into();
        self.hoverable = true;
        let (cols, rows) = build_api_table(&[
            ("title", "字符串", "卡片标题"),
            ("extra", "元素", "标题栏右侧扩展"),
            ("cover", "元素", "封面图"),
            ("footer", "元素", "底部区域"),
            ("bordered", "布尔", "显示边框"),
            ("borderless", "布尔标志", "无边框"),
            ("hoverable", "布尔标志", "悬浮效果"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CardCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- card_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础卡片：title + 子节点内容 -->
    <Card title="基础卡片">
        <p>这是卡片的内容区域。</p>
    </Card>

    <!-- 无边框：borderless="" -->
    <Card title="无边框卡片" borderless="">
        <p>无边框卡片，适合嵌入其他容器。</p>
    </Card>

    <!-- 悬浮效果：hoverable="" -->
    <Card title="可悬浮卡片" hoverable="">
        <p>鼠标悬浮时卡片会有视觉反馈。</p>
    </Card>

    <!-- 动态绑定：title={card_title} hoverable={hoverable} -->
    <Card title={card_title} hoverable={hoverable}>
        <p>{card_body}</p>
    </Card>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// card_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct CardCase {
    pub card_title: String,
    pub card_body: String,
    pub hoverable: bool,
}

impl ILifecycle for CardCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.card_title = "动态卡片".into();
        self.card_body = "这是通过 model 双向绑定控制的卡片内容。".into();
        self.hoverable = true;
    }
}

impl CardCase {
    #[command]
    pub fn on_toggle_hoverable(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.hoverable = !self.hoverable;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_toggle_hoverable(&mut self, _: &ClickEvent, _: &mut Context<Self>) {
        self.hoverable = !self.hoverable;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
