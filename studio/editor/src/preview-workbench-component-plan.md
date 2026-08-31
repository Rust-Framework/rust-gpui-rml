# IWorkbenchComponent 体系实施计划

> 本文档是 Arc Studio IDE 工作台组件体系(`IWorkbenchComponent` + `IWorkbenchComponentHost`
>
> * 共享文档/状态模型)的**待实施计划**,不是架构演进记录。所有内容面向实施,decision-complete。
>
> **强约束**:
>
> 1. studio 所有界面开发禁止 Rust 代码构建 UI,必须走 RML 推荐声明式开发方法,严格遵循 MVVM 模式。
>    任何 `impl IVisual::render` 内出现 `div()/v_flex()/h_flex()` 等直接构造 GPUI 元素的代码均视为违规。
> 2. `IWorkbenchComponent` 实现类统一命名 `XXXComponent`,避免与 `IWorkbench` 实现类(`XXXWorkbench`)冲突。

## 1. 摘要

落地 `IWorkbenchComponent` 多态呈现体系,核心交付:

1. **命名规范统一**:`CodeWorkbench` → `CodeComponent`,`PreviewWorkbench` → `PreviewComponent`,
   `RmlDesignComponent` 已符合。`EditorWorkbench`(IWorkbench 实现)保持不变。
2. **IWorkbench 统筹管理多个 IWorkbenchComponent**:新增 `IWorkbenchComponentHost` trait,
   `EditorWorkbench` impl 它,提供组件枚举、激活切换、共享文档/状态访问能力。
3. **统一视图状态管理**:新增 `WorkbenchState` 共享 Entity,跨组件管理 `dirty`/`saving` 等状态,
   避免"切换组件丢失修改标记"问题。
4. **组件间数据同步**:新增 `WorkbenchDocument` 共享 Entity,作为组件间数据同步媒介。
   任何组件修改文档 → GPUI Entity 通知 → 其他组件 observe 触发重新同步。
5. **加速 Phase 2**:`EditorWorkbench` 改造为纯壳(Header + Body 容器),Body 经条件分支
   渲染激活的 `IWorkbenchComponent`。`CodeComponent` 从 `EditorWorkbench` 接管代码编辑逻辑。

## 2. 背景与动机

### 2.1 现状缺口

| 文件                                                                                                        | 现状                                                          | 问题                                                                 |
| --------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------------ |
| [component.rs](../../../studio/core/src/component.rs)                          | 定义 `IWorkbenchComponent: IVisualContribution` + `matches()` | trait 契约完备,但缺少 host 协作契约                                           |
| [code\_workbench.rs](../../../studio/editor/src/code_workbench.rs)             | `IVisual::render` 直接 `div().into_any_element()`             | **违反 MVVM**:Rust 代码构造 UI;命名 `CodeWorkbench` 与 `EditorWorkbench` 冲突 |
| [editor\_workbench.rml.rs](../../../studio/editor/src/editor_workbench.rml.rs) | 持有 `editor_state`/`language_client`,直接渲染 `<CodeEditor>`     | 既是 IWorkbench 又是代码视图,无法多态切换;无组件间数据同步机制                             |
| —                                                                                                         | 无 `PreviewComponent` 实现                                     | `.md` 文件只能走代码视图,无只读富文本预览                                           |
| —                                                                                                         | 无共享文档/状态模型                                                  | design 视图编辑后切换到 code 视图看不到最新数据                                     |

### 2.2 业务目标

1. **多态呈现**:`.md` 文件同时匹配 `CodeComponent`(默认)与 `PreviewComponent`,
   `EditorWorkbench` Header 显示 `Code | Preview` 切换按钮,用户点击切换激活组件。
2. **数据一致性**:任意组件编辑文档后,切换到其他组件看到最新内容。
   例:design 视图编辑 RML AST → 写回 `WorkbenchDocument` → 切换到 code 视图显示新源码。
3. **状态一致性**:跨组件统一管理 `dirty`/`saving` 状态,Tab 标题修改标记不因切换组件而丢失。
4. **MVVM 合规**:全部视图经 `.rml` 模板 + `#[component]` ViewModel 生成,Rust 代码
   仅承载状态、命令、计算属性,不直接构造 UI 元素。

## 3. 设计原则(强约束)

### 3.1 声明式 UI 优先

* **禁止**:`impl IVisual::render` 内出现 `div()` / `v_flex()` / `h_flex()` / `img()`
  等任何 GPUI 直接元素构造,以及链式 `.child(...)` / `.when(...)` 拼装。

* **允许**:`impl IVisual::render` 内仅做"取 Entity → 同步状态 → 委托 `Render::render`"。

* **强制**:每个 `IWorkbenchComponent` 实现必须配套 `.rml` 模板文件,经 `#[component]`
  宏生成 `impl Render`,由框架 `include!` 引入。

### 3.2 命名规范

| 类型                        | 命名模式           | 示例                                                      |
| ------------------------- | -------------- | ------------------------------------------------------- |
| `IWorkbench` 实现类          | `XXXWorkbench` | `EditorWorkbench`、`CaseWorkbench`                       |
| `IWorkbenchComponent` 实现类 | `XXXComponent` | `CodeComponent`、`PreviewComponent`、`RmlDesignComponent` |

**理由**:`IWorkbench` 是资源会话(Tab 一对一),`IWorkbenchComponent` 是呈现组件(会话内多态)。
两者语义不同,命名应区分,避免 `CodeWorkbench` 与 `EditorWorkbench` 混淆"谁是 Tab 谁是组件"。

### 3.3 MVVM 分层

| 层                   | 职责                                                                              | 文件后缀                  |
| ------------------- | ------------------------------------------------------------------------------- | --------------------- |
| **Model/ViewModel** | 状态字段、`#[computed]` 计算属性、命令、`ILifecycle::on_loaded`                              | `.rml.rs`             |
| **View**            | 声明式模板,数据绑定 `{field}`,指令 `if`/`each`/`on-click`                                  | `.rml`                |
| **Trait 契约**        | `IContribution`/`IWorkbenchComponent`/`IWorkbenchComponentHost` 元数据 + render 入口 | `.rs`(同 ViewModel 文件) |

### 3.4 与框架既有模式对齐

参考 [editor\_workbench.rml.rs](../../../studio/editor/src/editor_workbench.rml.rs)
的成熟模式:

1. `#[component]` + `#[derive(Default)]` 标注 struct,生成 `IModel + IViewModel + IComponent + Render`
2. 手动 `impl IContribution + IVisual + ILifecycle + IWorkbenchComponent` 补充元数据与入口
3. `IVisual::render` 内调用 `get_or_create_entity::<Self>(cx)` + `entity.update(...)` 同步状态,
   最终 `this.render(window, ctx).into_any_element()` 委托给 `#[component]` 生成的 `Render`
