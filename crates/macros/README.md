# rust-rml-macros

> RML 框架过程宏集合（`#[derive(IModel)]`、`#[component]`、`#[window]`、`#[command]` 等）。

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
| `#[component]` | attribute | 生成 `IModel`/`ILifecycle`/`IViewModel`/`IComponent` impl + `include!` 注入 `Render` impl |
| `#[window]` | attribute | 在 `#[component]` 基础上额外生成 `IWindow` impl（title/width/height/open/handle/set_handle），窗口操作由 trait 默认实现提供 |
| `#[command]` | attribute | 标记方法为 UI 可调用命令（Phase B 生成 `ICommand` impl） |
| `#[computed]` | attribute | 标记计算属性（Phase B 生成依赖追踪 + 缓存代码） |
| `#[on_loaded]` | attribute | 视图首次渲染完成后触发（Phase B 联动 `ILifecycle`） |
| `#[on_unloaded]` | attribute | 视图卸载前触发（Phase B 联动 `ILifecycle`） |

## 模块结构

| 模块 | 职责 |
|------|------|
| `lib.rs` | 过程宏入口，导出所有 `#[proc_macro_*]` 函数 |
| `derive_model.rs` | `#[derive(IModel)]` 实现：遍历 `pub` 字段生成 `FieldMeta` 静态数组 |
| `component.rs` | `#[component]` 实现：生成 trait impl 链 + `include!` |
| `window.rs` | `#[window]` 实现：在 `#[component]` 基础上生成 `IWindow` impl + 窗口句柄字段 |
| `command.rs` | `#[command]` 实现：Phase A pass-through，Phase B 生成 `ICommand` |
| `computed.rs` | `#[computed]` 实现：Phase A pass-through，Phase B 生成缓存代码 |
| `lifecycle.rs` | `#[on_loaded]` / `#[on_unloaded]` 实现：方法重命名 + 联动 |

## 设计规范

1. **include! 位置**：`include!` 必须在模块顶层（不能在 `const _: () = { ... }` 块内），因为生成文件含 `impl Render` 块
2. **字段插值**：`syn::Field` 未实现 `ToTokens`，必须用 `let ty = &f.ty; quote!(#ty)` 而非 `quote!(#f.ty)`
3. **命名约定**：生成的方法名以 `__rml_` 前缀（如 `__rml_on_loaded_impl`），避免与用户方法冲突
4. **helper attribute**：`#[element]` 通过 `#[derive(IModel, attributes(element))]` 声明，`#[component]` 在展开时剥离并解析
5. **pass-through 策略**：Phase A 宏仅校验签名不生成代码，Phase B 逐步补全实际实现
6. **窗口宏精简**：`#[window]` 仅生成核心方法（title/width/height/open/handle/set_handle），窗口操作（close/show/hide/activate/state）由 `IWindow` trait 默认实现提供
