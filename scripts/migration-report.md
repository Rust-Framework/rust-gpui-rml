# RML 样式属性归一化迁移变更报告

**执行时间**：2026-07-07
**执行脚本**：[scripts/migrate_style_attrs.py](file:///d:/GitCode/RF/rust-gpui-rml/scripts/migrate_style_attrs.py)
**扫描范围**：`demo/src/` 下全部 `.rml` 文件
**执行模式**：`--write --no-backup`

---

## 1. 执行概述

| 指标 | 数值 |
|------|------|
| 扫描目录 | `demo/src/` |
| 扫描 `.rml` 文件总数 | 64 |
| 检测到旧属性残留的文件数 | 0 |
| 应用替换的总次数 | 0 |
| 创建备份文件数 | 0（因 0 变更） |
| 实际写入文件数 | 0（因 0 变更） |

**结论**：`demo/src/` 下所有 `.rml` 文件已全部使用归一化样式属性，无任何旧版 Tailwind 式属性残留。无需修复。

---

## 2. 应用的映射规则

脚本依据 [migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md) 中的映射表，共 10 条规则。

### 2.1 静态映射（8 条，固定替换）

| 旧属性 | 新属性（等价替换） |
|--------|-----------------|
| `v-flex=""` | `display="flex" flex-direction="column"` |
| `h-flex=""` | `display="flex" flex-direction="row"` |
| `h-full=""` | `height="full"` |
| `w-full=""` | `width="full"` |
| `min-w-0=""` | `min-width="0"` |
| `min-h-0=""` | `min-height="0"` |
| `items-center=""` | `align-items="center"` |
| `flex-wrap=""` | `flex-wrap="wrap"` |

### 2.2 动态映射（2 条，按数值计算 px）

| 旧属性 | 新属性 | 计算规则 |
|--------|--------|---------|
| `gap-N=""` | `gap="N*4px"` | N×4 像素（如 `gap-2` → `gap="8px"`） |
| `p-N=""` | `padding="N*4px"` | N×4 像素（如 `p-4` → `padding="16px"`） |

### 2.3 匹配规则

- 仅匹配旧属性的标准写法 `name=""`（空字符串值，RML 布尔标志约定）
- **不匹配裸属性形式** `name`（无 `=""`），避免误伤新属性 `flex-wrap="wrap"` 的属性名部分
- 使用 `\b` 词边界 + `re.escape`，避免误伤（如 `my-flex` 不被 `h-flex` 匹配）
- 动态映射先于静态执行，避免规则干扰

---

## 3. 扫描结果汇总

### 3.1 按目录分布

| 目录 | 文件数 | 检测到残留 | 替换次数 |
|------|--------|-----------|---------|
| `demo/src/cases/` | 58 | 0 | 0 |
| `demo/src/cases/common/` | 1 | 0 | 0 |
| `demo/src/shell/` | 3 | 0 | 0 |
| `demo/src/lsp/` | 2 | 0 | 0 |
| **合计** | **64** | **0** | **0** |

### 3.2 按旧属性类型检测

| 旧属性 | 检测到次数 |
|--------|-----------|
| `v-flex=""` | 0 |
| `h-flex=""` | 0 |
| `h-full=""` | 0 |
| `w-full=""` | 0 |
| `min-w-0=""` | 0 |
| `min-h-0=""` | 0 |
| `items-center=""` | 0 |
| `flex-wrap=""` | 0 |
| `gap-N=""`（任意 N） | 0 |
| `p-N=""`（任意 N） | 0 |
| **合计** | **0** |

---

## 4. 扫描文件清单（64 个）

以下文件均已完成迁移，本次扫描无变更。

### demo/src/cases/（58 个）

| # | 文件 | 状态 |
|---|------|------|
| 1 | [accordion_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml) | ✅ 已迁移 |
| 2 | [alert_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/alert_case.rml) | ✅ 已迁移 |
| 3 | [avatar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml) | ✅ 已迁移 |
| 4 | [avatar_group_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_group_case.rml) | ✅ 已迁移 |
| 5 | [badge_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/badge_case.rml) | ✅ 已迁移 |
| 6 | [button_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml) | ✅ 已迁移 |
| 7 | [button_group_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_group_case.rml) | ✅ 已迁移 |
| 8 | [card_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/card_case.rml) | ✅ 已迁移 |
| 9 | [checkbox_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/checkbox_case.rml) | ✅ 已迁移 |
| 10 | [code_editor_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/code_editor_case.rml) | ✅ 已迁移 |
| 11 | [collapsible_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/collapsible_case.rml) | ✅ 已迁移 |
| 12 | [conditional_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/conditional_case.rml) | ✅ 已迁移 |
| 13 | [counter_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml) | ✅ 已迁移 |
| 14 | [description_list_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/description_list_case.rml) | ✅ 已迁移 |
| 15 | [else_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/else_case.rml) | ✅ 已迁移 |
| 16 | [expression_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/expression_case.rml) | ✅ 已迁移 |
| 17 | [group_box_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/group_box_case.rml) | ✅ 已迁移 |
| 18 | [html_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/html_case.rml) | ✅ 已迁移 |
| 19 | [i18n_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/i18n_case.rml) | ✅ 已迁移 |
| 20 | [icon_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/icon_case.rml) | ✅ 已迁移 |
| 21 | [input_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml) | ✅ 已迁移 |
| 22 | [kbd_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/kbd_case.rml) | ✅ 已迁移 |
| 23 | [key_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/key_case.rml) | ✅ 已迁移 |
| 24 | [label_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/label_case.rml) | ✅ 已迁移 |
| 25 | [link_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/link_case.rml) | ✅ 已迁移 |
| 26 | [list_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/list_case.rml) | ✅ 已迁移 |
| 27 | [menu_context_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_context_case.rml) | ✅ 已迁移 |
| 28 | [menu_custom_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_custom_case.rml) | ✅ 已迁移 |
| 29 | [menu_dropdown_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_dropdown_case.rml) | ✅ 已迁移 |
| 30 | [menu_editor_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_editor_case.rml) | ✅ 已迁移 |
| 31 | [menu_features_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml) | ✅ 已迁移 |
| 32 | [native_status_bar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/native_status_bar_case.rml) | ✅ 已迁移 |
| 33 | [once_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/once_case.rml) | ✅ 已迁移 |
| 34 | [overflow_test_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/overflow_test_case.rml) | ✅ 已迁移 |
| 35 | [pagination_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/pagination_case.rml) | ✅ 已迁移 |
| 36 | [popover_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/popover_case.rml) | ✅ 已迁移 |
| 37 | [progress_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/progress_case.rml) | ✅ 已迁移 |
| 38 | [progress_circle_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/progress_circle_case.rml) | ✅ 已迁移 |
| 39 | [radio_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/radio_case.rml) | ✅ 已迁移 |
| 40 | [ref_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/ref_case.rml) | ✅ 已迁移 |
| 41 | [separator_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/separator_case.rml) | ✅ 已迁移 |
| 42 | [show_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/show_case.rml) | ✅ 已迁移 |
| 43 | [slider_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slider_case.rml) | ✅ 已迁移 |
| 44 | [spinner_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/spinner_case.rml) | ✅ 已迁移 |
| 45 | [status_bar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml) | ✅ 已迁移 |
| 46 | [switch_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/switch_case.rml) | ✅ 已迁移 |
| 47 | [tab_bar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tab_bar_case.rml) | ✅ 已迁移 |
| 48 | [table_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) | ✅ 已迁移 |
| 49 | [tab_preview_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tab_preview_case.rml) | ✅ 已迁移 |
| 50 | [tag_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tag_case.rml) | ✅ 已迁移 |
| 51 | [template_slot_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/template_slot_case.rml) | ✅ 已迁移 |
| 52 | [theme_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/theme_case.rml) | ✅ 已迁移 |
| 53 | [title_bar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/title_bar_case.rml) | ✅ 已迁移 |
| 54 | [tooltip_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tooltip_case.rml) | ✅ 已迁移 |
| 55 | [tree_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/tree_case.rml) | ✅ 已迁移 |
| 56 | [two_way_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml) | ✅ 已迁移 |
| 57 | [validation_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/validation_case.rml) | ✅ 已迁移 |
| 58 | [welcome_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/welcome_case.rml) | ✅ 已迁移 |

### demo/src/cases/common/（1 个）

| # | 文件 | 状态 |
|---|------|------|
| 57 | [case_doc_page.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rml) | ✅ 已迁移 |

### demo/src/shell/（3 个）

| # | 文件 | 状态 |
|---|------|------|
| 58 | [activity_panel.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/activity_panel.rml) | ✅ 已迁移 |
| 59 | [login_dialog.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/login_dialog.rml) | ✅ 已迁移 |
| 60 | [main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) | ✅ 已迁移 |

### demo/src/lsp/（2 个）

| # | 文件 | 状态 |
|---|------|------|
| 63 | [code_editor_tab.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml) | ✅ 已迁移 |
| 64 | [lsp_explorer_panel.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_explorer_panel.rml) | ✅ 已迁移 |

---

## 5. 结论与建议

### 5.1 迁移完整性确认

`demo/src/` 下全部 64 个 `.rml` 文件已完整迁移至归一化样式属性，未检测到任何旧版 Tailwind 式属性残留。这与先前的全项目 grep 验证（`h-flex|v-flex|h-full|w-full|min-w-0|min-h-0|items-center|flex-wrap|gap-\d|p-\d` 的 `name=""` 形式无匹配）结论一致。

### 5.2 引擎 deprecation 保障

即便未来出现旧属性回潮，引擎在 [attribute.rs:27-36](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) 已实现 deprecation warning，会在编译期输出 `[rml deprecation]` 警告并丢弃旧属性。本脚本可作为 CI 防御性检查工具：

```bash
# CI 中检查：如有旧属性残留则非零退出
py scripts/migrate_style_attrs.py demo/src
```

### 5.3 后续使用建议

- **新增 `.rml` 文件时**：直接使用归一化样式属性（参考 [03-property-classification.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/03-property-classification.md) 与 [07-size-layout-conventions.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/07-size-layout-conventions.md)）
- **批量迁移第三方 `.rml` 代码**：先 `py scripts/migrate_style_attrs.py <path>` dry-run 预览，确认后加 `--write` 写入
- **回归验证**：迁移后执行 `cargo build -p rust-rml-demo` 与 `cargo test -p rust-rml-engine` 确认无回归

---

## 6. 脚本用法参考

```bash
# 扫描单个文件（dry-run）
py scripts/migrate_style_attrs.py path/to/file.rml

# 扫描目录（dry-run，递归）
py scripts/migrate_style_attrs.py path/to/dir

# 实际写入（默认创建 .bak 备份）
py scripts/migrate_style_attrs.py path/to/dir --write

# 实际写入不备份
py scripts/migrate_style_attrs.py path/to/dir --write --no-backup
```

详细映射规则与设计说明见 [migration-style-normalization.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/migration-style-normalization.md)。
