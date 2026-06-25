# RML 框架自底向上架构实施计划

> **目标**：从最底层（rml-core trait 层）开始，自底向上完成 RML 框架的全部实现，使其与 `docs/` 11 章文档的所有功能说明 100% 对齐。
> **当前状态**：Phase A MVP 已完成（rml-core/macros/rml/app/demo 骨架就位），但 workspace 编译存在阻断错误，且 Phase B 功能（指令系统、绑定引擎、组件、样式、生命周期、事件流、热重载、LSP、调试工具）均未实现。
> **铁律**（继承自既有决策，不可违反）：
> 1. 所有 trait 必须以 `I` 开头（`IModel`/`IViewModel`/`IRmlView`/`ICommand`/`IConverter`/`ITwoWayBinding`/`ILifecycle`/`IBindingContext`/`IEvent`/`IComponent`/`IPlugin`）。
> 2. `#![forbid(unsafe_code)]` 全 crate 启用。
> 3. 生成代码只写 `OUT_DIR`，禁止写 `src/`。
> 4. 过程宏不做重活，模板编译由 `build.rs` 完成。
> 5. `rml-core` 仅依赖 GPUI 基础类型。
> 6. **双轨制组件策略**：原生轨（HTML 标签 → GPUI 原生元素）+ 扩展轨（`crates/ui` + feature flag 引入 `gpui-component`）。
> 7. **热重载与 LSP 全部完整实现**（用户已确认）。

---

## 一、当前状态分析（基于实际代码探勘）

### 1.1 文档承诺的 13 大类功能 vs 实际完成度

| # | 功能区域 | 当前完成度 | 缺口 |
|---|---------|----------|------|
| 1 | 标签映射（真实 GPUI 构造器） | ~10% | `tags.rs` 19 个标签全部返回 `"gpui::div()"` |
| 2 | 指令系统（10 个指令完整语义） | ~5% | 解析完整，codegen 仅 `if`/`show`/`model` 部分实现且逻辑错误；`else`/`each`/`key`/`once`/`html`/`ref`/`slot` 全 stub |
| 3 | 插值与属性（表达式求值） | ~30% | 仅简单字段访问，无嵌套/方法调用/算术/转换器 |
| 4 | 绑定引擎（编译期校验 + 运行时订阅） | ~10% | 无表达式解析器，无编译期字段校验 |
| 5 | 计算属性（`#[computed]` 依赖追踪 + 缓存） | ~5% | pass-through，不生成缓存代码 |
| 6 | 转换器（IConverter 接入 codegen） | ~15% | trait 已定义，codegen 未接入 |
| 7 | 宏系统（生成实际代码而非 pass-through） | ~15% | `#[view]`/`#[derive(IModel)]` 生成 trait impl；`#[command]`/`#[computed]`/`#[on_loaded]`/`#[on_unloaded]` 全 pass-through |
| 8 | 元素引用（ref + ElementRef 注入） | ~20% | `ElementRef<T>` 类型完整，codegen 未注入 |
| 9 | 命令系统（ICommand impl 生成 + 校验） | ~15% | trait 已定义，`#[command]` 不生成 impl |
| 10 | 事件系统（三阶段流 + 类型映射 + 防抖节流） | ~25% | 事件对象 11 个完整，但 codegen 全部硬编码为 `gpui::ClickEvent`；三阶段流 stub |
| 11 | 组件系统（codegen + Props + 插槽 + DI） | ~10% | `IComponent` trait 完整，codegen 报错"not supported in Phase A" |
| 12 | 样式系统（CSS 子集 + 主题 + 变量） | ~0% | `runtime/styling.rs` 空文件 |
| 13 | 高级特性（热重载 + LSP + 调试 + 性能） | ~0% | `runtime/watcher.rs` 空文件，无 LSP/CLI crate |

### 1.2 阻断性编译错误（必须先修复）

经探勘确认以下 5 个编译错误阻断 workspace 构建：

1. **`#[view]` 宏 `include!` 位置错误**（`crates/macros/src/view.rs:168-173`）
   - 现状：`include!(...)` 包裹在 `const _: () = { include!(...); };` 中
   - 问题：被包含的 `OUT_DIR/rml_generated/counter.rs` 含 `impl gpui::Render for Counter { ... }`，impl 块不能出现在 const 表达式块中
   - 错误：`expected expression, found keyword 'impl'` + `non-statement macro in statement position: include`
   - 修复：将 `include!` 从 const 块中移出到模块顶层

2. **`quote!(#f.ty)` 潜在编译错误**（`crates/macros/src/view.rs:60` 与 `crates/macros/src/derive_model.rs:41`）
   - 现状：`let ty_str = quote!(#f.ty).to_string()`
   - 问题：`syn::Field` 未实现 `ToTokens`，`#f` 插值会失败
   - 修复：改为 `let ty = &f.ty; let ty_str = quote!(#ty).to_string();`

3. **`gpui::ClickEvent → rml_core::ClickEvent` 的 From 实现缺失**
   - 现状：codegen 生成 `_ev: &gpui::ClickEvent` 然后调 `&_ev.into()` 传给用户命令方法
   - 问题：rml-core 中未找到 `impl From<gpui::ClickEvent> for rml_core::ClickEvent`
   - 修复：在 `crates/rml/src/runtime/event_flow.rs` 实现完整转换

4. **codegen 的 `if`/`show` 指令逻辑错误**（`crates/rml/src/compiler/codegen.rs`）
   - 现状：`.when(self.X, |el| el)`
   - 问题：GPUI `when` 的语义是"条件为真时执行闭包"，并非"条件为假时不渲染"，实际不会隐藏元素
   - 修复：`if` 改为 `if self.X { Some(element) } else { None }`；`show` 改为 `.when(self.X, |el| el).when(!self.X, |el| el.class("hidden"))` 或 GPUI 等效 API

5. **事件类型全部硬编码为 `gpui::ClickEvent`**
   - 现状：codegen 中所有事件回调都用 `gpui::ClickEvent`
   - 问题：`oninput`/`onchange`/`onkeydown` 等需要不同事件类型
   - 修复：建立事件名 → 事件类型映射表（见 Layer 2 §3.2.3.C）

