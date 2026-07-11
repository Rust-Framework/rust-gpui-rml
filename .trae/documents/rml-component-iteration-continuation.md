# RML gpui-component 迭代计划 — 续接（Phase R2/R3/R4）

> 创建日期：2026-07-11
> 前置文档：`.trae/documents/rml-component-deep-review-v2.md`（已批准的深度审查与迭代计划 v2）
> 当前状态：Phase R1 已实现完成（InputTranslator 创建、注册、demo 更新、engine 1253 测试通过），待 demo 编译验证后进入 R2

---

## 一、当前状态确认

### Phase R1 完成情况（已实现，待验证）

| 变更项 | 文件 | 状态 |
|--------|------|------|
| InputTranslator 创建 | `crates/engine/src/compiler/translator/component/input.rs` | ✅ 已创建 |
| mod.rs 注册 | `crates/engine/src/compiler/translator/component/mod.rs` | ✅ input::register 在 stateful::register 之前 |
| stateful.rs 排除 | `crates/engine/src/compiler/translator/component/stateful.rs` L37 | ✅ 排除 Input/TextInput |
| setters.rs 注释修复 | `crates/engine/src/compiler/setters.rs` L14 | ✅ 已修正 |
| input_case.rml 更新 | `demo/src/cases/input_case.rml` | ✅ 已更新 |
| input_case.rml.rs 更新 | `demo/src/cases/input_case.rml.rs` | ✅ 已更新 |
| engine 编译 + 测试 | `cargo build/test -p rust-rml-engine` | ✅ 1253 测试通过 |
| **demo 编译验证** | `cargo build -p rust-rml-demo` | ⏳ **待执行** |

**R1 验证步骤**: `cargo build -p rust-rml-demo` 成功即 R1 完成。

---

## 二、Phase R2: Dialog 交互能力修复（A2 + A3）

### 问题概述

**A2 — on_ok/on_cancel 返回值硬编码 true**:
- `dialog/setters.rs` L108-155 的 `bool_event_setter()` 生成代码中，handler 方法返回值被分号丢弃，闭包固定返回 `true`
- `alert_dialog/setters.rs` L124-171 存在相同问题
- 导致：无法实现表单验证失败时阻止关闭对话框

**A3 — footer 仅支持字符串，不支持 slot 元素注入**:
- `dialog/gen.rs` 子节点处理仅路由 `slot="trigger"` → `.trigger()`，无 `slot="footer"` → `.footer()` 路由
- `alert_dialog/gen.rs` 同样缺失
- 导致：对话框操作按钮只能放在 content 区域

### 变更清单

#### 2.1 修复 `dialog/setters.rs` — bool_event_setter 返回值传递

**文件**: `crates/engine/src/compiler/components/dialog/setters.rs`

**当前代码模式**（L116-155，三个 match 分支均相同）:
```rust
entity.update(cx, |this, cx| {
    this.{}(&rml_ev, cx);   // ← 分号丢弃返回值
});
true                         // ← 硬编码 true
```

**修改为**:
```rust
entity.update(cx, |this, cx| {
    this.{}(&rml_ev, cx)    // ← 无分号，返回 bool
})                           // ← 无分号，entity.update 返回 bool 作为闭包返回值
```

**涉及三个 match 分支**:
1. `EventHandler::Ident(_) | EventHandler::MethodName(_)`（L116-126）
2. `EventHandler::WithArgs(_, args) if args.is_empty()`（L128-138）
3. `EventHandler::WithArgs(_, args)` 带参数（L139-153）

**同步更新文件头注释**（L14-15）:
- `返回 true` → `返回 handler 方法的 bool 返回值`

**同步更新 `bool_event_setter` 函数注释**（L102-107）:
- 移除"固定返回 `true`"说明
- 改为"传递 handler 方法的 bool 返回值"

#### 2.2 修复 `alert_dialog/setters.rs` — 同步 bool_event_setter

**文件**: `crates/engine/src/compiler/components/alert_dialog/setters.rs`