4. `#[ctor::ctor]` 注册 `register_*_ability::<Self>()` + `register_workbench_component(factory)`

## 4. 架构总览

### 4.1 组件协作关系

```
EditorWorkbench(IWorkbench + IWorkbenchComponentHost, #[component])
  ├─ document: Entity<WorkbenchDocument>      ← 共享文档(组件间数据同步媒介)
  ├─ state: Entity<WorkbenchState>            ← 共享状态(dirty/saving/last_error)
  ├─ uri: SharedString / file_path: PathBuf   ← 资源标识
  ├─ view_names: Vec<SharedString>            ← 匹配组件名(each 数据源,字段非 computed)
  ├─ active_component_id: SharedString        ← 激活组件 id(默认首个匹配)
  │
  └─ 受理组件(经 register_workbench_component 全局注册 + matches(uri) 过滤):
       ├─ CodeComponent      (id="code",    matches=true 默认)
       ├─ PreviewComponent   (id="preview", matches=.md/.markdown/.html)
       └─ RmlDesignComponent (id="design",  matches=.rml,后续计划)
```

### 4.2 数据流:组件间同步

```
                    ┌─────────────────────────────────┐
                    │  WorkbenchDocument (Entity)     │
                    │  ─ uri: SharedString            │
                    │  ─ content: SharedString        │  ← 单一真相源
                    │  ─ original: SharedString│
                    │  ─ kind: DocumentKind           │
                    └────────────┬────────────────────┘
                                 │ GPUI Entity observe
          ┌──────────────────────┼──────────────────────┐
          ▼                      ▼                      ▼
   ┌──────────────┐       ┌──────────────┐       ┌──────────────┐
   │CodeComponent │       │PreviewComp.  │       │DesignComp.   │
   │              │       │              │       │              │
   │ input_state  │       │ content      │       │ ast          │
   │   ↓ observe  │       │   ↑ read     │       │   ↓ edit     │
   │ 写回 document│       │ (只读)       │       │ 写回 document│
   └──────────────┘       └──────────────┘       └──────────────┘
```

**同步链路**:

1. **编辑触发**:用户在 `CodeComponent` 编辑 → `InputState` 变更 → `observe` 回调
   → `document.update(cx, |d, _| d.set_content(new_text))`
2. **文档通知**:`WorkbenchDocument` 是 GPUI Entity,任何 `update` 触发所有 observers
3. **状态联动**:`EditorWorkbench` observe `document` → 比对 `original`
   → `state.update(cx, |s, _| s.set_dirty(true))` → Tab 标题显示修改标记
4. **切换同步**:用户点 `Preview` → `active_component_id = "preview"` → Body 条件渲染
   `PreviewComponent` → `PreviewComponent::render` 读 `document.content` → 显示最新内容

### 4.3 数据流:Tab 切换

```
1. 用户切 Tab → EditorWorkbench.render 检测 uri_changed → reload()
2. reload():
   ├─ 读新文件 → document.set_content(新内容)
   ├─ compute_view_names(matches 过滤)
   └─ active_component_id = 首个匹配组件 id
3. document 变化 → 所有 observe(document) 的组件触发重新同步
4. CodeComponent.render → 检测 last_synced_content != document.content
   → input_state.set_value(新内容) → 显示新 Tab 的文件内容
```

### 4.4 EditorWorkbench 内部结构

```
EditorWorkbench(#[component])
  ├─ 字段:
  │    ├─ uri: SharedString                    ← 资源 URI
  │    ├─ file_path: PathBuf                   ← 本地路径
  │    ├─ document: Entity<WorkbenchDocument>  ← 共享文档(组件间同步媒介)
  │    ├─ state: Entity<WorkbenchState>        ← 共享状态(dirty/saving)
  │    ├─ view_names: Vec<SharedString>        ← 匹配组件名(字段,each 要求)
  │    └─ active_component_id: SharedString    ← 激活组件 id
  │
  ├─ #[computed] show_view_switcher → bool     ← view_names.len() > 1
  ├─ #[computed] is_code_active → bool         ← active_component_id == "code"
  ├─ #[computed] is_preview_active → bool      ← active_component_id == "preview"
  ├─ #[computed] breadcrumb_text → SharedString
  ├─ #[computed] dirty_mark → SharedString     ← state.dirty ? "●" : ""
  │
  ├─ ILifecycle::on_loaded:
  │    ├─ 读文件 → document.set_initial_content + set_content
  │    ├─ compute_view_names(matches 过滤)
  │    ├─ active_component_id = 首个匹配
  │    └─ observe(document) → state.set_dirty(document.is_dirty())
  │
  └─ 命令:
       └─ switch_to(name) → 反查 component_id → active_component_id = id
```

## 5. 核心契约

### 5.1 IWorkbenchComponent(无变更)

[studio/core/src/component.rs](../../../studio/core/src/component.rs)
中的 `IWorkbenchComponent` trait 保持不变:

```rust
pub trait IWorkbenchComponent: IVisualContribution {
    /// 判断此组件是否能处理指定 URI 的资源。
    /// 默认返回 `true` —— 作为默认视图组件(如 CodeComponent)。
    /// 特化组件(如 PreviewComponent 仅 .md/.html)应 override 此方法。
    fn matches(&self, _uri: &Uri) -> bool {
        true
    }
}
```

### 5.2 IWorkbenchComponentHost(新增)

`IWorkbench` 实现按需 impl 此 trait,统筹管理多个 `IWorkbenchComponent`。
**不强制**所有 IWorkbench impl —— 不受理子组件的工作台(如 demo 的 `CaseWorkbench`)不 impl 即可。
这与 project\_memory 硬约束一致:"IWorkbench super trait 仅含 IContribution + IVisual,
Host 状态由实现决定"。

```rust
// studio/core/src/component.rs(追加)

use gpui::Entity;
use crate::document::{WorkbenchDocument, WorkbenchState};

/// 工作台组件宿主 —— IWorkbench 实现按需 impl,统筹管理多个 IWorkbenchComponent。
///
/// 提供三大能力:
/// 1. **组件枚举与激活**:`components()` / `active_component_id()` / `switch_component()`
/// 2. **共享文档访问**:`document()` —— 组件间数据同步的媒介
/// 3. **共享状态访问**:`state()` —— 跨组件统一管理 dirty/saving 等
///
/// # 实现示例
///
/// `EditorWorkbench` impl 此 trait,受理 `CodeComponent` / `PreviewComponent` 等。
/// 组件经 `get_or_create_entity::<EditorWorkbench>(cx)` 取 host,再读 document/state。
pub trait IWorkbenchComponentHost {
    /// 此工作台受理的所有组件(经 `matches(uri)` 过滤)。
    fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>>;

    /// 当前激活的组件 id。
    fn active_component_id(&self) -> SharedString;

    /// 切换激活组件。id 不在 `components()` 中时为 no-op。
    fn switch_component(&self, id: &str);

    /// 共享文档模型 —— 组件间数据同步的媒介。
    ///
    /// 组件 observe 此 Entity,任何组件修改 content → 通知所有 observers。
    fn document(&self) -> Entity<WorkbenchDocument>;

    /// 共享工作台状态 —— 跨组件统一管理 dirty/saving 等。
    fn state(&self) -> Entity<WorkbenchState>;
}
```

