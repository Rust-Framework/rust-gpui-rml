# CSS 变量动态注入 + 元素级 style 语法支持迭代计划

## 摘要

本计划分两阶段完善 RML 的 CSS 基础能力：

- **阶段 1**：CSS 变量动态注入 — 扩展 `:root` 变量的运行时支持，从仅颜色变量扩展到所有变量类型（长度、数字、关键字），实现完整的主题切换能力
- **阶段 2**：元素级 `style` 语法支持 — 修复扩展组件 `style="..."` 被丢弃的 bug、样式优先级倒置 bug、`style` 未注册的问题，并明确 `style` vs 归一化属性 vs 组件专用属性的边界

---

## 现状分析

### CSS 变量系统

#### 当前能力

| 变量类型 | 构建期 | 运行时 | 示例 |
|---------|--------|--------|------|
| 颜色变量 | ✅ `resolve_var` 内联 | ✅ `rml::theme::color()` | `--primary: #007bff` |
| 非颜色变量 | ✅ `resolve_var` 内联 | ❌ 不支持 | `--spacing: 8px` |

- **颜色属性**（`color`/`background`/`border-color`）：`Value::Var` → 生成 `rml::theme::color("--name")` 运行时查询（[mapper.rs:435-438](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L435-L438)）
- **非颜色属性**（`padding`/`font-size`/`opacity` 等）：`resolve_var()` 在构建期查找 `StyleSheet.variables` 内联值（[mapper.rs:371-388](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L371-L388)）

#### 关键 Gap

