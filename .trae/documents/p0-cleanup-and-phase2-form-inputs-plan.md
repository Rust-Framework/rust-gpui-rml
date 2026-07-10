# P0 收尾 + Phase 2 表单输入组件迭代计划

## Summary

本计划覆盖两阶段工作：
1. **P0 设计对齐收尾**：核实并清理综合评估报告（2026-07-04）中的 3 处 P0 设计妥协残留与 5 处文档不同步
2. **Phase 2 表单输入组件**：按 `gpui-component-coverage-iteration-plan.md` 补全 9 个 Stateful 表单组件的声明式支持

### 前置核实结论（探索阶段已完成）

| 评估项 | 计划文档状态 | 实际代码状态 | 结论 |
|--------|------------|------------|------|
| P0-1 EntityHostHandle | 待移除 | `host_handle.rs` 已删除，全 crate 无 `EntityHostHandle`/`HostOp`/`__rml_install_host` 引用 | **已解决** ✅ |
| P0-2 VisualEntityCache | 待评估 | `entity_cache.rs` 仍存在，`macros/contribute.rs:249` 依赖 `get_or_create_entity` | **待处理** ⚠️ |
| P0-3 get_contribution_registry 签名 | 返回 `Arc<ContributionRegistry>` | `extensions.rs:25` 已返回 `Arc<dyn IContributionRegistry>` | **已解决** ✅ |
| D-1 onclick→on-click | 6 份文档落后 | docs/ 无 `onclick` 引用 | **已解决** ✅ |
| D-3 `<slot_menu>` typo | menu-bar.md:5 | docs/ 无 `slot_menu` 引用 | **已解决** ✅ |
| D-4 架构文档废弃类型 | contribution-system.md 引用废弃类型 | docs/ 仅 `observable-refactor-plan.md` 提及 HostHandle（确认已移除的历史记录） | **已解决** ✅ |
| D-5 Registry 方法名 | `add_host/remove_host` 偏差 | `contribution.rs:285/288` 已用 `add/remove` | **已解决** ✅ |
| D-2 menu-bar items 绑定 | 误导性示例 | `menu-bar.md:22` 仍有 `<menu items={menu_items} />`，但 codegen 说明（L28）已澄清为 MVVM 运行时渲染路径 | **需核实** |

### else-if 计划（p0-4）实施进度

**100% 完成**。9 个步骤全部落地：
- AST (`ast.rs:91`)、Parser (`mod.rs:210-214`)、Codegen 链式配对 (`meta.rs:173-321`)、Validator (`validator.rs:86`)
- Printer 两处 (`utils.rs:39` + `meta.rs:451-452`)
- LSP 三文件 (`binder.rs:133/211/229`、`formatting.rs:226`、`ast_util.rs:192/264`)
- Demo (`conditional_case.rml:16-24`)、文档 (`directives.md`)
- 6 个测试 (`node.rs:661/692/713/739/753/771`)

### 基础组件支持现状

已注册约 50 个标签：
- **原生 HTML（23 个）**：div/span/p/h1-h6/button/input/textarea/ul/ol/li/img/svg/a/label/br/code/anchored/deferred
- **扩展组件（30+ 个）**：Button/ButtonGroup/Badge/Checkbox/Label/Separator/Tag/Progress/ProgressCircle/Slider/Switch/Input/TextInput/CodeEditor/Radio/RadioGroup/TitleBar/NativeStatusBar/ActivityBar/Card/Avatar/AvatarGroup/Breadcrumb/GroupBox/Collapsible/Pagination/Spinner/Skeleton/Link/Tree/Tabs/TabBar/Table/DescriptionList/Popover/Accordion/Alert/Icon/Kbd/MenuBar/menu
- **菜单（4 个）**：ContextMenu/DropdownMenu/MenuBar/AppMenuBar
- **根节点（5 个）**：window/modern-window/tab-window/dialog/component

gpui-component 覆盖计划 Phase 1 已完成 8/8（Spinner/Skeleton/Link/Collapsible/GroupBox/Pagination/Radio/RadioGroup 均已注册；Tooltip 作为属性已实现，独立组件标签延后）。

---

## Part A：P0 设计对齐收尾

### A1. VisualEntityCache 设计文档对齐

**文件**：`crates/app/src/contribution/entity_cache.rs` + `docs/09-architecture/contribution-system.md`

**现状**：`VisualEntityCache` 存储于 `ServiceCollection`，通过 `get_or_create_entity::<T>(cx)` 为 `IVisual::render` 复用 Entity，避免每次渲染重建导致状态丢失。`macros/contribute.rs:249` 生成的 `IVisual::render` 依赖此函数。