### 5.3 WorkbenchDocument(新增)

共享文档模型,作为组件间数据同步的单一真相源。

**文档类型采用开放字符串设计**(非封闭枚举),支持插件自由扩展:

```rust
// studio/core/src/document.rs(新文件)

use gpui::SharedString;

/// 文档类型标识常量 —— 开放扩展,插件可自由定义新类型。
///
/// 框架内置常用类型,插件可用任意字符串作为 `WorkbenchDocument::kind()`。
/// 组件优先经 `IWorkbenchComponent::matches(uri)` 判断适配性,
/// `kind` 作为辅助元数据供组件条件渲染参考。
pub mod document_kind {
    pub const TEXT: &str = "text";
    pub const MARKDOWN: &str = "markdown";
    pub const HTML: &str = "html";
    pub const RML: &str = "rml";
}

/// 共享文档模型 —— IWorkbenchComponent 间数据同步的媒介。
///
/// 持有当前文本内容 + 加载时原始内容(用于 dirty 判断)。
/// 任何组件修改 `content` → GPUI Entity 通知 → 其他组件 observe 触发重新渲染。
///
/// # 文档类型开放性
///
/// `kind` 是 `SharedString` 而非枚举,插件可自由定义新类型(如 "pdf"/"svg")。
/// 框架在 `document_kind` 模块提供常用类型常量。组件用
/// `document.kind() == document_kind::MARKDOWN` 判断,或直接经
/// `IWorkbenchComponent::matches(uri)` 基于扩展名判断(不依赖 kind)。
pub struct WorkbenchDocument {
    uri: SharedString,
    content: SharedString,
    original: SharedString,
    kind: SharedString,
}

impl Default for WorkbenchDocument {
    fn default() -> Self {
        Self {
            uri: SharedString::default(),
            content: SharedString::default(),
            original: SharedString::default(),
            kind: document_kind::TEXT.into(),
        }
    }
}

impl WorkbenchDocument {
    pub fn new(uri: SharedString, content: SharedString, kind: impl Into<SharedString>) -> Self {
        Self {
            original: content.clone(),
            content,
            uri,
            kind: kind.into(),
        }
    }

    pub fn uri(&self) -> SharedString { self.uri.clone() }
    pub fn content(&self) -> SharedString { self.content.clone() }
    pub fn kind(&self) -> SharedString { self.kind.clone() }
    pub fn original(&self) -> SharedString { self.original.clone() }

    pub fn set_content(&mut self, content: SharedString) {
        self.content = content;
    }

    pub fn reload(&mut self, uri: SharedString, content: SharedString, kind: impl Into<SharedString>) {
        self.uri = uri;
        self.original = content.clone();
        self.content = content;
        self.kind = kind.into();
    }

    pub fn is_dirty(&self) -> bool {
        self.content != self.original
    }

    pub fn mark_saved(&mut self) {
        self.original = self.content.clone();
    }
}
```

**为何不用封闭枚举 `DocumentKind`**:
- 封闭枚举限制插件扩展(插件无法添加 PDF/SVG/JSON-Tree 等新类型)
- 组件适配性判断已由 `IWorkbenchComponent::matches(uri)` 开放提供,`kind` 仅作辅助元数据
- 开放字符串 + 常量模块既保证框架内置类型的便利性,又允许插件自由扩展

### 5.4 WorkbenchState(新增)

跨组件统一管理的工作台状态。

```rust
// studio/core/src/document.rs(追加)

/// 工作台共享状态 —— 跨组件统一管理。
///
/// 不让每个组件各自管 dirty 标记,避免"切换组件丢失修改标记"问题。
/// `EditorWorkbench` observe `WorkbenchDocument` 变化 → 更新此状态 → Tab 标题联动。
#[derive(Default)]
pub struct WorkbenchState {
    dirty: bool,
    saving: bool,
    last_error: Option<SharedString>,
}

impl WorkbenchState {
    pub fn dirty(&self) -> bool { self.dirty }
    pub fn saving(&self) -> bool { self.saving }
    pub fn last_error(&self) -> Option<SharedString> { self.last_error.clone() }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub fn set_saving(&mut self, saving: bool) {
        self.saving = saving;
    }

    pub fn set_error(&mut self, error: Option<SharedString>) {
        self.last_error = error;
    }
}
```

### 5.5 能力扩展(ability\_ext.rs 追加)

```rust
// studio/core/src/ability_ext.rs(追加)

use crate::component::IWorkbenchComponentHost;

/// 工作台组件宿主能力扩展 —— 让 `dyn IValue` 可查询 `IWorkbenchComponentHost` 能力。
pub trait WorkbenchComponentHostAbilityExt {
    fn as_workbench_component_host(&self) -> Option<&dyn IWorkbenchComponentHost>;
}

#[allow(unsafe_code)]
impl WorkbenchComponentHostAbilityExt for dyn IValue {
    fn as_workbench_component_host(&self) -> Option<&dyn IWorkbenchComponentHost> {
        let erased = rml_core::ability::query::<dyn IWorkbenchComponentHost>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkbenchComponentHost>(erased) })
    }
}

#[allow(unsafe_code)]
pub fn register_workbench_component_host_ability<T: IWorkbenchComponentHost + 'static>() {
    rml_core::ability::register::<T, dyn IWorkbenchComponentHost>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let h: &dyn IWorkbenchComponentHost = s;
            unsafe { rml_core::ability::erase(h) }
        })
    });
}
```

## 6. 实施计划(分阶段)

### Phase 1:Core 契约扩展(前置)

| 步骤  | 任务                                                         | 文件                                   | 验证                         |
| --- | ---------------------------------------------------------- | ------------------------------------ | -------------------------- |
| 1.1 | 新增 `WorkbenchDocument` + `WorkbenchState` + `DocumentKind` | `studio/core/src/document.rs`(新)     | 编译通过                       |
| 1.2 | 新增 `IWorkbenchComponentHost` trait                         | `studio/core/src/component.rs`(追加)   | 编译通过                       |
| 1.3 | 新增 `WorkbenchComponentHostAbilityExt` + register 函数        | `studio/core/src/ability_ext.rs`(追加) | 编译通过                       |
| 1.4 | `lib.rs` 导出新模块                                             | `studio/core/src/lib.rs`             | `pub mod document;` 可被外部引用 |

### Phase 2:EditorWorkbench 改造为壳 + Host

