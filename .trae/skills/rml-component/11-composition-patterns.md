# 11 组合与声明式子节点

RML 中「子节点」有两种语义，不可混用：

## UI 子节点

渲染为视觉树的一部分，如 `Dialog` 正文、`Tabs` 内 `Tab` body。

## 声明式元数据子节点

不单独渲染，由宿主 codegen 消费，如：

```html
<Input ref="field">
    <KeyBinding key="Ctrl+S" on-press={on_save} />
</Input>
```

实现：`crates/engine/src/compiler/components/key_binding/attach.rs`

## 三类触发器模式

| 模式 | 组件 | 写法 |
|------|------|------|
| 元数据子节点 | KeyBinding + Input 等 | 子节点 `<KeyBinding/>` |
| slot 触发器 | Dialog, Popover, Sheet | `<Button slot="trigger"/>` |
| 首子触发器 | dropdown-menu, context-menu | 第一个非 menu-item 子元素 |

完整说明见 [docs/06-components/composition-patterns.md](../../../docs/06-components/composition-patterns.md)。

## Demo 案例

- `slot="demo"` 内用 `demo-section` 分场景，不用 Card 堆叠
- 详见 [docs/11-cookbook/demo-case-conventions.md](../../../docs/11-cookbook/demo-case-conventions.md)

## 反模式

- ❌ 三层嵌套 `<KeyBinding><KeyBinding><Input/></KeyBinding></KeyBinding>`（改用 Input 内多个 KeyBinding 子节点）
- ❌ `<Input><span>快捷键说明</span></KeyBinding>`（宿主只能有 KeyBinding 子节点）
- ❌ 文档/案例中使用 `r:if`、`r:model`、`onchange`（无连字符）
