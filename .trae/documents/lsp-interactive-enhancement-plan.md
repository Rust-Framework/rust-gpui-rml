# LSP 交互增强计划（续）

## 概述

接续上一轮会话，完成用户提出的 6 项 LSP 交互问题中的剩余工作。

### 用户原始诉求

1. hover 灵敏度不足 —— 移入 token 不立即触发 quickinfo
2. `.rs` / `.rml.rs` quickinfo 不触发、跳转定义右键菜单无效
3. 右键菜单应包含格式化、重命名等操作
4. LSP 案例中 header 左侧显示面包屑导航（纳入 ui crate 组件支持），对接语法服务
5. 提供代码折叠语法服务
6. 以上覆盖 `.rs` / `.rml` / `.rml.rs`

### 上一轮已完成（经验证仍存在）

| 项 | 状态 | 关键文件 |
|----|------|---------|
| hover 灵敏度 | 已修复（RaHost 未就绪时返回 "Loading..." HoverInfo） | `crates/lsp/src/rust/adapter.rs` |
| `.rs`/`.rml.rs` quickinfo + 跳转 | 已修复（doctype 路由：`is_rust_source` 排除 `.rml.rs`，`is_rust_file` 匹配任意 `.rs`） | `crates/lsp/src/server/doctype.rs` |
| 代码折叠 LSP server 端 | 已完成（缩进策略 + handler + dispatch + capability + client 方法） | `crates/lsp/src/features/fold.rs`、`handlers/folding_range.rs`、`server/dispatch.rs`、`server/connection.rs`、`crates/rml/src/lsp_client.rs` |
| CodeEditor `context-menu` codegen | 已完成（解析 `context-menu` 属性 → 生成 `.context_menu(closure)`） | `crates/engine/src/compiler/code_editor/gen.rs` |

### 剩余工作

- **Step 2b**：RML 新增 `on-action` 事件属性支持（框架层）
- **Step 2c**：Demo 定义 Action 类型 + action handler + `build_editor_menu` 方法
- **Step 2d**：Demo RML 模板加 `context-menu` + `on-action` 属性
- **Step 3**：ui crate 新增 `Breadcrumb` 组件 + Demo 对接 documentSymbol
- **Step 4**：构建 + 测试验证

---

## 当前状态分析

### 1. 代码折叠客户端侧（无需额外工作）

经核查 gpui-component `063e55b`：

- `InputMode::code_editor()` 默认 `folding: true`（`crates/ui/src/input/mode.rs:79`）
- `InputState::update_fold_candidates()` 是 **私有方法**，通过 tree-sitter 语法树提取折叠区域（`state.rs:2567`）
- `DisplayMap::set_fold_candidates()` 虽为 pub，但 `display_map` 字段为 `pub(super)`，外部无法注入 LSP 折叠区域
- **结论**：CodeEditor 默认已启用 tree-sitter 折叠。`.rs`/`.rml.rs` 由已注册的 rust grammar 驱动折叠；`.rml` 由 rml grammar 驱动。LSP server 端的 `textDocument/foldingRange` 作为语法服务对外提供（未来 gpui-component 开放注入 API 时可对接），客户端侧无需额外代码。

### 2. 右键菜单 — Action 注册机制

经核查 GPUI `1d217ee`：

- `NativeMenu::menu(label, Box<dyn Action>)` 分发 Action 到焦点链
- `Div::on_action<A: Action>(f: Fn(&A, &mut Window, &mut App))` 元素级监听
- `App::on_action<A: Action>(f: Fn(&A, &mut App))` 全局监听（Bubble phase）
- `Context::on_action(action_type, &mut Window, listener)` 需 `&mut Window`

RML 现状：
- `on-*` 前缀属性由 parser 归类为 `Attribute::Event`（`parser/mod.rs:260`）
- `event::apply_event(name, handler, ctx)` 映射到 GPUI `.on_click()` 等方法（`compiler/event.rs`）
- `on-action` 不在 `event_binding` 表中 → 当前返回空字符串被忽略
- **方案**：在 `event.rs` 中为 `on_action` 增加专属 codegen，解析值为逗号分隔的 `ActionType:method` 对，生成多个 `.on_action::<Type>(cx.listener(...))` 调用

