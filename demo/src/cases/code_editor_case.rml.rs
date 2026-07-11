use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{InputState, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.code_editor",
    kind = "case",
    group = "components",
    order = 38,
)]
#[component]
#[derive(Default)]
pub struct CodeEditorCase {
    /// 4 个语言编辑器通过 ref 引用，在 on_loaded 中预创建 InputState 实体
    ///（含 language + default_value），RML 层仅引用 ref name
    pub rust_editor: ElementRef<InputState>,
    pub json_editor: ElementRef<InputState>,
    pub python_editor: ElementRef<InputState>,
    pub js_editor: ElementRef<InputState>,

    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for CodeEditorCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.code_editor.title")
    }
}

impl ILifecycle for CodeEditorCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let rust_code = r#"// Rust 语法高亮示例
fn fibonacci(n: u32) -> u32 {
    match n {
        0 => 0,
        1 => 1,
        _ => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

struct Point<T> {
    x: T,
    y: T,
}

impl<T: std::ops::Add<Output = T>> Point<T> {
    fn sum(&self) -> T {
        self.x + self.y
    }
}

fn main() {
    let p = Point { x: 1, y: 2 };
    println!("sum = {}", p.sum());
    println!("fib(10) = {}", fibonacci(10));
}
"#;

        let json_code = r#"{
    "name": "RML Framework",
    "version": "0.1.0",
    "features": [
        "declarative-ui",
        "data-binding",
        "syntax-highlighting"
    ],
    "components": {
        "Button": { "variant": "primary" },
        "Input": { "placeholder": "Enter text" }
    },
    "metadata": {
        "author": "RML Team",
        "license": "MIT"
    }
}
"#;

        let python_code = r#"# Python 语法高亮示例
from dataclasses import dataclass
from typing import List, Optional

@dataclass
class Task:
    id: int
    title: str
    completed: bool = False
    tags: List[str] = None

    def __post_init__(self):
        if self.tags is None:
            self.tags = []

class TaskManager:
    def __init__(self):
        self.tasks: List[Task] = []

    def add(self, title: str, tags: Optional[List[str]] = None) -> Task:
        task = Task(id=len(self.tasks) + 1, title=title, tags=tags or [])
        self.tasks.append(task)
        return task

    def complete(self, task_id: int) -> bool:
        for t in self.tasks:
            if t.id == task_id:
                t.completed = True
                return True
        return False
"#;

        let js_code = r#"// JavaScript 语法高亮示例
class EventEmitter {
    constructor() {
        this.listeners = new Map();
    }

    on(event, callback) {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, []);
        }
        this.listeners.get(event).push(callback);
        return () => this.off(event, callback);
    }

    emit(event, ...args) {
        const callbacks = this.listeners.get(event) || [];
        callbacks.forEach(cb => cb(...args));
    }

    off(event, callback) {
        const callbacks = this.listeners.get(event) || [];
        const idx = callbacks.indexOf(callback);
        if (idx > -1) callbacks.splice(idx, 1);
    }
}

const emitter = new EventEmitter();
const unsubscribe = emitter.on('data', (payload) => {
    console.log('Received:', payload);
});
emitter.emit('data', { id: 1, name: 'RML' });
"#;

        // 预创建 InputState 实体并注册到 __rml_state
        // 这样 RML 层的 <CodeEditor ref="rust_editor" /> 直接复用预创建的实体
        // 避免在 slot 模板内使用 value 绑定导致的 __code 作用域问题
        let rust_code_owned = rust_code.to_string();
        self.__rml_state.get_or_init_ref("rust_editor", _window, &mut *cx, move |w, c| {
            InputState::new(w, c)
                .code_editor("rust")
                .multi_line(true)
                .default_value(&rust_code_owned)
        });
        let json_code_owned = json_code.to_string();
        self.__rml_state.get_or_init_ref("json_editor", _window, &mut *cx, move |w, c| {
            InputState::new(w, c)
                .code_editor("json")
                .multi_line(true)
                .default_value(&json_code_owned)
        });
        let python_code_owned = python_code.to_string();
        self.__rml_state.get_or_init_ref("python_editor", _window, &mut *cx, move |w, c| {
            InputState::new(w, c)
                .code_editor("python")
                .multi_line(true)
                .default_value(&python_code_owned)
        });
        let js_code_owned = js_code.to_string();
        self.__rml_state.get_or_init_ref("js_editor", _window, &mut *cx, move |w, c| {
            InputState::new(w, c)
                .code_editor("javascript")
                .multi_line(true)
                .default_value(&js_code_owned)
        });

        let (cols, rows) = build_api_table(&[
            ("language", "静态字符串", "代码语言：rust/json/python/javascript/go/tsx/css/html/sql 等（默认 rml）"),
            ("value", "绑定/静态", "代码内容（绑定 ViewModel 字段或静态字符串；slot 内不可用，需 ref + on_loaded 预创建）"),
            ("ref", "字符串", "引用名，配合 on_loaded 中 __rml_state.get_or_init_ref 预创建 InputState"),
            ("on-change", "事件", "内容变更回调（参数：&Entity<InputState>）"),
            ("on-focus/on-blur", "事件", "焦点事件回调"),
            ("bordered", "布尔", "外边框（默认 false）"),
            ("focus-bordered", "布尔", "聚焦边框（默认 false）"),
            ("context-menu", "方法名", "右键菜单构建方法"),
            ("font-family/font-size", "样式", "默认 mono 字体 + 字号（可覆盖）"),
            ("width/height", "样式", "默认 w_full + h(360px)（可覆盖）"),
            ("语法高亮", "内置", "基于 tree-sitter，支持 30+ 语言（依赖 tree-sitter-languages feature）"),
            ("代码折叠", "内置", "folding: true（默认启用）"),
            ("行号", "内置", "line_number: true（默认启用）"),
            ("缩进辅助", "内置", "indent_guides: true（默认启用）"),
            ("自动缩进", "内置", "代码编辑器模式自动启用"),
            ("搜索", "内置", "searchable: true（代码编辑器默认启用）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CodeEditorCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("code_editor_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("code_editor_case.rml.rs").to_string()
    }
}
