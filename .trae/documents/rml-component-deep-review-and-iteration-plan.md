# RML gpui-component 深度审查与迭代计划

> 创建日期：2026-07-11
> 目标：深入审查已实现的 gpui-component 组件，从架构规范遵守和开发者体验两个维度评估，识别异味用法和反人类思维，制定针对性迭代计划。

---

## 一、审查范围与方法

### 审查范围
- **UI 封装层**: `crates/ui/src/components/` 全部组件
- **Codegen 层**: `crates/engine/src/compiler/components/` 全部组件的 gen.rs/setters.rs
- **Translator 层**: `crates/engine/src/compiler/translator/component/` 全部 translator
- **注册层**: `tags.rs`、`props_registry.rs`、`setters.rs`
- **Demo 层**: `demo/src/cases/` 全部 .rml + .rml.rs 文件

### 审查维度
1. **架构规范遵守**: kebab-case 命名、独立布尔 variant、ComponentKind 选择、模块化、双向绑定机制
2. **开发者体验**: 声明式完整性、心智模型一致性、demo 可学习性、事件载荷可发现性
3. **异味识别**: 双路径分裂、返回值丢弃、声明式缺口、文档不一致

---

## 二、发现汇总

### A. 架构设计缺陷（严重 — 破坏开发者体验）

#### A1. Input/TextInput/NumberInput PascalCase ref 路径不支持 placeholder/default_value/masked

**严重度**: 🔴 高 — 破坏声明式范式

**现象**:
```xml
<!-- ❌ 不工作：placeholder 被错误映射到 .placeholder()，Input 无此方法 -->
<Input ref="username" placeholder="请输入用户名" />

<!-- ✅ 工作：value 路径通过 InputStateBridge 支持 placeholder -->
<Input value={username} placeholder="请输入用户名" />

<!-- ✅ 工作：Select 的 ref 路径支持 placeholder -->
<Select ref="city" placeholder="请选择城市" />
```

**根因**:
- Input 的 `state_ctor` 是固定闭包 `|w, c| rml_ui::InputState::new(w, c)`，不包含 builder 参数
- placeholder/default_value/masked 是 InputState 的 builder 方法，需在构造时链式调用
- ref 路径走 generic Stateful translator，不会从属性提取 builder 参数注入 state_ctor
- 对比: **OtpInput 已有此模式**（`otp_input.rs` translator 提取 length/masked/default_value 注入 state_ctor），Input 未应用

**影响**:
- 开发者必须在 `on_loaded` 中手动 `cx.new(|cx| InputState::new(w, cx).placeholder("..."))` 创建 Entity
- 字段名必须硬编码为 `input_state`（tags.rs 中 state_field 固定）
- 破坏声明式范式 — 同一组件因 `ref` vs `value` 产生完全不同的能力
- Select/Combobox/DatePicker 支持 placeholder，Input 不支持 — 同类组件不一致

**修复方案**: 创建 `InputTranslator`（类似 `OtpInputTranslator`），提取 placeholder/default_value/masked 注入 state_ctor

---

#### A2. Dialog/AlertDialog on_ok/on_cancel 返回值被忽略

**严重度**: 🔴 高 — 丢失关键交互能力

**现象**:
```xml
<!-- 开发者期望：表单验证失败时返回 false 阻止关闭 -->
<Dialog title="编辑用户" on-ok={on_save}>
    <Input ref="name" />
</Dialog>
```

```rust
// 开发者意图：验证失败时返回 false
pub fn on_save(&mut self, _: &ClickEvent, cx: &mut Context<Self>) -> bool {
    if self.name_field.is_empty() {
        return false; // ❌ 返回值被 codegen 丢弃
    }
    true
}
```

**根因**:
- `cx.listener()` 产生 `Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>)`（无返回值）
- Dialog 的 `on_ok` 需要 `Fn(&ClickEvent, &mut Window, &mut App) -> bool`
- codegen 用 entity 捕获闭包绕过类型不匹配，但**固定返回 `true`**
- 代码位置: `dialog/setters.rs` 第 108-155 行，`true` 硬编码在第 123/135/149 行

