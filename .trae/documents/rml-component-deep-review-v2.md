# RML gpui-component 深度审查与迭代计划 v2

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
3. **异味识别**: 双路径分裂、返回值丢弃、声明式缺口、文档不一致、方法名与属性名不匹配

---

## 二、发现汇总

### A. 架构设计缺陷（严重 — 破坏开发者体验）

#### A1. Input/TextInput PascalCase ref 路径不支持 placeholder/default_value/masked

**严重度**: 🔴 高 — 破坏声明式范式

**现象**:
```xml
<!-- ❌ 不工作：placeholder 走通用 setter 生成 .placeholder()，Input 组件无此方法 -->
<Input ref="username" placeholder="请输入用户名" />

<!-- ✅ 工作：value 路径通过 InputStateBridge 支持 placeholder -->
<Input value={username} placeholder="请输入用户名" />

<!-- ✅ 工作：NumberInput 有独立的 .placeholder() 方法 -->
<NumberInput ref="num" placeholder="请输入数字" />

<!-- ✅ 工作：Select 有独立的 .placeholder() 方法 -->
<Select ref="city" placeholder="请选择城市" />
```

**根因**:
- `placeholder` 在 `COMMON_STATIC_PROPS` 中（`props_registry.rs` 第 36 行），通用 `component_static_setter` 对所有组件生成 `.placeholder("...")`（`setters.rs` 第 63 行）
- Input 组件本身**没有** `.placeholder()` 方法 — placeholder 是 `InputState` 的 builder 方法
- Input 的 `state_ctor` 是固定闭包 `|w, c| rml_ui::InputState::new(w, c)`（`tags.rs` 第 344 行），不包含 builder 参数
- ref 路径走通用 Stateful translator，不会从属性提取 builder 参数注入 state_ctor
- `setters.rs` 第 14 行注释错误声称"Input 支持"，实际不支持
- 对比: **OtpInput 已有此模式**（`otp_input.rs` translator 提取 length/masked/default_value 注入 state_ctor），Input 未应用
- 对比: **NumberInput** 组件本身有 `.placeholder()` 方法，所以走通用 setter 能工作

**影响**:
- 开发者必须在 `on_loaded` 中手动 `cx.new(|cx| InputState::new(w, cx).placeholder("..."))` 创建 Entity
- 字段名必须硬编码为 `input_state`（tags.rs 中 state_field 固定）
- 破坏声明式范式 — 同一组件因 `ref` vs `value` 产生完全不同的能力
- Select/Combobox/DatePicker/NumberInput 支持 placeholder，Input 不支持 — 同类组件不一致
- `input_case.rml` 第 17 行明确文档化了此限制，等于将设计缺陷固化为"正常行为"

**修复方案**: 创建 `InputTranslator`（类似 `OtpInputTranslator`），提取 placeholder/default_value/masked 注入 state_ctor

---

#### A2. Dialog/AlertDialog on_ok/on_cancel 返回值被硬编码为 true

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
        return false; // ❌ 返回值被 codegen 丢弃，固定返回 true
    }
    true
}
```

**根因**:
- `cx.listener()` 产生 `Fn(&mut T, &ClickEvent, &mut Window, &mut Context<T>)`（无返回值）
- Dialog 的 `on_ok` 需要 `Fn(&ClickEvent, &mut Window, &mut App) -> bool`
- codegen 用 entity 捕获闭包绕过类型不匹配，但**固定返回 `true`**
- 代码位置: `dialog/setters.rs` 第 108-155 行，`true` 硬编码在第 123/135/149 行
- `alert_dialog/setters.rs` 同样存在此问题

**影响**:
- 无法实现表单验证失败时阻止关闭对话框
- 无法实现"保存中...请等待"异步关闭
- `on-cancel` 同理 — 取消前确认逻辑无法实现
- `dialog_case.rml` 第 79 行和 `alert_dialog_case.rml` 第 65 行将此限制文档化，等于将缺陷固化为"特性"

**修复方案**: 让 codegen 从 handler 方法返回值传递 bool，调整 handler 签名为 `Fn(&ClickEvent, &mut Context<Self>) -> bool`

---

#### A3. Dialog/AlertDialog footer 仅支持字符串，不支持 slot 元素注入

**严重度**: 🟡 中 — 限制 UI 表达力

**现象**:
```xml
<!-- ✅ footer 作为字符串属性工作 -->
<Dialog title="编辑" footer="确认要保存吗？">