### 1.3 既有 crate 状态总览

| Crate | complete | partial | stub | 关键问题 |
|-------|----------|---------|------|---------|
| rml-core | 12 | 2（binding、model、command） | 0 | binding 仅 1 方法；command 缺 can_execute；model 默认空切片 |
| rml-macros | 1（lib.rs） | 5（view/derive_model 有 bug；command/computed/lifecycle 全 pass-through） | 0 | include! 位置错误；#f.ty 插值问题 |
| rml | 9 | 3（tags、codegen、validator） | 4（runtime/* 全空） | tags 全 div；codegen 指令/事件/组件未实现；runtime 全 stub |
| rml-app | 2（lib、application） | 0 | 2（window、resources） | 单窗口可用，多窗口/资源加载 stub |
| demo | 4 | 0 | 0 | counter 三件套可运行（修复编译错误后） |

---

## 二、自底向上架构实施（按 Layer 顺序）

### Layer 0 · rml-core trait 层补全

**目标**：扩展 trait 以支撑 Phase B 全部特性，建立稳固的契约基础。

#### 2.0.1 修改 `crates/core/src/command.rs` —— ICommand 扩展

```rust
pub trait ICommand {
    fn rml_command_name() -> &'static str;
    fn rml_event_type() -> &'static str { "" }
    fn rml_params() -> &'static [ParamMeta] { &[] }
    /// 命令是否可执行（用于禁用按钮等，文档 §4.4）
    fn can_execute(&self) -> bool { true }
}
```

**改动**：新增 `can_execute(&self) -> bool` 默认实现。

#### 2.0.2 修改 `crates/core/src/binding.rs` —— BindingPath 扩展

```rust
pub enum BindingSegment {
    Field(String),
    Member(String),
    Index(String),
    MethodCall(String, Vec<String>),
}

impl BindingPath {
    pub fn parse(expr: &str) -> Result<BindingPath, BindingError> { /* 支持 user.name / items[0] / items.len() / count + 1 */ }
}
```

**改动**：扩展 `BindingPath::parse` 支持 Index/MethodCall/算术表达式。

#### 2.0.3 新建 `crates/core/src/plugin.rs` —— IPlugin trait

```rust
pub trait IPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_compile(&self, ctx: &mut PluginContext) {}
    fn on_render(&self, ctx: &mut RenderContext) {}
}

pub struct PluginContext { /* 编译期上下文 */ }
pub struct RenderContext { /* 渲染期上下文 */ }
```

**改动**：新增文件，用于 LSP/调试器/热重载扩展点。

#### 2.0.4 修改 `crates/core/src/lib.rs` + `prelude.rs`

新增 `pub mod plugin;`，导出 `IPlugin`/`PluginContext`/`RenderContext` 到 prelude。

#### 2.0.5 修改 `crates/core/src/events.rs` —— 补全字段

- `FocusEvent.target` 由 `Option<gpui::EntityId>` 填充
- `SubmitEvent.form_data` 由 `HashMap<SharedString, SharedString>` 填充

**改动**：从 stub 改为实际字段，提供构造方法供 runtime 填充。

---

### Layer 1 · rml-macros 过程宏完整实现

**目标**：从 pass-through 升级为生成实际代码，并修复阻断性 bug。

#### 2.1.1 修复 `crates/macros/src/view.rs` —— include! 位置 + #f.ty 问题

**关键修复 1**：将 `include!` 从 const 块中移出：

```rust
// 修复前
let include_stmt = quote! {
    #[allow(...)]
    const _: () = {
        include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
    };
};

// 修复后
let include_stmt = quote! {
    #[allow(non_snake_case, unused_imports, unused_variables, dead_code)]
    include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
};
```

**关键修复 2**：`gen_impl_i_model` 中 `#f.ty` 改为 `let ty = &f.ty; quote!(#ty)`。

**新增功能**：
- 扫描 `#[on_loaded]`/`#[on_unloaded]` 标记方法（通过方法名约定 `__rml_on_loaded_impl`）
- 在 `impl ILifecycle` 中调用 `self.__rml_on_loaded_impl(cx)`
- 收集 `#[element]` 字段，生成 `__rml_bind_elements` 方法
- `#[component]` 识别 `#[prop(default = ...)]` 与 `#[on_prop_change(field)]`

#### 2.1.2 修改 `crates/macros/src/command.rs` —— 生成 ICommand impl

```rust
// 用户代码
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) { ... }

// 宏生成
impl ICommand for Counter {
    fn rml_command_name() -> &'static str { "increment" }
    fn rml_event_type() -> &'static str { "ClickEvent" }
    fn rml_params() -> &'static [ParamMeta] { &[] }
}
```

**改动**：解析方法签名，提取事件类型（第二个参数类型名）和参数列表，生成 `impl ICommand`。

#### 2.1.3 修改 `crates/macros/src/computed.rs` —— 依赖追踪 + 缓存

```rust
// 用户代码
#[computed]
fn full_name(&self) -> String { format!("{} {}", self.first, self.last) }

// 宏生成
fn full_name(&self) -> String {
    let cache = &self.__cache_full_name;
    if cache.v_first == self.__v_first && cache.v_last == self.__v_last {
        if let Some(v) = &cache.value { return v.clone(); }
    }
    let v = self.__full_name_impl();
    // 更新缓存（通过 RefCell 内部可变性）
    v
}
fn __full_name_impl(&self) -> String { format!("{} {}", self.first, self.last) }
```

**改动**：使用 `syn::visit::Visit` 遍历方法体 AST，收集 `self.field` 访问，生成缓存结构（用 `RefCell<__Cache_<method>>` 实现内部可变性）。

#### 2.1.4 修改 `crates/macros/src/lifecycle.rs` —— 方法重命名机制

将 `#[on_loaded] fn setup(...)` 重命名为 `fn __rml_on_loaded_impl(...)`，供 `#[view]` 调用。

