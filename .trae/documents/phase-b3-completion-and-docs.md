# Phase B-3 收尾与文档更新计划

> 本计划承接上一会话。Phase B-3 双向绑定代码（codegen + macros + 测试文件）已完成且 `cargo build -p rust-rml-demo` 通过，但 **测试尚未验证、demo 尚未运行时验证、文档与实际实现严重不符**。本计划整合剩余三件事：① 验证 ② 更新宏 API 文档 ③ 更新用户指南。

---

## 一、当前状态分析（基于 Phase 1 探索）

### 1.1 已完成且稳定的代码

| 文件 | 状态 | 说明 |
|---|---|---|
| `crates/ui/src/lib.rs` | ✅ 已 re-export `InputEvent` | 供生成代码引用 |
| `crates/macros/src/component.rs` | ✅ 注入 `__rml_input_states` + `__rml_input_state_versions` | 不存储 `Vec<Subscription>`（Sync 约束） |
| `crates/macros/src/command.rs` | ✅ 自动注入 `bump_version` + `cx.notify()`，支持 `no_notify` | 已实现完整 |
| `crates/engine/src/compiler/codegen.rs` L497-535 | ✅ `gen_model_input` 重写完成 | 传引用给 `Input::new` |
| `crates/engine/src/compiler/codegen.rs` L541-566 | ✅ `gen_field_value_expr` + `gen_field_assign_expr` | 类型转换正确 |
| `crates/engine/src/compiler/codegen.rs` L850-933 | ✅ `gen_input_state_impl` 重写完成 | 含首次创建/正向同步/反向订阅 |
| `crates/engine/tests/codegen_two_way_binding_test.rs` | ✅ 11 个测试已重写 | 待运行验证 |

### 1.2 实际 codegen 生成的双向绑定机制（来自 L850-933 实读）

```
首次 render：
  1. cx.new(|cx| InputState::new(window, cx).placeholder(p))  // 创建 entity
  2. entity.update(cx, |state, cx| state.set_value(initial_value, window, cx))  // 初始正向同步
  3. cx.subscribe(&entity, |this, input_entity, event, cx| {
       match event {
         InputEvent::Change => {
           let value = input_entity.read(cx).value();
           match field { "name" => this.name = value.to_string(); ... }
           this.__rml_bump_version(field);
           this.__rml_input_state_versions.insert(field.to_string(), v);  // 循环防护
           cx.notify();
         }
         _ => {}
       }
     }).detach()  // Subscription 非 Sync，detach 让其随 entity 存活
  4. __rml_input_state_versions.insert(field, current_version)  // 记录初始版本

后续 render（VM 字段被 #[command] 修改后）：
  current_version != last_synced
  → entity.update(cx, |state, cx| state.set_value(value, window, cx))  // 正向同步
  → __rml_input_state_versions.insert(field, current_version)

循环防护：
  - set_value 内部 emit_events=false，不触发 InputEvent::Change
  - 反向闭包 bump_version 后立即标记 __rml_input_state_versions，render 时版本号相等跳过 set_value
```

### 1.3 文档现状（严重过时）

#### `docs/04-code-behind/macros.md`（333 行，需更新）
**过时点**：
- L88-91：`#[command]` 示例**仍写手动 `cx.notify()`**，但宏现在自动注入
- 未提及 `#[command(no_notify)]` 选项
- 未提及 `#[command]` 自动注入 `__rml_bump_version` 的机制
- 未提及 `#[computed]` 的版本追踪 + `ComputedCache` 缓存机制
- L73-78：`PrimaryButton` 示例用 `Arc<dyn Fn(&ClickEvent)>`，与当前 gpui-component 实践不符（应 `Rc<dyn Fn>`）

#### `docs/03-binding/two-way-binding.md`（333 行，需更新）
**过时点**：
- L17-26：声称等价于 `value={user_name} + oninput={update_user_name}`，**实际是 InputState entity + cx.subscribe + InputEvent::Change**
- L31-44：数据流图描述"RML 自动生成的命令"，**实际是订阅闭包**
- L127-147：字段要求说"实现 Default"，**实际是基于版本号追踪，Default 不是必须**
- L237-247：嵌套字段说"不支持"，**实际可通过 computed 或命令处理**
- L257-268：性能说"2 个订阅"，**实际是 1 个 cx.subscribe + 版本号对比**
- 未提及循环防护机制
- 未提及类型自动转换（i32→parse、String→to_string）

#### `docs/10-advanced/performance.md`（已较好，少量补充）
- Phase 1 已完成
- 可补充：双向绑定的性能特征（首次创建 + 版本号对比开销）

---

## 二、变更计划

### Step 1：验证 Phase B-3 代码（运行时）

```bash
# 1.1 运行双向绑定测试（11 个）
cargo test -p rust-rml-engine --test codegen_two_way_binding_test

# 1.2 全工作区测试（确保无回归）
cargo test --workspace

# 1.3 Demo 运行时验证
cargo run -p rust-rml-demo
```

