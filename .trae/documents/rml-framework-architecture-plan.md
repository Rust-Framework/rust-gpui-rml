# RML 框架架构设计规划

> **目标**：从最底层架构开始，自底向上设计 RML 框架，使实现与 `docs/` 11 章文档的功能说明完全匹配。
> **策略**：分两阶段交付 —— Phase A 实现 MVP 闭环（counter demo 可运行），Phase B 补全所有文档承诺的高级特性。
> **铁律**：所有 trait 定义必须以 `I` 开头（`IModel`、`IViewModel`、`IRmlView`、`ICommand`、`IConverter`、`ITwoWayBinding`、`ILifecycle`、`IBindingContext`、`IEvent`）。

---

## 一、概述

RML（Rust Markup Language）是基于 GPUI 的 HTML 友好型声明式 UI 框架，借鉴 WPF XAML 的 MVVM 模式。开发者编写 `.rml`（视图标记）和 `.rml.rs`（ViewModel 业务逻辑），由 `build.rs` 在编译期把 `.rml` 编译为原生 GPUI 代码，零运行时解析开销。

**三层架构**：
- **表现层**：`.rml` + `.rml.rs`（开发者编辑）
- **编译层**：`build.rs` 四阶段流水线（解析 → AST → 验证 → 代码生成）
- **框架层**：GPUI + gpui-component + RML Runtime

**5 个 crate**：`rml-core`（trait 定义）、`rml-macros`（过程宏）、`rml`（解析器+编译器+构建器+运行时）、`rml-app`（启动器）、`gpui-component`（外部依赖）。

---

## 二、当前状态分析

### 2.1 已完成 ✅

| 组件 | 状态 | 说明 |
|------|------|------|
| Workspace 骨架 | ✅ | `Cargo.toml`（workspace）、5 个子 crate 的 `Cargo.toml` |
| `rml-core` 全部 trait | ✅ | 8 个 trait + 事件对象 + ElementRef + BindingPath + prelude |
| `rml-macros` 入口 | ✅ | `lib.rs` 声明 8 个过程宏入口，模块文件未实现 |
| `rml` 骨架 | ✅ | `lib.rs` 引用 build/compiler/parser/tags/prelude 模块，未实现 |
| `rml-app` 骨架 | ✅ | `lib.rs` 引用 application/resources/window 模块，未实现 |

### 2.2 待实现 ❌

| 层 | 组件 | 待实现内容 |
|----|------|-----------|
| Layer 0 | `rml-core` 补全 | `IEvent` trait、`ICommand` 扩展、消除 `IViewModel`/`ILifecycle` 冗余、事件对象字段对齐 |
| Layer 1 | `rml-macros` | 8 个过程宏的实现模块（`derive_model.rs` 等 6 个文件） |
| Layer 2 | `rml/parser` | tokenizer + AST |
| Layer 2 | `rml/compiler` | validator + codegen |
| Layer 2 | `rml/tags` | HTML 标签到 GPUI 元素的映射表 |
| Layer 3 | `rml/build` | `RmlBuild` Builder API + cargo:rerun-if-changed + 增量缓存 |
| Layer 4 | `rml/runtime` | 事件流调度、组件注册表、样式系统、热重载 watcher |
| Layer 5 | `rml-app` | `RmlApplication` 启动器、窗口管理、资源加载 |
| Layer 6 | `demo` | counter 三件套（.rml + .rml.rs + main.rs + build.rs） |

### 2.3 文档与代码的关键差距（必须修复）

通过 subagent 深度核对发现：

1. **`IViewModel` 与 `ILifecycle` 重复定义** `rml_on_loaded`/`rml_on_unloaded`。决策：`IViewModel: ILifecycle`，生命周期方法仅保留在 `ILifecycle`。
2. **`ICommand` 缺少元信息**：仅有 `rml_command_name()`，无法支撑编译期参数/事件类型校验。决策：扩展为含关联类型 `type EventArgs` + 参数描述。
3. **`Event` trait 未定义**：文档 §5.2.9 要求 `prevent_default`/`stop_propagation`/`is_default_prevented`/`is_propagation_stopped`。决策：新增 `IEvent` trait，所有事件对象实现它。
4. **事件对象字段与文档不一致**：`ClickEvent` 缺 `modifiers`/`click_count`；`InputEvent` 文档写 `old_value` 代码写 `prev_value`；`WheelEvent` 结构不同；`FocusEvent`/`SubmitEvent` 字段缺失。决策：以文档为准补全字段（保留旧字段作 alias 直至 Phase B）。
5. **`ViewContext` vs `Context`**：文档统一用 `ViewContext<Self>`，实际 GPUI 现代版本用 `Context<Self>`。决策：以 GPUI 实际 API 为准（`Context<Self>`），文档侧通过 `pub type ViewContext<T> = Context<T>;` 别名兼容示例代码。
6. **`IConverter` 方法名**：文档写 `convert`/`convert_back`，代码写 `convert_to`/`convert_from`。决策：以代码为准（`convert_to`/`convert_from` 更清晰），文档侧修订。
7. **组件系统运行时缺失**：组件注册表、插槽调度、依赖注入、`#[on_prop_change]`、`#[prop]`、`#[element]` 字段绑定钩子均未实现。
8. **样式系统完全缺失**：CSS 解析器、CSS 变量表、主题切换、`cx.set_css_var` 等运行时方法都不存在。
9. **事件流调度缺失**：捕获/冒泡三阶段、`stop_propagation` 运行时落点不存在。

