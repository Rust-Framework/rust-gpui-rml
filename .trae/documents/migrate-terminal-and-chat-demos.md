# 迁移 Terminal 与 Chat 组件到 RML 框架 — Demo 集成计划

## Summary

将 `rust-agent-ide` 的 Terminal 终端组件和 AI Chat 聊天组件迁移到 RML 框架的 `crates/ui-term` 和 `crates/ui-chat`。两个 crate 的源码已在前序会话中完成，引擎注册（tags.rs / props_registry.rs）也已完成。本计划聚焦于**修复一个关键架构缺口**（缺少 EntityRef 通用 translator）并**完成 demo 集成**，使两个组件可在 RML 声明式模板中使用。

## Current State Analysis

### 已完成 ✅

| 项目 | 状态 | 位置 |
|------|------|------|
| `crates/ui-term` 源码 | 13 个 rs 文件，完整 | `crates/ui-term/src/` |
| `crates/ui-chat` 源码 | 9 个 rs 文件，完整 | `crates/ui-chat/src/` |
| Workspace Cargo.toml | 已注册两个 crate 为 workspace 依赖 | `Cargo.toml` L27-28 |
| Engine tags.rs | Terminal / Chat 注册为 `ComponentKind::EntityRef` | `crates/engine/src/tags.rs` L835-846 |
| Engine props_registry.rs | Terminal / Chat 注册为空属性 | `crates/engine/src/compiler/props_registry.rs` L333-336 |
| `TerminalView::spawn_default` | `pub fn spawn_default(cx: &mut Context<Self>) -> Self` | `crates/ui-term/src/view.rs` L559 |
| `ChatPanel::new` | `pub fn new(render_mode: RenderMode, window: &mut Window, cx: &mut Context<Self>) -> Self` | `crates/ui-chat/src/panel.rs` L44 |
| `ChatPanel::set_backend` | `pub fn set_backend(&mut self, backend: Arc<dyn ChatBackend>, cx: &mut Context<Self>)` | `crates/ui-chat/src/panel.rs` L75 |
| 两个组件均实现 `Render` | ✅ 可作为 GPUI View 嵌入 | view.rs L1288, panel.rs L188 |

### 关键架构缺口 ❌

**问题**：Terminal 和 Chat 在 `tags.rs` 注册为 `ComponentKind::EntityRef`，但引擎中**没有处理 EntityRef kind 的通用 translator**。

- `StatelessComponentTranslator` 仅匹配 `Stateless` / `StatelessNoId` kind
- `StatefulComponentTranslator` 仅匹配 `Stateful` / `StatefulWithDelegate` kind
- `ActivityBarTranslator` 是唯一处理 EntityRef 的 translator，但它是 ActivityBar 专用的（通过 canonical tag "ActivityBar" 精确匹配）
- Terminal / Chat 没有专用 translator，也没有通用 EntityRef translator → RML 中写 `<Terminal>` / `<Chat>` 将无法编译

**根因**：前序会话在 tags.rs 中注册了 EntityRef kind，但未创建对应的 translator。

### 待完成 ❌

1. 创建通用 EntityRef translator（修复上述缺口）
2. demo/Cargo.toml 添加两个 crate 依赖
3. demo/src/main.rs 添加 `extern crate` 别名
4. 创建 terminal_case demo（.rml + .rml.rs）
5. 创建 chat_case demo（.rml + .rml.rs）
6. 在 cases/mod.rs 注册两个 demo
7. 添加 i18n key
8. 验证编译与测试

## Proposed Changes

### Step 1: 创建通用 EntityRef translator

**文件**: `crates/engine/src/compiler/translator/component/entity_ref.rs`（新建）

**What**: 创建一个通用的 EntityRef 组件 translator，处理所有 `ComponentKind::EntityRef` 组件（Terminal、Chat 及未来扩展）。

