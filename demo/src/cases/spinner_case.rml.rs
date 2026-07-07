use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.spinner",
    kind = "case",
    group = "components",
    order = 64,
)]
#[component]
#[derive(Default)]
pub struct SpinnerCase {
    pub is_loading: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub skeleton_rows: Vec<TableRow>,
}

impl IContribution for SpinnerCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.spinner.title")
    }
}

impl ILifecycle for SpinnerCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_loading = true;
        let (cols, rows) = build_api_table(&[
            ("icon", "IconName 枚举变体名", "自定义图标（如 icon=\"Bell\"），默认 Loader"),
            ("size", "xsmall/small/medium/large", "尺寸（Sizable trait）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;

        let (_, skel_rows) = build_api_table(&[
            ("secondary", "布尔标志", "切换为次级颜色（次要占位）"),
        ]);
        self.skeleton_rows = skel_rows;
    }
}

impl SpinnerCase {
    #[computed]
    pub fn loading_label(&self) -> &'static str {
        if self.is_loading { "加载中" } else { "已完成" }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- spinner_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- Spinner 基础（默认 Loader 图标） -->
    <Spinner />
    <Spinner size="small" />
    <Spinner size="large" />

    <!-- icon 自定义图标（IconName 枚举变体名） -->
    <Spinner icon="Bell" />
    <Spinner icon="Settings" size="medium" />

    <!-- Skeleton 骨架屏 -->
    <Skeleton />
    <Skeleton secondary="" />

    <!-- if 条件渲染 + 字段绑定 -->
    <Spinner if={is_loading} size="small" />
    <Skeleton if={is_loading} />
    <Button label="切换" on-click={on_toggle_loading} />
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// spinner_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct SpinnerCase {
    pub is_loading: bool,
}

impl ILifecycle for SpinnerCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.is_loading = true;
    }
}

impl SpinnerCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 直接引用
    #[computed]
    pub fn loading_label(&self) -> &'static str {
        if self.is_loading { "加载中" } else { "已完成" }
    }

    // #[command] 标注的方法可被 on-click={on_xxx} 调用
    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.is_loading = !self.is_loading;
    }
}"#
            .to_string()
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
