# RML 基础组件支持完整度深度审查与迭代计划

> 本文档基于对 `crates/engine` 全量代码的审查，评估 RML 框架在元素、属性、事件、样式、主题适配、数据绑定、控制语法、组件封装等各方面的支持完整度，判定是否具备基于基础控件按 `.rml` 模式 MVVM 封装高级复杂组件的能力，并制定针对性迭代计划。

---

## 一、审查范围与方法

### 审查范围
- **元素支持**：HTML 原生标签、扩展组件（gpui-component 封装）、用户自定义组件
- **属性支持**：通用属性、组件专用属性、样式属性、shell 属性
- **事件支持**：鼠标/键盘/悬停/Action 事件、组件级事件
- **样式与主题**：CSS 子集映射、CSS 变量、主题适配
- **数据绑定**：单向/双向绑定、绑定路径、IConverter、校验
- **控制语法**：if/else/each/key/model/show/once/html/ref 指令
- **组件封装能力**：用户自定义组件、slot 机制、props 传递、嵌套能力

### 审查方法
- 阅读 [tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)（元素注册表）
- 阅读 [props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)（属性注册表）
- 阅读 [event.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/event.rs)（事件绑定）
- 阅读 [codegen/binding.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs)（双向绑定与校验）
- 阅读 [user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/user_component.rs)（用户组件封装）
- 阅读 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)（CSS 映射）
- 阅读 [parser/ast.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs)（指令定义）
- 阅读 [codegen/node.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs)（节点生成）
- 阅读 [translator/slot.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/slot.rs)（slot 占位符）
- 对比已有 [rml-iteration-plan.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-plan.md) 的待办事项
- 检查 demo 案例覆盖情况（68 个 .rml 文件）

---

## 二、当前状态分析（完整度矩阵）

### 2.1 元素支持

| 类别 | 数量 | 状态 | 清单 |
|------|------|------|------|
| HTML 原生标签 | 22 | ✅ 完整 | div/span/p/h1-h6/button/input/textarea/ul/ol/li/img/svg/a/label/br/code/anchored/deferred |
| 根节点 | 5 | ✅ 完整 | window/modern-window/tab-window/dialog/component |
| Stateless 扩展组件 | 14 | ✅ 完整 | Button/Alert/ButtonGroup/Checkbox/Label/Separator/Tag/Progress/ProgressCircle/Switch/Card/Link/Radio/Pagination |
| StatelessNoId 扩展组件 | 12 | ✅ 完整 | Badge/TitleBar/NativeStatusBar/Avatar/AvatarGroup/Breadcrumb/Icon/Kbd/Spinner/Skeleton/Collapsible/GroupBox |
| Stateful 扩展组件 | 5 | ✅ 完整 | Slider/Input/TextInput/CodeEditor/Tree |
| StatelessWithItems 扩展组件 | 6 | ✅ 完整 | DescriptionList/Accordion/Tabs/TabBar/Table/Popover |
| EntityRef 扩展组件 | 1 | ✅ 完整 | ActivityBar |
| 用户自定义组件 | 1 | ✅ 完整 | `#[component]` 标注 struct |

**结论**：元素覆盖完整，覆盖 gpui-component 全套基础控件。

### 2.2 属性支持

| 类别 | 状态 | 说明 |
|------|------|------|
| 通用静态属性 | ✅ | label/placeholder/tooltip/size/compact/loading/disabled/selected/font_*/style |
| 通用绑定属性 | ✅ | content/value/disabled/selected/checked/label/size |
| 通用事件属性 | ⚠️ 部分 | 仅 on_click（其他事件通过 component_event_setter 分发） |
| 组件专用属性 | ✅ | COMPONENT_PROPS 完整登记 |
| 样式属性 | ✅ | ~40+ CSS 属性（盒模型/文本/Flexbox/视觉/定位/阴影/cursor/Grid） |
| Shell 属性 | ✅ | window/modern-window/tab-window/dialog/component 各自完整 |

### 2.3 事件支持

