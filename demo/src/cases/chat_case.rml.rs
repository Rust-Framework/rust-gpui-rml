use std::sync::Arc;
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{ChatBackend, ChatError, ChatPanel, Conversation, RenderMode, TableColumn, TableRow};

use crate::cases::common::{build_api_table, CaseDocPage};

/// Echo 后端：回显用户消息，用于 demo 演示。
struct EchoBackend;

impl ChatBackend for EchoBackend {
    fn send_message(&self, _conv: &Conversation, content: &str) -> Result<String, ChatError> {
        Ok(format!("Echo: {}", content))
    }
    fn cancel(&self) -> Result<(), ChatError> {
        Ok(())
    }
}

#[contribute(
    host_id = "demo.shell",
    id = "components.chat",
    kind = "case",
    group = "components",
    order = 40,
)]
#[component]
#[derive(Default)]
pub struct ChatCase {
    /// EntityRef 组件字段：Option<Entity<ChatPanel>>。
    pub chat: Option<gpui::Entity<ChatPanel>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for ChatCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.chat.title")
    }
}

impl ILifecycle for ChatCase {
    fn on_loaded(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let chat = cx.new(|cx| ChatPanel::new(RenderMode::PlainText, window, cx));
        chat.update(cx, |panel, cx| {
            panel.set_backend(Arc::new(EchoBackend), cx);
        });
        self.chat = Some(chat);

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段（Option<Entity<ChatPanel>>），如 ref=\"chat\""),
            ("on_loaded 创建", "命令式 API", "cx.new(|cx| ChatPanel::new(RenderMode, window, cx)) 创建 Entity，闭包捕获 window 参数"),
            ("set_backend", "命令式 API", "通过 panel.set_backend(Arc<dyn ChatBackend>, cx) 注入后端实现"),
            ("ChatBackend trait", "trait", "实现 send_message（同步）/ stream_message（流式）/ cancel 方法"),
            ("RenderMode", "enum", "PlainText 纯文本渲染 / Markdown Markdown 渲染（via RML Markdown 组件）"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl ChatCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("chat_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("chat_case.rml.rs").to_string()
    }
}