---

## 三、设计决策与铁律

### 3.1 铁律（不可违反）

1. **所有 trait 必须以 `I` 开头**：`IModel`、`IViewModel`、`IRmlView`、`ICommand`、`IConverter`、`ITwoWayBinding`、`ILifecycle`、`IBindingContext`、`IEvent`、`IComponent`、`IDirective`、`IPlugin`。
2. **`#![forbid(unsafe_code)]`** 全 crate 启用（已在 core/macros/rml/app 启用）。
3. **`rml-core` 仅依赖 gpui 的基础类型**（`App`/`Context`/`Entity`/`SharedString` 等），不依赖渲染系统，保证未来可移植到其他后端。
4. **生成代码必须输出到 `OUT_DIR`**，禁止输出到 `src/`（会触发 build.rs 死循环）。
5. **过程宏不做重活**：`#[view]`/`#[component]` 只生成 trait 实现和 `include!`，模板编译由 `build.rs` 完成。
6. **MVP 阶段所有方法级属性宏为 pass-through**：`#[command]`/`#[computed]`/`#[on_loaded]`/`#[on_unloaded]`/`#[element]` 在 Phase A 不修改方法体，仅标记元信息；Phase B 再增强实际功能。

### 3.2 关键设计决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| GPUI 依赖来源 | Zed 官方 git 依赖 | 与文档一致，跟随上游 |
| gpui-component | 外部 crate 依赖 | 复用 longbridge 已有组件库 |
| 交付策略 | 分阶段（Phase A MVP → Phase B 完整） | 风险可控、早日闭环验证 |
| 上下文类型 | `Context<T>`（GPUI 现代版） | 文档示例用 `ViewContext` 是历史遗留，加 `pub type ViewContext<T> = Context<T>` 别名兼容 |
| 订阅粒度 | ViewModel 级 + GPUI diff 优化 | 文档 §3.6.3 明确指出"字段级细粒度订阅是未来路线图" |
| 双向绑定实现 | 编译期生成 `on_change` 回调 + `set_value` | 文档 §3.6.4 明确，无运行时反射 |
| `#[computed]` 依赖追踪 | 编译期静态分析 `self.field` 访问 | 性能最优，符合 RML 编译期哲学 |
| 模板路径默认约定 | 结构体名 snake_case + ".rml" | 与文档 §4.1.5 一致 |

### 3.3 假设

- GPUI API 已稳定（`Context<T>`/`Entity<T>`/`cx.notify()`/`cx.listener()`/`cx.spawn()`）。
- gpui-component 提供 `Button`/`Input`/`TextArea`/`Modal` 等基础组件。
- 用户机器已安装 Rust 工具链（cargo + rustc）。
- Phase A 不实现热重载（`hot-reload` feature 保留 stub），Phase B 再补全。

---

## 四、自底向上架构设计

### Layer 0 · `rml-core` 补全（trait 定义层）

**目标**：修复文档与代码的差距，让所有上层模块依赖一致的 trait 契约。

#### 4.0.1 修改 `crates/core/src/lifecycle.rs`

`ILifecycle` 保留 `rml_on_loaded`/`rml_on_unloaded` 不变。

#### 4.0.2 修改 `crates/core/src/view_model.rs`

```rust
pub trait IViewModel: IModel + ILifecycle {
    // 移除 rml_on_loaded/rml_on_unloaded（由 ILifecycle 提供）
}
```

#### 4.0.3 新建 `crates/core/src/event.rs`

```rust
/// RML 事件基础 trait（文档 §5.2.9）
pub trait IEvent: std::fmt::Debug + Clone + Send + Sync + 'static {
    fn prevent_default(&mut self);
    fn stop_propagation(&mut self);
    fn is_default_prevented(&self) -> bool;
    fn is_propagation_stopped(&self) -> bool;
}
```

#### 4.0.4 修改 `crates/core/src/events.rs`

为所有事件结构体补全字段并实现 `IEvent`：
- `ClickEvent` 增加 `modifiers: Modifiers`、`click_count: u32`，增加 `default_prevented`/`propagation_stopped` 标志位。
- `InputEvent` 字段 `prev_value` 保留，新增 `old_value` 作为 alias（`pub fn old_value(&self) -> &SharedString { &self.prev_value }`）。
- `WheelEvent` 保留 `delta_x`/`delta_y`，新增 `delta: ScrollDelta` 枚举字段（Phase B 实现）。
- `FocusEvent` 增加 `target: Option<gpui::EntityId>`。
- `SubmitEvent` 增加 `form_data: std::collections::HashMap<SharedString, SharedString>`（Phase B 实现）。
- 所有事件实现 `IEvent` trait。

