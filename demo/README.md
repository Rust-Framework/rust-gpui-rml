# rust-rml-demo

> RML 计数器 demo —— 验证 `.rml` + `.rml.rs` + `build.rs` 三件套闭环 + WPF 风格窗口 API。

## 职责

端到端验证 RML 框架的核心开发流程：模板文件（`.rml`）、Code-Behind（`.rml.rs`）、构建脚本（`build.rs`）协同工作，从模板编译到 GPUI 渲染到事件处理的完整闭环。同时验证 `#[window]` 宏 + `RmlApplication::main_window` 声明式 API。

## 文件结构

| 文件 | 职责 |
|------|------|
| `src/main.rs` | demo 入口，`RmlApplication::new().main_window::<Counter>().run()` |
| `src/counter.rml` | Counter 模板（div > h1 + p{count} + 2 buttons） |
| `src/counter.rml.rs` | Counter ViewModel：`#[window]` 标记 + `count: i32` + `#[command]` increment/decrement |
| `build.rs` | 构建脚本：`rml::build().scan_dir("src").output_dir(OUT_DIR).build()` |

## 运行

```bash
cargo run -p rust-rml-demo
```

## 开发流程（三件套）

1. **编写 `.rml` 模板**：声明式描述 UI 结构（HTML 语法 + RML 指令 + `{插值}`）
2. **编写 `.rml.rs` Code-Behind**：`#[window]` 标记 ViewModel 结构体，`#[command]` 标记事件方法
3. **配置 `build.rs`**：调用 `rml::build()` 扫描 `.rml` 文件，编译为 `OUT_DIR/rml_generated/<name>.rs`
4. **`#[window]` 宏**：自动生成 `IComponent` + `IWindow` impl，`include!` 注入 `Render` impl

## 验证要点

- 点击 `-` / `+` 按钮，`count` 字段增减，UI 实时更新
- `#[command]` 方法签名：`fn(&mut self, &ClickEvent, &mut Context<Self>)`
- `cx.notify()` 触发重渲染
- 事件对象通过 `event_flow::convert::from_gpui_click` 从 GPUI ClickEvent 转换
- `RmlApplication::new().main_window::<Counter>().run()` 声明式 API 启动窗口