#### 2.1.5 新建 `crates/macros/src/prop.rs` —— #[prop] + #[on_prop_change]

实现 helper attribute：
- `#[prop(default = ...)]`：标记组件属性字段，生成默认值构造
- `#[on_prop_change(field)]`：生成属性变化回调

#### 2.1.6 修改 `crates/macros/src/lib.rs`

声明新 helper attributes：`prop`、`on_prop_change`。

#### 2.1.7 修改 `crates/macros/src/derive_model.rs`

修复 `#f.ty` 插值问题（同 2.1.1）。

---

### Layer 2 · rml/parser + rml/compiler + rml/tags 完整实现

**目标**：让 `.rml` → Rust 代码的转换覆盖所有文档语法。

#### 2.2.1 修改 `crates/rml/src/tags.rs` —— 真实 GPUI 构造器映射

```rust
impl BuiltinTag {
    pub fn codegen_ctor(self) -> &'static str {
        match self {
            BuiltinTag::Div => "gpui::div()",
            BuiltinTag::Span => "gpui::div().inline()",
            BuiltinTag::P => "gpui::div()",
            BuiltinTag::H1 => "gpui::div().text_size(32.)",
            BuiltinTag::H2 => "gpui::div().text_size(28.)",
            BuiltinTag::H3 => "gpui::div().text_size(24.)",
            BuiltinTag::H4 => "gpui::div().text_size(20.)",
            BuiltinTag::H5 => "gpui::div().text_size(16.)",
            BuiltinTag::H6 => "gpui::div().text_size(14.)",
            BuiltinTag::Button => "gpui::div()",  // 原生轨：用 div + class
            BuiltinTag::Input => "gpui::div()",   // 原生轨：简化
            BuiltinTag::TextArea => "gpui::div()",
            BuiltinTag::Ul => "gpui::div().flex().flex_col()",
            BuiltinTag::Ol => "gpui::div().flex().flex_col()",
            BuiltinTag::Li => "gpui::div()",
            BuiltinTag::Img => "gpui::div()",
            BuiltinTag::A => "gpui::div()",
            BuiltinTag::Label => "gpui::div()",
            BuiltinTag::Br => "gpui::div().h_0()",
        }
    }
}
```

**改动**：从全 div 改为按标签类型返回真实 GPUI 调用链。原生轨 button/input 用 div + class 应用样式；扩展轨在 `crates/ui` 中覆盖映射。

#### 2.2.2 新建 `crates/rml/src/compiler/expr.rs` —— 表达式解析器

```rust
pub enum Expr {
    Field(String),                              // count
    Member(Box<Expr>, String),                  // user.name
    Index(Box<Expr>, String),                   // items[0]
    MethodCall(Box<Expr>, String, Vec<Expr>),   // items.len()
    BinaryOp(Op, Box<Expr>, Box<Expr>),         // count + 1
    Lit(String),                                // "hello" / 42
    Convert(Box<Expr>, String),                 // count, HexConverter
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),   // cond ? a : b
}

pub fn parse(s: &str) -> Result<Expr, ParseError>;
pub fn to_rust_code(expr: &Expr) -> String;  // 生成 self.count + 1 等
```

**改动**：新建文件，实现文档 §2.5 表达式子集（字段访问/嵌套/方法调用/算术/比较/转换器/三元）。

#### 2.2.3 修改 `crates/rml/src/compiler/codegen.rs` —— 核心重写

**A. 元素 ID 生成**（文档 §10.6.3）：

```rust
fn gen_element_id(view_name: &str, path: &[usize]) -> String {
    let path_str = path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("_");
    format!("{}_{}", view_name, path_str)
}
```

**B. 指令处理**（按文档 §2.4.10 顺序：each → if → key → 其他）：

- `each`：生成 `self.{iterable}.iter().enumerate().map(|({index}, {item})| { body }).collect::<Vec<_>>()`
- `if`：生成 `if self.{cond} { Some(element) } else { None }`
- `else`：兄弟节点配对处理（`gen_children_with_else`）
- `key`：配合 each，生成 `.key(|(_, item)| item.{key}.clone())`
- `model`：生成双向绑定 `.value(self.{field}.clone()).on_change(cx.listener(|this, v, cx| { this.{field} = v; cx.notify(); }))`
- `show`：生成 `.when(self.{cond}, |el| el).when(!self.{cond}, |el| el.class("hidden"))`
- `once`：标记一次性渲染，用 `RefCell<bool>` 控制
- `html`：生成 `.child(rml::runtime::html::parse(&self.{expr}))`
- `ref`：生成元素创建后 `self.{name}.set(handle)`
- `slot`：组件内位置插入插槽内容

**C. 事件类型映射表**：

```rust
fn event_type_name(event_name: &str) -> &'static str {
    match event_name {
        "onclick" | "ondblclick" => "ClickEvent",
        "onmousedown" | "onmouseup" | "onmouseenter" | "onmouseleave" | "onmousemove" => "MouseEvent",
        "onwheel" => "WheelEvent",
        "oninput" => "InputEvent",
        "onchange" => "ChangeEvent",
        "onkeydown" => "KeyDownEvent",
        "onkeyup" => "KeyUpEvent",
        "onfocus" | "onblur" => "FocusEvent",
        "onsubmit" => "SubmitEvent",
        "onload" => "LoadEvent",
        "onresize" => "ResizeEvent",
        "onscroll" => "ScrollEvent",
        _ => "ClickEvent",
    }
}
```

**D. GPUI→RML 事件转换**：

```rust
// 生成的事件回调代码
".on_click(cx.listener(move |this, ev: &gpui::ClickEvent, cx| {
    let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(ev);
    this.increment(&rml_ev, cx);
}))"
```

**E. 兄弟节点 if/else 配对**：新建 `gen_children_with_else` 函数。

**F. 组件 codegen**：

```rust
fn gen_component(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    // <MyComp prop="x" on_click={fn}>...</MyComp>
    // 生成: MyComp::new().prop("x").on_click(handler).children([...])
}
```

