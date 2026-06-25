# 1.2 架构总览

> **本节目标**：用一张图讲清 RML 的三层架构、五个 crates 的职责划分，以及 MVVM 数据流如何在 `.rml` 与 `.rml.rs` 之间流动。

## 1.2.1 三层架构

RML 把整个开发栈分为三层：表现层、框架层、编译层。三者通过清晰的契约协作。

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Presentation Layer                            │
│  ┌──────────────────┐    ┌──────────────────────────────────────┐   │
│  │   window.rml     │    │        window.rml.rs                 │   │
│  │   (UI 标记)      │───▶│   (Code-Behind 业务逻辑)            │   │
│  │   - 控件树       │    │   - 状态字段 (Model)                │   │
│  │   - 布局属性     │    │   - 事件处理器                      │   │
│  │   - 数据绑定     │    │   - 计算属性                        │   │
│  │   - 事件绑定     │    │   - 生命周期回调                    │   │
│  └──────────────────┘    └──────────────────────────────────────┘   │
│           │                              │                          │
│           ▼                              ▼                          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │              RML Compiler (build.rs / 过程宏)                │   │
│  │   .rml 解析 → AST 转换 → 语义验证 → GPUI 代码生成           │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       Framework Layer                               │
│  ┌───────────────┐  ┌───────────────┐  ┌───────────────────────┐   │
│  │     GPUI      │  │gpui-component │  │    RML Runtime        │   │
│  │  (渲染引擎)   │  │  (组件库)     │  │   (绑定系统/热重载)   │   │
│  └───────────────┘  └───────────────┘  └───────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

### 表现层（Presentation Layer）

开发者直接编辑的层，由两类文件组成：

- **`.rml`**：UI 标记文件，描述控件树、布局属性、数据绑定、事件绑定。设计师友好。
- **`.rml.rs`**：Code-Behind 文件，承载 ViewModel 角色，包含状态字段、事件处理器、计算属性、生命周期回调。工程师友好。

### 编译层（Compiler Layer）

由 `build.rs` 调用，对开发者透明：

1. **解析**：把 `.rml` 解析为 AST
2. **转换**：把 AST 转换为语义模型
3. **验证**：检查绑定路径、类型匹配、指令合法性
4. **生成**：输出 `.rml.generated.rs`，包含 `Render` trait 的实现

### 框架层（Framework Layer）

运行时支撑：

- **GPUI**：底层渲染引擎，提供 `div`、`Label`、`Entity`、`ViewContext` 等原语
- **gpui-component**：组件库，提供 `Button`、`Input`、`Dialog` 等高层组件
- **RML Runtime**：绑定系统、热重载、生命周期回调的运行时支持

## 1.2.2 Workspace 结构

RML 框架本身由 5 个独立 crates 组成，模块化设计便于单独复用或替换：

```
rml-framework/
├── Cargo.toml                          # Workspace 根配置
├── crates/
│   ├── core/                           # RML 框架核心
│   │   └── src/
│   │       ├── lib.rs                  # 核心 trait 导出
│   │       ├── view_model.rs           # ViewModel trait 定义
│   │       ├── binding.rs              # 绑定引擎
│   │       ├── command.rs              # 命令系统 (ICommand)
│   │       ├── converter.rs            # 值转换器 trait
│   │       └── lifecycle.rs            # 视图生命周期回调
│   │
│   ├── rml/                            # RML 解析引擎
│   │   └── src/
│   │       ├── parser/                 # 语法解析器
│   │       │   ├── tokenizer.rs        # HTML/XML 词法分析
│   │       │   └── ast.rs              # 抽象语法树
│   │       ├── compiler/               # 编译器 (.rml → Rust)
│   │       │   ├── codegen.rs          # 代码生成器
│   │       │   └── validator.rs        # 语义验证
│   │       └── runtime/                # 运行时支持
│   │           └── watcher.rs          # 热重载文件监听
│   │
│   ├── macros/                         # 过程宏定义
│   │   └── src/
│   │       ├── view.rs                 # #[view] 属性宏
│   │       ├── component.rs            # #[component] 组件宏
│   │       ├── command.rs              # #[command] 命令宏
│   │       └── computed.rs             # #[computed] 计算属性宏
│   │
│   └── app/                            # RML 应用框架
│       └── src/
│           ├── application.rs          # 应用启动器
│           ├── window.rs               # 窗口管理
│           └── resources.rs            # 资源加载
│
├── demo/                               # 示例项目
│   ├── build.rs                        # 构建脚本
│   └── src/
│       ├── main.rs
│       ├── views/
│       │   ├── counter.rml
│       │   └── counter.rml.rs
│       ├── components/
│       │   └── button.rml
│       └── styles/
│           └── theme.rml
│
└── target/
    └── generated/                      # .rml 编译生成的 .rs 文件
```