与 `dialog/setters.rs` 完全相同的修改模式（L124-171 的三个 match 分支）。

同步更新文件头注释（L24-25）和函数注释（L123）。

#### 2.3 Dialog gen.rs — 支持 footer slot 元素注入

**文件**: `crates/engine/src/compiler/components/dialog/gen.rs`

**当前子节点处理**（L70-99）:
- `slot="trigger"` → `.trigger(element)`
- 其余 → `.child(element)` / `.children(iterator)`

**修改为**:
- `slot="trigger"` → `.trigger(element)`（不变）
- `slot="footer"` → `.footer(element)`（新增）
- 其余 → `.child(element)` / `.children(iterator)`（不变）

**具体变更**:
- 新增 `let mut footer_code: Option<String> = None;`
- 在子节点遍历 match 中增加 `slot="footer"` 分支
- 多个 footer slot 报错（同 trigger 的"exactly one"模式）
- 在 trigger 注入后、content 注入前添加 footer 注入: `code.push_str(&format!("\n            .footer({})", fc));`
- 若同时存在 `footer="字符串"` 属性和 `slot="footer"` 元素，slot 覆盖属性（slot 在属性之后注入，覆盖先前的 `.footer("...")` 调用）

**新增测试**:
- `gen_dialog_with_footer_slot` — 验证 `slot="footer"` 元素生成 `.footer(element)`
- `gen_dialog_multiple_footers_error` — 验证多个 footer slot 报错

#### 2.4 AlertDialog gen.rs — 同步 footer slot 支持

**文件**: `crates/engine/src/compiler/components/alert_dialog/gen.rs`

与 `dialog/gen.rs` 完全相同的修改模式。

**新增测试**:
- `gen_alert_dialog_with_footer_slot`
- `gen_alert_dialog_multiple_footers_error`

#### 2.5 更新 dialog_case demo — 演示 on-ok/on-cancel + footer slot

**文件**: `demo/src/cases/dialog_case.rml`

1. **修复 description（B2）**: 移除 `on_close/on_ok/on_cancel` 中的 snake_case → kebab-case
2. **移除 codegen 实现细节（C4）**: 移除"codegen 直接使用 render 上下文的 cx 变量，不分配 ElementId"
3. **新增 Section: on-ok/on-cancel 事件演示**:
   ```xml
   <Dialog title="表单验证" on-ok={on_validate_ok} on-cancel={on_validate_cancel}>
       <Button slot="trigger" label="打开验证对话框" />
       <Input ref="validate_input" placeholder="请输入内容（留空则阻止关闭）" />
   </Dialog>
   ```
4. **新增 Section: footer slot 演示**:
   ```xml
   <Dialog title="自定义页脚">
       <Button slot="trigger" label="打开 footer slot 对话框" />
       <div class="dialog-tip">使用 slot=footer 注入自定义按钮</div>
       <Button slot="footer" label="自定义保存" primary />
   </Dialog>
   ```
5. **修复 API 区注释**: 移除"on_ok / on_cancel 回调返回 bool...codegen 使用 entity 捕获闭包固定返回 true"说明，改为"on-ok/on-cancel 返回 false 可阻止关闭"

**文件**: `demo/src/cases/dialog_case.rml.rs`

1. **新增字段**: `validate_input: ElementRef<rml_ui::InputState>`
2. **新增 handler 方法**:
   ```rust
   #[command]
   pub fn on_validate_ok(&mut self, _: &ClickEvent, cx: &mut Context<Self>) -> bool {
       // 验证逻辑：输入为空时返回 false 阻止关闭
       let value = self.validate_input.read(cx).value().to_string();
       if value.is_empty() {
           return false;
       }
       true
   }

   #[command]
   pub fn on_validate_cancel(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
       true // 取消总是关闭
   }
   ```
3. **更新 API 表格**: `on-ok` 行补充"返回 false 阻止关闭"；`footer` 行补充"也支持 slot=footer 元素注入"

#### 2.6 更新 alert_dialog_case demo — 演示 on-ok/on-cancel

**文件**: `demo/src/cases/alert_dialog_case.rml`

