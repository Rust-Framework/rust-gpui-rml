use std::sync::{Arc, Mutex};

use gpui::{
    AnyElement, App, Focusable, InteractiveElement, IntoElement, ParentElement, SharedString,
    Window, div,
};
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{Input, InputState, TableDelegate, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// 编辑状态（跨渲染持久化，存储在 ViewModel 中通过 Arc<Mutex> 共享给 delegate）
struct EditState {
    editing: Option<(usize, usize, SharedString)>,
    rows: Vec<TableRow>,
    input_state: Option<gpui::Entity<InputState>>,
    last_edit: SharedString,
}

impl Default for EditState {
    fn default() -> Self {
        Self {
            editing: None,
            rows: Vec::new(),
            input_state: None,
            last_edit: SharedString::default(),
        }
    }
}

/// 可编辑表格委托 —— 使用 Arc<Mutex> 管理编辑状态，满足 Send + Sync 要求。
pub struct EditableTableDelegate {
    state: Arc<Mutex<EditState>>,
    notify: Mutex<Option<Arc<dyn Fn(&mut App) + Send + Sync>>>,
}

impl Default for EditableTableDelegate {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(EditState::default())),
            notify: Mutex::new(None),
        }
    }
}

impl EditableTableDelegate {
    pub fn new(rows: Vec<TableRow>) -> Self {
        Self {
            state: Arc::new(Mutex::new(EditState {
                rows,
                ..Default::default()
            })),
            notify: Mutex::new(None),
        }
    }

    pub fn last_edit(&self) -> SharedString {
        self.state.lock().unwrap().last_edit.clone()
    }

    pub fn rows(&self) -> Vec<TableRow> {
        self.state.lock().unwrap().rows.clone()
    }
}

impl TableDelegate for EditableTableDelegate {
    fn can_edit(&self, _row: usize, _col: usize, column: &TableColumn) -> bool {
        column.editable
    }

    fn is_editing(&self, row: usize, col: usize) -> bool {
        self.state
            .lock()
            .unwrap()
            .editing
            .as_ref()
            .map_or(false, |(r, c, _)| *r == row && *c == col)
    }

    fn start_edit(&self, row: usize, col: usize) {
        let mut s = self.state.lock().unwrap();
        s.editing = Some((row, col, SharedString::default()));
        s.input_state = None;
    }

    fn stop_edit(&self) {
        let mut s = self.state.lock().unwrap();
        s.editing = None;
        s.input_state = None;
    }

    fn set_notify(&self, notify: Arc<dyn Fn(&mut App) + Send + Sync>) {
        *self.notify.lock().unwrap() = Some(notify);
    }

    fn render_editor(
        &self,
        row: usize,
        col: usize,
        column: &TableColumn,
        row_data: &TableRow,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        // 补全 editing 中的 column_key（start_edit 时未知 column_key）
        {
            let mut s = self.state.lock().unwrap();
            if let Some((r, c, _)) = &s.editing {
                if *r == row && *c == col {
                    s.editing = Some((row, col, column.key.clone()));
                }
            }
        }

        // 创建 InputState（仅首次）
        let state_entity = {
            let mut s = self.state.lock().unwrap();
            if s.input_state.is_none() {
                let initial = row_data.get(&column.key).to_string();
                let entity = cx.new(|cx| InputState::new(window, cx).default_value(&initial));
                let fh = entity.read(cx).focus_handle(cx);
                fh.focus(window, cx);
                s.input_state = Some(entity);
            }
            s.input_state.as_ref().unwrap().clone()
        };

        // 闭包捕获 state 和 notify（均为 Send + Sync）
        let delegate_state = Arc::clone(&self.state);
        let notify = self.notify.lock().unwrap().clone();

        div()
            .id(("editor", row * 10000 + col))
            .on_key_down(move |event, _window, cx| {
                if event.keystroke.key == "enter" {
                    let mut s = delegate_state.lock().unwrap();
                    if let Some((row, _col, column_key)) = s.editing.clone() {
                        if let Some(input) = s.input_state.as_ref() {
                            let new_value = input.read(cx).value();
                            if let Some(row_data) = s.rows.get_mut(row) {
                                row_data.cells.insert(
                                    column_key.clone(),
                                    SharedString::from(new_value.clone()),
                                );
                            }
                            s.last_edit = SharedString::from(format!(
                                "行 {} 列「{}」→ {}",
                                row + 1,
                                column_key,
                                new_value
                            ));
                        }
                        s.editing = None;
                        s.input_state = None;
                    }
                    drop(s);
                    if let Some(n) = &notify {
                        n(cx);
                    }
                } else if event.keystroke.key == "escape" {
                    let mut s = delegate_state.lock().unwrap();
                    s.editing = None;
                    s.input_state = None;
                    drop(s);
                    if let Some(n) = &notify {
                        n(cx);
                    }
                }
            })
            .child(Input::new(&state_entity))
            .into_any_element()
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "components.table_editable",
    kind = "case",
    group = "components",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct TableEditableCase {
    pub edit_delegate: Arc<EditableTableDelegate>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TableEditableCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.table_editable.title")
    }
}

impl ILifecycle for TableEditableCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let rows = vec![
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
        self.edit_delegate = Arc::new(EditableTableDelegate::new(rows));

        let (cols, rows) = build_api_table(&[
            ("editable", "Column 标志属性", "标记列为可编辑（editable=\"\" 或 editable=\"true\"）"),
            ("delegate", "Table 属性（绑定）", "Arc<dyn TableDelegate> —— 自定义编辑状态管理和编辑器渲染"),
            ("on-cell-edit", "Table 事件属性", "单元格编辑提交回调（row, col, new_value, cx）"),
            ("start_edit", "TableDelegate 方法", "进入编辑模式（由 Table 在点击可编辑单元格时调用）"),
            ("stop_edit", "TableDelegate 方法", "退出编辑模式"),
            ("is_editing", "TableDelegate 方法", "判断指定单元格是否处于编辑模式"),
            ("render_editor", "TableDelegate 方法", "渲染编辑器元素（如 Input）"),
            ("set_notify", "TableDelegate 方法", "注入重新渲染回调（由 Table 自动调用）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TableEditableCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("table_editable_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("table_editable_case.rml.rs").to_string()
    }

    #[computed]
    pub fn last_edit_info(&self) -> SharedString {
        self.edit_delegate.last_edit()
    }

    #[computed]
    pub fn editable_rows(&self) -> Vec<TableRow> {
        self.edit_delegate.rows()
    }
}