### 3. Breadcrumb 组件

经核查：
- ui crate 现有组件：activity_bar / alert_dialog / avatar / card / menu / status_bar / tab / table / tree（无 breadcrumb）
- 组件注册模式：`tags.rs::normalize_component_tag` 匹配 PascalCase 标签名 → `ComponentTag { ctor_path, kind, container }`
- 绑定属性 codegen 模式：`description_list/setters.rs::bind_setter` 处理 `items={expr}` → `.children(...)`
- `Node::Interpolation` 仅生成 `format!("{}", expr)` 文本，**不支持元素插值** → Breadcrumb 必须注册为 RML 组件标签

### 4. LSP documentSymbol 数据源

`crates/lsp/src/rust/adapter.rs:494` `document_symbol()` 已实现：
- 调用 `analysis.file_structure()` 获取 `Vec<StructureNode>`
- `build_document_symbols()` 递归构建嵌套 `DocumentSymbol`（含 `children`、`range`、`selection_range`、`name`、`kind`、`detail`）
- 客户端 `LspClient::document_symbol(uri)` 已存在

Breadcrumb 数据流：documentSymbol 响应 → 解析为 `Vec<DocumentSymbol>` → 按光标位置遍历嵌套树 → 取根到当前符号的路径 → 渲染为 Breadcrumb items。

---

## 实施步骤

### Step 2b：RML 新增 `on-action` 事件属性支持

**文件**：`crates/engine/src/compiler/event.rs`

**修改内容**：

1. 在 `apply_event` 函数开头增加 `on_action` 特殊分支：
   - 当 `name == "on_action"` 时，委托给新函数 `apply_action_event(handler)`
   - 返回多个 `.on_action::<Type>(cx.listener(...))` 拼接字符串

2. 新增 `apply_action_event(handler: &EventHandler) -> String`：
   - 从 `handler` 提取值字符串（`EventHandler::MethodName(s)` / `Ident(s)` / `WithArgs(s, _)`）
   - 按逗号分隔，每项按冒号拆分 `ActionType:method`
   - 对每对生成：
     ```rust
     .on_action::<ActionType>(cx.listener(move |this, _action: &ActionType, _window, cx| {
         this.method(_action, _window, cx);
     }))
     ```
   - 解析失败时返回空字符串（容错）

3. 新增单元测试：
   - 单个 `Type:method` 对
   - 多个对（逗号分隔，含空格）
   - 空值返回空字符串
   - 格式错误返回空字符串

**约束**：
- `on-action` 属性值为静态字符串（`Attribute::Event` + `EventHandler::MethodName`）
- Action 类型必须在视图模块作用域可见（demo 在 `code_editor_tab.rml.rs` 定义）
- handler 方法签名：`fn method(&mut self, action: &ActionType, window: &mut Window, cx: &mut Context<Self>)`

### Step 2c：Demo Action 类型 + action handler + build_editor_menu

**文件**：`demo/src/lsp/code_editor_tab.rml.rs`

**修改内容**：

1. 顶部新增 Action 类型定义（5 个）：
   ```rust
   #[derive(Action, Clone, PartialEq, Deserialize)]
   #[action(namespace = code_editor, no_json)]
   struct FormatDocument;

   #[derive(Action, Clone, PartialEq, Deserialize)]
   #[action(namespace = code_editor, no_json)]
   struct RenameSymbol;

   #[derive(Action, Clone, PartialEq, Deserialize)]
   #[action(namespace = code_editor, no_json)]
   struct FindReferences;

   #[derive(Action, Clone, PartialEq, Deserialize)]
   #[action(namespace = code_editor, no_json)]
   struct GoToDefinition;

   #[derive(Action, Clone, PartialEq, Deserialize)]
   #[action(namespace = code_editor, no_json)]
   struct ShowDocumentSymbols;
   ```

2. 提取现有 `#[command]` 方法体为 `do_*` 方法（无 `&ClickEvent` 参数）：
   - `do_format_document(&mut self, cx: &mut Context<Self>)`
   - `do_rename_symbol(&mut self, cx: &mut Context<Self>)`
   - `do_find_references(&mut self, cx: &mut Context<Self>)`
   - `do_show_document_symbols(&mut self, cx: &mut Context<Self>)`
   - 原 `#[command]` 方法改为调用对应 `do_*`