<!-- ❌ footer 作为 slot 元素注入不支持 -->
<Dialog title="编辑">
    <Button slot="footer" label="保存" primary />
    <Button slot="footer" label="取消" />
</Dialog>
```

**根因**:
- `footer` 作为 Static 属性映射到 `.footer("字符串")`（`dialog/setters.rs` 第 23 行）
- Dialog 的 `.footer()` 接收 `impl IntoElement`，可以接收元素
- 但 codegen 只处理字符串属性，未支持 `slot="footer"` 元素注入
- Dialog gen.rs 的子节点处理仅路由 `slot="trigger"` → `.trigger()`，其余走 `.child()`

**影响**:
- 对话框操作按钮只能放在 content 区域，不符合常规 UI 模式（footer 居中/右对齐按钮）
- AlertDialog 的 confirm 按钮是内置的，但 Dialog 的自定义 footer 按钮无法实现

**修复方案**: 在 Dialog gen.rs 中支持 `slot="footer"` → `.footer(element)`，同 trigger slot 模式

---

#### A4. `placeholder` 在 COMMON_STATIC_PROPS 中对不支持的组件生成编译错误

**严重度**: 🟡 中 — 静默生成不可编译代码

**现象**: `placeholder` 列在 `COMMON_STATIC_PROPS`（第 36 行），通用 setter 对所有组件生成 `.placeholder("...")`。但 Input 组件无此方法，导致 Rust 编译错误而非 RML 解析时错误。

**根因**: `placeholder` 语义上不是"所有组件共享"的属性 — 它是特定组件（Select/Combobox/DatePicker/NumberInput/Avatar）的方法。将其放在 COMMON 中导致：
- 对有 `.placeholder()` 方法的组件：正常工作
- 对无 `.placeholder()` 方法的组件（Input/TextInput）：生成不可编译代码
- 错误延迟到 Rust 编译阶段，开发者难以定位原因

**修复方案**: 
- A1 修复后（InputTranslator 接管 Input/TextInput），此问题对 Input 不再存在
- 但应审查 `COMMON_STATIC_PROPS` 中其他属性是否有类似问题（`tooltip` 是否所有组件都支持？`label` 呢？）

---

### B. 一致性问题（中等）

#### B1. 缺失 Demo 案例

**严重度**: 🟡 中

| 组件 | 状态 | 说明 |
|------|------|------|
| Skeleton | ❌ 缺失 | 有 UI 组件（`crates/ui/src/components/skeleton.rs`）、有 props 注册（`secondary`），但无 demo |
| Breadcrumb | ❌ 缺失 | 有 UI 组件（`crates/ui/src/components/breadcrumb.rs`）、有 props 注册（`items`/`on_select`），但无 demo |

---

#### B2. Demo 描述文本中 snake_case 属性名引用（15+ 文件）

**严重度**: 🟡 中 — 误导开发者

**现象**: Demo 的 `description` 属性和 `<p>` 描述文本中使用 snake_case 引用属性名，与实际 RML kebab-case 不一致

**已确认的文件和行号**:

| 文件 | 行号 | snake_case 引用 | 应改为 |
|------|------|----------------|--------|
| `alert_dialog_case.rml` | 34 | `close_button=false`、`overlay_closable=false` | `close-button`、`overlay-closable` |
| `alert_dialog_case.rml` | 63 | `close_button=false + overlay_closable=false` | `close-button`、`overlay-closable` |
| `alert_dialog_case.rml` | 65 | `on_ok / on_cancel` | `on-ok / on-cancel` |
| `dialog_case.rml` | 79 | `on_ok / on_cancel` | `on-ok / on-cancel` |
| `button_case.rml` | 85 | `font_bold / font_semibold / font_medium` | `font-bold / font-semibold / font-medium` |
| `button_case.rml` | 116 | `font_bold / font_semibold / font_medium / font_light` | `font-bold / font-semibold / font-medium / font-light` |
| `date_picker_case.rml` | 4 | `number_of_months`、`on_change` | `number-of-months`、`on-change` |
| `color_picker_case.rml` | 4 | `on_change` | `on-change` |
| `calendar_case.rml` | 4 | `on_select` | `on-select` |
| `combobox_case.rml` | 4 | `on_change` | `on-change` |
| `number_input_case.rml` | 4 | `on_change/on_enter/on_focus/on_blur` | `on-change/on-enter/on-focus/on-blur` |
| `input_case.rml` | 4 | `on_change/on_enter/on_focus/on_blur` | `on-change/on-enter/on-focus/on-blur` |
| `select_case.rml` | 4 | `on_change` | `on-change` |
| `sidebar_case.rml` | 4 | `default_open/click_to_open/click_to_toggle` | `default-open/click-to-open/click-to-toggle` |
| `sidebar_case.rml` | 97 | `disabled` | `disabled`（此条正确，disabled 无连字符） |

---

#### B3. SidebarMenuItem `disabled` 属性映射到 `.disable()` 方法（方法名不匹配）

**严重度**: 🟡 中 — 开发者心智模型不一致

**现象**:
```xml
<!-- RML 属性名为 disabled（与所有其他组件一致） -->
<SidebarMenuItem label="禁用项" disabled="true" />
```

**生成的代码**:
```rust
// 方法名为 .disable()，而非 .disabled() — 与其他组件不一致
SidebarMenuItem::new("禁用项").disable(true)
```

**根因**: `props_registry.rs` 第 282 行注释: "disabled → .disable(bool)（注意方法名）"。SidebarMenuItem 的底层 gpui-component 组件方法名为 `.disable()` 而非 `.disabled()`。

**影响**:
- 所有其他组件（Button/Checkbox/Input 等）的 disabled 属性映射到 `.disabled()` 方法
- SidebarMenuItem 映射到 `.disable()` — 开发者不知道此差异
- 虽然 codegen 层面已处理（专用 setter），但心智模型不一致

**修复方案**: 此为底层 gpui-component API 不一致。codegen 层已正确处理，在 demo 文档中明确标注即可。若底层可改，统一为 `.disabled()` 更佳。

---

#### B4. Input placeholder 能力不一致（A1 的延伸）

**严重度**: 🔴 高（与 A1 同根因）

| 组件 | ref 路径 placeholder | value 路径 placeholder | 实现方式 |
|------|---------------------|----------------------|---------|
| Input | ❌ 不支持 | ✅ 支持 | value 路径走 InputStateBridge |
| TextInput | ❌ 不支持 | ✅ 支持 | 同 Input |
| NumberInput | ✅ 支持 | ✅ 支持 | 组件有 `.placeholder()` 方法 |
| Select | ✅ 支持 | N/A | 组件有 `.placeholder()` 方法 |
| Combobox | ✅ 支持 | N/A | 组件有 `.placeholder()` 方法 |
| DatePicker | ✅ 支持 | N/A | 组件有 `.placeholder()` 方法 |

---

### C. 开发者体验改进（轻度）

#### C1. Dialog/AlertDialog 演示缺失 on-ok/on-cancel 事件处理

**严重度**: 🟢 低

- `dialog_case.rml` 完全没有演示 on-ok/on-cancel 事件
- `alert_dialog_case.rml` 也没有演示 on-ok/on-cancel 事件
- 开发者无法从 demo 中学习如何处理确认/取消逻辑
- API 表格标注了 `Fn(&ClickEvent, &mut Window, &mut App) -> bool` 签名，但未演示

---

#### C2. 事件载荷类型未在 API 表格中文档化

**严重度**: 🟢 低

不同组件的事件传递不同载荷类型，API 表格中仅标注 "event"：

| 组件 | 事件 | 实际载荷类型 | API 表格标注 |
|------|------|------------|-------------|
| Select | on-change | `Option<SharedString>` | "event" |
| Combobox | on-change | `Vec<SharedString>` | "event" |
| Calendar | on-select | `Date` | "event" |
| ColorPicker | on-change | `Option<Hsla>` | "event" |
| Accordion | on-toggle-click | `&[usize]` | "event" |
| Checkbox | on-click | `&bool` | "event" |
| Rating | on-click | `&usize` | "event" |
| Pagination | on-click | `&usize` | "event" |
| Stepper | on-click | `&usize` | "event" |
| Slider | on-change | `f32` | "event" |
| Input | on-change | `&Entity<InputState>` | "event" |
| Tree | on-activate | `usize` | "event" |
| Tree | on-select | `usize` | "event" |

---

#### C3. NumberInput 与 Input 共享 input_state 字段名

**严重度**: 🟢 低

- NumberInput 的 `state_field` 也是 `input_state`（`tags.rs` 第 363 行，与 Input 第 344 行相同）
- 同一 View 同时使用 Input 和 NumberInput 时，字段名冲突
- 需手动管理 Entity 或拆分到子 View

---

#### C4. Demo 描述包含 codegen 实现细节（无关信息泄露）

**严重度**: 🟢 低

**现象**: 多个 demo 的 description 属性包含 codegen 实现细节，这些信息对组件使用者无价值，反而增加认知负担。

**示例**:
- `dialog_case.rml` 第 4 行: "codegen 直接使用 render 上下文的 cx 变量，不分配 ElementId"
- `alert_dialog_case.rml` 第 65 行: "codegen 使用 entity 捕获闭包固定返回 true"
- `input_case.rml` 第 4 行: "由 __rml_state.get_or_init_ref 惰性创建，由 __rml_populate_refs 注入"
- `select_case.rml` 第 4 行: "SelectState::new(delegate, None, window, cx) 创建状态"

**影响**: 开发者不需要知道 codegen 如何实现，只需知道声明式用法。实现细节泄露违反渐进式披露原则。

**修复方案**: 将 description 精简为组件用途 + 声明式用法说明，移除 codegen/内部实现细节。

---

#### C5. `setters.rs` 注释错误声称 Input 支持 placeholder

**严重度**: 🟢 低 — 文档错误

**位置**: `crates/engine/src/compiler/setters.rs` 第 14 行
```
/// - `placeholder="..."` → `.placeholder("...")`（Input 支持）
```

**实际**: Input 组件无 `.placeholder()` 方法，该注释应删除或更正为"NumberInput/Select/Combobox 等支持"。

---

## 三、迭代计划

### Phase R1: Input 声明式完整性修复（A1/A4/B4 — 最高优先级）

**目标**: 创建 `InputTranslator`，统一 ref/value 路径的 placeholder/default_value/masked 支持

**变更文件**:

1. **新建 `crates/engine/src/compiler/translator/component/input.rs`**
   - 参考 `otp_input.rs` translator 模式（已在上下文中提供完整实现）
   - 提取 `placeholder`（Static/Bind）、`default_value`（Static）、`masked`（Static bool）属性
   - 构建自定义 state_ctor: `|w, c| rml_ui::InputState::new(w, c).placeholder("...").masked(true).default_value("...")`
   - SKIP_ATTRS = `["placeholder", "default_value", "masked"]`
   - 调用 `gen_stateful_body` 生成构造表达式
   - 剩余属性走通用 setter 分发
   - `matches()` 方法匹配 canonical_tag 为 "Input" 或 "TextInput"

2. **修改 `crates/engine/src/compiler/translator/component/mod.rs`**
   - 添加 `pub mod input;`
   - 在 `register_all()` 中添加 `input::register(registry);`
   - 注意：需确保 input translator 在 stateful 之前注册（translator 优先级按注册顺序）

3. **修改 `crates/engine/src/compiler/translator/component/stateful.rs`**
   - 在 `matches()` 方法的排除列表中添加 "Input" | "TextInput"（第 37 行）
   - 当前排除: `matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput")`
   - 改为: `matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput" | "Input" | "TextInput")`

