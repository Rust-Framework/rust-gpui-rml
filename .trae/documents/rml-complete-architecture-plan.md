# RML 框架完整架构设计计划（Phase A → Phase B 全功能对齐）

> **目标**：从最底层架构开始，自底向上完成 RML 框架的全部实现，使其与 `docs/` 11 章文档的所有功能说明 100% 对齐。
> **范围**：在已完成的 Phase A MVP 基础上，补全 Phase B 的全部特性——指令系统、绑定引擎、组件系统、样式系统、生命周期、事件流、热重载、LSP、调试工具。
> **铁律**：
> 1. 所有 trait 必须以 `I` 开头（`IModel`/`IViewModel`/`IRmlView`/`ICommand`/`IConverter`/`ITwoWayBinding`/`ILifecycle`/`IBindingContext`/`IEvent`/`IComponent`/`IDirective`/`IPlugin`）。
> 2. `#![forbid(unsafe_code)]` 全 crate 启用。
> 3. 生成代码只写 `OUT_DIR`，禁止写 `src/`。
> 4. 过程宏不做重活，模板编译由 `build.rs` 完成。
> 5. `rml-core` 仅依赖 GPUI 基础类型，不依赖渲染系统。
> 6. **双轨制组件策略**：基础 HTML 标签用原生 GPUI 元素封装；复杂组件（Dialog/List/Modal 等）通过 `crates/ui` 扩展 crate 引入 `gpui-component`，按 feature flag 启用。

---

## 一、概述与当前状态

### 1.1 RML 框架定位

RML（Rust Markup Language）是基于 GPUI 的 HTML 友好型声明式 UI 框架，借鉴 WPF XAML 的 MVVM 模式。开发者编写 `.rml`（视图标记）和 `.rml.rs`（ViewModel 业务逻辑），由 `build.rs` 在编译期把 `.rml` 编译为原生 GPUI 代码，零运行时解析开销。

**三层架构**：
- **表现层**：`.rml` + `.rml.rs`（开发者编辑）
- **编译层**：`build.rs` 四阶段流水线（解析 → AST → 验证 → 代码生成）
- **框架层**：GPUI + RML Runtime + 可选 `crates/ui`（封装 gpui-component）

**Workspace 结构**（含新增 `crates/ui`）：
- `crates/core` —— trait 定义层（I 前缀）
- `crates/macros` —— 过程宏层
- `crates/rml` —— 解析器 + 编译器 + 构建器 + 运行时
- `crates/app` —— 应用启动器
- `crates/ui` —— **新增**：gpui-component 适配层（feature gate）
- `demo` —— 示例项目

### 1.2 Phase A 已完成状态 ✅

| 层 | 组件 | 状态 |
|----|------|------|
| Layer 0 | `rml-core` 全部 trait（8 个 + 事件对象 + ElementRef + BindingPath） | ✅ API 完整 |
| Layer 1 | `rml-macros` 7 个过程宏（`#[view]`/`#[component]`/`#[command]`/`#[computed]`/`#[on_loaded]`/`#[on_unloaded]`/`#[derive(IModel)]`） | ✅ 入口完整，**多数 pass-through** |
| Layer 2 | `rml/parser`（tokenizer + AST + parser） | ✅ 完整 |
| Layer 2 | `rml/compiler`（validator + codegen） | ⚠️ Phase A 简化版 |
| Layer 2 | `rml/tags`（19 个内置标签） | ⚠️ 全部映射为 `gpui::div()` |
| Layer 3 | `rml/build`（Builder + scanner + cache） | ✅ 完整可用 |
| Layer 4 | `rml/runtime`（event_flow/component_registry/styling/watcher） | ❌ 全部存根 |
| Layer 5 | `rml-app`（RmlApplication 单窗口启动器） | ✅ 单窗口可用 |
| Layer 6 | `demo`（counter 三件套） | ✅ 可运行 |

### 1.3 Phase B 待完成清单

通过逐文件核对 30 个文档文件，识别出 13 大类功能差距：

| # | 功能区域 | 当前完成度 | 目标 |
|---|---------|----------|------|
| 1 | 标签映射（真实 GPUI 构造器） | ~10% | 100% |
| 2 | 指令系统（10 个指令完整语义） | ~5% | 100% |
| 3 | 插值与属性（表达式求值） | ~30% | 100% |
| 4 | 绑定引擎（编译期校验 + 运行时订阅） | ~10% | 100% |
| 5 | 计算属性（`#[computed]` 依赖追踪 + 缓存） | ~5% | 100% |
| 6 | 转换器（IConverter 接入 codegen） | ~15% | 100% |
| 7 | 宏系统（生成实际代码而非 pass-through） | ~15% | 100% |
| 8 | 元素引用（ref + ElementRef 注入） | ~20% | 100% |
| 9 | 命令系统（ICommand impl 生成 + 校验） | ~15% | 100% |
| 10 | 事件系统（三阶段流 + 类型映射 + 防抖节流） | ~25% | 100% |
| 11 | 组件系统（codegen + Props + 插槽 + DI） | ~10% | 100% |
| 12 | 样式系统（CSS 子集 + 主题 + 变量） | ~0% | 100% |
| 13 | 高级特性（热重载 + LSP + 调试 + 性能） | ~0% | 100% |

---

## 二、关键设计决策

### 2.1 双轨制组件策略（用户确认）

| 轨道 | 适用标签 | 实现方式 | 依赖 |
|------|---------|---------|------|
| **原生轨** | `div/span/p/h1-h6/button/input/textarea/ul/ol/li/img/a/label/br` | 在 `crates/rml/src/tags.rs` 直接映射到 GPUI 原生元素（`gpui::div()`/`gpui::Button`/`gpui::Label` 等） | 仅 `gpui` |
| **扩展轨** | `Dialog/Modal/List/Table/Dropdown/Toast/Switch/Checkbox/Radio/Select` 等复杂组件 | 新建 `crates/ui` crate，封装 `gpui-component`，按 `feature = "ui-components"` 启用 | `gpui-component` |

**实现要点**：
- `crates/rml` 默认只编译原生轨标签，不依赖 `gpui-component`
- `crates/ui` 通过 feature flag 启用，提供 `RmlExt` trait 扩展 RML 标签注册表
- 用户在 `Cargo.toml` 中 `rml = { workspace = true, features = ["ui-components"] }` 即可获得完整组件库支持
- `tags.rs` 的 `codegen_ctor()` 改为返回真实构造器字符串，按标签类型映射

### 2.2 代码生成策略