### 各 crate 的职责

| Crate             | 职责                                  | 依赖方向              |
| ----------------- | ----------------------------------- | ----------------- |
| `rml-core`        | 定义 `RmlView`、`BindingContext` 等基础 trait | 被所有其他 crate 依赖    |
| `rml`             | 解析 `.rml`、生成 Rust 代码                | 依赖 `rml-core`     |
| `rml-macros`      | 提供 `#[view]`、`#[component]` 等过程宏    | 依赖 `rml-core`     |
| `rml-app`         | 应用启动器、窗口管理、资源加载                     | 依赖 `rml-core`、GPUI |
| `gpui-component`  | 高层组件库（Button、Input、Dialog 等）         | 依赖 GPUI           |

💡 **设计要点**：`rml-core` 不依赖 GPUI，这意味着未来可以把 RML 编译到其他后端（如 Web、SwiftUI），只需替换 `rml` 编译器的代码生成目标。

## 1.2.3 MVVM 数据流

RML 完全对标 WPF 的 MVVM 模式。理解数据流是理解 RML 的关键。

```
┌──────────┐    数据绑定     ┌──────────┐    渲染     ┌──────────┐
│  Model   │ ◀────────────▶ │ ViewModel │ ─────────▶ │   View   │
│ (数据)   │    (双向/单向)  │  (状态)   │   (GPUI)   │  (.rml)  │
└──────────┘                └──────────┘            └──────────┘
      │                            │                       │
      ▼                            ▼                       ▼
  业务逻辑                     命令/事件                用户交互
  (Rust)                      (Rust)                  (点击/输入)
```

### 三层的职责契约

| 层级          | 文件                  | 职责                          | 禁止事项            |
| ----------- | ------------------- | --------------------------- | --------------- |
| **Model**   | 任意 `.rs`            | 纯数据结构、业务规则                  | 不含 UI 逻辑、不依赖 GPUI |
| **ViewModel** | `.rml.rs`           | 持有状态、响应命令、暴露计算属性、生命周期回调     | 不构造 UI 元素       |
| **View**    | `.rml`              | 描述结构、样式、绑定、事件               | 不含业务逻辑          |

### 数据流的方向

1. **用户交互** → View 触发事件 → ViewModel 的 `#[command]` 方法
2. **命令执行** → ViewModel 修改状态 → 调用 `cx.notify()`
3. **状态变更** → 绑定系统通知 View → 重新渲染受影响的部分
4. **双向绑定** → `model` 指令自动把输入回写到 ViewModel 字段

```rust
// ViewModel 修改状态后必须调用 cx.notify() 触发重绘
#[command]
pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();  // ← 关键：通知 View 重新读取绑定
}
```

⚠️ **常见错误**：忘记调用 `cx.notify()` 会导致 UI 不更新。这是 RML 开发中最常见的"为什么我的界面没反应"问题的根因。详见 [第 8 章 · 状态生命周期](../08-lifecycle/state-lifecycle.md)。

## 1.2.4 编译时 vs 运行时

RML 的一个核心设计决策是：**所有绑定路径检查和代码生成都在编译期完成**。

| 阶段   | 发生什么                                    | 开发者感知         |
| ---- | --------------------------------------- | ------------- |
| 编译期  | `.rml` → AST → 语义验证 → `.rml.generated.rs` | `cargo build` 时间略增 |
| 运行时  | 加载生成的代码、绑定订阅、热重载监听                      | 零反射、零解析开销     |

这意味着：

- ✅ 绑定路径错误在编译期暴露，不会带到运行时
- ✅ 生成的代码是原生 GPUI 调用，性能与手写一致
- ✅ IDE 可以基于生成的代码提供完整的 Rust-analyzer 补全
- ⚠️ `.rml` 修改需要重新编译（开发模式下热重载可缓解）

## 1.2.5 小结

RML 的架构可以用三句话概括：

1. **表现层**：`.rml` 描述 View，`.rml.rs` 承载 ViewModel，文件级分离。
2. **编译层**：`build.rs` 在编译期把 `.rml` 转为原生 GPUI 代码，零运行时开销。
3. **框架层**：GPUI 负责渲染，gpui-component 提供组件，RML Runtime 提供绑定与热重载。

掌握这张架构图，你就掌握了 RML 的"地图"。后续章节会逐层深入每一层的细节。

下一节 → [1.3 快速开始](./quick-start.md)
