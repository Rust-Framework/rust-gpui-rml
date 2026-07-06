use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.progress_circle",
    kind = "case",
    group = "components",
    order = 27,
)]
#[component]
#[derive(Default)]
pub struct ProgressCircleCase {
    pub current: f32,
    pub loading: bool,
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for ProgressCircleCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.progress_circle.title")
    }
}

impl ILifecycle for ProgressCircleCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.current = 75.0;
        let (cols, rows) = build_api_table(&[
            ("value", "f32 / 绑定", "进度值 0-100（自动 clamp）"),
            ("loading", "布尔/绑定", "加载中状态（value 被忽略）"),
            ("size", "xsmall/small/medium/large", "尺寸（8px/12px/16px/20px 直径）"),
            ("color", "Hsla", "自定义颜色（暂未支持 RML 声明式设置）"),
            ("子节点", "元素", "中心可放图标/文本"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ProgressCircleCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.loading {
            "加载中... (loading=true)".to_string()
        } else {
            format!("当前进度：{:.0}%", self.current)
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- progress_circle_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：value={f32_field} 绑定进度值 -->
    <ProgressCircle value={current} />

    <!-- loading 属性：循环旋转动画（value 被忽略） -->
    <ProgressCircle loading="" />
    <ProgressCircle loading={is_loading} />

    <!-- 尺寸 size：4 档视觉区分 -->
    <ProgressCircle value={current} size="xsmall" />
    <ProgressCircle value={current} size="small" />
    <ProgressCircle value={current} size="medium" />
    <ProgressCircle value={current} size="large" />

    <!-- 子节点：中心可放图标/文本 -->
    <ProgressCircle value={current} size="large">
        <Icon name="Check" />
    </ProgressCircle>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// progress_circle_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct ProgressCircleCase {
    pub current: f32,    // 进度值 0-100
    pub loading: bool,   // loading 状态切换
}

impl ILifecycle for ProgressCircleCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.current = 75.0;
    }
}

impl ProgressCircleCase {
    // #[computed] 标注的方法可在 RML 中以 {method_name} 直接引用
    #[computed]
    pub fn status_text(&self) -> String {
        if self.loading {
            "加载中... (loading=true)".to_string()
        } else {
            format!("当前进度：{:.0}%", self.current)
        }
    }

    // #[command] 标注的方法可被 on-click={on_xxx} 调用
    #[command]
    pub fn on_increase(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current + 10.0).min(100.0);
    }

    #[command]
    pub fn on_decrease(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current - 10.0).max(0.0);
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.loading = !self.loading;
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_increase(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current + 10.0).min(100.0);
    }

    #[command]
    pub fn on_decrease(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.current = (self.current - 10.0).max(0.0);
    }

    #[command]
    pub fn on_toggle_loading(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.loading = !self.loading;
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
