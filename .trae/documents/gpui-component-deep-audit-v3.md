# gpui-component 深度审查 v3 —— 反人类模式修复与文档迭代计划

> **审查目标**：深入审查已实现的 gpui-component 组件，评估声明式用法是否符合开发者直觉，识别异味用法与反人类思维模式，制定针对性迭代计划。
>
> **审查范围**：`crates/engine/src/compiler/` 全部组件 codegen + `crates/engine/src/tags.rs` 组件注册 + `demo/src/cases/` 全部 .rml/.rml.rs 演示。

---

## 一、审查发现总览

经逐文件审查，发现 **3 个反人类代码模式**（Phase A）和 **2 类文档问题**（Phase B）：

| 编号 | 类型 | 问题 | 影响 |
|------|------|------|------|
| A1 | 反人类命名 | Accordion `open-ixs` 缩写晦涩 | 开发者无法直觉理解 "ixs" = "indices" |
| A2 | 反人类架构 | Tree 不支持 `ref` 指令 + 字段名硬编码 | 无法多实例，字段名必须叫 `tree_state` |
| A3 | 反人类用法 | Slider 不支持声明式 min/max/step/default_value | 必须在 on_loaded 中手动创建 SliderState |
| B2 | 文档缺失 | 12 个 demo 的 API 表事件行缺少 payload 类型 | 开发者不知道回调签名 |
| B3 | 文档异味 | 14 个 demo 描述/注释暴露 codegen 内部实现 | 开发者被迫理解框架内部机制 |

> **B1（snake_case 属性）已验证无需修复**：所有 .rml 文件均正确使用 kebab-case。

---

## 二、Phase A —— 反人类模式修复

### A1. 重命名 `open-ixs` → `open-indices`

**问题**：Accordion 的受控模式属性 `open-ixs` 使用非标准缩写 "ixs"（indices），开发者不读 codegen 源码无法理解含义。违反 RML 属性命名应直观可读的规范。

**修复方案**：全量重命名 `open_ixs` → `open_indices`（RML 属性写 `open-indices`，parser 规范化为 `open_indices`）。

**涉及文件**：

