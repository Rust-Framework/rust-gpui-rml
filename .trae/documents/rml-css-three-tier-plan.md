# RML 三层 CSS 架构迭代计划

## 摘要

为 RML 框架补齐"页面级 CSS"能力（`<style source="index.css"/>`），与应用级 CSS（`cx.set_style()` / `with_style()`）配合，形成完整的两层 CSS 加载体系（元素级 CSS 暂缓）。目标是让 RML 具备类似 HTML+CSS 的基础样式能力，在此之上可构建高级复杂组件。

**优先级**：L2 页面级 > L1 应用级（页面规则追加在全局规则之后，GPUI "last write wins" 自然实现覆盖语义）。

---

## 现状分析

### L1 应用级 CSS — 已完全可用

两条路径协同工作：

1. **运行时主题变量**：`cx.set_style("styles.css")`（[theme.rs:206-223](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)）加载 CSS，提取 `:root` 变量作为主题颜色。不参与编译期 class 匹配。
2. **编译期 class 匹配**：`rml::build().with_style("styles.css")`（[build/mod.rs:121](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）→ `load_stylesheets()`（[build/mod.rs:398-447](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）合并所有 CSS 为单一 `StyleSheet` → 经 `CodegenCtx.stylesheet` 传递 → `apply_css_styles()`（[attribute.rs:87](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs)）匹配 `class`/`id` 生成 GPUI 样式方法调用。

**无需改动**。

### L2 页面级 CSS — 完全不存在

- `<style source="..."/>` 语法未实现
- 无按 .rml 文件加载 CSS 的机制
- [build/mod.rs:291](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)：`stylesheet: stylesheet.clone()` — 所有 .rml 文件共享同一全局样式表
- Parser 将 `<style>` 当作普通 Element 处理（无特殊逻辑）
- codegen 路由时 `<style>` 会在 [node.rs:150-153](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 触发 `unknown tag` 错误

### L3 元素级 CSS — 有 bug（本计划暂缓）

用户决定暂不实现。已知 bug（记录在案，未来迭代）：
1. `style="..."` 对扩展组件被静默丢弃（[setters.rs:154](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs)）
2. 扩展组件样式优先级倒置（setter 在 `apply_css_styles` 之前调用）
3. `"style"` 未在 `props_registry` 中注册

### 关键架构事实

- `compile()`（[context.rs:286](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/context.rs)）是**纯函数** — 无文件系统访问，仅解析+生成代码
- `CodegenCtx.stylesheet: Option<StyleSheet>`（[context.rs:105](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/context.rs)）— 经 build.rs 构建后传入
- `apply_css_styles()`（[attribute.rs:87](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs)）接收单一 `&StyleSheet` — **无需改签名**，合并发生在传入之前
- `StyleSheet`（[ast.rs:9](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/ast.rs)）：`rules: Vec<Rule>` + `variables: HashMap<String, Value>`
- 合并策略（[build/mod.rs:435-437](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）：`merged.rules.extend(sheet.rules)` + `merged.variables.extend(sheet.variables)`
- GPUI 采用 "last write wins" — 后调用的样式方法覆盖先调用的
- 因此：页面规则追加在全局规则之后 → 页面 > 全局（正确的优先级）

---

## 提议变更

### 阶段 1：核心 — 页面级 CSS 实现

#### 1.1 新建 `crates/engine/src/compiler/style_directive.rs`

**职责**：从 .rml 源码中扫描 `<style source="..."/>` 指令，返回 CSS 文件路径列表。

**函数签名**：
```rust
/// 扫描 .rml 源码中所有 `<style source="...">` 指令，返回 CSS 文件路径列表。
///
/// 递归遍历 AST 所有元素节点。`source` 属性必须是静态字符串（不支持 bind 形式）。
/// 路径相对于 .rml 文件所在目录（由调用方在 build.rs 中解析）。
///
/// # 错误
/// - 解析 .rml 失败时返回 `ParseError`
/// - `<style>` 缺少 `source` 属性或 `source` 为空时返回 `ParseError`
pub fn scan_style_directives(source: &str) -> Result<Vec<String>, parser::ParseError>
```

**实现要点**：
- 使用 `parser::parse(source)` 解析为 AST
- 递归遍历所有 `Node::Element`，匹配 `elem.tag == "style"`
- 从 `elem.attributes` 中查找 `Attribute::Static { name: "source", value, .. }`
- 缺少 `source` 或值为空 → 返回 `ParseError`（明确错误信息）
- 返回路径列表（保持出现顺序）

**单元测试**（至少 5 个）：
1. `empty_source_no_style_directives` — 无 `<style>` 返回空 Vec
2. `single_style_directive` — `<style source="index.css"/>` 返回 `["index.css"]`
3. `multiple_style_directives` — 多个 `<style>` 按顺序返回
4. `nested_style_in_component` — `<component><div><style source="a.css"/></div></component>` 递归发现
5. `missing_source_attribute_returns_error` — `<style/>` 返回错误
6. `empty_source_value_returns_error` — `<style source=""/>` 返回错误

#### 1.2 修改 `crates/engine/src/compiler/mod.rs`

**变更**：添加模块声明与 re-export。

```rust
pub mod style_directive;
pub use style_directive::scan_style_directives;
```

在现有 `pub mod` 列表中按字母序插入 `style_directive`，并在 `pub use context::{...}` 后添加 `pub use style_directive::scan_style_directives;`。

#### 1.3 修改 `crates/engine/src/build/mod.rs`

**职责**：为每个 .rml 文件加载其引用的页面 CSS，与全局样式表合并后传入 `CodegenCtx`。

**变更点**：

**1.3.1 新增 `load_page_stylesheets` 辅助函数**（在 `Builder` impl 块外或内部）：

```rust
/// 加载 .rml 文件引用的页面级 CSS 文件，合并为单个 StyleSheet。
///
/// 路径相对于 .rml 文件所在目录。多个 `<style source="...">` 按出现顺序合并，
/// 后者规则追加在末尾（优先级更高）。
///
/// 返回 (合并后的页面样式表, 所有 CSS 源码拼接用于哈希, 所有 CSS 文件路径用于 rerun-if-changed)
fn load_page_stylesheets(
    rml_source: &str,
    rml_dir: &std::path::Path,
) -> Result<(Option<css::StyleSheet>, String, Vec<PathBuf>), BuildError>
```

实现：
- 调用 `crate::compiler::scan_style_directives(rml_source)?`
- 对每个路径：拼接 `rml_dir.join(path)` → 读取文件 → `css::parse()` → 合并 rules + variables
- 收集所有 CSS 源码到 `String`（用于哈希）
- 收集所有文件路径（用于 `cargo:rerun-if-changed`）
- 无页面 CSS 时返回 `(None, String::new(), Vec::new())`

**1.3.2 修改 `build()` 主循环**（[build/mod.rs:234-355](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）：

在每个 .rml 文件的处理流程中，**在构造 `CodegenCtx` 之前**插入页面 CSS 加载逻辑：

```rust
// 加载页面级 CSS（<style source="..."/>）
let rml_dir = rml_path.parent().unwrap_or(std::path::Path::new("."));
let (page_sheet, page_css_source, page_css_paths) = load_page_stylesheets(&source, rml_dir)?;

// 声明 rerun-if-changed（页面 CSS 文件变更时触发重建）
for css_path in &page_css_paths {
    println!("cargo:rerun-if-changed={}", css_path.display());
}

// 合并全局 + 页面样式表（页面规则追加在全局之后 → 优先级更高）
let merged_stylesheet = match (&stylesheet, page_sheet) {
    (Some(global), Some(page)) => {
        let mut merged = global.clone();
        merged.rules.extend(page.rules);
        merged.variables.extend(page.variables);
        Some(merged)
    }
    (Some(global), None) => Some(global.clone()),
    (None, Some(page)) => Some(page),
    (None, None) => None,
};

// 缓存哈希：.rml 源 + .rml.rs code-behind + 页面 CSS 内容
let page_css_hash = hash_str(&page_css_source);
```

**1.3.3 修改缓存校验**（[build/mod.rs:263-270](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）：

在现有的 `.rml` 源哈希 + code-behind 哈希校验基础上，增加页面 CSS 哈希校验：

```rust
let rml_unchanged = cache.entries.get(&key) == Some(&hash);
let cb_unchanged = match &current_cb_hash {
    Some(h) => cache.is_codebehind_unchanged(&key, h),
    None => cache.is_codebehind_unchanged(&key, ""),
};
// 页面 CSS 哈希校验：CSS 文件变更时强制重新生成
let page_css_unchanged = cache.is_page_style_unchanged(&key, &page_css_hash);
if rml_unchanged && cb_unchanged && page_css_unchanged {
    continue;
}
```

**1.3.4 修改 `CodegenCtx` 构造**（[build/mod.rs:288-311](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）：

将 `stylesheet: stylesheet.clone()` 改为 `stylesheet: merged_stylesheet`（使用合并后的样式表）。

**1.3.5 缓存写入**（[build/mod.rs:338-344](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)）：

成功编译后，额外 stamp 页面 CSS 哈希：
```rust
cache.stamp_page_style(key.clone(), page_css_hash);
```

**1.3.6 修改 `crates/engine/src/build/cache.rs`**：

新增页面 CSS 哈希存储与校验方法：
- `CacheEntry` 增加 `page_style_hash: Option<String>` 字段（保持向后兼容，默认 `None`）
- `Cache` 新增方法：
  - `is_page_style_unchanged(&self, key: &str, hash: &str) -> bool`
  - `stamp_page_style(&mut self, key: String, hash: String)`

向后兼容：旧缓存条目无 `page_style_hash` 字段时，`is_page_style_unchanged` 返回 `false`（强制重新生成一次）。

#### 1.4 修改 `crates/engine/src/compiler/codegen/render.rs`

**职责**：从 codegen 输出中过滤 `<style>` 元素，避免生成渲染代码。

**变更点**：在 [render.rs:47-54](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs) `gen_render_impl_from_children` 中，构造 `body` 之前过滤掉 `<style>` 元素：

```rust
let mut body_children = if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
    shell::partition_slot_children(&elem.children).body
} else {
    elem.children.clone()
};
// 过滤 <style> 元素：页面级 CSS 指令不参与渲染，由 build.rs 在编译期处理
body_children.retain(|node| {
    !matches!(node, Node::Element(e) if e.tag == "style")
});
```

注意：对 `Tab`/`Modern` shell，`partition_slot_children` 已将非 `<template>` 子节点放入 `body`，`<style>` 会落入 `body`。对 `Window`/`None` shell，直接从 `elem.children` 过滤。

但更彻底的做法是在 `partition_slot_children` 之前过滤，确保 `<style>` 不会进入任何 slot。因此还需要修改 shell.rs。

#### 1.5 修改 `crates/engine/src/compiler/codegen/shell.rs`

**职责**：在 `partition_slot_children` 入口处过滤 `<style>` 元素，防止其进入 `body` 或被误识别。

**变更点**：在 [shell.rs:200](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) `partition_slot_children` 函数开头添加过滤：

```rust
pub(crate) fn partition_slot_children(children: &[Node]) -> ShellSlots {
    let mut slots = ShellSlots::default();

    for child in children {
        // 过滤 <style> 元素：页面级 CSS 指令由 build.rs 在编译期处理，不参与渲染
        if let Node::Element(elem) = child {
            if elem.tag == "style" {
                continue;
            }
        }
        // ... 原有逻辑
    }
    slots
}
```

**注意**：`<style>` 不应出现在 `<template slot="...">` 内部（validator 应拦截），但 `partition_slot_children` 仅处理顶层子节点，`<style>` 作为顶层子节点会被此处过滤。

#### 1.6 修改 `crates/engine/src/compiler/validator.rs`

**职责**：校验 `<style>` 元素的合法使用。

**校验规则**：
1. `<style>` 必须有 `source` 静态属性（非 bind 形式）
2. `<style>` 不能有子节点（自闭合或空）
3. `<style>` 不能出现在 `<template slot="...">` 内部（仅允许作为根元素的直接子节点）

**变更点**：在 `validate()` 函数中添加递归检查，遇到 `<style>` 元素时执行上述校验。

---

### 阶段 2：验证 — 应用级 CSS 回归

确保阶段 1 的改动不破坏 L1 应用级 CSS 的现有行为。

**验证项**：
1. `cargo build --workspace` 编译通过
2. `cargo test --workspace --exclude rust-rml-demo --lib` 全部测试通过
3. demo 应用启动后，全局 CSS 样式正常生效（class 匹配、主题变量）
4. 新增的页面级 CSS 在 demo 中生效（优先级高于全局）

---

## 假设与决策

### 决策
1. **CSS 路径基准**：相对于 .rml 文件所在目录（直觉友好，与 HTML `<link href>` 相对路径语义一致）
2. **合并策略**：页面 CSS 规则追加在全局规则之后（GPUI "last write wins" → 页面 > 全局）
3. **`compile()` 保持纯函数**：所有 CSS 文件加载在 build.rs 中完成，`CodegenCtx.stylesheet` 传入已合并的样式表
4. **`<style>` 不渲染**：从 codegen 输出中过滤，仅作为编译期指令
5. **仅支持 `source` 属性**：不支持内联 CSS 内容（`<style>.foo { ... }</style>`），保持与现有 CSS 文件加载机制一致
6. **元素级 CSS 暂缓**：按用户决定，本计划不涉及 L3 修复
7. **缓存失效**：页面 CSS 内容哈希纳入 per-file 缓存键，CSS 文件变更时对应 .rml 重新编译

### 假设
1. `parser::parse()` 成功解析含 `<style>` 的 .rml 文件（已验证：parser 接受任意标签名）
2. `css::parse()` 能解析页面 CSS 文件（复用现有解析器，无新增依赖）
3. 页面 CSS 文件数量较少（通常 0-2 个/页面），合并开销可忽略
4. 全局样式表 clone 开销可接受（build 期操作，非运行时）

---

## 验证步骤

### 单元测试
- [ ] `style_directive.rs` 单元测试全部通过（5+ 测试用例）
- [ ] `cache.rs` 新增方法的单元测试通过（向后兼容性验证）

### 集成测试
- [ ] 新增端到端测试：含 `<style source="page.css"/>` 的 .rml 文件编译成功
- [ ] 新增端到端测试：页面 CSS 规则覆盖全局 CSS 规则（同选择器同属性，页面值生效）
- [ ] 新增端到端测试：无 `<style>` 的 .rml 文件行为不变（回归验证）

### 编译与测试
- [ ] `cargo build --workspace` 编译通过（0 error）
- [ ] `cargo test --workspace --exclude rust-rml-demo --lib` 全部通过
- [ ] demo 应用 `cargo run` 启动成功

### Demo 验证
- [ ] 在某个 demo .rml 文件中添加 `<style source="page.css"/>`
- [ ] 创建对应 page.css，定义覆盖全局样式的规则
- [ ] 启动 demo 验证页面级 CSS 生效且优先级正确

---

## 实施顺序

1. **新建 `style_directive.rs`** — `scan_style_directives()` + 单元测试
2. **修改 `compiler/mod.rs`** — 导出新模块
3. **修改 `build/cache.rs`** — 新增 `page_style_hash` 字段与方法
4. **修改 `build/mod.rs`** — 页面 CSS 加载 + 合并 + 缓存校验
5. **修改 `codegen/render.rs`** — 过滤 `<style>` 元素
6. **修改 `codegen/shell.rs`** — 在 `partition_slot_children` 中过滤 `<style>`
7. **修改 `validator.rs`** — `<style>` 使用合法性校验
8. **新增端到端测试** — 验证页面 CSS 优先级
9. **`cargo build --workspace`** — 编译验证
10. **`cargo test --workspace --exclude rust-rml-demo --lib`** — 全量测试验证
11. **Demo 验证** — 实际页面级 CSS 效果