**G. 样式属性处理**：`style="color: red"` 内联样式直接应用，`class="card {dyn}"` 混合 class 用 `format!`。

#### 2.2.4 修改 `crates/rml/src/compiler/validator.rs` —— 编译期校验

新增校验项：
1. `else` 必须紧跟 `if` 元素（兄弟节点）
2. `each` 的 `key` 建议存在（警告）
3. `model` 只能用于 `input`/`textarea`/`checkbox` 或实现 `ITwoWayBinding` 的组件
4. `ref` 名同视图内唯一
5. `slot` 必须在组件内（PascalCase 标签）
6. 绑定路径校验（通过 `IModel::rml_fields()` 元信息，Phase B-2）
7. 事件绑定校验（通过 `ICommand` 注册表，Phase B-2）

#### 2.2.5 修改 `crates/rml/src/compiler/mod.rs`

接入表达式解析器，扩展 `CodegenCtx` 含样式表引用与组件注册表。

---

### Layer 3 · rml/build 构建流程增强

#### 2.3.1 修改 `crates/rml/src/build/mod.rs`

**新增配置**：
```rust
pub struct Builder {
    scan_dirs: Vec<PathBuf>,
    style_dirs: Vec<PathBuf>,  // 新增：扫描 src/styles/*.css
    output_dir: Option<PathBuf>,
    namespace: Option<String>,
    strict: bool,
    hot_reload: bool,
    public: bool,
}

impl Builder {
    pub fn style_dir(mut self, dir: impl Into<PathBuf>) -> Self { ... }
}
```

**新增功能**：
- A. 样式表扫描：扫描 `.css` 文件，调用 `styling::parser` 解析，序列化为 Rust 常量写入 `OUT_DIR/rml_generated/styles.rs`
- B. namespace 生效：注入生成代码的模块路径
- C. 热重载支持：`hot_reload(true)` 时启动监听线程
- D. 调试输出：支持 `RML_DUMP_AST=1` 环境变量，输出 AST 到 stderr
- E. `cargo:rerun-if-changed` 注册 `.css` 文件

---

### Layer 4 · rml/runtime 运行时完整实现

**目标**：从全 stub 升级为完整运行时，覆盖事件流、组件注册表、样式、热重载。

#### 2.4.1 实现 `crates/rml/src/runtime/event_flow.rs` —— 三阶段事件调度

```rust
pub enum EventPhase { Capture, Target, Bubble }

pub struct EventDispatcher { /* 事件路径（root → target 元素链） */ }

impl EventDispatcher {
    pub fn dispatch(&mut self, event: &mut dyn IEvent, cx: &mut App) {
        // 1. 捕获阶段：root → target 父节点
        // 2. 目标阶段
        // 3. 冒泡阶段：target 父节点 → root
        // 每阶段检查 is_propagation_stopped()
    }
}

/// GPUI 事件 → RML 事件转换（修复 From 实现缺失）
pub mod convert {
    pub fn from_gpui_click(ev: &gpui::ClickEvent) -> ClickEvent { ... }
    pub fn from_gpui_input(ev: &gpui::InputEvent) -> InputEvent { ... }
    pub fn from_gpui_key_down(ev: &gpui::KeyDownEvent) -> KeyDownEvent { ... }
    // ... 完整映射
}
```

**改动**：从空文件改为完整实现，提供 GPUI→RML 事件转换函数（修复阻断错误 #3）。

#### 2.4.2 实现 `crates/rml/src/runtime/component_registry.rs` —— 全局组件注册表

```rust
pub struct ComponentEntry {
    pub tag: String,
    pub constructor: fn() -> Box<dyn IRmlView>,
    pub props_meta: Vec<PropMeta>,
    pub slots: Vec<String>,
}

pub struct ComponentRegistry { entries: HashMap<String, ComponentEntry> }

impl ComponentRegistry {
    pub fn register(&mut self, entry: ComponentEntry);
    pub fn lookup(&self, tag: &str) -> Option<&ComponentEntry>;
    pub fn list(&self) -> Vec<&ComponentEntry>;
}

pub static GLOBAL_REGISTRY: RwLock<ComponentRegistry> = RwLock::new(...);

#[macro_export]
macro_rules! register_component {
    ($ty:ty, $tag:literal) => {
        inventory::submit! { /* ... */ }
    };
}
```

**改动**：从空文件改为完整注册表，使用 `inventory` crate 实现编译期注册。

#### 2.4.3 实现 `crates/rml/src/runtime/styling.rs` —— CSS 子集解析

新建 `crates/rml/src/styling/` 模块目录：
```
styling/
├── mod.rs           # 模块入口 + re-export
├── lexer.rs         # CSS 词法分析
├── parser.rs        # CSS 语法分析
├── selector.rs      # 选择器匹配（标签/类/ID/后代/子/属性/伪类）
├── properties.rs    # CSS 属性 → GPUI 调用映射
├── variable.rs      # CSS 变量表
└── theme.rs         # 主题切换
```

**核心结构**：
```rust
pub struct StyleSheet { pub rules: Vec<Rule> }
pub struct Rule { pub selectors: Vec<Selector>, pub declarations: Vec<Declaration> }
pub enum Selector {
    Tag(String), Class(String), Id(String),
    Descendant(Box<Selector>, Box<Selector>),
    Child(Box<Selector>, Box<Selector>),
    Attribute(String, Option<AttrOp>, Option<String>),
    PseudoClass(String), Group(Vec<Selector>),
}
pub enum Value {
    Length(f32, Unit), Color(Rgba), String(String),
    Var(String, Option<Box<Value>>), Calc(Box<Expr>),
}
```

**样式应用**：codegen 时查询 `StyleSheet`，将匹配声明转为 GPUI 调用链（`.bg()`/`.p()`/`.flex()` 等）。

**主题切换**：`pub fn set_theme(name: &str, cx: &mut App)` 切换全局变量表，触发 `cx.notify()`。

**改动**：`runtime/styling.rs` 改为 re-export `styling/` 模块。

#### 2.4.4 实现 `crates/rml/src/runtime/watcher.rs` —— 热重载

