# Compiler 目录重组 + Builtin 元数据修复

## Context

`crates/engine/src/compiler/` 根目录职责混杂：编译器入口、通用基础设施、通用 setter 工具、组件 codegen 实现（8 个散落单文件 + 12 个组件目录）交错散落，且 `component.rs` 命名误导（实为通用 setter 工具集）。同时 builtin translator 存在 7 个元数据/逻辑缺陷（`<br>` 无效、`<img>` 不支持 bind src、`<a>` 静默丢弃 href、`is_self_closing` 死字段、`<p>` 强制弱化样式、`<deferred>` container 语义歧义、`is_container` 无消费方）。

本次重组将组件 codegen 统一收纳到 `components/`，重命名 `component.rs` → `setters.rs`，随后修复 builtin 全部 7 个问题。目标：目录结构规范化、职责单一化，后续组件迭代集中在 `components/` 下，影响面最小化。

---

## Phase 1：目录重组（先执行）

### 1.1 文件移动

**重命名（留根目录）：**
- `compiler/component.rs` → `compiler/setters.rs`

**新建 `compiler/components/`，移入 8 个散落单文件：**
- `alert.rs`, `icon.rs`, `kbd.rs`, `label.rs`, `popover.rs`, `radio_group.rs`, `separator.rs`, `tag.rs`

**移入 12 个组件目录（整体搬入 `compiler/components/`）：**
- `accordion/`, `avatar/`, `badge/`, `card/`, `code_editor/`, `description_list/`, `input/`, `menu/`, `table/`, `tabs/`, `tab_bar/`, `tree/`

**保留在 compiler 根目录：** `mod.rs`, `codegen/`, `translator/`, `event.rs`, `expr.rs`, `props_registry.rs`, `printer.rs`, `source_map.rs`, `tooltip.rs`, `validator.rs`

### 1.2 路径引用修改（统一用绝对路径 `crate::compiler::...`，避免 `super::` 深度陷阱）

**A. `crate::compiler::component::` → `crate::compiler::setters::`（仅改名）：**
- `translator/user_component.rs`
- `translator/component/code_editor.rs`, `stateful.rs`, `stateless.rs`, `tree.rs`

**B. `crate::compiler::<moved>::` → `crate::compiler::components::<moved>::`（插入 components）：**
- `codegen/render.rs`（tabs::tab）
- `translator/user_component.rs`（tabs::tab）
- `translator/component/`：accordion.rs, alert.rs, code_editor.rs, description_list.rs, icon.rs, kbd.rs, label.rs, popover.rs, radio_group.rs, separator.rs, stateful.rs(input), table.rs, tabs.rs, tab_bar.rs, tag.rs, tree.rs
- `translator/menu/`：context_menu.rs, menu_bar.rs, app_menu_bar.rs, dropdown_menu.rs
- `menu/` 内部自引用：context.rs, dropdown.rs, item.rs, menu_bar.rs（改为 `crate::compiler::components::menu::`）

**C. setters.rs 内的 `super::<moved>::` → `super::components::<moved>::`：**
- 第 23,27,31,35,39 行（avatar/badge/card/table/description_list static_setter 委托）
- 第 209,213,217,221,225,230 行（bind_setter 委托）
- 第 360 行（input::is_input_event）
- **保留不动：** `super::tooltip::`（tooltip 未移动）

**D. 移动后单文件内的 `super::component::` → `crate::compiler::setters::`：**
- alert.rs, icon.rs, kbd.rs, label.rs, popover.rs, radio_group.rs, separator.rs, tag.rs（各文件内多处引用）

**E. 移动后目录文件内的 `super::super::component::` → `crate::compiler::setters::`：**
- accordion/{gen.rs, setters.rs, item.rs}
- avatar/setters.rs, badge/setters.rs, card/setters.rs
- description_list/{gen.rs, item.rs, setters.rs}
- code_editor/gen.rs
- table/{column.rs, gen.rs, setters.rs}
- tabs/{gen.rs, setters.rs, tab.rs}
- tab_bar/{gen.rs, setters.rs}
- tree/gen.rs

**F. 无需修改（源与目标同在 components/，深度不变）：**
- `tab_bar/gen.rs` 的 `super::super::tabs::tab::`（tab_bar 与 tabs 同在 components/ 下，深度不变）
- `code_editor/gen.rs` 的 `super::super::input::`（code_editor 与 input 同在 components/ 下）

### 1.3 mod.rs 声明修改

**`compiler/mod.rs`：**
- 删除 20 行散落组件声明：`pub mod accordion/alert/avatar/badge/card/code_editor/description_list/icon/input/kbd/label/menu/popover/radio_group/separator/tab_bar/tabs/tag/table/tree;`
- 删除 `pub mod component;`，新增 `pub mod setters;` 与 `pub mod components;`
- 保留：codegen, event, expr, props_registry, printer, source_map, tooltip, translator, validator

**新建 `compiler/components/mod.rs`：**
- 声明 20 个 `pub mod <name>;`（8 单文件 + 12 目录）
- 无需 re-export：调用方用全路径 `components::accordion::gen_accordion`

### 1.4 验证
```bash
cargo check -p rust-rml-engine
cargo test -p rust-rml-engine
```