| 步骤  | 任务                                                                     | 文件                         | 验证                                                               |
| --- | ---------------------------------------------------------------------- | -------------------------- | ---------------------------------------------------------------- |
| 2.1 | 移除 `editor_state`/`language_client` 字段                                 | `editor_workbench.rml.rs`  | 编译通过(临时占位)                                                       |
| 2.2 | 新增 `document`/`state`/`active_component_id` 字段                         | 同 2.1                      | 字段初始化成功                                                          |
| 2.3 | impl `IWorkbenchComponentHost`                                         | 同 2.1                      | `as_workbench_component_host()` 查询生效                             |
| 2.4 | `on_loaded` 改造:读文件 → `document.reload()` + `compute_view_names` + 默认激活 | 同 2.1                      | 打开文件后 document.content 正确                                        |
| 2.5 | observe `document` → `state.set_dirty`                                 | 同 2.1                      | 编辑后 state.dirty = true                                           |
| 2.6 | RML 模板 Body 改为条件分支                                                     | `editor_workbench.rml`     | `<CodeComponentView if={is_code_active} />` 等                    |
| 2.7 | 注册 `IWorkbenchComponentHost` 能力                                        | `lib.rs` 的 `#[ctor::ctor]` | `register_workbench_component_host_ability::<EditorWorkbench>()` |

### Phase 3:CodeComponent 落地(从 EditorWorkbench 接管代码编辑)

| 步骤  | 任务                                    | 文件                                                 | 验证                                           |
| --- | ------------------------------------- | -------------------------------------------------- | -------------------------------------------- |
| 3.1 | 新增 `CodeComponent` ViewModel          | `code_component.rml.rs`(新,改名自 `code_workbench.rs`) | 编译通过                                         |
| 3.2 | 新增 `CodeComponent` RML 模板             | `code_component.rml`(新)                            | `<CodeEditor>` 渲染正常                          |
| 3.3 | 接管 `InputState` + `LanguageClient` 逻辑 | 同 3.1                                              | 代码编辑 + LSP 集成不回归                             |
| 3.4 | observe `document` → 同步 `input_state` | 同 3.1                                              | Tab 切换后显示新文件内容                               |
| 3.5 | observe `input_state` → 写回 `document` | 同 3.1                                              | 编辑后 document.content 更新                      |
| 3.6 | 删除 `code_workbench.rs`(旧文件)           | —                                                  | 无残留引用                                        |
| 3.7 | 注册 `CodeComponent` 能力 + 工厂            | `lib.rs`                                           | `get_workbench_components()` 含 CodeComponent |

### Phase 4:PreviewComponent 落地

| 步骤  | 任务                                        | 文件                            | 验证                                              |
| --- | ----------------------------------------- | ----------------------------- | ----------------------------------------------- |
| 4.1 | 新增 `PreviewComponent` ViewModel           | `preview_component.rml.rs`(新) | 编译通过                                            |
| 4.2 | 新增 `PreviewComponent` RML 模板              | `preview_component.rml`(新)    | `.md` 渲染为 GFM 富文本                               |
| 4.3 | `matches()` 仅匹配 `.md`/`.markdown`/`.html` | 同 4.1                         | `.rs` 文件不显示 Preview 按钮                          |
| 4.4 | observe `document` → 重新渲染                 | 同 4.1                         | code 编辑后切换 preview 看到最新                         |
| 4.5 | 注册能力 + 工厂                                 | `lib.rs` 的 `#[ctor::ctor]`    | `get_workbench_components()` 含 PreviewComponent |

## 7. 文件结构(目标)

```
studio/core/src/
├── lib.rs                  ← 导出 document 模块 + 新契约
├── ability_ext.rs          ← 追加 IWorkbenchComponentHost 能力扩展
├── component.rs            ← IWorkbenchComponent(无变) + IWorkbenchComponentHost(新)
├── document.rs             ← WorkbenchDocument + WorkbenchState + DocumentKind(新)
├── command.rs              ← 无变更
├── worktree.rs             ← 无变更
├── workspace.rs            ← 无变更
└── registry.rs             ← 无变更

studio/editor/src/
├── lib.rs                  ← 注册 CodeComponent + PreviewComponent + Host 能力
├── editor_provider.rs      ← 无变更
├── editor_workbench.rml    ← Header + Body 条件渲染
├── editor_workbench.rml.rs ← IWorkbench + IWorkbenchComponentHost impl(改造为壳)
├── code_component.rml      ← 新(声明式模板)
├── code_component.rml.rs   ← 新(改名自 code_workbench.rs,实际重写)
├── preview_component.rml   ← 新
└── preview_component.rml.rs← 新
```

## 8. RML 模板规范

### 8.1 editor\_workbench.rml(Phase 2 改造后)

```xml
<component>
    <div display="flex" flex-direction="column" width="full" height="full" class="editor-pane">
        <!-- Header: 面包屑 + dirty 标记 + 视图切换按钮 -->
        <div class="editor-header" display="flex" align-items="center"
             justify-content="space-between" height="36px" flex-shrink="0"
             padding-left="12px" padding-right="8px">
            <span class="breadcrumb-text" font-size="13px">
                {dirty_mark}{breadcrumb_text}
            </span>
            <div if={show_view_switcher} class="view-switcher"
                 display="flex" align-items="center" gap="4px">
                <span each={name in view_names} class="view-button"
                      font-size="12px" padding-left="6px" padding-right="6px"
                      on-click={switch_to(name)}>{name}</span>
            </div>
        </div>

        <!-- Body: 按 active_component_id 条件分支渲染 -->
        <div class="editor-area" flex="1" min-height="0">
            <CodeComponentView if={is_code_active} />
            <PreviewComponentView if={is_preview_active} />
        </div>
    </div>
</component>
```

**关键点**:

* `dirty_mark` 计算属性:`state.dirty() ? "● " : ""`,与文件名拼接显示

* `switch_to(name)` 命令:反查 name → component\_id → 写入 `active_component_id`

* Body 经 `if` 条件分支,等待 RML 参数化模板能力后可重构为 `<DynamicComponent id={active_component_id} />`

### 8.2 code\_component.rml(Phase 3)

```xml
<component>
    <div display="flex" width="full" height="full" class="code-pane">
        <CodeEditor class="rml-code-editor" height="full" ref="editor" />
    </div>
</component>
```

### 8.3 preview\_component.rml(Phase 4)

