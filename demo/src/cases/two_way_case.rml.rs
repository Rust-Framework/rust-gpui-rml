use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

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
    /// `value={price | Currency}` 正向走 `Currency.convert(&self.price)` 显示 `¥1500.00`，
    /// 反向走 `Currency.convert_back(&value)` 解析 `¥1500.00` → `1500.0`。
    pub price: f64,
    /// B-3 demo：oninput 触发次数（逐键 +1，与 onchange 的失焦/回车触发互补）。
    pub input_event_count: u32,
    /// B-3 demo：onchange 触发次数（值提交时 +1）。
    pub change_event_count: u32,
    /// C5 demo：PascalCase Checkbox 自动双向绑定。
    /// `checked={agree}` 自动双向，无需 on-click 手动回写。
    pub agree: bool,
    /// C5 demo：PascalCase Switch 自动双向绑定。
    pub notifications: bool,
    /// C5 demo：PascalCase Rating 自动双向绑定（&usize 载荷）。
    pub score: usize,
    /// C5 demo：PascalCase Slider 自动双向绑定。
    pub volume: f32,
    /// C5 demo：PascalCase Input 自动双向绑定。
    pub username: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TwoWayCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.two_way.title")
    }
}

impl ILifecycle for TwoWayCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        let (cols, rows) = build_api_table(&[
            ("value={field}", "binding", "双向绑定到 pub 字段（input / Input / Rating / Slider）"),
            ("checked={field}", "binding", "Checkbox / Switch 自动双向，点击即回写 bool 字段"),
            ("value={field | Converter}", "binding", "双向绑定 + 转换器，如 value={price | Currency}"),
            ("#[validate(range(min, max))]", "Rust 属性", "标注数值验证规则，失败时框架自动显示红框 + tooltip"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

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

    /// C5 demo：汇总 PascalCase 组件双向绑定的字段状态。
    #[computed]
    pub fn pascal_summary(&self) -> String {
        format!(
            "agree={} | notifications={} | score={} | volume={:.1} | username=\"{}\"",
            self.agree, self.notifications, self.score, self.volume, self.username
        )
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("two_way_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("two_way_case.rml.rs").to_string()
    }

    /// B-3 demo：oninput 在 value 反向同步后触发，逐键递增计数。
    #[command]
    pub fn on_name_input(&mut self, _ev: &InputEvent, cx: &mut Context<Self>) {
        self.input_event_count += 1;
    }

    /// B-3 demo：onchange 在值提交时触发（失焦/回车），与 oninput 的逐键触发互补。
    #[command]
    pub fn on_age_change(&mut self, _ev: &ChangeEvent, cx: &mut Context<Self>) {
        self.change_event_count += 1;
    }
}
