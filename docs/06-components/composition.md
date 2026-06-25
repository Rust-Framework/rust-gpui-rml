# 6.5 组件组合

> **本节目标**：掌握组件的组合模式——父子通信、兄弟通信、依赖注入、高阶组件。

## 6.5.1 组合的核心思想

组合优于继承。通过组合小组件构建大组件，通过组合大组件构建页面：

```
页面
└── 布局组件
    ├── 导航栏组件
    │   ├── Logo 组件
    │   ├── 菜单组件
    │   └── 用户菜单组件
    ├── 侧边栏组件
    │   └── 菜单项组件
    └── 内容区组件
        ├── 搜索框组件
        ├── 列表组件
        │   └── 列表项组件
        └── 分页组件
```

## 6.5.2 父子组件通信

### 父 → 子：通过属性

```html
<!-- 父视图 -->
<div>
    <ChildComponent
        title={my_title}
        data={my_data}
        on_change={handle_change}
    />
</div>
```

```rust
// 子组件
#[derive(Model)]
#[component(template = "components/child.rml")]
pub struct ChildComponent {
    pub title: SharedString,
    pub data: Vec<Item>,
    pub on_change: Option<Arc<dyn Fn(&ChangeEvent)>>,
}
```

### 子 → 父：通过事件

```rust
// 子组件内部
impl ChildComponent {
    #[command]
    pub fn on_internal_change(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
        // 处理内部变化...

        // 触发事件通知父视图
        if let Some(callback) = &self.on_change {
            callback(ev);
        }
    }
}
```

### 完整示例

```html
<!-- 父视图 -->
<div class="user-management">
    <h1>用户管理</h1>

    <UserForm
        user={editing_user}
        on_save={handle_save}
        on_cancel={handle_cancel}
    />

    <UserList
        users={users}
        on_edit={handle_edit}
        on_delete={handle_delete}
    />
</div>
```

```rust
#[derive(Model)]
#[view]
pub struct UserManagement {
    pub users: Vec<User>,
    pub editing_user: Option<User>,
}

impl UserManagement {
    #[command]
    pub fn handle_save(&mut self, user: User, _: &SaveEvent, cx: &mut ViewContext<Self>) {
        if let Some(existing) = self.users.iter_mut().find(|u| u.id == user.id) {
            *existing = user;
        } else {
            self.users.push(user);
        }
        self.editing_user = None;
        cx.notify();
    }

    #[command]
    pub fn handle_cancel(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.editing_user = None;
        cx.notify();
    }

    #[command]
    pub fn handle_edit(&mut self, user_id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.editing_user = self.users.iter().find(|u| u.id == user_id).cloned();
        cx.notify();
    }

    #[command]
    pub fn handle_delete(&mut self, user_id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.users.retain(|u| u.id != user_id);
        cx.notify();
    }
}
```

## 6.5.3 兄弟组件通信

兄弟组件不直接通信，通过共同的父视图协调：

```
兄弟 A ──事件──▶ 父视图 ──属性──▶ 兄弟 B
```

### 示例

```html
<!-- 父视图 -->
<div class="search-panel">
    <SearchBox on_search={handle_search} />
    <SearchResults results={search_results} />
</div>
```

```rust
#[derive(Model)]
#[view]
pub struct SearchPanel {
    pub search_results: Vec<SearchResult>,
}

impl SearchPanel {
    #[command]
    pub fn handle_search(&mut self, ev: &SearchEvent, cx: &mut ViewContext<Self>) {
        let query = ev.query.clone();
        self.perform_search(&query, cx);
    }

    fn perform_search(&mut self, query: &str, cx: &mut ViewContext<Self>) {
        // 执行搜索，更新 search_results
        cx.notify();
    }
}
```

### 数据流

1. `SearchBox` 触发 `on_search` 事件
2. `SearchPanel` 的 `handle_search` 处理事件
3. `SearchPanel` 更新 `search_results` 字段
4. `SearchResults` 接收新的 `results` 属性并重新渲染

## 6.5.4 跨层级通信：依赖注入

当组件层级很深时，通过属性逐层传递会很繁琐。RML 提供依赖注入机制：

```
根视图
└── 组件 A
    └── 组件 B
        └── 组件 C  ← 需要访问根视图的数据
```

### 提供依赖