4. **修复 `crates/engine/src/compiler/setters.rs` 第 14 行注释**
   - 将 `（Input 支持）` 改为 `（NumberInput/Select/Combobox 等组件支持）` 或删除

5. **更新 `demo/src/cases/input_case.rml`**
   - Section 2 改为演示 `<Input ref="name" placeholder="请输入用户名" />` 直接使用
   - 移除 Pattern B 手动创建 InputState 的说明
   - 补充 default_value/masked 演示
   - 修复 description 中 snake_case（B2 修复）

6. **更新 `demo/src/cases/input_case.rml.rs`**
   - 移除手动创建 InputState 的 on_loaded 代码（如存在）
   - 简化字段定义

**验证**:
- `cargo test -p rust-rml-engine -- input` 通过
- `cargo build -p rust-rml-demo` 成功
- `<Input ref="name" placeholder="..." />` 编译通过且 placeholder 生效
- `<Input ref="name" masked />` 编译通过且输入被遮罩

---

### Phase R2: Dialog 交互能力修复（A2/A3 — 高优先级）

**目标**: 恢复 on_ok/on_cancel 返回值控制；支持 footer 元素注入

**变更文件**:

1. **修改 `crates/engine/src/compiler/components/dialog/setters.rs`**
   - `bool_event_setter()` 修改：handler 方法返回 `bool`，codegen 传递返回值
   - 生成代码变更:
     - 当前: `entity.update(cx, |this, cx| { this.on_ok(&rml_ev, cx); }); true`
     - 改为: `entity.update(cx, |this, cx| { this.on_ok(&rml_ev, cx) })` — 返回值直接作为闭包返回
   - 需要调整 handler 签名为 `Fn(&ClickEvent, &mut Context<Self>) -> bool`
   - 涉及第 116-155 行的三个 match 分支（Ident/MethodName、WithArgs空参、WithArgs有参）