**当前**：`codegen.rs` 直接生成单一表达式字符串。
**改进**：引入 `CodegenCtx` 扩展，支持：
- 稳定元素 ID 生成（`<view>_<path_hash>`）
- 条件分支（if/else 生成 `Option<AnyElement>`）
- 列表遍历（each 生成 `Vec<AnyElement>` + key 映射）
- 事件类型映射表（onclick→ClickEvent、oninput→InputEvent 等）
- 表达式求值器（支持字段访问、方法调用、算术）

**生成代码结构**：
```rust
impl gpui::Render for Counter {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {
        use gpui::*;
        // 1. 元素 ID 生成
        let _id_counter_root = "Counter_root";
        // 2. 条件分支（if/else）
        // 3. 列表遍历（each + key）
        // 4. 元素构建（含 class/style/event/ref）
        div().id(_id_counter_root)
            .class("counter")
            .child(div().child(Label::new(format!("{}", self.count))))
            .child(div()
                .child(div().id("Counter_root_dec")
                    .on_click(cx.listener(|this, _ev: &ClickEvent, cx| { this.decrement(_ev, cx); })))
                .child(div().id("Counter_root_inc")
                    .on_click(cx.listener(|this, _ev: &ClickEvent, cx| { this.increment(_ev, cx); }))))
    }
}
```

### 2.3 表达式求值策略

**支持范围**（按文档 §2.5）：
- 字段访问：`{user.name}` → `self.user.name`
- 嵌套字段：`{user.address.city}` → `self.user.address.city`
- 方法调用：`{items.len()}` → `self.items.len()`
- 算术表达式：`{count + 1}` → `self.count + 1`
- 比较表达式：`{count > 0}` → `self.count > 0`
- 转换器：`{count, HexConverter}` → `HexConverter.convert_to(&self.count)`

**实现**：在 `crates/rml/src/compiler/expr.rs` 新建表达式解析器，将表达式字符串解析为 `Expr` AST，codegen 时按 AST 生成 Rust 表达式。不引入完整 Rust 表达式解析器，仅支持上述子集。

### 2.4 生命周期联动机制

**机制**：`#[on_loaded]`/`#[on_unloaded]` 重命名方法为 `__rml_on_loaded_impl`/`__rml_on_unloaded_impl`，`#[view]` 宏在生成 `impl ILifecycle` 时调用此方法名。

```rust
// 用户代码
#[on_loaded]
fn setup(&mut self, cx: &mut Context<Self>) { ... }

// 宏展开后
fn __rml_on_loaded_impl(&mut self, cx: &mut Context<Self>) { ... }

impl ILifecycle for MyView {
    fn rml_on_loaded(&mut self, cx: &mut Context<Self>) {
        self.__rml_on_loaded_impl(cx);
    }
}
```

**触发时机**：`rml_on_loaded` 在首次 `render()` 调用后由 GPUI 的 `cx.observe` 触发；`rml_on_unloaded` 在 Entity drop 时触发（通过 `Drop` trait 或 GPUI 的 weak entity 机制）。

### 2.5 计算属性依赖追踪

**机制**：`#[computed]` 宏分析方法体 AST，提取所有 `self.field` 访问，生成缓存结构。

```rust
// 用户代码
#[computed]
fn double_count(&self) -> i32 { self.count * 2 }

// 宏展开后
fn double_count(&self) -> i32 {
    if self.__cache_double_count.is_fresh(&self.__versions) {
        return self.__cache_double_count.value;
    }
    let v = self.__double_count_impl();
    self.__cache_double_count.update(v, &self.__versions);
    v
}
fn __double_count_impl(&self) -> i32 { self.count * 2 }
```

**版本追踪**：每个 `pub` 字段维护版本号，`cx.notify()` 时递增。计算属性的缓存结构记录依赖字段的版本快照，任一版本变化即失效。

### 2.6 样式系统策略

**CSS 子集解析**：新建 `crates/rml/src/styling/` 模块，包含：
- `lexer.rs` —— CSS 词法分析
- `parser.rs` —— CSS 语法分析
- `selector.rs` —— 选择器匹配（标签/类/ID/后代/子/兄弟/属性/伪类）
- `properties.rs` —— CSS 属性 → GPUI 样式调用映射
- `variable.rs` —— CSS 变量表（`:root` + 局部覆盖）
- `theme.rs` —— 主题切换

**样式应用流程**：
1. `build.rs` 扫描 `src/styles/*.css`，解析为 `StyleSheet` 结构
2. `StyleSheet` 序列化为 Rust 常量，编译进二进制
3. codegen 时，`class="card"` 查询 `StyleSheet`，将匹配的声明转为 GPUI 调用链（`.bg()`/`.p()`/`.flex()` 等）
4. `style="color: red"` 内联样式优先级最高，直接应用

### 2.7 热重载架构

**完整实现**（用户确认）：
- **文件监听**：`runtime/watcher.rs` 使用 `notify` crate 监听 `.rml` 文件变化
- **重编译**：检测到变化后，调用编译器重新生成代码，写入临时目录
- **IPC 通信**：通过 Unix socket / Named Pipe 传递新代码到运行中应用
- **运行时替换**：应用接收新代码后，通过 `Entity::update` + `cx.notify()` 触发重渲染
- **状态保留**：ViewModel 状态不丢失，仅 View 重建
- **触发方式**：`Builder::hot_reload(true)` 启用；开发模式下 build.rs 启动监听线程

### 2.8 LSP 实现策略

**完整实现**（用户确认）：
- **新建 `crates/lsp` crate**：实现 LSP 协议服务器
- **复用编译器**：`rml::parser` + `rml::compiler::validator` 作为 LSP 后端
- **功能**：
  - 语法诊断（解析错误 + 验证错误）
  - 自动补全（标签名、属性名、指令名）
  - 悬停提示（标签/属性文档）
  - 跳转定义（`.rml` 中组件标签 → `crates/ui` 中的组件定义）
  - 格式化
- **集成**：发布为 `rml-lsp` 二进制，VS Code 插件通过 `vscode-languageclient` 调用

---

## 三、自底向上架构设计（Phase B 完整实施）

### Layer 0 · `rml-core` trait 层补全

**目标**：扩展 trait 以支撑 Phase B 全部特性。

#### 3.0.1 修改 `crates/core/src/command.rs`

扩展 `ICommand` 增加 `can_execute`：