---

## Phase 2：Builtin 修复（重组后执行）

### P0-1: `<br>` 无效实现
- **文件：** [builtin/br.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/br.rs)
- **问题：** `ctor: "gpui::div().hidden()"` → `display:none`，元素不渲染不占空间
- **修复：** `ctor: "gpui::div().w_full().h_0()"` —— w_full 在 flex 布局中占满宽度（视觉换行），h_0 不占高度。GPUI 无原生换行元素，此为最接近的近似
- **验证：** 编译通过，br 不再是 display:none

### P0-2: `<img>` 不支持 bind src
- **文件：** [builtin/img.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/img.rs)
- **问题：** 仅提取静态 src，`<img src={dynamic} />` 被 `apply_bind_attr` 丢弃
- **修复：** 在 `to_rust` 中检测 src 是否为 `Attribute::Bind`，若是则用 `gen_expr_code` 生成表达式作为构造参数：
  ```rust
  let src = elem.attributes.iter().find_map(|attr| match attr {
      Attribute::Static { name, value, .. } if name == "src" => Some(SrcKind::Static(value.clone())),
      Attribute::Bind { name, expr, .. } if name == "src" => Some(SrcKind::Bind(expr.clone())),
      _ => None,
  });
  let ctor = match src {
      Some(SrcKind::Static(s)) => format!("gpui::img({:?})", s),
      Some(SrcKind::Bind(e)) => {
          let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
          let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
          format!("gpui::img({})", gen_expr_code(&e, &lv, &computed))
      }
      None => "gpui::img(\"\")".to_string(),
  };
  ```
- **验证：** 新增测试用例验证 bind src 生成 `gpui::img(self.field)` 形式

### P0-3: `<a>` 静默丢弃 href
- **文件：** [builtin/a.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/a.rs)
- **问题：** href 被 `apply_static_attr` 静默丢弃，用户无感知
- **修复：** 在 a.rs 的 `to_rust` 中检测 href 属性，若有则输出 warning 提示用 `<Link>` 组件：
  ```rust
  if elem.attributes.iter().any(|a| matches!(a, Attribute::Static { name, .. } if name == "href")) {
      eprintln!("[rml warning] `<a href=\"...\">` is not functional in GPUI; use `<Link href=\"...\">` component for hyperlink behavior. href will be dropped.");
  }
  ```
- **验证：** 编译通过，warning 输出正确

### P1-4: 删除 is_self_closing 死字段
- **文件：** [builtin/meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/meta.rs) + 22 个 translator 文件
- **问题：** `BuiltinMeta.is_self_closing` 从未被读取，与 `is_void_tag()` 函数语义重叠且定义分离，赋值语义混乱
- **修复：**
  1. 从 `BuiltinMeta` struct 删除 `is_self_closing` 字段
  2. 从所有 22 个 translator 的 `META` 常量中删除 `is_self_closing: ...` 赋值行
  3. 保留 `is_void_tag()` 函数作为 void 标签判定的单一信源
- **验证：** `cargo check -p rust-rml-engine` 通过

### P1-5: `<p>` 强制弱化样式
- **文件：** [builtin/p.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/p.rs)
- **问题：** `ctor` 强制 `text_sm()` + `--text-muted`，破坏正文语义
- **修复：** `ctor: "gpui::div()"` —— 退化为普通 div，由 CSS 控制样式。`<p>` 语义是正文段落，不应强制小字号弱化颜色
- **验证：** 编译通过，`<p>` 不再强制样式

### P2-6: `<deferred>` container 语义文档化
- **文件：** [builtin/deferred.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/deferred.rs)
- **问题：** `container(true)` 语义歧义（Deferred 非 ParentElement，child 作为构造参数）
- **修复：** 在文档注释中明确说明"单子元素作为构造参数传入，非 ParentElement 链式调用"，保留 `container(true)`（确实包含子元素）
- **验证：** 文档更新即可

### P3-7: is_container 保持现状
- **文件：** [builtin/meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/translator/builtin/meta.rs)
- **问题：** `TranslatorMetadata.is_container` 无消费方
- **决策：** 保持现状。`is_container` 与 `allowed_children`/`default_child`/`requires_ref` 等同属设计时元数据（预留供 LSP/设计器），属于 `TranslatorMetadata` 的完整字段集，不应单独删除
- **验证：** 无需代码改动

---

## 验证策略

```bash
# Phase 1 验证
cargo check -p rust-rml-engine
cargo test -p rust-rml-engine

# Phase 2 验证
cargo check -p rust-rml-engine
cargo test -p rust-rml-engine
cargo test -p rust-rml-engine --lib props_registry::tests  # 组件属性注册护栏
cargo test -p rust-rml-engine --lib translator::builtin     # builtin 单元测试

# 全量验证
cargo build --workspace
cargo test --workspace
```

## 风险与回退

- **Phase 1 风险：** 路径修改遗漏导致编译失败。应对：用 `cargo check` 迭代修复，统一用绝对路径 `crate::compiler::...` 避免 `super::` 深度陷阱
- **Phase 2 风险：** 低。改动集中在 builtin/ 目录，影响面小
- **回退：** 两阶段独立，Phase 1 失败可单独回退不影响 Phase 2
