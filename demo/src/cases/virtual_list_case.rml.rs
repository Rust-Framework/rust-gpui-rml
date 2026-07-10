use std::rc::Rc;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.virtual_list",
    kind = "case",
    group = "components",
    order = 73,
)]
#[component]
#[derive(Default)]
pub struct VirtualListCase {
    /// 1000 项数据
    pub items: Vec<String>,
    /// 每项尺寸（虚拟列表必须预声明所有项尺寸）
    pub item_sizes: Rc<Vec<gpui::Size<gpui::Pixels>>>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for VirtualListCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.virtual_list.title")
    }
}

impl ILifecycle for VirtualListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        // 生成 1000 项测试数据
        self.items = (0..1000).map(|i| format!("Item {}", i)).collect();
        // 每项固定高度 40px（垂直列表用 height，水平列表用 width）
        self.item_sizes = Rc::new(
            (0..1000)
                .map(|_| gpui::size(gpui::px(800.), gpui::px(40.)))
                .collect(),
        );

        let (cols, rows) = build_api_table(&[
            ("direction", "vertical/horizontal", "方向（默认 vertical，选择 v/h_virtual_list 构造器）"),
            ("item-sizes", "绑定", "Rc<Vec<Size<Pixels>>>，每项尺寸（垂直用 height，水平用 width）"),
            ("on-scroll", "事件", "滚动事件（预留）"),
            ("width/height", "样式", "VirtualList 实现 Styled trait，支持所有样式属性"),
            ("slot=render", "模板", "渲染模板，必须使用 each={i in range} 声明循环变量"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl VirtualListCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("virtual_list_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("virtual_list_case.rml.rs").to_string()
    }
}
