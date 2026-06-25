# 9.4 项目结构规范

> **本节目标**：建立可扩展的 RML 项目目录约定，让“代码该放哪里”有标准答案。

## 9.4.1 推荐目录结构

一个中等规模 RML 应用的推荐结构：

```
my-app/
├── Cargo.toml
├── build.rs                  # 集成 rml-compiler
├── src/
│   ├── main.rs               # 应用入口
│   ├── app.rml               # 根视图
│   ├── app.rml.rs            # 根 ViewModel
│   │
│   ├── views/                # 页面级视图（每个对应一个路由 / 窗口）
│   │   ├── login/
│   │   │   ├── login.rml
│   │   │   ├── login.rml.rs
│   │   │   └── mod.rs
│   │   ├── dashboard/
│   │   │   ├── dashboard.rml
│   │   │   ├── dashboard.rml.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   ├── components/           # 可复用组件（跨视图）
│   │   ├── button/
│   │   │   ├── button.rml
│   │   │   ├── button.rml.rs
│   │   │   └── mod.rs
│   │   ├── dialog/
│   │   │   ├── dialog.rml
│   │   │   ├── dialog.rml.rs
│   │   │   └── mod.rs
│   │   └── mod.rs
│   │
│   ├── models/               # 纯数据 Model（无 GPUI 依赖）
│   │   ├── user.rs
│   │   ├── todo.rs
│   │   └── mod.rs
│   │
│   ├── services/             # I/O 层：网络、文件、数据库
│   │   ├── auth.rs
│   │   ├── todo_service.rs
│   │   └── mod.rs
│   │
│   ├── styles/               # 全局样式与主题
│   │   ├── theme.rmlcss      # 主题变量
│   │   ├── reset.rmlcss      # 重置样式
│   │   └── mod.rs
│   │
│   ├── context/              # 全局状态 / Context
│   │   ├── auth_context.rs
│   │   ├── theme_context.rs
│   │   └── mod.rs
│   │
│   └── utils/                # 工具函数
│       ├── format.rs
│       └── mod.rs
│
└── tests/                    # 集成测试
    ├── login_flow.rs
    └── todo_crud.rs
```

## 9.4.2 分层约定

### views/ —— 页面级视图

- 每个视图一个独立目录，包含 `.rml` + `.rml.rs` + `mod.rs`
- 视图对应“一个路由 / 一个窗口 / 一个独立功能页”
- 视图之间通过 Context 事件或路由跳转通信，**不直接互调**

```rust
// views/login/mod.rs
mod login;
pub use login::LoginViewModel;
```

### components/ —— 可复用组件

- 跨视图复用的 UI 单元
- 每个 component 自带 `.rml` + `.rml.rs`
- 组件应当**无业务知识**：只接受 props、触发事件，不直接调用 Service

```rust
// components/button/mod.rs
mod button;
pub use button::Button;
```

### models/ —— 纯数据

- `#[derive(Model, Clone, Debug)]` 的结构体
- 可包含 `impl` 中的纯函数式方法
- **不依赖** `gpui::*`，可独立编译为 crate

### services/ —— I/O 层

- 网络、文件、数据库、第三方 SDK 调用
- 通常以 `async fn` 形式暴露
- 返回 `Result<T>`，错误由 ViewModel 处理

```rust
// services/auth.rs
pub async fn login(email: &str, password: &str, cx: &mut AsyncApp) -> Result<Token> { ... }
```

### styles/ —— 全局样式

- 主题变量、重置样式、全局工具类
- 组件级样式放在组件目录内（`components/button/button.rmlcss`）

### context/ —— 全局状态

- 跨视图共享的状态：登录态、主题、用户配置
- 通过 `cx.set_global` / `cx.global` 访问

## 9.4.3 命名约定

| 类型          | 命名规则                          | 示例                          |
| ----------- | ----------------------------- | --------------------------- |
| 视图文件        | 与视图名同源，全小写下划线                 | `user_profile.rml`          |
| ViewModel   | 大驼峰 + `ViewModel` 后缀          | `UserProfileViewModel`      |
| 组件          | 大驼峰，无后缀                       | `Button`、`Dialog`           |
| 命令方法        | 动词开头，小驼峰                      | `on_submit`、`load_data`     |
| 计算属性        | 名词或 is/has 开头                  | `full_name`、`is_loading`    |
| Model       | 大驼峰，无后缀                       | `User`、`TodoItem`           |
| Service 模块  | 名词 + `_service` 或领域名          | `auth_service.rs`           |

## 9.4.4 模块边界规则

### 依赖方向

```
views ──▶ components ──▶ models
  │            │
  ▼            ▼
services ◀─────
  │
  ▼
context / utils
```

- `views` 可依赖 `components`、`services`、`models`、`context`
- `components` 可依赖 `models`、`context`，**不依赖** `views` 或 `services`
- `models` 不依赖任何上层
- `services` 可依赖 `models`、`context`，**不依赖** `views` 或 `components`

违反依赖方向 = 循环依赖 = 编译失败或测试困难。

### 禁止的依赖

- ❌ `components` 调用 `services`：组件应当无业务知识
- ❌ `models` 依赖 `gpui`：Model 必须可独立编译
- ❌ `views` 之间互相 `use`：通过 Context 事件通信

## 9.4.5 大型项目的拆分

当单 crate 膨胀到编译缓慢时，按下列维度拆 crate：

```
my-app/
├── Cargo.toml                 # workspace 根
├── crates/
│   ├── app/                   # 主应用 crate
│   ├── ui-kit/                # 组件库 crate（可独立发布）
│   ├── domain-models/         # Model crate（无 GPUI 依赖）
│   ├── services/              # Service crate
│   └── theme/                 # 主题 crate
```

拆分原则：

1. **ui-kit** 可独立发布，被多个项目复用
2. **domain-models** 无 GPUI 依赖，可被 CLI / 服务端复用
3. **services** 依赖 `domain-models`，可被多端复用
4. **app** 组装一切，是唯一可执行 crate

## 9.4.6 资源文件组织

非代码资源（图标、字体、本地化字符串）放在 `assets/`：

```
assets/
├── icons/
│   ├── logo.svg
│   └── ...
├── fonts/
│   └── inter.woff2
└── i18n/
    ├── zh-CN.json
    └── en-US.json
```

通过 `cx.asset("icons/logo.svg")` 引用，编译期校验路径存在。

## 9.4.7 测试组织

```
tests/                          # 集成测试（跨模块）
├── login_flow.rs
└── todo_crud.rs

src/views/login/login.rml.rs    # 单元测试在模块内 #[cfg(test)]
```

- **单元测试**：放在模块内 `#[cfg(test)] mod tests`，测试 ViewModel 纯逻辑
- **集成测试**：放在 `tests/`，测试跨模块流程
- **快照测试**：组件 `.rml` 渲染结果快照，放在 `tests/snapshots/`

## 9.4.8 项目结构检查清单

- [ ] 每个视图有独立目录，含 `.rml` + `.rml.rs`
- [ ] 组件目录无业务逻辑代码
- [ ] `models/` 中无 `gpui::*` import
- [ ] `services/` 中无 `cx.notify()` 调用
- [ ] 视图之间无直接 `use`，通过 Context 通信
- [ ] 全局样式在 `styles/`，组件样式在组件目录
- [ ] 命名遵循约定，无 `ViewModel2`、`temp` 等模糊命名
- [ ] 资源文件在 `assets/`，不散落在源码目录

下一节 → [9.5 可测试性设计](./testability.md)
