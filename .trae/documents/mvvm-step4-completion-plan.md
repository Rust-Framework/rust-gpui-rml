# MVVM 完善 — Step 4 收尾计划

## 摘要

前序计划（`mvvm-completion-remaining.md`）的 Steps 1-3 已完成：
- ✅ Step 1（B-2 收尾）：`observable.rs:155` 3-arg → 4-arg 调用修复 + converter 测试
- ✅ Step 3a（stop_propagation 注入）：`event.rs` 4 个 `apply_event` 分支 + 4 个测试
- ✅ Step 2（B-1 command binding）：`menu/item.rs` `gen_command_closure` + `command_bind_expr` + 3 个测试
- ✅ Step 3b（oninput/onchange 注入）：`InputHandlers` 结构 + `collect_model_input_handlers` + `gen_input_handler_call` + 2 个测试

新增 56 个测试全部通过（29 event + 7 menu + 20 two-way binding）。所有 `engine`/`core`/`ui`/`macros`/`app` crate 构建成功。

**唯一阻塞**：`cargo build --workspace` 因 demo 二进制的**预先存在**（非 MVVM 引入）的构建错误而失败，位于 `crates/engine/src/compiler/codegen/shell.rs` 的 `tabs`/`tab_item_template` codegen。

本计划完成 Step 4 剩余工作：修复 demo 构建错误 → 全量验证 → demo 接入 B-1/B-2/B-3 示例 → 轻量文档更新。

---

## 当前状态分析

### Demo 构建错误根因（2 处）

**错误 1（E0615）：`tabs={tab_bar_items}` 生成方法引用而非方法调用**

```
error[E0615]: attempted to take value of method `tab_bar_items` on type `&mut MainWindow`
  --> .../rml_generated/main_window.rs:50:132
50 | ...tabs(self.tab_bar_items).tab_item_template({...})
```

- **位置**：[shell.rs:290](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L290)
- **根因**：`shell_bind_expr`（L387-405）仅对 `computed_methods` 列表中的标识符生成 `self.{name}()` 调用；对其他简单标识符走 `expr::to_rust_code_with_ctx` 生成 `self.{name}` 字段访问。
- demo 的 `tab_bar_items` 是普通方法（[main_window.rml.rs:207](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L207)），返回 `Vec<Box<dyn Any>>`。它**不是** `#[computed]`（因为 `Vec<Box<dyn Any>>` 不 `Clone`，无法走 `ComputedCache`），也**不是** `observable_fields` 中的字段。
- 因此 `shell_bind_expr` 错误地生成 `self.tab_bar_items`（字段访问）而非 `self.tab_bar_items()`（方法调用）。

**错误 2（E0277）：`tab_item_template` 闭包被双重 `Arc::new` 包裹**

```
error[E0277]: expected a `Fn(usize, &Box<dyn Any>, &mut Window, &mut App)` closure,
              found `Arc<{closure@main_window.rs:52:41}>`
```

- **位置**：[shell.rs:292-306](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L292-306)
- **根因**：codegen 在 L297 用 `std::sync::Arc::new(move |...| {...})` 包裹闭包，但 `TabWindowShell::tab_item_template<F>` setter（[tab_window.rs:216-222](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L216-222)）签名是：
  ```rust
  pub fn tab_item_template<F>(mut self, template: F) -> Self
  where
      F: Fn(usize, &Box<dyn Any>, &mut Window, &mut App) -> TabItem + Send + Sync + 'static,
  {
      self.tab_item_template = Some(Arc::new(template));  // setter 内部已 Arc::new
      self
  }
  ```
  setter 内部已做 `Arc::new(template)`，codegen 再包一次变成 `Arc<Arc<closure>>`，而 `Arc<T>` 不自动实现 `Fn` trait，故类型不匹配。

### 关键 API 约定（已核实）

- **`TabWindowShell::tabs` setter**：`pub fn tabs(mut self, tabs: Vec<Box<dyn Any>>) -> Self`（[tab_window.rs:209](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L209)）— 接受 `Vec<Box<dyn Any>>`，几乎总是由方法构造
- **`TabWindowShell::tab_item_template` setter**：泛型 `F: Fn(...) + Send + Sync + 'static`，内部 `Arc::new` 转 `Arc<dyn Fn>`
- **`shell_bind_expr` 现状**：仅在 `computed_methods` 命中时生成 `()` 调用；其余简单标识符一律按字段访问
- **`ctx.observable_fields`**：`CodegenCtx` 字段，列出所有 pub 可观察字段（[compiler/mod.rs:108](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L108) 附近）
- **`Entity::update` for `App`**：返回闭包值（`App::Result<R> = R`），故 `entity.update(app, |this, cx| this.render_tab_item(...))` 直接返回 `TabItem`

