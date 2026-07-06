use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

#[contribute(
    host_id = "demo.shell",
    id = "components.table",
    kind = "case",
    group = "components",
    order = 15,
)]
#[component]
#[derive(Default)]
pub struct TableCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub user_rows: Vec<TableRow>,
    pub merged_rows: Vec<TableRow>,
    pub code_tab: usize,
}

impl IContribution for TableCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.table.title")
    }
}

impl ILifecycle for TableCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        // API 文档表格数据
        self.api_columns = vec![
            TableColumn::new("prop", "属性"),
            TableColumn::new("type", "类型"),
            TableColumn::new("desc", "说明"),
        ];
        self.api_rows = vec![
            TableRow::new()
                .cell("prop", "columns")
                .cell("type", "Vec<TableColumn>")
                .cell("desc", "数据绑定式列定义"),
            TableRow::new()
                .cell("prop", "rows")
                .cell("type", "Vec<TableRow>")
                .cell("desc", "行数据绑定"),
            TableRow::new()
                .cell("prop", "bordered")
                .cell("type", "布尔标志")
                .cell("desc", "显示边框"),
            TableRow::new()
                .cell("prop", "stripe")
                .cell("type", "布尔标志")
                .cell("desc", "斑马纹样式"),
            TableRow::new()
                .cell("prop", "delegate")
                .cell("type", "Rc<dyn TableDelegate>")
                .cell("desc", "模板委托（自定义渲染）"),
        ];

        // 用户数据表格
        self.user_rows = vec![
            TableRow::new()
                .cell("name", "张三")
                .cell("age", "28")
                .cell("email", "zhangsan@example.com"),
            TableRow::new()
                .cell("name", "李四")
                .cell("age", "34")
                .cell("email", "lisi@example.com"),
            TableRow::new()
                .cell("name", "王五")
                .cell("age", "22")
                .cell("email", "wangwu@example.com"),
        ];

        // 合并列示例：category 列跨 2 行
        self.merged_rows = vec![
            TableRow::new()
                .cell("category", "水果")
                .cell("name", "苹果")
                .cell("value", "5")
                .row_span("category", 2),
            TableRow::new()
                .cell("name", "香蕉")
                .cell("value", "3"),
            TableRow::new()
                .cell("category", "蔬菜")
                .cell("name", "胡萝卜")
                .cell("value", "8")
                .row_span("category", 2),
            TableRow::new()
                .cell("name", "菠菜")
                .cell("value", "4"),
        ];
    }
}

impl TableCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<!-- table_case.rml：声明式 UI，描述结构 + 绑定 + 事件 -->
<component>
    <!-- 1. 数据绑定式（columns + rows 双绑定） -->
    <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />

    <!-- 2. 声明式 Column 子标签 -->
    <Table rows={user_rows} bordered="">
        <Column key="name" title="姓名" width="120" />
        <Column key="age" title="年龄" align="center" />
        <Column key="email" title="邮箱" />
    </Table>

    <!-- 3. 小写标签形式 -->
    <table rows={user_rows} bordered="" stripe="">
        <column key="name" title="姓名" />
        <column key="age" title="年龄" align="center" />
        <column key="email" title="邮箱" />
    </table>

    <!-- 4. 插槽模板（header + footer） -->
    <Table rows={user_rows} bordered="">
        <Column key="name" title="姓名" />
        <Column key="age" title="年龄" />
        <template slot="header">
            <span style="color: blue;">自定义列头</span>
        </template>
        <template slot="footer">
            <span>共 3 条记录</span>
        </template>
    </Table>

    <!-- 5. 单元格模板（Scoped Slot：field="name" 指定列） -->
    <Table rows={user_rows} bordered="">
        <Column key="name" title="姓名" />
        <Column key="age" title="年龄" />
        <template slot="cell" field="name">
            <span style="color: blue;">第 {row_idx} 行</span>
        </template>
    </Table>

    <!-- 6. 合并列（row_span 在 TableRow 上设置） -->
    <Table rows={merged_rows} bordered="">
        <Column key="category" title="分类" width="120" />
        <Column key="name" title="名称" />
        <Column key="value" title="值" align="center" />
    </Table>
</component>"#
            .to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"// table_case.rml.rs：后端状态 + computed + command handler
use rml::prelude::*;
use rml_ui::{TableColumn, TableRow};

#[component]
#[derive(Default)]
pub struct TableCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub user_rows: Vec<TableRow>,
    pub merged_rows: Vec<TableRow>,
}

impl ILifecycle for TableCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        // 列定义
        self.api_columns = vec![
            TableColumn::new("prop", "属性"),
            TableColumn::new("type", "类型"),
            TableColumn::new("desc", "说明"),
        ];
        // 行数据
        self.user_rows = vec![
            TableRow::new()
                .cell("name", "张三")
                .cell("age", "28")
                .cell("email", "zhangsan@example.com"),
        ];
        // 合并列：row_span 跨 2 行
        self.merged_rows = vec![
            TableRow::new()
                .cell("category", "水果")
                .cell("name", "苹果")
                .cell("value", "5")
                .row_span("category", 2),
            TableRow::new()
                .cell("name", "香蕉")
                .cell("value", "3"),
        ];
    }
}"#
            .to_string()
    }

    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