#### 4.0.5 修改 `crates/core/src/command.rs`

```rust
pub trait ICommand {
    fn rml_command_name() -> &'static str;
    /// 事件对象类型名（编译期生成，如 "ClickEvent"）
    fn rml_event_type() -> &'static str { "" }
    /// 参数描述（编译期生成，由 #[command] 宏填充）
    fn rml_params() -> &'static [ParamMeta] { &[] }
}

#[derive(Debug, Clone)]
pub struct ParamMeta {
    pub name: &'static str,
    pub ty: &'static str,
}
```

#### 4.0.6 新建 `crates/core/src/component.rs`

```rust
/// 可复用组件 trait（文档 §6.2）
pub trait IComponent: IRmlView {
    /// 组件标签名（PascalCase），用于 .rml 中的 <MyComponent>
    fn rml_tag() -> &'static str;
}
```

#### 4.0.7 修改 `crates/core/src/lib.rs`

新增 `pub mod component;`、`pub mod event;`，导出新 trait。

#### 4.0.8 修改 `crates/core/src/prelude.rs`

加入 `pub use crate::component::IComponent;`、`pub use crate::event::IEvent;`、`pub use crate::command::ParamMeta;`，加入 `pub type ViewContext<T> = Context<T>;` 别名。

---

### Layer 1 · `rml-macros`（过程宏层）

**目标**：实现 8 个过程宏，Phase A 完成 derive/View 的代码生成，其余为 pass-through；Phase B 补全 computed 缓存与 command 元信息。

#### 4.1.1 新建 `crates/macros/src/derive_model.rs`

```rust
pub fn derive(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    // 解析 struct，对每个 pub 字段生成 FieldMeta
    // 生成 impl IModel for Struct { fn rml_fields(&self) -> &'static [FieldMeta] { ... } }
}
```

生成示例：
```rust
impl IModel for Counter {
    fn rml_fields(&self) -> &'static [FieldMeta] {
        &[FieldMeta { name: "count", ty: "i32" }]
    }
}
```

#### 4.1.2 新建 `crates/macros/src/view.rs`

实现 `expand(args, input)` 与 `expand_component(args, input)`：

1. 解析 `#[view]` 或 `#[view(template = "path")]` 参数。
2. 解析 struct，收集 `#[element]` 字段及其 `ref=` 参数。
3. 生成：
   - `impl IModel for Struct`（同 derive_model）
   - `impl ILifecycle for Struct`（默认空实现，由 `#[on_loaded]`/`#[on_unloaded]` 覆写）
   - `impl IViewModel for Struct {}`
   - `impl IRmlView for Struct { fn rml_template() -> &'static str { "path" } }`
   - 若 `#[component]`：额外生成 `impl IComponent for Struct { fn rml_tag() -> &'static str { "Struct" } }`
   - `include!(concat!(env!("OUT_DIR"), "/rml_generated/<name>.rs"));`
4. 默认模板路径计算：`<snake_case_struct_name>.rml`（如 `Counter` → `counter.rml`）。

#### 4.1.3 新建 `crates/macros/src/command.rs`

Phase A：pass-through（不修改方法），但解析签名提取 `event_type` 与 `params`，通过 `inventory` crate 或全局注册表提交元信息（Phase B 用）。Phase A 简化为完全 pass-through + 解析校验签名合法。

#### 4.1.4 新建 `crates/macros/src/computed.rs`

Phase A：pass-through，仅校验签名是 `&self` 且无参。
Phase B：分析方法体中 `self.field` 访问，生成缓存代码。

#### 4.1.5 新建 `crates/macros/src/lifecycle.rs`

Phase A：pass-through + 收集方法名，供 `#[view]` 在生成 `impl ILifecycle` 时调用。实现策略：使用全局 `inventory::submit!(LifecycleHook { struct_name, method_name, kind })`，`#[view]` 在展开时通过 `inventory::iter::<LifecycleHook>` 查找匹配项。

> **简化方案**（推荐）：`#[on_loaded]`/`#[on_unloaded]` 重命名方法为 `rml_on_loaded_impl`/`rml_on_unloaded_impl`，`#[view]` 生成 `impl ILifecycle` 时调用此方法名。

#### 4.1.6 新建 `crates/macros/src/element.rs`

Phase A：pass-through + 校验字段类型是 `ElementRef<T>`。
Phase B：在 `#[view]` 生成代码中收集 `#[element]` 字段与 `ref=` 名映射，注入到 `bind_elements` 钩子。

#### 4.1.7 修改 `crates/macros/Cargo.toml`