3. 新增 `do_goto_definition(&mut self, cx: &mut Context<Self>)`：
   - 调用 `client.lsp().definition(&uri, position)`
   - 解析 `Location` 响应
   - 状态栏显示 `goto: file:line:col` 摘要

4. 新增 5 个 action handler 方法（签名匹配 `on-action` codegen）：
   ```rust
   fn on_format_action(&mut self, _: &FormatDocument, _: &mut Window, cx: &mut Context<Self>) {
       self.do_format_document(cx);
   }
   // ...其余 4 个同理
   ```

5. 新增 `build_editor_menu` 方法：
   ```rust
   pub fn build_editor_menu(
       &mut self,
       menu: NativeMenu,
       _window: &mut Window,
       _cx: &mut Context<Self>,
   ) -> NativeMenu {
       menu
           .menu("Format Document", Box::new(FormatDocument))
           .menu("Rename Symbol", Box::new(RenameSymbol))
           .separator()
           .menu("Go to Definition", Box::new(GoToDefinition))
           .menu("Find References", Box::new(FindReferences))
           .separator()
           .menu("Show Document Symbols", Box::new(ShowDocumentSymbols))
   }
   ```

6. 新增 imports：`gpui::{Action, Deserialize, Window}`、`gpui_component::native_menu::NativeMenu`

7. 新增 Breadcrumb 相关字段与方法（见 Step 3）

### Step 2d：RML 模板更新

**文件**：`demo/src/lsp/code_editor_tab.rml`

**修改内容**：

```rml
<component>
    <div v-flex="" class="lsp-editor-pane"
         on-action="FormatDocument:on_format_action, RenameSymbol:on_rename_action, FindReferences:on_find_references_action, GoToDefinition:on_goto_definition_action, ShowDocumentSymbols:on_show_document_symbols_action">
        <div h-flex="" class="lsp-toolbar">
            <Breadcrumb items={breadcrumb_items} />
            <ButtonGroup>
                <Button label="Format" ghost="" size="small" on-click="on_format_document" />
                <Button label="Rename" ghost="" size="small" on-click="on_rename_symbol" />
                <Button label="References" ghost="" size="small" on-click="on_find_references" />
                <Button label="Symbols" ghost="" size="small" on-click="on_show_document_symbols" />
            </ButtonGroup>
        </div>
        <div class="lsp-editor-area">
            <CodeEditor h-full="" context-menu="build_editor_menu" />
        </div>
    </div>
</component>
```

**关键点**：
- `on-action` 在根 `<div>` 上注册 5 个 action 监听器
- `context-menu="build_editor_menu"` 在 CodeEditor 上设置右键菜单构建器
- `<Breadcrumb items={breadcrumb_items} />` 在 toolbar 左侧渲染面包屑

### Step 3：Breadcrumb 组件 + Demo 对接

#### 3a：ui crate 新增 Breadcrumb 组件

**新文件**：`crates/ui/src/components/breadcrumb.rs`

**组件设计**：

```rust
use gpui::{App, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::{h_flex, IconName, ActiveTheme};

/// 面包屑项
#[derive(Clone)]
pub struct BreadcrumbItem {
    pub label: SharedString,
    pub icon: Option<IconName>,
}

impl BreadcrumbItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self { label: label.into(), icon: None }
    }
    pub fn icon(mut self, icon: IconName) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// 面包屑导航组件（VSCode 风格）
#[derive(IntoElement)]
pub struct Breadcrumb {
    items: Vec<BreadcrumbItem>,
}

impl Breadcrumb {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }
    pub fn items(mut self, items: Vec<BreadcrumbItem>) -> Self {
        self.items = items;
        self
    }
}

impl Default for Breadcrumb {
    fn default() -> Self {
        Self::new()
    }
}

impl gpui::RenderOnce for Breadcrumb {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        h_flex()
            .items_center()
            .gap_1()
            .text_xs()
            .text_color(theme.muted_foreground)
            .children(self.items.into_iter().enumerate().flat_map(|(i, item)| {
                let is_last = i == self.items.len().saturating_sub(1);
                let mut elements = Vec::new();
                // 项目（icon + label）
                let mut item_el = h_flex().items_center().gap_1();
                if let Some(icon) = item.icon {
                    item_el = item_el.child(gpui_component::Icon::new(icon).size(px(12.)));
                }
                elements.push(item_el.child(item.label.clone()));
                // 分隔符（非最后一项）
                if !is_last {
                    elements.push(
                        div().text_color(theme.border)
                            .child(SharedString::from("›"))
                    );
                }
                elements
            }))
    }
}
```

