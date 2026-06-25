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