```rust
use notify::{Watcher, RecursiveMode, Event};

pub struct HotReloader {
    watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<Event>,
    callback: Box<dyn Fn(PathBuf) + Send>,
}

impl HotReloader {
    pub fn new<F>(dirs: Vec<PathBuf>, callback: F) -> std::io::Result<Self>;
    pub fn poll(&self, cx: &mut App);
}
```

**热重载流程**：
1. `build.rs` 在 `hot_reload(true)` 时启动监听线程
2. 检测到 `.rml` 变化 → 增量解析 → 语义验证 → 重新生成渲染代码 → 写入内存缓冲
3. 通过 IPC（Unix socket / Windows Named Pipe）通知运行中应用
4. 应用接收新代码 → 通过 `Entity::update` + `cx.notify()` 触发重渲染
5. ViewModel 状态保留，仅 View 重建
6. 失败不崩溃，保持上一有效状态，窗口角落显示错误

#### 2.4.5 新建 `crates/rml/src/runtime/html.rs` —— HTML 字符串解析

用于 `html={raw}` 指令，使用 `html5ever` 或简化解析器，解析为 GPUI 元素树。

#### 2.4.6 新建 `crates/rml/src/runtime/debounce.rs` —— 防抖/节流

```rust
pub fn debounce<F>(ms: u64, f: F) -> impl Fn()
where F: Fn() + 'static;

pub fn throttle<F>(ms: u64, f: F) -> impl Fn()
where F: Fn() + 'static;
```

基于 `cx.spawn` + `timer` 实现。

#### 2.4.7 修改 `crates/rml/src/runtime/mod.rs`

声明新模块：`pub mod html; pub mod debounce;`，re-export 公共 API。

#### 2.4.8 修改 `crates/rml/src/lib.rs`

新增 `pub mod styling;`。

---

### Layer 5 · crates/ui 扩展组件库（新增 crate）

**目标**：封装 `gpui-component`，提供 Dialog/List/Modal 等复杂组件的 RML 标签映射。

#### 2.5.1 新建 `crates/ui/Cargo.toml`

```toml
[package]
name = "rml-ui"
version.workspace = true
edition.workspace = true
license.workspace = true

[features]
default = []
ui-components = ["gpui-component"]

[dependencies]
gpui = { workspace = true }
gpui-component = { workspace = true, optional = true }
rml = { workspace = true }
rml-core = { workspace = true }
```

#### 2.5.2 新建 `crates/ui/src/lib.rs`

```rust
#![forbid(unsafe_code)]

#[cfg(feature = "ui-components")]
pub mod dialog;
#[cfg(feature = "ui-components")]
pub mod list;
#[cfg(feature = "ui-components")]
pub mod form;  // Checkbox/Radio/Switch/Select

/// 注册扩展组件到全局注册表
pub fn register() {
    #[cfg(feature = "ui-components")]
    {
        rml::runtime::component_registry::register::<Dialog>("Dialog");
        rml::runtime::component_registry::register::<List>("List");
        // ...
    }
}
```

#### 2.5.3 各组件适配文件

`dialog.rs`/`list.rs`/`form.rs`：每个文件封装一个 `gpui-component` 组件，实现 `IRmlView` + `IComponent` + `Render`。

---

### Layer 6 · rml-app 应用启动器增强

#### 2.6.1 修改 `crates/app/src/application.rs` —— 多窗口 + 扩展注册

```rust
impl RmlApplication {
    pub fn new() -> Self;
    pub fn title(mut self, t: impl Into<SharedString>) -> Self;
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self;
    pub fn with_global<G: Global + 'static>(mut self, global: G) -> Self;
    pub fn with_extensions(mut self, f: impl FnOnce(&mut Self)) -> Self;
    pub fn run<R: IRmlView + Render + Default + 'static>(self);
}
```

`run` 内部：
1. 注册内置组件
2. 注册扩展组件（`#[cfg(feature = "ui-components")] rml_ui::register()`）
3. 打开主窗口

#### 2.6.2 实现 `crates/app/src/window.rs` —— 多窗口管理

```rust
pub struct WindowManager { windows: Vec<WindowHandle> }
impl WindowManager {
    pub fn open<R: IRmlView + Render + Default + 'static>(&mut self, cx: &mut App) -> WindowHandle;
    pub fn close(&mut self, handle: WindowHandle);
    pub fn list(&self) -> &[WindowHandle];
}
```

#### 2.6.3 实现 `crates/app/src/resources.rs` —— 资源加载

```rust
pub struct Resources { assets_dir: PathBuf, cache: HashMap<String, Vec<u8>> }
impl Resources {
    pub fn load(&mut self, path: &str) -> std::io::Result<&[u8]>;
    pub fn load_string(&mut self, path: &str) -> std::io::Result<String>;
}
```

---

### Layer 7 · crates/lsp LSP 服务器（新增 crate）

**目标**：实现 LSP 协议服务器，提供语法诊断、自动补全、悬停提示、跳转定义、格式化。

#### 2.7.1 新建 `crates/lsp/Cargo.toml`

```toml
[package]
name = "rml-lsp"
version.workspace = true
edition.workspace = true

[[bin]]
name = "rml-lsp"
path = "src/main.rs"

[dependencies]
rml = { workspace = true }
rml-core = { workspace = true }
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
```

#### 2.7.2 新建 `crates/lsp/src/main.rs`

使用 `tower-lsp` 实现 LSP 服务器（`--stdio`）：
- `did_open`：解析 `.rml`，发布诊断
- `completion`：标签/属性/指令/绑定路径/命令补全
- `hover`：标签/属性文档
- `definition`：跳转定义（组件/字段/命令）
- `formatting`：格式化（2 空格缩进、100 字符换行、自闭合标签）

#### 2.7.3 新建 `crates/lsp/src/diagnostics.rs`

将 RML 解析/验证错误转换为 LSP `Diagnostic`（含行号/列号/严重级别/来源）。

#### 2.7.4 新建 `crates/lsp/src/completion.rs`

