# rust-rml-core

> RML 框架核心 trait 与基础类型定义层。

## 职责

`rust-rml-core` 是整个 RML 框架的契约层，定义所有公开 trait 与基础数据类型，不包含任何编译逻辑或运行时实现。所有上层 crate（macros / engine / app / ui）均依赖此 crate。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 仅依赖 `gpui` 基础类型（`App` / `Context` / `Entity` / `SharedString` / `Pixels` 等），不依赖 `gpui` 渲染层
- 所有 trait 以 `I` 开头（`IModel` / `IViewModel` / `IRmlView` / `ICommand` / `IConverter` / `ITwoWayBinding` / `ILifecycle` / `IBindingContext` / `IEvent` / `IComponent`）

## 模块结构

| 模块 | 核心类型 | 职责 |
|------|---------|------|
| `model` | `IModel`, `FieldMeta` | 响应式数据模型标记，`rml_fields()` 返回字段元信息 |
| `view_model` | `IViewModel` | ViewModel 层契约，扩展 `IModel` + `ILifecycle` |
| `view` | `IRmlView` | RML 视图标记，声明关联 `.rml` 模板路径 |
| `component` | `IComponent` | 可复用组件契约，扩展 `IRmlView`，支持嵌套/插槽 |
| `command` | `ICommand`, `ParamMeta` | 命令系统契约，`#[command]` 方法可被 `on*` 事件绑定调用 |
| `lifecycle` | `ILifecycle` | 视图生命周期（创建→加载→更新→卸载） |
| `binding` | `BindingPath`, `IBindingContext` | 绑定路径解析与运行时订阅 |
| `converter` | `IConverter` (关联类型 `Source`/`Target`) + 内置转换器 | 值转换器，`{x \| Converter}` 语法，内置 `UpperCase`/`LowerCase`/`Trim`/`Currency`/`Percent`/`BoolToYesNo` |
| `two_way_binding` | `ITwoWayBinding` | 组件双向绑定契约 |
| `event` | `IEvent` | 事件基础契约，`prevent_default` / `stop_propagation` |
| `events` | `ClickEvent`, `MouseEvent`, ... | RML 事件对象类型（11 个） |
| `element_ref` | `ElementRef<T>` | 元素引用包装，`ref="name"` 关联 `#[element]` 字段 |
| `prelude` | — | 重导出所有常用 trait/类型，`use rml_core::prelude::*;` |

## 设计规范

1. **trait 优先**：所有公开 API 以 trait 形式定义，允许上层 crate 灵活实现
2. **零渲染依赖**：不 import `gpui::Div` / `gpui::Render` 等渲染类型，保持纯粹
3. **FieldMeta 驱动**：`IModel::rml_fields()` 返回 `&'static [FieldMeta]`，供编译期字段校验与 LSP 补全
4. **事件对象独立**：RML 事件对象（`ClickEvent` 等）与 GPUI 事件类型解耦，通过 `event_flow::convert` 桥接
5. **Default + Clone**：所有事件对象实现 `Default`，允许跨 crate 用 `default()` + 字段赋值构造
