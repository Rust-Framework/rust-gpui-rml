# rust-gpui-rml 文档

本目录为 **RML 框架使用文档** 的 canonical 源。**RML（Rust Markup Language）** 是构建于 GPUI 之上的声明式 UI 框架，本手册依据 RML 框架使用文档 · 总目录整理。

## 结构

```
docs/
├── FOREWORD.md       # 前言
├── INDEX.md          # 总目录（RML 框架使用文档 · 总目录）
├── INDEX.json        # 文档网站左侧菜单
└── 01-overview/      # 章节（共 11 章）
    ├── 02-syntax/      # RML 标记语言
    ├── 03-binding/     # 数据绑定系统
    ├── 04-code-behind/ # Code-Behind 业务逻辑
    ├── 05-events/      # 事件系统
    ├── 06-components/  # 组件系统
    ├── 07-styling/     # 样式与主题
    ├── 08-lifecycle/   # 生命周期管理
    ├── 09-architecture/# 架构与最佳实践
    ├── 10-advanced/    # 高级技巧与工具链
    └── 11-cookbook/    # Cookbook
```

## 阅读

- [前言](FOREWORD.md) — 了解本书定位、读者画像与阅读路径
- [总目录](INDEX.md) — RML 框架使用文档 · 总目录
- [左侧菜单索引](INDEX.json)

> 手册采用**渐进式披露（progressive disclosure）**导航：总目录为最高层索引，各章 `INDEX.md` 提供小节级索引，小节内提供段落级锚点，请按层级逐层深入。

## 阅读建议

- **新手入门**：按 **第 1 章 概览** → **第 2 章 标记语言** → **第 6 章 组件系统** 的顺序阅读，建立整体认知
- **解决问题**：直接查阅[第 11 章 Cookbook](11-cookbook/)，含 FAQ、案例研究与坑位清单

## 维护

编辑 `docs/` 下的 Markdown 即可；Docbit 启动时会自动确保 `INDEX.json` 存在。