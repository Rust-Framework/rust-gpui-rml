# DescriptionList RML 标签设计与落地计划

## 概述

为 RML 框架的 DescriptionList 系列组件确立声明式标签形式，并补齐端到端验证用例。目标标签结构：

```rml
<descriptions bordered="" columns={3} label_width="120">
    <description label="用户名" value="alice" />
    <description label="邮箱" span={2}>{user_email}</description>
    <separator />
    <description label="角色">
        <Badge primary="">{role_name}</Badge>
    </description>
</descriptions>
```

标签映射约定：
- `<descriptions>`（小写别名）/ `<DescriptionList>`（PascalCase）→ 容器 `rml_ui::DescriptionList`
- `<description>`（小写别名）/ `<DescriptionItem>`（PascalCase）→ 子项 `rml_ui::DescriptionItem`
- `<separator />`（仅小写）→ 分隔符 `DescriptionSeparator`（调用容器 `.separator()`）

## 现状分析

经代码探索，**codegen 与 UI 层已完整落地**（上一轮会话已完成），具体包括：

### 已完成项

| 模块 | 文件 | 状态 |
|------|------|------|
| 编译器模块 | [crates/engine/src/compiler/description_list/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/description_list/mod.rs) | ✅ 15 行，声明 gen/item/setters 子模块 |
| 容器代码生成 | [crates/engine/src/compiler/description_list/gen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/description_list/gen.rs) | ✅ 380 行，14 个单元测试 |
| 子项代码生成 | [crates/engine/src/compiler/description_list/item.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/description_list/item.rs) | ✅ 379 行，11 个单元测试 |
| 属性映射 | [crates/engine/src/compiler/description_list/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/description_list/setters.rs) | ✅ 318 行，23 个单元测试 |
| 标签注册 | [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) | ✅ canonical_tag / component_lookup / is_item_builder_tag 已登记 |
| 组件路由 | [crates/engine/src/compiler/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) | ✅ StatelessWithItems 分支 + static/bind setter 委托 |
| 属性注册表 | [crates/engine/src/compiler/props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) | ✅ DescriptionList / DescriptionItem 已登记 |
| UI 再导出 | [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs) | ✅ 从 gpui_component 再导出 3 个类型 |
| prelude | [crates/ui/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/prelude.rs) | ✅ 已暴露 |

合计 **48 个单元测试**，覆盖：最小构造、PascalCase/小写双形式、静态/bind 属性、label 必填校验、value 属性优先级（attr > 文本子节点 > 元素子节点）、separator 子节点、混合子节点、ref 静默忽略、非法子节点拒绝、gen_component 顶层派发。

### 缺失项（本计划要解决的）

1. **无 demo 用例**：`demo/src/cases/` 下 16 个 case 中没有任何一个使用 `<descriptions>` 标签，端到端链路未验证
2. **测试未运行**：上一轮会话仅完成 `cargo build -p rust-rml-engine`，测试套件尚未执行
3. **i18n 未补齐**：`demo/assets/i18n/{zh-CN,en-US}.json` 没有 `case.description_list.title` 键

## 提议的变更

### 变更 1：新增 demo 用例文件（核心）

**新增文件**：`demo/src/cases/description_list_case.rml`

参照 [table_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) 的模式，包含以下演示区段：

1. **组件说明卡片**：说明 DescriptionList 用途、核心特性、支持大小写两种标签形式
2. **示例代码卡片**：展示 `.rml` 源码片段
3. **演示效果卡片**，含 5 个子区段：
   - **水平布局 + bordered + columns={3}**：基础用法，`<description label="..." value="..." />` 形式
   - **垂直布局**：`vertical=""` 切换轴向
   - **小写标签形式**：`<descriptions>` + `<description>` + `<separator />`
   - **bind 绑定**：`label_width={width}` / `value={field}` 动态数据
   - **元素子节点作为 value**：`<description label="角色"><Badge primary="">{role}</Badge></description>`

**新增文件**：`demo/src/cases/description_list_case.rml.rs`

参照 [table_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml.rs) 的模式：

```rust
#[contribute(
    host_id = "demo.activity",
    id = "components.description_list",
    kind = "case",
    group = "components",
    order = 16,  // 紧随 table_case (order=15) 之后
)]
#[component]
#[derive(Default)]
pub struct DescriptionListCase {
    pub user_name: String,
    pub user_email: String,
    pub role: String,
    pub width: f32,           // bind label_width
    pub code_sample: String,
}

impl IContribution for DescriptionListCase { ... }
impl ILifecycle for DescriptionListCase {
    fn on_loaded(...) {
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = 120.0;
        self.code_sample = r#"..."#.into();
    }
}
```

### 变更 2：注册 demo 模块

**修改文件**：[demo/src/cases/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs)

在 `pub mod table_case;` 之后追加：

```rust
#[path = "description_list_case.rml.rs"]
pub mod description_list_case;
```

