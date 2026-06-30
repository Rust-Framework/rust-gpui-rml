//! 窗口/对话框 impl 代码生成
//!
//! - `<window>`/`<modern_window>`/`<tab_window>` → `impl IWindow`
//! - `<dialog>` → `open(window, cx)` / `close(cx)` 方法

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};

/// 从 `<window>`/`<modern_window>`/`<tab_window>` 根节点生成 `impl IWindow`
///
/// 提取 `title`/`width`/`height` 属性，生成完整的 `impl IWindow` 代码块。
/// `chrome_transparent` 为 true 时生成 `WindowChrome::Transparent`。
pub(super) fn gen_window_impl(
    elem: &Element,
    ctx: &CodegenCtx,
    chrome_transparent: bool,
) -> Result<String, CodegenError> {
    let view_name = &ctx.view_struct_name;

    let title = extract_static_attr(elem, "title").unwrap_or_else(|| "RML Window".to_string());
    let width = extract_static_attr(elem, "width")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(800.0);
    let height = extract_static_attr(elem, "height")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(600.0);

    let left = extract_static_attr(elem, "left")
        .and_then(|s| s.parse::<f32>().ok());
    let top = extract_static_attr(elem, "top")
        .and_then(|s| s.parse::<f32>().ok());
    let startup = extract_static_attr(elem, "startup");
    let min_width = extract_static_attr(elem, "min_width")
        .and_then(|s| s.parse::<f32>().ok());
    let min_height = extract_static_attr(elem, "min_height")
        .and_then(|s| s.parse::<f32>().ok());

    let extra_methods = gen_window_extra_methods(left, top, startup.as_deref(), min_width, min_height);

    let chrome_method = if chrome_transparent {
        "    fn chrome(&self) -> rml_core::window::WindowChrome {\n        rml_core::window::WindowChrome::Transparent\n    }\n"
    } else {
        ""
    };

    let code = format!(
        r#"impl rml_core::window::IWindow for {view_name} {{
    fn title(&self) -> &str {{
        {title:?}
    }}

    fn width(&self) -> gpui::Pixels {{
        gpui::px({width:?})
    }}

    fn height(&self) -> gpui::Pixels {{
        gpui::px({height:?})
    }}
{extra_methods}
{chrome_method}
    fn open(&mut self, cx: &mut gpui::App) {{
        rml_ui::IWindowExt::open_rooted(self, cx);
    }}

    fn handle(&self) -> Option<gpui::AnyWindowHandle> {{
        self.__rml_window_handle
    }}

    fn set_handle(&mut self, handle: gpui::AnyWindowHandle) {{
        self.__rml_window_handle = Some(handle);
    }}
}}"#,
        view_name = view_name,
        title = title,
        width = width,
        height = height,
        extra_methods = extra_methods,
        chrome_method = chrome_method,
    );

    Ok(code)
}