加入 `rml-core = { workspace = true }` 依赖（生成代码时需要引用 trait 路径）。

---

### Layer 2 · `rml/parser` + `rml/compiler` + `rml/tags`（解析编译层）

**目标**：实现 `.rml` → AST → 类型化 AST → Rust 代码的纯转换流水线。无 IO，无文件系统访问，便于测试与未来接入 LSP。

#### 4.2.1 新建 `crates/rml/src/tags.rs`

HTML 标签到 GPUI 元素构造调用的映射表：

```rust
pub enum BuiltinTag {
    Div, Span, P, H1, H2, H3, H4, H5, H6,
    Button, Input, TextArea, Ul, Ol, Li, Img, A, Label, Br,
}

pub fn lookup(tag: &str) -> Option<BuiltinTag>;
pub fn codegen_ctor(tag: BuiltinTag) -> &'static str;  // 如 "gpui::div()"
pub fn is_self_closing(tag: BuiltinTag) -> bool;       // br/img/input 等
```

#### 4.2.2 新建 `crates/rml/src/parser/mod.rs` + `tokenizer.rs` + `ast.rs`

**`tokenizer.rs`**：HTML 词法分析器，识别：
- 标签开始 `<div>`、标签结束 `</div>`、自闭合 `<input />`
- 属性名、字符串值 `"..."`、绑定值 `{expr}`、混合值 `"card {theme}"`
- HTML 注释 `<!-- ... -->`
- 文本节点（含 `{interpolation}`）

**`ast.rs`**：抽象语法树：

```rust
pub enum Node {
    Element(Element),
    Text(SharedString),
    Interpolation(Expr),  // {expr}
    If { condition: Expr, body: Vec<Node>, else_body: Option<Vec<Node>> },
    Each { item: Ident, index: Option<Ident>, iterable: Expr, key: Option<Expr>, body: Vec<Node> },
}

pub struct Element {
    pub tag: String,
    pub attributes: Vec<Attribute>,
    pub children: Vec<Node>,
}

pub enum Attribute {
    Static { name: String, value: String },
    Bind { name: String, expr: Expr },
    Event { name: String, handler: EventHandler },
    Directive(Directive),
}

pub enum Directive {
    If(Expr), Else, Each { item: Ident, index: Option<Ident>, iterable: Expr, key: Option<Expr> },
    Model(Expr), Show(Expr), Once, Html(Expr), Ref(String), Slot(String),
}

pub enum EventHandler {
    Ident(String),                                  // onclick={fn}
    MethodName(String),                             // onclick="method"
    WithArgs(String, Vec<Expr>),                    // onclick={fn, {item.id}, 'x'}
}
```

**`mod.rs`**：`pub fn parse(source: &str) -> Result<Vec<Node>, ParseError>`，要求单根元素，返回错误含行号/列号。

#### 4.2.3 新建 `crates/rml/src/compiler/mod.rs` + `validator.rs` + `codegen.rs`

**`validator.rs`**：语义分析，产出类型化 AST（Phase A 简化版，仅做语法合法性校验，不做 ViewModel 字段类型校验 —— 因为编译期拿不到 ViewModel 类型信息）：

```rust
pub fn validate(ast: &[Node]) -> Result<(), ValidationError>;
```

校验项（Phase A）：
- `else` 必须紧跟 `if` 元素
- `each` 的 `key` 必须存在（警告而非错误）
- `model` 只能用于 `input`/`textarea`/`checkbox`
- `ref` 名同视图内唯一
- `slot` 必须在组件内（PascalCase 标签）

校验项（Phase B）：通过 `rml-core` 的 `IModel::rml_fields()` 元信息校验绑定路径（需引入 trait object 或编译期类型信息）。

**`codegen.rs`**：代码生成器：

```rust
pub fn codegen(ast: &[Node], ctx: &CodegenCtx) -> Result<String, CodegenError>;

pub struct CodegenCtx {
    pub view_struct_name: String,  // 如 "Counter"
    pub view_module_path: String,  // 如 "my_app::views::counter"
}
```

生成一个完整的 `impl Render for <ViewStruct>` 代码块，写入字符串返回。

**代码生成规则**（基于文档 §10.6.4）：

