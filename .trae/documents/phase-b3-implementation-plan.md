# Phase B-3 双向绑定 + 文档完善 执行计划

## 摘要

本计划承接上一会话的三阶段任务，按 **阶段 1 验证 → 阶段 2 双向绑定实现 → 阶段 3 文档完善** 顺序执行。

- **阶段 1（细粒度更新）**：代码修改已完成（`#[command]` 参数化、`__rml_changed_fields` 生成、性能文档重写），仅需运行 `cargo build --workspace` + `cargo test --workspace` 验证无回归。
- **阶段 2（双向绑定）**：重写 `gen_model_input` 支持多 Input + 类型转换，scanner 提取字段类型，宏注入 `__rml_input_states` HashMap。
- **阶段 3（文档）**：宏 API 参考文档、双向绑定指南更新、宏注释完善。

## 当前状态分析（Phase 1 探索结果）

### 阶段 1 已完成代码修改（待验证）

1. **`crates/macros/src/lib.rs` + `crates/macros/src/command.rs`**：`#[command]` 接受 `no_notify` 参数，条件注入 `cx.notify()`
2. **`crates/engine/src/compiler/codegen.rs`**：`gen_observable_impl` 新增 `__rml_changed_fields()` 方法
3. **`docs/10-advanced/performance.md`**：已重写为真实 GPUI 渲染模型

### 阶段 2 双向绑定现状（待实现）

**核心问题**：`gen_model_input`（`crates/engine/src/compiler/codegen.rs:493-534`）有三个硬编码：

1. **单一 `input_state` 字段**：`rml_ui::Input::new(&self.input_state)` —— 多个 `<input model={...}>` 共用同一 `Entity<InputState>`，导致状态冲突
2. **反向绑定假定 String 类型**：`this.<field> = state.value().to_string()` —— 不支持 `i32`/`f64`/`bool`
3. **未利用 `ITwoWayBinding`**：trait 已定义（`crates/core/src/two_way_binding.rs`）但 codegen 从未接入

**相关现状**：
- `crates/engine/src/tags.rs:245-256`：`Input`/`TextInput` 都映射到 `state_field: "input_state"`
- `crates/engine/src/compiler/component.rs:304-314`：`onchange` 事件已支持 `&rml_ui::InputState`
- `InputState` 来自外部 crate `gpui_component::input::InputState`，提供 `value() -> SharedString` 和 `set_value()` 方法
- `crates/engine/src/build/scanner.rs`：已提取 `observable_fields`/`computed_deps`/`computed_returns`，**未提取字段类型**
- `crates/engine/src/compiler/mod.rs:18-51`：`CodegenCtx` 有 4 个 observable 字段，**无 `field_types`**
- `crates/macros/src/component.rs:71-105`：`inject_tracking_fields` 注入 `__rml_<field>_version` + `__rml_computed_cache`，**未注入 InputState 存储**

**文档现状**：
- `docs/03-binding/two-way-binding.md`：描述了 `<input type="number">`、`type="checkbox"` 等，但 codegen 实际不处理 `type` 属性，文档与实现脱节
- 无 `docs/api/macros.md` 宏 API 参考文档
- `crates/macros/src/lib.rs` 中宏的文档注释已较完善（Phase B-2 Step 7-2 改进），但 `#[command]` 参数化后需更新

## 提议的变更

### 阶段 1：验证（Task #84）

**步骤 1.1**：运行 workspace 编译与测试
```powershell
cargo build --workspace
cargo test --workspace
```

**预期**：
- 编译通过（可能有 warning，但无 error）
- 219 个现有测试 + 7 个集成测试全部通过
- 验证 `#[command(no_notify)]` 行为：在 `demo/src/main_window.rml.rs` 临时添加 `#[command(no_notify)]` 方法验证编译

**通过标准**：`cargo build --workspace` 和 `cargo test --workspace` 均成功。

---

### 阶段 2：双向绑定实现

#### Step 2.1：scanner 提取字段类型（Task #85）

**文件**：`crates/engine/src/build/scanner.rs`

**变更**：
1. `StructMetadata` struct 添加 `field_types: HashMap<String, String>` 字段
   - 存储每个 pub 字段的类型字符串（如 `"i32"`、`"String"`、`"SharedString"`）
2. 在第一遍扫描 struct 时，对每个 pub 字段用 `quote!(#ty).to_string()` + `split_whitespace().collect()` 提取类型字符串（复用 `return_type_str` 的清理逻辑）
3. 添加单元测试：`scans_pub_field_types` 验证 `pub count: i32` → `field_types["count"] == "i32"`