| 类别 | 事件 | 状态 |
|------|------|------|
| 鼠标事件 | on_click/on_aux_click/on_mouse_down/on_mouse_up/on_mouse_move/on_wheel | ✅ |
| 键盘事件 | on_key_down/on_key_up | ✅ |
| 悬停事件 | on_hover/on_mouse_enter/on_mouse_leave | ✅ |
| Action 事件 | on_action（多 Action 类型） | ✅ |
| Input 组件事件 | on_change（通过 EventEmitter + cx.subscribe 模式） | ✅ |
| 事件参数 | WithArgs(method, [单参数]) | ⚠️ 仅单参数 |
| **焦点事件** | on_focus/on_blur | ❌ 不支持 |
| **表单事件** | on_submit | ❌ 不支持 |
| **容器事件** | on_scroll/on_resize/on_load | ❌ 不支持 |
| **循环变量传参** | on-click={command(item.id)} | ❌ 不支持（迭代计划2.3） |
| **用户组件事件** | 用户组件的 on-click 等 | ❌ 不支持（Phase 1 跳过） |

### 2.4 控制语法（指令）

| 指令 | 语法 | 状态 | 说明 |
|------|------|------|------|
| if | `if={cond}` | ✅ | 条件渲染 |
| else | `else` | ✅ | 分支 |
| **else if** | `else if={cond}` | ❌ | 不支持链式条件，需多个并列 if 替代 |
| each | `each={item in items}` | ✅ | 支持 `(item, idx) in items` 索引变量 |
| key | `key={expr}` | ✅ | 列表项唯一标识 |
| model | `model={field}` / `model={field \| Converter}` | ✅ | 双向绑定（仅 input） |
| show | `show={cond}` | ✅ | 显示/隐藏 |
| once | `once` | ⚠️ | slot 闭包内 bug（迭代计划2.1） |
| html | `html={raw}` | ✅ | 降级为 Label 文本节点 |
| ref | `ref="name"` | ✅ | 元素引用 |
| scope | `scope={name}` | ⚠️ | 仅简单标识符，不支持 foo.bar |

### 2.5 数据绑定

| 能力 | 状态 | 说明 |
|------|------|------|
| 单向绑定 `{field}` | ✅ | 完整支持 |
| 双向绑定 `model={field}` | ⚠️ | 仅 input 组件支持，Checkbox/Switch/Slider/Radio 等未支持 model 指令 |
| 绑定路径 `user.name` | ✅ | 嵌套字段支持 |
| IConverter 转换 | ✅ | `{field \| Converter}` 双向转换 |
| 校验规则 | ✅ | range/required/length/regex/custom/IValidate |
| 错误状态 UI | ✅ | 红框 + tooltip |
| **非 input 组件双向绑定** | ❌ | Checkbox/Switch/Slider/Radio 缺少 model 指令支持 |

### 2.6 CSS 与样式

| 类别 | 状态 | 说明 |
|------|------|------|
| 盒模型 | ✅ | width/height/padding/margin/border/border-radius 完整 |
| 文本 | ✅ | font-size/font-weight/font-family/text-align/line-height/white-space/color |
| Flexbox | ✅ | display/flex-direction/flex-wrap/justify-content/align-items/flex/gap |
| 视觉效果 | ✅ | opacity/overflow/box-shadow/cursor/visibility |
| 定位 | ✅ | position/top/right/bottom/left/inset |
| Grid | ✅ | grid-template-columns/rows/grid-column/row |
| CSS 变量 var() | ✅ | 构建期 + 运行时主题查询 |
| 简写 | ✅ | padding/margin/border/flex 多值简写已支持 |
| 主题切换 | ✅ | var(--name) 运行时查询 |
| **CSS 动画/过渡** | ❌ | transform/transition/animation 不支持 |
| **CSS 伪类** | ❌ | :hover/:focus/:active/:disabled 不支持 |
| **媒体查询** | ❌ | 响应式布局不支持 |
| **未映射属性** | ⚠️ | 静默丢弃（仅 stderr 警告） |

### 2.7 组件封装能力

| 能力 | 状态 | 说明 |
|------|------|------|
| 用户自定义组件 `#[component]` | ✅ | struct 标注 + 模板生成 |
| 静态属性传递 | ✅ | String/i32/u32/bool 等类型转换 |
| 绑定属性传递 | ✅ | 闭包外计算避免 cx 借用冲突 |
| 具名 slot | ✅ | `<template slot="name">` |
| 默认 slot | ✅ | `<template slot="default">` 或直接子节点 |
| slot 闭包捕获父视图 | ✅ | `__rml_self_entity` + `__rml_self_ref` 别名 |
| **用户组件事件** | ❌ | Phase 1 跳过（不支持 on-click 等） |
| **作用域插槽** | ⚠️ | scope 仅简单标识符 |
| **可复用模板片段** | ❌ | 缺失参数化模板/宏机制（迭代计划2.2） |
| **IVisual::render 声明式** | ❌ | 要求返回 AnyElement（迭代计划2.4） |