**验证标准**：
- 11 个双向绑定测试全部通过
- 工作区测试无回归（之前 219 个 + 7 个 codegen_observable_test + 11 个 codegen_two_way_binding_test）
- Demo 启动后，"姓名"和"年龄"输入框可输入，且修改 ViewModel 字段（如点击按钮）时输入框值同步更新

**如果测试失败**：根据失败信息修复 codegen 或测试断言（不重新设计架构）。

### Step 2：更新 `docs/04-code-behind/macros.md`（宏 API 文档）

**目标**：让文档准确反映当前宏行为，符合用户"添加详细的宏 API 文档"要求。

**修改清单**：

1. **L7-15 宏属性总览表**：保持 7 个宏不变，但补充每个宏的关键行为到说明列

2. **L82-130 `#[command]` 章节**：重写以反映自动注入机制
   - 删除"命令方法必须满足以下签名"中的 `cx.notify()` 手动示例
   - 新增 **4.2.4.1 自动行为**：说明宏自动注入 `__rml_bump_version` + `cx.notify()` 的机制
   - 新增 **4.2.4.2 `no_notify` 参数**：何时禁用自动 notify（异步任务中批量更新）
   - 新增 **4.2.4.3 字段修改检测**：说明 `self.field = ...` / `self.field += ...` 自动识别
   - 保留命名约定部分

3. **L131-161 `#[computed]` 章节**：补充缓存机制
   - 新增 **4.2.5.1 版本追踪机制**：`__rml_<field>_version` + `__rml_computed_deps_version`
   - 新增 **4.2.5.2 ComputedCache**：`get_or_compute::<T>` + 版本号对比
   - 新增 **4.2.5.3 依赖自动追踪**：build.rs 扫描 `#[computed]` 方法体的 `self.<field>` 访问
   - 保留签名要求和 .rml 访问方式

4. **L67-81 `#[component]` 章节**：补充注入字段说明
   - 新增说明：宏自动注入 `__rml_<field>_version`、`__rml_computed_cache`、`__rml_input_states`、`__rml_input_state_versions`（均为私有，不影响 `IModel::rml_fields()`）

5. **L17-66 `#[window]` 章节**：保持不变（已正确）

6. **L163-256 `#[on_loaded]`/`#[on_unloaded]` 章节**：保持不变（已正确）

7. **L258-333 `#[element]` + 组合 + 常见错误**：保持不变（已正确）

### Step 3：更新 `docs/03-binding/two-way-binding.md`（用户指南）

**目标**：让用户指南反映实际实现机制，符合用户"添加详细的用户指南"要求。

**修改清单**：

1. **L7-26 双向绑定定义**：修正等价表述
   - 删除"等价于 `value={user_name} + oninput={update_user_name}`"
   - 改为说明实际机制：每个 `<input model={field}>` 惰性创建一个 `Entity<InputState>`，通过 `cx.subscribe` 订阅 `InputEvent::Change` 实现反向同步，通过版本号对比实现正向同步

2. **L28-44 数据流图**：重画
   - 正向流：`#[command]` 修改字段 → `__rml_bump_version` → `cx.notify()` → render → `__rml_get_or_init_input_state` 检测版本号变化 → `InputState::set_value`
   - 反向流：用户输入 → `InputState` 触发 `InputEvent::Change` → 订阅闭包回写字段 + `bump_version` + 标记同步版本 + `cx.notify()`
   - 循环防护：`set_value` 内部 `emit_events=false` + 版本号标记

3. **L46-55 适用标签与字段类型表**：修正
   - 当前 codegen 仅支持 `<input model={field}>`（基于 `InputState`）
   - 类型支持：`i32/u32/i64/u64/isize/usize` → `parse::<T>().unwrap_or(0)`，`f32/f64` → `parse.unwrap_or(0.0)`，`bool` → `!value.is_empty()`，`String/SharedString` → `to_string()`
   - `<textarea>`/`<input type="checkbox">` 当前未实现（标注为"未来支持"）

4. **L57-125 基础用法示例**：更新代码示例
   - 文本输入示例保持（`String` 字段）
   - 数字输入示例保持（`i32`/`f64` 字段）
   - 删除复选框示例（未实现）
   - 删除多行文本示例（未实现）

5. **L127-147 字段要求**：修正
   - 保留 `pub` 要求
   - 删除"实现 Default"硬性要求（实际通过版本号追踪，Default 是 `#[derive(Default)]` 的惯例）
   - 新增 **类型支持说明**：列出 codegen 支持的字段类型

6. **L149-167 与事件处理协作**：修正
   - 说明 `model` 与 `onchange`/`onclick` 等事件的协作（`onchange` 由 `cx.subscribe` 内部处理，无需用户写）
   - 删除"`model` 的事件处理在 `oninput` 之前执行"（不准确，当前 `model` 通过订阅闭包处理，不与 `oninput` 冲突）

7. **L215-256 特殊场景**：修正
   - 自定义组件双向绑定：当前未实现（标注"未来支持"）
   - 嵌套字段：保持"不支持"说明，但补充可通过 `#[computed]` 派生

