use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "components.accordion",
    kind = "case",
    group = "components",
    order = 10,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub last_open: String,
    pub basic_open: Vec<usize>,
    pub multiple_open: Vec<usize>,
    pub sizes_small_open: Vec<usize>,
    pub sizes_large_open: Vec<usize>,
    pub with_icon_open: Vec<usize>,
    pub nested_open: Vec<usize>,
    pub nested_child_open: Vec<usize>,
    pub code_tab: usize,
    pub accordion_api_columns: Vec<TableColumn>,
    pub accordion_api_rows: Vec<TableRow>,
    pub item_api_columns: Vec<TableColumn>,
    pub item_api_rows: Vec<TableRow>,
}

impl IContribution for AccordionCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.accordion.title")
    }
}

impl ILifecycle for AccordionCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        // 受控模式下初始展开态由状态字段决定，避免在 item 上硬编码 open 导致点击无法收起
        self.basic_open = vec![0];
        self.multiple_open = vec![0, 1];
        let (cols, rows) = build_api_table(&[
            ("bordered", "布尔标志", "显示边框"),
            ("multiple", "布尔标志", "允许多项同时展开"),
            ("size", "small/medium/large", "尺寸变体"),
            ("open-ixs", "绑定", "展开项索引列表（Vec<usize>）"),
            ("on-toggle-click", "事件", "展开状态变化回调"),
        ]);
        self.accordion_api_columns = cols;
        self.accordion_api_rows = rows;

        let (cols, rows) = build_api_table(&[
            ("title", "字符串/绑定", "面板标题"),
            ("icon", "图标名", "标题图标"),
            ("disabled", "布尔", "禁用面板"),
        ]);
        self.item_api_columns = cols;
        self.item_api_rows = rows;
    }
}

impl AccordionCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- accordion_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 基础用法：bordered + open-ixs={basic_open}（Vec<usize> 绑定） -->
    <accordion bordered="" open-ixs={basic_open}>
        <item title="第一项">
            <p>内容</p>
        </item>
        <item title="第二项">
            <p>内容</p>
        </item>
        <item title="禁用项" disabled="true">
            <p>内容</p>
        </item>
    </accordion>

    <!-- multiple：允许多项同时展开 -->
    <accordion multiple="" bordered="" open-ixs={multiple_open}>
        <item title="多项展开">
            <p>允许多项同时展开</p>
        </item>
    </accordion>

    <!-- size 尺寸 + on-toggle-click 回调 -->
    <accordion size="small" bordered="" open-ixs={small_open} on-toggle-click={on_toggle}>
        <item title="small 尺寸">
            <p>内容</p>
        </item>
    </accordion>

    <!-- item 的 icon 属性 -->
    <accordion bordered="" open-ixs={icon_open} on-toggle-click={on_toggle}>
        <item title="设置" icon="Settings">
            <p>内容</p>
        </item>
        <item title="禁用" icon="Bell" disabled="true">
            <p>内容</p>
        </item>
    </accordion>

    <!-- 嵌套 -->
    <accordion bordered="" open-ixs={nested_open}>
        <item title="父级">
            <accordion bordered="" multiple="" open-ixs={child_open}>
                <item title="子级 1">
                    <p>内容</p>
                </item>
                <item title="子级 2">
                    <p>内容</p>
                </item>
            </accordion>
        </item>
    </accordion>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// accordion_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub basic_open: Vec<usize>,
    pub multiple_open: Vec<usize>,
    pub nested_open: Vec<usize>,
    pub nested_child_open: Vec<usize>,
}

impl ILifecycle for AccordionCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // 受控模式：初始展开态由状态字段决定
        self.basic_open = vec![0];
        self.multiple_open = vec![0, 1];
    }
}

impl AccordionCase {
    // on-toggle-click 回调签名：fn(&[usize], &mut Context<Self>)
    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        cx.notify();
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
