# Slot 作用域插槽增强 — 实施计划（续）

## 摘要

本计划继续上一会话的工作，目标是完成 RML 框架的"作用域插槽"（Scoped Slots）增强功能，让 `<template slot="bottom" scope={panel}>` 模板能够通过 `panel.maximize(_window, cx)` 等调用操控父级 `resizable` 容器。

**核心问题**：TabWindow 的 `left`/`right`/`bottom` 插槽被包裹在 `resizable` 中，当插槽内容（如终端面板）需要"最大化/还原/关闭"功能时，需要获得类似 `SlotContext` 的参数来操控 `resizable`。

**当前状态**：核心实现已完成约 70%（trait、TabWindowSlotScope、shell.rs codegen、render.rs 宏改造均已完成），但存在以下关键问题：
1. **P0 阻塞 bug**：自定义组件的插槽闭包包装存在 `cx.listener` 调用失败问题（由 `SlotRenderer` 签名变更引发）
2. **P1 缺失**：`validator.rs` 未校验 `scope` 属性
3. **P1 缺失**：无端到端 demo
4. **P2 缺失**：`docs/06-components/slots.md` 仍将作用域插槽标记为"规划中"，使用旧的 `let-item` 语法

## 当前状态分析

### 已完成（来自上一会话）

| 模块 | 文件 | 状态 |
|------|------|------|
| 1. ISlotScope trait + NullSlotScope + SlotRenderer | [crates/core/src/slot.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/slot.rs) | ✅ 完整 |
| 2. ShellSlots + extract_scope + wrap_shell_slot(scope_var) | [crates/engine/src/compiler/codegen/shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) | ✅ 完整 |
| 3. gen_slot_code! 宏（解构 (n, scope_var)，传 scope_var 作 loop_vars） | [crates/engine/src/compiler/codegen/render.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/render.rs) | ✅ 完整 |
| 4. `<slot>` 占位符注入 NullSlotScope | [crates/engine/src/compiler/codegen/node.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) | ✅ 完整 |
| 5. TabWindowSlotScope（maximize/restore/close + use_keyed_state） | [crates/ui/src/window/tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) | ✅ 完整 |
| 6. ModernWindowShell 保持 impl IntoElement setters | [crates/ui/src/window/modern_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs) | ✅ 决策落定 |

引擎层编译通过：`cargo check -p rust-rml-engine` ✅；引擎测试 913/913 通过 ✅。

### 待修复 / 待完成

#### 问题 1（P0 阻塞 bug）：自定义组件插槽 `cx.listener` 调用失败

