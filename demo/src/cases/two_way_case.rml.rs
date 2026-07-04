use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.shell",
    id = "binding.two-way",
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
    /// B-2 demo：货币双向绑定字段。
    /// `model={price | Currency}` 正向走 `Currency.convert(&self.price)` 显示 `¥1500.00`，
    /// 反向走 `Currency.convert_back(&value)` 解析 `¥1500.00` → `1500.0`。
    pub price: f64,
    /// B-3 demo：oninput 触发次数（逐键 +1，与 onchange 的失焦/回车触发互补）。
    pub input_event_count: u32,
    /// B-3 demo：onchange 触发次数（值提交时 +1）。
    pub change_event_count: u32,
}

impl IContribution for TwoWayCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.two_way.title").into()
    }
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

    /// B-2 demo：展示 converter 双向绑定后 VM 中的原始数值（非格式化串）。
    #[computed]
    pub fn price_raw(&self) -> String {
        format!("VM price 字段值：{}", self.price)
    }

    #[computed]
    pub fn code_sample(&self) -> String {
        r#"<input model={name} placeholder="姓名" />
<input model={age} placeholder="年龄" />
<input model={price | Currency} placeholder="金额" />
<p>{profile_summary}</p>"#
            .to_string()
    }

    /// B-3 demo：oninput 在 model 反向同步后触发，逐键递增计数。
    #[command]
    pub fn on_name_input(&mut self, _ev: &InputEvent, cx: &mut Context<Self>) {
        self.input_event_count += 1;
        cx.notify();
    }

    /// B-3 demo：onchange 在值提交时触发（失焦/回车），与 oninput 的逐键触发互补。
    #[command]
    pub fn on_age_change(&mut self, _ev: &ChangeEvent, cx: &mut Context<Self>) {
        self.change_event_count += 1;
        cx.notify();
    }
}
