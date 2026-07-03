# RML 命名规范统一与 DescriptionList 增强计划

## 概述

本计划回应四项用户反馈：
1. **`items` 绑定缺失**：`<descriptions items={desitems} ...>` 应受支持（类似 Table 的 `rows={api_rows}`）
2. **`size` 属性**：使用 `size="small"` / `size={size_value}` 替代 `small=""` / `large=""` / `xsmall=""` 布尔标志
3. **`vertical`/`horizontal` 简化**：默认横向，仅提供 `vertical="true"` 或 `vertical={is_vertical}`，移除 `horizontal`
4. **全局 kebab-case 强制**：所有 RML 属性名和标签名必须遵循 `label-width` 标准，**严格禁止下划线**

用户澄清决策：
- `onclick` / `onchange`（单词，无下划线）：**保持不变**
- 下划线兼容性：**严格禁止** —— 解析器应拒绝下划线属性，所有 .rml 文件必须更新
- `size` 属性：**完全替代** `small` / `xsmall` / `large` 布尔标志

---

## 现状分析

### 1. 解析器层（无属性名规范化）

- `crates/engine/src/parser/tokenizer.rs` 第 277 行 `read_attr_name()` 同时接受 `-` 和 `_`
- `crates/engine/src/parser/mod.rs` 第 223-232 行 `build_element()` 将 `attr.name` **原样存储**到 `Attribute::Static/Bind`
- **结论**：不存在属性名规范化层；标签有 `canonical_tag()` 规范化，但属性没有

### 2. 设置器层（snake_case 硬编码）

所有 setter match 分支使用 snake_case：
- `crates/engine/src/compiler/component.rs` 第 292 行：`"small" | "xsmall" | "large"` → `.small()` 等
- `crates/engine/src/compiler/component.rs` 第 307-322 行：`font_thin`/`h_flex`/`v_flex` 等
- `crates/engine/src/compiler/description_list/setters.rs`：`vertical`/`horizontal`/`bordered`/`columns`/`label_width`
- `crates/engine/src/compiler/tab_bar/setters.rs`：`selected_index`/`last_empty_space`/`title_icon`/`on_click`
- `crates/engine/src/compiler/tree/setters.rs`：`on_activate`
- `crates/engine/src/compiler/accordion/setters.rs`：`on_toggle_click`
- `crates/engine/src/compiler/codegen/shell.rs`：`selected_index`/`show_chrome`/`left_size`/`right_size`/`bottom_size`/`on_tab_click`/`on_chrome_toggle`/`tab_item_template`
- `crates/engine/src/compiler/menu/popup.rs`：`min_w`/`max_w`/`max_h`/`check_side`/`external_link_icon`

### 3. 注册表（snake_case 硬编码）

`crates/engine/src/compiler/props_registry.rs`：
- `COMMON_STATIC_PROPS`：含 `small`/`xsmall`/`large`/`font_*`/`h_flex`/`v_flex`
- `COMPONENT_PROPS`：DescriptionList 含 `vertical`/`horizontal`/`bordered`/`columns`/`label_width`
- `SHELL_PROPS`：含 `selected_index`/`show_chrome`/`left_size`/`right_size`/`bottom_size`/`on_tab_click`/`on_chrome_toggle`
- **缺失**：`tab_item_template` 未在 SHELL_PROPS 中注册（预存问题）

### 4. Size 枚举未导出

`crates/ui/src/lib.rs` 第 71-73 行 re-export 了 `Sizable` trait，但 **`Size` 枚举未显式 re-export**。codegen 需要引用 `rml_ui::Size::Small` 等。

### 5. DescriptionList 设置器局限

- `vertical` / `horizontal`：仅支持静态布尔（`vertical=""`），不支持 `vertical={is_vertical}` 绑定
- 无 `items` 绑定设置器
- DescriptionList API 已有 `children(impl IntoIterator<Item = impl Into<DescriptionItem>>)` 方法可用

### 6. .rml 文件中的 snake_case 属性（需迁移）