**Why**: 当前只有 ActivityBar 有专用 EntityRef translator。为 Terminal 和 Chat 各创建一个专用 translator 会产生重复代码（违反"拒绝 patchwork 框架代码"原则）。通用 translator 一次解决所有 EntityRef 组件的翻译问题。

**How**: 模仿 `activity_bar.rs` 的实现，但：
- `tag()` 返回 `"*entity-ref-component"`（通配符 tag）
- `matches()` 检查 `component_lookup_resolved(tag).kind == EntityRef`
- CSS 样式应用使用 canonical tag 名（而非硬编码 "ActivityBar"）
- codegen 生成 `self.<field>.as_ref().expect("init <field> in on_loaded").clone()` + CSS 样式

```rust
//! 通用 EntityRef 组件 translator
//!
//! 处理 `ComponentKind::EntityRef` 组件（Terminal、Chat 等）：
//! 从 ViewModel 的 `Option<Entity<T>>` 字段 clone Entity。
//! 通过 `ref="field_name"` 指令指定字段名。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct EntityRefComponentTranslator;

impl IRmlTranslator for EntityRefComponentTranslator {
    fn tag(&self) -> &'static str {
        "*entity-ref-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        matches!(
            tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
            Some(tags::ComponentKind::EntityRef)
        )
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let canonical = tags::canonical_tag(&elem.tag);

        let ref_name = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let name = ref_name.ok_or_else(|| CodegenError {
            message: format!(
                "EntityRef component <{}> requires `ref=\"field_name\"` directive",
                canonical
            ),
            span: Some(elem.span),
        })?;

        let mut code = format!(
            "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
            name, name
        );

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, &canonical, sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("*entity-ref-component", "EntityRef Component", ComponentCategory::Layout)
            .requires_ref(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(EntityRefComponentTranslator);
}
```

**注意**：`TranslatorMetadata::new(tag, label, category)` 是否有 `.requires_ref(true)` builder 方法需在实现时确认（`activity_bar.rs` L65 使用了此模式）。若不存在则省略。

**设计决策**：不迁移 ActivityBar 到通用 translator（surgical changes 原则 — 不重构可工作的代码）。ActivityBar 的专用 translator 通过 canonical tag 精确匹配优先于通配 translator，无冲突。

### Step 2: 注册 translator

**文件**: `crates/engine/src/compiler/translator/component/mod.rs`

**What**: 添加 `pub mod entity_ref;` 声明和 `entity_ref::register(registry);` 调用。

**具体改动**:
- L52 后添加: `pub mod entity_ref;`
- `register_all` 函数中 `activity_bar::register(registry);`（L160）后添加: `entity_ref::register(registry);`

### Step 3: 添加 extern crate 别名

**文件**: `demo/src/main.rs`

**What**: 在现有 `extern crate` 别名后添加两行。

```rust
extern crate rust_rml_ui_term as rml_ui_term;
extern crate rust_rml_ui_chat as rml_ui_chat;
```

**Why**: RML codegen 生成的代码引用 `rml_ui_term::TerminalView` 和 `rml_ui_chat::ChatPanel`，需要 extern crate 别名将 `rust-rml-ui-term` 包名映射为 `rml_ui_term` 短名。

### Step 4: 添加 demo 依赖

**文件**: `demo/Cargo.toml`

**What**: 在 `[dependencies]` 中添加两行（在 `rust-rml-ui = { workspace = true }` 后）：

```toml
rust-rml-ui-term = { workspace = true }
rust-rml-ui-chat = { workspace = true }
```

### Step 5: 添加 i18n key

**文件**: `demo/assets/i18n/zh-CN.json` 和 `demo/assets/i18n/en-US.json`

**What**: 添加两个 i18n key：

zh-CN.json:
```json
"case.terminal.title": "终端 Terminal",

"case.chat.title": "聊天 Chat",
```

en-US.json:
```json
"case.terminal.title": "Terminal",

