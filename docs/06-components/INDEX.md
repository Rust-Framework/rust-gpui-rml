# 第 6 章 · 组件系统

> **本章目标**：完整掌握 RML 的组件系统——从内置组件到自定义组件，从插槽到组合，构建可复用的 UI 单元。

## 章节大纲

| 小节                                                              | 主题                | 阅读时长   |
| --------------------------------------------------------------- | ----------------- | ------ |
| [6.1 内置组件](./builtin-components.md)                            | 标签体系与路由表索引        | 8 分钟   |
| [**6.x 组件参考**](./reference/INDEX.md)                           | 逐组件属性/事件/示例（权威）  | 按需查阅   |
| [6.2 自定义组件](./custom-components.md)                           | #[component] 宏与模板 | 15 分钟  |
| [6.3 插槽与内容分发](./slots.md)                                     | slot 与具名插槽        | 12 分钟  |
| [6.4 组件属性](./component-props.md)                              | 输入属性、事件属性、双向绑定    | 12 分钟  |
| [6.5 组件组合](./composition.md)                                  | 父子组件、兄弟组件、依赖注入    | 12 分钟  |

## 阅读建议

- **查某个标签怎么用**：直接打开 [组件参考目录](./reference/INDEX.md)。
- **如果你想快速上手自定义组件**：读 [6.2 自定义组件](./custom-components.md)。
- **如果你在做 Shell / 扩展点**：读 [ActivityBar](./reference/activity-bar.md)、[Tree](./reference/tree.md) 参考 + [贡献点架构](../09-architecture/contribution-system.md)。
- **如果你关心复用性**：重点读 [6.3 插槽](./slots.md) 和 [6.5 组件组合](./composition.md)。

## 本章核心

RML 的组件系统是构建大型应用的基础：

- **内置组件**：以 `tags.rs` 路由表为准，分 HTML 基础轨与 gpui-component 扩展轨
- **组件参考**：`reference/` 下每个已注册标签一份文档，属性仅收录 codegen 实际支持的项
- **自定义组件**：用 `#[component]` 宏封装可复用的 UI 单元
- **插槽**：窗口插槽（`slot_left` 等）与组件 `<slot>` 实现内容分发
- **MVVM Shell 控件**：ActivityBar / menu / status_bar / Tree 由 ViewModel 数据驱动

下一节 → [6.1 内置组件](./builtin-components.md)
