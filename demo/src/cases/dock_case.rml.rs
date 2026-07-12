use std::sync::Arc;

use gpui::{
    Context, FocusHandle, Focusable, InteractiveElement, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div, px,
};
use rml::prelude::*;
use rml::theme::color as theme_color;
use rml_core::i18n::t_static;
use rml_ui::{DockArea, DockItem, PanelView, SimplePanel, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

// ── 面板内容视图 ──────────────────────────────────────────────

/// 文件树面板内容
struct FileTreeView {
    focus_handle: FocusHandle,
    files: Vec<(SharedString, bool)>,
}

impl FileTreeView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            files: vec![
                ("src".into(), true),
                ("  main.rs".into(), false),
                ("  lib.rs".into(), false),
                ("  mod.rs".into(), false),
                ("Cargo.toml".into(), false),
                ("README.md".into(), false),
            ],
        }
    }
}

impl Focusable for FileTreeView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileTreeView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(8.))
            .gap(px(2.))
            .text_sm()
            .children(self.files.iter().map(|(name, is_folder)| {
                div()
                    .px(px(6.))
                    .py(px(3.))
                    .rounded(px(4.))
                    .hover(|s| s.bg(theme_color("--text").opacity(0.1)))
                    .text_color(if *is_folder {
                        theme_color("--success")
                    } else {
                        theme_color("--text")
                    })
                    .child(name.clone())
            }))
    }
}

/// 编辑器面板内容
struct EditorView {
    focus_handle: FocusHandle,
    _filename: SharedString,
    code: SharedString,
}

impl EditorView {
    fn new(filename: &str, code: &str, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            _filename: filename.into(),
            code: code.into(),
        }
    }
}

impl Focusable for EditorView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EditorView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(12.))
            .text_sm()
            .text_color(theme_color("--text"))
            .bg(theme_color("--code-bg"))
            .child(self.code.clone())
    }
}

/// 终端面板内容
struct TerminalView {
    focus_handle: FocusHandle,
    lines: Vec<SharedString>,
}

impl TerminalView {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            lines: vec![
                "$ cargo build --release".into(),
                "   Compiling rust-rml v0.1.0".into(),
                "    Finished release [optimized] target(s) in 3.42s".into(),
                "$ ".into(),
            ],
        }
    }
}

impl Focusable for TerminalView {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TerminalView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(8.))
            .gap(px(1.))
            .text_sm()
            .font_family("monospace")
            .text_color(theme_color("--success"))
            .bg(theme_color("--code-bg"))
            .children(self.lines.iter().cloned())
    }
}

// ── Dock 案例 ─────────────────────────────────────────────────

#[contribute(
    host_id = "demo.shell",
    id = "framework.dock",
    kind = "case",
    group = "framework",
    order = 60,
)]
#[component]
#[derive(Default)]
pub struct DockCase {
    pub dock_area: Option<gpui::Entity<DockArea>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for DockCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.dock.title")
    }
}

impl ILifecycle for DockCase {
    fn on_loaded(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        // 1. 创建 DockArea
        let dock_area = cx.new(|cx| DockArea::new("demo-dock", None, window, cx));
        let weak_dock = dock_area.downgrade();

        // 2. 创建面板内容视图
        let file_tree_view = cx.new(|cx| FileTreeView::new(cx));
        let editor1_view = cx.new(|cx| EditorView::new("main.rs", "fn main() {\n    println!(\"Hello, RML Dock!\");\n}", cx));
        let editor2_view = cx.new(|cx| EditorView::new("lib.rs", "//! RML Dock 演示库\n\npub fn version() -> &str {\n    \"0.1.0\"\n}", cx));
        let terminal_view = cx.new(|cx| TerminalView::new(cx));

        // 3. 创建 SimplePanel 包装
        let file_tree_panel = cx.new(|cx| {
            SimplePanel::new("file-tree", "文件树", file_tree_view.into(), window, cx)
        });
        let editor1_panel = cx.new(|cx| {
            SimplePanel::new("editor-main", "main.rs", editor1_view.into(), window, cx)
        });
        let editor2_panel = cx.new(|cx| {
            SimplePanel::new("editor-lib", "lib.rs", editor2_view.into(), window, cx)
        });
        let terminal_panel = cx.new(|cx| {
            SimplePanel::new("terminal", "终端", terminal_view.into(), window, cx)
                .closable(false)
        });

        // 4. 创建 DockItem 布局
        let left_item = DockItem::tab(file_tree_panel, &weak_dock, window, &mut *cx);
        let center_item = DockItem::tabs(
            vec![
                Arc::new(editor1_panel) as Arc<dyn PanelView>,
                Arc::new(editor2_panel) as Arc<dyn PanelView>,
            ],
            &weak_dock,
            window,
            &mut *cx,
        );
        let bottom_item = DockItem::tab(terminal_panel, &weak_dock, window, &mut *cx);

        // 5. 设置 dock 布局
        dock_area.update(cx, |da, cx| {
            da.set_left_dock(left_item, Some(px(240.)), true, window, cx);
            da.set_center(center_item, window, cx);
            da.set_bottom_dock(bottom_item, Some(px(180.)), true, window, cx);
        });

        self.dock_area = Some(dock_area);

        // API 表格
        let (cols, rows) = build_api_table(&[
            ("content={dock_area}", "Entity<DockArea>", "通过透明容器渲染 DockArea 实体"),
            ("SimplePanel::new", "Rust API", "创建面板适配器，包装 AnyView 为 Panel"),
            ("DockItem::tab", "Rust API", "创建单面板 DockItem"),
            ("DockItem::tabs", "Rust API", "创建多标签页 DockItem"),
            ("set_left_dock", "Rust API", "设置左侧 dock 面板"),
            ("set_center", "Rust API", "设置中心 dock 面板"),
            ("set_bottom_dock", "Rust API", "设置底部 dock 面板"),
            ("set_right_dock", "Rust API", "设置右侧 dock 面板"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl DockCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("dock_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("dock_case.rml.rs").to_string()
    }
}