1. **修复 description（B2）**: `close_button=false + overlay_closable=false` → `close-button=false + overlay-closable=false`；`on_ok / on_cancel` → `on-ok / on-cancel`
2. **修复 L34**: 已使用 kebab-case（`close-button=true`、`overlay-closable=true`），确认正确
3. **新增 Section: on-ok/on-cancel 演示**:
   ```xml
   <AlertDialog title="确认操作" description="点击 OK 关闭，点击 Cancel 也关闭" confirm
       on-ok={on_alert_ok} on-cancel={on_alert_cancel}>
       <Button slot="trigger" label="事件回调演示" />
   </AlertDialog>
   ```
4. **修复 API 区注释**: 移除"on_ok / on_cancel...固定返回 true"说明

**文件**: `demo/src/cases/alert_dialog_case.rml.rs`

1. **新增 handler 方法**:
   ```rust
   #[command]
   pub fn on_alert_ok(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
       true
   }

   #[command]
   pub fn on_alert_cancel(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) -> bool {
       true
   }
   ```
2. **更新 API 表格**: `on-ok`/`on-cancel` 行补充"返回 false 阻止关闭"

### R2 验证

- `cargo test -p rust-rml-engine -- dialog` 通过（含新增 footer slot 测试）
- `cargo test -p rust-rml-engine -- alert_dialog` 通过
- `cargo build -p rust-rml-demo` 成功
- 代码审查确认：生成的 `.on_ok(...)` 闭包返回 `entity.update(cx, |this, cx| this.method(...))` 而非硬编码 `true`

---

## 三、Phase R3: 缺失 Demo 补全（B1）

### 3.1 Skeleton Demo

**参考组件**: `crates/ui/src/components/skeleton.rs`
**Props 注册**: `props_registry.rs` L205: `("Skeleton", &["secondary"])`
**setters.rs**: L66-72: `secondary="" → .secondary()`

**新建 `demo/src/cases/skeleton_case.rml`**:
- Section 1: 基础骨架屏（矩形占位）
- Section 2: secondary 次级颜色变体
- Section 3: 与 Card 组合的加载占位场景
- Section 4: 不同尺寸的骨架屏（通过 style 控制宽度高度）

**新建 `demo/src/cases/skeleton_case.rml.rs`**:
- `SkeletonCase` 结构体，`#[contribute]` 注册（order 在 200+ 范围，避开已有）
- API 表格: `secondary`（布尔，次级颜色）
- `rml_sample` / `rust_sample` computed 方法

### 3.2 Breadcrumb Demo

**参考组件**: `crates/ui/src/components/breadcrumb.rs`
**Props 注册**: `props_registry.rs` L196: `("Breadcrumb", &["items", "on_select"])`
**setters.rs**: L324+: Breadcrumb items → `.items(Vec<BreadcrumbItem>.clone())`

**新建 `demo/src/cases/breadcrumb_case.rml`**:
- Section 1: 基础面包屑导航（items 绑定）
- Section 2: on-select 事件回调
- Section 3: 带图标的面包屑

**新建 `demo/src/cases/breadcrumb_case.rml.rs`**:
- `BreadcrumbCase` 结构体
- `breadcrumb_items: Vec<rml_ui::BreadcrumbItem>` 字段
- on_loaded 中初始化 breadcrumb_items 数据
- `on_breadcrumb_select` handler 方法
- API 表格: `items`（绑定，Vec<BreadcrumbItem>）、`on-select`（event，&usize）

### 3.3 注册新 Demo

**修改 `demo/src/cases/mod.rs`**:
```rust
#[path = "skeleton_case.rml.rs"]
pub mod skeleton_case;
#[path = "breadcrumb_case.rml.rs"]
pub mod breadcrumb_case;
```

**修改 i18n 文件**:
- `demo/assets/i18n/zh-CN.json`: 添加 `"case.skeleton.title": "骨架屏 Skeleton"` 和 `"case.breadcrumb.title": "面包屑导航 Breadcrumb"`
- `demo/assets/i18n/en-US.json`: 添加 `"case.skeleton.title": "Skeleton"` 和 `"case.breadcrumb.title": "Breadcrumb"`