**影响**:
- 无法实现表单验证失败时阻止关闭对话框
- 无法实现"保存中...请等待"异步关闭
- `on-cancel` 同理 — 取消前确认逻辑无法实现

**修复方案**: 让 codegen 从 handler 方法返回值传递 bool

---

#### A3. Dialog/AlertDialog footer 仅支持字符串

**严重度**: 🟡 中 — 限制 UI 表达力

**现象**:
```xml
<!-- ❌ footer 只接受字符串 -->
<Dialog title="编辑" footer="确认要保存吗？">

<!-- 开发者期望：footer 中放置按钮 -->
<Dialog title="编辑">
    <Button slot="footer" label="保存" primary />
    <Button slot="footer" label="取消" />
</Dialog>
```

**根因**:
- `footer` 作为 Static 属性映射到 `.footer("字符串")`
- Dialog 的 `.footer()` 接收 `impl IntoElement`，可以接收元素
- 但 codegen 只处理字符串属性，未支持 `slot="footer"` 元素注入

**影响**:
- 对话框操作按钮只能放在 content 区域，不符合常规 UI 模式（footer 居中/右对齐按钮）
- AlertDialog 的 confirm 按钮是内置的，但 Dialog 的自定义 footer 按钮无法实现

**修复方案**: 在 Dialog gen.rs 中支持 `slot="footer"` → `.footer(element)`，同 header slot 模式

---

### B. 一致性问题（中等）

#### B1. 缺失 Demo 案例

**严重度**: 🟡 中

| 组件 | 状态 | 说明 |
|------|------|------|
| Skeleton | ❌ 缺失 | Phase 1 组件，有 props 注册但无 demo |
| Breadcrumb | ❌ 缺失 | 有 props 注册（items/on_select）但无 demo |
| Tabs | ⚠️ 合并 | tab_bar_case 同时演示 TabBar 和 Tabs，但无独立 tabs_case |

#### B2. 文档中的 snake_case 不一致

**严重度**: 🟡 中 — 误导开发者

**现象**: Demo 描述文本（`<p>` 标签内）使用 snake_case 引用属性名，与实际 RML kebab-case 不一致

**示例**:
- `alert_dialog_case.rml` 第 34 行: `close_button=false 和 overlay_closable=false`（应为 `close-button=false`）
- `input_case.rml` 第 4 行: `on_change/on_enter/on_focus/on_blur`（应为 `on-change/on-enter/...`）
- 多个 demo 的 description 属性中使用 snake_case

**影响**: 开发者可能误用 snake_case 属性名，导致属性不被识别

#### B3. Input placeholder 能力不一致（A1 的延伸）

**严重度**: 🔴 高（与 A1 同根因）

| 组件 | ref 路径 placeholder | value 路径 placeholder |
|------|---------------------|----------------------|
| Input | ❌ 不支持 | ✅ 支持 |
| TextInput | ❌ 不支持 | ✅ 支持 |
| NumberInput | ❌ 不支持 | ✅ 支持 |
| Select | ✅ 支持 | N/A |
| Combobox | ✅ 支持 | N/A |
| DatePicker | ✅ 支持 (属性) | N/A |

---

### C. 开发者体验改进（轻度）

#### C1. Dialog/AlertDialog 演示缺失 on-ok/on-cancel

**严重度**: 🟢 低

- 两个 demo 都没有演示 on-ok/on-cancel 事件处理
- 开发者无法从 demo 中学习如何处理确认/取消逻辑
- API 表格标注了 `Fn(&ClickEvent, &mut Window, &mut App) -> bool` 签名，但未演示

#### C2. 事件载荷类型未在 API 表格中文档化

**严重度**: 🟢 低

不同组件的事件传递不同载荷类型，API 表格中仅标注 "event"：

| 组件 | 事件 | 载荷类型 | API 表格标注 |
|------|------|---------|-------------|
| Select | on-change | `Option<SharedString>` | "event" |
| Combobox | on-change | `Vec<SharedString>` | "event" |
| Calendar | on-select | `Date` | "event" |
| ColorPicker | on-change | `Option<Hsla>` | "event" |
| Accordion | on-toggle | `&[usize]` | "event" |
| Checkbox | on-change | `&bool` | "event" |