```rust
pub trait ICommand {
    fn rml_command_name() -> &'static str;
    fn rml_event_type() -> &'static str { "" }
    fn rml_params() -> &'static [ParamMeta] { &[] }
    /// 命令是否可执行（用于禁用按钮等）
    fn can_execute(&self) -> bool { true }
}
```

#### 3.0.2 新建 `crates/core/src/plugin.rs`

```rust
/// RML 插件接口（用于 LSP、调试器、热重载等扩展）
pub trait IPlugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_compile(&self, ctx: &mut PluginContext) {}
    fn on_render(&self, ctx: &mut RenderContext) {}
}

pub struct PluginContext { /* 编译期上下文 */ }
pub struct RenderContext { /* 渲染期上下文 */ }
```

#### 3.0.3 修改 `crates/core/src/binding.rs`

扩展 `BindingPath` 支持 Index 和 MethodCall 解析：

```rust
pub enum BindingSegment {
    Field(String),
    Member(String),           // 点分访问
    Index(String),            // [0] 或 ["key"]
    MethodCall(String, Vec<String>),  // .method(args)
}

impl BindingPath {
    pub fn parse(expr: &str) -> Result<BindingPath, BindingError> {
        // 支持 user.name / items[0] / items.len() / count + 1
    }
}
```

#### 3.0.4 修改 `crates/core/src/lib.rs` + `prelude.rs`

新增 `pub mod plugin;`，导出 `IPlugin`/`PluginContext`/`RenderContext`。

---

### Layer 1 · `rml-macros` 过程宏完整实现

**目标**：从 pass-through 升级为生成实际代码。

#### 3.1.1 修改 `crates/macros/src/view.rs`

扩展 `#[view]` 宏生成：

1. **生命周期联动**：扫描 `#[on_loaded]`/`#[on_unloaded]` 标记的方法，在 `impl ILifecycle` 中调用
2. **元素引用注入**：收集 `#[element]` 字段，生成 `__rml_bind_elements` 方法，在首次渲染后注入 Entity handle
3. **Props 处理**（仅 `#[component]`）：识别 `#[prop(default = ...)]` 字段，生成默认值填充
4. **属性变化回调**（仅 `#[component]`）：识别 `#[on_prop_change(field)]`，生成监听代码

```rust
// 生成的代码结构
impl ILifecycle for Counter {
    fn rml_on_loaded(&mut self, cx: &mut Context<Self>) {
        self.__rml_on_loaded_impl(cx);  // 调用 #[on_loaded] 方法
    }
    fn rml_on_unloaded(&mut self, cx: &mut Context<Self>) {
        self.__rml_on_unloaded_impl(cx);
    }
}

impl Counter {
    fn __rml_bind_elements(&mut self, cx: &mut Context<Self>) {
        // 对每个 #[element] 字段，在首次渲染后调用 field.set(handle)
        // 具体句柄获取通过 ref="name" 在 codegen 时注入
    }
}
```

#### 3.1.2 修改 `crates/macros/src/command.rs`

生成 `impl ICommand`：

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

解析方法签名，提取事件类型（第二个参数的类型名）和参数列表。

#### 3.1.3 修改 `crates/macros/src/computed.rs`

实现依赖追踪 + 缓存代码生成：

1. 使用 `syn::visit::Visit` 遍历方法体 AST
2. 收集所有 `self.field` 访问的 `field` 名
3. 生成缓存结构体 `__ComputedCache_<method_name>`
4. 生成版本检查 + 重算逻辑

```rust
// 用户代码
#[computed]
fn full_name(&self) -> String { format!("{} {}", self.first, self.last) }

// 宏生成
struct __Cache_full_name { value: Option<String>, v_first: u64, v_last: u64 }

impl Counter {
    fn full_name(&self) -> String {
        let cache = &self.__cache_full_name;
        if cache.v_first == self.__v_first && cache.v_last == self.__v_last {
            if let Some(v) = &cache.value { return v.clone(); }
        }
        let v = format!("{} {}", self.first, self.last);
        // 更新缓存（需 &mut self，通过 interior mutability 或 cx 上下文）
        v
    }
}
```

**注**：缓存更新需 `&mut self`，但 `#[computed]` 签名是 `&self`。采用 `RefCell<__Cache>` 实现内部可变性。

#### 3.1.4 修改 `crates/macros/src/lifecycle.rs`

将 `#[on_loaded]`/`#[on_unloaded]` 标记的方法重命名为 `__rml_on_loaded_impl`/`__rml_on_unloaded_impl`，供 `#[view]` 调用。

#### 3.1.5 新建 `crates/macros/src/prop.rs`

实现 `#[prop(default = ...)]` 和 `#[on_prop_change(field)]` helper attribute：
- `#[prop]`：标记组件属性字段，生成默认值构造
- `#[on_prop_change(field)]`：生成属性变化回调，在 props 更新时触发

#### 3.1.6 修改 `crates/macros/src/lib.rs`

声明新的 helper attributes：`prop`、`on_prop_change`。

---

### Layer 2 · `rml/parser` + `rml/compiler` + `rml/tags` 完整实现

#### 3.2.1 修改 `crates/rml/src/tags.rs`

为每个标签返回真实 GPUI 构造器：

```rust
impl BuiltinTag {
    pub fn codegen_ctor(self) -> &'static str {
        match self {
            BuiltinTag::Div => "gpui::div()",
            BuiltinTag::Span => "gpui::div().inline()",
            BuiltinTag::P => "gpui::div()",
            BuiltinTag::H1 => "gpui::div().text_size(32.)",
            BuiltinTag::H2 => "gpui::div().text_size(28.)",
            // ... H3-H6
            BuiltinTag::Button => "gpui::div()",  // 原生轨：用 div 模拟
            BuiltinTag::Input => "gpui::div()",   // 原生轨：简化
            BuiltinTag::TextArea => "gpui::div()",
            BuiltinTag::Ul => "gpui::div().flex_col()",
            BuiltinTag::Ol => "gpui::div().flex_col()",
            BuiltinTag::Li => "gpui::div()",
            BuiltinTag::Img => "gpui::div()",
            BuiltinTag::A => "gpui::div()",
            BuiltinTag::Label => "gpui::div()",
            BuiltinTag::Br => "gpui::div().h_0()",
        }
    }
}
```

**注**：原生轨 button/input 用 `gpui::div()` + class 应用样式；扩展轨（feature = "ui-components"）在 `crates/ui` 中覆盖映射，返回 `gpui_component::Button::new()` 等。

#### 3.2.2 新建 `crates/rml/src/compiler/expr.rs`

