# RML LSP 语法服务器设计方案

> 参考微软 Roslyn「编译器即服务」（CaaS）理念，为 RML 框架设计高效、简单优雅的语法服务器。

---

## 1. 设计目标与 Roslyn 原则映射

| Roslyn CaaS 原则 | RML LSP 落地方式 |
|------------------|------------------|
| 编译器即服务（Compiler-as-a-Service） | 复用 `rust-rml-engine` 的 parser/validator/props_registry/tags/scanner 作为单一信源，LSP 仅作薄客户端 + 语义叠加层 |
| 不可变语法树（Immutable SyntaxTree） | `Arc<SyntaxTree>` 快照共享，编辑后整体替换为新 Arc（MVP 全量重解析，预留增量） |
| 语法/语义分离（Syntax vs Semantic） | `syntax` 模块产 SyntaxTree；`semantics` 模块产 SemanticModel（绑定路径/命令名校验） |
| Workspace 抽象 | `workspace` 模块持有 `HashMap<Url, Document>` + `ProjectIndex`（.rml ↔ .rml.rs 配对 + StructMetadata 缓存） |
| 惰性求值（Lazy Evaluation） | 语义诊断在文档变更时按需计算；补全/悬停查询时才遍历 AST |
| 零拷贝查询 | `Arc<str>` 源码快照 + `Arc<SyntaxTree>` 共享，多查询无 clone |

**MVP 功能范围**：补全（completion）+ 诊断（diagnostics）+ 悬停（hover）。

---

## 2. 当前状态分析

### 2.1 Stage A —— Engine CaaS 改造（已完成）

engine crate 已完成 span 追踪基础设施，LSP 可直接复用：

- [crates/engine/src/parser/span.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/span.rs) —— `Span { start, end }` 半开字节区间，`contains()`/`empty()`/`Default`
- [crates/engine/src/parser/tokenizer.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/tokenizer.rs) —— `Token.span`、`Token.end_line/end_column`、`RawAttribute.span`、`CharStream.byte_offset`
- [crates/engine/src/parser/ast.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs) —— `Element` 派生 `Default`，新增 `pub span: Span`；`Attribute` 保持不变（避免 70 处 codegen 站点爆炸）
- [crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs) —— `StructMetadata` 新增 `pub commands: Vec<String>`；新增纯函数 `parse_struct_metadata(source: &str)`（不读磁盘，供 LSP 处理未保存缓冲区）；第二遍扫描同时识别 `#[computed]` 与 `#[command]`

验证：engine 的 257 个 lib 测试全部通过。

### 2.2 Stage B —— LSP crate 骨架（文件已建，存在编译错误）