---

## 三、组件封装能力评估结论

### 3.1 已具备的能力

当前 RML 框架**已具备**封装中低复杂度组件的能力：

1. **基础 UI 组件齐全**：覆盖 gpui-component 全套（按钮/输入/选择/卡片/标签/进度/表格/标签页/手风琴/气泡/树/头像/面包屑/分页/单选/骨架屏等）
2. **组合机制完整**：slot 具名/默认 + 闭包捕获父视图 + props 传递
3. **样式系统完整**：CSS 子集 + 主题适配 + 内联 style
4. **数据流完整**：单向绑定 + 双向绑定（input）+ IConverter + 校验
5. **控制语法齐全**：if/else/each/key/show/once/html/ref 覆盖常见场景

### 3.2 尚不完全具备的能力

当前 RML 框架**尚不完全具备**封装高复杂度组件的能力，主要瓶颈：

| 瓶颈 | 影响 | 严重度 |
|------|------|--------|
| 用户组件不支持事件绑定 | 无法在用户组件上响应 on-click 等 | 🔴 高 |
| 列表交互无法传递循环变量 | 列表项点击无法传 id 给命令 | 🔴 高 |
| 可复用模板片段缺失 | 组件内无法定义参数化 UI 片段 | 🟡 中 |
| 非input组件双向绑定缺失 | Checkbox/Switch/Slider 无法用 model 指令 | 🟡 中 |
| 焦点/表单事件缺失 | on_focus/on_blur/on_submit 不支持 | 🟡 中 |
| IVisual::render 命令式 | 状态栏贡献点被迫命令式 | 🟢 低 |
| once slot bug | once 指令在 slot 内不可用 | 🟢 低 |

### 3.3 总体判定

**结论**：当前框架支持封装"基础UI组合 + 简单数据展示 + 基础交互"的组件（如 CaseDocPage、Card 组合、表单展示等），但**尚不完全支持**封装"复杂交互列表 + 表单 + 状态机驱动"的高级组件（如带筛选/排序/分页的数据表格、多步表单、可视化编辑器面板等）。

---

## 四、迭代计划

### 4.1 优先级原则

- **P0**：阻塞核心场景，影响多个组件封装能力
- **P1**：影响常见交互场景，有 workaround 但代价高
- **P2**：增强能力，提升开发体验
- **P3**：锦上添花，非阻塞

### 4.2 迭代项清单

#### P0-1：用户自定义组件事件支持