### R3 验证

- `cargo build -p rust-rml-demo` 成功
- Skeleton/Breadcrumb demo 在 Demo 索引中可见
- Skeleton secondary 变体视觉差异可辨
- Breadcrumb items 绑定和 on-select 事件正常工作

---

## 四、Phase R4: 文档一致性修复（B2 + C2 + C4）

### 4.1 批量修复 snake_case 属性名引用（B2）

**涉及文件和具体修改**:

| 文件 | snake_case 引用 | 改为 |
|------|----------------|------|
| `alert_dialog_case.rml` L4 | `close_button=false`、`overlay_closable=false` | `close-button=false`、`overlay-closable=false` |
| `alert_dialog_case.rml` L65 | `on_ok / on_cancel` | `on-ok / on-cancel` |
| `dialog_case.rml` L79 | `on_ok / on_cancel` | `on-ok / on-cancel` |
| `button_case.rml` L85 | `font_bold / font_semibold / font_medium` | `font-bold / font-semibold / font-medium` |
| `button_case.rml` L116 | `font_bold / font_semibold / font_medium / font_light` | `font-bold / font-semibold / font-medium / font-light` |
| `date_picker_case.rml` L4 | `number_of_months`、`on_change` | `number-of-months`、`on-change` |
| `color_picker_case.rml` L4 | `on_change` | `on-change` |
| `calendar_case.rml` L4 | `on_select` | `on-select` |
| `combobox_case.rml` L4 | `on_change` | `on-change` |
| `number_input_case.rml` L4 | `on_change/on_enter/on_focus/on_blur` | `on-change/on-enter/on-focus/on-blur` |
| `input_case.rml` L4 | `on_change/on_enter/on_focus/on_blur` | `on-change/on-enter/on-focus/on-blur` |
| `select_case.rml` L4 | `on_change` | `on-change` |
| `sidebar_case.rml` L4 | `default_open/click_to_open/click_to_toggle` | `default-open/click-to-open/click-to-toggle` |

**注**: R2 中 dialog_case.rml 和 alert_dialog_case.rml 的 snake_case 修复已包含在 R2 变更中，此处不重复。

### 4.2 补充事件载荷类型到 API 表格（C2）

**涉及 .rml.rs 文件和更新内容**:

| 文件 | 事件属性 | 当前标注 | 改为 |
|------|---------|---------|------|
| `select_case.rml.rs` | on-change | "event" | "event (Option<SharedString>)" |
| `combobox_case.rml.rs` | on-change | "event" | "event (Vec<SharedString>)" |
| `calendar_case.rml.rs` | on-select | "event" | "event (Date)" |
| `color_picker_case.rml.rs` | on-change | "event" | "event (Option<Hsla>)" |
| `accordion_case.rml.rs` | on-toggle-click | "event" | "event (&[usize])" |
| `checkbox_case.rml.rs` | on-click | "event" | "event (&bool)" |
| `rating_case.rml.rs` | on-click | "event" | "event (&usize)" |
| `pagination_case.rml.rs` | on-click | "event" | "event (&usize)" |
| `stepper_case.rml.rs` | on-click | "event" | "event (&usize)" |
| `slider_case.rml.rs` | on-change | "event" | "event (f32)" |
| `input_case.rml.rs` | on-change | 已有标注 | 确认为 "event (&Entity<InputState>)" |
| `tree_case.rml.rs` | on-activate | "event" | "event (usize)" |
| `tree_case.rml.rs` | on-select | "event" | "event (usize)" |

### 4.3 清理 description 中的 codegen 实现细节（C4）

**涉及文件和清理内容**:

| 文件 | 移除内容 |
|------|---------|
| `dialog_case.rml` L4 | "codegen 直接使用 render 上下文的 cx 变量，不分配 ElementId" |
| `dialog_case.rml` L78 | "Dialog 构造器为 Dialog::new(cx: &mut App)，codegen 直接使用 render 上下文的 cx 变量，不分配 ElementId" |
| `dialog_case.rml` L79 | "on_ok / on_cancel 回调返回 bool（true 关闭对话框），codegen 使用 entity 捕获闭包固定返回 true" |
| `alert_dialog_case.rml` L4 | "构造器 AlertDialog::new(cx: &mut App)" 中的 codegen 引用 |
| `alert_dialog_case.rml` L65 | "on_ok / on_cancel 回调返回 bool（true 关闭对话框），codegen 使用 entity 捕获闭包固定返回 true" |
| `input_case.rml` L4 | "由 __rml_state.get_or_init_ref 惰性创建，由 __rml_populate_refs 注入" |
| `select_case.rml` L4 | "SelectState::new(delegate, None, window, cx) 创建状态" |

**注**: dialog_case.rml 和 alert_dialog_case.rml 的 C4 清理已包含在 R2 变更中。

### R4 验证

- 全文搜索 `_[a-z]+` 在 .rml 文件的 description 和 `<p>` 中无属性名引用
- API 表格中事件属性行包含载荷类型
- Demo description 不包含 codegen/internal 实现细节

---

## 五、执行顺序与依赖

```
Step 1: R1 验证（cargo build -p rust-rml-demo）
  ↓
Step 2: R2 实现（Dialog/AlertDialog 交互修复）
  ├─ 2.1 dialog/setters.rs 修复
  ├─ 2.2 alert_dialog/setters.rs 修复
  ├─ 2.3 dialog/gen.rs footer slot
  ├─ 2.4 alert_dialog/gen.rs footer slot
  ├─ 2.5 dialog_case demo 更新
  └─ 2.6 alert_dialog_case demo 更新
  ↓
Step 3: R3 实现（缺失 Demo）
  ├─ 3.1 Skeleton demo
  ├─ 3.2 Breadcrumb demo
  └─ 3.3 注册 + i18n
  ↓
Step 4: R4 实现（文档一致性）
  ├─ 4.1 snake_case 修复
  ├─ 4.2 事件载荷类型补充
  └─ 4.3 codegen 细节清理
  ↓
Step 5: 最终验证
  ├─ cargo build -p rust-rml-engine
  ├─ cargo test -p rust-rml-engine --lib
  ├─ cargo build -p rust-rml-demo
  └─ 代码审查
```

---

## 六、假设与决策

1. **on_ok/on_cancel handler 签名变更为返回 bool**: 这是破坏性变更，现有无返回值的 handler 需添加 `-> bool` 返回类型。遵循"无兼容性设计"原则，不保留旧签名。

2. **footer slot 覆盖 footer 属性**: 若同时存在 `footer="字符串"` 属性和 `slot="footer"` 元素，slot 在属性之后注入，自然覆盖。不需特殊处理。

3. **Skeleton/Breadcrumb demo order**: 使用 200+ 范围的 order 值，避免与已有 demo 冲突。Skeleton order=210，Breadcrumb order=211。

4. **Breadcrumb items 类型**: 使用 `Vec<rml_ui::BreadcrumbItem>`，在 on_loaded 中初始化数据。

5. **input_case.rml 的 C4 清理**: R1 中 input_case.rml 已更新，description 中可能仍有 codegen 引用，R4 统一清理。

---

## 七、验证清单

- [ ] `cargo build -p rust-rml-engine` 成功
- [ ] `cargo test -p rust-rml-engine --lib` 全部通过
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] Dialog `on-ok` 返回 `false` 时对话框不关闭
- [ ] Dialog `slot="footer"` 元素注入正常渲染
- [ ] AlertDialog `on-ok`/`on-cancel` 事件正常触发
- [ ] Skeleton demo 在 Demo 索引中可见
- [ ] Breadcrumb demo 在 Demo 索引中可见
- [ ] Demo 描述文本中无 snake_case 属性名引用
- [ ] API 表格中事件属性行包含载荷类型
- [ ] Demo description 不包含 codegen 内部实现细节
