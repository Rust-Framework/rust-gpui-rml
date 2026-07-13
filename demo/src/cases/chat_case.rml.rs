use std::sync::Arc;
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{
    ChatConversation, ChatError, ChatMessage, ChatMessageAction, ChatPanel, ChatRequest, IChatBackend,
    IconName, MessageActionItem, ModelInfo, RenderMode, TableColumn, TableRow,
};

use crate::cases::common::{build_api_table, CaseDocPage};

/// Echo 后端：回显用户消息（含 Markdown），用于 demo 演示。
struct EchoBackend;

impl IChatBackend for EchoBackend {
    fn send(
        &self,
        _conv: &ChatConversation,
        request: &ChatRequest,
    ) -> Result<ChatMessage, ChatError> {
        let reply = format!(
            "Echo: {}\n\n> _支持 **Markdown** 渲染、`代码块`、列表等格式_",
            request.content
        );
        Ok(ChatMessage::assistant(0, reply))
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

        let chat = cx.new(|cx| ChatPanel::new(RenderMode::Markdown, window, cx));
        chat.update(cx, |panel, cx| {
            panel.set_title("AI 助手", cx);
            panel.set_backend(Arc::new(EchoBackend), cx);
            panel.set_models(
                vec![
                    ModelInfo { id: "auto".into(), display_name: "自动模式".into() },
                    ModelInfo { id: "gpt-4".into(), display_name: "GPT-4".into() },
                    ModelInfo { id: "claude-3".into(), display_name: "Claude 3".into() },
                    ModelInfo { id: "local-llama".into(), display_name: "本地 Llama 3".into() },
                ],
                Some("auto".into()),
                cx,
            );
            panel.set_message_actions(
                vec![
                    MessageActionItem::new("regenerate", IconName::Redo2, "重新生成"),
                    MessageActionItem::new("thumbs-up", IconName::ThumbsUp, "点赞"),
                    MessageActionItem::new("thumbs-down", IconName::ThumbsDown, "点踩"),
                    MessageActionItem::new("speak", IconName::Play, "朗读"),
                ],
                cx,
            );
            panel.set_on_message_action(
                |panel, message_id, action, _window, cx| {
                    if let ChatMessageAction::Custom(id) = action {
                        if id.as_ref() == "regenerate" {
                            panel.regenerate(message_id, cx);
                        }
                    }
                },
                cx,
            );
        });
        self.chat = Some(chat);

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段（Option<Entity<ChatPanel>>），如 ref=\"chat\""),
            ("on_loaded 创建", "命令式 API", "cx.new(|cx| ChatPanel::new(RenderMode, window, cx)) 创建 Entity，闭包捕获 window 参数"),
            ("set_backend", "命令式 API", "通过 panel.set_backend(Arc<dyn IChatBackend>, cx) 注入后端实现"),
            ("set_title", "命令式 API", "panel.set_title(impl Into<String>, cx) 设置头部栏标题"),
            ("set_models", "命令式 API", "panel.set_models(Vec<ModelInfo>, Option<String>, cx) 配置模型选择器"),
            ("set_config", "命令式 API", "panel.set_config(ChatConfig, cx) 设置会话级配置（model/temperature 等）"),
            ("set_message_actions", "命令式 API", "panel.set_message_actions(Vec<MessageActionItem>, cx) 注入单条消息扩展操作按钮"),
            ("set_on_message_action", "命令式 API", "panel.set_on_message_action(|panel, id, action, window, cx| { ... }, cx) 设置消息操作回调"),
            ("regenerate", "命令式 API", "panel.regenerate(message_id, cx) 重新生成指定 AI 回复，自动剔除旧回复上下文"),
            ("IChatBackend trait", "trait", "实现 send（同步）/ stream（流式，默认实现调用 send）/ cancel 方法"),
            ("RenderMode", "enum", "PlainText 纯文本渲染 / Markdown Markdown 渲染（via RML Markdown 组件）"),
            ("ChatInput 胶囊/展开", "内置交互", "输入含换行或超 60 字自动切换为多行展开模式；Enter 发送，Shift+Enter 换行"),
            ("ChatInput 添加/模型", "内置交互", "(+) 按钮弹出添加菜单（图片/文件/计划）；模型按钮弹出模型选择 PopupMenu"),
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