| 模板语法 | 生成 Rust 代码 |
|---------|---------------|
| `<div class="card">` | `gpui::div().class("card")` |
| `<p>{title}</p>` | `gpui::div().child(gpui::Label::new(self.title.clone().to_string()))` |
| `<p>欢迎, {name}</p>` | `gpui::div().child(gpui::Label::new(format!("欢迎, {}", self.name)))` |
| `<div if={cond}>...</div>` | `if self.cond { Some(<body>.into_any_element()) } else { None }` |
| `<div show={cond}>...</div>` | `<body>.when(self.cond, \|x\| x)` |
| `<li each={item in items} key={item.id}>` | `self.items.iter().map(\|item\| <body>).collect::<Vec<_>>().into_iter()` |
| `<span once>{x}</span>` | 直接展开为字面量，不订阅 |
| `<input model={field} />` | `gpui_component::Input::new().value(self.field.clone()).on_change(cx.listener(\|this, v, cx\| { this.field = v; cx.notify(); }))` |
| `<button onclick={fn}>` | `<button>.on_click(cx.listener(\|this, ev, cx\| this.fn(&ev.into(), cx))` |
| `<button onclick={fn, {item.id}}>` | `<button>.on_click(cx.listener(move \|this, ev, cx\| { let p0 = item.id.clone(); this.fn(p0, &ev.into(), cx) }))` |
| `<div ref="name">` | 在 `render` 中创建元素后调用 `self.<element_field>.set(handle)`（Phase B） |
| `<div html={raw}>` | `gpui::div().child(rml::Html::parse(self.raw.as_ref()))`（Phase B） |
| `<MyComp prop="x" />` | `my_app::components::MyComp::new().prop("x")` |
| `<div class="card {dyn}">` | `gpui::div().class(format!("card {}", self.dyn))` |

**`mod.rs`**：顶层入口 `pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<String, CompileError>`，串起 parse → validate → codegen。

#### 4.2.4 新建 `crates/rml/src/prelude.rs`

重导出常用 API：`pub use rml_core::prelude::*;`、`pub use rml_macros::*;`、`pub use crate::build::RmlBuild;`、`pub use crate::compiler::compile;`。

---

### Layer 3 · `rml/build`（构建集成层）

**目标**：在用户 `build.rs` 中调用，扫描 `.rml` 文件、调用编译器、输出到 `OUT_DIR`、打印 `cargo:rerun-if-changed`。

#### 4.3.1 新建 `crates/rml/src/build/mod.rs` + `cache.rs` + `scanner.rs`

**`mod.rs`**：

```rust
pub fn build() -> Builder;

pub struct Builder {
    scan_dirs: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    namespace: Option<String>,
    strict: bool,
    hot_reload: bool,
    public: bool,
}

impl Builder {
    pub fn scan_dir(mut self, dir: impl Into<PathBuf>) -> Self;
    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self;
    pub fn namespace(mut self, ns: impl Into<String>) -> Self;
    pub fn strict(mut self, on: bool) -> Self;
    pub fn hot_reload(mut self, on: bool) -> Self;
    pub fn public(mut self, on: bool) -> Self;
    pub fn build(self) -> Result<(), BuildError>;
}
```

**`build()` 主流程**：
1. 从 `scan_dirs`（默认 `["src"]`）递归扫描 `*.rml` 文件。
2. 对每个文件 `println!("cargo:rerun-if-changed={}", path.display())`。
3. 读取 `OUT_DIR/rml_cache.json`，比对每个 `.rml` 哈希决定增量/全量。
4. 对每个 `.rml`：
   - 解析文件名为 view 结构名（如 `counter.rml` → `Counter`，按 PascalCase 转换）
   - 调用 `compiler::compile(source, &CodegenCtx { view_struct_name, view_module_path })`
   - 写入 `OUT_DIR/rml_generated/<name>.rs`
5. 写回 `rml_cache.json`。
6. 错误以 `cargo:warning=...` 形式输出 + 返回 `Err`。

**`scanner.rs`**：递归扫描目录，返回 `.rml` 文件列表。

**`cache.rs`**：`Cache` 结构 + JSON 序列化（哈希 → 文件名映射）。

#### 4.3.2 修改 `crates/rml/Cargo.toml`

加入 `serde = { version = "1", features = ["derive"] }`、`serde_json = "1"`、`walkdir = "2"`、`sha2 = "0.10"` 依赖（build 模块用）。注意：这些是 build-time 依赖，不影响运行时。

---

### Layer 4 · `rml/runtime`（运行时层）

**目标**：提供运行时支持，包括事件流调度、组件注册表、样式系统、热重载 watcher。

#### 4.4.1 新建 `crates/rml/src/runtime/mod.rs` + `event_flow.rs` + `component_registry.rs` + `styling.rs` + `watcher.rs`

**`event_flow.rs`**：事件流调度器（捕获 → 目标 → 冒泡），维护 `IEvent::stop_propagation`/`prevent_default` 状态。Phase A 简化为冒泡阶段。

**`component_registry.rs`**：全局组件注册表，PascalCase 标签名 → 构造器。Phase A 仅注册内置 HTML 标签（通过 `tags::lookup`），Phase B 支持用户组件注册。

**`styling.rs`**：CSS 子集解析器 + CSS 变量表 + 主题切换。Phase A stub（提供 `cx.set_theme` 等空实现），Phase B 实现完整 CSS 解析。

**`watcher.rs`**：热重载文件监听（依赖 `notify` crate，feature gate）。Phase A 完全 stub，Phase B 实现。

#### 4.4.2 修改 `crates/rml/src/lib.rs`

加入 `pub mod runtime;`。