**用户硬约束**：project_memory.md 记录"ComponentEntityCache 是不必要的；框架不存储此内容"。

**决策**：VisualEntityCache 的存在有技术必要性（防止视觉贡献 Entity 状态丢失），但与用户硬约束冲突。需向用户确认方向：
- **方案 A（推荐）**：保留 `VisualEntityCache`，更新 `project_memory.md` 和架构文档承认其为必要机制（Entity 生命周期管理，非"贡献缓存"），并澄清语义——它缓存的是渲染 Entity，而非贡献注册数据
- **方案 B**：移除 `VisualEntityCache`，改为在 `#[contributehost]` 宏生成的 host struct 上添加 `__rml_visual_entities: HashMap<TypeId, Box<dyn Any + Send + Sync>>` 字段，将存储从框架侧移到 host 侧（符合"contributions 直接交付给 IContributionHost"原则）

**改动（方案 A）**：
1. `entity_cache.rs` 模块注释更新：明确说明这是"视觉贡献 Entity 生命周期管理"，非"贡献缓存"
2. `docs/09-architecture/contribution-system.md`：补充 VisualEntityCache 的设计说明段落
3. `project_memory.md`：更新硬约束，区分"贡献注册缓存"（不需要）和"视觉 Entity 生命周期管理"（必要）

### A2. D-2 menu-bar items 绑定文档核实

**文件**：`docs/06-components/reference/menu-bar.md:22`

**现状**：文档第 22 行 `<menu items={menu_items} />`，codegen 说明（L28）已澄清为 MVVM 运行时渲染。需核实 `props_registry.rs` 是否确实支持 `items` 绑定。

**改动**：
1. 读取 `props_registry.rs` 确认 `menu`/`MenuBar` 的 `items` 属性是否注册
2. 若已支持：文档无需改动（当前说明已准确）
3. 若未支持：在 `menu-bar.md:22` 添加注释说明 `items` 绑定仅支持 `menu` 标签（非 `menu-bar`），或补充 props_registry 注册

---

## Part B：Phase 2 表单输入组件（9 个）

### 统一交付物清单（每个组件 6 项）

参照 `gpui-component-coverage-iteration-plan.md` §三，每个组件需完成：

| # | 交付物 | 文件位置 | 参考实现 |
|---|--------|---------|---------|
| 1 | UI re-export | `crates/ui/src/components/<name>.rs` + `mod.rs` 注册 | `badge.rs`/`icon.rs` |
| 2 | Compiler codegen | `crates/engine/src/compiler/components/<name>/gen.rs` + `mod.rs` 注册 | `code_editor/gen.rs`（Stateful） |
| 3 | Tags 注册 | `crates/engine/src/tags.rs` `component_lookup()` | 现有 Stateful 条目 |
| 4 | Props 注册 | `crates/engine/src/compiler/props_registry.rs` `COMPONENT_PROPS` | 现有条目 |
| 5 | Codegen 路由 | `crates/engine/src/compiler/component.rs` `gen_component()` | 现有专属分支 |
| 6 | Demo 案例 | `demo/src/cases/<name>_case.rml` + `.rml.rs` + `mod.rs` 注册 + i18n | 现有 case |

### B1. Stepper（Stateless, items builder）— 最简

**gpui-component 来源**：`gpui_component::stepper::{Stepper, StepperItem}`

| 项 | 内容 |
|----|------|
| 构造 | `Stepper::new(id)` |
| ComponentKind | `StatelessWithItems` |
| 属性 | `current`（usize, bind/static）、`direction`（horizontal/vertical，关联函数选择） |
| 子节点 | `<StepperItem>` / `<step-item>`（title/icon/description） |
| codegen 参考 | `accordion/gen.rs`（StatelessWithItems + 闭包 builder） |
| container | false |

**特殊处理**：`direction` 属性映射到关联函数 `Stepper::horizontal(id)` / `Stepper::vertical(id)`（参考 RadioGroup 的 vertical/horizontal 模式）。

### B2. Rating（Stateful, RatingState）