#### C3. NumberInput 与 Input 共享 input_state 字段名

**严重度**: 🟢 低

- NumberInput 的 `state_field` 也是 `input_state`（与 Input 相同）
- 同一 View 同时使用 Input 和 NumberInput 时，字段名冲突
- 需手动管理 Entity 或拆分到子 View

---

## 三、迭代计划

### Phase R1: Input 声明式完整性修复（A1/B3 — 最高优先级）

**目标**: 创建 `InputTranslator`，统一 ref/value 路径的 placeholder/default_value/masked 支持

**变更文件**:

1. **新建 `crates/engine/src/compiler/translator/component/input.rs`**
   - 参考 `otp_input.rs` translator 模式
   - 提取 `placeholder`（Static/Bind）、`default_value`（Static）、`masked`（Static bool）属性
   - 构建自定义 state_ctor: `|w, c| rml_ui::InputState::new(w, c).placeholder("...").masked(true).default_value("...")`
   - SKIP_ATTRS = `["placeholder", "default_value", "masked"]`
   - 调用 `gen_stateful_body` 生成构造表达式
   - 剩余属性走通用 setter 分发

2. **修改 `crates/engine/src/compiler/translator/component/mod.rs`**
   - 添加 `pub mod input;`
   - 在 `register_all()` 中添加 `input::register(registry);`
   - 注意：需确保 input translator 在 stateful 之前注册（优先级）

3. **修改 `crates/engine/src/compiler/translator/component/stateful.rs`**
   - 在 stateful register 中排除 Input/TextInput（由 input translator 接管）
   - 或在 stateful translator 的 `matches()` 中排除 Input/TextInput

4. **更新 `demo/src/cases/input_case.rml`**
   - Section 2 改为演示 `<Input ref="name" placeholder="请输入用户名" />` 直接使用
   - 移除 Pattern B 手动创建 InputState 的说明
   - 补充 default_value/masked 演示

**验证**:
- `cargo test -p rust-rml-engine -- input` 通过
- `cargo build -p rust-rml-demo` 成功
- `<Input ref="name" placeholder="..." />` 编译通过且 placeholder 生效

---

### Phase R2: Dialog 交互能力修复（A2/A3 — 高优先级）

**目标**: 恢复 on_ok/on_cancel 返回值控制；支持 footer 元素注入

**变更文件**:

1. **修改 `crates/engine/src/compiler/components/dialog/setters.rs`**
   - `bool_event_setter()` 修改：handler 方法返回 `bool`，codegen 传递返回值
   - 生成代码: `entity.update(cx, |this, cx| this.on_ok(&rml_ev, cx))` → 返回值作为闭包返回
   - 需要调整 handler 签名为 `Fn(&ClickEvent, &mut Context<Self>) -> bool`

2. **修改 `crates/engine/src/compiler/components/dialog/gen.rs`**
   - 在子节点处理中增加 `slot="footer"` → `.footer(element)` 路由
   - 同 header slot 模式（`slot="header"` → `.header()`）

3. **修改 `crates/engine/src/compiler/components/alert_dialog/`**
   - 同步 on_ok/on_cancel 返回值修复
   - 同步 footer slot 支持（如 AlertDialog 有 footer 方法）

4. **更新 `demo/src/cases/dialog_case.rml` + `.rml.rs`**
   - 添加 on-ok/on-cancel 演示（表单验证场景）
   - 添加 slot="footer" 按钮演示
   - 更新 API 表格

5. **更新 `demo/src/cases/alert_dialog_case.rml` + `.rml.rs`**
   - 添加 on-ok/on-cancel 演示

**验证**:
- `cargo test -p rust-rml-engine -- dialog` 通过
- `cargo build -p rust-rml-demo` 成功
- on-ok 返回 false 时对话框不关闭

---

### Phase R3: 缺失 Demo 补全（B1 — 中优先级）

**目标**: 为 Skeleton 和 Breadcrumb 创建 demo 案例

**变更文件**:

1. **新建 `demo/src/cases/skeleton_case.rml` + `.rml.rs`**
   - 演示基础骨架屏 + secondary 变体
   - 演示与 Card 组合的加载占位场景