**修改文件**：
- `crates/ui/src/components/mod.rs`：新增 `pub mod breadcrumb;` + `pub use breadcrumb::{Breadcrumb, BreadcrumbItem};`
- `crates/ui/src/lib.rs`：在 `pub use components::{...}` 中追加 `Breadcrumb, BreadcrumbItem`

#### 3b：RML 注册 Breadcrumb 组件标签

**修改文件**：`crates/engine/src/tags.rs`

在 `normalize_component_tag` 中新增：
```rust
"Breadcrumb" | "breadcrumb" => Some(ComponentTag {
    ctor_path: "rml_ui::Breadcrumb",
    kind: ComponentKind::StatelessNoId,
    container: false,
}),
```

#### 3c：Breadcrumb `items` 绑定 setter

**新文件**：`crates/engine/src/compiler/breadcrumb.rs`（或直接在 `component.rs` 的通用 `component_bind_setter` 中处理）

**方案**：在 `crates/engine/src/compiler/component.rs::component_bind_setter` 中为 `"Breadcrumb"` tag 增加匹配：
```rust
"Breadcrumb" => match name {
    "items" => Some(format!(".items({}.clone())", rust_expr)),
    _ => None,
},
```

同理在 `component_static_setter` 中处理（虽然 `items` 一般用绑定，但防御性处理）。

#### 3d：Demo 对接 Breadcrumb 数据流

**文件**：`demo/src/lsp/code_editor_tab.rml.rs`

**修改内容**：

1. 新增字段：
   ```rust
   use gpui_component::input::InputEvent;
   use gpui_component::IconName;
   use rml_ui::{BreadcrumbItem, NativeMenu};
   use lsp_types::DocumentSymbol;

   pub struct CodeEditorTab {
       editor_state: Option<Entity<InputState>>,
       language_client: Option<Arc<LanguageClient>>,
       uri: Option<Uri>,
       // 新增
       document_symbols: Vec<DocumentSymbol>,
       breadcrumb_items: Vec<BreadcrumbItem>,
   }
   ```

2. `new()` 中订阅 InputEvent::Change + 光标移动 → 更新面包屑：
   ```rust
   // 订阅 editor_state 变化以更新面包屑
   let editor_state_for_observe = editor_state.clone();
   cx.observe(&editor_state, move |this, _, cx| {
       this.update_breadcrumb(cx);
   }).detach();
   ```
   
   实际上 `cx.observe` 在 `new()` 闭包内无法直接调用 `this.update_breadcrumb`。改用 `cx.subscribe` 监听 `InputEvent`：
   ```rust
   // 在 new() 的 cx.new 闭包内
   cx.subscribe(&editor_state, |this, state, event: &InputEvent, cx| {
       match event {
           InputEvent::Change { .. } | InputEvent::CursorChange { .. } => {
               this.update_breadcrumb(cx);
           }
           _ => {}
       }
   }).detach();
   ```
   
   **注**：需确认 `InputEvent` 是否有 `CursorChange` 变体。若无，则监听 `Change` 事件 + 在 `update_breadcrumb` 内读取当前光标位置。若 `Change` 仅文本变化不包含光标移动，需在 `editor_state` 上额外订阅光标位置变化（gpui-component 可能未暴露此事件，则降级为：仅文本变化时更新 + 文档符号加载时更新）。

