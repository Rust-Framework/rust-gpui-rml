# 10.6 代码生成原理

> **本节目标**：理解 RML 编译器如何把 `.rml` 模板转换为 GPUI 渲染代码，掌握 AST、语义验证、代码生成的完整链路。

## 10.6.1 编译流水线

```
┌──────────────────────────────────────────────────────────┐
│                  RML 编译流水线                             │
│                                                          │
│  .rml 源码                                                │
│      │                                                   │
│      ▼  词法分析 + 语法分析                                  │
│  AST（抽象语法树）                                          │
│      │                                                   │
│      ▼  语义分析                                           │
│  类型化 AST（带绑定路径、命令引用、组件引用）                    │
│      │                                                   │
│      ▼  优化（常量折叠、死代码消除）                          │
│  优化后的 AST                                              │
│      │                                                   │
│      ▼  代码生成                                           │
│  Rust 源码（实现 Render trait）                            │
│      │                                                   │
│      ▼  格式化                                            │
│  格式化的 Rust 源码 → OUT_DIR                              │
└──────────────────────────────────────────────────────────┘
```

## 10.6.2 词法与语法分析

RML 使用基于 HTML 子集的词法分析器，产出 Token 流：

```
<div class="card" r:if="visible">
  <p>{title}</p>
</div>
```

Token 流：

```
TagStart("div")
Attr("class", "card")
Attr("r:if", "visible")
TagEnd
Text("\n  ")
TagStart("p")
TagEnd
Interpolation("title")
TagClose("p")
Text("\n")
TagClose("div")
EOF
```

语法分析器把 Token 流构建为 AST：

```rust
enum Node {
    Element(Element),
    Text(SharedString),
    Interpolation(Expr),
    If { condition: Expr, body: Vec<Node> },
    Each { item: Ident, iterable: Expr, key: Option<Expr>, body: Vec<Node> },
}

struct Element {
    tag: TagName,
    attributes: Vec<Attribute>,
    children: Vec<Node>,
}
```

## 10.6.3 语义分析

语义分析把 AST 转为“类型化 AST”，解析所有引用：

### 绑定路径解析

`{user.name}` 中的 `user.name` 被解析为：

```rust
BindingPath {
    root: ViewModelField("user"),
    segments: vec![Field("name")],
    ty: Type::SharedString,
}
```

解析时校验：

- `user` 字段在 ViewModel 中存在且 `pub`
- `User` 类型有 `name` 字段
- 类型与插值上下文兼容（`SharedString` 可渲染）

### 命令引用解析

`on:click="login"` 中的 `login` 被解析为：

```rust
CommandRef {
    method: "login",
    signature: Signature { event: ClickEvent, ... },
    vm: Type::LoginViewModel,
}
```

校验：

- `login` 方法存在且有 `#[command]` 宏
- 签名与 `ClickEvent` 兼容

### 组件引用解析

`<Button label="...">` 中的 `Button` 被解析为：

```rust
ComponentRef {
    ty: Type::Button,
    props: Vec<(PropName, PropValue)>,
}
```

校验：

- `Button` 组件存在
- 传入的 props 与组件定义匹配
- 必填 props 都已提供

## 10.6.4 优化

### 常量折叠

`r:if="true"` 被消除，body 直接内联：

```rust
// 折叠前
if true { <p>{title}</p> }
// 折叠后
<p>{title}</p>
```

### 死代码消除

`r:if="false"` 的 body 被完全移除，不生成代码。

### 静态子树提取

不含绑定的子树被标记为静态，生成时只构建一次：

```rust
// 静态子树缓存
static HEADER: Lazy<Element> = Lazy::new(|| {
    div().class("header").child(Label::new("Welcome"))
});
```

## 10.6.5 代码生成

代码生成器遍历优化后的 AST，产出 Rust 代码。

### 元素生成

```html
<div class="card"><p>{title}</p></div>
```

生成：

```rust
div()
    .class("card")
    .child(
        p().child(self.title.clone()) // 绑定展开为字段访问
    )
```

### r:if 生成