基于项目实际类型信息（非语法猜测）：
- 标签补全：HTML 标准标签 + 项目自定义组件 + 内置组件
- 属性补全：标准属性 + 组件 props + RML 指令
- 绑定路径补全：在 `model="|"` 或 `{ | }` 中补全 ViewModel 字段与计算属性
- 命令补全：在事件绑定中补全 `#[command]` 方法

---

### Layer 8 · crates/cli CLI 工具（新增 crate）

#### 2.8.1 新建 `crates/cli/Cargo.toml` + `src/main.rs`

`rml` CLI 工具，子命令：
- `cargo rml-expand <path>`：读取 `OUT_DIR/rml_generated/<name>.rs` 并输出
- `cargo rml-lint src/`：扫描 `.rml`，运行 validator，输出错误
- `cargo rml-format src/`：格式化 `.rml` 文件
- `cargo rml-check`：只做语义检查不生成代码（适合 CI）
- `cargo rml-dump-tree`：导出渲染树为文本

#### 2.8.2 环境变量支持

在 `rml/runtime` 中：
- `RML_LOG=debug`：启用绑定追踪日志（codegen 插入 `log::debug!`）
- `RML_DUMP_AST=1`：build.rs 输出 AST 到 stderr
- `RML_PROFILE=1`：记录 render 耗时
- `RML_TRACE_BINDING=1`：追踪绑定重算频率

#### 2.8.3 RML Inspector

运行时 `Ctrl+Shift+I` 打开，查看渲染树、绑定字段与当前值、样式来源、事件监听器、元素 ID。

---

## 三、分阶段实施计划

### Phase B-1 · 核心 codegen 完成 + 编译修复（优先级最高）

**目标**：修复编译错误，让 counter demo 运行，覆盖基础语法。

**任务**：
1. **修复 `#[view]` 宏 include! 位置**（2.1.1）— 解除编译阻断
2. **修复 `#f.ty` 插值问题**（2.1.1, 2.1.7）
3. **实现 GPUI→RML 事件转换**（2.4.1）— 修复 From 实现缺失
4. **标签映射**（2.2.1）：button/input/img/h1-h6 映射到真实 GPUI 构造器
5. **表达式解析器**（2.2.2）：支持字段访问、方法调用、算术
6. **if/each/show 指令**（2.2.3 B）：条件创建、列表遍历、显隐控制（修复逻辑错误 #4）
7. **事件类型映射**（2.2.3 C/D）：按事件名生成对应 RML 事件对象（修复硬编码 #5）
8. **生命周期联动**（2.1.1, 2.1.4）：`#[on_loaded]`/`#[on_unloaded]` 真正触发
9. **命令系统**（2.1.2）：`#[command]` 生成 ICommand impl
10. **元素 ID 生成**（2.2.3 A）：稳定 ID 用于 GPUI diff

**验证**：
- `cargo build --workspace` 全通过
- `cargo run -p rml-demo` counter demo 运行，点击 +/- 数字变化
- 扩展 demo 含 if/each/show，条件渲染、列表遍历正确
- `#[on_loaded]` 回调被调用（通过日志验证）
- 事件对象类型正确（ClickEvent/InputEvent/KeyDownEvent）

### Phase B-2 · 数据绑定完整

**目标**：实现 WPF 级数据绑定。

**任务**：
1. **model 双向绑定**（2.2.3 B）：input/textarea 双向数据流
2. **计算属性缓存**（2.1.3）：`#[computed]` 依赖追踪 + 缓存
3. **编译期字段校验**（2.2.4）：validator 校验绑定路径存在
4. **ref 元素引用注入**（2.2.3 B, 2.1.1）：运行时注入 Entity handle
5. **转换器**（2.2.2, 2.2.3）：`{expr, Converter}` 语法
6. **BindingPath 扩展**（2.0.2）：支持 Index/MethodCall
7. **else/html/once 指令**（2.2.3 B）：剩余指令
8. **ICommand.can_execute**（2.0.1）：命令启用条件

**验证**：实现 todo demo，覆盖 each/key/model/if/#[computed]/#[element]。

### Phase B-3 · 组件与样式

**目标**：组件系统 + 样式系统完整可用。

**任务**：
1. **组件 codegen**（2.2.3 F）：PascalCase 标签编译为子组件
2. **组件注册表**（2.4.2）：全局注册 + inventory
3. **Props 系统**（2.1.5）：`#[prop]` + 默认值 + 响应式
4. **`#[on_prop_change]`**（2.1.5）：属性变化回调
5. **插槽系统**（2.2.3 B）：默认/具名/作用域插槽
6. **CSS 子集解析**（2.4.3）：lexer + parser + selector
7. **样式应用**（2.4.3）：codegen 查询样式表生成 GPUI 调用
8. **主题系统**（2.4.3）：CSS 变量 + 主题切换
9. **`crates/ui` 扩展**（Layer 5）：封装 gpui-component
10. **多窗口管理**（2.6.2）：WindowManager
11. **资源加载**（2.6.3）：Resources

**验证**：
- 实现 component demo，覆盖 `#[component]`、Props、插槽
- 实现 theme demo，覆盖 CSS 变量、主题切换
- `features = ["ui-components"]` 启用后，`<Dialog>` 标签可用

### Phase B-4 · 高级特性

**目标**：调试、热重载、性能、LSP。

**任务**：
1. **事件流三阶段**（2.4.1）：捕获/冒泡/stop_propagation
2. **自定义事件**（2.4.x）：`Option<Arc<dyn Fn>>` 字段
3. **防抖/节流**（2.4.6）：debounce/throttle 原语
4. **热重载**（2.4.4）：文件监听 + IPC + 状态保留
5. **LSP 服务器**（Layer 7）：诊断 + 补全 + 悬停 + 跳转 + 格式化
6. **CLI 工具**（Layer 8）：rml-expand/rml-lint/rml-format/rml-check/rml-dump-tree
7. **日志与性能**（2.8.2）：RML_LOG/RML_DUMP_AST/RML_PROFILE/RML_TRACE_BINDING
8. **RML Inspector**（2.8.3）：Ctrl+Shift+I 运行时检查
9. **VirtualList**：基于 GPUI uniform_list 的虚拟滚动
10. **IPlugin 接口**（2.0.3）：扩展点