3. 文档加载后异步拉取 documentSymbol：
   ```rust
   fn fetch_document_symbols(&mut self, cx: &mut Context<Self>) {
       let (client, uri) = match (&self.language_client, &self.uri) {
           (Some(c), Some(u)) => (c.clone(), u.clone()),
           _ => return,
       };
       let rx = client.lsp().document_symbol(&uri);
       cx.spawn(async move |this, cx| {
           match rx.recv() {
               Ok(Ok(value)) => {
                   let symbols = parse_document_symbols(&value);
                   let _ = this.update(cx, |this, cx| {
                       this.document_symbols = symbols;
                       this.update_breadcrumb(cx);
                   });
               }
               Ok(Err(e)) => log::warn!("documentSymbol error: {e}"),
               Err(e) => log::warn!("documentSymbol channel: {e}"),
           }
           Ok::<(), anyhow::Error>(())
       }).detach();
   }
   ```

4. `update_breadcrumb` 方法：
   ```rust
   fn update_breadcrumb(&mut self, cx: &mut Context<Self>) {
       let Some(position) = self.current_position(cx) else {
           self.breadcrumb_items = Vec::new();
           return;
       };
       let path = find_symbol_path(&self.document_symbols, &position);
       self.breadcrumb_items = path.into_iter()
           .map(|s| BreadcrumbItem::new(s.name.clone()))
           .collect();
       cx.notify();
   }
   ```

5. 新增辅助函数：
   ```rust
   /// 从嵌套 DocumentSymbol 树中查找包含 position 的根到叶路径
   fn find_symbol_path(symbols: &[DocumentSymbol], position: &Position) -> Vec<DocumentSymbol> {
       for sym in symbols {
           if range_contains(&sym.range, position) {
               let mut path = vec![sym.clone()];
               if let Some(children) = &sym.children {
                   if let Some(child_path) = find_symbol_path(children, position).into_iter().next() {
                       // 递归找最深路径
                       let mut deeper = find_symbol_path(children, position);
                       if !deeper.is_empty() {
                           path.append(&mut deeper);
                       }
                   }
               }
               return path;
           }
       }
       Vec::new()
   }
   
   fn range_contains(range: &lsp_types::Range, pos: &Position) -> bool {
       let start_ok = pos.line > range.start.line
           || (pos.line == range.start.line && pos.character >= range.start.character);
       let end_ok = pos.line < range.end.line
           || (pos.line == range.end.line && pos.character <= range.end.character);
       start_ok && end_ok
   }
   
   fn parse_document_symbols(value: &serde_json::Value) -> Vec<DocumentSymbol> {
       let response: DocumentSymbolResponse = serde_json::from_value(value.clone())
           .unwrap_or(DocumentSymbolResponse::Nested(Vec::new()));
       match response {
           DocumentSymbolResponse::Flat(_) => Vec::new(), // flat 格式不支持嵌套，跳过
           DocumentSymbolResponse::Nested(symbols) => symbols,
       }
   }
   ```

6. `new()` 中调用 `fetch_document_symbols(cx)`（在 `cx.new` 闭包末尾）

### Step 4：构建 + 测试验证

1. `cargo build -p rust-rml-engine` — 验证 event.rs / tags.rs / component.rs 改动
2. `cargo build -p rust-rml-ui` — 验证 breadcrumb 组件
3. `cargo build -p rust-rml-lsp` — 验证 fold handler（已完成）
4. `cargo build -p rust-rml-demo` — 验证 demo 集成
5. `cargo test -p rust-rml-engine` — 验证 event.rs 新增测试
6. `cargo build` — 全工作区构建
7. `cargo test` — 全工作区测试

---

## 假设与决策

### 假设

1. hover 灵敏度、`.rs`/`.rml.rs` quickinfo 已在上一轮修复（经验证代码仍在）
2. 代码折叠客户端侧由 gpui-component 内置 tree-sitter 驱动，默认 `folding: true`，无需额外代码
3. `InputEvent` 至少有 `Change` 变体（已确认）；光标移动事件若不存在，降级为文本变化时更新面包屑
4. Action 类型在 `code_editor_tab.rml.rs` 模块内定义，`on-action` codegen 引用的类型名在作用域内可见
5. `DocumentSymbolResponse::Nested` 包含嵌套 children（rust adapter 已确认生成嵌套结构）

### 决策

1. **`on-action` 属性语法**：`on-action="Type1:method1, Type2:method2"`（逗号分隔，冒号配对）
   - 理由：单属性支持多 action 类型注册，避免重复属性名
   - codegen 在 `event.rs` 中特殊处理，不污染通用事件映射表