**为什么**：codegen 需要根据字段类型生成转换代码（`i32` → `parse::<i32>()`，`String` → `to_string()`，`SharedString` → `.into()`）。

#### Step 2.2：CodegenCtx 添加 field_types（Task #85）

**文件**：`crates/engine/src/compiler/mod.rs`

**变更**：
1. `CodegenCtx` struct 添加 `pub field_types: HashMap<String, String>` 字段
2. `crates/engine/src/build/mod.rs:211-219` 的 `CodegenCtx { ... }` 构造添加 `field_types: struct_meta.field_types.clone()`

**为什么**：将 scanner 提取的字段类型传递到 codegen 层供 `gen_model_input` 使用。

#### Step 2.3：宏注入 `__rml_input_states`（Task #86）

**文件**：`crates/macros/src/component.rs`

**变更**：
1. `inject_tracking_fields` 函数末尾追加注入：
   ```rust
   let input_states_field: Field = parse_quote! {
       #[allow(dead_code)]
       __rml_input_states: std::collections::HashMap<String, gpui::Entity<rml_ui::InputState>>
   };
   named.named.push(input_states_field);
   ```
2. 更新函数文档注释说明新增字段

**为什么**：为每个 model 绑定的字段提供独立的 `Entity<InputState>`，惰性初始化避免用户手动管理。

**注**：`HashMap<String, Entity<InputState>>::default()` 是空 map，`#[derive(Default)]` 兼容。

#### Step 2.4：重写 `gen_model_input`（Task #87）

**文件**：`crates/engine/src/compiler/codegen.rs:493-534`

**变更**：完全重写 `gen_model_input` 函数，新签名增加 `ctx: &CodegenCtx`（不再用 `_ctx`）：

```rust
fn gen_model_input(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    field: String,
) -> Result<String, CodegenError> {
    // 1. 查询字段类型（默认 String）
    let field_ty = ctx.field_types.get(&field).cloned().unwrap_or_default();
    
    // 2. 生成正向绑定代码（VM → UI）
    let value_expr = gen_value_forward(&field, &field_ty);
    
    // 3. 生成反向绑定代码（UI → VM，含类型转换）
    let assign_expr = gen_value_reverse(&field, &field_ty);
    
    // 4. 构造 Input 组件，使用 __rml_get_or_init_input_state 惰性初始化
    let mut code = format!(
        "rml_ui::Input::new(self.__rml_get_or_init_input_state({}, cx))\n            {}",
        quote!(#field), value_expr
    );
    
    // 5. on_change 回调
    code.push_str(&format!(
        ".on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| {{\n                    \
         {};\n                    \
         this.__rml_bump_version({:?});\n                    \
         cx.notify();\n                }}))",
        assign_expr, field
    ));
    
    // 6. 静态属性（placeholder/disabled）保留原逻辑
    for attr in &elem.attributes {
        if let Attribute::Static { name, value } = attr {
            if name == "placeholder" {
                code.push_str(&format!(".placeholder({:?})", value));
            } else if name == "disabled" {
                let v = if value.eq_ignore_ascii_case("true") || value == "1" || value.is_empty() { "true" } else { "false" };
                code.push_str(&format!(".disabled({})", v));
            }
        }
    }
    
    Ok(code)
}

/// 生成正向绑定：VM → UI 的 .value(...) 调用
fn gen_value_forward(field: &str, ty: &str) -> String {
    match ty {
        "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "usize" | "isize" => {
            format!(".value(self.{}.to_string())", field)
        }
        _ => format!(".value(self.{}.clone())", field),  // String/SharedString/其他
    }
}

/// 生成反向绑定：UI → VM 的赋值表达式
fn gen_value_reverse(field: &str, ty: &str) -> String {
    match ty {
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" => {
            format!("this.{} = state.value().parse::<{}>().unwrap_or(0)", field, ty)
        }
        "f32" => format!("this.{} = state.value().parse::<f32>().unwrap_or(0.0)", field),
        "f64" => format!("this.{} = state.value().parse::<f64>().unwrap_or(0.0)", field),
        "bool" => format!("this.{} = !state.value().is_empty()", field),
        _ => format!("this.{} = state.value().into()", field),  // String/SharedString
    }
}
```

**关键设计**：
- **惰性初始化**：`__rml_get_or_init_input_state(field_name, cx)` 在 render 时按需创建/复用 `Entity<InputState>`，存入 `__rml_input_states` HashMap
- **类型转换**：根据 scanner 提取的字段类型生成对应的转换代码
- **bump_version + notify**：反向绑定后自动 `bump_version`（与 `#[command]` 一致）和 `cx.notify()`
- **`type` 属性暂不实现**：保持简单，文档明确说明本版本仅支持文本类输入