"case.chat.title": "Chat",
```

### Step 6: 创建 Terminal demo case

**文件**: `demo/src/cases/terminal_case.rml`（新建）

```xml
<component>
    <CaseDocPage
        title={t("case.terminal.title")}
        description="嵌入式终端组件，基于 alacritty_terminal + portable-pty。支持 PTY 进程管理、VTE 解析渲染、键盘/鼠标输入、选择滚动等。EntityRef 组件，需在 on_loaded 中创建 Entity 后通过 ref 引用。"
        code-rml={rml_sample}
        code-rust={rust_sample}>
        <template slot="demo">
            <div class="demo-section">
                <h3>基础用法</h3>
                <p>下方为嵌入式终端，可直接输入命令：</p>
                <Terminal ref="term" style="height: 400px" />
            </div>
        </template>

        <template slot="api">
            <h3>Terminal</h3>
            <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
            <p>注：Terminal 为 EntityRef 组件，无 RML 属性绑定。配置通过 ElementRef API 在 ViewModel 中命令式操作。</p>
        </template>
    </CaseDocPage>
</component>
```

**文件**: `demo/src/cases/terminal_case.rml.rs`（新建）

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};
use rml_ui_term::TerminalView;

use crate::cases::common::{build_api_table, CaseDocPage};

#[contribute(
    host_id = "demo.shell",
    id = "components.terminal",
    kind = "case",
    group = "components",
    order = 39,
)]
#[component]
#[derive(Default)]
pub struct TerminalCase {
    /// EntityRef 组件字段：Option<Entity<TerminalView>>。
    /// 在 on_loaded 中通过 cx.new + spawn_default 创建。
    /// codegen 生成 self.term.as_ref().expect("init term in on_loaded").clone()
    pub term: Option<gpui::Entity<TerminalView>>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,
}

impl IContribution for TerminalCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.terminal.title")
    }
}

impl ILifecycle for TerminalCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));

        let term = cx.new(|cx| TerminalView::spawn_default(cx));
        self.term = Some(term);

        let (cols, rows) = build_api_table(&[
            ("ref", "string", "元素引用名，绑定到 ViewModel 同名字段（Option<Entity<TerminalView>>），如 ref=\"term\""),
            ("on_loaded 创建", "命令式 API", "在 on_loaded 中通过 cx.new(|cx| TerminalView::spawn_default(cx)) 创建 Entity，赋值到同名字段"),
            ("style / class", "string", "CSS 样式属性，如 style=\"height: 400px\" 确保终端有足够渲染空间"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TerminalCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("terminal_case.rml").to_string()
    }

    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("terminal_case.rml.rs").to_string()
    }
}
```

**关键设计点**：
- 字段类型为 `Option<gpui::Entity<TerminalView>>`（**非** `ElementRef<T>`），因为 EntityRef codegen 生成 `self.<field>.as_ref().expect(...)`，需要 `Option::as_ref()`。`ElementRef<T>` 没有 `as_ref()` 方法。
- `cx.new(|cx| TerminalView::spawn_default(cx))` 创建 `Entity<TerminalView>`。闭包内的 `cx` 是 `&mut Context<TerminalView>`，匹配 `spawn_default` 签名。
- `style="height: 400px"` 确保终端有渲染空间（否则 flex 布局可能折叠）。

### Step 7: 创建 Chat demo case

**文件**: `demo/src/cases/chat_case.rml`（新建）

```xml
<component>
    <CaseDocPage
        title={t("case.chat.title")}
        description="通用聊天组件，支持 IM 聊天与 AI 聊天快速定制。EntityRef 组件，需在 on_loaded 中创建 Entity 并注入 ChatBackend 实现。本 demo 使用 Echo 后端，输入消息后将收到回显响应。"
        code-rml={rml_sample}
        code-rust={rust_sample}>
        <template slot="demo">
            <div class="demo-section">
                <h3>基础用法（Echo 后端）</h3>
                <p>下方为聊天面板，输入消息后按回车发送：</p>
                <Chat ref="chat" style="height: 480px" />
            </div>
        </template>

        <template slot="api">
            <h3>Chat</h3>
            <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
            <p>注：Chat 为 EntityRef 组件，无 RML 属性绑定。通过 set_backend 注入 ChatBackend trait 实现决定消息处理逻辑。</p>
        </template>
    </CaseDocPage>
</component>
```

