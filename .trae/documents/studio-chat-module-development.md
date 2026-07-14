# studio/chat 模块开发计划(修订版 v2)

## Summary

为 Arc Studio IDE 新增聊天模块(`studio/chat`),包含三大能力:

1. **活动栏贡献** —— 微信风格聊天列表面板(像素级还原、大厂质感)
2. **studio/core 聊天契约** —— `IChatProvider`/`IChatter`/`IChatManager` 贡献点与 DI 接口
3. **ChatWorkbench 聊天工作台** —— 按 `chat://` URI 路由,经 `IWorkbenchComponent` 呈现聊天交互

本修订版 v2 基于实际进度:Part 1(core 契约)+ Part 2a(crate 基础文件)+ Part 2b(ChatPanel + ChatListItem)已完成,剩余 Part 2c(ChatWorkbench + ChatComponent)+ Part 3(装配接线)+ 编译验证。

## Current State Analysis

### 已完成(经 Phase 1 探索验证)

| 文件                                                                                                                   | 状态        | 说明                                                                                            |
| -------------------------------------------------------------------------------------------------------------------- | --------- | --------------------------------------------------------------------------------------------- |
| [studio/core/src/chat.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/core/src/chat.rs)                               | ✅ 已创建     | IChatter/IChatProvider/IChatManager + ChatProviderAbilityExt                                  |
| [studio/core/src/registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/core/src/registry.rs)                       | ✅ 已修改     | register\_chat\_provider/get\_chat\_providers(L86-117)                                        |
| [studio/core/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/core/src/lib.rs)                                 | ✅ 已修改     | `pub mod chat;` + re-export                                                                   |
| [studio/chat/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/Cargo.toml)                                 | ✅ 已创建     | crate 清单                                                                                      |
| [studio/chat/build.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/build.rs)                                     | ✅ 已创建     | RML 编译脚本                                                                                      |
| [studio/chat/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/lib.rs)                                 | ⚠️ 已创建但需补 | crate 根 + `#[ctor::ctor]` 自注册,**缺** **`chat_list_item`** **模块声明**                             |
| [studio/chat/src/chat\_manager.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_manager.rs)              | ✅ 已创建     | ChatManager 实现                                                                                |
| [studio/chat/src/chat\_provider.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_provider.rs)            | ✅ 已创建     | DefaultChatProvider/ChatWorkbenchProvider(**已引用** **`chat_workbench::ChatWorkbench`,模块尚不存在**) |
| [studio/chat/src/chat\_list\_item.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_list_item.rml.rs) | ✅ 已创建     | ChatListItem 子组件 + ChatterItem 数据结构                                                           |
| [studio/chat/src/chat\_list\_item.rml](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_list_item.rml)       | ✅ 已创建     | 微信风格列表项模板(64px 高 + 40x40 圆角方形头像)                                                              |
| [studio/chat/src/chat\_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_panel.rml.rs)          | ✅ 已创建     | ChatPanel ViewModel(IContribution + IVisual + ILifecycle)                                     |
| [studio/chat/src/chat\_panel.rml](file:///e:/GitCode/RF/rust-gpui-rml/studio/chat/src/chat_panel.rml)                | ✅ 已创建     | ChatPanel 模板(ChatListItem 列表容器)                                                               |

### 待完成(剩余工作)

* `studio/chat/src/chat_workbench.rml.rs` + `chat_workbench.rml` —— 聊天工作台(IWorkbench + IWorkbenchComponentHost 纯壳)

* `studio/chat/src/chat_component.rml.rs` + `chat_component.rml` —— 聊天交互组件(IWorkbenchComponent)

* `studio/chat/src/lib.rs` —— 新增 `chat_list_item` 模块声明

* 装配接线 —— workspace Cargo.toml + app Cargo.toml + main.rs

### 关键技术发现(Phase 1 探索)

1. **RML** **`each`** **+** **`on-click`** **不传递 item 上下文**:`each={item in items}` 内的 `<div on-click={handler} />` 调用处理器时仅传 `&ClickEvent`(或无参数),无法获知点击了哪个 item。
2. **Tree 组件不适合微信风格**:Tree 内置展开/折叠结构与缩进,且 `on-activate` 传 `item_id` 是组件特化能力,非通用 RML 特性。
3. **EventButton 子组件模式**:`Option<ClickHandler>` 字段经 `on-click={handler}` 注入,但 ClickHandler 也不携带 item 上下文。
4. **CodeComponent 跨组件通信模式**:经 `get_or_create_entity::<EditorWorkbench>(cx)` 获取宿主 Entity,再调用宿主方法 —— 这是项目成熟的跨组件通信范式。
5. **`#[command]`** **处理器参数**:参数类型由组件决定(Tree 传 `&SharedString`,Tab 传 `usize`,Button 传 `&ClickEvent`)。

### 关键模式参考

| 模式                                      | 参考文件                                                                                                                     | 要点                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------- |
| ActivityBar 面板                          | [explorer/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/explorer/src/lib.rs)                                    | `#[ctor::ctor]` + `register_activity_panel(factory)`           |
| IWorkbench + IWorkbenchComponentHost 纯壳 | [editor/src/editor\_workbench.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/editor_workbench.rml.rs)     | `#[component]` + 手动 impl 四 trait + `get_or_create_entity`      |
| IWorkbenchComponent                     | [editor/src/code\_component.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/code_component.rml.rs)         | `#[component]` + `register_workbench_component` + `matches()`  |
| 跨组件通信                                   | [editor/src/code\_component.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/code_component.rml.rs) L96-103 | `get_or_create_entity::<Host>(cx)` + `host.read(cx)`           |
| IWorkbenchProvider                      | [editor/src/editor\_provider.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/editor_provider.rs)               | `add_keyed_singleton::<dyn IWorkbenchProvider>("schema", ...)` |
| crate 自注册                               | [editor/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/lib.rs)                                        | `#[rml_core::ctor::ctor]` + `auto_register`                    |
| RML 子组件引用                               | [editor/src/editor\_workbench.rml](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/editor_workbench.rml) L12       | `<CodeComponent if={is_code_active} />`                        |
| 微信风格列表项交互                               | **本计划新增**                                                                                                                | ChatListItem 子组件 + `get_or_create_entity::<ChatPanel>` 回调宿主    |

***

## Proposed Changes

### Part 2b: ChatPanel + ChatListItem(4 个新文件 + 1 个修改)

#### 设计决策:为何需要 ChatListItem 子组件

**问题**:RML `each={item in items}` 内的 `<div on-click={handler} />` 无法将 item 上下文传递给处理器。ClickEvent 不含元素标识,与 DOM `data-*` 属性不同。

**解决方案**:为每个聊天列表项创建独立的 `ChatListItem` RML 子组件。ChatListItem 持有自己的 `ChatterItem` 数据,其 `#[command] on_click` 处理器:

1. 读取自身 `item.uri`
2. 经 `get_or_create_entity::<ChatPanel>(cx)` 获取 ChatPanel Entity(单例缓存)
3. 调用 `panel.update(cx, |panel, ctx| panel.open_chatter(uri, ctx))`

此模式与 CodeComponent 经 `get_or_create_entity::<EditorWorkbench>(cx)` 获取宿主完全一致。

#### 2b.1 修改 `studio/chat/src/lib.rs`

**What**: 新增 `chat_list_item` 模块声明。

**Why**: ChatListItem 是独立 RML 组件,需经 `#[path]` 声明模块。无需注册能力 cast(非贡献点,仅 ChatPanel 内部使用)。

```rust
#[path = "chat_panel.rml.rs"]
pub mod chat_panel;
#[path = "chat_list_item.rml.rs"]      // ← 新增
pub mod chat_list_item;
#[path = "chat_workbench.rml.rs"]
pub mod chat_workbench;
#[path = "chat_component.rml.rs"]
pub mod chat_component;
```

**register\_chat\_services 无需修改**:ChatListItem 不注册到 DI/能力/ActivityBar,仅由 ChatPanel 模板引用。

#### 2b.2 `studio/chat/src/chat_list_item.rml.rs`

**What**: `ChatListItem` —— 聊天列表项子组件,微信风格单行布局。

**ViewModel 字段**:

* `item: ChatterItem` —— 此项绑定的聊天对象数据(含 id/name/initial/kind/uri/last\_message/time/unread)

**ChatterItem**(普通 `#[derive(Clone)]` struct,非 `#[component]`,定义在此文件):

```rust
#[derive(Clone)]
pub struct ChatterItem {
    pub id: SharedString,
    pub name: SharedString,
    pub initial: SharedString,      // 名称首字符(头像占位)
    pub kind: SharedString,
    pub uri: SharedString,
    pub last_message: SharedString, // MVP 占位:"开始对话..."
    pub time: SharedString,         // MVP 占位:""
    pub unread: u32,                // MVP 占位:0
}

impl ChatListItem {
    /// 经 get_or_create_entity 获取 ChatPanel 宿主,调用 open_chatter。
    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let uri = self.item.uri.clone();
        let panel = get_or_create_entity::<crate::chat_panel::ChatPanel>(cx);
        panel.update(cx, |panel, ctx| {
            panel.open_chatter(uri, ctx);
        });
    }

    #[computed]
    pub fn unread_text(&self) -> SharedString {
        self.item.unread.to_string().into()
    }

    #[computed]
    pub fn is_selected(&self) -> bool {
        // 经 ChatPanel 单例读取 selected_id 比对
        // (避免每项独立持有 selected_id 副本导致状态不一致)
        false // MVP: 选中态由 ChatPanel 列表刷新时写入 item,简化为 false
    }
}
```

**impl 列表**:

* `#[component]` 生成 RML 框架契约(IModel/IViewModel/IComponent/Render)

* 无 IContribution/IVisual/ILifecycle(非贡献点,仅子组件)

* `#[command] on_click` —— 点击回调,经 get\_or\_create\_entity 通信宿主

**微信风格设计要点**(主题 token 映射):

* 列表项高度:64px(微信标准)

* 头像:40x40px,`border-radius: 4px`(微信圆角方形,非圆形),背景 `--primary`

* 头像文字:18px 白色(名称首字符)

* 名称:14px `--foreground`

* 最后消息:12px `--muted-foreground`,`white-space: nowrap` + `text-overflow: ellipsis`

* 时间:11px `--muted-foreground`

* 未读角标:`--danger` 背景,18px 高,圆角胶囊,白色 11px 数字

* 选中项背景:`--list-active`(L3 elevated)

* 悬停项背景:`--list-hover`(L2 surface)

#### 2b.3 `studio/chat/src/chat_list_item.rml`

**What**: ChatListItem RML 模板 —— 微信风格单行布局。

```xml
<component>
    <div display="flex"
         align-items="center"
         height="64px"
         padding="0 12px"
         on-click={on_click}
         cursor="pointer">
        <!-- 头像:圆角方形(微信风格,非圆形) -->
        <div width="40px"
             height="40px"
             border-radius="4px"
             background="var(--primary)"
             display="flex"
             align-items="center"
             justify-content="center"
             flex-shrink="0"
             margin-right="12px">
            <span font-size="18px" color="white">{item.initial}</span>
        </div>
        <!-- 内容:名称 + 最后消息 -->
        <div display="flex"
             flex-direction="column"
             flex="1"
             min-width="0">
            <div display="flex"
                 justify-content="space-between"
                 align-items="center">
                <span font-size="14px"
                      color="var(--foreground)"
                      white-space="nowrap"
                      overflow="hidden"
                      text-overflow="ellipsis">{item.name}</span>
                <span font-size="11px"
                      color="var(--muted-foreground)"
                      flex-shrink="0"
                      margin-left="8px">{item.time}</span>
            </div>
            <span font-size="12px"
                  color="var(--muted-foreground)"
                  margin-top="2px"
                  white-space="nowrap"
                  overflow="hidden"
                  text-overflow="ellipsis">{item.last_message}</span>
        </div>
        <!-- 未读角标 -->
        <div if={item.unread > 0}
             background="var(--danger)"
             border-radius="9px"
             padding="0 6px"
             min-width="18px"
             height="18px"
             display="flex"
             align-items="center"
             justify-content="center"
             flex-shrink="0"
             margin-left="8px">
            <span font-size="11px" color="white">{unread_text}</span>
        </div>
    </div>
</component>
```

#### 2b.4 `studio/chat/src/chat_panel.rml.rs`

**What**: `ChatPanel` —— ActivityBar 面板,微信风格聊天列表容器。

**ViewModel 字段**:

* `chatter_list: Vec<ChatterItem>` —— 全部聊天对象列表

* `selected_id: SharedString` —— 当前选中项 id

**关键方法**:

* `ILifecycle::on_loaded` → 从 DI 获取 `IChatManager` → 构建 `chatter_list`

* `open_chatter(uri, cx)` —— 由 ChatListItem 经 get\_or\_create\_entity 回调,解析 URI → `IWorkbenchManager::open(uri)` + 更新 selected\_id

* `#[computed] filtered_list` —— 按 search\_text 过滤(MVP:无搜索框,直接返回 chatter\_list)

**impl 列表**:

* `IContribution`: id="chat-panel", name="Chat", icon=MessageCircle

* `IVisual::render`: 经 `get_or_create_entity` 复用 Entity,委托 Render

* `ILifecycle::on_loaded`: 初始化 chatter\_list

```rust
#[component]
#[derive(Default)]
pub struct ChatPanel {
    chatter_list: Vec<ChatterItem>,
    selected_id: SharedString,
}

impl IContribution for ChatPanel {
    fn id(&self) -> &str { "chat-panel" }
    fn name(&self) -> SharedString { "Chat".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("MessageCircle")) }
}

impl IVisual for ChatPanel {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<ChatPanel>(cx);
        entity.update(cx, |this, ctx| this.render(window, ctx).into_any_element())
    }
}

impl ILifecycle for ChatPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_chatters(cx);
    }
}

impl ChatPanel {
    fn refresh_chatters(&mut self, cx: &mut Context<Self>) {
        let chatters = cx
            .get_trait::<dyn studio_core::chat::IChatManager>()
            .map(|mgr| mgr.chatters())
            .unwrap_or_default();

        self.chatter_list = chatters.iter().map(|c| ChatterItem {
            id: c.id().into(),
            name: c.name(),
            initial: c.name().chars().next().unwrap_or('?').to_string().into(),
            kind: c.kind(),
            uri: c.uri(),
            last_message: "开始对话...".into(),
            time: "".into(),
            unread: 0,
        }).collect();
        cx.notify();
    }

    /// 由 ChatListItem 经 get_or_create_entity 回调。
    pub fn open_chatter(&mut self, uri: SharedString, cx: &mut Context<Self>) {
        // 更新选中态
        if let Some(item) = self.chatter_list.iter().find(|i| i.uri == uri) {
            self.selected_id = item.id.clone();
        }
        // 解析 URI → IWorkbenchManager::open
        if let Ok(parsed) = uri.parse::<rml_core::workbench::Uri>() {
            if let Some(mgr) = cx.get_trait::<dyn rml_core::workbench::IWorkbenchManager>() {
                mgr.open(&parsed);
            }
        }
        cx.notify();
    }

    #[computed]
    pub fn filtered_list(&self) -> Vec<ChatterItem> {
        // MVP:无搜索框,直接返回全部
        self.chatter_list.clone()
    }
}
```

**能力注册**: `register_contribution_ability::<ChatPanel>()` + `register_visual_ability::<ChatPanel>()`

#### 2b.5 `studio/chat/src/chat_panel.rml`

**What**: ChatPanel RML 模板 —— 搜索栏(MVP 隐藏)+ 聊天列表。

```xml
<component>
    <div display="flex" flex-direction="column" width="full" height="full" background="var(--secondary)">
        <!-- 聊天列表(微信风格:无边框,紧凑布局) -->
        <div flex="1" min-height="0" overflow-y-auto="">
            <ChatListItem each={item in filtered_list} item={item} />
        </div>
    </div>
</component>
```

**注**:MVP 阶段暂不实现搜索框(避免 RML `on-input` 事件绑定复杂性)。后续迭代经 Input 组件 + `on-input` 回调扩展。

***

### Part 2c: ChatWorkbench + ChatComponent(4 个新文件)

#### 2c.1 `studio/chat/src/chat_workbench.rml.rs`

**What**: `ChatWorkbench` —— IWorkbench + IWorkbenchComponentHost,纯壳。

**ViewModel 字段**(参照 EditorWorkbench):

* `uri: SharedString` —— chatter URI(`chat://{provider_id}/{chatter_id}`)

* `chatter_name: SharedString` —— 从 IChatManager 解析的 chatter 名称(Header 显示)

* `document: Option<Entity<WorkbenchDocument>>` —— 共享文档(URI 传递媒介,MVP content 为空)

* `state: Option<Entity<WorkbenchState>>` —— 共享状态

* `active_component_id: SharedString` —— 默认 "chat"

* `chat_component: Option<Entity<ChatComponent>>` —— RML 模板引用

* `preview: Arc<AtomicBool>` —— 预览模式标记(VSCode 风格 Tab,经 IWorkbench::set\_preview 切换)

**关键 impl**(参照 [editor\_workbench.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/studio/editor/src/editor_workbench.rml.rs) 模式):

* `IContribution`: id=uri, name=chatter\_name, icon=MessageCircle

* `IVisual::render`: 经 `get_or_create_entity` 复用 Entity,检测 URI 变化 → reload

* `ILifecycle::on_loaded`: 初始化 document/state/chat\_component,从 IChatManager 解析 chatter\_name

* `IWorkbench`: uri/close/activate/set/closable + preview/set\_preview(经 `Arc<AtomicBool>` 支持 VSCode 风格预览 Tab)

* `IWorkbenchComponentHost`: components() 返回 \[ChatComponent]\(MVP 硬编码,避免 CodeComponent 污染)

**`components()`** **设计决策**:

```rust
fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>> {
    // MVP: 单一 ChatComponent,直接构造
    // 不从 get_workbench_components() 获取 —— 避免 CodeComponent(matches=all) 出现
    vec![Arc::new(crate::chat_component::ChatComponent::default()) as Arc<dyn IWorkbenchComponent>]
}
```

**`set_uri`** **方法**(由 ChatWorkbenchProvider 调用):

```rust
pub fn set_uri(&mut self, uri: SharedString) {
    self.uri = uri;
}
```

**`reload()`** **逻辑**: 从 IChatManager 解析 chatter\_name → document.reload(uri, "", "chat") → 默认激活 "chat"

#### 2c.2 `studio/chat/src/chat_workbench.rml`

**What**: ChatWorkbench RML 模板 —— 纯壳容器(无 Header,因 ChatPanel 自带 44px 头部栏)。

**设计决策**:不渲染独立 Header。现成 `rml_ui::ChatPanel` 已内置 44px 头部栏(icon + title,见 [panel.rs L350-387](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/chat/panel.rs#L350)),chatter 名称经 `ChatComponent::on_loaded` 中 `panel.set_title(chatter_name, cx)` 注入。若 ChatWorkbench 再渲染 Header 会导致重复。

```xml
<component>
    <div display="flex" flex-direction="column" width="full" height="full" class="chat-pane">
        <!-- Body: ChatComponent(内部经 <Chat ref="chat" /> 渲染现成 ChatPanel) -->
        <ChatComponent if={is_chat_active} />
    </div>
</component>
```

#### 2c.3 `studio/chat/src/chat_component.rml.rs`

**What**: `ChatComponent` —— IWorkbenchComponent,复用现成 `rml_ui::ChatPanel` 经 `<Chat ref="chat" />` EntityRef 渲染。

**重大简化**(Phase 1 探索发现):`crates/ui/src/components/chat/` 已提供完整聊天组件:

* `ChatPanel`(GPUI View)—— 内置 44px 头部栏 + `MessageListView`(消息列表)+ `ChatInput`(输入区),见 [panel.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/chat/panel.rs)

* `IChatBackend` trait —— `send()` 同步 / `stream()` 流式 / `cancel()`,见 [backend.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/chat/backend.rs)

* RML 内置 `<Chat ref="chat" />` 标签(EntityRef,见 [tags.rs L846-850](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L846))

* 用法参照 [chat\_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/chat_case.rml.rs) + [chat\_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/chat_case.rml)

**无需自行实现**:消息列表、输入区、on\_send command、ChatMessage struct —— 全部由 ChatPanel 内部处理。

**ViewModel 字段**:

* `chat: Option<Entity<ChatPanel>>` —— EntityRef 字段(RML `<Chat ref="chat" />` 经字段名 "chat" 匹配)

**关键 impl**:

* `IContribution`: id="chat", name="Chat", icon=MessageCircle

* `IVisual::render`: 经 `get_or_create_entity` 复用 Entity,委托 Render

* `ILifecycle::on_loaded`: 创建 ChatPanel Entity + 注入 EchoChatBackend + 从 host 获取 chatter\_name → `set_title`

* `IWorkbenchComponent`: `matches(uri)` 返回 `uri.scheme() == "chat"`

**`matches()`** **实现**:

```rust
fn matches(&self, uri: &Uri) -> bool {
    uri.scheme() == "chat"
}
```

**`on_loaded`** **逻辑**(参照 chat\_case.rml.rs L59-96):

```rust
impl ILifecycle for ChatComponent {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // 1. 创建 ChatPanel Entity(Markdown 渲染模式)
        let chat = cx.new(|cx| ChatPanel::new(RenderMode::Markdown, window, cx));

        // 2. 从 host(ChatWorkbench)获取 chatter_name + 注入 backend
        chat.update(cx, |panel, cx| {
            // 从 host 读取 chatter_name(经 get_or_create_entity::<ChatWorkbench>)
            let host = get_or_create_entity::<crate::chat_workbench::ChatWorkbench>(cx);
            let chatter_name = host.read(cx).chatter_name.clone();
            panel.set_title(chatter_name, cx);

            // 注入 EchoChatBackend(MVP:回显后端)
            panel.set_backend(Arc::new(EchoChatBackend) as Arc<dyn IChatBackend>, cx);
        });

        self.chat = Some(chat);
    }
}
```

**EchoChatBackend**(定义在此文件,MVP 回显后端,参照 chat\_case.rml.rs L13-30):

```rust
struct EchoChatBackend;

impl IChatBackend for EchoChatBackend {
    fn send(&self, _conv: &ChatConversation, request: &ChatRequest) -> Result<ChatMessage, ChatError> {
        let reply = format!("Echo: {}", request.content);
        Ok(ChatMessage::assistant(0, reply))
    }
    fn cancel(&self) -> Result<(), ChatError> { Ok(()) }
}
```

**`register_chat_component()`**:

```rust
pub fn register_chat_component() {
    register_workbench_component_ability::<ChatComponent>();
    register_workbench_component(|| {
        Arc::new(ChatComponent::default()) as Arc<dyn IWorkbenchComponent>
    });
}
```

#### 2c.4 `studio/chat/src/chat_component.rml`

**What**: ChatComponent RML 模板 —— 极简,经 `<Chat ref="chat" />` 渲染现成 ChatPanel。

```xml
<component>
    <Chat ref="chat" />
</component>
```

**注**:`<Chat>` 是 RML 内置 EntityRef 标签(见 [tags.rs L846](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L846)),从 ViewModel 的 `Option<Entity<ChatPanel>>` 字段(字段名需与 ref 一致)clone Entity 渲染。ChatPanel 自带 44px 头部栏 + 消息列表 + 输入区,无需额外模板代码。

***

### Part 3: 装配接线(3 个文件修改)

#### 3.1 `Cargo.toml`(workspace 根)

**What**: members 添加 `"studio/chat"` + workspace.dependencies 添加 `studio-chat = { path = "studio/chat" }`。

**修改位置**:

* L2: `members = [..., "studio/chat"]`(在 `"studio/app"` 前添加)

* L38 后: `studio-chat = { path = "studio/chat" }`

```toml
members = ["crates/core", "crates/macros", "crates/engine", "crates/ui", "crates/ui-term", "crates/app", "crates/di", "crates/rml", "crates/lsp", "demo", "studio/core", "studio/shell", "studio/explorer", "studio/editor", "studio/chat", "studio/app"]

# Studio 内部 crate
studio-core = { path = "studio/core" }
studio-shell = { path = "studio/shell" }
studio-explorer = { path = "studio/explorer" }
studio-editor = { path = "studio/editor" }
studio-chat = { path = "studio/chat" }
```

#### 3.2 `studio/app/Cargo.toml`

**What**: dependencies 添加 `studio-chat = { workspace = true }`。

```toml
[dependencies]
...
studio-explorer = { workspace = true }
studio-chat = { workspace = true }   # ← 新增
gpui = { workspace = true }
```

#### 3.3 `studio/app/src/main.rs`

**What**: 添加 `extern crate studio_chat as _;` 强制链接(触发 `#[ctor::ctor]` 自注册)。

```rust
extern crate studio_editor as _;
extern crate studio_explorer as _;
extern crate studio_chat as _;   // ← 新增
```

***

## Assumptions & Decisions

### 关键设计决策

1. **ChatListItem 子组件模式**:RML `each` + `on-click` 不传递 item 上下文。采用子组件 + `get_or_create_entity::<ChatPanel>` 回调宿主模式(与 CodeComponent → EditorWorkbench 通信一致)。

2. **ChatWorkbench 组件管理**: `components()` 返回硬编码 `[ChatComponent]`,**不从** **`get_workbench_components()`** **获取**。原因:CodeComponent 的 `matches()` 默认返回 `true`,若 ChatWorkbench 使用全局注册表,CodeComponent 会污染视图切换器。

3. **ChatComponent** **`matches()`**: 仅匹配 `chat://` scheme。即使注册到全局 `register_workbench_component`,也不会出现在 EditorWorkbench 的 `file://` 视图中。

4. **ChatWorkbenchProvider schema**: `"chat"`,与 EditorProvider("file")/WelcomeProvider("rml") 同构。

5. **共享文档模型**: 复用 `WorkbenchDocument`(kind="chat"),content 为空(MVP 无聊天历史持久化)。

6. **微信风格范围**: 仅 ChatPanel + ChatListItem(活动栏面板)。ChatWorkbench/ChatComponent 复用现成 `rml_ui::ChatPanel` 通用聊天 UI。

7. **★ 复用现成 ChatPanel**(v2 修订):`crates/ui/src/components/chat/` 已提供完整聊天组件,ChatComponent 经 `<Chat ref="chat" />` EntityRef 渲染,无需自行实现消息列表/输入区/on\_send。MVP 后端用 EchoChatBackend(回显),后续可按 IChatter.kind() 注入不同 IChatBackend 实现(AI/IM/Email)。

8. **★ ChatWorkbench 无独立 Header**(v2 修订):现成 ChatPanel 自带 44px 头部栏(icon + title),chatter 名称经 `panel.set_title(chatter_name, cx)` 注入。ChatWorkbench.rml 仅作纯壳容器,避免重复 Header。

9. **未读角标/时间/最后消息**: MVP 阶段为占位值(unread=0, time="", last\_message="开始对话...")。

10. **搜索框暂不实现**: MVP 阶段 ChatPanel 不含搜索框,直接展示全部 chatter。后续迭代经 Input 组件 + `on-input` 回调扩展。

11. **ChatListItem 选中态**: MVP 阶段选中态简化处理(不渲染选中背景)。后续迭代经 ChatPanel 单例同步 selected\_id 到 ChatListItem。

### 主题 token 映射(微信风格 —— 仅 ChatPanel + ChatListItem 活动栏面板)

| UI 元素   | 主题 token             | 语义层级        |
| ------- | -------------------- | ----------- |
| 面板背景    | `--secondary`        | L0 chrome   |
| 列表项悬停   | `--list-hover`       | L2 surface  |
| 列表项选中   | `--list-active`      | L3 elevated |
| 头像背景    | `--primary`          | Primary     |
| 名称文字    | `--foreground`       | Foreground  |
| 最后消息/时间 | `--muted-foreground` | Muted       |
| 未读角标    | `--danger`           | Danger      |
| 分隔线     | `--border`           | Border      |

**注**:消息气泡/输入区域背景由现成 ChatPanel 内部管理(使用 gpui-component ActiveTheme,非 CSS 变量),不在此映射中。

***

## Verification Steps

1. **编译验证**: `cargo check --workspace` —— 全 workspace 编译通过,无错误无警告
2. **模块链接验证**: `cargo build -p arc-studio` —— app 二进制构建成功,`studio-chat` crate 被链接
3. **运行时验证**(手动): 启动应用 → 活动栏出现 Chat 面板图标 → 点击展开 → 显示聊天列表(3 个演示 chatter)→ 点击 chatter → 打开 ChatWorkbench Tab → 显示聊天界面 → 输入消息 → 点击发送 → 消息出现在列表中

## File Checklist

### 已完成文件(12)

* [x] `studio/core/src/chat.rs` —— IChatter/IChatProvider/IChatManager 契约

* [x] `studio/core/src/registry.rs` —— register\_chat\_provider/get\_chat\_providers

* [x] `studio/core/src/lib.rs` —— chat 模块导出

* [x] `studio/chat/Cargo.toml` —— crate 清单

* [x] `studio/chat/build.rs` —— RML 编译脚本

* [x] `studio/chat/src/lib.rs` —— crate 根 + `#[ctor::ctor]` 自注册(⚠️ 需补 `chat_list_item` 模块声明)

* [x] `studio/chat/src/chat_manager.rs` —— ChatManager 实现

* [x] `studio/chat/src/chat_provider.rs` —— DefaultChatProvider/ChatWorkbenchProvider

* [x] `studio/chat/src/chat_list_item.rml.rs` —— ChatListItem 子组件 ViewModel

* [x] `studio/chat/src/chat_list_item.rml` —— ChatListItem 模板(微信风格单行)

* [x] `studio/chat/src/chat_panel.rml.rs` —— ChatPanel ViewModel(IContribution + IVisual + ILifecycle)

* [x] `studio/chat/src/chat_panel.rml` —— ChatPanel 模板(聊天列表容器)

### 新建文件(4)—— Part 2c

* [ ] `studio/chat/src/chat_workbench.rml.rs` —— ChatWorkbench ViewModel(IWorkbench + IWorkbenchComponentHost)

* [ ] `studio/chat/src/chat_workbench.rml` —— ChatWorkbench 模板(Header + Body)

* [ ] `studio/chat/src/chat_component.rml.rs` —— ChatComponent ViewModel(IWorkbenchComponent)

* [ ] `studio/chat/src/chat_component.rml` —— ChatComponent 模板(消息列表 + 输入)

### 修改文件(4)—— Part 3 装配接线

* [ ] `studio/chat/src/lib.rs` —— 新增 `chat_list_item` 模块声明

* [ ] `Cargo.toml`(workspace 根)—— members + workspace.dependencies

* [ ] `studio/app/Cargo.toml` —— dependencies

* [ ] `studio/app/src/main.rs` —— `extern crate studio_chat as _;`

## 执行顺序(剩余工作)

1. **Part 2c.1** —— 创建 `chat_workbench.rml.rs`(IWorkbench + IWorkbenchComponentHost 纯壳)
2. **Part 2c.2** —— 创建 `chat_workbench.rml`(Header + Body 模板)
3. **Part 2c.3** —— 创建 `chat_component.rml.rs`(IWorkbenchComponent 聊天交互)
4. **Part 2c.4** —— 创建 `chat_component.rml`(消息列表 + 输入区模板)
5. **Part 3.1** —— 修改 `studio/chat/src/lib.rs` 新增 `chat_list_item` 模块声明
6. **Part 3.2** —— 修改 `Cargo.toml`(workspace 根)添加 `studio/chat` member + dependency
7. **Part 3.3** —— 修改 `studio/app/Cargo.toml` 添加 `studio-chat` dependency
8. **Part 3.4** —— 修改 `studio/app/src/main.rs` 添加 `extern crate studio_chat as _;`
9. **验证** —— `cargo check --workspace` 编译通过