2. **Action handler 方法签名**：`fn method(&mut self, action: &ActionType, window: &mut Window, cx: &mut Context<Self>)`
   - 理由：匹配 `cx.listener` 闭包签名，与 GPUI 惯例一致

3. **Breadcrumb 作为 RML 组件标签**：注册 `<Breadcrumb items={expr} />`
   - 理由：RML `Node::Interpolation` 仅支持文本插值（`format!("{}", expr)`），不支持元素插值
   - 组件遵循 `#[derive(IntoElement)] + RenderOnce` 模式（与 Avatar 一致）

4. **Breadcrumb 数据源**：复用现有 `documentSymbol` LSP 服务
   - 理由：rust adapter 已实现嵌套 `DocumentSymbol` 生成，无需新增 LSP 方法

5. **`do_*` 方法提取**：原 `#[command]` 方法体提取为 `do_*`，供 action handler 和 command 共用
   - 理由：DRY，避免逻辑重复

6. **`build_editor_menu` 不依赖 `&mut Context`**：方法签名 `fn(&mut self, NativeMenu, &mut Window, &mut Context<Self>) -> NativeMenu`
   - 理由：codegen 生成 `.context_menu(move |menu, w, c| __view.update(c, |this, cx| this.build_editor_menu(menu, w, cx)))`，闭包内已有 `&mut Context<Self>`

---

## 验证步骤

| 步骤 | 命令 | 预期结果 |
|------|------|---------|
| 1 | `cargo build -p rust-rml-engine` | event.rs / tags.rs / component.rs 编译通过 |
| 2 | `cargo test -p rust-rml-engine` | 新增 `on-action` 单元测试通过 |
| 3 | `cargo build -p rust-rml-ui` | breadcrumb.rs 编译通过 |
| 4 | `cargo build -p rust-rml-demo` | demo 集成编译通过 |
| 5 | `cargo build` | 全工作区编译通过 |
| 6 | `cargo test` | 全工作区测试通过 |

### 运行时验证（手动）

1. 启动 demo，打开 LSP 案例
2. 打开 `.rs` 文件 → 编辑器显示代码折叠标记（tree-sitter 驱动）
3. 右键编辑器 → 显示菜单：Format Document / Rename Symbol / Go to Definition / Find References / Show Document Symbols
4. 点击 "Format Document" → 代码被格式化
5. 选中符号后右键 → "Go to Definition" → 状态栏显示跳转位置
6. header 左侧显示面包屑：`crate > module > struct > method`（随光标移动更新）
7. 打开 `.rml` 文件 → 同样有折叠 + 右键菜单 + 面包屑
8. 打开 `.rml.rs` 文件 → 同样有折叠 + 右键菜单 + 面包屑

---

## 文件清单

### 新增

- `crates/ui/src/components/breadcrumb.rs` — Breadcrumb 组件

### 修改

- `crates/engine/src/compiler/event.rs` — 新增 `on_action` codegen
- `crates/engine/src/tags.rs` — 注册 `Breadcrumb` 标签
- `crates/engine/src/compiler/component.rs` — Breadcrumb `items` setter
- `crates/ui/src/components/mod.rs` — 导出 breadcrumb 模块
- `crates/ui/src/lib.rs` — re-export Breadcrumb / BreadcrumbItem
- `demo/src/lsp/code_editor_tab.rml.rs` — Action 类型 + handlers + build_editor_menu + breadcrumb 数据流
- `demo/src/lsp/code_editor_tab.rml` — on-action + context-menu + Breadcrumb 标签

### 已完成（无需改动）

- `crates/lsp/src/features/fold.rs`
- `crates/lsp/src/handlers/folding_range.rs`
- `crates/lsp/src/handlers/mod.rs`
- `crates/lsp/src/features/mod.rs`
- `crates/lsp/src/server/dispatch.rs`
- `crates/lsp/src/server/connection.rs`
- `crates/rml/src/lsp_client.rs`
- `crates/engine/src/compiler/code_editor/gen.rs`
- `crates/lsp/src/rust/adapter.rs`（hover loading hint + document_symbol + folding_ranges）
