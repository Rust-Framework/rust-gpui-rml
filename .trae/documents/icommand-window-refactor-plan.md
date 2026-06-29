# ICommand 接口重构与 Window/ModernWindow 派生关系重构

## Context

用户指出当前 RML 框架的两个架构问题：

1. **ICommand 接口污染**：与 WPF `ICommand` 差距大——没有 `execute()` 方法，`rml_command_name`/`rml_event_type`/`rml_params` 三个编译器元信息方法污染接口，`rml_` 前缀冗余。
2. **Window/ModernWindow 职责不清**：`ModernWindowShell` 重复窗口参数定义（title 在 `<window>` 和 `<ModernWindowShell>` 上各写一次），两者平级而非 WPF 风格的基类/派生类关系。

经探索发现：
- `ICommand` 的三个元信息方法**从未被任何生产代码调用**（仅存在于 trait 定义和 mock 测试中），`#[command]` 宏是纯 pass-through（提取的元信息被丢弃绑定到 `_event_type`/`_params`），codegen 生成 onclick 绑定时直接调用 `this.on_click(ev, cx)` 绕过 trait，`can_execute` 也未被 codegen 调用（command.rs 第 45-48 行注释是虚假文档）。
- `open()` 有**三份重复实现**（codegen 模板 + builtin Window + builtin ModernWindow），逻辑都是 `init(cx) → window_options() → cx.open_window() → 存 handle`。
- builtin `ModernWindow::chrome()` 返回 `Native` 是** bug**：ModernWindowShell 自绘 TitleBar 需要 `Transparent`（`appears_transparent: true` + `WindowDecorations::Client`），`Native` 会导致 OS 原生标题栏覆盖自绘标题栏。

目标：对齐 WPF 设计，接口纯净，消除重复，建立派生关系。

---

## 重构方案

### Part 1: ICommand 接口重构

**设计理念**：对齐 WPF `ICommand`（Execute + CanExecute），接口纯净无编译器元信息。命令执行由 codegen 直接调用方法保留事件类型安全；`ICommand::execute` 作为统一执行入口（快捷键、命令面板等动态调度场景），由用户按需实现。

**新 trait 定义**（`crates/core/src/command.rs`）：
```rust
pub trait ICommand: 'static {
    /// 执行命令（WPF: Execute）
    /// parameter 类型擦除，实现方按需 downcast
    fn execute(&mut self, parameter: &dyn std::any::Any, cx: &mut Context<Self>);
    /// 是否可执行（WPF: CanExecute）
    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool { true }
}
```

**改动**：
- 删除 `rml_command_name`/`rml_event_type`/`rml_params` 三个方法
- 删除 `ParamMeta` 结构体（仅 command.rs 和 prelude.rs 引用，无生产使用）
- `can_execute` 签名从 `(&self) -> bool` 改为 `(&self, &dyn Any) -> bool`，与 execute 参数对齐
- 添加 `: 'static` supertrait（与 IModel 一致，确保 `Context<Self>` 可用）
- 同步移除 `crates/core/src/prelude.rs` 中的 `ParamMeta` 导出
- `#[command]` 宏保持 pass-through（只校验签名），不生成 `impl ICommand`，用户按需手写

### Part 2: IWindow 扩展窗口操作方法

**设计理念**：用户要求"IWindow 应该包含窗口操作常规方法，类似 WPF Window 提供的基本方法"。当前已有 close/show/hide/activate/state，补充 set_state/maximize/minimize/restore。

**IWindow trait 改动**（`crates/core/src/window.rs`）：
- supertrait 改为 `pub trait IWindow: IComponent + Default`（codegen 和 RmlApplication 已隐式要求 Default）
- 新增窗口状态操作方法（基于 GPUI `zoom_window` toggle 语义，GPUI 无独立 maximize/restore）：
  ```rust
  fn set_state(&mut self, state: WindowState, cx: &mut App) {
      if let Some(handle) = self.handle() {
          let _ = handle.update(cx, |_v, window, _cx| match state {
              WindowState::Minimized => window.minimize_window(),
              WindowState::Maximized => { if !window.is_maximized() { window.zoom_window(); } }
              WindowState::Normal => { if window.is_maximized() { window.zoom_window(); } }
          });
      }
  }
  fn maximize(&mut self, cx: &mut App) { self.set_state(WindowState::Maximized, cx); }
  fn minimize(&mut self, cx: &mut App) { self.set_state(WindowState::Minimized, cx); }
  fn restore(&mut self, cx: &mut App) { self.set_state(WindowState::Normal, cx); }
  ```