20 个 .rml 文件使用 snake_case 属性，涉及属性清单：
- 布尔标志：`v_flex`/`h_flex`/`small`/`xsmall`/`large`/`font_*`
- 组件属性：`selected_index`/`label_width`/`last_empty_space`/`title_icon`/`max_h`/`min_w`/`max_w`/`check_side`/`external_link_icon`/`gap_4`
- 事件：`on_activate`/`on_tab_click`/`on_chrome_toggle`/`on_toggle_click`/`on_click`（TabBar/Tab 专用）
- Shell 属性：`show_chrome`/`left_size`/`right_size`/`bottom_size`/`tab_item_template`

---

## 设计决策

### 决策 1：解析器层规范化（非设置器层）

**方案**：在 `build_element()` 中添加 `normalize_attr_name()` 函数，将 `-` 转换为 `_`。

**理由**：
- 所有现有 setter match 分支和注册表条目保持 snake_case 不变
- 仅需修改解析器一处，无需触碰 7+ 个 setter 文件
- 用户写 `label-width` → 解析器规范化为 `label_width` → 命中现有 setter
- 单词属性 `onclick`/`bordered`/`columns` 无 `-`，不受影响

### 决策 2：严格禁止下划线

**方案**：在 `read_attr_name()` 中移除 `_` 接受；若遇到 `_` 则终止属性名读取，触发解析错误。

**理由**：用户明确要求"严格禁止下划线"，确保命名规范强制性。

### 决策 3：`size` 属性替代布尔标志

**方案**：
- 静态：`size="small"` → `.with_size(rml_ui::Size::Small)`
- 静态：`size="xsmall"` → `.with_size(rml_ui::Size::XSmall)`
- 静态：`size="large"` → `.with_size(rml_ui::Size::Large)`
- 静态：`size="medium"` → `.with_size(rml_ui::Size::Medium)`（显式中等）
- 绑定：`size={size_value}` → `.with_size(self.size_value)`
- 从 `COMMON_STATIC_PROPS` 移除 `small`/`xsmall`/`large`，添加 `size`
- 从 `component.rs` 移除 `small`/`xsmall`/`large` match 分支，添加 `size` 静态+绑定分支
- 添加 `size` 到 `COMMON_BIND_PROPS`
- re-export `Size` 枚举从 `rml_ui`

### 决策 4：`vertical` 简化与绑定支持

**方案**：
- 移除 `horizontal` 属性（从注册表和 setter）
- `vertical=""` / `vertical="true"` → `.layout(gpui::Axis::Vertical)`（保持现有行为）
- `vertical="false"` → 不生成 `.layout()` 调用（默认横向）
- `vertical={is_vertical}` → `.layout(if self.is_vertical { gpui::Axis::Vertical } else { gpui::Axis::Horizontal })`
- 添加 `vertical` 到 DescriptionList 的 bind setter

### 决策 5：`items` 绑定

**方案**：
- `items={descriptions}` → `.children(self.descriptions.clone())`
- DescriptionList 的 `children()` 方法 **扩展** Vec（与 inline `<description>` 子元素共存）
- 添加 `items` 到 DescriptionList 的 `COMPONENT_PROPS` 和 bind setter

---

## 实施步骤

### 阶段 1：解析器规范化层

**文件**：`crates/engine/src/parser/tokenizer.rs`

修改 `read_attr_name()`（第 274-293 行）：
- 移除 `c == '_'` 条件，仅保留 `c.is_alphanumeric() || c == '-' || c == ':'`
- 当遇到 `_` 时停止读取（自然终止），后续会触发解析错误

**文件**：`crates/engine/src/parser/mod.rs`

在 `build_element()` 中添加规范化：
1. 新增辅助函数 `normalize_attr_name(name: &str) -> String`：将 `-` 替换为 `_`
2. 在第 213 行 `name if name.starts_with("on")` 分支前，对 `attr.name` 调用 `normalize_attr_name()`
3. 在第 223-232 行 catch-all 分支中，使用规范化后的 name 存储到 `Attribute::Static`/`Attribute::Bind`

