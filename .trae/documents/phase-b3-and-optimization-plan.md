# 三阶段计划：细粒度更新 + 双向绑定 + 文档

## Context

Phase B-2 已完成 observable 数据绑定基础（`#[command]` 自动注入 `bump_version + cx.notify()`，`#[computed]` 自动缓存）。但存在三个待解决的问题：

1. **性能**：`cx.notify()` 触发 GPUI 全量 render 重建（`AnyElement` 是 `ArenaBox`，frame 结束释放，无法跨 frame 缓存子树）。双向绑定场景下每次按键都全量重建，性能堪忧。
2. **功能缺口**：`model` 指令已部分实现（`gen_model_input`），但硬编码 `input_state` 字段名 + String 类型，不支持多 Input / 类型转换 / textarea。`ITwoWayBinding` trait 是死代码。
3. **文档缺口**：`docs/` 有 11 章框架但缺独立宏 API 参考；demo 仅 1 个示例；部分宏文档注释简略。

本计划按"细粒度更新 → 双向绑定 → 文档"顺序依次实施，因为细粒度更新是双向绑定的性能基础。

## 阶段 1：细粒度更新优化

### 目标

在 GPUI 约束下（`AnyElement` 不可跨 frame 缓存，`cx.notify()` 全量触发），提供务实的性能控制手段：让用户能控制 notify 时机，避免不必要的全量重渲染。

### 设计决策

**D1：不追求"真正的子树跳过"**。GPUI 的 `AnyElement(ArenaBox<dyn ElementObject>)` 在 frame 结束时释放，无法缓存复用。render 方法必须每次返回完整的 element 树。

**D2：务实方案 = 选择性 notify + 数据缓存（已实现）**。
- `#[command]` 参数化：支持 `no_notify` 选项，让用户控制是否触发 notify
- `#[computed]` 已缓存数据：render 重建时计算结果被复用
- GPUI 内置 element ID intern：layout/paint 状态自动缓存

**D3：提供 `__rml_changed_fields()` 方法**。codegen 生成，返回本次 `#[command]` 中变更的字段列表，供用户在手动 notify 时判断是否需要更新（可用于条件 notify）。

### 实施

#### Step 1.1：`#[command]` 参数解析

**文件**：[crates/macros/src/command.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs)、[crates/macros/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/lib.rs)

修改 `#[command]` 入口（lib.rs 第 97 行），将 `_args` 改为 `args`，传入 `command::expand(args, input)`。

`command::expand` 新签名：`pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream`。

参数解析（用 `syn::parse::Parse`）：
```rust
mod args {
    use syn::parse::{Parse, ParseStream};
    use syn::{Ident, LitStr, Token};

    pub enum CommandArg {
        NoNotify,           // #[command(no_notify)]
        Debounce(LitStr),   // #[command(debounce = "100ms")]（预留，本阶段不实现逻辑）
    }

    pub struct CommandArgs(pub Vec<CommandArg>);
    impl Parse for CommandArgs { ... }
}
```

逻辑：
- `no_notify`：不注入 `cx.notify()`（仍注入 `bump_version`）
- 无参数：默认行为（注入 notify）
- 未知参数：编译错误

#### Step 1.2：codegen 生成 `__rml_changed_fields()`

**文件**：[crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs)（`gen_observable_impl` 函数，第 678 行）

在 `gen_observable_impl` 中新增第四个方法：
```rust
fn __rml_changed_fields(&self) -> &'static [&'static str] {
    // 返回所有 observable 字段名（供用户在手动 notify 时判断）
    &["count", "name"]
}
```

实际上更实用的设计是返回 `Vec<&str>`，但为了简单用 `&'static [&'static str]`。

#### Step 1.3：性能优化文档

**文件**：`docs/10-advanced/performance.md`（新建或更新）

内容：
- GPUI 渲染模型说明（render 重建 + layout/paint intern 缓存）
- `#[computed]` 数据缓存机制
- `#[command(no_notify)]` 选择性 notify 用法
- 何时使用 `no_notify`（批量操作前）、何时用默认 notify（即时反馈）
- Input 双向绑定的性能考量（debounce 思路，本阶段不实现）

### 验证

- `cargo build --workspace` 通过
- `cargo test --workspace` 全部通过
- 手动测试：`#[command(no_notify)]` 标注的方法不触发 UI 更新，`#[command]` 标注的正常更新

---

## 阶段 2：Phase B-3 双向绑定

### 目标

完善 `model` 指令，支持多 Input、字段类型转换、textarea，让 `<input model="name">` 自动同步 `self.name`。

### 设计决策

**D1：宏注入 `__rml_input_states: HashMap<String, Entity<InputState>>` 字段**。解决硬编码 `input_state` 字段名问题，支持单 View 多个 Input。

**D2：scanner 提取字段类型**。扩展 `StructMetadata` 添加 `field_types: HashMap<String, String>`，codegen 据此生成类型转换代码（String 直接用，i32/f64 用 parse）。