- `open()` 保持必需方法（无默认实现）——因为 rml_core 不依赖 rml_ui，无法在默认实现中调用 `rml_ui::init()` 和 `Root::new()`

**rml_ui 新增 IWindowExt trait**（`crates/ui/src/window/mod.rs` 或新文件 `actions.rs`）：
```rust
pub trait IWindowExt: IWindow + Default {
    /// 含 init + Root 包裹的完整 open 实现
    fn open_rooted(&mut self, cx: &mut App) {
        crate::init(cx);
        let options = self.window_options();
        let handle = cx.open_window(options, |window, cx| {
            let view = cx.new(|_| Self::default());
            cx.new(|cx| crate::Root::new(view, window, cx))
        }).expect("failed to open window");
        self.set_handle(handle.into());
    }
}
impl<W: IWindow + Default> IWindowExt for W {}
```
消除 open() 三份重复——codegen 和 builtin 都调用 `IWindowExt::open_rooted()`。

### Part 3: Window/ModernWindow 派生关系

**设计理念**：WPF 风格派生——ModernWindow 派生自 Window，定制高级窗口特性（TitleBar + Menu + StatusBar）。用 trait 默认方法复用，消除字段重复。

**builtin Window/ModernWindow 重构**（`crates/ui/src/window/builtin_window.rs`）：
- 两者都 impl Default（已有）
- 两者都用 IWindow 默认方法（close/show/hide/activate/state/set_state/maximize/minimize/restore）
- 两者 `open()` 改为调用 `IWindowExt::open_rooted(self, cx)`（消除重复）
- **Window**：基础窗口，`chrome()` 用默认 `Transparent`，render 返回 children
- **ModernWindow**：`chrome()` 改为 `Transparent`（修复 bug），额外提供 menu/status_bar 字段，render 用 ModernWindowShell

### Part 4: codegen 支持 `<modern_window>` 根元素

**设计理念**：`<modern_window>` 作为根元素，自动生成 ModernWindowShell 包裹，消除用户嵌套 `<ModernWindowShell>` 和 title 重复定义。

**codegen.rs 改动**（`crates/engine/src/compiler/codegen.rs`）：
- `gen_window_impl(elem, ctx, is_modern)`：
  - `is_modern=true` 时 `chrome()` 生成 `WindowChrome::Transparent`（修复 bug，当前是 Native）
  - 生成 `open()` 调用 `rml_ui::IWindowExt::open_rooted(self, cx)`（替代当前内联的 init/Root 逻辑）
  - title/width/height 从根元素属性提取（如当前）
- `gen_render_impl_from_children`：
  - `is_modern=true` 时生成 ModernWindowShell 包裹：
    ```rust
    rml_ui::ModernWindowShell::new()
        .title(self.title().into())           // 复用 IWindow::title()，不重复定义
        .menu(self.menu_items().clone())     // 从 menu={menu_items} 属性提取
        .status_bar(self.status_items().clone())
        .child(<children render code>)
    ```
  - title 从根元素属性提取一次，同时用于 `IWindow::title()` 和 `ModernWindowShell::title()`，消除重复定义
  - menu/status_bar 属性提取复用 component.rs 的 `component_bind_setter` 逻辑（已有 `tag == "ModernWindowShell"` 分支处理 computed 方法调用）

### Part 5: ModernWindowShell 降为内部实现

**改动**（`crates/ui/src/window/modern_window.rs` 和 `tags.rs`）：
- ModernWindowShell 保留 title/menu/status_bar setter（作为数据传入通道）
- 从 `crates/engine/src/tags.rs` 的 `component_lookup` 路由表移除 `ModernWindowShell` 条目（不再作为用户可用的 `<ModernWindowShell>` 标签）
- 修正 `tags.rs:144` 的错误文档注释：`<modern_window>` 实际使用 `WindowChrome::Transparent`（自绘标题栏），当前注释写的是 `Native` 是 bug 描述
- 同步删除 `tags.rs:179` 和 `tags.rs:258` 注释中 ModernWindowShell 的提及
- ModernWindowShell 改为 codegen 内部使用的组件（pub 保留，codegen 生成代码引用）
- 移除 `crates/engine/src/compiler/component.rs` 中 ModernWindowShell 专用 setter 分支（`menu`/`status_bar`/`title` 三个 `if tag == "ModernWindowShell"` 分支），这些 setter 改由 codegen 根元素处理路径生成

### Part 6: demo 简化