### Demo binds 盘点（[main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)）

| Bind 表达式 | demo 中的性质 | 期望生成 | 当前生成 | 状态 |
|---|---|---|---|---|
| `icon={IconName::Frame}` | 字面量 | `rml_ui::IconName::Frame` | 同左（Err 分支回退） | ✅ |
| `tabs={tab_bar_items}` | 方法 | `self.tab_bar_items()` | `self.tab_bar_items` | ❌ E0615 |
| `tab_item_template={render_tab_item}` | 方法 | 裸闭包 | `Arc::new(闭包)` | ❌ E0277 |
| `selected_index={selected_tab}` | 字段 | `self.selected_tab` | 同左 | ✅ |
| `show_chrome={show_chrome}` | 字段 | `self.show_chrome` | 同左 | ✅ |
| `left_size={slot_left_size}` | 字段 | `self.slot_left_size` | 同左 | ✅ |

---

## 提议变更

### Step A：修复 demo 构建错误（阻塞优先）

#### A-1：扩展 `shell_bind_expr` 区分字段与方法（修复 E0615）

**文件**：[crates/engine/src/compiler/codegen/shell.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs)

**改动 1**：`shell_bind_expr` 签名增加 `observable_fields: &[&str]` 参数，并修改判定逻辑：

```rust
fn shell_bind_expr(
    expr: &str,
    computed: &[&str],
    observable_fields: &[&str],  // 新增
    loop_vars: &[&str],
) -> String {
    let trimmed = expr.trim();
    if computed.iter().any(|c| *c == trimmed) {
        return format!("self.{}()", trimmed);
    }
    match expr::parse(expr) {
        Ok(expr::Expr::Field(field_name))
            if computed.iter().any(|c| *c == field_name.as_str()) =>
        {
            format!("self.{}()", field_name)
        }
        Ok(expr::Expr::Field(field_name)) => {
            // 在 observable_fields 中 → 字段访问；否则按方法调用
            if observable_fields.iter().any(|f| *f == field_name.as_str()) {
                format!("self.{}", field_name)
            } else {
                format!("self.{}()", field_name)
            }
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            if computed.iter().any(|c| *c == trimmed) {
                format!("self.{}()", trimmed)
            } else if observable_fields.iter().any(|f| *f == trimmed) {
                format!("self.{}", trimmed)
            } else {
                format!("self.{}()", trimmed)
            }
        }
    }
}
```

**改动 2**：`gen_tab_window_wrapper` 调用处（L286）传入 `observable_fields`：

```rust
let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
let observable: Vec<&str> = ctx.observable_fields.iter().map(|s| s.as_str()).collect();  // 新增
let empty: Vec<&str> = Vec::new();
...
let rust_expr = shell_bind_expr(expr, &computed, &observable, &empty);  // 改
```

**为什么选这个方案而非"特例化 `tabs`"**：与 `tab_item_template` 已有的"表达式即方法名"特例不同，`tabs`/`menu`/`footer`/`selected_index` 等 bind 既可能接字段也可能接方法。引入 `observable_fields` 判定后，框架能自动选择 `self.x` vs `self.x()`，符合"框架自动化"设计哲学（见 project_memory：*"framework-level logic automation"*）。这也顺带修掉了 `gen_modern_window_wrapper` 中 `menu`/`footer` bind 的同类潜在 bug（虽然 demo 未触发）。