```xml
<component>
    <div display="flex" flex-direction="column" width="full" height="full" class="preview-pane">
        <!-- Header: 面包屑 + 只读标记 -->
        <div class="preview-header" display="flex" align-items="center"
             justify-content="space-between" height="36px" flex-shrink="0"
             padding-left="12px" padding-right="12px">
            <span class="breadcrumb-text" font-size="13px">{breadcrumb_text}</span>
            <span class="readonly-badge" font-size="11px"
                  padding-left="6px" padding-right="6px">Read-only</span>
        </div>

        <!-- Body: 按 document.kind 分支渲染 -->
        <div class="preview-area" flex="1" min-height="0" overflow="auto" padding="16px">
            <Markdown if={is_markdown} content={content}
                      style="background: var(--surface-variant); border-radius: 6px; padding: 16px;" />
            <pre if={is_html} class="html-source"
                 font-family="monospace" font-size="13px"
                 white-space="pre-wrap">{content}</pre>
            <pre if={is_text} class="text-source"
                 font-family="monospace" font-size="13px"
                 white-space="pre-wrap">{content}</pre>
        </div>
    </div>
</component>
```

## 9. ViewModel 规范

### 9.1 EditorWorkbench(Phase 2 改造后)

```rust
//! EditorWorkbench ViewModel —— IWorkbench + IWorkbenchComponentHost,纯壳。
//!
//! 不再直接持有 `editor_state`/`language_client` —— 代码编辑逻辑由 `CodeComponent` 接管。
//! EditorWorkbench 仅负责:
//! 1. 资源会话管理(IWorkbench):uri/close/activate/closable
//! 2. 组件宿主管理(IWorkbenchComponentHost):枚举/激活/切换 + 共享文档/状态
//! 3. Header 渲染:面包屑 + dirty 标记 + 视图切换按钮
//! 4. Body 容器:经条件分支渲染激活的 IWorkbenchComponent

use std::path::PathBuf;
use std::sync::Arc;

use gpui::{AnyElement, App, Entity, SharedString, Window};
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{IconSpec, IContribution, IVisual};
use rml_core::workbench::{IWorkbench, Uri};
use studio_core::ability_ext::{WorkbenchComponentAbilityExt, register_workbench_component_host_ability};
use studio_core::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use studio_core::document::{DocumentKind, WorkbenchDocument, WorkbenchState};
use studio_core::get_workbench_components;

#[component]
#[derive(Default)]
pub struct EditorWorkbench {
    uri: SharedString,
    file_path: PathBuf,
    document: Option<Entity<WorkbenchDocument>>,
    state: Option<Entity<WorkbenchState>>,
    /// 匹配当前 URI 的视图组件名称列表(each 指令要求字段而非方法)。
    view_names: Vec<SharedString>,
    active_component_id: SharedString,
}

impl IContribution for EditorWorkbench {
    fn id(&self) -> &str { &self.uri }
    fn name(&self) -> SharedString {
        self.file_path.file_name().and_then(|n| n.to_str()).unwrap_or("untitled").into()
    }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("File")) }
}

impl IVisual for EditorWorkbench {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<EditorWorkbench>(cx);
        let uri = self.uri.clone();
        let file_path = self.file_path.clone();
        entity.update(cx, |this, ctx| {
            let uri_changed = this.uri != uri;
            this.uri = uri;
            this.file_path = file_path;
            if uri_changed {
                this.reload(ctx);
            }
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for EditorWorkbench {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 初始化共享 document + state
        self.document = Some(cx.new(|_| WorkbenchDocument::default()));
        self.state = Some(cx.new(|_| WorkbenchState::default()));
        self.reload(cx);
    }
}

impl IWorkbench for EditorWorkbench {
    fn uri(&self) -> &str { &self.uri }
    fn close(&self) {}
    fn activate(&self) {}
    fn set(&self, _key: SharedString, _value: Box<dyn std::any::Any + Send + Sync>) {}
    fn closable(&self) -> bool { true }
}

impl IWorkbenchComponentHost for EditorWorkbench {
    fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>> {
        let Ok(uri) = self.uri.parse::<Uri>() else { return Vec::new(); };
        get_workbench_components()
            .into_iter()
            .filter_map(|c| {
                c.as_workbench_component()
                    .filter(|wc| wc.matches(&uri))
                    .map(|_| {
                        // 经 as_workbench_component 返回 &dyn,需 Arc clone
                        // 实际实现:downcast 到 Arc<dyn IContribution> 后再 as_workbench_component
                        // 此处简化,真实代码需调整注册表返回 Arc<dyn IWorkbenchComponent>
                        unimplemented!("见 9.4 注册表调整")
                    })
            })
            .collect()
    }

    fn active_component_id(&self) -> SharedString {
        self.active_component_id.clone()
    }

    fn switch_component(&self, _id: &str) {
        // 经 Entity 内部可变性更新 active_component_id
        // 真实实现需 RwLock 或经 Entity::update
    }

    fn document(&self) -> Entity<WorkbenchDocument> {
        self.document.expect("document initialized in on_loaded")
    }

    fn state(&self) -> Entity<WorkbenchState> {
        self.state.expect("state initialized in on_loaded")
    }
}

impl EditorWorkbench {
    /// 重新加载:读文件 → document.reload → compute_view_names → 默认激活。
    fn reload(&mut self, cx: &mut Context<Self>) {
        self.view_names = self.compute_view_names();

        // 默认激活首个匹配组件
        if self.active_component_id.is_empty() {
            self.active_component_id = self
                .components()
                .first()
                .map(|c| c.id().to_string().into())
                .unwrap_or_default();
        }

        // 读文件 → document.reload
        let kind = infer_kind(&self.file_path);
        let content = std::fs::read_to_string(&self.file_path).unwrap_or_default().into();
        if let Some(doc) = self.document {
            doc.update(cx, |d, _| d.reload(self.uri.clone(), content, kind));
        }

        // observe document → state.set_dirty
        if let Some(doc) = self.document {
            if let Some(state) = self.state {
                let state_clone = state.clone();
                cx.observe(&doc, move |_: &mut Self, doc, cx| {
                    let dirty = doc.read(cx).is_dirty();
                    state_clone.update(cx, |s, _| s.set_dirty(dirty));
                }).detach();
            }
        }

        cx.notify();
    }

    fn compute_view_names(&self) -> Vec<SharedString> {
        let Ok(uri) = self.uri.parse::<Uri>() else { return Vec::new(); };
        get_workbench_components()
            .iter()
            .filter_map(|c| {
                c.as_workbench_component()
                    .filter(|wc| wc.matches(&uri))
                    .map(|wc| wc.name())
            })
            .collect()
    }

    #[computed]
    pub fn breadcrumb_text(&self) -> SharedString {
        if self.file_path.as_os_str().is_empty() { return "untitled".into(); }
        let segments: Vec<&std::ffi::OsStr> = self.file_path.iter().rev().take(3).collect();
        segments.into_iter().rev().filter_map(|s| s.to_str())
            .collect::<Vec<_>>().join(" › ").into()
    }

    #[computed]
    pub fn show_view_switcher(&self) -> bool { self.view_names.len() > 1 }

    #[computed]
    pub fn is_code_active(&self) -> bool { self.active_component_id == "code" }

    #[computed]
    pub fn is_preview_active(&self) -> bool { self.active_component_id == "preview" }

    #[computed]
    pub fn dirty_mark(&self) -> SharedString {
        // 读 state Entity 判断 dirty
        // 真实实现:state.read(cx).dirty() —— 但 #[computed] 无 cx 参数
        // 改为字段缓存 dirty,在 observe 回调中更新
        // 见 10.3 computed 与 Entity 读取限制
        SharedString::default()
    }

    /// 设置文件路径和 URI(由 EditorProvider 调用)。
    pub fn set_file(&mut self, uri: SharedString, file_path: PathBuf) {
        self.uri = uri;
        self.file_path = file_path;
    }
}

fn infer_kind(path: &std::path::Path) -> DocumentKind {
    match path.extension().and_then(|e| e.to_str()) {
        Some("md") | Some("markdown") => DocumentKind::Markdown,
        Some("html") | Some("htm") => DocumentKind::Html,
        Some("rml") => DocumentKind::Rml,
        _ => DocumentKind::Text,
    }
}

// 能力注册
pub fn register_editor_abilities() {
    use rml_core::contribution::{register_contribution_ability, register_visual_ability};
    use rml_core::workbench::register_workbench_ability;
    use std::sync::Once;
    static ABILITY_REGISTERED: Once = Once::new();
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<EditorWorkbench>();
        register_visual_ability::<EditorWorkbench>();
        register_workbench_ability::<EditorWorkbench>();
        register_workbench_component_host_ability::<EditorWorkbench>();
    });
}
```