2. **修改 `crates/engine/src/compiler/components/dialog/gen.rs`**
   - 在子节点处理中增加 `slot="footer"` → `.footer(element)` 路由
   - 同 `slot="trigger"` → `.trigger()` 模式
   - 若同时存在 `footer="字符串"` 属性和 `slot="footer"` 元素，slot 覆盖属性

3. **修改 `crates/engine/src/compiler/components/alert_dialog/setters.rs`**
   - 同步 on_ok/on_cancel 返回值修复（与 Dialog 相同的 `bool_event_setter` 模式）

4. **修改 `crates/engine/src/compiler/components/alert_dialog/gen.rs`**
   - 同步 footer slot 支持（如 AlertDialog 有 footer 方法）

5. **更新 `demo/src/cases/dialog_case.rml`**
   - 添加 on-ok/on-cancel 演示（表单验证场景：验证失败返回 false 阻止关闭）
   - 添加 `slot="footer"` 按钮演示
   - 修复 description 中 snake_case（B2 修复）
   - 移除"固定返回 true"的说明

6. **更新 `demo/src/cases/dialog_case.rml.rs`**
   - 添加 on_ok/on_cancel handler 方法（返回 bool）
   - 添加表单验证逻辑演示

7. **更新 `demo/src/cases/alert_dialog_case.rml` + `.rml.rs`**
   - 添加 on-ok/on-cancel 演示
   - 修复 snake_case（B2 修复）
   - 移除"固定返回 true"的说明