- **现状**：[user_component.rs:242](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/user_component.rs#L242) 中 `Attribute::Event { .. } => return Ok(None)`，跳过事件属性
- **影响**：用户组件无法响应 on-click/on-change 等事件，严重限制封装能力
- **修复方向**：
  1. 在 `gen_user_component_body` 中处理 `Attribute::Event`
  2. 用户组件需声明事件回调字段（如 `on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>`）
  3. codegen 生成 `__rml_entity.update(cx, |this, _cx| { this.on_click = Some(Box::new(...)); })`
- **验证**：新增 `user_component_event_case.rml` demo，验证 on-click 事件传递

#### P0-2：循环变量作为命令参数

- **现状**：[迭代计划2.3](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-plan.md) - `on-click={command(item.id)}` 不支持
- **影响**：列表项点击无法传 id，被迫命令式渲染（welcome_case 等）
- **修复方向**：扩展事件处理器语法 `on-click={command(expr)}`，codegen 生成 `let __rml_arg = expr; cx.listener(move |this, _ev, _window, cx| { this.command(__rml_arg, cx); })`
- **验证**：welcome_case 回退为声明式 each + on-click={open_case(item.id)}

#### P0-3：once 指令 slot 闭包 bug

- **现状**：[迭代计划2.1](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-plan.md) - once 在 slot 内生成 `&mut self` 代码
- **影响**：once_case 无法在 slot 内使用
- **修复方向**：将 once 的快照存储改为 `RefCell` 或 `Mutex` 内部可变性，使 `once_get_or_init` 只需 `&self`
- **验证**：once_case slot 内使用 once 指令

#### P1-1：非 input 组件双向绑定（model 指令扩展）

- **现状**：[codegen/binding.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs) 中 `gen_model_input` 仅处理 input
- **影响**：Checkbox/Switch/Slider/Radio 无法用 `model={field}` 双向绑定
- **修复方向**：
  1. 新增 `gen_model_checkbox` / `gen_model_switch` / `gen_model_slider` 等
  2. 各组件专用 codegen 处理 model 指令
  3. 扩展 [model.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/model.rs) 的字段收集逻辑
- **验证**：新增 `model_checkbox_case.rml` / `model_switch_case.rml` demo

#### P1-2：焦点与表单事件支持

- **现状**：[event.rs:74](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/event.rs#L74) 中 `on_focus` / `on_blur` / `on_submit` 返回 None
- **影响**：表单组件无法响应焦点事件
- **修复方向**：
  1. GPUI 的 focus/blur 通过 `FocusHandle` + `cx.on_focus` / `cx.on_blur` 实现
  2. 扩展 `event_binding()` 添加 on_focus/on_blur/on_submit 分支
  3. 生成 `.on_focus(...)` / `.on_blur(...)` 调用（需元素为 Stateful）
- **验证**：新增 `focus_event_case.rml` demo

#### P1-3：else if 链式条件支持

- **现状**：仅支持 if/else 两分支，多分支需多个并列 if（[conditional_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/conditional_case.rml) 演示）
- **影响**：多分支条件渲染冗长，性能略差（每次都判断）
- **修复方向**：
  1. parser 支持解析 `else if={cond}` 语法
  2. AST 新增 `Directive::ElseIf { expr, span }`
  3. codegen 生成 `if ... else if ... else ...` 链
- **验证**：conditional_case 改用 else if 链

#### P1-4：可复用模板片段机制

- **现状**：[迭代计划2.2](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-plan.md) - 缺乏参数化模板/宏
- **影响**：组件内无法复用带参数的 UI 片段，被迫命令式
- **修复方向**：采用 Vue 风格 `<template define="name(param)">` 定义 + `<template use="name" args="..." />` 引用
- **验证**：template_slot_case 回退为声明式模板片段

#### P2-1：用户组件 ref 机制

- **现状**：用户组件未明确支持 `ref="name"` 获取 `ElementRef<T>`
- **影响**：父组件无法直接操作子组件实体
- **修复方向**：在 `gen_user_component_body` 中处理 ref 属性，生成 `__rml_state.get_or_init_ref(...)` 调用
- **验证**：新增 `user_component_ref_case.rml` demo

#### P2-2：作用域插槽表达式扩展

- **现状**：[slot_scope_case.rml:30](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_scope_case.rml#L30) - scope 仅简单标识符
- **影响**：无法 `scope={panel.current_size}` 等复杂表达式
- **修复方向**：解析 scope 表达式为路径访问，codegen 生成对应方法调用链

#### P2-3：CSS 未映射属性编译期警告

- **现状**：[css/mapper.rs:398](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L398) 仅 stderr 警告
- **影响**：属性静默丢弃，开发者难以发现
- **修复方向**：收集未映射属性到 CodegenError 列表，编译期输出诊断
- **验证**：新增测试验证未映射属性产生诊断

#### P3-1：IVisual::render 声明式支持

- **现状**：[迭代计划2.4](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-plan.md) - 状态栏贡献点被迫命令式
- **影响**：status_bar_case 等被迫命令式
- **修复方向**：为 `#[contribute(kind = "status")]` 关联 .rml 模板片段
- **验证**：status_bar_case 回退为声明式

#### P3-2：CSS 动画/过渡支持

- **现状**：transform/transition/animation 不支持
- **影响**：无法实现动画效果
- **修复方向**：扩展 css/mapper.rs 支持 transform/transition 属性映射

### 4.3 迭代优先级排序

| 优先级 | 项目 | 影响面 | 复杂度 | 依赖 |
|--------|------|--------|--------|------|
| P0 | P0-1 用户组件事件支持 | 框架核心能力 | 中 | 无 |
| P0 | P0-2 循环变量传参 | 列表交互通用能力 | 中 | 无 |
| P0 | P0-3 once slot bug | 框架核心能力 | 低 | 无 |
| P1 | P1-1 非input组件双向绑定 | 表单组件通用 | 中 | 无 |
| P1 | P1-2 焦点/表单事件 | 表单交互 | 中 | 无 |
| P1 | P1-3 else if 链式条件 | 代码简洁性 | 低 | 无 |
| P1 | P1-4 可复用模板片段 | 组件复用能力 | 高 | 无 |
| P2 | P2-1 用户组件 ref | 组件操作 | 低 | P0-1 |
| P2 | P2-2 作用域插槽表达式 | slot 能力增强 | 中 | 无 |
| P2 | P2-3 CSS 未映射属性警告 | 开发体验 | 低 | 无 |
| P3 | P3-1 IVisual::render 声明式 | 状态栏贡献点 | 中 | 无 |
| P3 | P3-2 CSS 动画/过渡 | 视觉效果 | 高 | 无 |

### 4.4 建议实施顺序

**第一批（P0，解锁核心封装能力）**：
1. P0-3 once slot bug（最简单，先解阻塞）
2. P0-1 用户组件事件支持（解锁用户组件交互）
3. P0-2 循环变量传参（解锁列表交互）

**第二批（P1，完善表单与控制语法）**：
4. P1-1 非input组件双向绑定
5. P1-2 焦点/表单事件
6. P1-3 else if 链式条件
7. P1-4 可复用模板片段

**第三批（P2-P3，增强体验）**：
8. P2-1 用户组件 ref
9. P2-2 作用域插槽表达式
10. P2-3 CSS 未映射属性警告
11. P3-1 IVisual::render 声明式
12. P3-2 CSS 动画/过渡

---

## 五、验证策略

### 5.1 每个迭代项的验证标准

每个 P0/P1 项完成后需验证：
1. **新增 demo case**：演示新能力
2. **回退 workaround**：将原命令式 demo 改为声明式
3. **单元测试**：新增 codegen 测试覆盖新语法
4. **全量测试**：`cargo test -p rust-rml-engine` 全部通过
5. **demo 编译**：`cargo build -p rml-demo` 成功

### 5.2 整体验证

所有迭代完成后需验证：
1. **68 个现有 demo** 全部编译通过
2. **新增 demo** 演示各项新能力
3. **MVVM 合规审计**：所有 demo 遵循 `.rml` + `.rml.rs` 模式，无命令式 UI 绕过
4. **组件封装能力评估**：新增 1-2 个高复杂度组件 demo（如带筛选/排序/分页的数据表格）验证框架能力

---

## 六、假设与决策

### 假设
1. GPUI 框架本身支持 on_focus/on_blur 等事件（通过 FocusHandle）
2. gpui-component 的 Checkbox/Switch/Slider 支持 `.on_click(&bool)` 等回调，可扩展为 model 指令
3. 用户组件事件字段类型为 `Option<Box<dyn Fn(...)>>`，由 codegen 注入闭包

### 决策
1. **不扩展 HTML 语义元素**（form/nav/main 等）：当前 div+CSS 已足够，扩展会增加维护成本且无功能收益
2. **不支持 CSS 动画/过渡**（P3）：GPUI 动画通过 `Animation` API 实现，与 CSS 动画语义不同
3. **不支持 CSS 伪类**：GPUI 通过 `.hoverable()` / 状态字段实现交互态样式
4. **不支持媒体查询**：桌面应用无响应式需求
5. **优先完善组件封装能力**而非扩展 CSS 子集：当前 CSS 子集已覆盖 95% 场景

---

## 七、总结

RML 框架在**元素覆盖、属性体系、基础控制语法、样式映射、数据绑定校验**等方面已具备完整能力，能够支持中低复杂度组件的声明式封装。但**用户组件事件、列表交互参数传递、非input双向绑定、焦点事件**等方面的缺失，限制了高复杂度组件的封装能力。

通过本次迭代的 P0 三项（用户组件事件、循环变量传参、once slot bug）可解锁核心封装能力；通过 P1 四项（非input双向绑定、焦点事件、else if、模板片段）可完善表单与控制语法；通过 P2-P3 可增强开发体验。

预计完成 P0+P1 后，框架将具备封装绝大多数高级复杂组件的能力。