**gpui-component 来源**：`gpui_component::rating::{Rating, RatingState, RatingEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `RatingState::new()` + `Rating::new(&Entity<RatingState>)` |
| ComponentKind | `Stateful { state_field: "rating_state", state_ctor: "\|_w, _c\| RatingState::new()" }` |
| 属性 | `value`（bind）、`max`（usize）、`allow_half`（bool）、`disabled`（bool） |
| 事件 | `RatingEvent`（通过 `cx.subscribe` + `on_change` 回调） |
| codegen 参考 | `code_editor/gen.rs`（Stateful + EventEmitter 订阅） |
| container | false |

### B3. NumberInput（Stateful, 复用 InputState）

**gpui-component 来源**：`gpui_component::input::number_input::{NumberInput, NumberInputEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `NumberInput::new(&Entity<InputState>)` — 复用 InputState |
| ComponentKind | `Stateful { state_field: "input_state", state_ctor: "\|w, c\| InputState::new(w, c)" }` |
| 属性 | 同 Input + `precision`、`step`、`min`、`max` |
| 事件 | `NumberInputEvent`（需在 subscribe 回调中区分 NumberInputEvent vs InputEvent） |
| codegen 参考 | `code_editor/gen.rs`（复用 InputState 的延迟初始化模式） |
| container | false |

**关键差异**：codegen 需生成 `cx.subscribe(&entity, move |_, event, cx| { if let Ok(e) = event.downcast_ref::<NumberInputEvent>() {...} })` 处理 NumberInputEvent。

### B4. OtpInput（Stateful, OtpState）

**gpui-component 来源**：`gpui_component::input::otp_input::{OtpInput, OtpState}`

| 项 | 内容 |
|----|------|
| 构造 | `OtpState::new(length, w, cx)` + `OtpInput::new(&Entity<OtpState>)` |
| ComponentKind | `Stateful { state_field: "otp_state", state_ctor: "\|w, c\| OtpState::new(6, w, c)" }`（默认 length=6） |
| 属性 | `length`（构造器参数，需 codegen 提取为字面量）、`default_value`、`mask`（bool） |
| 事件 | `InputEvent` |
| codegen 参考 | `code_editor/gen.rs` + length 属性提取 |
| container | false |

**特殊处理**：`length` 属性需在 codegen 时提取为 usize 字面量，注入 `state_ctor` 闭包：`\|w, c\| OtpState::new({length}, w, c)`。

### B5. ColorPicker（Stateful, ColorPickerState）

**gpui-component 来源**：`gpui_component::color_picker::{ColorPicker, ColorPickerState, ColorPickerEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `ColorPickerState::new()` + `ColorPicker::new(&Entity<ColorPickerState>)` |
| ComponentKind | `Stateful { state_field: "color_picker_state", state_ctor: "\|_w, _c\| ColorPickerState::new()" }` |
| 属性 | `default_value`（Hsla）、`placeholder` |
| 事件 | `ColorPickerEvent` |
| codegen 参考 | `code_editor/gen.rs` |
| container | false |

### B6. Calendar（Stateful, CalendarState）

**gpui-component 来源**：`gpui_component::time::calendar::{Calendar, CalendarState, CalendarEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `CalendarState::new()` + `Calendar::new(&Entity<CalendarState>)` |
| ComponentKind | `Stateful { state_field: "calendar_state", state_ctor: "\|_w, _c\| CalendarState::new()" }` |
| 属性 | `disabled_matcher`（高级，暂不支持）、`year_range` |
| 事件 | `CalendarEvent` |
| UI re-export | `pub use gpui_component::time::calendar::{Calendar, CalendarState, CalendarEvent};` |
| container | false |

### B7. DatePicker（Stateful, DatePickerState）

**gpui-component 来源**：`gpui_component::time::date_picker::{DatePicker, DatePickerState}`

| 项 | 内容 |
|----|------|
| 构造 | `DatePickerState::new()` + `DatePicker::new(&Entity<DatePickerState>)` |
| ComponentKind | `Stateful { state_field: "date_picker_state", state_ctor: "\|_w, _c\| DatePickerState::new()" }` |
| 属性 | `placeholder`、`cleanable`（bool）、`default_value` |
| 事件 | DatePicker 事件（位于 time 模块） |
| UI re-export | `pub use gpui_component::time::date_picker::{DatePicker, DatePickerState};` |
| container | false |

### B8. Select（Stateful, SelectState）

**gpui-component 来源**：`gpui_component::select::{Select, SelectState, SelectEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `SelectState::new()` + `Select::new(&Entity<SelectState>)` |
| ComponentKind | `Stateful { state_field: "select_state", state_ctor: "\|_w, _c\| SelectState::new()" }` |
| 属性 | `placeholder`、`menu_width`、`menu_max_h`、`icon`、`value`（bind） |
| 事件 | `SelectEvent`、`DismissEvent` |
| 子节点 | 选项数据绑定（`items={options}` 或 `<option>` 子标签） |
| codegen 参考 | `code_editor/gen.rs` + 选项数据绑定 |
| container | false |

**复杂度中高**：需处理选项数据绑定。方案：通过 `items={vec}` bind 传入 `Vec<SelectItem>`，codegen 生成 `.items(vec)` 调用。

### B9. ComboBox（Stateful, ComboboxState）

**gpui-component 来源**：`gpui_component::combobox::{Combobox, ComboboxState, ComboboxEvent}`

| 项 | 内容 |
|----|------|
| 构造 | `ComboboxState::new()` + `Combobox::new(&Entity<ComboboxState>)` |
| ComponentKind | `Stateful { state_field: "combobox_state", state_ctor: "\|_w, _c\| ComboboxState::new()" }` |
| 属性 | 同 Select + `search_placeholder` |
| 事件 | `ComboboxEvent`、`DismissEvent` |
| codegen 参考 | Select + 搜索属性 |
| container | false |

### 实施顺序

按复杂度递增：
1. **B1 Stepper**（StatelessWithItems，无 Stateful 复杂度）→ 验证 items builder 模式
2. **B2 Rating**（简单 Stateful + RatingState）→ 验证 Stateful + EventEmitter 模式
3. **B3 NumberInput**（复用 InputState）→ 验证状态复用模式
4. **B4 OtpInput**（独立 OtpState + length 参数）→ 验证构造器参数注入
5. **B5 ColorPicker**（独立 ColorPickerState）→ 标准 Stateful
6. **B6 Calendar**（time 模块）→ 验证跨模块 re-export
7. **B7 DatePicker**（time 模块，依赖 Calendar）→ 同上
8. **B8 Select**（选项数据绑定）→ 验证 items 绑定
9. **B9 ComboBox**（Select + 搜索）→ 复用 Select 模式

---

## Assumptions & Decisions

1. **VisualEntityCache 方案选择**：计划默认推荐方案 A（保留 + 文档对齐），因移除需改动宏生成代码且技术风险较高。若用户选择方案 B，Part A 工作量增加约 3 倍。
2. **Stepper direction 关联函数**：参考 RadioGroup 的 `vertical(id)`/`horizontal(id)` 模式，codegen 根据 `direction` 属性选择关联函数。
3. **NumberInput 事件区分**：NumberInput 复用 InputState 但发出 `NumberInputEvent`，codegen 需在 subscribe 回调中用 `downcast_ref` 区分事件类型。
4. **OtpInput length 参数**：`length` 为构造器参数而非 setter，codegen 需从属性提取 usize 字面量注入 `state_ctor` 闭包。默认值 6。
5. **Select/ComboBox 选项绑定**：通过 `items={vec}` bind 传入选项数据，不使用 `<option>` 子标签（避免 items builder 复杂度）。若后续需要声明式选项，可扩展。
6. **Tooltip 独立组件**：本迭代不纳入，延后至 Phase 3 或独立小迭代。
7. **每个组件需附带 codegen 单元测试**：覆盖构造器选择、关键 setter、事件绑定、ref/id 增量（参考 `code_editor/gen.rs` 测试模式）。

## Verification Steps

### Part A 验证
1. `cargo build --workspace` — 确认无编译错误
2. `grep -r "EntityHostHandle" crates/` — 无结果（已验证）
3. `grep -r "get_contribution_registry" crates/` — 返回 `Arc<dyn IContributionRegistry>`（已验证）
4. 若方案 A：`docs/09-architecture/contribution-system.md` 包含 VisualEntityCache 设计说明

### Part B 验证（每个组件完成后）
1. `cargo build -p rust-rml-ui` — UI re-export 编译通过
2. `cargo build -p rust-rml-engine` — codegen 模块编译通过
3. `cargo test -p rust-rml-engine -- <component>` — 组件 codegen 测试通过
4. `cargo test -p rust-rml-engine --test props_registry_complete` — props 注册表完整性测试通过
5. `cargo build -p rust-rml-demo` — demo 编译通过
6. 运行 demo — 组件 case 正常渲染与交互

### 全量验证（全部完成后）
1. `cargo build --workspace` — 全工作区编译通过
2. `cargo test -p rust-rml-engine` — 全部引擎测试通过
3. `cargo run -p rust-rml-demo` — 所有新 case 在 demo 中可正常访问
4. `crates/engine/src/compiler/components/` 与 `crates/ui/src/components/` 仅做 re-export，无业务代码
5. 每个组件独占一个 rs 文件/目录
6. 无 `rml_` 前缀标识符（除框架内部 `__rml_*`）