/// 从 `<dialog>` 根节点生成 `open(window, cx)` / `close(cx)` 方法
///
/// 对话框不是独立 OS 窗口，而是父窗口 `Root` 层内的模态组件。
/// 复用 `#[window]` 宏注入的 `__rml_window_handle: Option<AnyWindowHandle>` 字段
/// 存储父窗口句柄，供 `close()` 通过 `AnyWindowHandle::update` 调用
/// `WindowExt::close_dialog`。
///
/// 标题栏由 `rml_ui::DialogTitleBar` 组件提供，内置拖动支持 + 关闭按钮。
/// 拖动偏移通过 `DialogDragState` 持久化，builder 闭包每帧读取 offset 并通过
/// `Styled::style().margin` 叠加到 Dialog 定位上。
pub(super) fn gen_dialog_impl(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    let view_name = &ctx.view_struct_name;

    let title = extract_static_attr(elem, "title").unwrap_or_else(|| "Dialog".to_string());
    let width = extract_static_attr(elem, "width")
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(480.0);
    let margin_top = extract_static_attr(elem, "margin_top")
        .and_then(|s| s.parse::<f32>().ok());

    let margin_top_expr = match margin_top {
        Some(mt) => format!("Some(gpui::px({:?}))", mt),
        None => "None".to_string(),
    };

    let code = format!(
        r#"impl {view_name} {{
    /// 在指定父窗口上打开对话框（模态）。
    ///
    /// 内部创建 entity + `DialogDragState`，存储父窗口句柄到 `__rml_window_handle`，
    /// 然后调用 `WindowExt::open_dialog`。标题栏内置拖动 + 关闭按钮。
    pub fn open(self, window: &mut gpui::Window, cx: &mut gpui::App) {{
        use rml_ui::WindowExt;
        use gpui::{{ParentElement, Styled}};
        let __rml_parent_handle = window.window_handle();
        let __rml_title: String = {title:?}.to_string();
        let __rml_width: gpui::Pixels = gpui::px({width:?});
        let __rml_margin_top: Option<gpui::Pixels> = {margin_top_expr};
        let __rml_drag_state = cx.new(|_| rml_ui::DialogDragState::default());
        let __rml_entity = cx.new(|_| {{
            let mut __rml_this = self;
            __rml_this.__rml_window_handle = Some(__rml_parent_handle);
            __rml_this
        }});
        window.open_dialog(cx, move |__rml_d, __rml_w, __rml_cx| {{
            let __rml_offset = {{ __rml_drag_state.read(__rml_cx).offset }};
            let __rml_title_bar = rml_ui::DialogTitleBar::new(
                __rml_title.clone(),
                __rml_drag_state.clone(),
            );
            let mut __rml_d = __rml_d
                .title(__rml_title_bar)
                .width(__rml_width)
                .close_button(false)
                .p_0();
            if let Some(__rml_mt) = __rml_margin_top {{
                __rml_d = __rml_d.margin_top(__rml_mt);
            }}
            __rml_d.style().margin.left = Some(__rml_offset.x.into());
            __rml_d.style().margin.top = Some(__rml_offset.y.into());
            __rml_d.child(__rml_entity.clone())
        }});
    }}

    /// 关闭对话框（通过父窗口句柄调用 `WindowExt::close_dialog`）。
    pub fn close(&mut self, cx: &mut gpui::Context<Self>) {{
        use rml_ui::WindowExt;
        if let Some(__rml_handle) = self.__rml_window_handle {{
            let _ = __rml_handle.update(cx, |_, __rml_window, __rml_cx| {{
                __rml_window.close_dialog(__rml_cx);
            }});
        }}
    }}
}}"#,
        view_name = view_name,
        title = title,
        width = width,
        margin_top_expr = margin_top_expr,
    );

    Ok(code)
}

/// 生成 IWindow 可选配置方法（left/top/startup/min_size）
fn gen_window_extra_methods(
    left: Option<f32>,
    top: Option<f32>,
    startup: Option<&str>,
    min_width: Option<f32>,
    min_height: Option<f32>,
) -> String {
    let mut out = String::new();

    if let Some(left) = left {
        out.push_str(&format!(
            "\n    fn left(&self) -> Option<gpui::Pixels> {{\n        Some(gpui::px({left:?}))\n    }}\n"
        ));
    }
    if let Some(top) = top {
        out.push_str(&format!(
            "\n    fn top(&self) -> Option<gpui::Pixels> {{\n        Some(gpui::px({top:?}))\n    }}\n"
        ));
    }
    if let Some(startup) = startup {
        let variant = match startup {
            "CenterScreen" | "center" | "center_screen" => {
                "rml_core::window::WindowStartupLocation::CenterScreen"
            }
            _ => "rml_core::window::WindowStartupLocation::Manual",
        };
        out.push_str(&format!(
            "\n    fn startup_location(&self) -> rml_core::window::WindowStartupLocation {{\n        {variant}\n    }}\n"
        ));
    }
    if min_width.is_some() || min_height.is_some() {
        let w = min_width.unwrap_or(0.0);
        let h = min_height.unwrap_or(0.0);
        out.push_str(&format!(
            "\n    fn min_size(&self) -> Option<gpui::Size<gpui::Pixels>> {{\n        Some(gpui::Size {{ width: gpui::px({w:?}), height: gpui::px({h:?}) }})\n    }}\n"
        ));
    }

    out
}

/// 从元素属性中提取静态属性值
pub(super) fn extract_static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name: n, value } if n == name => Some(value.clone()),
        _ => None,
    })
}
