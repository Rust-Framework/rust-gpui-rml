use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::build_api_table;

#[contribute(
    host_id = "demo.shell",
    id = "framework.ref",
    kind = "case",
    group = "framework",
    order = 55,
)]
#[component]
#[derive(Default)]
pub struct RefCase {
    /// 通过 `ref="input_state"` 指令关联，
    /// 首次渲染后由 `__rml_populate_refs` 注入 `Entity<InputState>` 句柄。
    ///
    /// 命令式访问（focus / set_value 等）需在 command 中通过 `with_mut` 调用：
    /// ```ignore
    /// self.input_state.with_mut(cx, |state| {
    ///     // state.focus(window, cx); // 需要 window 参数
    /// });
    /// ```
    /// 注：command 签名为 `(&mut self, &ClickEvent, &mut Context<Self>)`，
    /// 不直接接收 Window；需要 Window 的操作可通过 `cx.window()` 获取。
    pub input_state: ElementRef<InputState>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for RefCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.ref.title")
    }
}

impl ILifecycle for RefCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let _ = (_window, _cx);
        let (cols, rows) = build_api_table(&[
            ("ref=\"name\"", "指令", "声明元素引用名，关联到 ElementRef<T> 字段"),
            ("ElementRef<T>", "字段类型", "命令式访问句柄（focus/scroll/measure 等）"),
            ("__rml_populate_refs", "方法", "首次 render 后由 RML Runtime 调用，注入 Entity<T>"),
            ("with_mut(cx, f)", "方法", "可变访问底层 state；on_loaded 阶段返回 None（句柄未注入）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}
