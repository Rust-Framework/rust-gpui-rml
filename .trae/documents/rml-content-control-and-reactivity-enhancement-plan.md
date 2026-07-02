# RML ContentControl 机制引入与响应式能力增强计划

## 摘要

本计划承接前序 4 阶段重构（已完成 Phase 1-3 + Phase 4.1），聚焦三大目标：
1. **Phase 4 收尾**：删除 CaseHost 中间层，MainWindow 直接承担 case 渲染宿主职责
2. **Phase 5（新增）**：扩展 RML 模板语法，引入 `ContentControl` 机制（类似 WPF `ContentControl Content={...}`），支持 `content={expression}` 绑定动态 `AnyElement`
3. **Phase 6（新增）**：响应式能力增强分析报告 —— 评估 RML 框架对比 WPF `INotifyPropertyChanged`/`INotifyCollectionChanged` 的差距与增强路径

**用户澄清项**（已在 Phase 3 实现）：`#[contributehost]` 宏编译期断言目标对象必须实现 `IContributionHost` 接口 —— 已在 [contributehost.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/contributehost.rs#L106-L111) L106-111 实现，本计划不再重复。

---

## 当前状态分析

### 已完成（Phase 1-3 + Phase 4.1）

| 模块 | 状态 | 关键文件 |
|------|------|----------|
| 核心契约 | ✅ | `crates/core/src/contribution.rs` — `IVisualContribution`、`RenderContext`、`IContributionHost`、`ComponentEntityCache`(`#[doc(hidden)]`) |
| App 层 | ✅ | `crates/app/src/contribution/render.rs` — `render_component_view`、`render_contribution_visual` |
| `#[contributehost]` 宏 | ✅ | `crates/macros/src/contributehost.rs` — 编译期断言 + 注册函数 |
| `#[contribute]` 宏 | ✅ | `crates/macros/src/contribute.rs` — `host_id` 字符串字面量 + 自动 `IVisualContribution` |
| Demo 批量替换 | ✅ | 12 个文件 `host=MainWindow` → `host_id="demo.shell"` |
| Demo 导入清理 | ✅ | 9 个 case 文件移除 `use crate::shell::MainWindow` |
| `case_host.rml` 删除 | ✅ | 已删除 |

### 待完成（Phase 4.2-4.4 + Phase 5 + Phase 6）

| 待办 | 当前阻塞 |
|------|----------|
| MainWindow 重构 | `case_host.rml.rs` 仍含旧 10 字段代码；`main_window.rml.rs` 仍含 `bindings`/`case_host`/`refresh_bindings` |
| `case_host.rml.rs` 处置 | RML 框架限制：`#[component]` 强制要求 `.rml` 文件；`<CaseHost>` 标签需 `#[component]` 注册 |
| `main_window.rml` body | `<CaseHost>` 需替换，但 RML 模板无法渲染动态 `AnyElement` |
| ContentControl 机制 | RML 模板 `apply_bind_attr` 对未知 bind 名硬编码 `format!("{}", expr)`，无 `AnyElement` 旁路 |
| 响应式能力分析 | 需输出对比 WPF 的差距报告 |

### RML 框架响应式能力现状（基于探索）

| 能力 | RML 现状 | WPF 对比 |
|------|----------|----------|
| 属性变更通知 | ✅ `AtomicU64` 版本 + `#[command]` 注入 `__rml_bump_version` + `cx.notify()` | `INotifyPropertyChanged` |
| 计算属性缓存 | ✅ `#[computed]` + 依赖字段版本和（`ComputedCache`） | `DependencyProperty` + Coerce |
| `#[computed]` 参数 | ❌ 仅 `&self`，无 `Window`/`App` 访问 | N/A |
| 集合变更通知 | ❌ 仅 `#[command]` AST 模式匹配（`push`/`pop`/`clear`），无细粒度通知 | `ObservableCollection<T>` + `INotifyCollectionChanged` |
| 双向绑定 | ✅ `<input model={field}>` | `Binding Mode=TwoWay` |
| ContentControl | ❌ 无 | ✅ `Content={Binding}` |
| `content={expr}` 绑定 | ❌ `apply_bind_attr` 对未知 bind 名 `format!("{}", expr)` 字符串化 | ✅ 直接嵌入 `AnyElement` |

---

## 决策记录

### 决策 1：CaseHost 处置 —— 完全删除

**方案**：删除 `case_host.rml.rs`，移除 `mod.rs` 中的 `case_host` 模块声明，`main_window.rml` 中 `<CaseHost>` 替换为 `ContentControl` 机制（Phase 5 实现）。

**理由**：
- 用户明确要求删除 `case_host.rml`/`case_host.rml.rs`
- `case_host.rml.rs` 中 10 个预创建 entity 字段是反模式（硬编码案例列表），违背"数据驱动"设计
- `IVisualContribution::render()` 应直接用于 tab body 渲染，无需中间路由组件

### 决策 2：ContentControl 机制 —— 扩展 RML 语法

**方案**：扩展 `apply_bind_attr` 和 `component_bind_setter`，增加 `content` bind 属性分支，直接 emit 表达式（非 `format!` 字符串化）。表达式可引用 `_window`/`cx`。

**语法**：
```rml
<div content={self.active_case_view(_window, cx)} />
```
**生成代码**：
```rust
gpui::div().child(self.active_case_view(_window, cx))
```

**理由**：
- 用户确认"直接嵌入表达式（可访问 _window/cx）"
- RML 模板是编译时生成，表达式来自 `.rml` 文件（非用户输入），安全
- 最小改动：仅 `apply_bind_attr` + `component_bind_setter` 两处增加 `content` 分支

### 决策 3：响应式能力增强 —— 分析报告（不实现）

**方案**：Phase 6 输出分析报告，评估 `#[computed]` 扩展、`ObservableCollection`、`INotifyPropertyChanged` 增强路径，但不在本次重构中实现。

**理由**：
- 响应式增强是独立的、大范围的重构项目
- 本次重构聚焦 ContentControl 机制引入 + CaseHost 删除
- 分析报告为后续规划提供依据

---

## Phase 4：MainWindow 重构（删除 CaseHost 依赖）

### 4.1 删除 case_host 模块

**文件**：[demo/src/shell/case_host.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_host.rml.rs)

**操作**：删除整个文件

**文件**：[demo/src/shell/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/mod.rs)

**修改**：移除 `case_host` 模块声明（L6-7）

```rust
// 修改前
#[path = "case_host.rml.rs"]
pub mod case_host;

// 修改后（删除这两行）
```

### 4.2 MainWindow 重构

**文件**：[demo/src/shell/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

**修改清单**：

1. **移除 `bindings` 参数**（L24）：
   ```rust
   // 修改前
   #[contributehost(id = "demo.shell", bindings = "refresh_bindings")]
   // 修改后
   #[contributehost(id = "demo.shell")]
   ```

2. **移除 CaseHost 相关导入**（L15, L17）：
   ```rust
   // 删除
   use crate::shell::case_host::CaseHost;
   use rml_core::contribution::ComponentEntityCache;
   ```

3. **移除 `case_host` 字段**（L36）：
   ```rust
   // 删除
   case_host: Option<gpui::Entity<CaseHost>>,
   ```

4. **移除 `on_loaded` 中 CaseHost 初始化**（L62-69）：
   ```rust
   // 删除
   self.case_host.get_or_insert_with(|| {
       let id = self.active_case_id.clone();
       cx.new(move |_| {
           let mut host = CaseHost::default();
           host.active_case_id = id;
           host
       })
   });
   ```

5. **移除 `refresh_bindings` 方法**（L154-168）：
   ```rust
   // 删除整个方法
   fn refresh_bindings(&mut self, cx: &mut Context<Self>) { ... }
   ```

6. **移除 `on_loaded` 中 `self.refresh_bindings(cx)` 调用**（L121）：
   ```rust
   // 删除
   self.refresh_bindings(cx);
   ```

7. **移除 `apply_switch_en` 中 `self.refresh_bindings(cx)` 调用**（L239）：
   ```rust
   // 删除
   self.refresh_bindings(cx);
   ```

8. **移除 `open_case`/`on_tab_click` 中 `case_host` 更新**（L204-206, L214-216）：
   ```rust
   // 删除
   if let Some(host) = self.case_host.as_ref() {
       host.update(cx, |h, _| h.active_case_id = case_id);
   }
   ```

9. **添加手动 `impl IContributionHost`**（满足宏的编译期断言）：
   ```rust
   impl rml_core::contribution::IContributionHost for MainWindow {
       const ID: &'static str = "demo.shell";
   }
   ```
   注：`#[contributehost]` 宏已生成 `pub const ID: &'static str = "demo.shell"`，但编译期断言要求 `IContributionHost` trait 实现。需手动 impl。

10. **添加 `subscribe_host_changes`**（替代 `refresh_bindings` 的响应式更新）：
    ```rust
    // 在 on_loaded 末尾添加
    use rml_app::contribution::subscribe_host_changes;
    subscribe_host_changes(Self::ID, cx, |this, cx| {
        this.refresh_shell_chrome(cx);
        cx.notify();
    });
    ```

11. **重命名 `refresh_bindings` → `refresh_shell_chrome`**（保留 shell chrome 刷新逻辑，但不再由 `bindings` 参数触发，改由 `subscribe_host_changes` 触发）：
    ```rust
    fn refresh_shell_chrome(&mut self, cx: &mut Context<Self>) {
        let ShellChromeBindings {
            activity_panels,
            status_items,
            menu_items,
        } = map_shell_chrome(Self::ID, cx, &self.menu_commands);
        self.activity_panels = activity_panels.clone();
        self.status_items = status_items;
        self.menu_items = menu_items;
        if let Some(bar) = &self.activity_bar {
            bar.update(cx, |bar, cx| bar.set_panels(activity_panels, cx));
        }
    }
    ```

12. **添加 `active_case_view` 方法**（Phase 5 完善调用，先写签名）：
    ```rust
    /// 渲染当前激活的 IVisualContribution 视图（供 RML 模板 content={...} 调用）
    pub fn active_case_view(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
    ) -> gpui::AnyElement {
        use rml_app::contribution::{contribution_entries, render_contribution_visual};
        let entries = contribution_entries(Self::ID, cx);
        let entry = entries.iter().find(|e| e.contribution.id() == self.active_case_id);
        if let Some(entry) = entry {
            if let Some(visual) = &entry.visual {
                render_contribution_visual(visual, window, cx)
                    .unwrap_or_else(|| gpui::div().into_any_element())
            } else {
                gpui::div().into_any_element()
            }
        } else {
            gpui::div().into_any_element()
        }
    }
    ```

### 4.3 main_window.rml 模板更新

**文件**：[demo/src/shell/main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

**修改**（L34）：暂时替换为空 div（Phase 5 替换为 `content={...}`）

```rml
// 修改前
<CaseHost active_case_id={active_case_id} />

// 修改后（Phase 4 临时）
<div />

// Phase 5 最终形态
<div content={self.active_case_view(_window, cx)} />
```

### 4.4 shell/ 文件文档（轻量）

检查 [demo/src/shell/menu_shell_contribs.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs) 和 [demo/src/shell/shell_meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_meta.rs) 是否有 `MainWindow` 类型引用（已通过 Phase 4.1 批量替换为 `host_id` 字符串）。若仅剩注释引用，更新注释即可。

---

## Phase 5：引入 ContentControl 机制

### 5.1 修改 `apply_bind_attr` —— 原生元素 `content` 属性

**文件**：[crates/engine/src/compiler/codegen/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L500-L507)

**当前代码**（L500-507）：
```rust
fn apply_bind_attr(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str]) -> String {
    match name {
        "value" => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
        "class" | "id" | "style" => String::new(),
        "disabled" | "checked" | "readonly" => format!(".when({}, |el| el)", gen_expr_code(expr, loop_vars, computed)),
        _ => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
    }
}
```

**修改后**：
```rust
fn apply_bind_attr(name: &str, expr: &str, loop_vars: &[&str], computed: &[&str]) -> String {
    match name {
        // content={expr}：直接嵌入表达式作为 child（支持 AnyElement/impl IntoElement）
        // 表达式可引用 _window/cx（render 方法作用域内可用）
        "content" => format!(".child({})", expr),
        "value" => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
        "class" | "id" | "style" => String::new(),
        "disabled" | "checked" | "readonly" => format!(".when({}, |el| el)", gen_expr_code(expr, loop_vars, computed)),
        _ => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
    }
}
```

**关键点**：
- `content` 分支直接使用 `expr` 原始字符串，不经过 `gen_expr_code`（避免表达式解析器对 `self.method(_window, cx)` 的解析失败回退到 `self.{expr}`）
- 生成的 `.child(self.active_case_view(_window, cx))` 中，`_window`/`cx` 来自 RML 模板生成的 `render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>)` 方法签名作用域

### 5.2 修改 `component_bind_setter` —— 扩展组件 `content` 属性

**文件**：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L298-L318)

**当前代码**（L298-318）：
```rust
pub fn component_bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
    let _tag = tag;
    match name {
        "value" => Some(format!(".value({}.clone())", rust_expr)),
        "disabled" => Some(format!(".disabled({})", rust_expr)),
        "selected" => Some(format!(".selected({})", rust_expr)),
        "checked" => Some(format!(".selected({})", rust_expr)),
        "label" => Some(format!(".label({}.clone())", rust_expr)),
        "items" if tag == "menu" || tag == "MenuBar" || tag == "status_bar" => {
            Some(format!(".items({}.clone())", rust_expr))
        }
        _ => None,
    }
}
```

**修改后**：增加 `content` 分支
```rust
pub fn component_bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let _tag = tag;
    match name {
        // content={expr}：直接嵌入表达式作为 child（与原生 div 的 content 分支一致）
        "content" => Some(format!(".child({})", expr_str)),
        "value" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".value({}.clone())", rust_expr))
        }
        "disabled" => {
            let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".disabled({})", rust_expr))
        }
        // ... 其他分支保持不变，均使用 component_bind_rust_expr
    }
}
```

### 5.3 ContentControl 组件决策 —— 不新增组件

**决策**：不新增 `ContentControl` 组件，直接在原生 `div`（及其他原生元素）上支持 `content` 属性。

**理由**：
- WPF `ContentControl` 是单独组件，但 RML 的 `div` 已是通用容器
- `content={expr}` 直接生成 `.child(expr)`，与 `div` 的 `ParentElement` trait 兼容
- 减少新增组件的维护成本

**验证**：`main_window.rml` 中 `<div content={self.active_case_view(_window, cx)} />` 应生成：
```rust
gpui::div().child(self.active_case_view(_window, cx))
```

### 5.4 main_window.rml 模板最终更新

**文件**：[demo/src/shell/main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

**修改**（L34）：
```rml
// Phase 4 临时
<div />

// Phase 5 最终
<div content={self.active_case_view(_window, cx)} />
```

### 5.5 MainWindow `active_case_view` 方法实现

**已在 Phase 4.2 步骤 12 实现**。方法签名：
```rust
pub fn active_case_view(
    &mut self,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::AnyElement
```

**关键点**：
- 接收 `&mut self`、`&mut Window`、`&mut App`（非 `Context<Self>`，因 `contribution_entries` 接收 `&App`）
- 从 `ContributionRegistryGlobal` 查找 `active_case_id` 对应的 `IVisualContribution`
- 调用 `render_contribution_visual` 获取 `AnyElement`
- 找不到时返回空 `div`

**注意**：`contribution_entries` 签名为 `fn contribution_entries<'a, C>(host_id: &str, cx: &'a Context<C>) -> &'a [ContributedEntry]`，需要 `Context<C>` 而非 `App`。需调整调用方式：
- 方案 A：在 `active_case_view` 中接收 `&mut gpui::Context<Self>`（而非 `&mut App`）
- 方案 B：直接访问 `ContributionRegistryGlobal` global（绕过 `contribution_entries` 便捷函数）

采用**方案 A**：`active_case_view` 接收 `&mut gpui::Context<Self>`，RML 模板 `content={self.active_case_view(_window, cx)}` 中 `cx` 即 `Context<Self>`。

```rust
pub fn active_case_view(
    &mut self,
    window: &mut gpui::Window,
    cx: &mut gpui::Context<Self>,
) -> gpui::AnyElement {
    use rml_app::contribution::{contribution_entries, render_contribution_visual};
    let entries = contribution_entries(Self::ID, cx);
    let entry = entries.iter().find(|e| e.contribution.id() == self.active_case_id);
    if let Some(entry) = entry {
        if let Some(visual) = &entry.visual {
            render_contribution_visual(visual, window, cx)
                .unwrap_or_else(|| gpui::div().into_any_element())
        } else {
            gpui::div().into_any_element()
        }
    } else {
        gpui::div().into_any_element()
    }
}
```

注：`render_contribution_visual` 签名为 `fn render_contribution_visual(visual: &VisualRenderer, window: &mut Window, cx: &mut App) -> Option<AnyElement>`，接收 `&mut App`。`Context<Self>` 可 `Deref` 为 `App`，但 `&mut Context<Self>` → `&mut App` 需通过 `cx.deref_mut()` 或直接传递（GPUI 的 `Context` 实现了 `DerefMut<Target = App>`）。需验证。

---

## Phase 6：响应式能力增强分析报告

### 6.1 `#[computed]` 扩展分析

**现状**：`#[computed]` 强制 `&self`，无 `Window`/`App` 访问。由 [crates/engine/src/compiler/codegen/observable.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs#L104-L137) `gen_computed_wrappers` 生成包装方法。

**问题**：
- 无法在 `#[computed]` 中调用 `cx.t()`（i18n）—— 当前通过 `i18n_version` 字段依赖触发重渲绕过
- 无法调用 `render_contribution_visual`（需要 `&mut Window`/`&mut App`）
- 无法访问 `ContributionRegistryGlobal` 等 global 状态

**增强路径**：
1. **方案 A：新增 `#[computed_with_cx]` 方法类型**
   - 签名：`fn method(&self, window: &mut Window, cx: &mut App) -> T`
   - 修改 `gen_computed_wrappers` 生成带 `window`/`cx` 参数的包装
   - 缓存键仍为依赖字段版本和
   - 优点：语义清晰，与 `#[computed]` 分离
   - 缺点：新增宏，增加框架复杂度

2. **方案 B：扩展 `#[computed]` 支持 `Window`/`App` 参数（可选）**
   - 签名：`fn method(&self, #[cx] window: &mut Window, #[cx] cx: &mut App) -> T`
   - 通过属性标注区分参数用途
   - 优点：复用现有宏
   - 缺点：参数标注复杂，解析困难

3. **方案 C：保持 `#[computed]` 仅 `&self`，动态内容用 `content={expr}`**
   - 本次 Phase 5 采用的方案
   - `content={expr}` 中表达式可访问 `_window`/`cx`，绕过 `#[computed]` 限制
   - 优点：最小改动，不引入新宏
   - 缺点：`content={expr}` 无缓存，每次 `render` 都重新求值

**推荐**：短期采用方案 C（本次 Phase 5），长期评估方案 A（独立项目）。

### 6.2 `ObservableCollection` 分析

**现状**：RML 无 `ObservableVec`/`ObservableMap`。`Vec<T>` 变更仅通过 `#[command]` AST 模式匹配（`push`/`pop`/`clear`/`extend`/`retain`/`truncate`）检测，注入 `__rml_bump_version` + `cx.notify()`。

**问题**：
- 间接修改（`let p = &mut self.items; p.push()`）不被检测
- 外部方法修改不被检测
- 无细粒度通知（Add/Remove/Replace/Move/Reset），整个 `#[computed]` 缓存失效

**WPF 对比**：
- `ObservableCollection<T>` 实现 `INotifyCollectionChanged`
- 细粒度通知：`NotifyCollectionChangedAction.Add`/`Remove`/`Replace`/`Move`/`Reset`
- `CollectionView` 支持过滤、排序、分组

**增强路径**：
1. **方案 A：引入 `ObservableVec<T>` 包装类型**
   - 内部 `RefCell<Vec<T>>` + `Vec<Listener>`
   - `push`/`remove`/`clear` 等方法触发 listener
   - 实现 `Deref`/`DerefMut` 兼容现有代码
   - 优点：细粒度通知
   - 缺点：需重构所有 `Vec<T>` 字段，工作量大

2. **方案 B：增强 `#[command]` AST 模式匹配**
   - 识别间接修改（`let p = &mut self.items; p.push()`）
   - 识别外部方法调用（`self.items.modify(...)`）
   - 优点：无需新类型
   - 缺点：AST 分析复杂，无法覆盖所有场景

3. **方案 C：引入 `#[observable]` 字段属性**
   - `#[observable] items: Vec<T>` → 自动包装为 `ObservableVec<T>`
   - `#[command]` 宏识别 `#[observable]` 字段，生成细粒度通知
   - 优点：渐进式采用
   - 缺点：增加宏复杂度

**推荐**：短期保持现状（`#[command]` AST 模式匹配 + 手动 `cx.notify()`），长期评估方案 A（独立项目）。

### 6.3 `INotifyPropertyChanged` 对比

**现状**：RML 通过 `AtomicU64` 版本号 + `#[command]` 注入 `__rml_bump_version` + `cx.notify()` 实现属性变更通知。

**WPF 对比**：
- `INotifyPropertyChanged`：事件驱动，细粒度通知属性名
- RML：版本号驱动，通过版本和判断 `#[computed]` 缓存失效

**评估**：
- RML 的版本号机制已覆盖 `INotifyPropertyChanged` 的核心能力
- 细粒度通知（属性名）在 RML 中通过 `__rml_changed_fields()` 方法暴露（[observable.rs:91-93](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs#L91-L93)）
- 差距：RML 无事件订阅机制（WPF `PropertyChanged` 事件），但 GPUI 的 `cx.observe`/`cx.subscribe` 已提供类似能力

**结论**：RML 的版本号机制 + GPUI 原生 `cx.observe`/`cx.subscribe` 已具备 `INotifyPropertyChanged` 的功能等价能力，无需额外增强。

### 6.4 分析报告输出

**输出形式**：本计划文件 Phase 6 章节即为分析报告。不创建独立文档（避免文档冗余）。

**后续规划建议**：
1. **短期**（本次重构）：Phase 5 `content={expr}` 机制解决动态渲染
2. **中期**（独立项目）：`#[computed_with_cx]` 方法类型（方案 A）
3. **长期**（独立项目）：`ObservableVec<T>` + 细粒度集合通知（方案 A）

---

## 验证步骤

### Phase 4 验证

1. `cargo build -p rust-rml-core -p rust-rml-app -p rust-rml-macros` —— 框架编译通过
2. `cargo build -p rust-rml-demo` —— Demo 编译通过（Phase 4 临时 `<div />` body）
3. 检查 `#[contributehost]` 宏编译期断言：移除 `impl IContributionHost for MainWindow`，编译应失败

### Phase 5 验证

1. `cargo build -p rust-rml-engine` —— 引擎编译通过
2. `cargo build -p rust-rml-demo` —— Demo 编译通过（`content={self.active_case_view(_window, cx)}` 生成正确代码）
3. 运行 demo，切换 tab，验证 case 内容正确渲染
4. 验证 `active_case_view` 在 `active_case_id` 变化时重新渲染（通过 `cx.notify()` 触发）

### Phase 6 验证

- 分析报告无需编译验证
- 确认报告内容覆盖 `#[computed]` 扩展、`ObservableCollection`、`INotifyPropertyChanged` 三大主题

---

## 假设与风险

### 假设

1. `content={expr}` 中 `expr` 直接作为 Rust 表达式嵌入，不经过 `gen_expr_code` 解析器
2. RML 模板生成的 `render` 方法中，`_window`/`cx` 在作用域内可用（已验证，见 [codegen/mod.rs:151](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L151)）
3. `Context<Self>` 可 `DerefMut` 为 `App`，`render_contribution_visual` 可接收 `&mut Context<Self>`（需验证）
4. `contribution_entries` 接收 `&Context<C>`，`&mut Context<Self>` 可通过 `&*cx` 转换为 `&Context<Self>`（需验证）

### 风险

1. **`content={expr}` 表达式安全性**：RML 模板是编译时生成，表达式来自 `.rml` 文件（非用户输入），风险可控
2. **`Context<Self>` → `App` 转换**：若 `render_contribution_visual` 无法接收 `&mut Context<Self>`，需调整 `active_case_view` 签名或使用 `cx.deref_mut()`
3. **`active_case_view` 无缓存**：每次 `render` 都重新查找 `IVisualContribution` 并调用 `render_contribution_visual`，但 `render_contribution_visual` 内部使用 `ComponentEntityCache` 缓存 Entity，性能可接受
4. **Phase 6 分析报告完整性**：报告基于当前探索，可能遗漏边界场景，后续独立项目需深入验证