### 变更 3：登记 catalog i18n key

**修改文件**：[demo/src/cases/catalog.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/catalog.rs)

在 `case_title_key` 函数的 match 中追加：

```rust
"components.description_list" => "case.description_list.title",
```

### 变更 4：补齐 i18n 翻译

**修改文件**：`demo/assets/i18n/zh-CN.json` 与 `demo/assets/i18n/en-US.json`

在对应位置追加：

```json
"case.description_list.title": "描述列表"
```
```json
"case.description_list.title": "Description List"
```

### 变更 5：运行测试套件验证（无代码修改，仅执行）

执行以下命令链，确认所有测试通过且无警告：

```powershell
cargo test -p rust-rml-engine
cargo test -p rust-rml-engine -- description_list
cargo test -p rust-rml-engine -- component_props_tags_align
cargo build -p rust-rml-ui
cargo build -p rust-rml-demo
```

## 假设与决策

### 决策 1：标签命名沿用已实现的设计
- 容器：`<descriptions>` / `<DescriptionList>` 双形式
- 子项：`<description>` / `<DescriptionItem>` 双形式
- 分隔符：**仅** `<separator />`（小写）；PascalCase `<Separator>` 已被独立 Separator 组件占用，不冲突

**理由**：与 TabBar/Tab、Table/Column、Accordion/AccordionItem 的"容器 PascalCase + 子项 PascalCase + 两者均有小写别名"约定一致；separator 仅小写避免与独立 Separator 组件命名冲突。

### 决策 2：label 作为构造器参数而非 setter
`DescriptionItem::new(label)` 要求 label 在构造时传入（无 `.label()` setter）。codegen 在 [item.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/description_list/item.rs) 中提取 label 作为构造器首个参数，缺失时报 CodegenError。

### 决策 3：value 优先级
`<description>` 的 value 按以下优先级解析（已实现，无需修改）：
1. `value="..."` 属性 或 `value={expr}` 绑定 → 直接 `.value(...)`
2. 文本子节点 → `.value("text")`
3. 单个元素子节点 → `.value(<element>)`
4. 多个元素子节点 → `.value(gpui::div().child(e1).child(e2))`

### 决策 4：ref 指令静默忽略
`DescriptionList::new()` 不接受 ElementId，与 TitleBar 一致。codegen 遇到 `ref="..."` 时静默忽略（不报错），保持与现有 StatelessNoId 组件行为一致。

### 决策 5：demo order=16
紧随 table_case (order=15)，归入 `group = "components"`，与 Table、Accordion 等容器类组件相邻展示。

### 假设
- gpui-component 上游已提供 `DescriptionList` / `DescriptionItem` / `DescriptionText` 类型（Cargo.toml 已声明 git 依赖，lib.rs 已再导出）
- `DescriptionList` 实现 `ParentElement`，`.child(DescriptionItem)` 和 `.separator()` 可用（codegen 已基于此假设生成代码）
- demo 的 `#[contribute]` 宏与 `#[component]` 宏的组合模式与 table_case 一致

## 验证步骤

### 步骤 1：运行引擎单元测试
```powershell
cargo test -p rust-rml-engine
```
**预期**：全部通过（含 48 个 description_list 测试 + component_props_tags_align 一致性测试）。

### 步骤 2：运行 description_list 专项测试
```powershell
cargo test -p rust-rml-engine -- description_list
```
**预期**：48 个测试全部通过。

### 步骤 3：验证属性注册表一致性
```powershell
cargo test -p rust-rml-engine -- component_props_tags_align
```
**预期**：通过（DescriptionList 在 component_lookup 中已注册，DescriptionItem 通过 is_item_builder_tag 跳过）。

### 步骤 4：构建 UI crate
```powershell
cargo build -p rust-rml-ui
```
**预期**：0 警告，0 错误。

### 步骤 5：构建并运行 demo
```powershell
cargo build -p rust-rml-demo
```
**预期**：0 警告，0 错误。demo 启动后在"组件"分组下可见"描述列表"案例页签，5 个演示区段均正常渲染。

### 步骤 6：人工验证 demo 渲染（可选）
启动 demo 应用，点击侧边栏"描述列表"案例，确认：
- 水平/垂直布局切换正常
- bordered 边框显示正确
- columns={3} 三列布局生效
- label_width 宽度生效
- separator 分隔线显示
- bind 绑定的动态值正确显示
- 元素子节点（Badge）作为 value 正常渲染

## 实施顺序

1. 创建 `description_list_case.rml`（模板）
2. 创建 `description_list_case.rml.rs`（代码后置）
3. 修改 `mod.rs` 注册模块
4. 修改 `catalog.rs` 登记 i18n key
5. 修改 `zh-CN.json` / `en-US.json` 补齐翻译
6. 运行步骤 1-5 验证命令，修复直到全绿
7. （可选）启动 demo 人工验证渲染效果