---

### Layer 5 · `rml-app`（应用启动层）

**目标**：封装 GPUI 的 `Application` + 窗口创建，提供 `RmlApplication::new().run::<RootView>()` API。

#### 4.5.1 新建 `crates/app/src/application.rs`

```rust
pub struct RmlApplication {
    title: SharedString,
    width: Pixels,
    height: Pixels,
    globals: Vec<Box<dyn FnOnce(&mut App)>>,
}

impl RmlApplication {
    pub fn new() -> Self;
    pub fn title(mut self, t: impl Into<SharedString>) -> Self;
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self;
    pub fn with_global<G: Global + 'static>(mut self, global: G) -> Self;
    pub fn run<R: IRmlView + Render>(self);
}
```

`run` 内部：`gpui::Application::new().run(|cx| { cx.open_window(..., |window, cx| cx.new(|cx| R::new(...))); })`。

#### 4.5.2 新建 `crates/app/src/window.rs`

窗口管理 helper（多窗口、关闭回调等）。Phase A 仅单窗口。

#### 4.5.3 新建 `crates/app/src/resources.rs`

资源加载（`assets/` 目录的图标、字体、i18n）。Phase A stub。

#### 4.5.4 修改 `crates/app/Cargo.toml`

加入 `gpui = { workspace = true }`、`rml-core = { workspace = true }` 依赖。

---

### Layer 6 · `demo`（验证闭环）

**目标**：counter 三件套 —— `.rml` + `.rml.rs` + `main.rs` + `build.rs`，验证全栈可用。

#### 4.6.1 新建 `demo/Cargo.toml`

```toml
[package]
name = "rml-demo"
version.workspace = true
edition.workspace = true

[dependencies]
rml = { workspace = true }
rml-app = { workspace = true }
gpui = { workspace = true }

[build-dependencies]
rml = { workspace = true }
```

#### 4.6.2 新建 `demo/build.rs`

```rust
fn main() {
    rml::build()
        .scan_dir("src")
        .output_dir(std::env::var("OUT_DIR").unwrap())
        .build()
        .expect("RML build failed");
}
```

#### 4.6.3 新建 `demo/src/main.rs`

```rust
use rml_app::RmlApplication;
mod counter;

fn main() {
    RmlApplication::new()
        .title("RML Counter Demo")
        .size(px(400.), px(300.))
        .run::<counter::Counter>();
}
```

#### 4.6.4 新建 `demo/src/counter.rml`

```html
<div class="counter">
    <h1>计数器</h1>
    <p class="count">{count}</p>
    <div class="buttons">
        <button onclick={decrement}>-</button>
        <button onclick={increment}>+</button>
    </div>
</div>
```

#### 4.6.5 新建 `demo/src/counter.rml.rs`

```rust
use rml::prelude::*;

#[derive(IModel)]
#[view]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count -= 1;
        cx.notify();
    }
}
```

#### 4.6.6 加入 `Cargo.toml` workspace members

已包含 `demo`。

---

## 五、文档对照验收清单

### Phase A · MVP 闭环

| 文档章节 | 验收项 | 状态 |
|---------|--------|------|
| §1.2 三层架构 | 5 个 crate 划分清晰，依赖方向正确 | ✅ |
| §1.2 MVVM 数据流 | `cx.notify()` 触发重渲染可工作 | 待验证 |
| §2.1 标签映射 | 12 个内置 HTML 标签可用 | 待实现 |
| §2.3 属性系统 | 4 类属性（标准/绑定/事件/指令）均支持 | 待实现 |
| §2.4 指令系统 | `if`/`each`/`key`/`model`/`show`/`once`/`ref` 可用（`else`/`html`/`slot` Phase B） | 待实现 |
| §2.5 插值 | 文本插值、属性插值、混合插值可用 | 待实现 |
| §3.1 单向绑定 | `{field}`、`{user.name}`、`{items.len()}` 可用 | 待实现 |
| §3.2 双向绑定 | `model={field}` 编译期生成 `on_change` | 待实现 |
| §4.1 ViewModel | `#[derive(IModel)]` + `#[view]` 可用 | 待实现 |
| §4.2 宏 | 8 个宏全部可解析（Phase A 多为 pass-through） | 待实现 |
| §4.4 命令 | `#[command]` 方法可被 `onclick` 调用 | 待实现 |
| §5.1 事件绑定 | `onclick`/`oninput`/`onchange` 等可绑定 | 待实现 |
| §5.2 事件对象 | `ClickEvent`/`InputEvent`/`ChangeEvent` 等可接收 | 待实现 |
| §10.4 build.rs | `rml::build().scan_dir().output_dir().build()` 可工作 | 待实现 |
| §10.6 代码生成 | `OUT_DIR/rml_generated/<name>.rs` 正确生成 | 待实现 |

### Phase B · 完整功能