**`demo/src/main_window.rml`** 改为单层根元素：
```html
<modern_window title="MainWindow" width="800" height="450"
    menu={menu_items} status_bar={status_items}>
    <div class="container">
        <h1 ref="title">Hello, RML!</h1>
        <p class="count">点击次数：{count}</p>
        <Button ref="click_btn" label="点击我" primary="" onclick={on_click} />
    </div>
</modern_window>
```

**`demo/src/main_window.rml.rs`** 基本不变：`#[computed] menu_items()` 和 `#[computed] status_items()` 保留，`#[command] on_click()` 保留，`#[derive(Default)]` 保留。

---

## 实施步骤

1. **ICommand 重构**（`crates/core/src/command.rs` + `prelude.rs`）
   - 重定义 trait（execute + can_execute，类型擦除参数，`: 'static`）
   - 删除 ParamMeta 及三个元信息方法
   - 更新测试（mock 实现新接口）

2. **IWindow 扩展**（`crates/core/src/window.rs`）
   - supertrait 加 Default
   - 新增 set_state/maximize/minimize/restore 默认实现

3. **IWindowExt 新增**（`crates/ui/src/window/` 新文件或 actions.rs）
   - 定义 IWindowExt trait + open_rooted 默认实现
   - blanket impl `impl<W: IWindow + Default> IWindowExt for W`
   - 在 `crates/ui/src/window/mod.rs` 导出 IWindowExt

4. **builtin Window/ModernWindow 重构**（`crates/ui/src/window/builtin_window.rs`）
   - open() 改为调用 IWindowExt::open_rooted
   - ModernWindow chrome() 改为 Transparent（修复 bug）
   - ModernWindow 添加 menu/status_bar 字段和 builder（可选，主要用于 builtin 场景）

5. **codegen `<modern_window>` 支持**（`crates/engine/src/compiler/codegen.rs`）
   - gen_window_impl：is_modern 时 chrome=Transparent，open 调用 IWindowExt::open_rooted
   - gen_render_impl_from_children：is_modern 时生成 ModernWindowShell 包裹
   - 提取 menu/status_bar 根元素属性生成 setter 调用

6. **ModernWindowShell 降级**（`crates/engine/src/tags.rs`）
   - 移除 ModernWindowShell 路由条目
   - 移除 component.rs 中 ModernWindowShell 专用 setter 分支（改为 codegen 根元素处理）

7. **demo 简化**（`demo/src/main_window.rml`）
   - 改为 `<modern_window>` 单层根元素

8. **测试更新**
   - `crates/core/src/command.rs`：删除 `ParamMeta` 相关测试（`param_meta_construction`/`param_meta_clone`/`param_meta_debug_format`/`param_meta_slice_operations`）和 `rml_command_name`/`rml_event_type`/`rml_params` 断言；mock 实现（`AlwaysEnabled`/`AlwaysDisabled`）改为新接口（`execute` + `can_execute(&dyn Any)`）
   - `crates/engine/src/compiler/component.rs`：删除 ModernWindowShell 相关测试（`gen_component_modern_window_shell_minimal`、`bind_setter_modern_window_shell_menu` 等），因为路由表已移除该组件，gen_component 不再处理它
   - 若 codegen.rs 有 modern_window 生成测试，适配新的 ModernWindowShell 包裹结构（当前 codegen.rs 未见 modern_window 专用测试，主要靠 demo 端到端验证）

---

## 验证方法

1. `cargo build --workspace` — 编译通过
2. `cargo test --workspace` — 全部测试通过（当前 219 测试，重构后 ICommand 测试调整）
3. `cargo run -p rust-rml-demo` — GUI 启动验证：
   - 窗口标题栏正常显示（TitleBar 自绘，非 OS 原生）
   - 菜单点击下拉正常
   - StatusBar 贴底显示
   - 窗口控制按钮（min/max/restore/close）正常
   - title 只在 `<modern_window>` 上定义一次，TitleBar 显示正确
4. 确认无回归：菜单下拉、窗口控制按钮、StatusBar 贴底（前一会话修复的 3 个 bug）

---

## 关键文件

- `crates/core/src/command.rs` — ICommand trait 重定义
- `crates/core/src/window.rs` — IWindow 扩展方法 + Default bound
- `crates/core/src/prelude.rs` — 移除 ParamMeta 导出
- `crates/ui/src/window/` — IWindowExt + builtin Window/ModernWindow 重构
- `crates/engine/src/compiler/codegen.rs` — `<modern_window>` 根元素 codegen
- `crates/engine/src/tags.rs` — 移除 ModernWindowShell 路由
- `crates/engine/src/compiler/component.rs` — 移除 ModernWindowShell 专用 setter
- `demo/src/main_window.rml` — demo 简化
