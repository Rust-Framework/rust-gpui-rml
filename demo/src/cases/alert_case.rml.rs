use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.alert",
    kind = "case",
    group = "components",
    order = 40,
)]
#[component]
#[derive(Default)]
pub struct AlertCase {
    pub is_visible: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for AlertCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.alert.title")
    }
}

impl ILifecycle for AlertCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_visible = true;
        let (cols, rows) = build_api_table(&[
            ("info / success / warning / error", "布尔标志", "4 种 variant 关联函数（构造器选择 Alert::info(id, msg) 等）"),
            ("variant", "default/info/success/warning/error", "variant 属性（builder 方法 .with_variant(AlertVariant::Info)）"),
            ("message", "String / 绑定", "提示内容（构造器参数，优先级：静态属性 > 绑定 > 文本子节点）"),
            ("title", "String", "提示标题"),
            ("banner", "布尔标志", "切换为顶部横幅模式（无边框，撑满宽度）"),
            ("visible", "bool / 绑定", "是否可见"),
            ("icon", "IconName 枚举变体名", "自定义图标（如 icon=\"Bell\"）"),
            ("on-close", "事件", "关闭按钮点击回调（ClickEvent）"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl AlertCase {
    #[computed]
    pub fn message_from_field(&self) -> &'static str {
        "绑定字段作为 message"
    }

    #[computed]
    pub fn visibility_label(&self) -> &'static str {
        if self.is_visible { "可见" } else { "已关闭" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- alert_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- variant 关联函数：空属性切换构造器 -->
    <Alert message="默认提示" />
    <Alert info="" message="Info 提示" />
    <Alert success="" message="Success 提示" />
    <Alert warning="" message="Warning 提示" />
    <Alert error="" message="Error 提示" />

    <!-- variant 属性：builder 方法 -->
    <Alert variant="info" message="variant=info" />

    <!-- title + banner -->
    <Alert info="" title="标题" message="带标题的提示" />
    <Alert warning="" banner="" message="横幅警告" />

    <!-- icon 自定义 -->
    <Alert success="" icon="Check" message="自定义图标" />

    <!-- on-close 事件 + if 条件渲染 -->
    <Alert info="" title="可关闭" message="点击关闭" on-close={on_close} if={is_visible} />
    <Button label="重置" on-click={on_reset} />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// alert_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct AlertCase {
    pub is_visible: bool,  // 是否可见（控制 if 条件渲染）
}

impl ILifecycle for AlertCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_visible = true;
    }
}

impl AlertCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 直接引用
    #[computed]
    pub fn visibility_label(&self) -> &'static str {
        if self.is_visible { "可见" } else { "已关闭" }
    }

    // #[command] 标注的方法可被 on-close={on_close} 调用
    #[command]
    pub fn on_close(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = false;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = true;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_close(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = false;
    }

    #[command]
    pub fn on_reset(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_visible = true;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
