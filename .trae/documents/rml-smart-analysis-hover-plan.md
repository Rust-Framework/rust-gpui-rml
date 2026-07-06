# RML 智能分析能力 — Hover 阶段实施计划

## 摘要

本计划为 RML 语言服务器增加智能 hover 能力,覆盖四类场景:

1. **RML 标签 hover** — 反查 ra_ap_ide 获取组件 Rust 源码文档注释(`//!` / `///`)
2. **i18n hover** — `t("key")` 显示所有 locale 的本地化定义
3. **CSS hover** — `class="xxx"` 显示样式定义(应用层 + 页面层)
4. **属性 hover** — 属性 hover 时附加组件文档

资源发现采用**自动扫描 workspace** 策略,property docs 从 **Rust 源码提取**(长期方案)。
本阶段仅实现 **hover**,goto-def 留待后续阶段。

---

## 现状分析

### 当前 hover 实现

- `crates/lsp/src/features/hover.rs::hover(uri, position, workspace)` — 仅接收 `workspace`,无 rust_query/i18n/css 索引
- `format_tag_hover(elem)` — 对所有组件标签返回硬编码字符串 `"gpui-component extension."`
- `format_attribute_name_hover` / `format_attribute_value_hover` — 仅显示属性名/值/类别,无文档
- `crates/lsp/src/handlers/hover.rs` — `.rs` 走 `rust_query.hover()`,`.rml` 走 `hover::hover()`(无 rust_query)

### 可复用基础设施

- `RustSemanticQuery` trait 已具备:
  - `find_struct(name) -> Option<SymbolLocation>` — 全 workspace 精确匹配 struct
  - `hover(uri, pos) -> Option<HoverInfo>` — 提取 Markdown 文档(含 `///`/`//!`)
- `RaAdapter` 实现:
  - `find_struct` 用 `symbol_index::Query::new(name).exact().only_types()`,过滤 `SymbolKind::Struct`
  - `hover` 用 `HoverConfig { documentation: true, format: Markdown }`
- `rust_rml_engine::css::parser::parse` + `css::ast::{StyleSheet, Rule, Selector, Declaration}` 可复用
- `crates/core/src/i18n.rs::flatten_json_value` 逻辑可参考(但 lsp crate 不依赖 core,需本地实现)

### 约束

- `crates/lsp/Cargo.toml` 依赖 `rust-rml-engine` 但**不依赖** `rust-rml-core`(避免拉入 gpui)
- CSS `Rule` 结构**无行号字段**,Phase 1 hover 跳过行号,仅展示声明内容
- `ComponentTag { ctor_path, kind, container }` **无 doc 字段**,需通过 `ctor_path` 反查

### RML 检测模式(已验证)

- `{t("login.title")}` → `Node::Interpolation { expr: "t(\"login.title\")", span }`
- `label={t("login.submit")}` → `Attribute::Bind { name: "label", expr }`
- `class="login"` → `Attribute::Static { name: "class", value: "login" }`
- `class="case-pane doc-pane"` → 多类名,空格分隔

### 资源文件示例

- `demo/assets/i18n/zh-CN.json` — 扁平 key-value,如 `"login.title": "登录 RML Demo"`
- `demo/assets/styles.css` — 类定义,如 `.case-host { flex: 1; ... }`

---

## 实施步骤

### Step 1.1 基础设施 — Workspace 资源扫描器

**新建文件**: `crates/lsp/src/workspace/assets.rs`

实现两个索引结构:

```rust
// I18nIndex: 扫描 **/i18n/*.json
pub struct I18nIndex {
    /// key → 各 locale 的翻译
    entries: HashMap<String, Vec<I18nEntry>>,
}

pub struct I18nEntry {
    pub locale: String,   // 文件名 stem,如 "zh-CN"
    pub value: String,
    pub file_uri: Url,
}

impl I18nIndex {
    pub fn new() -> Self;
    /// 扫描 root_path 下所有 **/i18n/*.json 文件
    pub fn scan(&mut self, root_path: &Path);
    /// 查询 key 的所有 locale 翻译
    pub fn lookup(&self, key: &str) -> Option<&Vec<I18nEntry>>;
}

// CssIndex: 扫描 **/*.css
pub struct CssIndex {
    /// class 名 → 各文件中的声明
    entries: HashMap<String, Vec<CssClassEntry>>,
}

pub struct CssClassEntry {
    pub declarations: Vec<(String, String)>, // (property, value 文本)
    pub file_uri: Url,
}

impl CssIndex {
    pub fn new() -> Self;
    /// 扫描 root_path 下所有 **/*.css 文件
    pub fn scan(&mut self, root_path: &Path);
    /// 查询 class 名的所有声明
    pub fn lookup(&self, class_name: &str) -> Option<&Vec<CssClassEntry>>;
}
```