**D3：render 内惰性初始化 InputState**。codegen 生成 `__rml_get_or_init_input_state(field, cx)` 方法，首次调用时创建 `Entity<InputState>`，后续返回缓存。`render(&mut self, ...)` 有 `&mut self`，可修改 `__rml_input_states`。

**D4：`on_change` 回调自动注入 `bump_version` + `cx.notify()`**。与 `#[command]` 一致的语义，用户无需手动调用。

### 实施

#### Step 2.1：scanner 提取字段类型

**文件**：[crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs)、[crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)、[crates/engine/src/build/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)

- `StructMetadata` 添加 `pub field_types: HashMap<String, String>`
- scanner 第一遍扫描 struct 时，从 `pub field: Type` 提取类型字符串（用 `quote!(#ty).to_string()` + `split_whitespace().collect()`，复用 `return_type_str` 逻辑）
- `CodegenCtx` 添加 `pub field_types: HashMap<String, String>`
- build.rs 传入 `field_types`
- 测试 ctx() 函数添加 `field_types: HashMap::new()`

#### Step 2.2：宏注入 `__rml_input_states` 字段

**文件**：[crates/macros/src/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs)（`inject_tracking_fields` 函数，第 71 行）

在 `inject_tracking_fields` 末尾追加：
```rust
let input_states_field: Field = parse_quote! {
    #[allow(dead_code)]
    __rml_input_states: std::collections::HashMap<String, gpui::Entity<rml_ui::InputState>>
};
named.named.push(input_states_field);
```

注意：`HashMap` 和 `Entity` 都需要 `Default`，`HashMap::default()` 是空 map，`#[derive(Default)]` 兼容。

#### Step 2.3：重写 `gen_model_input`

**文件**：[crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs)（`gen_model_input` 函数，第 493 行）

重写为：
```rust
fn gen_model_input(elem: &Element, ctx: &CodegenCtx, id_counter: &mut usize, field: String) -> Result<String, CodegenError> {
    let field_type = ctx.field_types.get(&field).cloned().unwrap_or_default();
    
    // 类型转换：state.value() -> field type
    let convert_expr = match field_type.as_str() {
        "String" => "state.value()".to_string(),
        "i32" | "i64" | "u32" | "u64" => format!("state.value().parse::<{}>().unwrap_or(0)", field_type),
        "f32" | "f64" => format!("state.value().parse::<{}>().unwrap_or(0.0)", field_type),
        _ => return Err(CodegenError { message: format!("unsupported field type for model: {}", field_type) }),
    };
    
    // 生成代码
    let code = format!(
        r#"{{
            let __state = self.__rml_get_or_init_input_state("{field}", cx);
            rml_ui::Input::new(&__state)
                .value(self.{field}.clone())
                .on_change(cx.listener(move |this, state: &rml_ui::InputState, _window, cx| {{
                    this.{field} = {convert_expr};
                    this.__rml_bump_version("{field}");
                    cx.notify();
                }}))
        }}"#,
        field = field,
        convert_expr = convert_expr,
    );
    Ok(code)
}
```

#### Step 2.4：codegen 生成 `__rml_get_or_init_input_state`

**文件**：[crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs)（`gen_observable_impl` 函数）

在 `gen_observable_impl` 中新增方法：
```rust
fn __rml_get_or_init_input_state(&mut self, field: &str, cx: &mut gpui::Context<Self>) -> gpui::Entity<rml_ui::InputState> {
    if let Some(state) = self.__rml_input_states.get(field) {
        return *state;
    }
    let state = cx.new_entity(|_| rml_ui::InputState::default());
    self.__rml_input_states.insert(field.to_string(), state);
    state
}
```

#### Step 2.5：Demo 双向绑定示例

**文件**：[demo/src/main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml)、[demo/src/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml.rs)

在 demo 中添加：
- `pub name: String` 字段 + `<input model="name" placeholder="输入姓名" />`
- `pub age: i32` 字段 + `<input model="age" placeholder="输入年龄" />`
- 显示绑定值的插值：`{name}`、`{age}`

### 验证

- `cargo build -p rust-rml-demo` 通过
- `cargo test --workspace` 通过
- 运行 demo：Input 输入 → 模型字段更新 → 插值显示更新
- 验证多 Input 不冲突（name 和 age 独立）
- 验证 i32 类型转换（输入非数字时不崩溃，返回 0）

---

## 阶段 3：文档

### 目标

编写宏 API 参考文档 + 完善用户指南 + 扩展 demo 示例。

### 实施

#### Step 3.1：宏 API 参考文档

**文件**：`docs/api/macros.md`（新建）

完整宏 API 参考，每个宏包含：
- 签名
- 参数说明
- 行为描述
- 完整示例
- 限制说明

