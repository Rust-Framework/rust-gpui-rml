use std::sync::Arc;
use std::sync::Once;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::build_api_table;

/// DescriptionList items 绑定的演示数据项。
/// name() → label，id() → value（通过 as_contribution() 能力查询提取）。
pub struct DescEntry {
    name: SharedString,
    id: String,
}

impl IContribution for DescEntry {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.name.clone()
    }
}

static DESC_ENTRY_REGISTERED: Once = Once::new();

fn ensure_desc_entry_registered() {
    DESC_ENTRY_REGISTERED.call_once(|| {
        register_contribution_ability::<DescEntry>();
    });
}

#[contribute(
    host_id = "demo.shell",
    id = "components.description_list",
    kind = "case",
    group = "components",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct DescriptionListCase {
    pub user_name: String,
    pub user_email: String,
    pub role: String,
    pub width: gpui::Pixels,
    pub is_vertical: bool,
    pub desitems: Vec<Arc<dyn IValue>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for DescriptionListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.description_list.title")
    }
}

impl ILifecycle for DescriptionListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        ensure_desc_entry_registered();
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = gpui::px(120.0);
        self.is_vertical = true;
        let (cols, rows) = build_api_table(&[
            ("vertical", "布尔/绑定", "纵向布局（默认横向）"),
            ("bordered", "布尔标志", "显示边框"),
            ("columns", "数字", "列数"),
            ("label-width", "像素值", "标签列宽"),
            ("items", "绑定", "批量数据绑定（Vec<Arc<dyn IValue>>）"),
            ("description label", "字符串", "子项标签（必填）"),
            ("description value", "字符串/元素", "子项值"),
            ("description span", "数字", "跨列数"),
            ("separator", "无属性", "分隔符"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
        self.desitems = vec![
            Arc::new(DescEntry { name: "产品名称".into(), id: "RML 框架".into() }),
            Arc::new(DescEntry { name: "版本".into(), id: "1.0.0".into() }),
            Arc::new(DescEntry { name: "许可证".into(), id: "MIT".into() }),
            Arc::new(DescEntry { name: "作者".into(), id: "Rust 社区".into() }),
        ];
    }
}

impl DescriptionListCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<descriptions bordered="" columns="3" label-width="120">
    <description label="用户名" value="alice" />
    <description label="邮箱" value="alice@example.com" />
    <description label="状态" value="活跃" span="2" />
</descriptions>

<descriptions vertical="" bordered="">
    <description label="姓名" value="张三" />
    <description label="年龄" value="28" />
</descriptions>

<descriptions bordered="" columns="2">
    <description label="产品" value="RML 框架" />
    <separator />
    <description label="版本" value="1.0.0" />
</descriptions>

<descriptions bordered="" columns="2" label-width={width}>
    <description label="用户名" value={user_name} />
    <description label="角色" value={role} span="2" />
</descriptions>

<descriptions bordered="" columns="2">
    <description label="角色">
        <Badge primary="">{role}</Badge>
    </description>
</descriptions>

<descriptions items={desitems} bordered="" columns="2" label-width="100" />
<descriptions vertical={is_vertical} bordered="" columns="2" label-width="100">
    <description label="字段 A" value="值 A" />
    <description label="字段 B" value="值 B" />
</descriptions>"#
            .to_string()
    }
}