### 9.2 CodeComponent(Phase 3)

```rust
//! CodeComponent ViewModel —— 默认代码编辑视图组件(IWorkbenchComponent)。
//!
//! 从 EditorWorkbench 接管代码编辑逻辑(InputState + LanguageClient)。
//! 经 observe(document) 同步文件内容,经 observe(input_state) 写回 document。

use std::sync::Arc;

use gpui::{AnyElement, App, SharedString, Window};
use gpui_component::input::InputState;
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{IconSpec, IContribution, IVisual};
use rml_core::workbench::Uri;
use rust_rml_client::{file_path_to_uri, LanguageClient};
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::IWorkbenchComponent;
use studio_core::register_workbench_component;

#[component]
#[derive(Default)]
pub struct CodeComponent {
    editor_state: Option<gpui::Entity<InputState>>,
    language_client: Option<Arc<LanguageClient>>,
    /// 上次同步到 input_state 的内容,避免循环同步。
    last_synced_content: SharedString,
}

impl IContribution for CodeComponent {
    fn id(&self) -> &str { "code" }
    fn name(&self) -> SharedString { "Code".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("FileCode")) }
}

impl IVisual for CodeComponent {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<CodeComponent>(cx);
        entity.update(cx, |this, ctx| {
            // 从 host 同步 document → input_state
            this.sync_from_document(ctx);
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for CodeComponent {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.init_editor(window, cx);
    }
}

impl IWorkbenchComponent for CodeComponent {
    // matches(uri) 使用默认实现(返回 true)—— CodeComponent 是默认文本视图
}

impl CodeComponent {
    /// 初始化编辑器:从 document 读内容 → 创建 InputState → 安装 LSP → observe 写回。
    fn init_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let host = get_or_create_entity::<crate::editor_workbench::EditorWorkbench>(cx);
        let doc = host.read(cx).document();
        let doc_read = doc.read(cx);
        let content = doc_read.content();
        let uri = doc_read.uri();
        let kind = doc_read.kind();
        drop(doc_read);

        // 解析 file_path(从 uri)
        let file_path = uri.parse::<Url>().ok()
            .and_then(|u| u.to_file_path().ok())
            .unwrap_or_default();
        let language = match kind {
            studio_core::document::DocumentKind::Rml => "rml",
            _ => detect_language(&file_path),
        };

        let state = cx.new(|cx| {
            InputState::new(window, cx).code_editor(language).multi_line(true).default_value(&content)
        });

        // observe input_state → 写回 document
        let doc_clone = doc.clone();
        cx.observe(&state, move |this: &mut Self, state, cx| {
            let new_text = state.read(cx).text().to_string();
            if this.last_synced_content != new_text.as_str() {
                this.last_synced_content = new_text.clone().into();
                doc_clone.update(cx, |d, _| d.set_content(new_text.into()));
            }
        }).detach();

        self.last_synced_content = content;
        self.editor_state = Some(state);
    }

    /// 从 document 同步内容到 input_state(Tab 切换时)。
    fn sync_from_document(&mut self, cx: &mut Context<Self>) {
        let host = get_or_create_entity::<crate::editor_workbench::EditorWorkbench>(cx);
        let content = host.read(cx).document().read(cx).content();
        if self.last_synced_content != content {
            if let Some(ref state) = self.editor_state {
                state.update(cx, |s, _| s.set_value(content.clone()));
            }
            self.last_synced_content = content;
        }
    }
}

fn detect_language(path: &std::path::Path) -> &str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "rust",
        Some("rml") => "rml",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("py") => "python",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("toml") | Some("lock") => "toml",
        Some("html") => "html",
        Some("css") => "css",
        _ => "plaintext",
    }
}

pub fn register_code_component() {
    register_workbench_component_ability::<CodeComponent>();
    register_workbench_component(|| {
        Arc::new(CodeComponent::default()) as Arc<dyn rml_core::contribution::IContribution>
    });
}
```

### 9.3 PreviewComponent(Phase 4)

