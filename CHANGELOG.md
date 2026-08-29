# Changelog

All notable changes to **RML (rust-gpui-rml)** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v1.0.0] — 2026-08-16 — 开源发布 · Open-Source Release

> **English** · **简体中文**

### English

**RML** (Rust Markup Language for GPUI) is released as open source under the
`Rust-Framework` organization.

### Added

- Compile-time declarative UI framework: `.rml` markup + `.rml.rs` code-behind +
  `build.rs` compile-time expansion
- MVVM data binding, slot UI, data-driven menu / status-bar binding
- Full repository source + bilingual (English / Chinese) documentation system
- Repo clean-up: removed build artifacts, `.gitignore` excludes `.trae/` and
  `.agents/`, added `docs/` landing page

> Note: this project depends on the zed GPUI git dependency and is **not published to
> crates.io**; it is distributed as source.

---

### 简体中文

**RML**（Rust Markup Language for GPUI）以完整源码形式开源到 `Rust-Framework` 组织。

### 新增

- **开源发布**：项目以完整源码形式开源到 `Rust-Framework` 组织下的 `rust-gpui-rml` 仓库。
  - 运行时编译的声明式 UI 框架（`.rml` + `.rml.rs` 代码后置 + `build.rs` 编译期展开）
  - mvvm 数据绑定、插槽 UI、菜单 / 状态栏数据驱动绑定
  - 全量仓库源码 + 中英文 README 文档体系
- **仓库规范化**：清理调试 / 构建产物，`.gitignore` 排除 `.trae/` 与 `.agents/`，新增 `docs/` 落地页。

> 说明：本项目强依赖 zed GPUI 的 git 依赖，目前**未发布到 crates.io**，以仓库源码形式分发。

[v1.0.0]: https://github.com/Rust-Framework/rust-gpui-rml/releases/tag/v1.0.0