| 文档章节 | 验收项 |
|---------|--------|
| §2.4 `else`/`html`/`slot` 指令 | 三个剩余指令可用 |
| §3.3 计算属性 | `#[computed]` 自动追踪依赖 + 缓存 |
| §3.4 值转换器 | `IConverter` 实现可在 `{x \| Converter}` 中使用 |
| §3.6 绑定引擎 | 编译期路径校验、运行时订阅 |
| §4.5 元素引用 | `#[element]` + `ref=` 联动可用 |
| §5.3-5.5 事件流 | 捕获/冒泡、`stop_propagation`、自定义事件 |
| §5.6 防抖节流 | 框架级 `Debounce` 原语 |
| §6 组件系统 | `#[component]`、Props、插槽、`#[on_prop_change]`、依赖注入 |
| §7 样式系统 | CSS 子集解析、主题切换、`cx.set_css_var`、Tailwind 互操作 |
| §8 生命周期 | `#[on_loaded]`/`#[on_unloaded]` 自动注入、异步任务管理 |
| §9.5 可测试性 | ViewModel 可脱离 GPUI 单测 |
| §10.1 性能优化 | `r:each` 稳定 key、虚拟列表、`cx.notify()` 合并 |
| §10.2 调试 | `cargo rml-expand`、`RML_LOG=debug`、`RML_DUMP_AST=1` |
| §10.3 热重载 | 文件监听 + IPC + 状态保留 |
| §10.5 IDE 支持 | LSP 协议接入（编译器纯转换层可复用） |

---

## 六、实施顺序与里程碑

### Phase A · MVP 闭环（按依赖顺序）

1. **Layer 0 补全** —— `rml-core` 修复差距（`IEvent`、`ICommand` 扩展、`IViewModel: ILifecycle`、事件对象字段对齐）
2. **Layer 1 实现** —— `rml-macros` 6 个模块文件（`derive_model` + `view` 真生成代码，其余 pass-through）
3. **Layer 2 实现** —— `rml/parser` + `rml/compiler` + `rml/tags`（HTML 子集 → GPUI 代码）
4. **Layer 3 实现** —— `rml/build` Builder API + 增量缓存
5. **Layer 5 实现** —— `rml-app` 启动器（单窗口）
6. **Layer 6 实现** —— `demo` counter 三件套
7. **验证** —— `cargo build` + `cargo run -p rml-demo` 通过

### Phase B · 完整功能

1. `rml/runtime` 事件流调度器
2. `rml-macros` 增强所有 pass-through 宏为实际功能（`#[computed]` 缓存、`#[element]` 字段绑定、`#[command]` 元信息）
3. `rml/compiler` 编译期 ViewModel 字段校验（通过 `IModel::rml_fields()`）
4. `rml/runtime` 组件注册表 + 插槽
5. `rml/runtime` 样式系统（CSS 解析）
6. `rml/runtime` 热重载 watcher
7. `rml/runtime` 依赖注入容器
8. 验收清单全部通过

---

## 七、验证步骤

### Phase A 验证

1. `cargo build -p rml-core` —— 编译通过
2. `cargo build -p rml-macros` —— 编译通过
3. `cargo build -p rml` —— 编译通过
4. `cargo build -p rml-app` —— 编译通过
5. `cargo build -p rml-demo` —— build.rs 触发 RML 编译，`OUT_DIR/rml_generated/counter.rs` 生成
6. `cargo run -p rml-demo` —— 窗口打开，显示计数器，点击 +/- 按钮数字变化
7. 修改 `counter.rml` 后重新 `cargo run`，UI 变化反映模板修改

### Phase B 验证

1. 实现一个 todo demo，覆盖 `each`/`key`/`model`/`if`/`#[computed]`/`#[element]`
2. 实现一个 login demo，覆盖 `#[on_loaded]`、异步 `cx.spawn`、错误显示
3. 实现一个 component demo，覆盖 `#[component]`、Props、插槽
4. 实现一个 theme demo，覆盖 CSS 变量、主题切换
5. 启用 `hot-reload` feature，验证模板修改热重载
6. 运行 `cargo rml-expand` 查看生成代码
7. 运行 `RML_LOG=debug cargo run` 验证日志
8. 运行 `RML_DUMP_AST=1 cargo build` 验证 AST dump

---

## 八、关键技术要点备忘

### 8.1 GPUI API 适配

- 使用 `Context<T>`（非 `ViewContext<T>`），`Entity<T>`、`cx.notify()`、`cx.listener()`、`cx.spawn(async move |this, mut cx| {...})`
- `Render::render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`
- `cx.new(|cx| Struct::new())` 创建 Entity
- `cx.open_window(options, |window, cx| cx.new(|cx| ...))` 创建窗口
- `Application::new().run(|cx| {...})` 启动应用

### 8.2 过程宏与 build.rs 的协同