```rust
use rml::prelude::*;

#[derive(Model)]
#[view]
pub struct App {
    pub theme: Entity<Theme>,
    pub user_session: Entity<UserSession>,
    pub notification_service: Entity<NotificationService>,
}

impl App {
    pub fn new(cx: &mut AppContext) -> Self {
        Self {
            theme: cx.new_model(|_| Theme::Light),
            user_session: cx.new_model(|_| UserSession::default()),
            notification_service: cx.new_model(|_| NotificationService::new()),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 提供依赖
        cx.provide(self.theme.clone());
        cx.provide(self.user_session.clone());
        cx.provide(self.notification_service.clone());
    }
}
```

### 注入依赖

```rust
#[derive(Model)]
#[component(template = "components/user_avatar.rml")]
pub struct UserAvatar {
    pub user_id: u64,
    pub avatar_url: SharedString,

    // 注入的依赖
    user_session: Option<Entity<UserSession>>,
}

impl UserAvatar {
    pub fn new(user_id: u64) -> Self {
        Self {
            user_id,
            avatar_url: SharedString::default(),
            user_session: None,
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        // 注入依赖
        self.user_session = cx.use_provider::<Entity<UserSession>>();

        // 加载头像
        if let Some(session) = &self.user_session {
            let avatar_url = session.read(cx).get_avatar_url(self.user_id);
            self.avatar_url = avatar_url;
            cx.notify();
        }
    }
}
```

### 依赖注入的优点

| 优点     | 说明                          |
| ------ | --------------------------- |
| 解耦     | 组件不依赖具体的父视图                 |
| 可测试    | 测试时可注入 mock 依赖              |
| 灵活     | 可在不同层级注入不同实现                |
| 减少属性传递 | 无需逐层传递                      |

## 6.5.5 高阶组件

高阶组件（HOC）是接收一个组件返回新组件的函数，用于增强组件功能：

```rust
use rml::prelude::*;

/// 加载状态增强器
pub fn with_loading<T: Model>(inner: Entity<T>) -> impl Model {
    LoadingWrapper::new(inner)
}

#[derive(Model)]
#[component(template = "components/loading_wrapper.rml")]
pub struct LoadingWrapper {
    pub is_loading: bool,
    pub error: Option<SharedString>,
    inner: Option<Entity<dyn Model>>,
}

impl LoadingWrapper {
    pub fn new(inner: Entity<impl Model>) -> Self {
        Self {
            is_loading: false,
            error: None,
            inner: Some(inner.into()),
        }
    }
}
```

```html
<!-- components/loading_wrapper.rml -->
<div class="loading-wrapper">
    <div if={is_loading} class="loading">加载中...</div>
    <div if={error.is_some()} class="error">{error}</div>
    <div if={!is_loading && error.is_none()}>
        <slot></slot>
    </div>
</div>
```

### 使用高阶组件

```html
<LoadingWrapper is_loading={is_data_loading} error={data_error}>
    <DataView data={data} />
</LoadingWrapper>
```

## 6.5.6 组件的组合模式

### 模式一：容器组件 + 展示组件

```rust
// 容器组件：负责数据获取和状态管理
#[derive(Model)]
#[view]
pub struct UserListContainer {
    pub users: Vec<User>,
    pub is_loading: bool,
    pub error: Option<SharedString>,
}

impl UserListContainer {
    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.fetch_users(cx);
    }

    fn fetch_users(&mut self, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            match fetch_users_from_api().await {
                Ok(users) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.users = users;
                        this.is_loading = false;
                        cx.notify();
                    });
                }
                Err(e) => {
                    let _ = this.update(&mut cx, |this, cx| {
                        this.error = Some(e.to_string().into());
                        this.is_loading = false;
                        cx.notify();
                    });
                }
            }
        }).detach();
    }
}
```

```html
<!-- 容器组件的 .rml -->
<div>
    <div if={is_loading}>加载中...</div>
    <div if={error.is_some()}>错误: {error}</div>
    <UserListPresentation
        if={!is_loading && error.is_none()}
        users={users}
        on_edit={handle_edit}
        on_delete={handle_delete}
    />
</div>
```

```rust
// 展示组件：只负责 UI 展示
#[derive(Model)]
#[component(template = "components/user_list_presentation.rml")]
pub struct UserListPresentation {
    pub users: Vec<User>,
    pub on_edit: Option<Arc<dyn Fn(u64, &ClickEvent)>>,
    pub on_delete: Option<Arc<dyn Fn(u64, &ClickEvent)>>,
}
```