**关键**：指令属性（`if`/`each`/`model`/`show`/`once`/`html`/`ref`/`slot`）和事件属性（`on*`）都需经过规范化。最简方案是在 `for attr in raw_attrs` 循环开头对 `attr.name` 统一规范化。

### 阶段 2：Size 属性

**文件**：`crates/ui/src/lib.rs`

在第 72 行 re-export 中添加 `Size`：
```rust
pub use gpui_component::{
    button::ButtonVariants, ActiveTheme, Disableable, Selectable, Sizable, Size, StyledExt,
};
```
（注：需验证 `Size` 的具体导出路径，可能为 `gpui_component::Size` 或 `gpui_component::sizable::Size`）

**文件**：`crates/engine/src/compiler/props_registry.rs`

- 从 `COMMON_STATIC_PROPS` 移除 `"small"`, `"xsmall"`, `"large"`
- 在 `COMMON_STATIC_PROPS` 添加 `"size"`
- 在 `COMMON_BIND_PROPS` 添加 `"size"`

**文件**：`crates/engine/src/compiler/component.rs`

- 移除第 291-298 行 `"small" | "xsmall" | "large"` match 分支
- 添加 `size` 静态 setter 分支（在 `component_static_setter`）：
  ```rust
  "size" => {
      let size = match value {
          "xsmall" => "rml_ui::Size::XSmall",
          "small" => "rml_ui::Size::Small",
          "medium" => "rml_ui::Size::Medium",
          "large" => "rml_ui::Size::Large",
          _ => return None,
      };
      Some(format!(".with_size({})", size))
  }
  ```
- 添加 `size` 绑定 setter 分支（在 `component_bind_setter`）：
  ```rust
  "size" => Some(format!(".with_size({})", rust_expr)),
  ```

### 阶段 3：DescriptionList 增强

**文件**：`crates/engine/src/compiler/props_registry.rs`

修改 DescriptionList 条目（第 104 行）：
- 移除 `"horizontal"`
- 添加 `"items"`
- 结果：`("DescriptionList", &["vertical", "bordered", "columns", "label_width", "items"])`

**文件**：`crates/engine/src/compiler/description_list/setters.rs`

`static_setter` 修改：
- 移除 `"horizontal"` 分支（第 36-42 行）
- 更新 `"vertical"` 分支：`vertical="false"` 时返回 `None`（默认横向，不生成 layout 调用）

`bind_setter` 修改：
- 添加 `"vertical"` 分支：
  ```rust
  "vertical" => Some(format!(
      ".layout(if {} {{ gpui::Axis::Vertical }} else {{ gpui::Axis::Horizontal }})",
      rust_expr
  ))
  ```
- 添加 `"items"` 分支：
  ```rust
  "items" => Some(format!(".children({}.clone())", rust_expr))
  ```

更新模块文档注释（移除 `horizontal` 描述，添加 `vertical` 绑定和 `items` 绑定描述）。

更新测试：
- 移除 `static_setter_horizontal` 测试
- 添加 `bind_setter_vertical` 测试
- 添加 `bind_setter_items` 测试

### 阶段 4：全局 .rml 文件迁移

**4.1 布尔标志 → size 属性**（4 个文件）

| 文件 | 旧 | 新 |
|------|----|----|
| `demo/src/cases/avatar_case.rml` | `large=""` / `small=""` | `size="large"` / `size="small"` |
| `demo/src/cases/accordion_case.rml` | `small=""` / `large=""` | `size="small"` / `size="large"` |
| `demo/src/cases/button_case.rml` | `small=""` / `large=""` | `size="small"` / `size="large"` |
| `demo/src/cases/tab_bar_case.rml` | `xsmall=""` / `small=""` / `large=""` | `size="xsmall"` / `size="small"` / `size="large"` |

**4.2 snake_case → kebab-case 属性迁移**（20 个文件）

属性名映射表：

