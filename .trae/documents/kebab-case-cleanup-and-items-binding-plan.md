# kebab-case 命名规范清理 + DescriptionList items 绑定修复

## 概述

用户提出四项要求：
1. 支持 `<descriptions items={desitems}>` 批量数据绑定
2. `size="small"` / `size={size_value}` 替代 `small=""` / `large=""` 等布尔标志（已完成）
3. `vertical` 默认横向，仅提供 `vertical="true"` / `vertical={is_vertical}`，移除 `horizontal`（已完成）
4. **全局 kebab-case**：所有 `.rml` 属性名、tag 名必须遵循 `label-width` 标准，禁止下划线

用户明确指示 items 绑定 **"统一使用 IValue"**——即采用 `Vec<Arc<dyn IValue>>` 字段类型（与 TabBar 一致），通过 `as_contribution()` 能力查询提取 `name()`/`id()` 构造 `DescriptionItem`。

## 当前状态分析

### 已完成（前序会话）

- **Parser 层**：`read_attr_name()` 拒绝 `_`，`normalize_attr_name()` 将 kebab-case 转为 snake_case 供内部查找 ✓
- **size 属性**：`props_registry.rs` 已注册 `"size"`，移除 `"small"`/`"xsmall"`/`"large"`；`component.rs` 已实现 `size` static/bind setter ✓
- **vertical/horizontal**：`setters.rs` 已仅保留 `vertical`，移除 `horizontal` ✓

### 待修复

| 问题 | 位置 | 根因 |
|------|------|------|
| items 绑定生成不可编译代码 | `setters.rs:110` | `.children(self.desitems.clone())` 要求 `Vec<DescriptionItem>: Clone`，但 `DescriptionItem` 含 `AnyElement` 不可 Clone |
| Demo 缺字段 | `description_list_case.rml.rs` | `.rml` 引用 `items={desitems}` 和 `vertical={is_vertical}`，但 struct 无对应字段 |
| code_sample 下划线 | `description_list_case.rml.rs:42,59` | `label_width` 应为 `label-width` |
| code_sample 布尔标志 | `avatar_case.rml.rs:28` | `large=""` 应为 `size="large"` |
| code_sample 下划线 | `menu_editor_case.rml.rs:46` | `check_side` 应为 `check-side` |
| code_sample 下划线 | `menu_features_case.rml.rs:55` | `max_h` 应为 `max-h` |
| 文档过时 | `description-list.md` | `label_width`、`horizontal`、`small/xsmall/large`、缺 items 绑定、缺 vertical bind |
| 文档过时 | `props-mapping.md` | `small/xsmall/large`、`horizontal`、shell 属性下划线 |

### 关键技术约束

- `DescriptionItem` 是 enum 含 `AnyElement`（一次性 GPUI 元素），**不可 Clone**
- `#[computed]` 要求 `T: Clone`（`ComputedCache::get_or_compute`），故 `#[computed]` 返回 `Vec<DescriptionItem>` 也不可行
- `Arc<dyn IValue>` 是 Clone（Arc 是 Clone），解决 Clone 约束
- `as_contribution()` 通过 `ability::query::<dyn IContribution>(&dyn IValue)` 查询，需 `(TypeId, TypeId)` 注册
- `#[contribute]` 宏生成 `__rml_register_*` 函数完成注册，但要求 `host_id`/`id` 等参数，对简单数据项过重
- `IValue` blanket impl：所有 `Send + Sync + 'static` 类型自动实现 `IValue`
- `IContribution: IValue`，`Arc<dyn IContribution>` 可 trait upcast 为 `Arc<dyn IValue>`

## 设计决策

### items 绑定方案：`Vec<Arc<dyn IValue>>` + 能力查询

用户明确指示 "统一使用 IValue"。方案如下：

- **字段类型**：`Vec<Arc<dyn IValue>>`（与 TabBar 一致）
- **Codegen**：`as_contribution()` 查询 → `name()` 为 label、`id()` 为 value → 构造 `DescriptionItem`
- **Demo 数据项**：`DescEntry` 实现 `IContribution`（自动 `IValue`），存储为 `Arc<dyn IValue>`

### 新增框架辅助函数：`register_contribution_ability::<T>()`

**问题**：`as_contribution()` 依赖 `ability::query`，需 `(TypeId::of::<T>(), TypeId::of::<dyn IContribution>)` 注册。`#[contribute]` 宏完成注册但要求 `host_id`/`id`/`kind` 等参数，对简单数据项（如 `DescEntry`）过重且语义不当。

