use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.button",
    kind = "case",
    group = "components",
    order = 11,
)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,
    pub is_disabled: bool,
    pub is_selected: bool,
    pub is_loading: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.button.title")
    }
}

impl ILifecycle for ButtonCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("label", "字符串", "按钮文字"),
            ("primary / secondary / danger / success / warning / info / ghost / link / text", "布尔标志", "9 种 variant"),
            ("size", "xsmall/small/medium/large", "尺寸"),
            ("disabled", "布尔/绑定", "禁用"),
            ("selected", "布尔/绑定", "选中态"),
            ("loading", "布尔/绑定", "加载中"),
            ("compact", "布尔标志", "紧凑模式"),
            ("tooltip", "字符串", "悬浮提示"),
            ("font-bold / font-semibold / font-medium ...", "布尔标志", "字体权重"),
            ("on-click", "事件", "点击回调"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ButtonCase {
    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled { "禁用".into() } else { "可用".into() }
    }

    #[computed]
    pub fn selected_status_text(&self) -> String {
        if self.is_selected { "选中".into() } else { "未选中".into() }
    }

    #[computed]
    pub fn loading_status_text(&self) -> String {
        if self.is_loading { "加载中".into() } else { "空闲".into() }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- button_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 变体：空属性切换 9 种 variant -->
    <Button label="Default" />
    <Button label="Primary" primary="" />
    <Button label="Danger" danger="" />
    <Button label="Ghost" ghost="" />

    <!-- 尺寸 -->
    <Button label="Small" size="small" primary="" />
    <Button label="Large" size="large" primary="" />

    <!-- 状态绑定：disabled={bool_field} -->
    <Button label="Disabled" disabled={is_disabled} primary="" />
    <Button label="Loading" loading={is_loading} primary="" />

    <!-- 紧凑模式 -->
    <Button label="Compact" compact="" primary="" />

    <!-- tooltip -->
    <Button label="Save" tooltip="保存 (Ctrl+S)" primary="" />

    <!-- 事件绑定：on-click={method_name} -->
    <Button label="Click" on-click={on_basic_click} />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// button_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,        // 普通字段：可被 RML 绑定
    pub is_disabled: bool,
    pub is_loading: bool,
}

impl ILifecycle for ButtonCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // 初始化逻辑（如加载远程数据）
    }
}

impl ButtonCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 直接引用
    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled { "禁用".into() } else { "可用".into() }
    }

    // #[command] 标注的方法可被 on-click={on_basic_click} 调用
    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_loading = !self.is_loading;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
