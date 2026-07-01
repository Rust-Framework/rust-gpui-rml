//! `AppMenuBar` codegen
//!
//! 注意：当前为占位实现。`AppMenuBar` 基于 gpui-component 的 `GlobalState::app_menus()`
//! + `OwnedMenu`（GPUI 原生类型），与 MVVM 数据绑定设计冲突——ViewModel 会依赖
//! GPUI 原生类型。主窗口菜单栏应使用 `<menu items={...}>` MVVM 路径。
//! 如未来需要 macOS 原生菜单栏集成，需在框架层从 `IMenuItem` 翻译到 `OwnedMenu`。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;

pub fn gen_app_menu_bar(_elem: &Element, _ctx: &CodegenCtx) -> Result<String, CodegenError> {
    Ok("rml_ui::AppMenuBar::new(cx)".to_string())
}