| snake_case（旧） | kebab-case（新） |
|------------------|------------------|
| `v_flex` | `v-flex` |
| `h_flex` | `h-flex` |
| `font_thin` | `font-thin` |
| `font_extralight` | `font-extralight` |
| `font_light` | `font-light` |
| `font_normal` | `font-normal` |
| `font_medium` | `font-medium` |
| `font_semibold` | `font-semibold` |
| `font_bold` | `font-bold` |
| `font_extrabold` | `font-extrabold` |
| `font_black` | `font-black` |
| `selected_index` | `selected-index` |
| `label_width` | `label-width` |
| `last_empty_space` | `last-empty-space` |
| `title_icon` | `title-icon` |
| `max_h` | `max-h` |
| `min_w` | `min-w` |
| `max_w` | `max-w` |
| `check_side` | `check-side` |
| `external_link_icon` | `external-link-icon` |
| `gap_4` | `gap-4` |
| `show_chrome` | `show-chrome` |
| `left_size` | `left-size` |
| `right_size` | `right-size` |
| `bottom_size` | `bottom-size` |
| `tab_item_template` | `tab-item-template` |
| `on_activate` | `on-activate` |
| `on_tab_click` | `on-tab-click` |
| `on_chrome_toggle` | `on-chrome-toggle` |
| `on_toggle_click` | `on-toggle-click` |
| `on_click`（TabBar/Tab 专用） | `on-click` |

**注意**：`onclick`（单词，Button 等通用组件）保持不变；仅 TabBar/Tab 的 `on_click` 迁移为 `on-click`（解析器规范化回 `on_click` 命中现有 setter）。

需迁移的文件清单：
1. `demo/src/shell/main_window.rml` — `on_tab_click`/`on_chrome_toggle`/`selected_index`/`show_chrome`/`left_size`/`right_size`/`bottom_size`
2. `demo/src/shell/activity_panel.rml` — `on_activate`
3. `demo/src/lsp/lsp_explorer_panel.rml` — `on_activate`
4. `demo/src/cases/accordion_case.rml` — `v_flex`/`on_toggle_click` + size
5. `demo/src/cases/avatar_case.rml` — `v_flex`/`h_flex`/`gap_4` + size
6. `demo/src/cases/button_case.rml` — `v_flex` + size
7. `demo/src/cases/counter_case.rml` — `v_flex`
8. `demo/src/cases/description_list_case.rml` — `v_flex`/`label_width` + 新增 items/vertical 演示
9. `demo/src/cases/i18n_case.rml` — `v_flex`
10. `demo/src/cases/menu_context_case.rml` — `v_flex`
11. `demo/src/cases/menu_custom_case.rml` — `v_flex`
12. `demo/src/cases/menu_dropdown_case.rml` — `v_flex`
13. `demo/src/cases/menu_editor_case.rml` — `v_flex`/`check_side`
14. `demo/src/cases/menu_features_case.rml` — `v_flex`/`max_h`
15. `demo/src/cases/slot_case.rml` — `v_flex`
16. `demo/src/cases/status_bar_case.rml` — `v_flex`
17. `demo/src/cases/table_case.rml` — `v_flex`/`h_flex`
18. `demo/src/cases/tab_bar_case.rml` — `v_flex`/`selected_index` + size
19. `demo/src/cases/two_way_case.rml` — `v_flex`
20. `demo/src/cases/welcome_case.rml` — `v_flex`

**4.3 同步更新 .rml.rs 中的代码示例字符串**

部分 .rml.rs 文件的 `#[computed] code_sample` 包含 .rml 源码字符串，需同步更新：
- `demo/src/cases/description_list_case.rml.rs` — `label_width` → `label-width`
- `demo/src/cases/avatar_case.rml.rs` — `large=""` → `size="large"`
- `demo/src/cases/menu_editor_case.rml.rs` — `check_side` → `check-side`
- `demo/src/cases/menu_features_case.rml.rs` — `max_h` → `max-h`

### 阶段 5：DescriptionList Demo 增强

**文件**：`demo/src/cases/description_list_case.rml` 和 `.rml.rs`

新增演示场景：
- **items 绑定演示**：`<descriptions items={desitems} columns="2" bordered="">` —— 从 ViewModel 字段绑定 `Vec<DescriptionItem>`
- **vertical 绑定演示**：`<descriptions vertical={is_vertical} ...>` —— 通过 ViewModel 布尔字段控制方向