**为什么不直接给 `tab_bar_items` 加 `#[computed]`**：`#[computed]` 走 `ComputedCache::get_or_compute::<T>`，要求返回类型 `T: Clone`（见 [computed_cache.rs:74](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs#L74)）。`Vec<Box<dyn Any>>` 不 `Clone`，会编译失败。

#### A-2：移除 `tab_item_template` codegen 中的 `Arc::new` 包裹（修复 E0277）

**文件**：[crates/engine/src/compiler/codegen/shell.rs:292-306](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L292-306)

**改动**：删除 `std::sync::Arc::new(...)` 外层包裹，让 setter 自己做 `Arc::new`：

```rust
"tab_item_template" => {
    let method = expr.trim();
    code.push_str(&format!(
        ".tab_item_template({{\n                    \
         let weak = cx.weak_entity();\n                    \
         move |ix: usize, data: &Box<dyn std::any::Any>, \
         window: &mut gpui::Window, app: &mut gpui::App| {{\n            \
         if let Some(entity) = weak.upgrade() {{\n                \
         entity.update(app, |this, cx| this.{}(ix, data, window, cx))\n        \
         }} else {{\n            \
         rml_ui::TabItem::new()\n        \
         }}\n    }})\n}})",
        method
    ));
}
```

唯一删除的是 `std::sync::Arc::new(` 与对应的 `)`。闭包体本身保持不变。

#### A-3：新增 codegen 单元测试

**文件**：[crates/engine/src/compiler/codegen/shell.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs)（tests 模块，L407 起）

新增 3 个测试：
1. `shell_bind_tabs_method_generates_method_call`：`tabs={tab_bar_items}` 且 `tab_bar_items` 不在 `observable_fields` 时，生成 `self.tab_bar_items()`
2. `shell_bind_selected_index_field_generates_field_access`：`selected_index={selected_tab}` 且 `selected_tab` 在 `observable_fields` 时，生成 `self.selected_tab`（无 `()`）
3. `tab_item_template_generates_bare_closure_without_arc`：生成代码不含 `std::sync::Arc::new`，含 `move |ix`

### Step B：全量构建 + 测试验证

**命令**（在 `e:\GitCode\RF\rust-gpui-rml` 下）：

```bash
cargo build --workspace     # 期望：0 错误
cargo test --workspace      # 期望：553 旧 + 56 新 + 3 (A-3) ≈ 612 通过，0 失败，27 ignored
```

若仍有 demo 构建错误，定位并修复（不预期发生）。

### Step C：Demo 接入 B-1/B-2/B-3 使用示例

复用现有 case 文件，避免新建 case（降低接入成本）。

#### C-1：B-1 command binding（接入 `menu_features_case`）

**文件**：
- [demo/src/cases/menu_features_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml.rs)
- [demo/src/cases/menu_features_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml)

**改动**：
1. `MenuFeaturesCase` 结构体增加 `save_command: Arc<dyn ICommand>` 字段
2. `ILifecycle::on_loaded` 中初始化：`self.save_command = RelayCommand::new(...)`（具体 RelayCommand 构造方式执行时核实 [crates/core/src/command.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs)）
3. RML 中新增一个菜单项：`<menu-item label="Save (Command)" command={save_command} />`
4. 该菜单项点击时由 codegen 生成的 clone-Arc-out 闭包调用 `save_command.execute(...)`
5. command 的 execute 回调设置 `self.last_action = "Save command executed"`（与现有 `on_available` 等模式一致）

**验证**：执行 Step B 的 `cargo build --workspace` 时该 case 应编译通过；codegen 应生成 `gen_command_closure` 模式的 on_click 闭包（已在 Step 2 测试覆盖）。

#### C-2：B-2 converter（接入 `two_way_case`）

**文件**：
- [demo/src/cases/two_way_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml.rs)
- [demo/src/cases/two_way_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml)

**改动**：
1. `TwoWayCase` 增加 `pub price: i32` 字段（带 `#[validate(range(min = 0, max = 100000))]`）
2. 在同文件或 `demo/src/converters.rs`（若不存在则新建并 `mod converters;`）定义 `Currency` converter：
   ```rust
   pub struct Currency;
   impl Currency {
       pub fn convert(value: i32) -> String { format!("¥{}", value) }
       pub fn convert_back(s: &str) -> Result<i32, ...> {
           s.trim_start_matches('¥').parse::<i32>().map_err(...)
       }
   }
   ```
   （具体 converter trait/签名执行时核实 [crates/engine/src/compiler/codegen/observable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs) 中 `gen_field_assign_expr` 对 converter 的调用形式）
3. RML 中新增：
   ```rml
   <input model={price | Currency} placeholder="输入金额" />
   <p>当前金额：{Currency::convert(price)}</p>
   ```

**验证**：codegen 应生成 `Currency::convert_back(...)` 调用（已在 Step 1 测试 `model_with_converter_generates_convert_back_call` 覆盖）。

#### C-3：B-3 oninput/onchange（接入 `two_way_case`）

**文件**：同 C-2

**改动**：
1. `TwoWayCase` 增加 `pub last_input_event: String` 和 `pub last_change_event: String` 字段
2. 实现两个 handler 方法：
   ```rust
   pub fn on_name_input(&mut self, ev: &InputEvent, cx: &mut Context<Self>) {
       self.last_input_event = format!("input: {}", ev.value());
       cx.notify();
   }
   pub fn on_name_change(&mut self, ev: &ChangeEvent, cx: &mut Context<Self>) {
       self.last_change_event = format!("change: {}", ev.value());
       cx.notify();
   }
   ```
   （`InputEvent`/`ChangeEvent` 的确切类型与 `value()` 方法签名执行时核实 [crates/engine/src/runtime/event_flow.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/runtime/event_flow.rs)）
3. RML 中为现有 `<input model={name}>` 增加 handler：
   ```rml
   <input model={name} oninput={on_name_input} onchange={on_name_change} placeholder={...} />
   ```
4. 在演示区增加一段说明：`<p>最近 input 事件：{last_input_event}</p>` 与 `<p>最近 change 事件：{last_change_event}</p>`

**验证**：codegen 应在 `cx.subscribe` 回调的 reverse_arms 中注入 `this.on_name_input(&__rml_input_ev, cx)` 与 `this.on_name_change(&__rml_change_ev, cx)` 调用（已在 Step 3b 测试 `oninput_handler_injected_into_subscribe_callback` / `onchange_handler_separate_from_oninput` 覆盖）。

### Step D：轻量文档更新

**范围**：仅更新与 B-1/B-2/B-3 直接相关的参考文档，不展开教程级重写。

**文件**：
- [docs/03-binding/two-way-binding.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/03-binding/two-way-binding.md)：补充 `model={field | Converter}` 语法与 `oninput`/`onchange` handler 注入说明
- [docs/04-code-behind/command-system.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/04-code-behind/command-system.md)：补充 `<menu-item command={field} />` 声明式绑定说明
- [docs/06-components/reference/menu-items.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/06-components/reference/menu-items.md)：属性表增加 `command` 行

每个文件增量 5-15 行，聚焦"语法 + 生成代码 + demo 位置"三点。

---

## 假设与决策

1. **不重构 `gen_modern_window_wrapper`**：其 `menu`/`footer` bind 在 demo 中走 slot 语法而非 bind，未触发同类 bug；Step A-1 的 `shell_bind_expr` 改进顺带覆盖其潜在风险，但本次不动其调用点（最小改动原则）。
2. **`tab_bar_items` 保持为普通方法**：不加 `#[computed]`（返回类型不 `Clone`），不改 demo 把它改成字段（会丢失"按需构造 `Vec<Box<dyn Any>>`"的语义）。框架侧通过 `observable_fields` 判定自动区分字段/方法。
3. **converter API 签名执行时核实**：`gen_field_assign_expr` 已在 Step 1 改为接受 `converter: Option<&str>` 并生成 `Currency::convert_back(...)` 形式。demo 的 `Currency` 实现需匹配该调用形式（静态方法 `convert_back(&str) -> Result<T, E>` 或类似）。
4. **`InputEvent`/`ChangeEvent` 类型执行时核实**：Step 3b 的 `gen_input_handler_call` 生成 `rml_convert::convert::input(value, SharedString::default())` 与 `rml_convert::convert::change(value)`，handler 签名为 `fn(&InputEvent, &mut Context<Self>)` / `fn(&ChangeEvent, &mut Context<Self>)`。确切路径与 `value()` 访问器执行时读 [crates/engine/src/runtime/event_flow.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/runtime/event_flow.rs) 确认。
5. **手动运行 demo 不在本计划验证范围**：本环境无法真正启动 GUI 验证交互；以 `cargo build`/`cargo test` 通过作为客观标准，并明确告知用户"UI 交互未手动验证"。
6. **RelayCommand 构造方式执行时核实**：C-1 中 `RelayCommand::new(...)` 的确切构造 API（是否需要 `IContribution` 注册、参数形式）读 [crates/core/src/command.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs) 确认。

---

## 验证步骤

1. **Step A 完成后**：
   - `cargo build -p rust-rml-engine` — engine crate 编译通过
   - `cargo test -p rust-rml-engine --lib shell` — shell codegen 测试通过（含 3 个新测试）
2. **Step B 完成后**：
   - `cargo build --workspace` — 0 错误（demo 二进制也通过）
   - `cargo test --workspace` — 全部通过，0 失败，27 ignored 不变
3. **Step C 完成后**：
   - `cargo build -p rust-rml-demo` — demo 编译通过
   - 抽查生成的 `target/debug/build/rust-rml-demo-*/out/rml_generated/menu_features_case.rs` 含 `command={save_command}` 生成的 clone-Arc-out 闭包
   - 抽查 `two_way_case.rs` 生成代码含 `Currency::convert_back` 与 `this.on_name_input`/`this.on_name_change` 注入
4. **Step D 完成后**：
   - 3 个文档文件已更新，无死链

**完成标准**：Step A + B + C + D 全部通过，`cargo build --workspace` 与 `cargo test --workspace` 绿。明确告知用户：UI 交互行为未在本环境手动验证，建议用户本地 `cargo run -p rust-rml-demo` 抽查 B-1/B-2/B-3 三个 demo case 的实际行为。

---

## 执行顺序

1. Step A-1 + A-2（修复 shell.rs，~15 行改动）
2. Step A-3（新增 3 个 shell 测试）
3. `cargo build -p rust-rml-engine && cargo test -p rust-rml-engine`（局部验证）
4. Step B（全量 build + test，确认 demo 构建解锁）
5. Step C-1 → C-2 → C-3（依次接入三个 demo 示例，每个接入后跑 `cargo build -p rust-rml-demo`）
6. Step D（文档更新）
7. 最终 `cargo build --workspace && cargo test --workspace` 收尾

---

## 执行结果（已完成）

| 步骤 | 状态 | 备注 |
|---|---|---|
| A-1 + A-2 | ✅ | `shell_bind_expr` 引入 `observable_fields` 判定；`tab_item_template` codegen 移除冗余 `Arc::new` |
| A-3 | ✅ | 新增 3 个 shell 测试全部通过 |
| B | ✅ | 全量 build + test 通过 |
| C-1 | ✅ | B-1 command binding demo：`MenuFeaturesCase` 用 `Arc<RelayCommand>` + `#[derive(Default)]`；框架新增 `impl Default for RelayCommand`（空对象模式） |
| C-2 | ✅ | B-2 converter demo：修复 `gen_field_value_expr` 正向同步未调用 `convert()` 的 gap；`TwoWayCase` 增 `price: f64` + `model={price \| Currency}` |
| C-3 | ✅ | B-3 oninput/onchange demo：修复 `rml_convert` 别名作用域 bug（仅 `render()` 内可见）→ 改用全限定路径 `rml::runtime::event_flow::convert::input/change`；`TwoWayCase` 增 input/change 事件计数器 |
| D | ✅ | 3 份文档更新：`menu-items.md`、`two-way-binding.md`、`command-system.md` |

### 最终验证

- `cargo build --workspace`：✅ 0 错误（28.95s）
- `cargo test --workspace`：✅ 616 passed, 0 failed, 27 ignored

### 计划外修复（执行中发现）

1. **`impl Default for RelayCommand`**（`crates/core/src/command.rs`）：原计划 C-1 写 `Arc<dyn ICommand>` 字段，但 `dyn` 无 `Default`，需手写 8+ 个宏注入的 `__rml_*` 字段初始化。改为 `Arc<RelayCommand>` + 框架级 `impl Default`（no-op 空对象），ViewModel 用 `#[derive(Default)]` 自动初始化，`on_loaded` 中替换为真实命令。
2. **`gen_field_value_expr` 正向 converter gap**（`crates/engine/src/compiler/codegen/binding.rs`）：原 codegen 正向同步（VM→UI）未调用 `convert()`，输入框会显示原始值而非格式化值。新增 `converter: Option<&str>` 参数，converter 存在时生成 `Converter.convert(&self.field).into()`。
3. **`rml_convert` 别名作用域 bug**（`crates/engine/src/compiler/codegen/observable.rs`）：`use rml::runtime::event_flow::convert as rml_convert;` 仅在 `render()` 方法内可见（render.rs:45），但 oninput/onchange handler 代码生成在 `__rml_get_or_init_input_state` 方法内。改用全限定路径 `rml::runtime::event_flow::convert::input(...)` / `::change(...)`。
4. **`RelayCommand: IValue` 阻塞**（`crates/core/src/value.rs`）：`cargo clean -p rust-rml-core` 后全量重编暴露先前 `IValue` 重构的潜在编译错误。调查确认 `value.rs:20` 已有 blanket impl `impl<T: Send + Sync + Any> IValue for T {}`，`RelayCommand: Send + Sync + Any` 自动满足。clean 后二次编译通过——首次错误为陈旧缓存产物。

### 未验证项

- **UI 交互行为未手动验证**：本环境无法启动 GUI。建议用户本地 `cargo run -p rust-rml-demo` 抽查三个 demo case 的实际行为：
  - B-1：`menu_features_case` 中 "Save (Command)" 菜单项点击后 `last_action` 应更新
  - B-2：`two_way_case` 中金额输入框应显示 `¥1500.00` 格式，反向输入应解析回 `f64`
  - B-3：`two_way_case` 中 name/age 输入框的 input/change 事件计数器应递增