```rust
//! PreviewComponent ViewModel —— 只读预览视图组件(IWorkbenchComponent)。

use std::path::PathBuf;

use gpui::{AnyElement, App, SharedString, Window};
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::{IconSpec, IContribution, IVisual};
use rml_core::workbench::Uri;
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::IWorkbenchComponent;
use studio_core::document::document_kind;
use studio_core::register_workbench_component;

#[component]
#[derive(Default)]
pub struct PreviewComponent {
    /// 缓存的文档内容(从 document 同步)。
    content: SharedString,
    /// 缓存的文档类型(开放字符串,从 document 同步)。
    kind: SharedString,
    /// 上次同步的内容,避免重复更新。
    last_synced_content: SharedString,
}

impl IContribution for PreviewComponent {
    fn id(&self) -> &str { "preview" }
    fn name(&self) -> SharedString { "Preview".into() }
    fn icon(&self) -> Option<IconSpec> { Some(IconSpec::named("FileText")) }
}

impl IVisual for PreviewComponent {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<PreviewComponent>(cx);
        entity.update(cx, |this, ctx| {
            this.sync_from_document(ctx);
            this.render(window, ctx).into_any_element()
        })
    }
}

impl ILifecycle for PreviewComponent {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.sync_from_document(cx);
    }
}

impl IWorkbenchComponent for PreviewComponent {
    fn matches(&self, uri: &Uri) -> bool {
        matches!(
            uri.path().rsplit('.').next(),
            Some("md") | Some("markdown") | Some("html")
        )
    }
}

impl PreviewComponent {
    /// 从 host 的 document 同步内容 + 类型。
    fn sync_from_document(&mut self, cx: &mut Context<Self>) {
        let host = get_or_create_entity::<crate::editor_workbench::EditorWorkbench>(cx);
        let doc = host.read(cx).document();
        let doc_read = doc.read(cx);
        let content = doc_read.content();
        let kind = doc_read.kind();
        drop(doc_read);

        if self.last_synced_content != content {
            self.content = content.clone();
            self.kind = kind;
            self.last_synced_content = content;
            cx.notify();
        }
    }

    #[computed]
    pub fn breadcrumb_text(&self) -> SharedString {
        // 从 host 读 file_path(简化,真实实现经 host 取)
        "preview".into()
    }

    /// 经开放字符串常量比较,支持插件自定义类型扩展。
    #[computed]
    pub fn is_markdown(&self) -> bool { self.kind == document_kind::MARKDOWN }

    #[computed]
    pub fn is_html(&self) -> bool { self.kind == document_kind::HTML }

    #[computed]
    pub fn is_text(&self) -> bool { self.kind == document_kind::TEXT }
}

pub fn register_preview_component() {
    register_workbench_component_ability::<PreviewComponent>();
    register_workbench_component(|| {
        Arc::new(PreviewComponent::default()) as Arc<dyn rml_core::contribution::IContribution>
    });
}
```

### 9.4 lib.rs 注册入口(Phase 2-4)

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;

#[path = "editor_workbench.rml.rs"]
pub mod editor_workbench;
#[path = "code_component.rml.rs"]
pub mod code_component;
#[path = "preview_component.rml.rs"]
pub mod preview_component;
pub mod editor_provider;

#[rml_core::ctor::ctor]
fn register_editor_services() {
    use std::sync::Arc;
    use rml_core::workbench::IWorkbenchProvider;
    use rust_rml_di::{auto_register, ServiceCollection};

    crate::editor_workbench::register_editor_abilities();
    crate::code_component::register_code_component();
    crate::preview_component::register_preview_component();
    auto_register(|s: &mut ServiceCollection| {
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", |_| {
            Arc::new(crate::editor_provider::EditorProvider) as Arc<dyn IWorkbenchProvider>
        });
    });
}
```

### 9.5 注册表调整(Phase 2)

当前 `register_workbench_component` 工厂返回 `Arc<dyn IContribution>`。为支持
`IWorkbenchComponentHost::components()` 返回 `Vec<Arc<dyn IWorkbenchComponent>>`,
需调整 [registry.rs](../../../studio/core/src/registry.rs):

**选项 A(推荐)**:工厂仍返回 `Arc<dyn IContribution>`,`components()` 内部经
`as_workbench_component()` 查询能力后,无法直接拿到 `Arc<dyn IWorkbenchComponent>`
(能力查询返回 `&dyn`,不是 `Arc`)。

**选项 B**:工厂改为返回 `Arc<dyn IWorkbenchComponent>`(强类型),注册表存
`Vec<Box<dyn Fn() -> Arc<dyn IWorkbenchComponent>>>`。调用方无需能力查询。

```rust
// studio/core/src/registry.rs(改造)

type WorkbenchComponentFactory = Box<dyn Fn() -> Arc<dyn IWorkbenchComponent> + Send + Sync>;

static WORKBENCH_COMPONENTS: OnceLock<Mutex<Vec<WorkbenchComponentFactory>>> = OnceLock::new();

pub fn register_workbench_component(
    f: impl Fn() -> Arc<dyn IWorkbenchComponent> + Send + Sync + 'static,
) {
    WORKBENCH_COMPONENTS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock().unwrap()
        .push(Box::new(f));
}

pub fn get_workbench_components() -> Vec<Arc<dyn IWorkbenchComponent>> {
    match WORKBENCH_COMPONENTS.get() {
        Some(registry) => registry.lock().unwrap().iter().map(|f| f()).collect(),
        None => Vec::new(),
    }
}
```

> 选项 B 更简洁,`components()` 直接 filter `matches(uri)` 即可。本计划采用选项 B。

## 10. 关键技术点

### 10.1 MVVM 合规的 IVisual::render 等价模式

```rust
// ✅ 合规:同步状态 + 委托 #[component] Render
impl IVisual for CodeComponent {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<CodeComponent>(cx);
        entity.update(cx, |this, ctx| {
            this.sync_from_document(ctx);           // 同步状态
            this.render(window, ctx).into_any_element()  // 委托 Render
        })
    }
}

// ❌ 违规:直接构造 GPUI 元素
impl IVisual for CodeComponent {
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        div().child(CodeEditor::new(...)).into_any_element()    // 禁止
    }
}
```

### 10.2 组件间数据同步:observe 链路

```
CodeComponent.edit
  → InputState 变化
  → cx.observe(&input_state, |this, state, cx| {
        let new_text = state.read(cx).text().to_string();
        if this.last_synced_content != new_text {  // 避免循环
            this.last_synced_content = new_text.clone().into();
            document.update(cx, |d, _| d.set_content(new_text.into()));
        }
    })

document 变化
  → EditorWorkbench cx.observe(&document, |_, doc, cx| {
        let dirty = doc.read(cx).is_dirty();
        state.update(cx, |s, _| s.set_dirty(dirty));
    })
  → PreviewComponent render 时 sync_from_document → 读到最新 content
```

**循环同步防护**:`last_synced_content` 字段记录上次同步内容,若相同则跳过 update,
避免 `input_state → document → input_state` 死循环。

### 10.3 computed 与 Entity 读取限制

`#[computed]` 方法签名为 `fn xxx(&self) -> T`,无 `cx` 参数,无法直接读 `Entity<WorkbenchState>`。
两种解法:

* **方案 A(推荐)**:在 `EditorWorkbench` 加 `dirty: bool` 字段,在 `observe(document)` 回调中更新,
  `#[computed] dirty_mark` 读字段。

* **方案 B**:把 `dirty_mark` 改成普通方法 `fn dirty_mark(&self, cx: &App) -> SharedString`,
  RML 模板内无法直接绑定(需经字段中转)。

本计划采用方案 A:`dirty` 字段缓存,observe 回调更新,`#[computed]` 读字段。

### 10.4 Entity 缓存与 URI 变更

`get_or_create_entity::<T>()` 按 `TypeId` 缓存,所有 `EditorWorkbench` 实例共享同一 Entity。

* Tab 切换 → `IVisual::render` 检测 `uri_changed` → `reload()` → `document.reload()`

