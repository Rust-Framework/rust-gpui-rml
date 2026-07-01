# Tree



## 概述



`Tree` 标签路由到 `rml_ui::TreeView`，作为 **MVVM/Stateful 默认渲染器**（默认文件夹/文件图标 + `on_activate`）。**不是** gpui-component 的替代包装——声明式 `<TreeNode>` 若将来落地，应由 engine codegen 直译 `Tree::new`。



## 基本用法



```html

<!-- 在 CaseActivityPanel 组件内 -->

<Tree on_activate="on_case_activate" />

```



## 属性



| 属性 | 类型 | 绑定 | 说明 |

|------|------|------|------|

| — | — | — | Tree 无 RML 静态/绑定属性；数据通过 `TreeState` 在 Rust 侧设置 |



通用组件属性（`label`、`disabled` 等）对 Tree **无效**。



## 事件



| 事件 | 回调签名 | 说明 |

|------|----------|------|

| `on_activate` | `fn(&mut self, item_id: &SharedString, cx: &mut Context<Self>)` | 用户点击**叶子节点**（非文件夹）时触发 |



## 数据绑定



树数据不通过 RML 属性绑定，而是在 code-behind 中操作 `TreeState`。Demo 通过框架 `map_case_tree_items` 从 `MainWindow::ID` host 构建：

```rust
// case_activity_panel.rml.rs
fn refresh_tree(&mut self, cx: &mut Context<Self>) {
    let items = map_case_tree_items(MainWindow::ID, cx);
    // state.set_items(items, cx)
}
```

贡献注册：`#[contribute]` 宏 + `ctor` 自动 bootstrap；案例树条目 `kind = "case"` 由 `HostChromeMapper` 聚合，无需 demo 侧 `register_all`。



## 子节点 / 插槽



不支持子节点。树项完全由 `TreeState` 管理。



## 完整示例



`demo/src/shell/main_window.rml`：



```html

<ActivityBar panels={activity_panels} on_panel_change="on_panel_change">

    <div if={active_panel_id == "samples"} class="nav-tree">

        <CaseActivityPanel />

    </div>

</ActivityBar>

```



`CaseActivityPanel::on_case_activate` 经 demo 的 `case_activation::activate_case` 打开案例 Tab。



## 常见错误



1. **未在 `on_loaded` 初始化 TreeState** — `CaseActivityPanel` 在 `refresh_tree` 中懒创建；父窗口须 `cx.new` CaseActivityPanel Entity。

2. **字段名不是 `tree_state`** — codegen 在 `CaseActivityPanel` 内路由表硬编码该字段名。

3. **期望文件夹节点触发 `on_activate`** — 仅叶子节点（非 folder）触发回调。

4. **在 RML 中写 `items={...}`** — codegen 不支持，须在 Rust 中 `tree.set_items(...)`。



## 相关组件



- [activity-bar.md](./activity-bar.md) — 常见父容器

- [贡献点架构](../../09-architecture/contribution-system.md) — `demo.shell` host、`kind=case` 数据流



## RML 未覆盖的 API



自定义项渲染、禁用节点、`TreeState` 高级 API 需在 Rust 中直接操作 `TreeState` / 扩展 `TreeView`。