2. **新建 `demo/src/cases/breadcrumb_case.rml` + `.rml.rs`**
   - 演示 items 绑定 + on_select 事件
   - 演示面包屑导航场景

3. **修改 `demo/src/cases/mod.rs`**
   - 注册新 demo 模块

4. **更新 i18n 文件**
   - 添加 `case.skeleton.title` / `case.breadcrumb.title`

**验证**:
- `cargo build -p rust-rml-demo` 成功
- 新 demo 在 Demo 索引中可见

---

### Phase R4: 文档一致性修复（B2/C1/C2 — 低优先级）

**目标**: 修复文档 snake_case 不一致；补充事件载荷类型；完善 Dialog demo

**变更文件**:

1. **批量修改 demo .rml 文件中的描述文本**
   - 将 `<p>` 和 `description` 属性中的 snake_case 属性名改为 kebab-case
   - 重点: `close_button` → `close-button`、`overlay_closable` → `overlay-closable`、`on_change` → `on-change` 等
   - 涉及文件: `alert_dialog_case.rml`、`input_case.rml`、`select_case.rml`、`combobox_case.rml`、`date_picker_case.rml` 等约 10-15 个文件

2. **更新各 demo 的 API 表格**
   - 在事件属性行补充载荷类型
   - 例如: `("on-change", "event (Option<SharedString>)", "选择确认回调")`
   - 涉及文件: `select_case.rml.rs`、`combobox_case.rml.rs`、`calendar_case.rml.rs`、`color_picker_case.rml.rs`、`accordion_case.rml.rs`、`checkbox_case.rml.rs` 等

**验证**:
- 全文搜索 `_[a-z]+=` 在 .rml 文件中无实际属性使用（仅描述文本中的引用也改为 kebab-case）
- API 表格中事件属性行包含载荷类型

---

## 四、优先级与依赖关系

```
Phase R1 (Input 声明式) ──────────────┐
                                       ├──→ 可并行
Phase R2 (Dialog 交互) ────────────────┘
                                       
Phase R3 (缺失 Demo) ─── 独立，可与 R1/R2 并行

Phase R4 (文档修复) ─── 依赖 R1/R2 完成（避免改完又改）
```

| Phase | 优先级 | 预估工时 | 依赖 |
|-------|--------|---------|------|
| R1 | P0 最高 | 4-6h | 无 |
| R2 | P1 高 | 4-6h | 无 |
| R3 | P2 中 | 2-3h | 无 |
| R4 | P3 低 | 2-3h | R1, R2 |

---

## 五、假设与决策

1. **InputTranslator 接管 Input/TextInput**: stateful.rs 中的通用 Stateful translator 需排除 Input/TextInput，由 input translator 优先匹配。NumberInput 暂不接管（其 placeholder 需求较低，有步进按钮等特殊逻辑）。

2. **Dialog on_ok 签名调整**: 当前 handler 签名为 `Fn(&ClickEvent, &mut Context<Self>)`，需改为返回 `bool`。这是破坏性变更（现有 handler 需添加返回值），但无兼容性设计原则支持。

3. **footer slot 与 footer 属性共存**: `footer="字符串"` 和 `<Button slot="footer" />` 可共存 — 属性优先（简单场景），slot 补充（复杂场景）。若同时存在，slot 覆盖属性。

4. **Breadcrumb items 为 bind 属性**: Breadcrumb 已在 props_registry 注册 `items`/`on_select`，demo 需演示 `items={breadcrumb_items}` 绑定模式。

5. **Tabs 不拆分独立 demo**: 当前 tab_bar_case 已充分演示 Tabs + TabBar，无需单独 tabs_case。仅在文档中明确说明两者区别。

---

## 六、验证清单

- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine --lib` 全部通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] `<Input ref="name" placeholder="..." />` 编译通过且 placeholder 生效
- [ ] Dialog `on-ok` 返回 `false` 时对话框不关闭
- [ ] Dialog `slot="footer"` 元素注入正常渲染
- [ ] Skeleton/Breadcrumb demo 在 Demo 索引中可见
- [ ] Demo 描述文本中无 snake_case 属性名引用
- [ ] API 表格中事件属性行包含载荷类型