覆盖宏：
- `#[derive(IModel)]` + `#[element]` 字段属性
- `#[component]`
- `#[window]`
- `#[command]`（含 `no_notify` 参数）
- `#[computed]`
- `#[on_loaded]` / `#[on_unloaded]`

#### Step 3.2：完善宏文档注释

**文件**：[crates/macros/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/lib.rs)

为每个宏的 `///` 注释补充：
- 参数说明
- 完整示例
- 限制说明
- 与其他宏的协作关系

#### Step 3.3：双向绑定用户指南

**文件**：`docs/03-binding/two-way.md`（更新或新建）

内容：
- `model` 指令用法
- 支持的字段类型（String/i32/f64）
- 多 Input 用法
- textarea 用法（如本阶段实现）
- `ITwoWayBinding` trait 扩展点（自定义组件双向绑定）
- 完整示例

#### Step 3.4：性能优化指南

**文件**：`docs/10-advanced/performance.md`（新建）

内容：
- GPUI 渲染模型（render 重建 + layout/paint intern）
- `#[computed]` 数据缓存
- `#[command(no_notify)]` 选择性 notify
- 何时 debounce（思路，不实现）
- element ID 稳定性（ref 指令）

#### Step 3.5：扩展 demo 示例

**文件**：`demo/`（可能新增 `examples/` 目录）

添加示例：
- 双向绑定示例（阶段 2 已添加）
- `#[command(no_notify)]` 示例
- `#[computed]` 缓存示例

### 验证

- `cargo test --workspace` 通过（文档测试不破坏）
- 人工审阅文档完整性

---

## 关键文件改动清单

| 阶段 | 文件 | 操作 |
|------|------|------|
| 1 | `crates/macros/src/lib.rs` | 修改：`#[command]` 接受 args |
| 1 | `crates/macros/src/command.rs` | 修改：参数解析 + 条件注入 notify |
| 1 | `crates/engine/src/compiler/codegen.rs` | 修改：生成 `__rml_changed_fields` |
| 1 | `docs/10-advanced/performance.md` | 新建：性能优化指南 |
| 2 | `crates/engine/src/build/scanner.rs` | 修改：提取字段类型 |
| 2 | `crates/engine/src/compiler/mod.rs` | 修改：CodegenCtx 添加 `field_types` |
| 2 | `crates/engine/src/build/mod.rs` | 修改：传入 `field_types` |
| 2 | `crates/macros/src/component.rs` | 修改：注入 `__rml_input_states` 字段 |
| 2 | `crates/engine/src/compiler/codegen.rs` | 修改：重写 `gen_model_input` + 生成 `__rml_get_or_init_input_state` |
| 2 | `demo/src/main_window.rml` + `.rml.rs` | 修改：添加双向绑定示例 |
| 3 | `docs/api/macros.md` | 新建：宏 API 参考 |
| 3 | `crates/macros/src/lib.rs` | 修改：完善文档注释 |
| 3 | `docs/03-binding/two-way.md` | 新建/更新：双向绑定指南 |
| 3 | `docs/10-advanced/performance.md` | 新建：性能指南 |

## 验证步骤

1. **阶段 1 完成后**：
   - `cargo build --workspace` 通过
   - `cargo test --workspace` 全部通过
   - 手动测试 `#[command(no_notify)]` 不触发 UI 更新

2. **阶段 2 完成后**：
   - `cargo build -p rust-rml-demo` 通过
   - `cargo test --workspace` 通过
   - 运行 demo：Input 输入 → 模型更新 → 插值更新
   - 验证多 Input + 类型转换

3. **阶段 3 完成后**：
   - 文档审阅完整
   - demo 示例覆盖双向绑定、性能优化

## 依赖顺序

```
阶段 1（细粒度更新）
   ↓
阶段 2（双向绑定）── 依赖阶段 1 的 no_notify 选项（Input 场景可用）
   ↓
阶段 3（文档）── 依赖阶段 1-2 功能稳定
```

## 假设与决策

1. **不实现真正的子树缓存**：`AnyElement` 是 `ArenaBox`，frame 结束释放，无法跨 frame 复用。GPUI 无原生 memo 机制。
2. **不实现 debounce**：需要 GPUI timer/异步机制，复杂度高，本计划仅预留参数位置。
3. **`__rml_input_states` 用 HashMap**：支持任意数量 Input，key 为字段名。惰性初始化避免在 struct 构造时创建所有 InputState。
4. **字段类型转换有限**：仅支持 String/i32/i64/u32/u64/f32/f64。其他类型报错，用户需用 `ITwoWayBinding` trait 自定义。
5. **`ITwoWayBinding` trait 暂不接入 codegen**：保持为扩展点，供未来自定义组件双向绑定使用。本阶段聚焦 Input 组件。
6. **textarea 暂走 input 路径**：gpui-component 的 TextInput 可能与 Input 不同，本阶段不单独处理 textarea。