**文件**: `demo/src/cases/chat_case.rml.rs`（新建）

```rust
use std::sync::Arc;
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};
use rml_ui_chat::{ChatBackend, ChatError, ChatPanel, Conversation, RenderMode};

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
```

**关键设计点**：
- `ChatPanel::new` 需要 `window: &mut Window`。`cx.new(|cx| ChatPanel::new(mode, window, cx))` 闭包从外层 `on_loaded` 捕获 `window`。`Context` 和 `Window` 是不同对象，无借用冲突。
- `EchoBackend` 实现 `ChatBackend` trait，使用默认 `stream_message`（调用 `send_message` 后一次性推送完整响应）。
- `chat.update(cx, |panel, cx| panel.set_backend(...))` 在创建后立即注入后端。

### Step 8: 注册 demo case

**文件**: `demo/src/cases/mod.rs`

**What**: 在文件末尾（`pub mod edna;` 前）添加：

```rust
// Terminal / Chat 组件 demo
#[path = "terminal_case.rml.rs"]
pub mod terminal_case;
#[path = "chat_case.rml.rs"]
pub mod chat_case;
```

## Assumptions & Decisions

1. **字段类型 `Option<Entity<T>>` 而非 `ElementRef<T>`**：EntityRef codegen 生成 `self.<field>.as_ref().expect(...)`，`Option` 有 `as_ref()` 方法而 `ElementRef` 没有。这是 EntityRef（用户管理 Entity）与 Stateful ref（框架管理 Entity via `__rml_state.get_or_init_ref`）的本质区别。

2. **不迁移 ActivityBar 到通用 translator**：遵循 surgical changes 原则。ActivityBar 专用 translator 通过 canonical tag 精确匹配优先于通配 translator，无冲突。后续可选择性统一。

3. **EchoBackend 使用同步 `send_message`**：默认 `stream_message` 实现会调用 `send_message` 并一次性推送响应，适合 demo。真实 AI 后端应实现 `stream_message` 逐 token 推送。

4. **`style="height: ..."` 内联样式**：Terminal / Chat 作为 EntityRef 组件，其内联 style 通过 `apply_css_styles` 应用。高度确保组件在 flex 布局中有足够渲染空间。

5. **`TranslatorMetadata::requires_ref(true)`**：需在实现时确认此 builder 方法存在（`activity_bar.rs` L65 使用了 `.requires_ref(true)`）。若签名不同则调整。

6. **i18n key 格式**：遵循现有 `case.<name>.title` 格式，添加到 zh-CN.json 和 en-US.json。

## Verification Steps

```bash
# 1. 引擎编译（验证 translator 注册无语法错误）
cargo check -p rust-rml-engine

# 2. 引擎属性注册测试（验证 Terminal / Chat 注册一致性）
cargo test -p rust-rml-engine --lib props_registry::tests

# 3. ui-term crate 编译
cargo check -p rust-rml-ui-term

# 4. ui-chat crate 编译
cargo check -p rust-rml-ui-chat

# 5. demo 编译检查（验证 RML codegen + extern crate + 依赖）
cargo check -p rust-rml-demo

# 6. demo 完整构建（验证 build.rs RML 编译通过）
cargo build -p rust-rml-demo
```

**验证顺序**：1→2 先验证引擎改动正确，3→4 验证 crate 独立编译，5→6 验证 demo 集成。若 Step 5 失败，检查 RML codegen 输出（`target/debug/build/rust-rml-demo-*/out/` 下的生成代码）确认 EntityRef translator 生成了正确的 `self.term.as_ref().expect(...)` 代码。
