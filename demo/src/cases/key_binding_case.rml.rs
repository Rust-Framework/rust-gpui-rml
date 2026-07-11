use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.key_binding",
    kind = "case",
    group = "framework",
    order = 57,
)]
#[component]
#[derive(Default)]
pub struct KeyBindingCase {
    pub last_triggered: SharedString,
    pub trigger_count: u32,
    pub shortcut_enabled: bool,
    pub debug_count: u32,
    pub input_state: Option<gpui::Entity<InputState>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for KeyBindingCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.key_binding.title")
    }
}

impl ILifecycle for KeyBindingCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));
        self.last_triggered = "无".into();
        self.shortcut_enabled = true;
        let (cols, rows) = build_api_table(&[
            ("key", "static: String", "快捷键组合，如 'Ctrl+S' / 'Escape'。遵循 GPUI Keystroke::parse 语法"),
            ("when", "bind: bool", "是否启用快捷键（默认 true）。传入 false 时禁用"),
            ("on-press", "event: fn(&mut self, cx)", "快捷键触发回调，签名为 Fn(&mut Window, &mut App)"),
            ("KeyBinding", "组件", "声明式键盘快捷键容器，RenderOnce + ParentElement，通过事件冒泡接收 keydown"),
            ("Keystroke::parse", "GPUI API", "快捷键解析器，支持 ctrl/alt/shift/cmd/win/super/fn/secondary 修饰键"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl KeyBindingCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("key_binding_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("key_binding_case.rml.rs").to_string()
    }

    #[computed]
    pub fn shortcut_status(&self) -> SharedString {
        if self.shortcut_enabled {
            "已启用".into()
        } else {
            "已禁用".into()
        }
    }

    fn trigger(&mut self, label: &str) {
        self.last_triggered = label.into();
        self.trigger_count += 1;
    }

    #[command]
    pub fn on_save(&mut self, _cx: &mut Context<Self>) {
        self.trigger("Ctrl+S (保存)");
    }

    #[command]
    pub fn on_open(&mut self, _cx: &mut Context<Self>) {
        self.trigger("Ctrl+O (打开)");
    }

    #[command]
    pub fn on_clear(&mut self, _cx: &mut Context<Self>) {
        self.trigger("Escape (清除)");
    }

    #[command]
    pub fn on_debug(&mut self, _cx: &mut Context<Self>) {
        self.debug_count += 1;
    }

    #[command]
    pub fn on_save_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.on_save(cx);
    }

    #[command]
    pub fn on_open_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.on_open(cx);
    }

    #[command]
    pub fn on_clear_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.on_clear(cx);
    }
}