**方案**：在 `rml_core/src/contribution.rs` 新增安全辅助函数，封装 `unsafe { ability::erase }`：

```rust
/// 为实现 IContribution 但未使用 #[contribute] 的类型注册能力 cast 函数。
///
/// 用于简单数据项（非 UI 贡献），使 `Arc<T>` 存储为 `Arc<dyn IValue>` 后
/// 可通过 `as_contribution()` 查询到 `IContribution` 能力。
#[allow(unsafe_code)]
pub fn register_contribution_ability<T: IContribution + 'static>() {
    crate::ability::register::<T, dyn IContribution>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let contrib: &dyn IContribution = s;
            unsafe { crate::ability::erase(contrib) }
        })
    });
}
```

- 7 行，符合 "10 行而非 20 行" 简洁原则
- `unsafe` 封装在框架内，调用方无需 `unsafe`
- 从 `prelude` 导出，demo 通过 `rml::prelude::*` 可用
- 幂等（`ability::register` 重复注册同一 key 等价覆盖）

## 实施步骤

### Phase A：框架层——items setter + 辅助函数

#### A1. 新增 `register_contribution_ability` 辅助函数

**文件**：`crates/core/src/contribution.rs`

在 `ContributionAbilityExt` impl 块之后（约 line 127）新增 `register_contribution_ability` 函数（见上方代码）。

**文件**：`crates/core/src/prelude.rs`

在 `contribution` 导出行追加 `register_contribution_ability`：

```rust
pub use crate::contribution::{
    ContributionAbilityExt, ContributionOptions, IContribution, IContributionHost,
    IContributionRegistry, IVisualContribution, VisualAbilityExt, register_contribution_ability,
};
```

#### A2. 修复 items bind setter

**文件**：`crates/engine/src/compiler/description_list/setters.rs`

**修改 1**（line 110）：items 分支

```rust
// 旧：
"items" => Some(format!(".children({}.clone())", rust_expr)),

// 新：
"items" => Some(format!(
    ".children({}.clone().into_iter().filter_map(|c| c.as_contribution().map(|c| rml_ui::DescriptionItem::new(c.name()).value(c.id()))).collect::<Vec<_>>())",
    rust_expr
)),
```

**修改 2**（line 86）：模块文档注释

```rust
// 旧：
/// - `items={data}` → `.children(self.data.clone())`（与 inline <description> 子元素共存）

// 新：
/// - `items={data}` → `.children(self.data.clone().into_iter().filter_map(|c| ...).collect())`
///   data: Vec<Arc<dyn IValue>>，通过 as_contribution() 获取 name()/id() 构造 DescriptionItem
```

**修改 3**（line 290-293）：更新测试断言

```rust
#[test]
fn bind_setter_items() {
    let code = bind_setter("items", "desitems", &[], &[], "DescriptionList").unwrap();
    assert_eq!(
        code,
        ".children(self.desitems.clone().into_iter().filter_map(|c| c.as_contribution().map(|c| rml_ui::DescriptionItem::new(c.name()).value(c.id()))).collect::<Vec<_>>())"
    );
}
```

### Phase B：Demo 修复——description_list_case

#### B1. 修复 `description_list_case.rml.rs`

**文件**：`demo/src/cases/description_list_case.rml.rs`

**修改 1**：新增 `DescEntry` struct + `IContribution` impl（在 `DescriptionListCase` struct 之前）

```rust
use std::sync::Arc;
use std::sync::Once;

/// DescriptionList items 绑定的演示数据项。
/// name() → label，id() → value（通过 as_contribution() 能力查询提取）。
pub struct DescEntry {
    name: SharedString,
    id: String,
}

impl IContribution for DescEntry {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.name.clone()
    }
}

static DESC_ENTRY_REGISTERED: Once = Once::new();

fn ensure_desc_entry_registered() {
    DESC_ENTRY_REGISTERED.call_once(|| {
        register_contribution_ability::<DescEntry>();
    });
}
```

**修改 2**：struct 增加字段

```rust
#[derive(Default)]
pub struct DescriptionListCase {
    pub user_name: String,
    pub user_email: String,
    pub role: String,
    pub width: gpui::Pixels,
    pub is_vertical: bool,
    pub desitems: Vec<Arc<dyn IValue>>,
}
```