1. **非颜色变量无法运行时切换**：`--spacing: 8px` 在构建期内联为 `gpui::px(8.0)`，主题切换时不会更新
2. **变量仅来源构建期 CSS**：`ThemeState` 仅存储颜色变量（`parse_theme_css` 只提取颜色，[theme.rs:292](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs#L292)），非颜色变量不在运行时主题中
3. **未定义变量静默丢失**：若 `var(--undefined)` 既不在构建期 `StyleSheet.variables` 也不在运行时主题中，`resolve_var` 返回未解析的 `Value::Var`，大多数 `length_method` 无法处理 → 静默丢弃

### 元素级 style 系统

#### 当前能力

| 元素类型 | `style="..."` | 归一化属性 (`padding="..."`) | CSS class |
|---------|--------------|---------------------------|-----------|
| 原生元素 (div/span) | ✅ `apply_inline_style` | ✅ `apply_style_attr` | ✅ `apply_css_styles` |
| 扩展组件 (Button/Input) | ❌ **被丢弃** | ✅ `apply_style_attr` | ✅ `apply_css_styles` |

#### 三个已知 Bug

1. **`style` 对扩展组件被静默丢弃**：[setters.rs:154](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs#L154) `"class" | "id" | "style" | "src" | "type" | "value" => None`
2. **扩展组件样式优先级倒置**：translator 先调用 `gen_xxx()`（含 setter + 归一化属性），后调用 `apply_css_styles`（CSS class）→ CSS class 覆盖归一化属性
3. **`"style"` 未在 `props_registry` 注册**：`is_prop_registered("style")` 返回 `false`

#### margin/padding 归一化（已完善）

[mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) 已实现完整的 HTML 简写→GPUI 方法变体映射：
- `padding: 10px` → `.p(gpui::px(10.))`
- `padding: 10px 20px` → `.py(gpui::px(10.)).px(gpui::px(20.))`
- `padding: 10px 20px 30px 40px` → `.pt().pr().pb().pl()`
- margin 同理

**无需改动**，已向 HTML 心智靠拢。

#### 三层样式优先级（目标）

```
最高 →  归一化属性 (padding="20px")     — 最具体
        ↓
        内联 style (style="padding:10px") — 内联 CSS
        ↓
        页面 CSS class (L2)               — 页面样式表
        ↓
最低 →  应用 CSS class (L1)               — 全局样式表
```

GPUI "last write wins" → 代码生成顺序：L1 → L2（合并）→ style → 归一化属性

---

## 提议变更

### 阶段 1：CSS 变量动态注入

#### 1.1 扩展 `ThemeState` 存储所有变量类型

**文件**：[crates/core/src/theme.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)

**变更**：

1. 新增 `ThemeVar` 枚举，表示运行时变量值：
```rust
/// 运行时 CSS 变量值（支持主题切换）
#[derive(Debug, Clone, Copy)]
pub enum ThemeVar {
    /// 颜色变量 `--primary: #007bff`
    Color(Rgba),
    /// 长度变量 `--spacing: 8px`（像素值）
    Length(f32),
    /// 数字变量 `--opacity: 0.5`
    Number(f32),
}
```

2. `ThemeState` 新增字段：
```rust
pub struct ThemeState {
    // ... 现有字段 ...
    /// `:root` 非颜色变量（`--spacing` → `ThemeVar::Length(8.0)`）
    base_vars: HashMap<String, ThemeVar>,
    /// 当前主题的非颜色变量（覆盖 base_vars）
    theme_vars: HashMap<String, ThemeVar>,
}
```

3. 新增方法：
```rust
impl ThemeState {
    /// 查询非颜色变量（合并 base + theme，theme 优先）
    pub fn var(&self, name: &str) -> Option<ThemeVar> {
        self.theme_vars.get(name).copied()
            .or_else(|| self.base_vars.get(name).copied())
    }
    
    /// 设置基础变量（由 `set_style` 调用）
    pub fn set_base_vars(&mut self, vars: HashMap<String, ThemeVar>) {
        self.base_vars = vars;
        self.merge_vars();
    }
    
    fn merge_vars(&mut self) {
        // 合并逻辑：theme_vars 覆盖 base_vars
    }
}
```

#### 1.2 扩展 `set_style()` 提取所有 `:root` 变量

**文件**：[crates/core/src/theme.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)

**变更**：`set_style()` 当前调用 `parse_theme_css()` 仅提取颜色。新增 `parse_theme_vars()` 提取所有变量类型：

```rust
/// 解析 :root 块中所有变量（颜色 + 非颜色）
fn parse_theme_vars(css: &str) -> (HashMap<String, Rgba>, HashMap<String, ThemeVar>) {
    // 复用 engine 的 css::parse 解析完整 StyleSheet
    // 颜色值 → HashMap<String, Rgba>（现有行为）
    // 非颜色值（Length/Number）→ HashMap<String, ThemeVar>
}
```

`set_style()` 调用后同时设置 `base_colors` 和 `base_vars`。

**注意**：`core` crate 不能依赖 `engine` crate（避免循环依赖）。`parse_theme_vars` 需在 `core` 内实现独立解析逻辑，或由 `engine` 在构建期预处理后通过 generated 代码注入。

**方案选择**：在 `core` 中增强现有 `parse_theme_css` 为 `parse_theme_vars`，复用现有的字符串解析逻辑（不依赖 engine 的 CSS parser），增加对 `px`/`pt`/`%`/纯数字的识别。

#### 1.3 新增运行时查询函数

**文件**：[crates/core/src/theme.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)

```rust
/// 运行时查询长度变量（供 codegen 生成代码调用）
pub fn length(name: &str) -> gpui::Pixels {
    with_theme_state(|state| {
        match state.var(name) {
            Some(ThemeVar::Length(n)) => gpui::px(n),
            _ => gpui::px(0.0),
        }
    })
}

/// 运行时查询数字变量
pub fn number(name: &str) -> f32 {
    with_theme_state(|state| {
        match state.var(name) {
            Some(ThemeVar::Number(n)) => n,
            _ => 0.0,
        }
    })
}
```

#### 1.4 更新 mapper.rs 为非颜色 var() 生成运行时查询

**文件**：[crates/engine/src/css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)

**变更**：当前 `map_declaration` 对非颜色属性先调用 `resolve_var()` 尝试构建期内联。改为：

1. **构建期可解析的 var()**（变量在 `StyleSheet.variables` 中）：保持构建期内联（零运行时开销，适合不参与主题切换的变量）
2. **构建期不可解析的 var()**（变量不在 `StyleSheet.variables` 中）：生成运行时查询

```rust
fn map_declaration(decl: &Declaration, vars: &HashMap<String, Value>) -> Option<String> {
    let prop = decl.property.as_str();

    // 颜色属性：保持现有逻辑（始终生成运行时查询）
    match prop {
        "background" | "background-color" => return color_method("bg", &decl.value, vars),
        "color" => return color_method("text_color", &decl.value, vars),
        _ => {}
    }

    // 非颜色属性：先尝试构建期解析
    let resolved = resolve_var(&decl.value, vars);
    
    // 如果 resolve_var 成功内联（值类型变了），用内联值
    if !matches!(resolved, Value::Var(_, _)) {
        return map_non_color_property(prop, &resolved, vars);
    }
    
    // resolve_var 失败（var 不在构建期 vars 中）：生成运行时查询
    if let Value::Var(name, _) = &decl.value {
        return runtime_var_method(prop, name);
    }
    
    map_non_color_property(prop, &resolved, vars)
}

/// 为非颜色属性的 var() 生成运行时查询
fn runtime_var_method(prop: &str, var_name: &str) -> Option<String> {
    match prop {
        // 长度类属性 → rml::theme::length("--name")
        "padding" => Some(format!("p(rml::theme::length({:?}))", var_name)),
        "padding-top" => Some(format!("pt(rml::theme::length({:?}))", var_name)),
        "padding-bottom" => Some(format!("pb(rml::theme::length({:?}))", var_name)),
        "padding-left" => Some(format!("pl(rml::theme::length({:?}))", var_name)),
        "padding-right" => Some(format!("pr(rml::theme::length({:?}))", var_name)),
        "margin" => Some(format!("m(rml::theme::length({:?}))", var_name)),
        "margin-top" => Some(format!("mt(rml::theme::length({:?}))", var_name)),
        "margin-bottom" => Some(format!("mb(rml::theme::length({:?}))", var_name)),
        "margin-left" => Some(format!("ml(rml::theme::length({:?}))", var_name)),
        "margin-right" => Some(format!("mr(rml::theme::length({:?}))", var_name)),
        "width" => Some(format!("w(rml::theme::length({:?}))", var_name)),
        "height" => Some(format!("h(rml::theme::length({:?}))", var_name)),
        "font-size" => Some(format!("text_size(rml::theme::length({:?}))", var_name)),
        "gap" => Some(format!("gap(rml::theme::length({:?}))", var_name)),
        "border-radius" => Some(format!("rounded(rml::theme::length({:?}))", var_name)),
        // 数字类属性 → rml::theme::number("--name")
        "opacity" => Some(format!("opacity(rml::theme::number({:?}))", var_name)),
        "flex-grow" => Some(format!("flex_grow(rml::theme::number({:?}))", var_name)),
        // 简写属性（padding: var(--spacing)）不支持运行时拆分，仅支持单值
        // 多值简写（padding: var(--a) var(--b)）需构建期解析，不支持运行时
        _ => None,
    }
}
```

**设计决策**：
- 简写属性（`padding: 10px 20px`）的 `var()` 仅在构建期解析，不生成运行时查询（因为运行时无法拆分 `py`/`px`）
- 单值属性（`padding: var(--spacing)`）生成 `.p(rml::theme::length("--spacing"))`
- 这与 HTML CSS 行为一致：`var()` 在简写中需要浏览器支持计算，RML 退化到构建期解析

---

### 阶段 2：元素级 style 语法支持

#### 2.1 修复 `style` 对扩展组件被丢弃

**文件**：[crates/engine/src/compiler/setters.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs)

**变更**：[setters.rs:154](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/setters.rs#L154) 从 `None` 分支移除 `"style"`，新增 `style` 处理：

```rust
// 修改前：
"class" | "id" | "style" | "src" | "type" | "value" => None,

// 修改后：
"class" | "id" | "src" | "type" | "value" => None,
// style 属性：复用 apply_inline_style，对扩展组件也生效
// （gpui-component 实现 Styled trait，支持所有 Styled 方法）
"style" => {
    let code = crate::compiler::codegen::attribute::apply_inline_style(value);
    if code.is_empty() { None } else { Some(code) }
}
```

**注意**：`apply_inline_style` 当前是 `attribute.rs` 中的私有函数。需要改为 `pub(crate)` 可见性。

#### 2.2 修复扩展组件样式优先级倒置

**问题**：所有扩展组件 translator 的代码生成顺序为：
1. `gen_xxx()` — 组件专用 setter + 归一化属性
2. `apply_css_styles()` — CSS class

导致 CSS class 覆盖归一化属性（违反优先级）。

**修复策略**：在所有扩展组件 translator 中，将 `apply_css_styles` 调用移到 `gen_xxx` 之前。

**受影响文件**（需逐一审计）：
- `crates/engine/src/compiler/translator/component/code_editor.rs`
- `crates/engine/src/compiler/translator/component/tabs.rs`
- 其他所有 `translator/component/*.rs` 文件

**统一修复模式**：
```rust
// 修改前（优先级倒置）：
let mut code = gen_button(elem, ctx, ...)?;  // setter + 归一化属性
code.push_str(&apply_css_styles(elem, tag, sheet, parents));  // CSS class（覆盖前者）

// 修改后（正确优先级）：
let mut code = apply_css_styles(elem, tag, sheet, parents);  // CSS class（基础）
code.push_str(&gen_button(elem, ctx, ...)?);  // setter + 归一化属性（覆盖前者）
```

**审计清单**：搜索所有调用 `apply_css_styles` 的 translator 文件，确认顺序。

#### 2.3 在 props_registry 注册 `style`

**文件**：[crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)

**变更**：在 `COMMON_STATIC_PROPS` 中添加 `"style"`：

```rust
pub const COMMON_STATIC_PROPS: &[&str] = &[
    // ... 现有属性 ...
    "style",  // 新增：内联 CSS 样式字符串
];
```

这确保 `is_prop_registered("style")` 返回 `true`，通过 props_registry 一致性测试。

#### 2.4 明确属性分类边界

**文档化属性三层分类**（写入代码注释 + 计划文档）：

| 分类 | 语法 | 处理路径 | 适用元素 | 示例 |
|------|------|---------|---------|------|
| **内联 style** | `style="..."` | `apply_inline_style` → `css::mapper` | 所有元素 | `style="padding: 10px; color: red;"` |
| **归一化属性** | `property="value"` | `apply_style_attr` → `css::mapper` | 所有元素 | `padding="10px"` `color="red"` |
| **组件专用属性** | `attr="..."` | `component_static_setter` | 仅扩展组件 | `label="OK"` `primary=""` |
| **原生专用属性** | `attr="..."` | `apply_static_attr` match | 仅原生元素 | `href="..."` `src="..."` |

**设计原则**：
1. `style="..."` 是 CSS 字符串，支持任意 CSS 属性组合，统一入口
2. 归一化属性是 `style` 的语法糖，每个属性等价于 `style` 中的一条声明
3. 两者共存时，归一化属性优先级更高（更具体）—— GPUI "last write wins"：`style` 先应用，归一化属性后应用
4. 组件专用属性（`label`/`primary`/`disabled`）不是 CSS 属性，独立于样式体系

#### 2.5 确保 `apply_inline_style` 可见性

**文件**：[crates/engine/src/compiler/codegen/attribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs)

**变更**：将 `apply_inline_style` 从 `fn`（私有）改为 `pub(crate) fn`，供 `setters.rs` 调用：

```rust
// 修改前：
fn apply_inline_style(style_str: &str) -> String {

// 修改后：
pub(crate) fn apply_inline_style(style_str: &str) -> String {
```

---

## 假设与决策

### 阶段 1 决策
1. **构建期 vs 运行时变量解析**：构建期 `StyleSheet.variables` 中存在的变量保持构建期内联（零开销）；仅在运行时主题中存在的变量生成运行时查询
2. **ThemeVar 类型**：仅支持 `Color`/`Length`/`Number` 三种，不支持 `Keyword`/`String`（这些属性通常不参与主题切换）
3. **简写属性的 var()**：`padding: var(--spacing) var(--spacing-y)` 等多值简写仅在构建期解析，不生成运行时查询
4. **core crate 独立解析**：`parse_theme_vars` 在 `core` 中独立实现，不依赖 `engine` 的 CSS parser（避免循环依赖）

### 阶段 2 决策
1. **`style` 语法保持字符串形式**：`style="padding: 10px; color: red;"`，不引入 `style={{}}` Rust 对象语法（GPUI 构建器模式是编译期的，运行时动态样式对象无意义）
2. **归一化属性 > style > CSS class**：归一化属性作为更具体的写法覆盖 `style`，`style` 覆盖 CSS class
3. **所有扩展组件 translator 统一修复**：不逐个修改，而是审计所有文件后批量修复
4. **margin/padding 归一化无需改动**：mapper.rs 已完善 HTML 简写→GPUI 方法变体映射

### 共同假设
1. GPUI `Styled` trait 方法接受 `gpui::Pixels`/`f32`/`gpui::Rgba` 等具体类型，运行时查询函数返回对应类型
2. 主题切换时 `refresh_windows()` 触发重渲染，运行时变量查询自动更新

---

## 验证步骤

### 阶段 1 验证
- [ ] `ThemeState` 存储 + 查询非颜色变量单元测试
- [ ] `parse_theme_vars` 提取颜色 + 非颜色变量单元测试
- [ ] `rml::theme::length()` / `rml::theme::number()` 运行时查询单元测试
- [ ] mapper.rs 对未定义 var() 生成运行时查询的单元测试
- [ ] 端到端测试：`padding: var(--spacing)` 在主题切换时更新

### 阶段 2 验证
- [ ] `style="..."` 对扩展组件（Button/Input）生效的端到端测试
- [ ] 扩展组件样式优先级：归一化属性 > style > CSS class 的端到端测试
- [ ] `props_registry` 一致性测试通过（`style` 已注册）
- [ ] `cargo build --workspace` 编译通过
- [ ] `cargo test --workspace --exclude rust-rml-demo --lib` 全部通过

### 回归验证
- [ ] 原生元素 `style="..."` 行为不变
- [ ] 原有 CSS class 匹配行为不变
- [ ] 颜色变量运行时查询行为不变

---

## 实施顺序

### 阶段 1：CSS 变量动态注入
1. **修改 `crates/core/src/theme.rs`** — 新增 `ThemeVar` 枚举、`ThemeState.base_vars`/`theme_vars` 字段、`var()` 方法
2. **修改 `crates/core/src/theme.rs`** — 新增 `parse_theme_vars()` 函数，增强 `set_style()` 提取所有变量
3. **修改 `crates/core/src/theme.rs`** — 新增 `rml::theme::length()` / `rml::theme::number()` 运行时查询函数 + 单元测试
4. **修改 `crates/engine/src/css/mapper.rs`** — `map_declaration` 对未定义 var() 生成运行时查询 + 单元测试
5. `cargo test -p rust-rml-core --lib` — core 测试验证
6. `cargo test -p rust-rml-engine --lib` — engine 测试验证

### 阶段 2：元素级 style 语法支持
7. **修改 `crates/engine/src/compiler/codegen/attribute.rs`** — `apply_inline_style` 改为 `pub(crate)`
8. **修改 `crates/engine/src/compiler/setters.rs`** — 移除 `"style"` 从 None 分支，新增 style 处理
9. **修改 `crates/engine/src/compiler/props_registry.rs`** — `COMMON_STATIC_PROPS` 添加 `"style"`
10. **审计 + 修复所有扩展组件 translator** — 搜索 `apply_css_styles` 调用，调整为正确优先级顺序
11. **新增端到端测试** — style 对扩展组件生效 + 优先级验证
12. `cargo build --workspace` — 编译验证
13. `cargo test --workspace --exclude rust-rml-demo --lib` — 全量测试验证
