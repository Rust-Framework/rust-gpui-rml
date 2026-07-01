# rust-rml-demo

> RML Showcase demo —— 验证 `.rml` + `.rml.rs` + `build.rs` 三件套闭环 + TabWindow Shell + 贡献点架构。

## 职责

端到端验证 RML 框架开发流程：模板（`.rml`）、Code-Behind（`.rml.rs`）、构建脚本（`build.rs`），以及 `RmlApplication::main_window` 声明式启动。

## 样式策略（defaults-first）

默认外观由 **gpui-component Theme** + **rml_ui Shell 组件** + **语义 HTML 标签** 提供，无需大量 CSS。

| 文件 | 角色 |
|------|------|
| `src/app.rs` | 启动时 `Theme.font_size = 14px` |
| `assets/themes/{light,dark}.css` | 颜色变量（含 `--text-muted`） |
| `assets/styles.css` | **可选覆盖层**：demo 布局 utility（`.case-pane` 等），非必需 |

需要定制时再通过 CSS class 或组件 builder（如 `MenuBar::gap()`）覆盖。

## 运行

```bash
cargo run -p rust-rml-demo
```

## 主要目录

| 路径 | 说明 |
|------|------|
| `src/shell/` | MainWindow、LoginDialog、ActivityBar 树 |
| `src/cases/` | 各功能案例 RML 组件 |
| `src/app.rs` | 全局 init：style / i18n / theme / 贡献点注册 |