**关键实现细节**:

- JSON 扁平化:本地实现 `flatten_json_value`(复制 `crates/core/src/i18n.rs` L98-L126 逻辑,不依赖 core)
- CSS 解析:调用 `rust_rml_engine::css::parser::parse`,遍历 `Rule.selectors`,提取 `Selector::Class(name)` 的 declarations
- 复合选择器(`.button.primary`)拆分:递归 `Selector::Compound(parts)` 提取每个 `Class(name)`
- 递归目录扫描:`std::fs::read_dir` 递归,匹配 `i18n/*.json` 与 `*.css`
- 错误处理:单文件解析失败记录日志跳过,不中断扫描

**修改文件**: `crates/lsp/src/workspace/mod.rs`

```rust
pub mod assets;  // 新增
pub mod document;
pub mod project_index;

// 在 Workspace 结构或独立位置导出 I18nIndex / CssIndex
pub use assets::{CssIndex, CssClassEntry, I18nIndex, I18nEntry};
```

**修改文件**: `crates/lsp/src/server/connection.rs`

`ServerState` 添加两个字段:

```rust
pub struct ServerState {
    pub workspace: Workspace,
    pub rust_query: Box<dyn RustSemanticQuery>,
    #[cfg(feature = "rust-backend")]
    pub ra_host: Arc<RaHost>,
    pub root_path: Option<PathBuf>,
    pub i18n_index: I18nIndex,   // 新增
    pub css_index: CssIndex,     // 新增
    pub shutdown_requested: bool,
}
```

`ServerState::new()` 初始化为空索引(`I18nIndex::new()` / `CssIndex::new()`)。

**修改文件**: `crates/lsp/src/server/dispatch.rs`

在 `"initialized"` 通知处理中,`start_rust_backend(state)` 之后增加资源扫描:

```rust
"initialized" => {
    log::debug!("client initialized");
    start_rust_backend(state);
    scan_workspace_assets(state);  // 新增
}
```

新增函数:

```rust
fn scan_workspace_assets(state: &mut ServerState) {
    if let Some(root) = state.root_path.clone() {
        log::info!("scanning workspace assets at {:?}", root);
        state.i18n_index.scan(&root);
        state.css_index.scan(&root);
        log::info!(
            "asset scan done: {} i18n keys, {} css classes",
            state.i18n_index.len(),
            state.css_index.len()
        );
    }
}
```

**验证**: `cargo build -p rust-rml-lsp` 编译通过;`scan` 函数能在 demo 工作区扫描到 i18n zh-CN.json 与 styles.css。

---

### Step 1.2a 重构 hover 签名 — 传递 rust_query + 索引

**修改文件**: `crates/lsp/src/features/hover.rs`

`hover()` 函数签名扩展:

```rust
pub fn hover(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,  // 新增
    i18n_index: &I18nIndex,              // 新增
    css_index: &CssIndex,                // 新增
) -> Option<Hover> {
    // ...原有逻辑
    // 三级检测中传入新参数给 format_* 函数
}
```

各 `format_*` 函数签名相应扩展(按需):

- `format_tag_hover(elem, rust_query)` — 反查组件文档
- `format_attribute_name_hover(elem, attr, rust_query)` — 附加组件文档
- `format_attribute_value_hover(elem, attr, source, i18n_index, css_index)` — i18n/CSS 检测
- `format_attribute_hover(elem, attr)` — 不变(兜底,信息量少)

**修改文件**: `crates/lsp/src/handlers/hover.rs`

```rust
pub fn handle_hover(params: serde_json::Value, state: &mut ServerState) -> Result<Option<Hover>> {
    let params: HoverParams = serde_json::from_value(params)?;
    let uri = params.text_document_position_params.text_document.uri.clone();
    let position = params.text_document_position_params.position;

    if doctype::is_rust_file(&uri) {
        Ok(state.rust_query.hover(&uri, position).map(|info| Hover { ... }))
    } else {
        Ok(hover::hover(
            &uri, position, &state.workspace,
            state.rust_query.as_ref(),  // 新增
            &state.i18n_index,          // 新增
            &state.css_index,           // 新增
        ))
    }
}
```

