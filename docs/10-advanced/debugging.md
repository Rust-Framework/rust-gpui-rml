# 10.2 调试技巧

> **本节目标**：掌握 RML 项目的调试工具链——代码展开、日志、断点、渲染树检查。

## 10.2.1 rml-expand：查看生成的代码

RML 编译器把 `.rml` 转成 Rust 代码。当代码行为异常时，先看生成的代码是否符合预期。

### 用法

```sh
cargo rml-expand views/login/login.rml
```

输出展开后的 Rust 代码：

```rust
impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .class("login-form")
            .child(
                input()
                    .attr("type", "email")
                    .bind_model(&self.handle, |vm| &vm.email)
                    .on_change(cx.listener(|this, ev, cx| this.login(ev, cx)))
            )
            // ...
    }
}
```

### 常见诊断

| 现象                | 可能原因                  |
| ----------------- | --------------------- |
| 生成的代码缺少某元素        | 模板语法错误，元素被静默忽略        |
| 绑定路径不对            | ViewModel 字段名拼写错误     |
| 事件未绑定             | `on:click` 写成 `onclick` |
| `r:if` 不生效        | 条件表达式返回非 bool         |

## 10.2.2 日志：追踪绑定与命令

### 内置日志

RML 运行时在 `RML_LOG=debug` 下输出绑定与命令的调用日志：

```sh
RML_LOG=debug cargo run 2>&1 | grep rml
```

输出示例：

```
[DEBUG rml::binding] eval binding: vm.email (deps: 1)
[DEBUG rml::command] invoke command: LoginViewModel::login
[DEBUG rml::binding] eval binding: vm.is_loading (deps: 3)
```

### 自定义日志

在命令中加 `tracing` 日志：

```rust
#[command]
pub fn login(&mut self, _ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
    tracing::info!(email = %self.email, "开始登录");
    self.is_loading = true;
    cx.notify();
    // ...
}
```

用 `RUST_LOG=my_app=debug` 过滤自己的日志。

## 10.2.3 断点调试

### VS Code 配置

`.vscode/launch.json`：

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug RML App",
      "cargo": { "args": ["build"] },
      "program": "${workspaceFolder}/target/debug/my-app"
    }
  ]
}
```

在 `.rml.rs` 的命令方法上打断点，触发事件即可命中。

### 条件断点

```rust
#[command]
pub fn on_select(&mut self, ev: &SelectEvent, cx: &mut ViewContext<Self>) {
    // 条件断点：ev.id == 42
    self.selected = Some(ev.id);
    cx.notify();
}
```

右键断点 → Edit Breakpoint → 输入 `ev.id == 42`。

## 10.2.4 渲染树检查

### RML Inspector

运行时按 `Ctrl+Shift+I` 打开 Inspector（需在 debug 构建中启用）：

```
┌─ RML Inspector ──────────────────────────────┐
│ ▼ <div class="login-form">                   │
│   ▼ <input type="email" />                   │
│   ▼ <input type="password" />                │
│   ▼ <p class="error">邮箱格式错误</p>            │
│   ▼ <button disabled>登录中…</button>          │
└──────────────────────────────────────────────┘
```

点击元素可查看：

- 绑定的 ViewModel 字段与当前值
- 应用到的样式与来源
- 事件监听器
- 元素 ID（用于 diff 调试）

### 命令行导出

```sh
cargo rml-dump-tree > tree.txt
```

把当前渲染树导出为文本，用于离线分析或对比。

## 10.2.5 状态快照

在任意时刻导出 ViewModel 状态：

```rust
#[command]
pub fn debug_snapshot(&self, cx: &mut ViewContext<Self>) {
    let snapshot = serde_json::to_string_pretty(&self).unwrap();
    log::info!("ViewModel 快照:\n{}", snapshot);
}
```

或在 Inspector 中右键 ViewModel → Export Snapshot。

## 10.2.6 常见 bug 排查清单

### UI 不更新

1. 检查命令中是否调用了 `cx.notify()`
2. 检查绑定路径是否正确（`rml-expand` 看生成代码）
3. 检查 `#[computed]` 的依赖是否真的变化
4. 检查是否在后台线程修改状态（必须通过 `this.update`）

### 事件不触发

1. 检查事件名拼写：`on:click` 而非 `onclick`（两者都支持，但混用易错）
2. 检查命令方法是否有 `#[command]` 宏
3. 检查命令签名是否匹配事件载荷
4. 检查元素是否被 `r:if="false"` 隐藏

### 绑定值不对

1. `rml-expand` 看绑定路径
2. 检查 ViewModel 字段是否 `pub`
3. 检查 `#[computed]` 是否被正确调用（模板中写函数名）
4. 检查值转换器方向（`convert` vs `convert_back`）

### 性能卡顿

1. `RML_TRACE_BINDING=1` 看绑定重算频率
2. 检查 `r:each` 是否有 `r:key`
3. 检查循环中是否 notify
4. 检查大列表是否用了虚拟化

## 10.2.7 调试技巧速查

| 问题          | 第一步工具              |
| ----------- | ------------------ |
| 行为异常        | `cargo rml-expand` |
| 绑定不更新       | `RML_LOG=debug`    |
| 事件不触发       | 断点 + Inspector     |
| 性能卡顿        | `RML_TRACE_BINDING=1` |
| 状态错乱        | 状态快照 + 日志          |
| 渲染树异常       | Inspector + dump-tree |

下一节 → [10.3 热重载](./hot-reload.md)