**验证**:
- `cargo test -p rust-rml-engine -- dialog` 通过
- `cargo build -p rust-rml-demo` 成功
- on-ok 返回 false 时对话框不关闭
- `slot="footer"` 元素注入正常渲染

---

### Phase R3: 缺失 Demo 补全（B1 — 中优先级）

**目标**: 为 Skeleton 和 Breadcrumb 创建 demo 案例

**变更文件**:

1. **新建 `demo/src/cases/skeleton_case.rml`**
   - 演示基础骨架屏
   - 演示 secondary 变体（次级颜色）
   - 演示与 Card 组合的加载占位场景
   - 演示不同宽度/高度的骨架屏

2. **新建 `demo/src/cases/skeleton_case.rml.rs`**
   - `SkeletonCase` 结构体
   - API 表格（secondary 属性）
   - `#[contribute]` 注册

3. **新建 `demo/src/cases/breadcrumb_case.rml`**
   - 演示 items 绑定模式
   - 演示 on-select 事件回调
   - 演示面包屑导航场景

4. **新建 `demo/src/cases/breadcrumb_case.rml.rs`**
   - `BreadcrumbCase` 结构体
   - breadcrumb_items 数据
   - API 表格（items/on-select 属性）
   - `#[contribute]` 注册

5. **修改 `demo/src/cases/mod.rs`**
   - 添加 `#[path = "skeleton_case.rml.rs"] pub mod skeleton_case;`
   - 添加 `#[path = "breadcrumb_case.rml.rs"] pub mod breadcrumb_case;`

6. **更新 i18n 文件**
   - `demo/assets/i18n/zh-CN.json`: 添加 `"case.skeleton.title"` 和 `"case.breadcrumb.title"`
   - `demo/assets/i18n/en-US.json`: 同上

**验证**:
- `cargo build -p rust-rml-demo` 成功
- 新 demo 在 Demo 索引中可见
- Skeleton secondary 变体视觉差异可辨
- Breadcrumb items 绑定和 on-select 事件正常工作

---

### Phase R4: 文档一致性修复（B2/C1/C2/C4/C5 — 低优先级）

**目标**: 修复文档 snake_case 不一致；补充事件载荷类型；清理 codegen 实现细节；完善 Dialog demo

**变更文件**:

1. **批量修改 demo .rml 文件中的描述文本（B2 修复）**
   - 将 `<p>` 和 `description` 属性中的 snake_case 属性名改为 kebab-case
   - 涉及约 15 个文件（详见 B2 表格）
   - 重点: `close_button` → `close-button`、`overlay_closable` → `overlay-closable`、`on_change` → `on-change`、`on_ok` → `on-ok`、`font_bold` → `font-bold` 等

