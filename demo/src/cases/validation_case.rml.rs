use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.validation",
    kind = "case",
    group = "framework",
    order = 45,
)]
#[component]
#[derive(Default)]
pub struct ValidationCase {
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
    pub email: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ValidationCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.validation.title")
    }
}

impl ILifecycle for ValidationCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.name = "张三".into();
        self.age = 25;
        self.email = "user@example.com".into();
        let (cols, rows) = build_api_table(&[
            ("value={field}", "绑定属性", "双向绑定 input 到 pub 字段"),
            ("#[validate(range(min,max))]", "属性", "数值范围验证，失败显示红框"),
            ("#[computed]", "方法", "基于字段计算派生值"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ValidationCase {
    #[computed]
    pub fn name_valid(&self) -> bool {
        !self.name.trim().is_empty()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("validation_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("validation_case.rml.rs").to_string()
    }

    #[computed]
    pub fn email_valid(&self) -> bool {
        let email = self.email.trim();
        !email.is_empty() && email.contains('@') && email.contains('.')
    }

    #[computed]
    pub fn form_status(&self) -> String {
        let mut issues = Vec::new();
        if !self.name_valid() {
            issues.push("姓名为空");
        }
        if self.age < 0 || self.age > 150 {
            issues.push("年龄超出范围");
        }
        if !self.email_valid() {
            issues.push("邮箱格式错误");
        }
        if issues.is_empty() {
            "表单验证通过".to_string()
        } else {
            format!("验证失败：{}", issues.join("、"))
        }
    }
}