#### Step 2.5：生成 `__rml_get_or_init_input_state`（Task #87）

**文件**：`crates/engine/src/compiler/codegen.rs`，在 `gen_observable_impl` 之后新增 `gen_input_state_impl`

**生成的代码模式**：
```rust
impl MainWindow {
    fn __rml_get_or_init_input_state(
        &mut self,
        field: &'static str,
        cx: &mut gpui::Context<Self>,
    ) -> &gpui::Entity<rml_ui::InputState> {
        if !self.__rml_input_states.contains_key(field) {
            let entity = cx.new_entity(|_| rml_ui::InputState::default());
            self.__rml_input_states.insert(field.to_string(), entity);
        }
        self.__rml_input_states.get(field).unwrap()
    }
}
```

**注意**：生成的代码需在 `compile()` 输出中追加（与 `gen_observable_impl`/`gen_computed_wrappers` 同级输出）。

**调用点**：在 `crates/engine/src/compiler/codegen.rs` 的 `codegen` 主函数中，将 `gen_input_state_impl(ctx)` 的输出追加到最终字符串。

#### Step 2.6：Demo 双向绑定示例（Task #88）

**文件**：`demo/src/main_window.rml` + `demo/src/main_window.rml.rs`

**变更**：
1. `main_window.rml.rs` 添加 `name: String` 和 `age: i32` 字段：
   ```rust
   #[window]
   #[derive(Default)]
   pub struct MainWindow {
       pub count: i32,
       pub name: String,
       pub age: i32,
   }
   ```
2. `main_window.rml` 添加两个 Input：
   ```html
   <div class="form">
       <input model={name} placeholder="姓名" />
       <input model={age} placeholder="年龄" />
       <p>你好，{name}（{age}岁），点击 {count} 次</p>
   </div>
   ```
3. 可选：在 `src/styles.css` 添加 `.form { display: flex; flex-direction: column; gap: 8px; }` 样式

**验证点**：
- 两个 Input 独立工作（不共用 state）
- `name` 字段输入实时同步到下方 `<p>` 显示
- `age` 字段输入数字时正确转换为 i32，输入非数字时保持上一次有效值（`unwrap_or(0)` 兜底）

#### Step 2.7：阶段 2 验证（Task #89）

**步骤**：
```powershell
cargo build --workspace
cargo test --workspace
cargo run -p rust-rml-demo
```

**通过标准**：
- 编译通过
- 所有现有测试通过
- 添加 1-2 个 `gen_model_input` 的单元测试（验证生成的代码包含 `__rml_get_or_init_input_state` 调用、类型转换代码）
- Demo 启动后两个 Input 独立工作

---

### 阶段 3：文档完善

#### Step 3.1：宏 API 参考文档（Task #90）

**文件**：新建 `docs/api/macros.md`

**内容**：
- 完整的宏属性参考：`#[derive(IModel)]`、`#[component]`、`#[window]`、`#[command]`、`#[computed]`、`#[on_loaded]`、`#[on_unloaded]`、`#[element]`
- 每个宏的：用途、参数、示例、生成代码、限制
- `#[command]` 参数化后的完整参数说明（`no_notify`、`debounce` 预留）
- `#[computed]` 的依赖追踪机制说明

**结构**：
```markdown
# 宏 API 参考

## 派生宏
### #[derive(IModel)]
### #[element]

## 属性宏
### #[component]
### #[window]
### #[command]
  - 参数：no_notify / debounce（预留）
### #[computed]
### #[on_loaded] / #[on_unloaded]

## 生成代码
（每个宏展开后的代码示例）
```

#### Step 3.2：更新双向绑定指南（Task #92）

**文件**：`docs/03-binding/two-way-binding.md`

**变更**：
1. **重写 §3.3.3 适用标签与字段类型**：移除虚构的 `type="number"`/`type="checkbox"` 表格，改为实际支持的字段类型表格（String/SharedString/i32/u32/f64 等）
2. **新增 §3.3.4 多 Input 共存**：说明 `__rml_input_states` 自动管理，用户无需手动声明字段
3. **新增 §3.3.5 类型转换**：说明 `i32`/`f64` 等数字类型的自动转换与失败兜底
4. **修正 §3.3.6 与命令协作**：移除 `model` + `oninput` 同时使用会冲突的描述（codegen 实际只生成 `on_change`，不与 `oninput` 冲突）
5. **更新 §3.3.10 常见陷阱**：补充"未声明 `pub` 字段时编译失败"等真实陷阱