2. **更新各 demo 的 API 表格（C2 修复）**
   - 在事件属性行补充载荷类型
   - 涉及文件和更新内容:
     - `select_case.rml.rs`: `("on-change", "event (Option<SharedString>)", ...)`
     - `combobox_case.rml.rs`: `("on-change", "event (Vec<SharedString>)", ...)`
     - `calendar_case.rml.rs`: `("on-select", "event (Date)", ...)`
     - `color_picker_case.rml.rs`: `("on-change", "event (Option<Hsla>)", ...)`
     - `accordion_case.rml.rs`: `("on-toggle-click", "event (&[usize])", ...)`
     - `checkbox_case.rml.rs`: `("on-click", "event (&bool)", ...)`
     - `rating_case.rml.rs`: `("on-click", "event (&usize)", ...)`
     - `pagination_case.rml.rs`: `("on-click", "event (&usize)", ...)`
     - `stepper_case.rml.rs`: `("on-click", "event (&usize)", ...)`
     - `slider_case.rml.rs`: `("on-change", "event (f32)", ...)`
     - `input_case.rml.rs`: `("on-change", "event (&Entity<InputState>)", ...)`
     - `tree_case.rml.rs`: `("on-activate", "event (usize)", ...)`、`("on-select", "event (usize)", ...)`

3. **清理 demo description 中的 codegen 实现细节（C4 修复）**
   - 移除描述中的 codegen 内部实现引用
   - 保留组件用途 + 声明式用法说明
   - 涉及文件: `dialog_case.rml`、`alert_dialog_case.rml`、`input_case.rml`、`select_case.rml` 等

4. **修复 `setters.rs` 注释（C5 修复）**
   - 第 14 行: `（Input 支持）` → `（NumberInput/Select/Combobox 等支持）`

**验证**:
- 全文搜索 `_[a-z]+` 在 .rml 文件的 description 和 `<p>` 中无属性名引用（kebab-case 形式）
- API 表格中事件属性行包含载荷类型
- Demo description 不包含 codegen/internal 实现细节

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
| R4 | P3 低 | 3-4h | R1, R2 |

---

## 五、假设与决策

1. **InputTranslator 接管 Input/TextInput**: stateful.rs 中的通用 Stateful translator 需排除 Input/TextInput，由 input translator 优先匹配。NumberInput 不接管 — NumberInput 组件本身有 `.placeholder()` 方法，走通用 setter 能正常工作。

2. **Dialog on_ok 签名调整**: 当前 handler 签名为 `Fn(&ClickEvent, &mut Context<Self>)`，需改为返回 `bool`。这是破坏性变更（现有 handler 需添加返回值），但无兼容性设计原则支持。

3. **footer slot 与 footer 属性共存**: `footer="字符串"` 和 `<Button slot="footer" />` 可共存 — 属性优先（简单场景），slot 覆盖（复杂场景）。若同时存在，slot 覆盖属性。

4. **Breadcrumb items 为 bind 属性**: Breadcrumb 已在 props_registry 注册 `items`/`on_select`，demo 需演示 `items={breadcrumb_items}` 绑定模式。

5. **Tabs 不拆分独立 demo**: 当前 tab_bar_case 已充分演示 Tabs + TabBar，无需单独 tabs_case。仅在文档中明确说明两者区别。

6. **SidebarMenuItem disabled → .disable() 不改**: 此为底层 gpui-component API 不一致，codegen 层已正确处理。仅在 demo 文档中明确标注方法名差异。

7. **COMMON_STATIC_PROPS 审查暂不扩展**: A4 指出 `placeholder` 在 COMMON 中对不支持的组件生成编译错误。R1 修复后 Input 问题解决。其他通用属性（`tooltip`/`label`）的类似问题留待后续审查，本次不扩展范围。

---

## 六、验证清单

- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine --lib` 全部通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] `<Input ref="name" placeholder="..." />` 编译通过且 placeholder 生效
- [ ] `<Input ref="name" masked />` 编译通过且输入被遮罩
- [ ] Dialog `on-ok` 返回 `false` 时对话框不关闭
- [ ] Dialog `slot="footer"` 元素注入正常渲染
- [ ] Skeleton/Breadcrumb demo 在 Demo 索引中可见
- [ ] Demo 描述文本中无 snake_case 属性名引用
- [ ] API 表格中事件属性行包含载荷类型
- [ ] Demo description 不包含 codegen 内部实现细节
- [ ] `setters.rs` 第 14 行注释修正
