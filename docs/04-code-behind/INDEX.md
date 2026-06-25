# 第 4 章 · Code-Behind 业务逻辑

> **本章目标**：完整掌握 `.rml.rs` 文件的全部能力——ViewModel 结构、宏属性、元素引用、命令系统、状态管理。

## 章节大纲

| 小节                                                          | 主题                  | 阅读时长   |
| ----------------------------------------------------------- | ------------------- | ------ |
| [4.1 ViewModel 结构](./viewmodel-structure.md)               | Model 派生、字段约定与构造函数 | 10 分钟  |
| [4.2 宏属性详解](./macros.md)                                   | 全部宏属性一览表            | 15 分钟  |
| [4.3 元素引用](./element-ref.md)                               | `ref` 与 `ElementRef` 的命令式访问 | 12 分钟  |
| [4.4 命令系统](./command-system.md)                            | ICommand trait 与命令参数 | 15 分钟  |
| [4.5 状态管理](./state-management.md)                         | cx.notify()、Entity 模型与跨视图状态 | 15 分钟  |

## 阅读建议

- **如果你想快速查阅**：直接看 [4.2 宏属性详解](./macros.md)，这是 `.rml.rs` 的核心。
- **如果你想系统学习**：按顺序读完五节。
- **如果你关心状态管理**：重点读 [4.5 状态管理](./state-management.md)。

## 本章核心

`.rml.rs` 文件是 ViewModel 的载体，承担三类职责：

1. **状态持有**：通过 `#[derive(Model)]` 成为 GPUI Entity
2. **命令暴露**：通过 `#[command]` 让 UI 可调用方法
3. **生命周期管理**：通过 `#[on_loaded]`、`#[on_unloaded]` 响应视图事件

掌握本章，你就能写出任何业务逻辑。

下一节 → [4.1 ViewModel 结构](./viewmodel-structure.md)
