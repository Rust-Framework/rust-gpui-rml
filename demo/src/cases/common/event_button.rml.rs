use gpui::SharedString;
use rml::prelude::*;

/// 用户组件事件绑定 demo：自定义按钮组件
///
/// 声明 `on_click` 事件回调字段（`Option<ClickHandler>`）。
/// 父视图通过 `<EventButton on-click={handler} />` 绑定时，
/// codegen 将父视图的 handler 方法包装为闭包并注入到此字段。
///
/// 模板内通过 `on-click={self.on_click}` 将回调应用到内部 div 元素，
/// 当 div 被点击时调用注入的闭包。
#[component]
#[derive(Default)]
pub struct EventButton {
    pub label: SharedString,
    pub on_click: Option<rml_core::event::ClickHandler>,
}