### 模式二：复合组件

```rust
// 主组件
#[derive(Model)]
#[component(template = "components/tabs.rml")]
pub struct Tabs {
    pub active_tab: SharedString,
    pub tabs: Vec<TabItem>,
}

// 子组件
#[derive(Model)]
#[component(template = "components/tab_item.rml")]
pub struct TabItem {
    pub id: SharedString,
    pub title: SharedString,
    pub is_active: bool,
}

#[derive(Model)]
#[component(template = "components/tab_panel.rml")]
pub struct TabPanel {
    pub id: SharedString,
    pub is_active: bool,
}
```

```html
<!-- 使用复合组件 -->
<Tabs active_tab={active_tab}>
    <TabItem id="profile" title="个人信息" />
    <TabItem id="settings" title="设置" />

    <TabPanel id="profile">
        <ProfileForm user={user} />
    </TabPanel>
    <TabPanel id="settings">
        <SettingsForm settings={settings} />
    </TabPanel>
</Tabs>
```

### 模式三：Render Props

通过插槽传递渲染逻辑：

```html
<!-- components/data_loader.rml -->
<div>
    <div if={is_loading}>加载中...</div>
    <div if={error.is_some()}>错误: {error}</div>
    <div if={!is_loading && error.is_none()}>
        <slot let-data={data}></slot>
    </div>
</div>
```

```html
<!-- 使用 Render Props -->
<DataLoader url="/api/users">
    <template let-data>
        <ul>
            <li each={user in data} key={user.id}>
                {user.name}
            </li>
        </ul>
    </template>
</DataLoader>
```

## 6.5.7 组件组合的最佳实践

### 1. 单向数据流

```
父 → 子（属性）
子 → 父（事件）
```

避免双向数据流导致的难以追踪问题。

### 2. 组件无状态化

```rust
// ✅ 无状态展示组件
#[component(template = "components/user_card.rml")]
pub struct UserCard {
    pub user: User,
    pub on_click: Option<Arc<dyn Fn(u64)>>,
}

// ❌ 有状态展示组件
#[component(template = "components/user_card.rml")]
pub struct UserCard {
    pub user: User,
    pub is_hovered: bool,  // 内部状态，增加复杂度
}
```

### 3. 合理拆分粒度

```rust
// ✅ 合理拆分
pub struct UserCard {
    pub user: User,
}

pub struct UserList {
    pub users: Vec<User>,
}

// ❌ 过度拆分
pub struct UserName { ... }
pub struct UserEmail { ... }
pub struct UserAvatar { ... }
// 过度拆分增加复杂度，没有实际收益
```

### 4. 显式依赖

```rust
// ✅ 显式声明依赖
#[component(template = "components/user_avatar.rml")]
pub struct UserAvatar {
    pub user_id: u64,
    pub avatar_url: SharedString,
}

// ❌ 隐式依赖（通过全局状态）
fn get_user_avatar(user_id: u64) -> SharedString {
    // 从全局状态获取，难以测试和追踪
    GLOBAL_STATE.get_avatar(user_id)
}
```

## 6.5.8 完整示例：用户管理页面

```html
<!-- views/user_management.rml -->
<div class="user-management">
    <header class="page-header">
        <h1>用户管理</h1>
        <Button variant="primary" onclick={show_add_form}>添加用户</Button>
    </header>

    <SearchBox
        placeholder="搜索用户..."
        on_search={handle_search}
        on_clear={handle_clear_search}
    />

    <div class="content-area">
        <UserList
            if={!is_form_visible}
            users={filtered_users}
            on_edit={handle_edit}
            on_delete={handle_delete}
        />

        <UserForm
            if={is_form_visible}
            user={editing_user}
            on_save={handle_save}
            on_cancel={handle_cancel}
        />
    </div>

    <Dialog
        if={is_delete_dialog_open}
        title="确认删除"
        on_close={handle_delete_dialog_close}
    >
        <template>
            <p>确定要删除用户 {deleting_user_name} 吗？</p>
        </template>
        <template slot="footer">
            <Button onclick={handle_delete_cancel}>取消</Button>
            <Button variant="danger" onclick={handle_delete_confirm}>确认删除</Button>
        </template>
    </Dialog>
</div>
```

