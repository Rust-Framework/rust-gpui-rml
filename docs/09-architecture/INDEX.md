# 第 9 章 · 架构与最佳实践

> **本章目标**：建立 RML 项目的架构思维——明确职责边界、落地 MVVM 与 SOLID、规范项目结构、设计可测试代码、识别反模式。

## 章节大纲

| 小节                                                                  | 主题                          | 阅读时长   |
| ------------------------------------------------------------------- | --------------------------- | ------ |
| [9.1 职责归属与划分](./responsibility.md)                                | `.rml` / `.rml.rs` / Model / ViewModel / View 的边界 | 15 分钟  |
| [9.2 MVVM 模式实践](./mvvm-practice.md)                               | 三层协作契约与数据流                  | 18 分钟  |
| [9.3 SOLID 原则在 RML 中的落地](./solid-principles.md)                  | 五大原则的 RML 映射与代码示例           | 20 分钟  |
| [9.4 项目结构规范](./project-structure.md)                              | views / components / styles / models 目录约定 | 12 分钟  |
| [9.5 可测试性设计](./testability.md)                                    | ViewModel 单测、组件快照、集成测试       | 16 分钟  |
| [9.6 反模式与代码异味](./anti-patterns.md)                                | 胖 ViewModel、上帝组件、绑定爆炸        | 18 分钟  |
| [9.7 贡献点架构](./contribution-system.md)                                  | 功能模块自注册、MVVM 数据绑定、Host 消费模式 | 20 分钟  |

## 阅读建议

- **如果你是架构师 / Tech Lead**：按顺序通读全章，再回看第 1 章的架构总览。
- **如果你正在起步新项目**：先读 [9.4 项目结构规范](./project-structure.md)，再读 [9.1 职责归属与划分](./responsibility.md)。
- **如果你在重构遗留代码**：直接看 [9.6 反模式与代码异味](./anti-patterns.md)，对照清单逐项排查。
- **如果你关心工程质量**：重点读 [9.5 可测试性设计](./testability.md) 与 [9.3 SOLID 原则](./solid-principles.md)。

## 本章核心

RML 不是“能跑就行”的标记语言，而是一套**有契约、有边界、有原则**的工程化框架：

- **职责边界**：标记只描述结构，逻辑只处理状态，命令只承接事件，样式只管外观。
- **MVVM 契约**：Model 不可变、ViewModel 持状态、View 只渲染。
- **SOLID 落地**：单一职责靠拆分 ViewModel，开闭原则靠组件 + 样式继承，依赖倒置靠 trait + Context。
- **项目结构**：按“视图 / 组件 / 样式 / 模型 / 服务”五层分目录，命名即文档。
- **可测试性**：ViewModel 与 GPUI 解耦，可在无渲染环境下纯逻辑单测。
- **反模式识别**：胖 ViewModel、上帝组件、绑定爆炸、深嵌套、隐式状态——本章给出诊断与处方。

下一节 → [9.1 职责归属与划分](./responsibility.md)
