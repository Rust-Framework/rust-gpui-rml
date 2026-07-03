use rml::prelude::*;

#[contribute(
    host_id = "demo.activity",
    id = "binding.two-way",
    name = "case.two_way.title",
    kind = "case",
    group = "binding",
    order = 2,
)]
#[component]
#[derive(Default)]
pub struct TwoWayCase {
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
}

impl ILifecycle for TwoWayCase {}

impl TwoWayCase {
    #[computed]
    pub fn profile_summary(&self) -> String {
        if self.name.is_empty() {
            format!("请输入姓名（年龄：{}）", self.age)
        } else {
            format!("你好，{}（{}岁）", self.name, self.age)
        }
    }
}
