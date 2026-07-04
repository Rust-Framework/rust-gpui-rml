# rust-rml-macros

> RML 框架过程宏集合（`#[derive(IModel)]`、`#[component]`、`#[window]`、`#[command]`、`#[rml::main]` 等）。

## 职责

`rust-rml-macros` 是过程宏 crate（`proc-macro = true`），为用户代码提供声明式宏属性，自动生成 trait 实现代码。**过程宏不做重活**——模板编译由 `build.rs` + engine 完成，宏仅负责生成 trait impl 与 `include!` 注入。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 过程宏不做模板解析/编译，仅生成 trait impl 骨架
- 生成的 `include!` 路径指向 `OUT_DIR/rml_generated/<snake>.rs`，由 `build.rs` 产出

## 宏清单

| 宏 | 类型 | 生成内容 |
|----|------|---------|
| `#[derive(IModel)]` | derive | 为 `pub` 字段生成 `FieldMeta`，实现 `IModel::rml_fields()` |
| `#[component]` | attribute | 生成 `IModel`/`ILifecycle`/`IViewModel`/`IComponent` impl + `include!` 注入 `Render` impl；注入 `__rml_state: RmlState` 字段（统一承载版本追踪/缓存/InputState/校验/插槽等运行时状态） |
| `#[window]` | attribute | 在 `#[component]` 基础上额外生成 `IWindow` impl（title/width/height/open/handle/set_handle），窗口操作由 trait 默认实现提供 |
| `#[command]` | attribute | 标记方法为 UI 可调用命令：生成 `ICommand` impl + **自动注入 `bump_version` 与 `cx.notify()`**（见下方 MVVM 数据驱动） |
| `#[command(no_notify)]` | attribute | 同上但禁用自动 notify，适合需要精确控制 notify 时机的场景 |
| `#[computed]` | attribute | 标记计算属性：重命名为 `__rml_computed_<name>`，生成版本感知缓存 wrapper（通过 `ComputedCache::get_or_compute::<T>`） |
| `#[validate]` | attribute | 字段校验：支持 `range`/`length`/`required`/`regex`/`custom` 规则，或 `#[validate(MyValidator)]` 接口式校验（C# Attribute 风格） |
| `#[on_loaded]` | attribute | 视图首次渲染完成后触发（联动 `ILifecycle`） |
| `#[on_unloaded]` | attribute | 视图卸载前触发（联动 `ILifecycle`） |
| `#[rml::main]` | attribute | **资源单点入口**：在 `fn main` 之前注入 `rml::embed_assets!()`，触发 build.rs 生成的 `rml_assets.rs` 的 `include!()`；与 `#[ctor::ctor]` 协同实现 main.rs 零资源代码 |

## 模块结构

| 模块 | 职责 |
|------|------|
| `lib.rs` | 过程宏入口，导出所有 `#[proc_macro_*]` 函数 |
| `derive_model.rs` | `#[derive(IModel)]` 实现：遍历 `pub` 字段生成 `FieldMeta` 静态数组 |
| `component.rs` | `#[component]` 实现：生成 trait impl 链 + `include!` + 注入 observable 追踪字段 |
| `window.rs` | `#[window]` 实现：在 `#[component]` 基础上生成 `IWindow` impl + 窗口句柄字段 |
| `command.rs` | `#[command]` 实现：AST 扫描字段修改 + 自动注入 `bump_version` / `cx.notify()` |
| `computed.rs` | `#[computed]` 实现：方法重命名 + 版本感知缓存 wrapper 生成 |
| `validate.rs` | `#[validate]` 实现：规则式（range/length/required/regex/custom）与接口式（`IValidate`）两种校验 codegen |
| `lifecycle.rs` | `#[on_loaded]` / `#[on_unloaded]` 实现：方法重命名 + 联动 |
| `main_attr.rs` | `#[rml::main]` 实现：注入 `rml::embed_assets!()` 触发资源 `include!` |

## 设计规范

1. **include! 位置**：`include!` 必须在模块顶层（不能在 `const _: () = { ... }` 块内），因为生成文件含 `impl Render` 块
2. **字段插值**：`syn::Field` 未实现 `ToTokens`，必须用 `let ty = &f.ty; quote!(#ty)` 而非 `quote!(#f.ty)`
3. **命名约定**：生成的方法名以 `__rml_` 前缀（如 `__rml_on_loaded_impl`、`__rml_bump_version`），字段统一收敛到 `__rml_state: RmlState`，避免与用户方法/字段冲突
4. **helper attribute**：`#[element]` 通过 `#[derive(IModel, attributes(element))]` 声明，`#[component]` 在展开时剥离并解析
5. **窗口宏精简**：`#[window]` 仅生成核心方法（title/width/height/open/handle/set_handle），窗口操作（close/show/hide/activate/state）由 `IWindow` trait 默认实现提供
6. **MVVM 数据驱动（`#[command]`）**：
   - 宏在编译期扫描方法体 AST，识别 `self.<field> = ...` 与 `self.<field> += ...` 等直接赋值/复合赋值模式（syn 2.x 中复合赋值是 `Expr::Binary` + `BinOp::AddAssign` 等）
   - 命中后自动在赋值语句后注入 `self.__rml_bump_version("<field>")`
   - 当方法返回类型为 `()` 且存在 `&mut Context<Self>` 参数时，在方法末尾自动追加 `cx.notify()`
   - **AST 模式匹配的边界**（需在文档中诚实说明）：
     - ✅ 识别：`self.count = 0`、`self.count += 1`、`self.name = "x".into()`
     - ⚠️ 不识别：局部借用变量修改（`let p = &mut self.x; *p = 1;`）、方法调用修改（`self.items.push()`、`self.items.retain()`）
     - ⚠️ 不注入：异步闭包内（`cx.spawn`、`this.update`）—— 宏只注入命令方法体本身，不递归进入闭包
   - 不识别 / 不注入的场景需在代码中手写 `cx.notify()`，并用注释 `// ⚠️ 局部借用修改，宏不识别，需手动 notify` 标注
7. **`#[command(no_notify)]`**：禁用自动 notify 注入，用于：返回非 `()` 类型、批量更新后想控制 notify 时机、或完全由异步任务驱动 UI 的场景
8. **`#[rml::main]` 单点入口**：在 `fn main` 之前注入 `rml::embed_assets!()` 宏调用，该宏展开为 `include!(concat!(env!("OUT_DIR"), "/rml_assets.rs"))`，将 build.rs 生成的资源注册代码（含 `#[ctor::ctor]` 函数）链接到用户 crate 的编译单元。**必须在用户 crate 中展开**，因为 `include_bytes!` 需要在用户 crate 的编译上下文中展开