- `#[view]` 宏生成 `include!(concat!(env!("OUT_DIR"), "/rml_generated/<name>.rs"))`
- `build.rs` 必须输出同名文件到 `OUT_DIR/rml_generated/<name>.rs`
- 文件名计算：`<snake_case_struct_name>.rs`（如 `Counter` → `counter.rs`）
- 模板路径计算：`<snake_case_struct_name>.rml`（如 `Counter` → `counter.rml`），从 `src/` 递归扫描查找

### 8.3 代码生成的元素 ID 稳定性

- 每个生成的元素需要稳定 ID（用于 GPUI diff 复用）
- ID 生成规则：`<view_struct>_<element_path_hash>`（如 `Counter_root_div_buttons_button_0`）
- `each` 内的元素 ID 加上 key 哈希

### 8.4 错误报告格式

- 包含文件名、行号、列号、源码片段高亮、修复建议
- 通过 `cargo:warning=...` 输出到 stderr
- 相似字段建议（编辑距离算法）

### 8.5 cargo:rerun-if-changed

- 对每个 `.rml` 文件打印 `cargo:rerun-if-changed=<path>`
- 对 `rml_cache.json` 打印 `cargo:rerun-if-changed=<path>`
- 对 build.rs 本身打印 `cargo:rerun-if-changed=build.rs`（cargo 自动）

---

## 九、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| GPUI API 不稳定 | 编译失败 | 锁定到具体 git rev，定期跟进 |
| gpui-component API 变化 | 代码生成不兼容 | 在 `tags.rs` 集中映射，便于适配 |
| 过程宏 + build.rs 协同复杂 | 编译失败难定位 | 详细的 `cargo:warning=` 错误输出 |
| 编译期 ViewModel 字段校验拿不到类型信息 | Phase B 校验受限 | Phase A 仅做语法校验，Phase B 通过 `IModel::rml_fields()` 元信息做运行时校验 |
| 热重载状态保留机制复杂 | Phase B 风险高 | Phase A 完全不实现热重载，Phase B 单独迭代 |

---

## 十、附录：文件清单

### 10.1 待新建文件

```
crates/core/src/event.rs                          # IEvent trait
crates/core/src/component.rs                      # IComponent trait
crates/macros/src/derive_model.rs                 # #[derive(IModel)]
crates/macros/src/view.rs                         # #[view] + #[component]
crates/macros/src/command.rs                      # #[command]
crates/macros/src/computed.rs                     # #[computed]
crates/macros/src/lifecycle.rs                    # #[on_loaded] + #[on_unloaded]
crates/macros/src/element.rs                      # #[element]
crates/rml/src/tags.rs                            # HTML 标签映射
crates/rml/src/parser/mod.rs                      # parser 入口
crates/rml/src/parser/tokenizer.rs                # 词法分析
crates/rml/src/parser/ast.rs                      # AST 定义
crates/rml/src/compiler/mod.rs                    # compiler 入口
crates/rml/src/compiler/validator.rs              # 语义验证
crates/rml/src/compiler/codegen.rs                # 代码生成
crates/rml/src/prelude.rs                         # rml prelude
crates/rml/src/build/mod.rs                       # RmlBuild Builder
crates/rml/src/build/cache.rs                     # 增量缓存
crates/rml/src/build/scanner.rs                   # 文件扫描
crates/rml/src/runtime/mod.rs                     # runtime 入口
crates/rml/src/runtime/event_flow.rs              # 事件流调度
crates/rml/src/runtime/component_registry.rs      # 组件注册表
crates/rml/src/runtime/styling.rs                 # 样式系统
crates/rml/src/runtime/watcher.rs                 # 热重载 watcher
crates/app/src/application.rs                     # RmlApplication
crates/app/src/window.rs                          # 窗口管理
crates/app/src/resources.rs                       # 资源加载
demo/Cargo.toml                                   # demo 配置
demo/build.rs                                     # demo build script
demo/src/main.rs                                  # demo 入口
demo/src/counter.rml                              # counter 模板
demo/src/counter.rml.rs                           # counter ViewModel
```

### 10.2 待修改文件

```
Cargo.toml                                         # workspace deps（已基本就绪）
crates/core/src/lib.rs                             # 新增 mod event/component
crates/core/src/prelude.rs                         # 新增导出 + ViewContext 别名
crates/core/src/view_model.rs                      # IViewModel: ILifecycle，移除重复方法
crates/core/src/command.rs                         # 扩展 ICommand 元信息
crates/core/src/events.rs                          # 补全字段 + 实现 IEvent
crates/core/Cargo.toml                             # 无需修改
crates/macros/Cargo.toml                           # 加入 rml-core 依赖
crates/macros/src/lib.rs                           # 无需修改（已就绪）
crates/rml/Cargo.toml                              # 加入 serde/walkdir/sha2 依赖
crates/rml/src/lib.rs                              # 新增 mod runtime
crates/app/Cargo.toml                              # 加入 gpui/rml-core 依赖
crates/app/src/lib.rs                              # 无需修改（已就绪）
```