[crates/lsp/](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/) 下 27 个源文件全部创建，workspace [Cargo.toml](file:///d:/GitCode/RF/rust-gpui-rml/Cargo.toml) 已注册 `crates/lsp` 成员与 `rust-rml-lsp` workspace 依赖。

**架构分层（已落地）**：

```
crates/lsp/src/
├── lib.rs                      # 模块声明 + pub use run_server
├── main.rs                     # 二进制入口（--stdio）
├── server/                     # LSP 协议层
│   ├── connection.rs           # stdio 连接 + main_loop + ServerState + capabilities
│   ├── dispatch.rs             # 请求/通知路由 → handlers
│   └── conv.rs                 # Span ↔ LSP Range 换算（字节偏移 ↔ UTF-16 码元）
├── handlers/                   # LSP 方法处理（每个方法一个文件）
│   ├── completion.rs           # textDocument/completion → features::completion
│   ├── hover.rs                # textDocument/hover → features::hover
│   ├── diagnostics.rs          # 合并语法/校验/语义三类诊断
│   ├── sync.rs                 # didOpen/didChange/didSave/didClose
│   └── initialize.rs           # 预留扩展点
├── features/                   # 功能提供器（组合 syntax + semantics + engine registry）
│   ├── completion.rs           # 按光标上下文分派（TagName/AttributeName/BindingExpr/CommandName）
│   ├── hover.rs                # 标签文档拼装（tags + props_registry）
│   └── source.rs               # 补全数据源单一出口（CompletionSource）
├── workspace/                  # Roslyn Workspace 等价物
│   ├── workspace.rs            # 文档表 + 项目索引
│   ├── document.rs             # 单文档（uri + version + tree + semantic）
│   └── project_index.rs        # .rml ↔ .rml.rs 配对 + StructMetadata 缓存
├── syntax/                     # 不可变语法树快照
│   ├── tree.rs                 # SyntaxTree { source, root, errors, line_starts }
│   └── parse.rs                # parse_document() → Arc<SyntaxTree>
└── semantics/                  # Roslyn SemanticModel 等价物
    ├── model.rs                # SemanticModel::analyze_with_uri()
    ├── binder.rs               # 绑定路径/命令名校验
    └── diagnostics.rs          # SemanticDiagnostic { span, message, severity }
```

### 2.3 已知编译错误（3 处，需修复）

| # | 文件 | 行号 | 问题 | 修复 |
|---|------|------|------|------|
| 1 | [server/connection.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/connection.rs) | 62, 65 | `main_loop` 传 `&connection.sender` 给 dispatch，但 dispatch 已改为接收 `&Connection` | 改为 `&connection` |
| 2 | [handlers/sync.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/handlers/sync.rs) | 18, 38, 65 | 参数类型 `sender: &lsp_server::Sender`，但 `lsp_server::Sender` 在 0.7 版本是私有类型（E0603） | 改为 `conn: &Connection`，调用处 `sender` → `conn`，补充 `use lsp_server::Connection;` |
| 3 | [syntax/parse.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/syntax/parse.rs) | 19, 23 | `SyntaxTree::new(...)` 返回 `SyntaxTree`，但函数签名要求 `Arc<SyntaxTree>` | 用 `Arc::new(...)` 包裹 |

**根因**：`lsp-server` 0.7 未将 `Sender` 类型公开导出，`Response.result` 字段为 `Option<Value>`（非 `Result`）。dispatch.rs 已修正为 `&Connection` + `Option<Value>`，但 sync.rs 与 connection.rs 的调用点尚未同步。

---

## 3. 剩余实施方案

### 3.1 修复编译错误（Stage B 收尾）

#### 3.1.1 修复 `server/connection.rs`

**目标**：`main_loop` 中将 `&connection.sender` 改为 `&connection`。

```rust
// 第 62 行
dispatch::handle_request(req, state, &connection)?;
// 第 65 行
dispatch::handle_notification(not, state, &connection)?;
```

#### 3.1.2 修复 `handlers/sync.rs`

**目标**：三个 handler 函数签名从 `sender: &lsp_server::Sender` 改为 `conn: &Connection`，内部调用同步改名。

- 顶部新增 `use lsp_server::Connection;`
- `handle_did_open` / `handle_did_change` / `handle_did_save` 的第三参数：`sender: &lsp_server::Sender` → `conn: &Connection`
- 三处 `dispatch::send_diagnostics(&uri, diags, sender)` → `dispatch::send_diagnostics(&uri, diags, conn)`

#### 3.1.3 修复 `syntax/parse.rs`

**目标**：两个返回点用 `Arc::new(...)` 包裹。

```rust
pub fn parse_document(source: &str) -> Arc<SyntaxTree> {
    let source_arc: Arc<str> = Arc::from(source);
    match parser::parse(source) {
        Ok(root) => Arc::new(SyntaxTree::new(source_arc, Some(root), Vec::new())),
        Err(err) => Arc::new(SyntaxTree::new(source_arc, None, vec![err])),
    }
}
```

#### 3.1.4 迭代修复剩余编译错误

上述 3 处是已知错误。修复后运行 `cargo build -p rust-rml-lsp`，收集所有剩余错误并逐一修复。预期可能出现的次要问题：

- 未使用的 import（`use` 警告 → 删除）
- `lsp-types` 0.95 的 API 变体（如 `CompletionOptions` 字段差异）
- `MarkedString` / `HoverContents` 在 lsp-types 0.95 中的可用性

### 3.2 Stage C —— 功能验证（补全/诊断/悬停）

编译通过后，验证三个 MVP 功能的代码路径完整性：

#### 3.2.1 诊断链路（C1）

`didOpen` → `workspace.open_document()` → `parse_document()` + `SemanticModel::analyze_with_uri()` → `diagnostics::collect()` → `publishDiagnostics`

- 语法错误：`ParseError` 的 line/column 转 LSP Position（1-based → 0-based）
- 校验错误：`validator::validate()` 的 message 用根元素 span 定位
- 语义诊断：`binder::bind()` 检查绑定路径（observable_fields/computed_methods）与命令名（commands）

#### 3.2.2 补全链路（C2）

`completion` → `features::completion::complete()` → 按光标位置推断上下文 → `CompletionSource` 查询 engine 注册表

- `TagName`：builtin HTML + root + 扩展组件标签
- `AttributeName`：`props_registry::props_for()` / `shell_props_for()`
- `BindingExpr`：`StructMetadata.observable_fields` + `computed_methods`
- `CommandName`：`StructMetadata.commands`

#### 3.2.3 悬停链路（C3）

`hover` → `features::hover::hover()` → `find_element_at_offset()` 递归定位元素 → `format_tag_hover()` 拼装 markdown 文档

### 3.3 Stage D —— 端到端验证

#### 3.3.1 单元测试

```sh
cargo test -p rust-rml-lsp
```

验证 [server/conv.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/conv.rs) 的 4 个换算测试：
- `line_starts_simple`：行起始偏移计算
- `offset_to_pos_basic`：基本偏移→Position
- `offset_to_pos_multibyte`：中文多字节
- `roundtrip_pos_to_offset`：Position↔偏移往返

#### 3.3.2 集成编译验证

```sh
cargo build -p rust-rml-lsp
cargo build -p rust-rml-demo
```

确保 LSP crate 不破坏 workspace 其他 crate 的编译。

#### 3.3.3 功能正确性人工验证（可选）

用 demo 中的 .rml 文件验证命令识别。例如 [demo/src/cases/button_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml.rs) 的 `#[command] fn on_button_demo_click` 与对应 [button_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml) 的 `onclick={on_button_demo_click}`：

- 补全：在 `onclick="` 后应出现 `on_button_demo_click`
- 诊断：`onclick={unknown_fn}` 应触发 "event handler not registered" warning
- 悬停：悬停 `<Button>` 应显示组件属性清单

---

## 4. 关键设计决策

### 4.1 为何不重复实现 parser

Roslyn 的核心洞见：编译器与服务共享同一套语法树。RML 的 engine parser 已稳定（257 测试通过），重复实现会引入双源真值漂移。因此 LSP 的 `syntax/parse.rs` 直接调用 `engine::parser::parse()`，仅在外层包裹 `Arc<SyntaxTree>` 快照。

### 4.2 为何 `Element.span` 而非 `Attribute.span`

补全/悬停/诊断的最小定位粒度是元素级（"这个标签未知"、"这个绑定路径缺失"）。给 `Attribute` 加 span 会波及 70 处 codegen 站点，爆炸半径过大。`Element` 仅 16 处站点，且 `#[derive(Default)]` 让 codegen 用 `..Default::default()` 优雅补齐 span 字段。

### 4.3 为何用 `&Connection` 而非 `&Sender`

`lsp-server` 0.7 的 `Sender` 类型未公开导出（E0603）。`Connection` 持有 `sender: Sender<Message>` 公开字段，传 `&Connection` 既可发送消息又保留扩展性（未来可读 `receiver`），且与 rust-analyzer 的实践一致。

### 4.4 为何 MVP 用 `TextDocumentSyncKind::FULL`

增量同步（`INCREMENTAL`）需要在 `didChange` 中应用 patch 并重新计算受影响区间，复杂度高。.rml 文件通常 < 500 行，全量重解析毫秒级。MVP 用 FULL 换简单性，后续可升级。

### 4.5 为何语义诊断用 warning 而非 error

`binder.rs` 检查绑定路径时，无法区分 ViewModel 字段与闭包变量/循环变量（`each={item in items}` 的 `item`）。误报 error 会干扰用户。降级为 warning，待后续引入作用域分析后再升级。

---

## 5. 模块职责边界（高内聚低耦合）

| 模块 | 职责 | 不做的事 |
|------|------|----------|
| `server/connection` | stdio 传输、initialize 握手、main_loop | 不解析消息语义 |
| `server/dispatch` | 按 method 字符串路由 | 不处理业务逻辑 |
| `server/conv` | 偏移↔Position 换算 | 不依赖 LSP 方法语义 |
| `handlers/*` | 单个 LSP 方法的参数解析 + 调用 features | 不直接访问 engine |
| `features/*` | 功能逻辑（补全推断、悬停拼装） | 不直接读写 workspace 状态 |
| `workspace/*` | 文档表 + 项目索引管理 | 不做语义分析 |
| `syntax/*` | 语法树快照 | 不做语义校验 |
| `semantics/*` | 绑定/命令校验 | 不直接调用 engine parser |

---

## 6. 验证步骤清单

1. **修复 3 处已知编译错误**（见 §3.1）
2. `cargo build -p rust-rml-lsp` —— 迭代修复所有剩余编译错误
3. `cargo test -p rust-rml-lsp` —— conv 换算测试通过
4. `cargo build -p rust-rml-demo` —— 不破坏 workspace
5. `cargo test -p rust-rml-engine --lib` —— engine 测试仍通过（确认 Stage A 改动无回归）
6. `cargo clippy -p rust-rml-lsp` —— 无 warning（符合项目规范）

---

## 7. 后续演进方向（非 MVP）

- **增量同步**：`TextDocumentSyncKind::INCREMENTAL` + 区间级重解析
- **定义跳转**：`textDocument/definition` —— .rml 绑定路径 → .rml.rs 字段定义
- **引用查找**：`textDocument/references` —— 字段/命令的所有引用点
- **重命名**：`textDocument/rename` —— 跨 .rml/.rml.rs 同步重命名
- **格式化**：`textDocument/formatting` —— RML 源码格式化
- **作用域分析**：区分 ViewModel 字段与循环变量，升级语义诊断精度
- **项目级索引**：`initialize` 时扫描 root_uri 下所有 .rml/.rml.rs 建立全局索引