表达式解析器，支持：
- 字段访问：`user.name` → `Field("user"), Member("name")`
- 索引访问：`items[0]` → `Field("items"), Index("0")`
- 方法调用：`items.len()` → `Field("items"), MethodCall("len", [])`
- 算术：`count + 1` → `BinaryOp(Add, Field("count"), Lit("1"))`
- 比较：`count > 0` → `BinaryOp(Gt, Field("count"), Lit("0"))`
- 转换器：`count, HexConverter` → `Convert(Field("count"), "HexConverter")`

codegen 时按 AST 生成 Rust 表达式字符串。

#### 3.2.3 修改 `crates/rml/src/compiler/codegen.rs` —— 核心改造

**完整重写**，实现所有指令、属性、事件的代码生成：

**A. 元素 ID 生成**：
```rust
fn gen_element_id(view_name: &str, path: &[usize]) -> String {
    let path_str = path.iter().map(|i| i.to_string()).collect::<Vec<_>>().join("_");
    format!("{}_{}", view_name, path_str)
}
```

**B. 指令处理**（按文档 §2.4.10 顺序：each → if → key → 其他）：

```rust
fn gen_directives(elem: &Element, ctx: &CodegenCtx, path: &[usize]) -> Result<String, CodegenError> {
    let mut prefix = String::new();
    let mut suffix = String::new();
    
    for d in &elem.directives {
        match d {
            Directive::Each(each) => {
                // 生成: self.{iterable}.iter().enumerate().map(|({index}, {item})| { body }).collect::<Vec<_>>()
                prefix.push_str(&format!(
                    "self.{}.iter().enumerate().map(|({}, {})| {{ ",
                    each.iterable, each.index.as_deref().unwrap_or("_"), each.item
                ));
                suffix.push_str(" }).collect::<Vec<_>>().into_iter()");
            }
            Directive::If(cond) => {
                // 生成: if self.{cond} { Some(element) } else { None }
                prefix.push_str(&format!("if self.{} {{ Some(", cond));
                suffix.push_str(") } else { None }");
            }
            Directive::Else => {
                // 在兄弟节点层级处理（见 gen_children_with_else）
            }
            Directive::Key(key) => {
                // 配合 each，生成 .key(|(_, item)| item.{key}.clone())
                // 注：GPUI 的 uniform_list 需要 key 函数
            }
            Directive::Model(field) => {
                // 生成双向绑定：value + on_change
                suffix.push_str(&format!(
                    ".on_change(cx.listener(move |this, v: &SharedString, cx| {{ this.{0} = v.to_string(); cx.notify(); }}))",
                    field
                ));
            }
            Directive::Show(cond) => {
                // 生成 display 样式控制
                suffix.push_str(&format!(
                    ".when(self.{0}, |el| el).when(!self.{0}, |el| el.class(\"hidden\"))",
                    cond
                ));
            }
            Directive::Once => {
                // 标记为一次性渲染，codegen 时直接展开字面量
                // 需运行时支持，Phase B 用 RefCell<bool> 标记
            }
            Directive::Html(expr) => {
                // 生成 HTML 字符串解析（需 html_parser crate）
                suffix.push_str(&format!(".child(rml::runtime::html::parse(&self.{}))", expr));
            }
            Directive::Ref(name) => {
                // 在元素创建后调用 self.{name}.set(handle)
                // 需 codegen 在元素构建完成后插入注入代码
                suffix.push_str(&format!(".after_render(|handle| {{ self.{}.set(handle); }})", name));
            }
            Directive::Slot(name) => {
                // 在组件内部位置插入插槽内容
                // 见组件系统 codegen
            }
        }
    }
    Ok(format!("{}{}{}", prefix, /* element code */, suffix))
}
```

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

