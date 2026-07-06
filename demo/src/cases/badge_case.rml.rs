use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.badge",
    kind = "case",
    group = "components",
    order = 22,
)]
#[component]
#[derive(Default)]
pub struct BadgeCase {
    pub count: usize,
    pub max_val: usize,
    pub is_dot: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for BadgeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.badge.title")
    }
}

impl ILifecycle for BadgeCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.count = 5;
        self.max_val = 9;
        let (cols, rows) = build_api_table(&[
            ("count", "usize / 绑定", "Number variant 计数（0 时隐藏）"),
            ("max", "usize / 绑定", "Number variant 最大显示（超出显示 N+，默认 99）"),
            ("dot", "布尔标志", "切换为 Dot variant（小红点）"),
            ("icon", "图标名", "切换为 Icon variant（如 icon=\"Bell\"）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
            ("子节点", "元素", "包裹的内容（被徽标标记的元素）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl BadgeCase {
    #[computed]
    pub fn count_display(&self) -> String {
        if self.count > self.max_val {
            format!("{}+", self.max_val)
        } else {
            self.count.to_string()
        }
    }

    #[computed]
    pub fn variant_label(&self) -> &'static str {
        if self.is_dot { "Dot" } else { "Number" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- badge_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- Number variant（默认）：count 设置计数，count=0 时隐藏 -->
    <Badge count="5">
        <Avatar name="Alice" />
    </Badge>

    <!-- max 属性：超出显示 N+（默认 99） -->
    <Badge count="100" max="9">
        <Avatar name="Bob" />
    </Badge>

    <!-- Dot variant：dot="" 切换为小红点 -->
    <Badge dot="">
        <Avatar name="Carol" />
    </Badge>

    <!-- Icon variant：icon="Bell" 切换为图标徽标 -->
    <Badge icon="Bell">
        <Avatar name="Dave" />
    </Badge>

    <!-- 尺寸 size -->
    <Badge count="5" size="small">
        <Avatar name="Eve" />
    </Badge>
    <Badge count="5" size="large">
        <Avatar name="Frank" />
    </Badge>

    <!-- 动态绑定：count={field} -->
    <Badge count={count}>
        <Avatar name="Grace" />
    </Badge>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// badge_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct BadgeCase {
    pub count: usize,      // Number variant 计数（usize 匹配 Badge::count 签名）
    pub max_val: usize,   // 最大显示上限
    pub is_dot: bool,     // 切换 Number / Dot variant
}

impl ILifecycle for BadgeCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.count = 5;
        self.max_val = 9;
    }
}

impl BadgeCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 直接引用
    #[computed]
    pub fn count_display(&self) -> String {
        if self.count > self.max_val {
            format!("{}+", self.max_val)
        } else {
            self.count.to_string()
        }
    }

    // #[command] 标注的方法可被 on-click={on_xxx} 调用
    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count = 0;
    }

    #[command]
    pub fn on_toggle_dot(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_dot = !self.is_dot;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_increment(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_increment_10(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += 10;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count = 0;
    }

    #[command]
    pub fn on_toggle_dot(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_dot = !self.is_dot;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
