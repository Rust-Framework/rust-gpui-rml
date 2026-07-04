# 04 数据绑定

## 四种绑定形式

RML 支持四种数据绑定形式：

### 1. children（直接子节点）

**语法**：元素子节点直接作为 children

```xml
<Button>Click me</Button>
<!-- → .label("Click me") -->

<TabBar>
    <Tab label="A" />
    <Tab label="B" />
</TabBar>
<!-- → .child(Tab::new().label("A")).child(Tab::new().label("B")) -->
```

**适用**：StatelessNoId 容器组件（TitleBar/StatusBar/AvatarGroup）、StatelessWithItems 组件（TabBar/Accordion）。

### 2. items={expr}（集合绑定）

**语法**：`items={field}` 绑定一个 `Vec<T>` 字段

```xml
<MenuBar items={menus} />
<!-- → .items(self.menus.clone()) -->

<StatusBar items={status} />
<!-- → .items(self.status.clone()) -->

<DescriptionList items={desitems} />
<!-- → .children(self.desitems.clone().into_iter().filter_map(...).collect()) -->

<Tree items={tree_nodes} />
<!-- → .items(self.tree_nodes.clone()) -->
```

**适用组件**：
- MenuBar / menu：`.items(Vec<...>)`
- StatusBar：`.items(Vec<...>)`
- DescriptionList：`.children(Vec<DescriptionItem>)`（内部转换）
- Tree：`.items(Vec<Arc<dyn IValue>>)`

**设计选择**：`items` 绑定语义统一为"批量数据注入"，具体 builder 方法（`.items()` / `.children()`）由组件实现决定。

### 3. {each}（循环指令）

**语法**：`each={item in collection}` 在子节点上声明循环

```xml
<ul>
    <li each={item in items}>{item.name}</li>
</ul>
<!-- → .children(items.iter().map(|item| div().child(item.name)).collect()) -->

<TabBar>
    <tab-item each={tab in tabs} title={tab.title} />
</TabBar>
<!-- → .children(tabs.iter().map(|tab| TabItem::new().title(tab.title)).collect()) -->
```

**适用**：原生 HTML 元素（`<li>`/`<div>` 等）、StatelessWithItems 组件的 item builder 子标签。

**限制**：`each` **不扩展**到扩展容器本身（如 `<Accordion each={...}>` 不支持），用 `items={expr}` 代替。

### 4. model={field}（双向绑定）

**语法**：`model={field}` 双向绑定到视图字段

```xml
<Input model={name} />
<!-- → .value(self.name.clone()) + on_change 更新 self.name -->
```

**适用**：Stateful 组件（Input/TextInput/CodeEditor）。

**实现**：`codegen/binding.rs::gen_model_input` 生成 `.value()` + `.on_change()` 双向绑定代码。

**不支持 v-model**：RML 不提供 `v-model` 指令，双向绑定通过 `model={field}` 显式声明。

## 绑定表达式规范化

`component_bind_rust_expr(expr, loop_vars, computed)` 将绑定表达式转换为 Rust 代码：

| 表达式 | 生成代码 | 说明 |
|--------|----------|------|
| `field` | `self.field` | 简单字段 |
| `field` (in computed) | `self.field()` | computed 方法 |
| `field.sub` | `self.field.sub` | 字段链 |
| `items[0]` | `self.items[0]` | 索引访问 |
| `items.len()` | `self.items.len()` | 方法调用 |
| `t("key")` | `t("key", cx)` | i18n 调用 |
| `item.name` (in each) | `item.name` | 循环变量（不加 self. 前缀） |

**loop_vars**：`each` 循环中的变量名列表，这些变量不加 `self.` 前缀。

**computed**：视图的 `#[computed]` 方法列表，匹配时调用 `self.method()` 而非 `self.field`。

## 绑定属性 vs 静态属性

| 维度 | 静态 `name="value"` | 绑定 `name={expr}` |
|------|---------------------|---------------------|
| 解析 | `Attribute::Static` | `Attribute::Bind` |
| setter | `static_setter` | `bind_setter` |
| 生成 | `.method("value")` | `.method(self.expr)` |
| 适用 | 字面量 | 字段/表达式 |

## 绑定规范

1. **items 语义统一**：所有支持 `items={expr}` 的组件，expr 必须是 `Vec<T>` 类型
2. **each 不扩展到扩展容器**：`<Accordion each={...}>` 不支持，用 `<Accordion items={...}>` 代替
3. **model 仅限 Stateful**：`model={field}` 仅对 Stateful 组件（Input/TextInput/CodeEditor）生效
4. **computed 自动识别**：绑定表达式匹配 computed 方法时自动调用 `self.method()`
5. **loop_vars 自动剥离 self.**：`each` 循环变量不加 `self.` 前缀