fn gpui_event_type(event_name: &str) -> &'static str {
    match event_name {
        "onclick" => "gpui::ClickEvent",
        "oninput" => "gpui::InputEvent",
        // ... 完整映射
        _ => "gpui::ClickEvent",
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

**E. 兄弟节点 if/else 配对**：

新建 `gen_children_with_else` 函数，扫描子节点列表，识别 `if`/`else` 配对，生成 `if cond { Some } else { None }` 链。

**F. 组件 codegen**：

```rust
fn gen_component(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    let tag = &elem.tag;  // PascalCase
    // 生成: <Module>::<Tag>::new()
    //         .prop1(value1)
    //         .prop2(value2)
    //         .on_event(handler)
    //         .children([slot_content])
    let mut code = format!("{}::new()", tag);
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                code.push_str(&format!(".{}({:?})", name, value));
            }
            Attribute::Bind { name, expr } => {
                code.push_str(&format!(".{}(self.{})", name, expr));
            }
            Attribute::Event { name, handler } => {
                code.push_str(&apply_event(name, handler, ctx));
            }
        }
    }
    // 处理插槽
    for child in &elem.children {
        let slot_name = child.directives.iter().find_map(|d| match d {
            Directive::Slot(n) => Some(n.clone()),
            _ => None,
        }).unwrap_or_else(|| "default".to_string());
        code.push_str(&format!(".slot_{}({})", slot_name, gen_node(child, ctx, 0)?));
    }
    Ok(code)
}
```

#### 3.2.4 修改 `crates/rml/src/compiler/validator.rs`

增加编译期校验：

1. **绑定路径校验**：通过 `IModel::rml_fields()` 元信息校验 `{field}` 存在（Phase B 通过注册表）
2. **事件绑定校验**：检查 `onclick={method}` 的 method 在 ViewModel 中存在（通过 `ICommand` 注册表）
3. **指令合法性**：
   - `else` 必须紧跟 `if` 元素（兄弟节点）
   - `each` 的 `key` 建议存在（警告）
   - `model` 只能用于 `input`/`textarea`/`checkbox` 或实现 `ITwoWayBinding` 的组件
   - `ref` 名同视图内唯一
   - `slot` 必须在组件内（PascalCase 标签）
4. **类型校验**：检查绑定返回类型实现 `Display`（Phase B 通过类型注册表）

#### 3.2.5 新建 `crates/rml/src/compiler/typed_ast.rs`

类型化 AST（可选优化，Phase B-4）：
- 在 AST 节点上附加类型信息
- 支持常量折叠、死代码消除

---

### Layer 3 · `rml/build` 构建流程增强

#### 3.3.1 修改 `crates/rml/src/build/mod.rs`

**A. 样式表扫描**：扩展 `Builder` 增加 `style_dir` 配置，扫描 `src/styles/*.css`。

```rust
pub struct Builder {
    scan_dirs: Vec<PathBuf>,
    style_dirs: Vec<PathBuf>,  // 新增
    output_dir: Option<PathBuf>,
    // ...
}

impl Builder {
    pub fn style_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.style_dirs.push(dir.into());
        self
    }
}
```

**B. namespace 生效**：将 `namespace` 注入生成代码的模块路径。

**C. 热重载支持**：当 `hot_reload(true)` 时，启动监听线程。

**D. 调试输出**：支持 `RML_DUMP_AST=1` 环境变量，输出 AST 到 stderr。

**E. CSS 编译**：扫描 `.css` 文件，调用 `styling::parser` 解析，序列化为 Rust 常量写入 `OUT_DIR/rml_generated/styles.rs`。

---

### Layer 4 · `rml/runtime` 运行时完整实现

#### 3.4.1 实现 `crates/rml/src/runtime/event_flow.rs`

**三阶段事件调度**：

```rust
pub enum EventPhase { Capture, Target, Bubble }

pub struct EventDispatcher {
    // 事件路径（从 root 到 target 的元素链）
}

impl EventDispatcher {
    /// 调度事件，按 捕获 → 目标 → 冒泡 顺序
    pub fn dispatch(&mut self, event: &mut dyn IEvent, cx: &mut App) {
        // 1. 捕获阶段：从 root 到 target 的父节点
        for handler in self.capture_handlers() {
            handler(event);
            if event.is_propagation_stopped() { return; }
        }
        // 2. 目标阶段
        for handler in self.target_handlers() {
            handler(event);
            if event.is_propagation_stopped() { return; }
        }
        // 3. 冒泡阶段：从 target 的父节点到 root
        for handler in self.bubble_handlers() {
            handler(event);
            if event.is_propagation_stopped() { return; }
        }
    }
}

/// GPUI 事件 → RML 事件转换
pub mod convert {
    pub fn from_gpui_click(ev: &gpui::ClickEvent) -> ClickEvent { ... }
    pub fn from_gpui_input(ev: &gpui::InputEvent) -> InputEvent { ... }
    // ... 完整映射
}
```

#### 3.4.2 实现 `crates/rml/src/runtime/component_registry.rs`

**全局组件注册表**：

```rust
use std::collections::HashMap;
use std::sync::RwLock;

pub struct ComponentEntry {
    pub tag: String,           // PascalCase 标签名
    pub constructor: fn() -> Box<dyn IRmlView>,
    pub props_meta: Vec<PropMeta>,
    pub slots: Vec<String>,    // 支持的插槽名
}

pub struct ComponentRegistry {
    entries: HashMap<String, ComponentEntry>,
}

impl ComponentRegistry {
    pub fn register(&mut self, entry: ComponentEntry);
    pub fn lookup(&self, tag: &str) -> Option<&ComponentEntry>;
    pub fn list(&self) -> Vec<&ComponentEntry>;
}

/// 全局注册表（lazy_static）
pub static GLOBAL_REGISTRY: RwLock<ComponentRegistry> = RwLock::new(ComponentRegistry::new());

/// 注册宏（在 #[component] 中调用）
#[macro_export]
macro_rules! register_component {
    ($ty:ty, $tag:literal) => {
        inventory::submit! {
            rml::ComponentEntry {
                tag: $tag.to_string(),
                constructor: || Box::new(<$ty>::default()),
                // ...
            }
        }
    };
}
```

#### 3.4.3 实现 `crates/rml/src/runtime/styling.rs` —— 样式系统

**模块结构**：
```
crates/rml/src/styling/
├── mod.rs           # 模块入口
├── lexer.rs         # CSS 词法分析
├── parser.rs        # CSS 语法分析
├── selector.rs      # 选择器匹配
├── properties.rs    # CSS 属性 → GPUI 调用映射
├── variable.rs      # CSS 变量表
└── theme.rs         # 主题切换
```

**CSS 解析**：
```rust
pub struct StyleSheet {
    pub rules: Vec<Rule>,
}

pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

pub enum Selector {
    Tag(String),           // div
    Class(String),         // .card
    Id(String),            // #main
    Descendant(Box<Selector>, Box<Selector>),  // div .card
    Child(Box<Selector>, Box<Selector>),       // div > .card
    Attribute(String, Option<AttrOp>, Option<String>),  // [type="text"]
    PseudoClass(String),   // :hover
    Group(Vec<Selector>),  // div, .card
}

pub struct Declaration {
    pub property: String,
    pub value: Value,
}

pub enum Value {
    Length(f32, Unit),     // 10px
    Color(Rgba),           // #fff
    String(String),        // "flex"
    Var(String, Option<Box<Value>>),  // var(--name, fallback)
    Calc(Box<Expr>),       // calc(100% - 10px)
}
```

**样式应用**：codegen 时查询 `StyleSheet`，将匹配声明转为 GPUI 调用：
```rust
// .card { background: #f0f0f0; padding: 16px; }
// 生成: div().bg(rgb(0xf0f0f0)).p(px(16.))
```

**主题切换**：
```rust
pub struct Theme {
    pub name: String,
    pub variables: HashMap<String, Value>,  // --name → value
}

pub fn set_theme(name: &str, cx: &mut App) {
    // 切换全局变量表，触发 cx.notify()
}
```

#### 3.4.4 实现 `crates/rml/src/runtime/watcher.rs` —— 热重载

**完整实现**：

```rust
use notify::{Watcher, RecursiveMode, Event};
use std::path::PathBuf;
use std::sync::mpsc;

pub struct HotReloader {
    watcher: notify::RecommendedWatcher,
    rx: mpsc::Receiver<Event>,
    callback: Box<dyn Fn(PathBuf) + Send>,
}

impl HotReloader {
    pub fn new<F>(dirs: Vec<PathBuf>, callback: F) -> std::io::Result<Self>
    where F: Fn(PathBuf) + Send + 'static
    {
        let (tx, rx) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        })?;
        for dir in &dirs {
            watcher.watch(dir, RecursiveMode::Recursive)?;
        }
        Ok(Self { watcher, rx, callback: Box::new(callback) })
    }
    
    /// 在 GPUI 任务中轮询
    pub fn poll(&self, cx: &mut App) {
        while let Ok(event) = self.rx.try_recv() {
            for path in event.paths {
                if path.extension().and_then(|e| e.to_str()) == Some("rml") {
                    (self.callback)(path);
                }
            }
        }
        // 重新调度自身
        cx.spawn(|cx| async move {
            cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
            // 重新 poll
        }).detach();
    }
}
```

**热重载流程**：
1. `build.rs` 在 `hot_reload(true)` 时启动监听线程
2. 检测到 `.rml` 变化后，重新编译，写入临时目录
3. 通过 IPC（Unix socket / Named Pipe）通知运行中应用
4. 应用接收新代码，通过 `Entity::update` + `cx.notify()` 触发重渲染
5. ViewModel 状态保留，仅 View 重建

#### 3.4.5 新建 `crates/rml/src/runtime/html.rs`

HTML 字符串解析（用于 `html={raw}` 指令）：
- 使用 `html5ever` 或简化解析器
- 解析为 GPUI 元素树

#### 3.4.6 新建 `crates/rml/src/runtime/debounce.rs`

防抖/节流原语：

```rust
pub fn debounce<F>(ms: u64, f: F) -> impl Fn()
where F: Fn() + 'static
{
    let last = std::sync::Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));
    move || {
        // 基于 cx.spawn + timer 实现
    }
}

pub fn throttle<F>(ms: u64, f: F) -> impl Fn()
where F: Fn() + 'static
{
    // 类似实现
}
```

---

### Layer 5 · `crates/ui` 扩展组件库（新增）

**目标**：封装 `gpui-component`，提供复杂组件的 RML 标签映射。

#### 3.5.1 新建 `crates/ui/Cargo.toml`

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

#### 3.5.2 新建 `crates/ui/src/lib.rs`

```rust
//! RML UI 扩展组件库
//!
//! 通过 feature flag `ui-components` 启用，提供 Dialog/Modal/List 等复杂组件。
//! 启用后，RML 标签注册表自动扩展，支持 <Dialog>/<Modal> 等 PascalCase 标签。

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
        rml::runtime::component_registry::register(Dialog::new, "Dialog");
        rml::runtime::component_registry::register(List::new, "List");
        // ...
    }
}
```

#### 3.5.3 各组件适配文件

每个文件封装一个 `gpui-component` 组件，实现 `IRmlView` + `IComponent`：

```rust
// dialog.rs
use gpui_component::dialog::Dialog as GpuiDialog;
use rml_core::prelude::*;

pub struct Dialog {
    inner: GpuiDialog,
    pub open: bool,
    pub title: SharedString,
}

impl IComponent for Dialog {
    fn rml_tag() -> &'static str { "Dialog" }
}

// 实现 IRmlView + IModel + ILifecycle + Render
```

---

### Layer 6 · `rml-app` 应用启动器增强

#### 3.6.1 修改 `crates/app/src/application.rs`

**多窗口支持**：

```rust
impl RmlApplication {
    pub fn new() -> Self { ... }
    pub fn title(mut self, t: impl Into<SharedString>) -> Self { ... }
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self { ... }
    
    /// 添加全局状态
    pub fn with_global<G: Global + 'static>(mut self, global: G) -> Self { ... }
    
    /// 注册 RML 扩展（如 ui-components）
    pub fn with_extensions(mut self, f: impl FnOnce(&mut Self)) -> Self { ... }
    
    /// 启动应用，以 R 为根视图
    pub fn run<R: IRmlView + Render + Default + 'static>(self) {
        Application::new().run(move |cx: &mut App| {
            // 1. 注册内置组件
            rml::runtime::component_registry::init();
            // 2. 注册扩展组件（如 ui-components）
            #[cfg(feature = "ui-components")]
            rml_ui::register();
            // 3. 打开主窗口
            cx.open_window(options, |_, cx| cx.new(|_| R::default()));
        });
    }
}
```

#### 3.6.2 实现 `crates/app/src/window.rs`

多窗口管理：
```rust
pub struct WindowManager {
    windows: Vec<WindowHandle>,
}

impl WindowManager {
    pub fn open<R: IRmlView + Render + Default + 'static>(&mut self, cx: &mut App) -> WindowHandle { ... }
    pub fn close(&mut self, handle: WindowHandle) { ... }
    pub fn list(&self) -> &[WindowHandle] { ... }
}
```

#### 3.6.3 实现 `crates/app/src/resources.rs`

资源加载：
```rust
pub struct Resources {
    assets_dir: PathBuf,
    cache: HashMap<String, Vec<u8>>,
}

impl Resources {
    pub fn load(&mut self, path: &str) -> std::io::Result<&[u8]> { ... }
    pub fn load_string(&mut self, path: &str) -> std::io::Result<String> { ... }
}
```

---

### Layer 7 · `crates/lsp` LSP 服务器（新增）

**目标**：实现 LSP 协议服务器，提供语法诊断、自动补全、悬停提示。

#### 3.7.1 新建 `crates/lsp/Cargo.toml`

```toml
[package]
name = "rml-lsp"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
rml = { workspace = true }
rml-core = { workspace = true }
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }
```

#### 3.7.2 新建 `crates/lsp/src/main.rs`

LSP 服务器入口，使用 `tower-lsp` 实现：

```rust
use tower_lsp::{LspServer, LanguageServer};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspServer::new_build(stdin, stdout).build();
    serve(socket).await;
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        // 解析 .rml 文件，发布诊断
        let diagnostics = rml::parser::parse(&params.text_document.text)
            .map_err(|e| vec![e.to_lsp_diagnostic()])
            .map(|_| vec![])?;
        self.client.publish_diagnostics(params.text_document.uri, diagnostics, None).await;
    }
    
    async fn completion(&self, params: CompletionParams) -> Option<CompletionResponse> {
        // 根据光标位置返回标签/属性/指令补全
    }
    
    async fn hover(&self, params: HoverParams) -> Option<Hover> {
        // 返回标签/属性文档
    }
}
```

#### 3.7.3 新建 `crates/lsp/src/diagnostics.rs`

将 RML 解析/验证错误转换为 LSP Diagnostic：

```rust
pub fn parse_error_to_diagnostic(e: &rml::parser::ParseError) -> tower_lsp::lsp_types::Diagnostic {
    Diagnostic {
        range: Range {
            start: Position { line: e.line as u32 - 1, character: e.column as u32 - 1 },
            end: Position { line: e.line as u32 - 1, character: e.column as u32 },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: e.message.clone(),
        source: Some("rml".to_string()),
        ..Default::default()
    }
}
```

---

### Layer 8 · 调试工具与 CLI

#### 3.8.1 新建 `crates/cli/Cargo.toml` + `src/main.rs`

`rml` CLI 工具，提供子命令：

```rust
// cargo rml-expand views::counter
fn cmd_expand(args: &[String]) {
    // 读取 OUT_DIR/rml_generated/<name>.rs 并输出
}

// cargo rml-lint src/
fn cmd_lint(args: &[String]) {
    // 扫描 .rml 文件，运行 validator，输出错误
}

// cargo rml-format src/
fn cmd_format(args: &[String]) {
    // 格式化 .rml 文件
}
```

#### 3.8.2 环境变量支持

在 `rml/runtime` 中：
- `RML_LOG=debug` —— 启用绑定追踪日志（codegen 插入 `log::debug!`）
- `RML_DUMP_AST=1` —— build.rs 输出 AST 到 stderr
- `RML_PROFILE=1` —— 记录 render 耗时

---

## 四、Phase B 分阶段实施计划

### Phase B-1 · 核心 codegen 完成（优先级最高）

**目标**：让最简单的 `.rml` 能正确编译运行，覆盖基础语法。

**任务**：
1. **标签映射**（3.2.1）：button/input/img/h1-h6 映射到真实 GPUI 构造器
2. **表达式解析器**（3.2.2）：支持字段访问、方法调用、算术
3. **if/each/show 指令**（3.2.3 B）：条件创建、列表遍历、显隐控制
4. **事件类型映射**（3.2.3 C/D）：按事件名生成对应 RML 事件对象
5. **生命周期联动**（3.1.1, 3.1.4）：`#[on_loaded]`/`#[on_unloaded]` 真正触发
6. **命令系统**（3.1.2）：`#[command]` 生成 ICommand impl
7. **元素 ID 生成**（3.2.3 A）：稳定 ID 用于 GPUI diff

**验证**：扩展 demo，覆盖 if/each/show/onclick/oninput。

### Phase B-2 · 数据绑定完整

**目标**：实现 WPF 级数据绑定。

**任务**：
1. **model 双向绑定**（3.2.3 B）：input/textarea 双向数据流
2. **计算属性缓存**（3.1.3）：`#[computed]` 依赖追踪 + 缓存
3. **编译期字段校验**（3.2.4）：validator 校验绑定路径存在
4. **ref 元素引用注入**（3.2.3 B, 3.1.1）：运行时注入 Entity handle
5. **转换器**（3.2.2, 3.2.3）：`{expr, Converter}` 语法
6. **BindingPath 扩展**（3.0.3）：支持 Index/MethodCall
7. **else/html/once 指令**（3.2.3 B）：剩余指令

**验证**：实现 todo demo，覆盖 each/key/model/if/#[computed]/#[element]。

### Phase B-3 · 组件与样式

**目标**：组件系统 + 样式系统完整可用。

**任务**：
1. **组件 codegen**（3.2.3 F）：PascalCase 标签编译为子组件
2. **组件注册表**（3.4.2）：全局注册 + inventory
3. **Props 系统**（3.1.5）：`#[prop]` + 默认值 + 响应式
4. **`#[on_prop_change]`**（3.1.5）：属性变化回调
5. **插槽系统**（3.2.3 F）：默认/具名/作用域插槽
6. **依赖注入**（3.4.x）：`cx.provide` / `cx.use_provider`
7. **CSS 子集解析**（3.4.3）：lexer + parser + selector
8. **样式应用**（3.4.3）：codegen 查询样式表生成 GPUI 调用
9. **主题系统**（3.4.3）：CSS 变量 + 主题切换
10. **`crates/ui` 扩展**（3.5）：封装 gpui-component

**验证**：
- 实现 component demo，覆盖 `#[component]`、Props、插槽
- 实现 theme demo，覆盖 CSS 变量、主题切换

### Phase B-4 · 高级特性

**目标**：调试、热重载、性能、LSP。

**任务**：
1. **事件流三阶段**（3.4.1）：捕获/冒泡/stop_propagation
2. **自定义事件**（3.4.x）：`Option<Arc<dyn Fn>>` 字段
3. **防抖/节流**（3.4.6）：debounce/throttle 原语
4. **热重载**（3.4.4）：文件监听 + IPC + 状态保留
5. **LSP 服务器**（3.7）：诊断 + 补全 + 悬停
6. **CLI 工具**（3.8.1）：rml-expand/rml-lint/rml-format
7. **日志与性能**（3.8.2）：RML_LOG/RML_DUMP_AST/RML_PROFILE
8. **VirtualList**（3.4.x）：基于 GPUI uniform_list 的虚拟滚动
9. **类型化 AST**（3.2.5）：可选优化

**验证**：
- 实现 login demo，覆盖 `#[on_loaded]`、异步 `cx.spawn`、错误显示
- 启用 hot-reload，验证模板修改热重载
- 运行 `cargo rml-expand` 查看生成代码
- 运行 `RML_LOG=debug cargo run` 验证日志

---

## 五、假设与决策

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

---

## 六、验证步骤

### 6.1 Phase B-1 验证

1. `cargo build` —— 全 workspace 编译通过
2. `cargo run -p rml-demo` —— counter demo 运行，点击 +/- 数字变化
3. 扩展 demo 含 if/each/show —— 条件渲染、列表遍历正确
4. `#[on_loaded]` 回调被调用（通过日志验证）
5. 事件对象类型正确（ClickEvent/InputEvent/KeyDownEvent）

### 6.2 Phase B-2 验证

1. 实现 todo demo：
   - `each={item in todos}` 列表遍历
   - `key={item.id}` 稳定 key
   - `model={new_todo}` 双向绑定输入框
   - `if={todos.is_empty()}` 空列表提示
   - `#[computed]` 计算剩余数量
   - `#[element]` + `ref` 聚焦输入框
2. `{count, HexConverter}` 转换器正常工作
3. 编译期错误：绑定不存在的字段时 build.rs 报错

### 6.3 Phase B-3 验证

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

### 6.4 Phase B-4 验证

1. 实现 login demo：
   - `#[on_loaded]` 初始化焦点
   - `cx.spawn` 异步登录请求
   - 错误显示
2. 启用 `hot_reload = true`，修改 `.rml` 后 UI 实时更新，状态保留
3. `cargo rml-expand` 输出生成代码
4. `RML_LOG=debug cargo run` 输出绑定追踪日志
5. `RML_DUMP_AST=1 cargo build` 输出 AST
6. VS Code 安装 RML 插件，打开 `.rml` 文件，语法诊断 + 补全可用

---

## 七、文件清单

### 7.1 待新建文件

```
# Layer 0 - rml-core 补全
crates/core/src/plugin.rs                         # IPlugin trait

# Layer 1 - rml-macros 完整实现
crates/macros/src/prop.rs                         # #[prop] + #[on_prop_change]

# Layer 2 - rml/compiler 完整实现
crates/rml/src/compiler/expr.rs                   # 表达式解析器
crates/rml/src/compiler/typed_ast.rs              # 类型化 AST（可选）

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
crates/ui/Cargo.toml                              # UI 扩展 crate 配置
crates/ui/src/lib.rs                              # 入口 + register()
crates/ui/src/dialog.rs                           # Dialog 组件适配
crates/ui/src/list.rs                             # List 组件适配
crates/ui/src/form.rs                             # Checkbox/Radio/Switch/Select

# Layer 7 - LSP 服务器
crates/lsp/Cargo.toml                             # LSP crate 配置
crates/lsp/src/main.rs                            # LSP 服务器入口
crates/lsp/src/diagnostics.rs                     # 错误 → LSP Diagnostic
crates/lsp/src/completion.rs                      # 自动补全
crates/lsp/src/hover.rs                           # 悬停提示

# Layer 8 - CLI 工具
crates/cli/Cargo.toml                             # CLI crate 配置
crates/cli/src/main.rs                            # rml-expand/rml-lint/rml-format

# VS Code 插件（可选，独立目录）
editors/vscode/package.json                       # VS Code 插件配置
editors/vscode/src/extension.ts                   # 插件入口
```

### 7.2 待修改文件

```
# Layer 0 - rml-core
crates/core/src/command.rs                        # ICommand 增加 can_execute
crates/core/src/binding.rs                        # BindingPath 支持 Index/MethodCall
crates/core/src/lib.rs                            # 新增 mod plugin
crates/core/src/prelude.rs                        # 导出 IPlugin

# Layer 1 - rml-macros
crates/macros/src/lib.rs                          # 声明 prop/on_prop_change helper
crates/macros/src/view.rs                         # 生命周期联动 + element 注入 + props
crates/macros/src/command.rs                      # 生成 ICommand impl
crates/macros/src/computed.rs                     # 依赖追踪 + 缓存代码
crates/macros/src/lifecycle.rs                    # 方法重命名机制

# Layer 2 - rml/compiler + tags
crates/rml/src/tags.rs                            # 真实 GPUI 构造器映射
crates/rml/src/compiler/codegen.rs                # 完整重写：指令/事件/组件/ID
crates/rml/src/compiler/validator.rs              # 编译期字段/类型校验
crates/rml/src/compiler/mod.rs                    # 接入表达式解析器

# Layer 3 - rml/build
crates/rml/src/build/mod.rs                       # style_dir + namespace + hot_reload + RML_DUMP_AST

# Layer 4 - rml/runtime
crates/rml/src/runtime/event_flow.rs              # 三阶段调度 + 事件转换
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

### 7.3 待新建 demo

```
demo/src/todos.rml                                # todo 列表 demo
demo/src/todos.rml.rs                             # todo ViewModel
demo/src/login.rml                                # 登录 demo
demo/src/login.rml.rs                             # login ViewModel
demo/src/components/todo_item.rml                 # 自定义组件 demo
demo/src/components/todo_item.rml.rs              # TodoItem 组件
demo/src/styles/theme.css                         # 主题样式
demo/src/theme_demo.rml                           # 主题切换 demo
demo/src/theme_demo.rml.rs                        # theme ViewModel
```

---

## 八、关键技术要点

### 8.1 GPUI API 适配

- `Context<T>` 带 `'a` 生命周期参数：`Context<'a, T>`
- `Keystroke` 替代 `Key`
- `cx.notify()` / `cx.listener()` / `cx.spawn(async move |this, mut cx| {...})`
- `Render::render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement`
- `cx.new(|cx| Struct::new())` 创建 Entity
- `cx.open_window(options, |window, cx| cx.new(|cx| ...))` 创建窗口
- `Application::new().run(|cx| {...})` 启动应用

### 8.2 过程宏 + build.rs 协同

- `#[view]` 宏生成 `include!(concat!(env!("OUT_DIR"), "/rml_generated/<name>.rs"))`
- `build.rs` 输出同名文件到 `OUT_DIR/rml_generated/<name>.rs`
- 文件名计算：`<snake_case_struct_name>.rs`（`Counter` → `counter.rs`）
- 模板路径计算：`<snake_case_struct_name>.rml`（`Counter` → `counter.rml`）

### 8.3 元素 ID 稳定性

- ID 规则：`<view_struct>_<element_path>`（如 `Counter_root_div_buttons_button_0`）
- `each` 内元素 ID 加 key 哈希：`Counter_root_list_0_<key_hash>`
- 用于 GPUI diff 复用，避免全量重建

### 8.4 错误报告

- 包含文件名、行号、列号、源码片段、修复建议
- 通过 `cargo:warning=...` 输出
- 相似字段建议（编辑距离算法）

### 8.5 cargo:rerun-if-changed

- 每个 `.rml` 文件
- `rml_cache.json`
- 每个 `.css` 文件（Phase B-3）
- build.rs 本身（cargo 自动）

---

## 九、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| GPUI API 不稳定 | 编译失败 | 锁定 git rev，定期跟进 |
| gpui-component API 变化 | `crates/ui` 不兼容 | feature flag 隔离，按需启用 |
| 表达式解析器复杂度 | Phase B-1 延期 | 限制为字段访问 + 方法调用 + 算术，不引入完整 Rust 解析器 |
| CSS 解析器复杂度 | Phase B-3 延期 | 严格按文档子集实现，不支持完整 CSS |
| 热重载 IPC 复杂度 | Phase B-4 延期 | 单独迭代，Phase B-4 末尾实现 |
| LSP 协议完整度 | Phase B-4 延期 | 优先实现诊断 + 补全，其他功能按需 |
| 计算属性缓存线程安全 | 运行时 panic | 使用 RefCell + 单线程 GPUI 上下文 |

---

## 十、实施顺序总览

```
Phase A (已完成) ✅
    ↓
Phase B-1 (核心 codegen)
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
    ├── 依赖注入
    ├── CSS 子集解析
    ├── 样式应用
    ├── 主题系统
    └── crates/ui 扩展
    ↓
Phase B-4 (高级特性)
    ├── 事件流三阶段
    ├── 自定义事件
    ├── 防抖/节流
    ├── 热重载
    ├── LSP 服务器
    ├── CLI 工具
    ├── 日志与性能
    └── VirtualList
```

每个 Phase 完成后，必须通过对应的验证步骤（第六节）才能进入下一 Phase。