```html
<p r:if="visible">{title}</p>
```

生成：

```rust
if self.visible {
    Some(p().child(self.title.clone()).into_any_element())
} else {
    None
}
```

### r:each 生成

```html
<li r:each="items" r:key="id">{title}</li>
```

生成：

```rust
self.items.iter().map(|item| {
    li().key(item.id).child(item.title.clone())
}).collect::<Vec<_>>()
```

### 绑定生成

`r:model="email"` 生成双向绑定：

```rust
input()
    .bind_value(
        cx.entity(),
        |vm| &vm.email,           // 读路径
        |vm, v| vm.email = v,     // 写路径
    )
```

### 命令生成

`on:click="login"` 生成事件监听：

```rust
button()
    .on_click(cx.listener(|this, ev: &ClickEvent, cx| {
        this.login(ev, cx)
    }))
```

## 10.6.6 Render 实现的组装

所有元素生成后，组装为 `Render` 实现：

```rust
impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let vm = self.viewmodel.read(cx);
        div()
            .class("login-form")
            .child(
                input()
                    .attr("type", "email")
                    .bind_value(cx.entity(), |vm| &vm.email, |vm, v| vm.email = v)
            )
            .child(
                input()
                    .attr("type", "password")
                    .bind_value(cx.entity(), |vm| &vm.password, |vm, v| vm.password = v)
            )
            .when(vm.error.is_some(), |d| {
                d.child(p().class("error").child(vm.error.clone().unwrap()))
            })
            .child(
                button()
                    .attr("disabled", vm.is_loading)
                    .on_click(cx.listener(|this, ev: &ClickEvent, cx| this.login(ev, cx)))
                    .child(if vm.is_loading { "登录中…" } else { "登录" })
            )
    }
}
```

## 10.6.7 绑定引擎的运行时

生成的代码在运行时与绑定引擎协作：

1. **首次渲染**：建立绑定订阅，ViewModel 字段变化时触发重渲染
2. **重渲染**：`cx.notify()` 后，绑定引擎标记脏绑定，下次 `render` 重算
3. **双向绑定**：UI 事件 → 写入 ViewModel 字段 → 触发 notify → 重渲染

```rust
// 生成的订阅注册（在 View 构造时）
cx.observe(&self.viewmodel, |this, cx| {
    cx.notify(); // ViewModel 变化时通知 View 重渲染
}).detach();
```

## 10.6.8 错误报告

编译器在语义分析阶段产出友好的错误：

```
error: 绑定路径不存在
  --> src/views/login/login.rml:5:12
   |
5 | <p>{user.emial}</p>
   |            ^^^^^
   |
   = help: ViewModel `LoginViewModel` 没有 `emial` 字段
   = note: 相似字段：`email`
```

错误包含：

- 文件名、行号、列号
- 高亮的源码片段
- 修复建议（如拼写相似的字段名）

## 10.6.9 调试编译器

### 打印 AST

```sh
RML_DUMP_AST=1 cargo build
```

输出每个 `.rml` 文件的 AST 到 `OUT_DIR/ast/`。

### 打印生成代码

```sh
cargo rml-expand views/login/login.rml
```

### 单步编译器

RML 编译器自身是 Rust 项目，可在 `rml-compiler` crate 中打断点调试。

## 10.6.10 编译器的扩展点

### 自定义指令

通过 `rml-compiler` 的插件 API 注册自定义指令：

```rust
rml_compiler::register_directive("r:auth", AuthDirective);
```

`AuthDirective` 实现 `Directive` trait，参与语义分析与代码生成。

### 自定义组件解析

默认组件解析基于 `#[component]` 宏注册的类型。可通过插件覆盖，实现动态组件加载等高级场景。

## 10.6.11 性能与缓存

编译器维护 `rml_cache.json`，记录每个文件的哈希与解析结果。未变化的文件：

- 跳过词法 / 语法分析
- 跳过语义分析
- 直接复用上次的生成代码

这使得大型项目的增量构建保持在秒级。

下一章 → [第 11 章 · 实战指南](../11-cookbook/INDEX.md)