* `CodeComponent` observe `document` → 触发 `sync_from_document` → `input_state` 更新

`CodeComponent` 也是单例 Entity,经 `last_synced_content` 判断是否需要重新同步。

### 10.5 .html 文件的降级策略

GPUI 无原生 HTML 渲染器,RML `html={expr}` 指令降级为文本节点(参考
[html\_case.rml](../../../demo/src/cases/html_case.rml))。
`.html` 预览使用 `<pre>` 纯文本展示源码。后续若引入 webview,扩展 `DocumentKind::Html` 分支。

### 10.6 kebab-case 强约束

* **RML 标签**:`<view-switcher>`(✅) / `<view_switcher>`(❌)

* **属性**:`on-click`(✅) / `on_click`(❌)

* **class 名**:`preview-header`(✅) / `preview_header`(❌)

* **CSS 变量**:`--surface-variant`(✅) / `--surface_variant`(❌)

### 10.7 命名规范对照

| 旧命名                             | 新命名                        | 类型                     |
| ------------------------------- | -------------------------- | ---------------------- |
| `CodeWorkbench`                 | `CodeComponent`            | IWorkbenchComponent 实现 |
| `PreviewWorkbench`(计划文档原称)      | `PreviewComponent`         | IWorkbenchComponent 实现 |
| `RmlDesignComponent`            | `RmlDesignComponent`(无变)   | IWorkbenchComponent 实现 |
| `EditorWorkbench`               | `EditorWorkbench`(无变)      | IWorkbench 实现          |
| `code_workbench.rs`             | `code_component.rml.rs`    | 文件名                    |
| `preview_workbench.rml.rs`(原计划) | `preview_component.rml.rs` | 文件名                    |

## 11. 验证清单

### Phase 1 验证(Core 契约)

* [ ] `cargo build -p rust-rml-studio-core` 编译通过

* [ ] `WorkbenchDocument::set_content` / `is_dirty` / `mark_saved` / `reload` 单元测试通过

* [ ] `WorkbenchState::set_dirty` / `set_saving` 单元测试通过

* [ ] `IWorkbenchComponentHost` trait 可被 impl,方法签名正确

* [ ] `register_workbench_component_host_ability::<T>()` 编译通过

### Phase 2 验证(EditorWorkbench 改造)

* [ ] `cargo build -p rust-rml-studio-editor` 编译通过

* [ ] `EditorWorkbench` 不再持有 `editor_state`/`language_client`

* [ ] `EditorWorkbench` impl `IWorkbenchComponentHost` 全部方法

* [ ] 打开 `.md` 文件后 `document.content` 正确,`view_names` 含 `["Code", "Preview"]`

* [ ] 打开 `.rs` 文件后 `view_names` 仅 `["Code"]`,`active_component_id = "code"`

* [ ] `IVisual::render` 内无 `div()` 等 GPUI 元素直接构造代码

* [ ] `register_workbench_component_host_ability::<EditorWorkbench>()` 已调用

### Phase 3 验证(CodeComponent)

* [ ] `code_workbench.rs` 已删除,`code_component.rml.rs` + `code_component.rml` 已新增

* [ ] `CodeComponent` 加 `#[component]` 标注,`IVisual::render` 委托 Render

* [ ] 代码编辑 + LSP 集成(补全/hover/diagnostics)不回归

* [ ] 编辑后 `document.content` 更新,`state.dirty = true`

* [ ] Tab 切换后 `CodeComponent` 显示新文件内容(经 `sync_from_document`)

* [ ] `get_workbench_components()` 含 `id="code"` 的组件

### Phase 4 验证(PreviewComponent + 数据同步)

* [ ] `PreviewComponent::matches(uri)` 对 `.md`/`.markdown`/`.html` 返回 `true`,对其余返回 `false`

* [ ] 打开 `.md` 文件 Header 显示 `Code | Preview` 切换按钮

* [ ] 点击 `Preview` 切换显示 Markdown 富文本渲染

* [ ] **数据同步**:在 `Code` 视图编辑 → 切换到 `Preview` → 看到编辑后的最新内容

* [ ] **状态保留**:在 `Code` 视图编辑(dirty=true)→ 切换到 `Preview` → Tab 标题仍显示 `●` 修改标记

* [ ] **切换不丢状态**:切换 Code ↔ Preview 多次,`InputState` 未保存内容不丢失

* [ ] PreviewComponent 视图正确渲染 HTML 源码(纯文本)

* [ ] RML 模板所有标签/属性/class 名均为 kebab-case

## 12. 风险与对策

| 风险                                                                 | 影响                                               | 对策                                             |
| ------------------------------------------------------------------ | ------------------------------------------------ | ---------------------------------------------- |
| `get_or_create_entity` 按 TypeId 缓存                                 | 多 Tab 共享 Entity,切换需 reload                       | 沿用现有 `uri_changed` 检测 + `reload()` 模式,已验证可行    |
| 组件 observe document 循环同步                                           | `input_state → document → input_state` 死循环       | `last_synced_content` 字段比对,内容相同则跳过 update      |
| `#[computed]` 无 cx 参数,无法读 Entity                                   | `dirty_mark` 无法直接读 `WorkbenchState`              | 字段缓存 + observe 回调更新(方案 A)                      |
| RML 框架暂不支持参数化模板                                                    | Body 无法 `<Component id={active_component_id} />` | 采用 `if` 条件分支兜底                                 |
| 工厂返回类型从 `Arc<dyn IContribution>` 改为 `Arc<dyn IWorkbenchComponent>` | registry.rs 破坏性变更                                | 选项 B,同步更新所有 `register_workbench_component` 调用点 |
| `.html` 文件无原生渲染                                                    | 用户预期可视化页面,实际为源码                                  | Header 显示 `Read-only` 标记 + 文档说明;后续可引入 webview  |
| `Markdown` 大文件性能                                                   | `TextView::markdown` 全量解析,无虚拟滚动                  | MVP 阶段可接受;后续评估虚拟化                              |

## 13. 与既有计划的关系

* **本计划** 落地 [arc-studio-plan.md](../../../.trae/documents/arc-studio-plan.md)
  第 4.2 节定义的 `PreviewComponent(id="preview")` + 组件间数据同步机制,
  命名调整为 `XXXComponent` 规范。

* **不涉及** `RmlDesignComponent(id="design")` —— 该组件依赖
  [rml-visual-designer-plan.md](../../../.trae/documents/rml-visual-designer-plan.md)
  的设计器内核,后续独立计划推进。本计划的 `WorkbenchDocument` 共享模型已为 design 视图预留同步通路。

* **IWorkbench trait 不变**:本计划新增 `IWorkbenchComponentHost` trait(独立于 IWorkbench),
  符合 project\_memory 硬约束:"IWorkbench super trait 仅含 IContribution + IVisual,
  Host 状态由实现决定"。