| 文件 | 行号 | 修改内容 |
|------|------|----------|
| [props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L109) | L109 | `"open_ixs"` → `"open_indices"` |
| [accordion/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/gen.rs#L41) | L41,42,46,57,107,144 | `open_ixs` / `open_ixs_expr` → `open_indices` / `open_indices_expr`；注释 L39,56,100,143 同步 |
| [accordion/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/components/accordion/setters.rs#L56) | L56,58,68,69,136 | 注释和测试中的 `open_ixs` → `open_indices` |
| [accordion_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml#L4) | L4,10,12,28,41,46,56,69,89 | `open-ixs` → `open-indices`；描述 L4 同步 |
| [accordion_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs#L68) | L68,107,108 | API 表 L68 `"open-ixs"` → `"open-indices"`；方法参数 L107 `open_ixs` → `open_indices`；L108 format! 同步 |

**验证**：
- `cargo test -p rust-rml-engine -- accordion` 通过
- `cargo build -p rust-rml-demo` 成功
- `grep -r "open.ixs" crates/ demo/` 无匹配

---

### A2. Tree 改造为 StatefulWithDelegate —— 支持 ref + items 声明式绑定

**问题**：Tree 当前是 Stateful 组件，存在三个反人类点：
1. **不支持 `ref` 指令**：gen_tree 硬编码 `Tree::new(self.tree_state.as_ref())`，无法多实例
2. **字段名硬编码**：tags.rs 中 `state_field: "tree_state"`，开发者的 ViewModel 字段必须叫 `tree_state`
3. **必须手动创建 TreeState**：在 on_loaded 中 `cx.new(|cx| TreeState::new(cx).items(items))`，无法声明式绑定 items

**对比参考**：Select / Combobox 已采用 StatefulWithDelegate 模式，支持 `ref="name" items={field}` 声明式绑定，是 Tree 应遵循的规范。

**修复方案**：将 Tree 从 Stateful 改为 StatefulWithDelegate，支持 `ref="name" items={field}` 声明式绑定。

#### 步骤 1：tags.rs —— 改 ComponentKind

[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L399-L406) L399-406：

```rust
// 修改前
"Tree" => Some(ComponentTag {
    ctor_path: "rml_ui::Tree",
    kind: ComponentKind::Stateful {
        state_field: "tree_state",
        state_ctor: "|_w, c| rml_ui::TreeState::new(c)",
    },
    container: false,
}),

// 修改后
"Tree" => Some(ComponentTag {
    ctor_path: "rml_ui::Tree",
    kind: ComponentKind::StatefulWithDelegate {
        state_field: "tree_state",
        state_ctor: "move |_w, c| rml_ui::TreeState::new(c).items(__rml_delegate)",
        delegate_attr: "items",
    },
    container: false,
}),
```

#### 步骤 2：tree/gen.rs —— 移除构造逻辑，仅保留测试或删除

gen_tree 当前同时处理构造 + setter，与 tree.rs translator 存在 setter 重复应用。改造后 tree.rs 直接生成构造代码，gen_tree 不再需要。删除 gen.rs 或仅保留 event_setter 引用。

#### 步骤 3：tree.rs translator —— 重写构造逻辑

[tree.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/tree.rs) 改造要点：
- 提取 `ref` 指令名和 `items` 绑定表达式
- 生成 StatefulWithDelegate 构造代码（delegate 注入 state_ctor）
- 保留 Tree 专用 event_setter（on_activate/on_select）在 setter 循环中优先调用
- 跳过 `items` bind 属性（已由 delegate 注入消费）

构造代码生成逻辑：
```rust
// ref + items 路径（StatefulWithDelegate 必须有 ref）
({
    let __rml_delegate = (self.tree_items).clone();
    let __rml_entity = (self.__rml_state.get_or_init_ref("basic_tree", _window, &mut *cx, move |_w, c| rml_ui::TreeState::new(c).items(__rml_delegate))).clone();
    rml_ui::Tree::new(Some(&__rml_entity))
})
```

> **注意 Tree::new 签名**：当前 `Tree::new(state: Option<&Entity<TreeState>>)`，与 gen_stateful_with_delegate_body 生成的 `Tree::new(&__rml_entity)` 不匹配。tree.rs 需自行生成 `Tree::new(Some(&__rml_entity))`。

setter 循环改造（event 分支增加 Tree 专用 setter 优先匹配）：
```rust
Attribute::Event { name, handler, .. } => {
    if let Some(s) = crate::compiler::components::tree::setters::event_setter(name, handler, &resolved) {
        code.push_str(&s);
    } else if let Some(s) = component_event_setter(name, handler, &resolved) {
        code.push_str(&s);
    }
}
```

bind 分支跳过 delegate_attr：
```rust
Attribute::Bind { name, .. } if name == "items" => continue,
```

#### 步骤 4：stateful.rs —— 保持 Tree 在排除列表

[stateful.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateful.rs#L37) L37 排除列表保持 "Tree"（tree.rs 专用 translator 处理）。

#### 步骤 5：tree_case.rml —— 声明式改造

[tree_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml) 改造：
- 描述 L4：移除 "Tree 不支持 ref 指令"、"gen_tree 硬编码"、"state_field 硬编码" 等 codegen 内部描述，改为用户视角
- L13：`<Tree on-activate={on_activate} on-select={on_select} />` → `<Tree ref="basic_tree" items={tree_items} on-activate={on_activate} on-select={on_select} />`
- L14 hint：移除 "Tree 不支持 ref 指令" 限制说明
- L34 API 注释：移除 codegen 内部描述

#### 步骤 6：tree_case.rml.rs —— ElementRef 改造

[tree_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml.rs) 改造：
- 字段：`pub tree_state: Option<gpui::Entity<TreeState>>` → `pub basic_tree: ElementRef<TreeState>` + `pub tree_items: Vec<TreeItem>`
- on_loaded：移除 `cx.new(|cx| TreeState::new(cx).items(items))`，改为 `self.tree_items = vec![...]`
- API 表：移除 "tree_state" 行，增加 "ref"、"items" 行

**验证**：
- `cargo test -p rust-rml-engine -- tree` 通过
- `cargo build -p rust-rml-demo` 成功
- `grep -r "tree_state" demo/src/cases/tree_case` 无匹配（字段名不再硬编码）

---

### A3. SliderTranslator —— 声明式 min/max/step/default_value

**问题**：Slider 当前由通用 StatefulComponentTranslator 处理，state_ctor 为 `|_w, _c| rml_ui::SliderState::new()`（无配置）。开发者必须在 on_loaded 中手动创建 SliderState 并链式调用 `.min().max().step().default_value()`，与 Input（支持声明式 placeholder/default_value/masked）形成鲜明对比。

**对比参考**：InputTranslator 从元素属性提取 placeholder/default_value/masked，注入到 state_ctor 闭包。SliderTranslator 应复制此模式，提取 min/max/step/default_value 注入 state_ctor。

**修复方案**：新建 `slider.rs` translator，参照 InputTranslator 模式。

#### 步骤 1：新建 translator/component/slider.rs

参照 [input.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/input.rs) 模式，创建 SliderTranslator：

```rust
const SKIP_ATTRS: &[&str] = &["min", "max", "step", "default_value"];
```

三条路径（与 InputTranslator 对称）：

1. **`value={field}` 路径** → 委托 StateBridge（`gen_model_state_bridge`），Slider 已在 STATE_BRIDGE_REGISTRY 注册
2. **ref/无绑定路径** → 提取 min/max/step/default_value，构建自定义 state_ctor，调用 `gen_stateful_body`

state_ctor 构建逻辑：
```rust
fn build_slider_state_ctor(elem, min, max, step, default_value, loop_vars, computed) -> String {
    // Static 属性 → 直接注入字面量
    // Bind 属性 → clone 前置，move 闭包捕获
    // 示例: |_w, _c| rml_ui::SliderState::new().min(0.0).max(100.0).step(1.0).default_value(50.0)
}
```

属性提取（复用 input.rs 的 extract_static_string / extract_static_bool 模式）：
- `min="0.0"` / `min={min_val}` → `.min(0.0)` / `.min(__rml_min)`
- `max="100.0"` / `max={max_val}` → `.max(100.0)` / `.max(__rml_max)`
- `step="1.0"` / `step={step_val}` → `.step(1.0)` / `.step(__rml_step)`
- `default-value="50.0"` / `default-value={default_val}` → `.default_value(50.0)` / `.default_value(__rml_default_value)`

> **default_value 类型**：Static 仅支持 f32（如 `default-value="50.0"`）；元组 `(f32, f32)` 范围滑块通过 Bind 表达式 `default-value={range_default}` 支持（`range_default: (f32, f32)` 字段）。

#### 步骤 2：stateful.rs —— 排除 Slider

[stateful.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/stateful.rs#L37) L37 排除列表增加 "Slider"：

```rust
if matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput" | "Input" | "TextInput" | "Slider") {
    return false;
}
```

#### 步骤 3：mod.rs —— 注册 SliderTranslator

[mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/component/mod.rs#L113) register_all 中，在 stateful 之前注册：

```rust
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    stateless::register(registry);
    input::register(registry);
    slider::register(registry);  // 新增
    stateful::register(registry);
    // ...
}
```

mod 声明区增加：`pub mod slider;`

#### 步骤 4：props_registry.rs —— 更新 Slider 条目

[props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) 增加 Slider 条目（仅列 on_change，min/max/step/default_value 由 translator 注入，不参与 setter 分发）：

```rust
("Slider", &["on_change"]),
```

#### 步骤 5：slider_case.rml —— 声明式改造

[slider_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/slider_case.rml) 改造：

```html
<!-- 基础用法：声明式 min/max/step/default-value -->
<Slider ref="slider_state" min="0" max="100" step="1" default-value="50" />

<!-- 禁用状态 -->
<Slider ref="disabled_state" min="0" max="100" default-value="30" disabled={true} />

<!-- 范围滑块：default-value 通过 Bind 传元组 -->
<Slider ref="range_state" min="0" max="100" step="5" default-value={range_default} />
```

描述 L4 移除 "需在 on_loaded 中初始化 SliderState Entity"，改为 "通过 min/max/step/default-value 属性声明式配置"。

#### 步骤 6：slider_case.rml.rs —— ElementRef 改造

[slider_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/slider_case.rml.rs) 改造：
- 字段：`Option<Entity<SliderState>>` → `ElementRef<SliderState>` + `range_default: (f32, f32)`
- on_loaded：移除手动 `SliderState::new().min().max()...` 创建，仅设置 `self.range_default = (20.0, 80.0)`
- API 表：min/max/step/default-value 从 "SliderState 构造器方法" 改为 "RML 属性"

**验证**：
- `cargo test -p rust-rml-engine -- slider` 通过
- `cargo build -p rust-rml-demo` 成功
- slider_case.rml 中不再有手动 SliderState 创建代码

---

## 三、Phase B —— 文档修复

### B2. 补全 API 表事件 payload 类型

**问题**：12 个 demo 的 API 表中，事件行仅写 "点击回调" 而未注明 payload 类型/签名，开发者无法从文档得知回调参数。

**涉及文件与修改**：

| 文件 | 行号 | 当前 | 修改为 |
|------|------|------|--------|
| counter_case.rml.rs | L42 | `"按钮点击回调"` | `"按钮点击回调（无参数）"` |
| avatar_case.rml.rs | L48 | `"点击回调"` | `"点击回调（ClickEvent）"` |
| alert_case.rml.rs | L47 | `"关闭按钮点击回调（ClickEvent）"` | `"关闭按钮点击回调（参数：&ClickEvent）"` |
| link_case.rml.rs | L41 | `"点击回调（ClickEvent）"` | `"点击回调（参数：&ClickEvent）"` |
| button_group_case.rml.rs | L53 | `"点击回调（ClickEvent）"` | `"点击回调（参数：&ClickEvent）"` |
| button_case.rml.rs | L49 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| menu_context_case.rml.rs | L39 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| menu_custom_case.rml.rs | L40 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| menu_dropdown_case.rml.rs | L41 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| menu_editor_case.rml.rs | L41 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |
| sidebar_case.rml.rs | L70 | `"点击事件回调"` | `"点击事件回调（参数：&ClickEvent）"` |
| tag_case.rml.rs | L44 | `"点击回调"` | `"点击回调（参数：&ClickEvent）"` |

**验证**：`grep -r '"事件", ".*回调"' demo/src/cases/*.rml.rs` 所有事件行均含 payload 信息

---

### B3. 清理 demo 描述中的 codegen 内部细节

**问题**：14 个 demo 的 description 属性或 API 注释暴露 codegen 内部实现（normalize_component_tag、gen_tree、tags.rs、state_field、codegen、StatelessWithItems 等），迫使开发者理解框架内部机制。违反"文档面向用户视角"原则。

**涉及文件与修改**：

| 文件 | 行号 | codegen 内部术语 | 修改方向 |
|------|------|------------------|----------|
| avatar_case.rml | L87 | `normalize_component_tag 统一为 Avatar` | `avatar 与 Avatar 标签等价` |
| alert_case.rml | L89 | `normalize_component_tag 统一为 Alert` | `alert 与 Alert 标签等价` |
| accordion_case.rml | L89 | `normalize_component_tag...StatelessWithItems codegen 自动收集` | `accordion 与 Accordion 等价。item 标签映射为 AccordionItem，自动收集为子项` |
| content_binding_case.rml | L4 | `codegen 对简单字段访问自动添加 & 前缀` | 移除 codegen 描述，改为用户视角的绑定说明 |
| key_case.rml | L4 | `codegen 生成 .id(...)` | 移除 codegen 描述，改为用户视角的 key 作用说明 |
| hover_card_case.rml | L102 | `normalize_component_tag 统一为 HoverCard` | `hover-card 与 HoverCard 标签等价` |
| input_case.rml | L69 | `前者走 component codegen...后者走 value 双向绑定 codegen` | `PascalCase Input 用于 ref + 事件订阅模式；小写 input 用于 value 双向绑定` |
| user_component_event_case.rml | L4 | `codegen 将父方法包装为闭包并注入` | `父视图用 on-click={handler} 绑定时，方法自动包装为闭包注入子组件` |
| table_case.rml | L80 | `normalize_component_tag 统一为 Table` | `table 与 Table 标签等价` |
| stepper_case.rml | L4 | `StatelessWithItems 组件` | 移除 codegen 分类术语，改为用户视角 |
| sheet_case.rml | L75 | `normalize_component_tag 统一为 Sheet` | `sheet 与 Sheet 标签等价` |
| sheet_case.rml | L76 | `codegen 直接使用 render 上下文的 _window 和 cx 变量` | 移除 codegen 描述 |
| popover_case.rml | L102 | `normalize_component_tag 统一为 Popover` | `popover 与 Popover 标签等价` |
| tree_case.rml | L4,14,34 | `gen_tree 硬编码...tags.rs state_field...` | 由 A2 改造后同步修复 |

> tree_case.rml 的 codegen 内部描述由 A2 改造时一并修复，不在此单独列出。

**验证**：`grep -rE "(gen_tree|tags\.rs|state_field|normalize_component_tag|StatelessWithItems|codegen)" demo/src/cases/*.rml` 无匹配（或仅剩 user_component_event_case 等无法完全移除的少量技术描述）

---

## 四、执行顺序与依赖关系

```
Phase A（反人类模式修复）─── A1/A2/A3 可并行
  │
  ├── A1: open-ixs → open-indices（独立，最简单）
  ├── A2: Tree StatefulWithDelegate 改造（独立，中等复杂度）
  └── A3: SliderTranslator 新建（独立，参照 InputTranslator 模式）
  │
  ▼
Phase B（文档修复）─── B2/B3 可并行，依赖 Phase A 完成
  │
  ├── B2: API 表事件 payload 补全（独立）
  └── B3: 描述 codegen 内部清理（tree_case.rml 依赖 A2 完成）
  │
  ▼
最终验证
  ├── cargo build -p rust-rml-engine
  ├── cargo test -p rust-rml-engine --lib
  └── cargo build -p rust-rml-demo
```

---

## 五、假设与决策

1. **A2 Tree::new 签名**：当前 `Tree::new(state: Option<&Entity<TreeState>>)`。tree.rs 生成 `Tree::new(Some(&__rml_entity))` 适配现有签名，不修改 ui crate 的 Tree::new。若后续统一为 `Tree::new(&Entity<TreeState>)`，可再迭代。

2. **A2 Tree 专用 translator 保留**：Tree 的 on_activate/on_select 事件需要专用 event_setter（生成 `.on_activate_rc(Rc::new(...))` + weak_entity 闭包），不迁入通用 component_event_setter。tree.rs translator 保留，仅改造构造逻辑。

3. **A3 default_value 元组支持**：Static 属性仅支持 f32（`default-value="50.0"`）；元组 `(f32, f32)` 范围滑块通过 Bind 表达式 `default-value={range_default}` 支持。不解析 Static 字符串为元组（避免复杂解析逻辑）。

4. **A3 value={field} 路径**：Slider 已在 STATE_BRIDGE_REGISTRY 注册，`value={field}` 走 StateBridge 双向绑定。此路径不支持 min/max/step 声明式配置（使用 SliderState::new() 默认值）。开发者需要自定义配置时使用 ref + min/max/step 路径。

5. **B2 ClickEvent payload**：GPUI 的 click 事件回调签名为 `Fn(&ClickEvent, &mut Window, &mut App)`。文档中统一标注 "参数：&ClickEvent"。无参数的事件标注 "无参数"。

6. **B3 技术描述边界**：描述中允许保留用户需知的框架概念（如 "Stateful 组件"、"ref 指令"、"双向绑定"），但移除实现细节（如 "gen_tree"、"tags.rs"、"normalize_component_tag"、"codegen 生成"）。

---

## 六、验证检查清单

- [ ] A1: `grep -r "open.ixs" crates/ demo/` 无匹配
- [ ] A1: `cargo test -p rust-rml-engine -- accordion` 通过
- [ ] A2: `grep -r "tree_state" demo/src/cases/tree_case` 无匹配（字段名不再硬编码）
- [ ] A2: tree_case.rml 使用 `ref="basic_tree" items={tree_items}` 声明式绑定
- [ ] A2: `cargo test -p rust-rml-engine -- tree` 通过
- [ ] A3: slider_case.rml 使用 `min="0" max="100" step="1" default-value="50"` 声明式配置
- [ ] A3: slider_case.rml.rs 中无手动 `SliderState::new().min()...` 创建代码
- [ ] A3: `cargo test -p rust-rml-engine -- slider` 通过
- [ ] B2: 所有 demo API 表事件行均含 payload 类型
- [ ] B3: `grep -rE "(gen_tree|tags\.rs|state_field|normalize_component_tag|StatelessWithItems)" demo/src/cases/*.rml` 无匹配
- [ ] 最终: `cargo build -p rust-rml-engine` 成功
- [ ] 最终: `cargo test -p rust-rml-engine --lib` 通过
- [ ] 最终: `cargo build -p rust-rml-demo` 成功