**验证**: `cargo build -p rust-rml-lsp` 编译通过;现有 hover 测试调整签名后仍通过。

---

### Step 1.2b 实现 Method B — RML 标签 hover 反查 ra_ap_ide

**修改文件**: `crates/lsp/src/features/hover.rs::format_tag_hover`

逻辑:

1. 通过 `tags::component_lookup(tag)` 获取 `ComponentTag { ctor_path, ... }`
2. 从 `ctor_path`(如 `"rml_ui::Button"`)提取 struct 名(`Button`)
3. 调用 `rust_query.find_struct("Button")` 获取 `SymbolLocation { uri, range }`
4. 用 `range.start` 作为 Position 调用 `rust_query.hover(&uri, range.start)`
5. 将返回的 Markdown 内容**追加**到现有硬编码文档之后

```rust
fn format_tag_hover(elem: &Element, rust_query: &dyn RustSemanticQuery) -> String {
    let tag = &elem.tag;
    let mut md = String::new();

    // 原有分类逻辑(root/html/component/unknown)...
    if tags::component_lookup(tag).is_some() {
        md.push_str(&format!("## `<{}>` — Component\n\n", tag));
        md.push_str("gpui-component extension.\n");

        // === 新增:反查 ra_ap_ide 获取源码文档 ===
        if let Some(tag_info) = tags::component_lookup(tag) {
            // ctor_path 形如 "rml_ui::Button",取最后一段作为 struct 名
            let struct_name = tag_info.ctor_path.rsplit("::").next().unwrap_or(tag);
            if let Some(loc) = rust_query.find_struct(struct_name) {
                if let Some(info) = rust_query.hover(&loc.uri, loc.range.start) {
                    if !info.content.is_empty() {
                        md.push_str("\n---\n\n");
                        md.push_str(&info.content);
                    }
                }
            }
        }

        // 原有 props_registry 列表(statics/binds/events)...
    }
    // ...
}
```

**降级策略**: `find_struct` 返回 None(workspace 未加载/未找到)时,仅显示原有硬编码文档,不报错。

**验证**: 在 demo 中 hover `<Button>`,应显示 ra_ap_ide 提取的 `Button` struct 文档注释 + 原有属性列表。

---

### Step 1.3 实现 i18n hover — `t("key")` 检测

**修改文件**: `crates/lsp/src/features/hover.rs`

新增辅助函数:

```rust
/// 从表达式文本中提取 t("key") 的 key
/// 支持: t("key"), t('key'), t("key",), t("key", args)
fn extract_i18n_key(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    if !trimmed.starts_with("t(") {
        return None;
    }
    // 跳过 "t(" 找到引号开始
    let after_paren = trimmed[2..].trim_start();
    let (quote, rest) = if after_paren.starts_with('"') {
        ('"', &after_paren[1..])
    } else if after_paren.starts_with('\'') {
        ('\'', &after_paren[1..])
    } else {
        return None;
    };
    // 找到闭合引号
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}
```

**检测点 1**: `Node::Interpolation { expr, span }`

在 `hover()` 主函数中,光标落入元素但不在属性/标签名上时,检查是否在某个 `Node::Interpolation` 的 span 内:

```rust
// 在 hover() 主函数,标签名检测之前,新增插值检测
if let Some(info) = check_interpolation_hover(root, byte_offset, i18n_index, source, line_starts) {
    return Some(info);
}
```

新增函数:

```rust
fn check_interpolation_hover(
    root: &Node,
    offset: usize,
    i18n_index: &I18nIndex,
    source: &str,
    line_starts: &[u32],
) -> Option<Hover> {
    // 递归遍历 AST,找到 offset 落入 span 的 Node::Interpolation
    fn find_interp<'a>(node: &'a Node, offset: usize) -> Option<&'a rust_rml_engine::parser::ast::Node> {
        match node {
            Node::Element(e) => {
                if !e.span.contains(offset) { return None; }
                for c in &e.children {
                    if let Some(f) = find_interp(c, offset) { return Some(f); }
                }
                None
            }
            Node::Interpolation { span, .. } if span.contains(offset) => Some(node),
            _ => None,
        }
    }
    let interp = find_interp(root, offset)?;
    let expr = match interp {
        Node::Interpolation { expr, span } => (expr, *span),
        _ => return None,
    };
    let key = extract_i18n_key(&expr.0)?;
    let entries = i18n_index.lookup(&key)?;
    let md = format_i18n_hover(&key, entries);
    Some(make_hover(expr.1, md, source, line_starts))
}

fn format_i18n_hover(key: &str, entries: &[I18nEntry]) -> String {
    let mut md = String::new();
    md.push_str(&format!("### i18n: `{}`\n\n", key));
    for e in entries {
        md.push_str(&format!("- **{}**: {}\n", e.locale, e.value));
    }
    md.push_str(&format!("\n*Defined in {}*\n", entries[0].file_uri.path()));
    md.trim_end().to_string()
}
```

**检测点 2**: `Attribute::Bind { expr }` — 如 `label={t("login.submit")}`

在 `format_attribute_value_hover` 中,若 attr 是 Bind 且 `extract_i18n_key` 命中,追加 i18n 信息:

```rust
fn format_attribute_value_hover(
    elem: &Element, attr: &Attribute, source: &str,
    i18n_index: &I18nIndex,  // 新增
) -> String {
    // ...原有逻辑生成 value_desc...
    let mut md = String::new();
    // ...原有内容...

    // === 新增:i18n 检测 ===
    if let Attribute::Bind { .. } = attr {
        let expr_text = attr_bind_expr_span(attr, source)
            .and_then(|s| source.get(s.start..s.end))
            .unwrap_or("");
        if let Some(key) = extract_i18n_key(expr_text) {
            if let Some(entries) = i18n_index.lookup(&key) {
                md.push_str("\n\n---\n\n");
                md.push_str(&format_i18n_hover(&key, entries));
            }
        }
    }
    md.trim_end().to_string()
}
```

**验证**: 在 demo `login_dialog.rml` 中 hover `{t("login.title")}`,应显示 `zh-CN: 登录 RML Demo` 及文件位置。

---

### Step 1.4 实现 CSS hover — `class="xxx"` 检测

**修改文件**: `crates/lsp/src/features/hover.rs::format_attribute_value_hover`

在 `format_attribute_value_hover` 中,若 attr 是 `Static { name: "class", value }`,查询 `css_index`:

```rust
fn format_attribute_value_hover(
    elem: &Element, attr: &Attribute, source: &str,
    i18n_index: &I18nIndex,
    css_index: &CssIndex,  // 新增
) -> String {
    // ...原有逻辑 + i18n 检测...

    // === 新增:CSS class 检测 ===
    if let Attribute::Static { name, value, .. } = attr {
        if name == "class" {
            // 多类名拆分: "case-pane doc-pane" → ["case-pane", "doc-pane"]
            let classes: Vec<&str> = value.split_whitespace().collect();
            let mut css_sections = Vec::new();
            for class in classes {
                if let Some(entries) = css_index.lookup(class) {
                    css_sections.push(format_css_class_hover(class, entries));
                }
            }
            if !css_sections.is_empty() {
                md.push_str("\n\n---\n\n");
                md.push_str(&css_sections.join("\n\n---\n\n"));
            }
        }
    }
    md.trim_end().to_string()
}

fn format_css_class_hover(class: &str, entries: &[CssClassEntry]) -> String {
    let mut md = String::new();
    md.push_str(&format!("### CSS: `.{}`\n\n", class));
    for entry in entries {
        md.push_str(&format!("**{}**\n\n", entry.file_uri.path()));
        for (prop, val) in &entry.declarations {
            md.push_str(&format!("- `{}`: `{}`\n", prop, val));
        }
    }
    md.trim_end().to_string()
}
```

**说明**:
- 仅处理**应用层 + 页面层**(class 属性),inline style (`style="..."`) 不在此阶段处理
- `style="..."` 的 inline 样式 hover 留待后续(需调用 `rust_rml_engine::css::parser::parse` 解析 inline 文本)

**验证**: 在 demo 中 hover `class="case-pane"`,应显示 styles.css 中 `.case-pane` 的所有声明。

---

### Step 1.5 实现属性 hover — 附加组件文档

