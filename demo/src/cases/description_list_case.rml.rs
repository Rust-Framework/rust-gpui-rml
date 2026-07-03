use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
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
}

impl IContribution for DescriptionListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.description_list.title").into()
    }
}

impl ILifecycle for DescriptionListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = gpui::px(120.0);
    }
}

impl DescriptionListCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<DescriptionList bordered="" columns="3" label_width="120">
    <DescriptionItem label="用户名" value="alice" />
    <DescriptionItem label="邮箱" value="alice@example.com" />
    <DescriptionItem label="状态" value="活跃" span="2" />
</DescriptionList>

<DescriptionList vertical="" bordered="">
    <DescriptionItem label="姓名" value="张三" />
    <DescriptionItem label="年龄" value="28" />
</DescriptionList>

<descriptions bordered="" columns="2">
    <description label="产品" value="RML 框架" />
    <separator />
    <description label="版本" value="1.0.0" />
</descriptions>

<DescriptionList bordered="" columns="2" label_width={width}>
    <DescriptionItem label="用户名" value={user_name} />
    <DescriptionItem label="角色" value={role} span="2" />
</DescriptionList>

<DescriptionList bordered="" columns="2">
    <DescriptionItem label="角色">
        <Badge primary="">{role}</Badge>
    </DescriptionItem>
</DescriptionList>"#
            .to_string()
    }
}