**修改 3**：`on_loaded` 初始化新字段

```rust
impl ILifecycle for DescriptionListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        ensure_desc_entry_registered();
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = gpui::px(120.0);
        self.is_vertical = true;
        self.desitems = vec![
            Arc::new(DescEntry { name: "产品名称".into(), id: "RML 框架".into() }),
            Arc::new(DescEntry { name: "版本".into(), id: "1.0.0".into() }),
            Arc::new(DescEntry { name: "许可证".into(), id: "MIT".into() }),
            Arc::new(DescEntry { name: "作者".into(), id: "Rust 社区".into() }),
        ];
    }
}
```

**修改 4**：`code_sample` 全部改为 kebab-case + 小写标签

```rust
#[computed]
pub fn code_sample(&self) -> String {
    r#"<descriptions bordered="" columns="3" label-width="120">
    <description label="用户名" value="alice" />
    <description label="邮箱" value="alice@example.com" />
    <description label="状态" value="活跃" span="2" />
</descriptions>

<descriptions vertical="" bordered="">
    <description label="姓名" value="张三" />
    <description label="年龄" value="28" />
</descriptions>

<descriptions bordered="" columns="2">
    <description label="产品" value="RML 框架" />
    <separator />
    <description label="版本" value="1.0.0" />
</descriptions>

<descriptions bordered="" columns="2" label-width={width}>
    <description label="用户名" value={user_name} />
    <description label="角色" value={role} span="2" />
</descriptions>

<descriptions bordered="" columns="2">
    <description label="角色">
        <Badge primary="">{role}</Badge>
    </description>
</descriptions>

<descriptions items={desitems} bordered="" columns="2" label-width="100" />
<descriptions vertical={is_vertical} bordered="" columns="2" label-width="100">
    <description label="字段 A" value="值 A" />
    <description label="字段 B" value="值 B" />
</descriptions>"#
        .to_string()
}
```

#### B2. 修复 `description_list_case.rml` API 表描述

**文件**：`demo/src/cases/description_list_case.rml`

**修改 1**（line 7）：组件说明中 `label-width` 已正确，但需更新 items 描述

```
<p>核心特性：默认 horizontal 布局，vertical 控制纵向；bordered 边框控制；columns 多列布局；label-width 标签列宽；items 绑定批量数据（Vec&lt;Arc&lt;dyn IValue&gt;&gt;）；description 子项支持 label 必填 + value/span 属性；separator 分隔符；value 支持文本属性、文本子节点、元素子节点三种形式。</p>
```

**修改 2**（line 18）：API 表 items 类型

```
<div class="api-row"><span class="api-prop-name">items</span><span class="api-prop-type">绑定</span><span>批量数据绑定（Vec&lt;Arc&lt;dyn IValue&gt;&gt;）</span></div>
```

**修改 3**（line 85）：section 6 描述

```
<p>items={desitems} 从 ViewModel 绑定 Vec&lt;Arc&lt;dyn IValue&gt;&gt;，通过 as_contribution() 提取 name()/id() 构造 DescriptionItem：</p>
```

### Phase C：修复其他 code_sample 下划线

#### C1. avatar_case.rml.rs

**文件**：`demo/src/cases/avatar_case.rml.rs` line 28

```rust
// 旧：
r#"<Avatar src="https://..." large="" />
// 新：
r#"<Avatar src="https://..." size="large" />
```

#### C2. menu_editor_case.rml.rs

**文件**：`demo/src/cases/menu_editor_case.rml.rs` line 46

```rust
// 旧：
r#"<dropdown-menu check_side="Right">
// 新：
r#"<dropdown-menu check-side="Right">
```

#### C3. menu_features_case.rml.rs

**文件**：`demo/src/cases/menu_features_case.rml.rs` line 55

```rust
// 旧：
r#"<dropdown-menu scrollable="" max_h="280">
// 新：
r#"<dropdown-menu scrollable="" max-h="280">
```

### Phase D：文档更新

#### D1. `docs/06-components/reference/description-list.md`

**全面更新**，主要改动：

1. **容器属性表**：
   - 移除 `horizontal` 行
   - `vertical` 类型改为 `布尔标志 / {expr}`，说明 "默认横向，vertical=true 或 vertical={is_vertical} 控制纵向"
   - 移除 `small` / `xsmall` / `large`，改为 `size` 行（`size="small"` / `size={size_value}`）
   - 新增 `items` 行（`Vec<Arc<dyn IValue>>` 绑定）

