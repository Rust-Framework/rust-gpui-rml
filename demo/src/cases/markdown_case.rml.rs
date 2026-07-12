use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "framework.markdown",
    kind = "case",
    group = "framework",
    order = 59,
)]
#[component]
#[derive(Default)]
pub struct MarkdownCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for MarkdownCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.markdown.title")
    }
}

impl ILifecycle for MarkdownCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let (cols, rows) = build_api_table(&[
            ("content", "string / binding", "Markdown 源文本，如 content=\"# Hello\" 或 content={field}"),
            ("padding", "style", "内边距，如 padding=\"16px\""),
            ("background", "style", "背景色，如 background=\"var(--surface-variant)\""),
            ("border-radius", "style", "圆角，如 border-radius=\"6px\""),
            ("margin-top", "style", "顶部外边距，如 margin-top=\"12px\""),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl MarkdownCase {
    #[computed]
    pub fn basic_markdown(&self) -> String {
        "# 标题一\n\n\
         这是 **粗体** 和 *斜体* 和 ~~删除线~~ 文本。\n\n\
         - 列表项一\n\
         - 列表项二\n\
         - 列表项三\n\n\
         > 引用块文本\n\n\
         行内代码 `let x = 42;` 示例。"
            .to_string()
    }

    #[computed]
    pub fn code_block_markdown(&self) -> String {
        "```rust\n\
         fn main() {\n\
         \x20   let message = \"Hello, RML!\";\n\
         \x20   println!(\"{}\", message);\n\
         }\n\
         ```"
            .to_string()
    }

    #[computed]
    pub fn ai_response(&self) -> String {
        "## AI 助手回复\n\n\
         RML 框架已支持以下 IDE 级组件：\n\n\
         1. **Grid 布局** — 等宽网格，支持 `col-span` / `row-span`\n\
         2. **CodeEditor** — 基于 tree-sitter 的代码编辑器\n\
         3. **Tree** — 虚拟滚动文件树\n\
         4. **Markdown** — GFM 富文本渲染\n\n\
         > 提示：使用 `<Markdown content={field} />` 可实现 AI 聊天回复的实时渲染。\n\n\
         ```typescript\n\
         const response = await ai.chat(prompt);\n\
         markdownView.content = response;\n\
         ```"
            .to_string()
    }

    #[computed]
    pub fn table_markdown(&self) -> String {
        "| 组件 | 类型 | 说明 |\n\
         |------|------|------|\n\
         | Grid | Layout | 等宽网格布局 |\n\
         | Markdown | RichText | 富文本渲染 |\n\
         | Tree | Data | 虚拟滚动树 |"
            .to_string()
    }

    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("markdown_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("markdown_case.rml.rs").to_string()
    }
}