**验证**：
- 实现 login demo，覆盖 `#[on_loaded]`、异步 `cx.spawn`、错误显示
- 启用 hot-reload，验证模板修改热重载
- 运行 `cargo rml-expand` 查看生成代码
- 运行 `RML_LOG=debug cargo run` 验证日志
- VS Code 安装 RML 插件，打开 `.rml` 文件，语法诊断 + 补全可用

---

## 四、假设与决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 组件实现层 | 双轨制 | 基础标签原生 GPUI，复杂组件通过 `crates/ui` 按需引入 gpui-component |
| 表达式求值 | 自实现子集解析器 | 避免引入完整 Rust 表达式解析器，控制复杂度 |
| 计算属性缓存 | RefCell 内部可变性 | `#[computed]` 签名是 `&self`，需 interior mutability |
| 生命周期联动 | 方法重命名 + 调用 | 简单可靠，避免 inventory 复杂度 |
| CSS 解析 | 自实现子集 | 文档明确为 CSS 子集，无需完整 CSS 解析器 |
| 热重载 IPC | Unix socket / Named Pipe | 跨平台，性能足够 |
| LSP 实现 | tower-lsp + tokio | 成熟生态，复用 rml::parser |
| 组件注册 | inventory crate + 全局注册表 | 编译期注册，零运行时开销 |
| 稳定元素 ID | `<view>_<path_hash>` | 用于 GPUI diff 复用，支持 each + key |
| 指令前缀 | 零前缀（if/each/model）| 以第 2 章语法权威章节为准 |
| 事件属性 | `on*` 无冒号（onclick/oninput）| 以第 2 章语法权威章节为准 |

---

## 五、文件清单

### 5.1 待新建文件

```
# Layer 0 - rml-core 补全
crates/core/src/plugin.rs                         # IPlugin trait

# Layer 1 - rml-macros 完整实现
crates/macros/src/prop.rs                         # #[prop] + #[on_prop_change]

# Layer 2 - rml/compiler 完整实现
crates/rml/src/compiler/expr.rs                   # 表达式解析器

# Layer 4 - rml/runtime 完整实现
crates/rml/src/runtime/html.rs                    # HTML 字符串解析
crates/rml/src/runtime/debounce.rs                # 防抖/节流
crates/rml/src/styling/mod.rs                     # 样式系统入口
crates/rml/src/styling/lexer.rs                   # CSS 词法分析
crates/rml/src/styling/parser.rs                  # CSS 语法分析
crates/rml/src/styling/selector.rs                # 选择器匹配
crates/rml/src/styling/properties.rs              # CSS 属性映射
crates/rml/src/styling/variable.rs                # CSS 变量表
crates/rml/src/styling/theme.rs                   # 主题切换

# Layer 5 - crates/ui 扩展组件库
crates/ui/Cargo.toml
crates/ui/src/lib.rs
crates/ui/src/dialog.rs
crates/ui/src/list.rs
crates/ui/src/form.rs

# Layer 7 - LSP 服务器
crates/lsp/Cargo.toml
crates/lsp/src/main.rs
crates/lsp/src/diagnostics.rs
crates/lsp/src/completion.rs
crates/lsp/src/hover.rs

# Layer 8 - CLI 工具
crates/cli/Cargo.toml
crates/cli/src/main.rs

# 新增 demo
demo/src/todos.rml                                # todo 列表 demo
demo/src/todos.rml.rs
demo/src/login.rml                                # 登录 demo
demo/src/login.rml.rs
demo/src/components/todo_item.rml                 # 自定义组件 demo
demo/src/components/todo_item.rml.rs
demo/src/styles/theme.css                         # 主题样式
demo/src/theme_demo.rml                           # 主题切换 demo
demo/src/theme_demo.rml.rs
```

### 5.2 待修改文件

```
# Layer 0 - rml-core
crates/core/src/command.rs                        # ICommand 增加 can_execute
crates/core/src/binding.rs                        # BindingPath 支持 Index/MethodCall
crates/core/src/events.rs                         # 补全 FocusEvent.target / SubmitEvent.form_data
crates/core/src/lib.rs                            # 新增 mod plugin
crates/core/src/prelude.rs                        # 导出 IPlugin

# Layer 1 - rml-macros
crates/macros/src/lib.rs                          # 声明 prop/on_prop_change helper
crates/macros/src/view.rs                         # 修复 include! + #f.ty + 生命周期联动 + element 注入 + props
crates/macros/src/command.rs                      # 生成 ICommand impl
crates/macros/src/computed.rs                     # 依赖追踪 + 缓存代码
crates/macros/src/lifecycle.rs                    # 方法重命名机制
crates/macros/src/derive_model.rs                 # 修复 #f.ty 插值

# Layer 2 - rml/compiler + tags
crates/rml/src/tags.rs                            # 真实 GPUI 构造器映射
crates/rml/src/compiler/codegen.rs                # 完整重写：指令/事件/组件/ID/样式
crates/rml/src/compiler/validator.rs              # 编译期字段/类型校验
crates/rml/src/compiler/mod.rs                    # 接入表达式解析器

# Layer 3 - rml/build
crates/rml/src/build/mod.rs                       # style_dir + namespace + hot_reload + RML_DUMP_AST

# Layer 4 - rml/runtime
crates/rml/src/runtime/event_flow.rs              # 三阶段调度 + GPUI→RML 事件转换
crates/rml/src/runtime/component_registry.rs      # 全局注册表 + inventory
crates/rml/src/runtime/styling.rs                 # 改为 re-export styling/ 模块
crates/rml/src/runtime/watcher.rs                 # 完整热重载实现
crates/rml/src/runtime/mod.rs                     # 新增 html/debounce 模块
crates/rml/src/lib.rs                             # 新增 mod styling

# Layer 6 - rml-app
crates/app/src/application.rs                     # 多窗口 + 扩展注册
crates/app/src/window.rs                          # WindowManager 实现
crates/app/src/resources.rs                       # 资源加载实现

# Workspace
Cargo.toml                                        # 新增 crates/ui, crates/lsp, crates/cli 成员
```