8. **L257-268 性能**：修正
   - 删除"2 个订阅"
   - 改为：每个 `<input model={field}>` 创建 1 个 `Entity<InputState>` + 1 个 `cx.subscribe` 订阅（`detach`）
   - 正向同步开销：版本号对比（`AtomicU64::load`，O(1)）
   - 仅在版本号变化时调用 `set_value`

9. **L270-321 常见陷阱**：更新
   - 保留"忘记 `pub`"陷阱
   - 修正"命令中重复修改字段"陷阱：说明 `#[command]` 自动 `bump_version`，但 `model` 的反向闭包也会 `bump_version`，可能导致循环（实际有版本号标记防护）
   - 保留"列表中使用 model"陷阱

10. **新增 3.3.10 循环防护机制**：详细说明
    - `set_value` 内部 `emit_events=false`
    - 反向闭包 `bump_version` 后立即标记 `__rml_input_state_versions`
    - render 时版本号相等跳过 `set_value`

### Step 4：补充 `docs/10-advanced/performance.md`

**目标**：补充双向绑定性能特征，符合用户"分析当前代码的性能瓶颈"要求。

**修改清单**：

1. **在 10.1.2 `#[computed]` 数据缓存章节后**新增 **10.1.X 双向绑定的性能特征**：
   - 惰性初始化：首次 render 创建 `Entity<InputState>`，后续 render 仅版本号对比
   - 正向同步开销：`AtomicU64::load` + 比较，O(1)
   - 反向同步开销：`InputEvent::Change` 触发订阅闭包，闭包内 `bump_version` + `HashMap::insert` + `cx.notify()`
   - 内存开销：每个 `<input model={field}>` 一个 `Entity<InputState>` + 一个 `Subscription`（detach 后随 entity 存活）

2. **在"性能瓶颈通常出现在"列表**补充：
   - 双向绑定的 `set_value` 在每次版本号变化时调用，频繁修改字段可能触发多次 `set_value`（但版本号标记防护避免循环）

---

## 三、不做的事项（明确边界）

- **不**重写 codegen 架构（已完成且 demo 编译通过）
- **不**实现 `<textarea>` / `<input type="checkbox">` 双向绑定（超出当前任务范围，标注"未来支持"）
- **不**实现自定义组件双向绑定（超出当前任务范围）
- **不**修改 `crates/macros/src/command.rs` 或 `component.rs`（已完成）
- **不**修改 `docs/04-code-behind/macros.md` 中 `#[window]`/`#[on_loaded]`/`#[on_unloaded]`/`#[element]` 章节（已正确）
- **不**修改其他文档文件（如 `one-way-binding.md`、`computed.md` 等）

---

## 四、验证步骤

### 4.1 代码验证
```bash
cargo test -p rust-rml-engine --test codegen_two_way_binding_test  # 11 个测试通过
cargo test --workspace  # 无回归
cargo run -p rust-rml-demo  # Demo 启动，输入框可交互
```

### 4.2 文档验证
- `docs/04-code-behind/macros.md`：`#[command]` 示例不再写手动 `cx.notify()`；包含 `no_notify` 说明；包含 `#[computed]` 缓存机制
- `docs/03-binding/two-way-binding.md`：数据流图反映实际机制；包含循环防护说明；类型支持表准确
- `docs/10-advanced/performance.md`：包含双向绑定性能特征

### 4.3 一致性检查
- 文档中的代码示例与 `crates/demo/src/main_window.rml` + `main_window.rml.rs` 一致
- 文档中的宏行为描述与 `crates/macros/src/command.rs` 实际实现一致
- 文档中的双向绑定机制与 `crates/engine/src/compiler/codegen.rs` L850-933 实际生成代码一致

---

## 五、执行顺序

1. **Step 1**：运行测试 + Demo 验证（如失败则修复，预计 5-15 分钟）
2. **Step 2**：更新 `macros.md`（预计 20-30 分钟）
3. **Step 3**：更新 `two-way-binding.md`（预计 25-35 分钟）
4. **Step 4**：补充 `performance.md`（预计 10-15 分钟）
5. **最终验证**：再读一遍三个文档，确保一致性

---

## 六、假设与决策

### 假设
- Phase 1 探索读取的 codegen.rs L497-566 和 L850-933 是当前最新版本（已通过 `cargo build` 验证）
- `crates/engine/tests/codegen_two_way_binding_test.rs` 内容如系统提醒所示（11 个测试）
- demo 的 `main_window.rml` 已包含 `<input model={name}>` 和 `<input model={age}>`

### 决策
- 文档采用**更新而非重写**策略，保留现有结构和编号，仅修正过时内容
- 代码示例使用 `crates/demo/src/main_window.rml.rs` 的真实字段（`name: String`、`age: i32`）
- 未实现的功能（`<textarea>`、复选框、自定义组件双向绑定）在文档中明确标注"未来支持"，不假装已实现
- 遵循 gpui/gpui-component 最佳实践：`Subscription.detach()`、`Entity<InputState>`、`cx.subscribe` 模式（已在代码中实现）
