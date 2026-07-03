# 10.5 IDE 支持

> **本节目标**：了解 RML 的 IDE 工具链——语法高亮、自动补全、跳转定义、诊断与格式化。

## 10.5.1 VS Code 插件

RML 官方提供 VS Code 插件 `rml-vscode`，安装后获得：

- `.rml` 文件语法高亮
- 标签 / 属性自动补全
- 跳转到 ViewModel 定义
- 实时诊断（绑定路径、命令签名）
- 格式化

### 安装

在 VS Code 扩展市场搜索 `RML`，或命令行：

```sh
code --install-extension rml.rml-vscode
```

### 配置

`.vscode/settings.json`：

```json
{
  "rml.server.path": "~/.cargo/bin/rml-lsp",
  "rml.format.onSave": true,
  "rml.diagnostics.strict": true
}
```

## 10.5.2 语法高亮

插件基于 TextMate 语法提供高亮，覆盖：

- HTML 标签与属性
- RML 指令（`r:if`、`r:each`、`r:model`…）
- 插值表达式 `{...}`
- 事件绑定 `on:click`
- 注释 `<!-- -->`

支持自定义主题适配，遵循 VS Code 主题 token 规范。

## 10.5.3 自动补全

补全由 LSP 服务器 `rml-lsp` 提供，基于项目实际类型信息。

### 标签补全

输入 `<` 后，列出所有可用标签：

- HTML 标准标签（`div`、`span`、`button`…）
- 项目自定义组件（`<Button>`、`<Dialog>`…）
- 内置组件（`<VirtualList>`、`<If>`…）

### 属性补全

在标签内输入属性名时，列出该标签支持的属性：

```html
<input type="|" placeholder="|" r:model="|" />
```

补全来源：

- HTML 标准属性
- 组件 props（来自 `#[component]` 定义）
- RML 指令

### 绑定路径补全

在 `r:model="|"` 或 `{ | }` 中，补全 ViewModel 的字段与计算属性：

```html
<!-- 输入 user. 后补全 -->
<span>{user.}</span>
<!-- 补全项：name, email, role, full_name(), is_admin() -->
```

### 命令补全

在 `on:click="|"` 中，补全 ViewModel 的 `#[command]` 方法：

```html
<button on:click="|">
<!-- 补全项：login, logout, toggle_remember -->
```

## 10.5.4 跳转定义

- 在 `<Button>` 上 `Ctrl+Click` → 跳到 `Button` 组件定义
- 在 `r:model="email"` 上 `Ctrl+Click` → 跳到 ViewModel 的 `email` 字段
- 在 `on:click="login"` 上 `Ctrl+Click` → 跳到 `login` 命令方法
- 在 `{user.name}` 上 `Ctrl+Click` → 跳到 `User::name` 字段

## 10.5.5 实时诊断

LSP 在编辑时实时检查：

| 检查项                | 严重级别 |
| ------------------ | ---- |
| 绑定路径不存在            | 错误   |
| 命令方法不存在            | 错误   |
| 命令签名与事件不匹配         | 错误   |
| `r:each` 缺少 `r:key` | 警告   |
| 未使用的 `r:if` 条件     | 警告   |
| 样式类未定义             | 警告   |

诊断基于项目实际类型，不是语法猜测。

## 10.5.6 格式化

`Shift+Alt+F` 格式化当前 `.rml` 文件：

- 标签缩进 2 空格
- 属性换行规则：超过 100 字符换行
- 自闭合标签：`<input />` 而非 `<input></input>`
- 插值表达式前后无空格：`{name}` 而非 `{ name }`

格式化规则可在 `.rmlfmt.toml` 中自定义：

```toml
indent = 2
max_line_length = 100
self_closing = true
```

## 10.5.7 其他 IDE 支持

### JetBrains 系列

通过 LSP 插件（如 IntelliJ LSP）接入 `rml-lsp`，获得补全与诊断。语法高亮需手动配置 TextMate 语法。

### Neovim

```lua
-- init.lua
require('lspconfig').rml_lsp.setup{}
```

配合 Treesitter 的 `rml` grammar 获得高亮。

### Helix

Helix 内置 LSP 支持，安装 `rml-lsp` 到 PATH 即可自动启用。

## 10.5.8 命令行工具

### rml-lsp

LSP 服务器，可被任何 LSP 客户端调用：

```sh
rml-lsp --stdio
```

### rml-fmt

独立格式化工具：

```sh
rml-fmt src/**/*.rml --write
```

### rml-check

类似 `cargo check`，只做语义检查不生成代码：

```sh
rml-check
```

输出所有错误与警告，适合 CI 中使用。

## 10.5.9 IDE 工作流推荐

1. **打开项目**：插件自动启动 LSP，索引全项目
2. **编辑模板**：实时补全 + 诊断
3. **保存**：自动格式化 + 触发热重载
4. **跳转**：`Ctrl+Click` 在 `.rml` 与 `.rml.rs` 间穿梭
5. **重构**：重命名 ViewModel 字段时，LSP 同步更新所有 `.rml` 中的绑定

## 10.5.10 故障排查

### LSP 不启动

- 检查 `rml.server.path` 是否指向有效二进制
- 检查项目根目录是否有 `Cargo.toml`
- 查看 VS Code 输出面板的 `RML` 通道

### 补全不出现

- 等待索引完成（首次启动可能 10-30s）
- 检查 ViewModel 是否有 `#[derive(IModel)]`
- 检查字段是否 `pub`

### 诊断不更新

- 保存 `.rml.rs` 文件触发 LSP 重新分析
- 重启 LSP：`Ctrl+Shift+P` → `RML: Restart Server`

下一节 → [10.6 代码生成原理](./code-generation.md)