#### Step 3.3：完善宏注释（Task #91）

**文件**：`crates/macros/src/lib.rs`

**变更**：
- `#[command]` 的文档注释已较完善（Phase B-2 Step 7-2 已更新），确认 `no_notify` 参数说明完整
- `#[computed]` 注释添加依赖追踪机制说明（一行）
- 其他宏注释按需微调

#### Step 3.4：扩展 demo 示例（可选，Task #92 部分）

**文件**：`demo/src/main_window.rml` + `main_window.rml.rs`

**可选变更**：在 demo 中添加一个 `#[computed]` 示例（如 `summary` 计算属性展示姓名+年龄），让文档示例有真实运行参考。

## 假设与决策

### 决策 1：多 Input 支持方案 —— 选择 `HashMap` 惰性初始化

**选项**：
- A：注入 `__rml_input_states: HashMap<String, Entity<InputState>>`，按字段名索引惰性初始化（**选定**）
- B：每个 model 绑定生成独立的预定义字段（如 `__rml_input_state_<field>`）

**理由**：
- A 方案无需 scanner 预扫描 `.rml` 中的 model 指令
- A 方案对动态生成的 input（列表渲染）更友好（未来扩展）
- A 方案 HashMap 查找开销可忽略（render 时单次 lookup）

### 决策 2：类型转换策略 —— codegen 根据字段类型生成

**选项**：
- A：scanner 提取字段类型，codegen 生成对应转换代码（**选定**）
- B：要求所有 model 绑定字段都是 `String`/`SharedString`

**理由**：
- A 方案符合 WPF 风格（自动类型转换）
- A 方案让 i32/f64 等基础类型字段无需用户手动转换
- 失败兜底（`unwrap_or(0)`）保证输入非法字符时不 panic

### 决策 3：`type` 属性暂不实现

**理由**：
- 当前 `gpui-component` 的 `Input` 组件不区分 type
- 实现 `type="checkbox"` 需要 Switch/Checkbox 组件路由，超出本阶段范围
- 文档中明确说明本版本仅支持文本类输入，未来扩展

### 决策 4：保留 `ITwoWayBinding` trait

**理由**：
- 作为未来自定义组件双向绑定的扩展点
- 在 `docs/api/macros.md` 中说明此 trait 供高级用户实现自定义组件双向绑定

### 决策 5：`__rml_get_or_init_input_state` 需要 `&mut self`

**注意点**：
- `render(&mut self, ...)` 提供 `&mut self`，因此 `__rml_get_or_init_input_state` 可以取 `&mut self` 修改 HashMap
- 但 `Input::new(&Entity<InputState>)` 接收引用，所以方法返回 `&Entity<InputState>`（不能返回 `Entity` clone，因为 render 内避免不必要的 clone）
- 替代方案：返回 `gpui::Entity<InputState>`（clone 一次，引用计数廉价）

**选定**：返回 `gpui::Entity<InputState>`（.clone()），避免生命周期纠缠。Entity 是 Arc-based，clone 廉价。

## 验证步骤

### 阶段 1 验证
```powershell
cargo build --workspace
cargo test --workspace
```
**通过标准**：编译通过，所有测试通过。

### 阶段 2 验证
```powershell
cargo build --workspace
cargo test --workspace
cargo run -p rust-rml-demo
```
**通过标准**：
- 编译通过
- 新增单元测试通过（gen_model_input 多 Input + 类型转换）
- Demo 启动后两个 Input 独立工作

### 阶段 3 验证
- 文档渲染正确（Markdown 语法无误）
- 代码示例与实际 API 一致
- 宏注释符合 rustdoc 规范

## 任务追踪

| ID | 阶段 | 任务 | 状态 |
|----|------|------|------|
| #84 | 1 | 验证编译+测试 | 待运行 |
| #85 | 2 | scanner 提取字段类型 + CodegenCtx | 待开始 |
| #86 | 2 | 宏注入 `__rml_input_states` | 待开始 |
| #87 | 2 | 重写 `gen_model_input` + 生成辅助方法 | 待开始 |
| #88 | 2 | Demo 双向绑定示例 | 待开始 |
| #89 | 2 | 阶段 2 验证编译+测试+Demo | 待开始 |
| #90 | 3 | 宏 API 参考文档 | 待开始 |
| #91 | 3 | 完善宏文档注释 | 待开始 |
| #92 | 3 | 用户指南文档（双向绑定） | 待开始 |