需要在 `description_list_case.rml.rs` 中：
- 添加 `desitems: Vec<DescriptionItem>` 字段（构造示例数据）
- 添加 `is_vertical: bool` 字段
- 更新 `code_sample` 字符串以反映新属性名（kebab-case）

### 阶段 6：补充注册表缺失项

**文件**：`crates/engine/src/compiler/props_registry.rs`

- 在 `SHELL_PROPS` 的 `tab_window` 条目中添加 `"tab_item_template"`（预存缺失）

### 阶段 7：文档更新

**文件**：`docs/06-components/reference/description-list.md`
- 更新所有属性名为 kebab-case（`label-width`/`selected-index` 等）
- 移除 `horizontal` 属性文档
- 添加 `vertical` 绑定用法
- 添加 `items` 绑定用法
- 更新 `size` 属性文档（替代 `small`/`large`）

**文件**：`docs/06-components/reference/props-mapping.md`
- 更新 DescriptionList/DescriptionItem 属性表（kebab-case + 新属性）

**文件**：`docs/02-syntax/tags-mapping.md`
- 第 2.2.9 节：更新命名规范说明，明确"禁止下划线，统一 kebab-case"

**文件**：`docs/02-syntax/attributes.md`（若存在属性系统章节）
- 添加 kebab-case 命名规范说明
- 添加 `size` 通用属性文档

---

## 假设与决策

1. **`Size` 导出路径**：假设 `Size` 枚举可通过 `gpui_component::Size` 访问。若实际路径不同（如 `gpui_component::sizable::Size`），需在实施时调整 re-export 语句。
2. **`gap_4` 处理**：`gap_4` 在引擎中无 setter 实现，可能被静默丢弃。迁移为 `gap-4` 仅为命名一致性；若需功能实现应另行处理（不在本计划范围）。
3. **`onclick` vs `on-click`**：用户确认 `onclick`（单词）保持不变。解析器规范化不影响 `onclick`（无 `-` 可转）。TabBar/Tab 的 `on_click` 迁移为 `on-click`，规范化回 `on_click` 命中现有 setter。
4. **向后兼容**：用户明确要求"严格禁止下划线"，不提供兼容期。解析器拒绝 `_` 后，旧 .rml 文件将编译失败，强制迁移。
5. **`vertical="false"` 行为**：返回 `None`（不生成 `.layout()` 调用），依赖 DescriptionList 默认横向布局。需确认 DescriptionList 默认布局确实为横向。

---

## 验证步骤

1. **解析器单元测试**：
   - 验证 `label-width` 解析后属性名为 `label_width`
   - 验证 `onclick` 解析后属性名仍为 `onclick`
   - 验证含 `_` 的属性名触发解析错误

2. **设置器单元测试**：
   - `size` 静态 setter：`size="small"` → `.with_size(rml_ui::Size::Small)`
   - `size` 绑定 setter：`size={val}` → `.with_size(self.val)`
   - `vertical` 绑定 setter：`vertical={flag}` → `.layout(if self.flag { ... Vertical } else { ... Horizontal })`
   - `items` 绑定 setter：`items={data}` → `.children(self.data.clone())`
   - 确认 `small`/`xsmall`/`large`/`horizontal` setter 已移除

3. **注册表测试**：
   - `is_prop_registered("Button", "size")` 返回 true
   - `is_prop_registered("Button", "small")` 返回 false
   - `is_prop_registered("DescriptionList", "items")` 返回 true
   - `is_prop_registered("DescriptionList", "horizontal")` 返回 false
   - `is_shell_prop_registered("tab_window", "tab_item_template")` 返回 true

4. **端到端编译**：
   - `cargo build --workspace` 0 错误
   - `cargo test --workspace` 全部通过（含现有 530+ 测试 + 新增测试）

5. **Demo 运行**（可选，受环境限制）：
   - `cargo run -p rust-rml-demo` 启动成功
   - DescriptionList case 展示 items 绑定和 vertical 绑定效果
   - 各组件 size 属性生效