---

## 六、验证步骤

### 6.1 Phase B-1 验证（编译修复 + 核心 codegen）

1. `cargo build --workspace` —— 全 workspace 编译通过（无 `expected expression, found keyword 'impl'` 等错误）
2. `cargo run -p rml-demo` —— counter demo 运行，点击 +/- 数字变化
3. 扩展 demo 含 if/each/show —— 条件渲染、列表遍历正确
4. `#[on_loaded]` 回调被调用（通过 `println!` 或 `log::debug!` 验证）
5. 事件对象类型正确（`onclick` → ClickEvent，`oninput` → InputEvent，`onkeydown` → KeyDownEvent）
6. `cargo rml-expand counter` —— 输出生成代码可读

### 6.2 Phase B-2 验证（数据绑定）

1. 实现 todo demo：
   - `each={item in todos}` 列表遍历
   - `key={item.id}` 稳定 key
   - `model={new_todo}` 双向绑定输入框
   - `if={todos.is_empty()}` 空列表提示
   - `#[computed]` 计算剩余数量
   - `#[element]` + `ref` 聚焦输入框
2. `{count, HexConverter}` 转换器正常工作
3. 编译期错误：绑定不存在的字段时 build.rs 报错（含行号/列号/建议）
4. `#[command(can_execute = "can_submit")]` 按条件禁用按钮

### 6.3 Phase B-3 验证（组件与样式）

1. 实现 component demo：
   - `#[component]` 自定义 `TodoItem` 组件
   - Props 传递 `title`/`done`
   - `#[on_prop_change(done)]` 回调
   - 默认插槽 + 具名插槽
2. 实现 theme demo：
   - `src/styles/theme.css` 定义 CSS 变量
   - `cx.set_theme("dark")` 切换主题
   - `class="card"` 应用样式表
3. `features = ["ui-components"]` 启用后，`<Dialog>` 标签可用
4. 多窗口：`WindowManager::open` 打开第二个窗口

### 6.4 Phase B-4 验证（高级特性）

1. 实现 login demo：
   - `#[on_loaded]` 初始化焦点
   - `cx.spawn` 异步登录请求
   - 错误显示
2. 启用 `hot_reload = true`，修改 `.rml` 后 UI 实时更新，状态保留
3. `cargo rml-expand` 输出生成代码
4. `RML_LOG=debug cargo run` 输出绑定追踪日志
5. `RML_DUMP_AST=1 cargo build` 输出 AST
6. `Ctrl+Shift+I` 打开 RML Inspector
7. VS Code 安装 RML 插件，打开 `.rml` 文件：
   - 语法诊断（红色波浪线）
   - 标签/属性/绑定路径补全
   - Ctrl+Click 跳转定义
   - 保存时自动格式化

---

## 七、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| GPUI API 不稳定 | 编译失败 | 锁定 git rev `1d217ee`，定期跟进 |
| gpui-component API 变化 | `crates/ui` 不兼容 | feature flag 隔离，按需启用 |
| 表达式解析器复杂度 | Phase B-1 延期 | 限制为字段访问 + 方法调用 + 算术，不引入完整 Rust 解析器 |
| CSS 解析器复杂度 | Phase B-3 延期 | 严格按文档子集实现，不支持完整 CSS |
| 热重载 IPC 复杂度 | Phase B-4 延期 | 单独迭代，Phase B-4 末尾实现 |
| LSP 协议完整度 | Phase B-4 延期 | 优先实现诊断 + 补全，其他功能按需 |
| 计算属性缓存线程安全 | 运行时 panic | 使用 RefCell + 单线程 GPUI 上下文 |
| `#[view]` 宏 include! 修复后引入告警 | 编译告警 | 用 `#[allow(...)]` 抑制 |

---

## 八、实施顺序总览

```
Phase A (已完成) ✅
    ↓
Phase B-1 (编译修复 + 核心 codegen)
    ├── 修复 #[view] 宏 include! 位置
    ├── 修复 #f.ty 插值
    ├── 实现 GPUI→RML 事件转换
    ├── 标签映射
    ├── 表达式解析器
    ├── if/each/show 指令
    ├── 事件类型映射
    ├── 生命周期联动
    ├── 命令系统
    └── 元素 ID
    ↓
Phase B-2 (数据绑定)
    ├── model 双向绑定
    ├── 计算属性缓存
    ├── 编译期校验
    ├── ref 元素引用
    ├── 转换器
    ├── BindingPath 扩展
    └── else/html/once 指令
    ↓
Phase B-3 (组件与样式)
    ├── 组件 codegen
    ├── 组件注册表
    ├── Props 系统
    ├── 插槽系统
    ├── CSS 子集解析
    ├── 样式应用
    ├── 主题系统
    ├── crates/ui 扩展
    ├── 多窗口管理
    └── 资源加载
    ↓
Phase B-4 (高级特性)
    ├── 事件流三阶段
    ├── 自定义事件
    ├── 防抖/节流
    ├── 热重载
    ├── LSP 服务器
    ├── CLI 工具
    ├── 日志与性能
    ├── RML Inspector
    └── VirtualList
```

每个 Phase 完成后，必须通过对应的验证步骤（第六节）才能进入下一 Phase。

---

## 九、执行起点

**立即执行的第一步**：修复 `crates/macros/src/view.rs` 中 `#[view]` 宏的 `include!` 位置问题（2.1.1），解除 workspace 编译阻断，建立可工作的基线。然后按 Phase B-1 顺序继续。

```rust
// 修复后的 include_stmt（crates/macros/src/view.rs:168-173）
let include_stmt = quote! {
    #[allow(non_snake_case, unused_imports, unused_variables, dead_code)]
    include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
};
```

修复后立即运行 `cargo build --workspace` 验证编译通过，再开始 Phase B-1 的实际功能实现。