**根因**：[user_component.rs:105](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs#L105) 的插槽闭包包装使用 `let __rml_self_ref = __rml_self_entity.read(cx);` 然后直接执行 `({slot_code}).into_any_element()`，闭包参数 `cx: &mut gpui::App`。

但当 `slot_code` 内含 `cx.listener(...)` 调用时（按钮的 `on_click` 事件处理器生成此调用），由于 `gpui::App` 上不存在 `listener` 方法（仅 `gpui::Context<T>` 有），导致编译失败。

**影响**：demo 的 button_case.rml 编译失败，无法端到端验证 slot 增强。

**注意**：这不是 shell slot（tab-window/modern-window）的问题 — `wrap_shell_slot` 已经使用 `__rml_entity.update(_app, |this, cx| {...})` 模式，`cx` 在 update 内是 `&mut Context<Self>`，`cx.listener` 可用。问题仅存在于 `user_component.rs` 中的自定义组件插槽路径。

#### 问题 2（P1）：validator.rs 未校验 scope 属性

[validator.rs:109-135](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs#L109-L135) 仅校验 shell slot 名白名单（`menu`/`title`/`footer`/`left`/`right`/`bottom`/`tabs`），未对 `scope` 属性做任何校验。

缺失校验：
- `scope` 应该只出现在 `<template slot="...">` 上（普通元素不应使用）
- `scope` 表达式应为简单标识符（不能是 `foo.bar` / `foo(1)` 等复杂表达式）
- 在 `menu`/`title`/`footer` 等无 resizable 的 shell slot 上写 `scope` 应给出警告（无作用，但允许编译）

#### 问题 3（P1）：缺少端到端 demo

需要新建 demo 案例 `slot_scope_case.{rml,rml.rs}`，演示 `TabWindow` + `scope={panel}` + 最大化/还原/关闭按钮的完整流程。

#### 问题 4（P2）：文档未更新

[docs/06-components/slots.md:218-240](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md#L218-L240) 仍以旧的 `let-item` 语法描述"规划中"的作用域插槽，与实际实现的 `scope={name}` 语法 + `ISlotScope` trait API 不一致。

## 提议变更

### Step 1（P0 阻塞）：修复 user_component.rs 插槽闭包包装

**文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

**当前代码**（L104-L107，具名插槽；L119-L121，默认插槽）：

```rust
"    let {}: rml_core::slot::SlotRenderer = Box::new({{ let __rml_self_entity = __rml_self_entity.clone(); move |_scope: &dyn rml_core::slot::ISlotScope, _window: &mut gpui::Window, cx: &mut gpui::App| -> gpui::AnyElement {{ let __rml_self_ref = __rml_self_entity.read(cx); ({}).into_any_element() }} }});\n",
binding, slot_code
```

**修改后**：仿照 `wrap_shell_slot` 的 `entity.update(_app, |this, cx| {...})` 模式，让 `cx` 在 update 内成为 `&mut Context<Self>`，使 `cx.listener(...)` 可用。

```rust
"    let {}: rml_core::slot::SlotRenderer = Box::new({{ let __rml_self_entity = __rml_self_entity.clone(); move |_scope: &dyn rml_core::slot::ISlotScope, _window: &mut gpui::Window, _app: &mut gpui::App| -> gpui::AnyElement {{ __rml_self_entity.update(_app, |this, cx| {{ let __rml_self_ref: &Self = this; ({}).into_any_element() }}) }} }});\n",
binding, slot_code
```

**关键变更点**：
1. 闭包参数 `cx: &mut gpui::App` → `_app: &mut gpui::App`（重命名避免与 update 内的 cx 冲突）
2. 新增 `__rml_self_entity.update(_app, |this, cx| { ... })` 包装层
3. 在 update 内 `let __rml_self_ref: &Self = this;` 保留原 alias 机制（slot_code 内 `self.xxx` 已被替换为 `__rml_self_ref.xxx`）
4. `cx` 在 update 内为 `&mut Context<Self>`，`cx.listener(...)` 可用

**默认插槽**（L119-L121）做相同改造。

**不改动**：
- `with_self_alias("__rml_self_ref", ...)` 调用机制保持不变
- `__rml_self_ref` alias 名保持不变（避免影响其他生成代码）
- shell slot 路径（`wrap_shell_slot`）已正确，无需修改

**验证**：
```bash
cargo check -p rust-rml-engine
cargo check -p rust-rml-demo  # button_case.rs 编译应通过
cargo test -p rust-rml-engine --lib
```

### Step 2（P1）：validator.rs 增加 scope 属性校验

**文件**：[crates/engine/src/compiler/validator.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs)

在现有 shell slot 名白名单校验（L109-L135）后追加：

```rust
// 校验 scope 属性：仅可在 <template slot="..."> 上使用，且必须为简单标识符
for child in &elem.children {
    if let Node::Element(child_elem) = child {
        if child_elem.tag == "template" && child_elem.slot_name.is_some() {
            if let Some(Attribute::Bind { name, expr, span }) =
                child_elem.attributes.iter().find_map(|a| match a {
                    Attribute::Bind { name, expr, span } if name == "scope" => Some((name, expr, span)),
                    _ => None,
                })
            {
                // 1. scope 表达式必须是简单标识符
                let trimmed = expr.trim();
                let is_simple_ident = !trimmed.is_empty()
                    && trimmed.chars().all(|c| c.is_alphanumeric() || c == '_')
                    && trimmed.chars().next().map_or(false, |c| c.is_alphabetic() || c == '_');
                if !is_simple_ident {
                    return Err(ValidationError {
                        message: format!(
                            "scope 属性必须是简单标识符，得到 `{}`（示例：scope={{panel}}）",
                            expr
                        ),
                    });
                }

                // 2. 在无 resizable 的 shell slot 上使用 scope → 警告（不阻塞编译）
                if let Some(slot_name) = &child_elem.slot_name {
                    if matches!(slot_name.as_str(), "menu" | "title" | "footer" | "tabs") {
                        eprintln!(
                            "[rml warning] <template slot=\"{}\"> 不支持 resizable 操控，scope 变量将仅暴露插槽名",
                            slot_name
                        );
                    }
                }
            }
        } else if child_elem.tag == "template" {
            // 3. scope 不能出现在无 slot 属性的 <template> 上
            let has_scope = child_elem.attributes.iter().any(|a| match a {
                Attribute::Bind { name, .. } => name == "scope",
                _ => false,
            });
            if has_scope {
                return Err(ValidationError {
                    message: "scope 属性仅可出现在 <template slot=\"...\"> 上".to_string(),
                });
            }
        }
    }
}
```

**不校验**：
- 普通 HTML 元素上的 `scope` 属性（如 `<div scope={x}>`）— 因为 `scope` 在 props_registry 未登记，会被现有 `validate_unknown_props` 拦截，无需重复
- 自定义组件的 `<template slot="...">` 内的 scope — 自定义组件暂不支持 scope（首参 `_scope` 被忽略），后续扩展时再补校验

**验证**：
```bash
cargo test -p rust-rml-engine --lib scope  # 若新增单测
cargo check -p rust-rml-demo  # 现有 demo 不应触发新校验
```

### Step 3（P1）：创建 slot_scope_case demo

**新文件**：
- `demo/src/cases/slot_scope_case.rml`
- `demo/src/cases/slot_scope_case.rml.rs`

**修改文件**：
- `demo/src/cases/mod.rs` — 追加 `#[path = "slot_scope_case.rml.rs"] pub mod slot_scope_case;`

**demo 设计**：在 demo 主体窗口（`main_window.rml`）中追加 `<template slot="bottom" scope={panel}>` 插槽，展示终端面板式的最大化/还原/关闭按钮组。这比新建一个嵌套 tab-window demo 更真实，因为：
1. main_window 本身就是 tab-window，正好演示真实场景
2. 嵌套 tab-window 在 case 内容中渲染会很奇怪
3. 演示效果直接可见，无需另开窗口

**main_window.rml 修改**：在 `<template slot="footer">` 之前追加：

```rml
<template slot="bottom" scope={panel}>
    <div display="flex" flex-direction="column" height="100%">
        <div display="flex" flex-direction="row" align-items="center" justify-content="space-between"
             padding="6px 12px" border-bottom="1px solid var(--border-color)" bg="var(--panel-bg)">
            <span font-size="13px" font-weight="600">终端面板</span>
            <div display="flex" flex-direction="row" gap="4px">
                <Button ghost on-click={self.on_panel_maximize(panel, _window, cx)}>
                    <Icon name="Maximize" small />
                </Button>
                <Button ghost on-click={self.on_panel_restore(panel, _window, cx)}>
                    <Icon name="Restore" small />
                </Button>
                <Button ghost on-click={self.on_panel_close(panel, _window, cx)}>
                    <Icon name="Close" small />
                </Button>
            </div>
        </div>
        <div flex="1" padding="8px" overflow="auto" font-family="monospace">
            <span font-size="12px" text-muted>$ demo terminal — type commands here</span>
        </div>
    </div>
</template>
```

**main_window.rml.rs 修改**：在 `impl MainWindow` 追加三个事件处理方法：

```rust
pub fn on_panel_maximize(
    &mut self,
    panel: &dyn rml_core::slot::ISlotScope,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<Self>,
) {
    panel.maximize(_window, _cx);
}

pub fn on_panel_restore(
    &mut self,
    panel: &dyn rml_core::slot::ISlotScope,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<Self>,
) {
    panel.restore(_window, _cx);
}

pub fn on_panel_close(
    &mut self,
    panel: &dyn rml_core::slot::ISlotScope,
    _window: &mut gpui::Window,
    _cx: &mut gpui::Context<Self>,
) {
    panel.close(_window, _cx);
}
```

**slot_scope_case.{rml,rml.rs}**：纯文档型 case，仅展示 API 表格 + 代码示例（与 `template_slot_case` 同风格），用于在案例列表中索引"作用域插槽"概念，避免重复实现嵌套 tab-window。

**关键设计决策**：
- `panel` 是 `&dyn ISlotScope`（不是 owned），事件处理器签名需用 `&dyn rml_core::slot::ISlotScope`
- `panel.maximize(_window, _cx)` 直接代理到 ISlotScope trait method
- 按钮 variant 用 `ghost` 保持终端面板风格
- 不在 case_view 中渲染嵌套 tab-window，避免复杂的状态嵌套

**验证**：
```bash
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo  # 手动验证：点击最大化/还原/关闭按钮，bottom 面板尺寸应变化
```

### Step 4（P2）：更新 docs/06-components/slots.md

**文件**：[docs/06-components/slots.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md)

**修改点**：
1. 删除 L218-L240 "规划中特性" 节（旧的 `let-item` 语法描述）
2. 在 L213 "已知限制" 之前追加新章节 "6.3.6 作用域插槽（Scoped Slots）"

**新章节内容大纲**：

```markdown
## 6.3.6 作用域插槽（Scoped Slots）

### 概念

作用域插槽让 `<template slot="...">` 内容能够接收来自插槽宿主（slot host）的上下文参数，
用于操控父容器（如 resizable）行为。普通插槽仅能渲染内容，作用域插槽还能"反向操控"宿主。

### 语法

`<template slot="bottom" scope={panel}>...</template>`

- `scope={name}` 中 `name` 为接收 `&dyn ISlotScope` 的变量名
- `name` 必须为简单标识符（不能是 `foo.bar` / `foo(1)`）
- 不写 `scope={...}` 时，插槽首参以 `_scope` 忽略，向后兼容

### ISlotScope API

| 方法 | 返回 | 说明 |
|------|------|------|
| `slot_name()` | `&str` | 插槽名（"left"/"right"/"bottom"/...） |
| `current_size()` | `Option<Pixels>` | 当前尺寸（left/right 为宽度，bottom 为高度） |
| `container_size()` | `Option<Pixels>` | 容器总尺寸（用于 maximize 计算） |
| `has_resizable()` | `bool` | 是否支持 resizable 操控 |
| `maximize(window, cx)` | `()` | 最大化此面板（记录原尺寸供 restore 还原） |
| `restore(window, cx)` | `()` | 还原到 maximize 之前的尺寸 |
| `close(window, cx)` | `()` | 关闭/折叠此面板（尺寸调为 0 或最小阈值） |

### 实现方

- `NullSlotScope`：默认空作用域，所有方法返回 `None` / no-op
- `TabWindowSlotScope`：TabWindow 的 left/right/bottom 插槽，暴露 resizable 操控

### 使用示例

```html
<tab-window title="..." left-size={left_size}>
    <template slot="bottom" scope={panel}>
        <div>
            <Button ghost on-click={self.on_panel_maximize(panel, _window, cx)}>最大化</Button>
            <Button ghost on-click={self.on_panel_restore(panel, _window, cx)}>还原</Button>
            <Button ghost on-click={self.on_panel_close(panel, _window, cx)}>关闭</Button>
        </div>
    </template>
</tab-window>
```

### 限制

- `menu`/`title`/`footer`/`tabs` 等 shell slot 不支持 resizable 操控（has_resizable 返回 false）
- 自定义组件的 `<slot>` 默认传 `NullSlotScope`，不暴露父容器操控权
- `scope` 仅可在 `<template slot="...">` 上使用，普通元素无效
```

**同时更新** "6.3.7 小结" 节，将作用域插槽从"规划中"移到"已支持"。

**验证**：人工 review 文档与代码一致性。

### Step 5（P0）：编译与测试验证

执行以下命令，确认所有变更无回归：

```bash
# 引擎层
cargo check -p rust-rml-engine
cargo test -p rust-rml-engine --lib

# UI 层
cargo check -p rust-rml-ui

# 宏层
cargo check -p rust-rml-macros

# Demo（关键：button_case 的 cx.listener 错误应消失）
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo
```

**手动验证清单**：
- [ ] demo 启动后 TabWindow 底部出现"终端面板"
- [ ] 点击"最大化"按钮，bottom 面板高度撑满
- [ ] 点击"还原"按钮，bottom 面板恢复原高度
- [ ] 点击"关闭"按钮，bottom 面板折叠
- [ ] 案例列表中"作用域插槽"案例可访问，显示 API 表格
- [ ] 无控制台错误或警告

## 假设与决策

### 假设

1. **TabWindow 已具备 `slot_bottom` 字段与 `bottom_size` 状态**：经核查 [tab_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) 已有 `slot_bottom: Option<SlotRenderer>` 与 `bottom` 渲染分支。
2. **main_window.rml.rs 的 `MainWindow` 已实现 `IWindow`/`ILifecycle` 等基础 trait**：经核查 L92-L116 已实现。
3. **`Icon` 组件支持 `Maximize`/`Restore`/`Close` 图标名**：需在实施时确认；若不支持可改用文字按钮。
4. **`Button` 的 `on-click` 支持 `&dyn ISlotScope` 参数传递**：需验证 codegen 能正确处理 `panel` 作为闭包参数（已通过 `loop_vars` 传递机制）。

### 决策

1. **修复方式选择"包装 update"而非"重命名 cx"**：
   - 包装 `entity.update(_app, |this, cx| {...})` 让 `cx` 在内部为 `&mut Context<Self>`，是最小侵入修复
   - 替代方案是改 `SlotRenderer` 签名让闭包接收 `&mut Context<T>`，但这破坏 `Send + Sync`（`Context<T>` 非 Send）
   - update 包装与 `wrap_shell_slot` 一致，保持架构统一

2. **demo 修改 main_window 而非新建嵌套 tab-window**：
   - main_window 本身就是 tab-window，是真实场景
   - 嵌套 tab-window 在 case 内容中渲染会很奇怪
   - 演示效果直接可见，无需另开窗口

3. **slot_scope_case 仅做文档展示**：
   - 真实演示放在 main_window 的 bottom slot
   - case 文件本身仅展示 API 表格 + 代码示例（与 template_slot_case 同风格）

4. **scope 校验只警告不阻塞**：
   - 在 `menu`/`title`/`footer`/`tabs` 等无 resizable slot 上写 scope 仅警告（slot 仍能编译，scope 变量仅暴露 slot_name）
   - 避免过度限制，保持灵活性

## 实施顺序

按优先级与依赖关系：

1. **Step 1（P0）**：修复 user_component.rs — 解除 demo 编译阻塞
2. **Step 5（P0 部分）**：`cargo check -p rust-rml-demo` 验证 button_case 编译通过
3. **Step 2（P1）**：添加 validator.rs 校验
4. **Step 3（P1）**：创建 demo slot_scope_case + 修改 main_window.rml
5. **Step 4（P2）**：更新文档
6. **Step 5（P0 收尾）**：完整测试 + 手动验证

## 风险与回滚

- **风险**：修改 user_component.rs 闭包包装可能影响其他自定义组件 demo
  - 缓解：修改后运行 `cargo test -p rust-rml-engine --lib` 确认 913 个测试全通过
  - 回滚：单文件 git checkout 即可

- **风险**：main_window.rml 增加 bottom slot 可能与现有 footer slot 冲突
  - 缓解：经核查 main_window.rml 当前无 bottom slot，footer 是 status bar
  - 回滚：git checkout main_window.rml + main_window.rml.rs 即可