**修改文件**: `crates/lsp/src/features/hover.rs::format_attribute_name_hover`

在属性名 hover 中,若元素是组件标签,附加组件源码文档(复用 Method B 的反查逻辑):

```rust
fn format_attribute_name_hover(
    elem: &Element, attr: &Attribute,
    rust_query: &dyn RustSemanticQuery,  // 新增
) -> String {
    // ...原有逻辑生成 md...

    // === 新增:附加组件文档 ===
    if tags::component_lookup(&elem.tag).is_some() {
        if let Some(doc) = lookup_component_doc(&elem.tag, rust_query) {
            md.push_str("\n\n---\n\n");
            md.push_str(&doc);
        }
    }
    md.trim_end().to_string()
}

/// 复用 Method B 的反查逻辑(抽取出公共函数)
fn lookup_component_doc(tag: &str, rust_query: &dyn RustSemanticQuery) -> Option<String> {
    let tag_info = tags::component_lookup(tag)?;
    let struct_name = tag_info.ctor_path.rsplit("::").next()?;
    let loc = rust_query.find_struct(struct_name)?;
    let info = rust_query.hover(&loc.uri, loc.range.start)?;
    if info.content.is_empty() { None } else { Some(info.content) }
}
```

**验证**: 在 demo 中 hover `<Button label=...>` 的 `label` 属性名,应显示 Button struct 文档 + 原有属性信息。

---

## 假设与决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 资源发现策略 | 自动扫描 workspace `**/i18n/*.json` + `**/*.css` | 零配置,开发者无感知 |
| 属性文档来源 | 从 Rust 源码提取(长期) | ra_ap_ide 已具备能力,与 Method B 一致 |
| 实施范围 | 先 hover,后 goto-def | hover 是只读查询,风险低;goto-def 涉及位置计算,后续阶段 |
| CSS 层级覆盖 | 应用层 + 页面层(class 属性),跳过 inline(style) | inline 需单独解析,Phase 1 聚焦 class |
| JSON 扁平化 | 本地实现(复制 core 逻辑) | lsp crate 不依赖 core(避免 gpui) |
| CSS 行号 | 跳过(Rule 无行号字段) | Phase 1 仅展示声明内容,行号留待 goto-def 阶段 |
| 降级策略 | find_struct/hover 返回 None 时显示原有硬编码 | 保证 workspace 未加载时仍有基础体验 |

---

## 验证步骤

### 编译验证

```bash
cargo build --workspace
cargo build -p rust-rml-lsp --features rust-backend
```

### 单元测试

```bash
cargo test -p rust-rml-lsp
```

新增测试用例:
- `assets.rs`: `i18n_index_scan_finds_json`, `css_index_scan_finds_css`, `flatten_nested_json`
- `hover.rs`: `i18n_hover_in_interpolation`, `i18n_hover_in_bind_attr`, `css_hover_for_class`, `tag_hover_with_rust_doc`

### 集成验证(手动)

1. 启动 rml-lsp,在 VSCode 中打开 demo 工程
2. hover `<Button>` 标签 → 应显示 Button struct 文档注释
3. hover `{t("login.title")}` → 应显示 `zh-CN: 登录 RML Demo`
4. hover `class="case-pane"` → 应显示 `.case-pane` 的 CSS 声明
5. hover `label` 属性名 → 应显示 Button 文档 + 属性信息
6. workspace 未加载完成时 hover → 应降级显示原有硬编码文档

### 回归验证

- 现有 hover 测试全部通过
- `.rs` 文件 hover 不受影响(走 rust_query.hover() 原路径)

---

## 文件清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `crates/lsp/src/workspace/assets.rs` | 新建 | I18nIndex + CssIndex 实现 |
| `crates/lsp/src/workspace/mod.rs` | 修改 | 添加 `pub mod assets;` + re-export |
| `crates/lsp/src/server/connection.rs` | 修改 | ServerState 添加 i18n_index + css_index 字段 |
| `crates/lsp/src/server/dispatch.rs` | 修改 | "initialized" 触发资源扫描 |
| `crates/lsp/src/features/hover.rs` | 修改 | hover 签名扩展 + 四类 hover 实现 |
| `crates/lsp/src/handlers/hover.rs` | 修改 | 传递 rust_query + 索引给 hover() |

预计代码量: ~400-500 行(含测试)。