```rust
// views/user_management.rml.rs
use rml::prelude::*;
use crate::components::{search_box::SearchBox, dialog::Dialog, button::Button};
use crate::views::user_management::{user_list::UserList, user_form::UserForm};

#[derive(Model)]
#[view]
pub struct UserManagement {
    pub users: Vec<User>,
    pub search_query: SharedString,
    pub filtered_users: Vec<User>,
    pub is_form_visible: bool,
    pub editing_user: Option<User>,
    pub is_delete_dialog_open: bool,
    pub deleting_user_id: Option<u64>,
    pub deleting_user_name: SharedString,
}

impl UserManagement {
    pub fn new() -> Self {
        Self {
            users: Vec::new(),
            search_query: SharedString::default(),
            filtered_users: Vec::new(),
            is_form_visible: false,
            editing_user: None,
            is_delete_dialog_open: false,
            deleting_user_id: None,
            deleting_user_name: SharedString::default(),
        }
    }

    #[on_loaded]
    pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
        self.fetch_users(cx);
    }

    fn fetch_users(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            let users = fetch_users_from_api().await.unwrap_or_default();
            let _ = this.update(&mut cx, |this, cx| {
                this.users = users.clone();
                this.filtered_users = users;
                cx.notify();
            });
        }).detach();
    }

    #[command]
    pub fn show_add_form(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.editing_user = None;
        self.is_form_visible = true;
        cx.notify();
    }

    #[command]
    pub fn handle_edit(&mut self, user_id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.editing_user = self.users.iter().find(|u| u.id == user_id).cloned();
        self.is_form_visible = true;
        cx.notify();
    }

    #[command]
    pub fn handle_save(&mut self, user: User, _: &SaveEvent, cx: &mut ViewContext<Self>) {
        if let Some(existing) = self.users.iter_mut().find(|u| u.id == user.id) {
            *existing = user;
        } else {
            self.users.push(user);
        }
        self.is_form_visible = false;
        self.editing_user = None;
        self.apply_filter();
        cx.notify();
    }

    #[command]
    pub fn handle_cancel(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.is_form_visible = false;
        self.editing_user = None;
        cx.notify();
    }

    #[command]
    pub fn handle_delete(&mut self, user_id: u64, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(user) = self.users.iter().find(|u| u.id == user_id) {
            self.deleting_user_id = Some(user_id);
            self.deleting_user_name = user.name.clone();
            self.is_delete_dialog_open = true;
            cx.notify();
        }
    }

    #[command]
    pub fn handle_delete_confirm(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if let Some(id) = self.deleting_user_id.take() {
            self.users.retain(|u| u.id != id);
            self.apply_filter();
        }
        self.is_delete_dialog_open = false;
        cx.notify();
    }

    #[command]
    pub fn handle_delete_cancel(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.is_delete_dialog_open = false;
        self.deleting_user_id = None;
        cx.notify();
    }

    #[command]
    pub fn handle_delete_dialog_close(&mut self, _: &DialogCloseEvent, cx: &mut ViewContext<Self>) {
        self.is_delete_dialog_open = false;
        self.deleting_user_id = None;
        cx.notify();
    }

    #[command]
    pub fn handle_search(&mut self, ev: &SearchEvent, cx: &mut ViewContext<Self>) {
        self.search_query = ev.query.clone();
        self.apply_filter();
        cx.notify();
    }

    #[command]
    pub fn handle_clear_search(&mut self, cx: &mut ViewContext<Self>) {
        self.search_query = SharedString::default();
        self.filtered_users = self.users.clone();
        cx.notify();
    }

    fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            self.filtered_users = self.users.clone();
        } else {
            self.filtered_users = self
                .users
                .iter()
                .filter(|u| u.name.to_lowercase().contains(&query))
                .cloned()
                .collect();
        }
    }
}
```

## 6.5.9 小结

组件组合是构建大型应用的核心：

- **父子通信**：父 → 子（属性），子 → 父（事件）
- **兄弟通信**：通过共同父视图协调
- **依赖注入**：跨层级通信，避免属性逐层传递
- **高阶组件**：增强组件功能
- **组合模式**：容器+展示、复合组件、Render Props

掌握组件组合，你就能构建出结构清晰、易于维护的大型应用。

下一章 → [第 7 章 · 样式与主题](../07-styling/INDEX.md)