2. **子项属性表**：
   - 移除 `small` / `xsmall` / `large`，改为 `size` 行

3. **布局方向章节**：
   - 移除 "水平布局（默认）" 和 "垂直布局" 的分节，合并为 "布局方向" 一节
   - 说明默认横向，`vertical=""` / `vertical="true"` / `vertical={is_vertical}` 切换纵向

4. **新增 "items 绑定" 章节**：
   - 说明 `items={desitems}` 绑定 `Vec<Arc<dyn IValue>>`
   - 通过 `as_contribution()` 提取 `name()` → label、`id()` → value
   - 数据项需实现 `IContribution` 并通过 `register_contribution_ability::<T>()` 注册
   - 与 inline `<description>` 子元素共存

5. **数据绑定表**：
   - 新增 `items` 行（`Vec<Arc<dyn IValue>>`）
   - 新增 `vertical` 行（`bool`）

6. **全文 `label_width` → `label-width`**（RML 语法示例）

7. **Codegen 示例**：`label_width` → `label-width`

8. **常见错误**：
   - 更新第 5 条 `label_width` → `label-width`
   - 新增：items 绑定字段类型需为 `Vec<Arc<dyn IValue>>`

9. **Code-behind 示例**：更新为含 `desitems` / `is_vertical` 字段 + `DescEntry` + `register_contribution_ability`

10. **RML 未覆盖 API**：移除 "动态增删条目"（items 绑定已支持）

#### D2. `docs/06-components/reference/props-mapping.md`

1. **通用属性静态属性表**（line 47）：
   - 移除 `small` / `xsmall` / `large` 行
   - 新增 `size="small"` / `size="xsmall"` / `size="medium"` / `size="large"` 行 → `.with_size(Size::*)`

2. **组件专用属性表**（line 83）：
   - DescriptionList：移除 `horizontal`，新增 `items`
   - DescriptionItem：移除 `small` / `xsmall` / `large`

3. **Shell 窗口属性表**（line 88-101）：
   - 所有属性名改为 kebab-case：`selected-tab` / `show-chrome` / `left-size` / `right-size` / `bottom-size` / `on-tab-click` / `on-chrome-toggle`
   - 添加说明：".rml 中使用 kebab-case，内部规范化为 snake_case"

### Phase E：验证

1. `cargo build --workspace` —— 0 错误
2. `cargo test --workspace` —— 全部通过
3. 重点验证：
   - `crates/engine/src/compiler/description_list/setters.rs` 的 `bind_setter_items` 测试
   - `crates/core/src/contribution.rs` 编译通过（新辅助函数）
   - `demo/src/cases/description_list_case.rml.rs` 编译通过（DescEntry + 新字段）
   - `cargo test -p rust-rml-engine` 的 `props_registry` 测试通过

## 假设与决策

1. **items 映射语义**：`IContribution::name()` → label，`IContribution::id()` → value。虽 `id()` 语义为 "唯一标识" 而非 "值"，但 IContribution 接口仅提供 `id()`/`name()` 两个字符串方法，这是最简映射。Demo 中 DescEntry 的 `id` 字段实际存储 "值" 内容（如 "RML 框架"），语义为数据项的 value。

2. **未注册项静默过滤**：`filter_map` 会跳过 `as_contribution()` 返回 `None` 的项。Demo 中通过 `register_contribution_ability::<DescEntry>()` 确保 DescEntry 已注册。

3. **注册时机**：在 `on_loaded` 中通过 `Once::call_once` 注册，早于首次 render（`items` 绑定求值）。`on_loaded` 在 entity 创建后、首次 render 前调用。

4. **code_sample 标签统一**：code_sample 中全部使用小写标签（`<descriptions>` / `<description>`），符合 "推荐小写语法" 约定。

5. **i18n 标题中的 `check_side`**：`zh-CN.json:173` / `en-US.json:172` 的标题文案含 `check_side`，这是展示文案而非 RML 代码，不在本次修改范围（可选优化）。

6. **props_registry 内部 snake_case 不改**：`props_registry.rs` 中 `label_width` / `left_size` 等是 parser 规范化后的内部键，`normalize_attr_name()` 已将 kebab-case 转为 snake_case。内部 snake_case 是 Rust 命名规范，`.rml` 用户侧为 kebab-case，二者通过 `normalize_attr_name` 桥接。
