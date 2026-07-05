# RML 框架生产级提升规划（语法 / 组件 / 运行时）

## Summary（摘要）

基于对 RML 仓库语法层、组件层、运行时层的深度调研，制定将三大核心维度推进到生产水平的 6 个里程碑（M1~M6），按"先修缺陷、再补能力、后强运行时"的顺序递进。

**总目标**：在 30 周内（约 7.5 个月）将 RML 的语法、组件、运行时三大核心维度提升到生产水平。

**核心策略**：
- M1-M2：修复已实现但残缺的能力（4 个 codegen 未消费指令、show 语义、ref 注入、style 内联）+ 半集成组件补全
- M3-M4：CSS 三层架构 + 伪类 + 动画 + 关键缺失能力
- M5：运行时关键能力（on_prop_change、checkbox/select 状态、多窗口、循环防护）
- M6：热重载（P1 优先级，独立阶段）

**范围边界**：
- ✅ 纳入：缺陷修复、半集成组件、CSS 三层 + 伪类 + 动画、on_prop_change、checkbox/select 状态、多窗口、热重载
- ❌ 不纳入：MultiBinding、PriorityBinding、RoutedCommand、InputGesture、ControlTemplate、可视化设计器、LSP 扩展、CLI 工具、文档站点

**项目记忆约束遵守**：
- Phase C 已拒绝 → 不新增宏；on_prop_change 通过 `#[component]` 宏的属性参数扩展实现，不新增 `#[on_prop_change]` 宏
- 所有属性/标签名强制 kebab-case
- IContribution / IVisualContribution trait 签名不可修改
- DescriptionList 沿用 `<descriptions items={desitems} ...>` 语法

---

## Current State Analysis（现状分析）

### 语法层现状

| 子系统 | 已实现 | 关键缺陷 |
|--------|--------|----------|
| **解析器** | 标签/属性/指令/插值/注释/kebab-case 强制 | CSS ParseError 无 line/col；RML ParseError 无 source snippet；无错误恢复；无增量解析 |
| **AST** | 9 个指令 + 4 类属性 + 4 类节点 | Attribute 节点未保留 Span（LSP 跳转受限） |
| **CSS 映射** | 22 个属性（37% 覆盖）+ 7 类选择器 + `:root` 变量 + 颜色 var() 运行时 | 缺 max-w/h、flex-wrap/grow/shrink/basis、border 简写、position、calc()、伪类、动画、CSS Grid、局部变量作用域 |
| **CSS 三层架构** | L1 全局样式表 + L2 class 属性 | L3 内联 `style="..."` 未实现 |
| **指令 codegen** | if / each / model / ref(部分) / slot | **else / once / html / key 四个指令已解析但 codegen 完全不消费**；show 语义错误（等同 if）；ref 仅生成 .id() 未注入 ElementRef |

**最大风险**：用户写了 `else`/`once`/`html`/`key` 不会报错也不会生效——这是生产级应用的不可接受缺陷。

### 组件层现状

| 类别 | 数量 | 清单 |
|------|------|------|
| **A. 完整集成** | 35 个 | Button, ButtonGroup, Checkbox, Label, Separator, Tag, Progress, ProgressCircle, Slider, Switch, Badge, TitleBar, NativeStatusBar, Avatar, AvatarGroup, Card, Accordion, TabBar, Table, DescriptionList, Input, TextInput, CodeEditor, Tree, ActivityBar, MenuBar, ContextMenu, DropdownMenu, AppMenuBar, MenuItem, MenuSeparator, Column, Tab, TabItem, AccordionItem, DescriptionItem, DescriptionSeparator |
| **B. 半集成** | 12 个 | Form, Kbd, List, Popover, Radio, Select, Tooltip, Notification(族), AlertDialog(族), Dialog(仅根节点), Icon/IconName |
| **C. 文档/实现不一致** | 1 项 | StatusBar（PascalCase）/status_bar（snake_case）reference 有文档但 `component_lookup` 未注册 |

**Demo 缺口**：Badge, ButtonGroup, Card, Checkbox, CodeEditor, Label, Progress, ProgressCircle, Separator, Slider, Switch, Tag, TitleBar, ActivityBar, NativeStatusBar, Tree, Input/TextInput, MenuBar/AppMenuBar 共 17 个组件无独立 demo。

### 运行时层现状

| 子系统 | 已实现 | 关键缺口 |
|--------|--------|----------|
| **绑定引擎** | 单/双向、版本号、Computed、Converter 管道 | 订阅图、循环防护、MultiBinding（不纳入） |
| **命令系统** | ICommand/RelayCommand、`command={field}`、debounce | throttle（不纳入宏，文档方式提供） |
| **生命周期** | on_loaded/on_unloaded、应用级 on_launch/exit | **on_prop_change（文档承诺但未实现）**、on_rendered、统一任务注册表 |
| **校验系统** | IValidate、规则式 codegen、field_errors 暴露 | 异步校验（不纳入） |
| **状态管理** | RmlState 单字段收敛、惰性 input_states | **仅文本 InputState**，无 checkbox/radio/select 状态 |
| **主题/i18n** | CSS `:root` 颜色、JSON catalog、静态快照 | 非颜色变量、ICU 复数（不纳入）、**切换防抖** |
| **应用框架** | RmlApplication builder、IWindow 默认实现 | **多窗口管理器**、关闭确认 |
| **热重载** | 仅构建期缓存（三层哈希） | **进程内热重载完全未实现** |

### 关键文件锚点

- 解析器：`crates/engine/src/parser/{mod.rs, tokenizer.rs, ast.rs, span.rs}`
- CSS：`crates/engine/src/css/{mapper.rs, matcher.rs, parser.rs, ast.rs}`
- 指令 codegen：`crates/engine/src/compiler/codegen/node.rs`
- 组件路由：`crates/engine/src/tags.rs`
- 属性注册：`crates/engine/src/compiler/props_registry.rs`
- 组件 codegen 入口：`crates/engine/src/compiler/component.rs`
- 组件子模块：`crates/engine/src/compiler/{accordion,avatar,card,code_editor,description_list,input,menu,tab_bar,table,tree}/`
- 绑定引擎：`crates/core/src/{binding.rs, two_way_binding.rs, computed_cache.rs, converter/}`
- 命令系统：`crates/core/src/command.rs`、`crates/macros/src/command.rs`
- 生命周期：`crates/core/src/lifecycle.rs`、`crates/app/src/lifecycle.rs`
- 校验：`crates/core/src/validate.rs`
- 状态容器：`crates/ui/src/state.rs`
- 主题/i18n：`crates/core/src/{theme.rs, i18n.rs}`
- 应用框架：`crates/app/src/{application.rs, lifecycle.rs}`
- 窗口：`crates/core/src/window.rs`、`crates/ui/src/window/`
- 构建流程：`crates/engine/src/build/{mod.rs, cache.rs}`
- ui re-export：`crates/ui/src/lib.rs`、`crates/ui/src/components/mod.rs`

---

## Proposed Changes（迭代规划）

### M1：语法层缺陷修复（4 周）

**目标**：消除"已解析但 codegen 未消费"的指令缺陷，修正 show 语义，实现 style 内联，修复 ref 注入。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 1.1 实现 `else` 指令 codegen | `crates/engine/src/compiler/codegen/node.rs` | 在 if 分支后追踪同父的 else 节点，生成 `if cond { elem } else { else_elem }`；不支持 `else if`（文档已声明） |
| 1.2 实现 `once` 指令 codegen | `crates/engine/src/compiler/codegen/node.rs` + `crates/ui/src/state.rs` | 在 RmlState 增加 `rendered_once: HashSet<&'static str>`；codegen 生成 `if !cx.once_rendered("key") { elem }` 并在首次渲染后标记 |
| 1.3 实现 `html` 指令 codegen | `crates/engine/src/compiler/codegen/node.rs` | 生成 `gpui::div().child_html(raw)` 或降级为 `Label::new(raw.to_string())`（GPUI 无原生 HTML 渲染，先降级） |
| 1.4 实现 `key` 指令消费 | `crates/engine/src/compiler/codegen/node.rs` | 列表渲染时为每个项生成稳定 element_id（基于 key 而非数组索引），与 GPUI element ID intern 协作 |
| 1.5 修正 `show` 语义 | `crates/engine/src/compiler/codegen/node.rs` | 改为生成 `.when(cond, \|d\| d).when(!cond, \|d\| d.hidden())` 或 `display: none` 内联样式，与 if 区分 |
| 1.6 修复 `ref` ElementRef 注入 | `crates/engine/src/compiler/codegen/node.rs` + `crates/macros/src/component.rs` | codegen 在生成 `.id("rml_ref:name")` 后，额外生成 `self.__rml_state.ref_handles.insert("name", handle)` 调用；宏侧 `#[element]` 字段从 ref_handles 取值 |
| 1.7 实现 `style="..."` 内联属性 | `crates/engine/src/css/mapper.rs` + `crates/engine/src/compiler/codegen/attribute.rs` | 新增 `apply_inline_style(style_str) -> Vec<StyledMethod>`，在 codegen 中插入到元素构建链；支持所有已映射 CSS 属性 |
| 1.8 修复 StatusBar 路由不一致 | `crates/engine/src/tags.rs` | 在 `component_lookup` 注册 `StatusBar`/`status-bar` 路由，或修正文档移除该 reference |
| 1.9 解析器错误诊断增强 | `crates/engine/src/parser/{mod.rs, tokenizer.rs}` + `crates/engine/src/css/parser.rs` | RML ParseError 增加 `source_snippet` + `expected/actual`；CSS ParseError 增加 line/column（基于 pos 计算） |
| 1.10 Attribute Span 保留 | `crates/engine/src/parser/ast.rs` + `crates/engine/src/compiler/codegen/node.rs` | `Attribute::Static/Bind/Event` 增加 `span: Span` 字段，为后续 LSP 跳转预留 |

**验证**：
- `else`/`once`/`html`/`key` 四个指令在 demo 中可用且行为符合文档
- `show` 与 `if` 行为可区分（show 隐藏后保留布局空间，if 不保留）
- `ref="input1"` 后 `self.input1.focus(cx)` 可调用
- `style="color: red; padding: 10px;"` 在 .rml 中生效
- 错误消息显示 `error at line 12:5: expected '}' but found 'EOF'` + 上下文片段

**交付物**：4 个指令 codegen 实现 + show 语义修正 + ref 注入 + style 内联 + 错误诊断增强

---

### M2：半集成组件补全（6 周）

**目标**：将 12 个"已 re-export 但未注册"的组件全部推进到完整集成，覆盖核心交互组件。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 2.1 Tooltip 集成 | `tags.rs`、`props_registry.rs`、`component.rs`、新增 `compiler/tooltip/` | Stateless，支持 `label`/`placement`/`trigger` 属性 |
| 2.2 Popover 集成 | 同上 + 新增 `compiler/popover/` | StatelessWithItems，支持 `trigger`/`placement`/`content` |
| 2.3 Radio + RadioGroup 集成 | 同上 + 新增 `compiler/radio/` | RadioGroup 容器 + Radio 子项，支持 `value`/`disabled`/`on-change` |
| 2.4 Select 集成 | 同上 + 新增 `compiler/select/` | Stateless，支持 `items`/`value`/`on-change`/`placeholder` |
| 2.5 Form + FormItem 集成 | 同上 + 新增 `compiler/form/` | Form 为容器，FormItem 支持 `label`/`required`/`validate` |
| 2.6 Dialog 完整集成 | `tags.rs` + 新增 `compiler/dialog/` | 扩展属性映射 + `<dialog>` 根标签支持完整属性（title/open/on-close） |
| 2.7 AlertDialog 集成 | 同上 + 新增 `compiler/alert_dialog/` | 复用 Dialog 机制，增加 `cancel-text`/`confirm-text`/`on-confirm` |
| 2.8 List 集成 | 同上 + 新增 `compiler/list/` | StatelessWithItems，支持 `items`/`render`/`on-select` |
| 2.9 Notification 集成 | 同上 + 新增 `compiler/notification/` | 与 Root 集成，支持 `title`/`description`/`type`/`duration` |
| 2.10 Kbd 集成 | `tags.rs`、`props_registry.rs` | Stateless，简单展示组件，支持 `key`/`size` |
| 2.11 Icon 集成 | 同上 | Stateless，支持 `name`/`size`/`color` |
| 2.12 修复 StatusBar 路由 | `tags.rs` | 在 `component_lookup` 注册 StatusBar 路由 |
| 2.13 各组件 demo 案例 | `demo/src/cases/` | 11 个新案例（tooltip/popover/radio/select/form/dialog/alert_dialog/list/notification/kbd/icon） |
| 2.14 组件集成回归测试 | `crates/engine/src/compiler/component.rs` | 为每个新组件增加 `gen_component` 测试 + props_registry 对齐测试 |
| 2.15 补齐 17 个已注册组件 demo | `demo/src/cases/` | 为 Badge/ButtonGroup/Card/Checkbox/CodeEditor/Label/Progress/ProgressCircle/Separator/Slider/Switch/Tag/TitleBar/ActivityBar/NativeStatusBar/Tree/Input/MenuBar 各新增 demo |

**验证**：
- 12 个新组件在 .rml 中可用且通过 demo 验证
- `cargo test -p rust-rml-engine` 全绿
- `props_registry` 的对齐测试 `component_props_tags_align_with_routing_table` 通过
- 所有已注册组件都有独立 demo

**交付物**：12 个完整集成组件 + 11 个新 demo + 17 个补齐 demo + 测试用例

---

### M3：CSS 三层架构 + 关键能力（5 周）

**目标**：实现应用层/页面层/内联层三层 CSS 架构，扩展 CSS 标准属性至 70% 覆盖率，补齐 calc() 与属性选择器。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 3.1 `<style>` 标签解析 | `crates/engine/src/parser/`、`parser/ast.rs` | 识别 `<style>` 元素，支持 `source` 属性 + 文本子节点 |
| 3.2 CodegenCtx 持有页面样式 | `crates/engine/src/compiler/mod.rs` | 新增 `page_stylesheet: Option<StyleSheet>` 字段 |
| 3.3 多级样式表查询 | `crates/engine/src/css/matcher.rs` | `collect_matching_declarations` 先查 page 后查 global |
| 3.4 `<style source="..."/>` 文件加载 | `crates/engine/src/build/mod.rs` | 构建期读取外部 CSS，解析为 StyleSheet 注入 page_stylesheet |
| 3.5 优先级叠加规则 | `crates/engine/src/css/matcher.rs` | Layer 1 < Layer 2 < Layer 3，同层后者覆盖前者 |
| 3.6 CSS P0 属性扩展 | `crates/engine/src/css/mapper.rs` | max-width/max-height、flex-wrap、flex-grow/shrink/basis、align-self、align-content、overflow-x/y |
| 3.7 CSS P1 属性扩展 | `crates/engine/src/css/mapper.rs` | border 简写、border-width/color/style、outline、cursor、letter-spacing、font-style、font-family、position/top/right/bottom/left/z-index |
| 3.8 CSS 颜色函数完善 | `crates/engine/src/css/mapper.rs` | rgb()/rgba()/hsl()/hsla() 函数值完整内联映射（当前 AST 已建模 Value::Function，mapper 未消费） |
| 3.9 属性选择器支持 | `crates/engine/src/css/parser.rs`、`matcher.rs` | `[type="text"]`、`[disabled]` 选择器 |
| 3.10 calc() 函数支持 | `crates/engine/src/css/mapper.rs` | 解析 `calc(100% - 20px)` 等表达式，编译期求值或生成 GPUI 表达式 |
| 3.11 linear-gradient/radial-gradient | `crates/engine/src/css/mapper.rs` | 消费 Value::Function，映射到 GPUI 渐变 |
| 3.12 CSS 分层 demo | `demo/src/cases/css_layering_case.rml` | 演示三层 CSS 叠加效果 + calc + 属性选择器 |
| 3.13 局部 CSS 变量作用域 | `crates/engine/src/css/parser.rs`、`matcher.rs` | 支持 `.card { --color: red }` 局部变量，子元素 var() 解析时优先查局部 |

**验证**：
- `<style source="/button.css"/>` 在 .rml 中可用且作用域正确
- CSS 属性覆盖率从 37% 提升至 70%+
- 属性选择器 `[disabled]` 匹配测试通过
- `calc(100% - 20px)` 在 .rml 中生效
- demo 演示三层叠加效果（全局样式被页面样式覆盖、被内联样式覆盖）

**交付物**：三层 CSS 架构 + 38 个新 CSS 属性映射 + 属性选择器 + calc + 渐变 + 局部变量

---

### M4：CSS 伪类 + 动画（4 周）

**目标**：实现 `:hover`/`:focus`/`:active` 伪类支持，提供 transition/animation 基础能力。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 4.1 伪类 CSS 解析 | `crates/engine/src/css/{parser.rs, ast.rs}` | 扩展 Selector 支持 `:hover`/`:focus`/`:active` 伪类节点 |
| 4.2 伪类匹配器 | `crates/engine/src/css/matcher.rs` | 区分基础声明与伪类声明，伪类声明收集到 `pseudo_declarations` |
| 4.3 伪类事件挂接 | `crates/engine/src/compiler/codegen/node.rs` | codegen 为匹配伪类规则的元素生成 `.on_hover`/`.on_focus`/`.on_active` 事件监听，运行时切换样式 |
| 4.4 伪类运行时状态 | `crates/ui/src/state.rs` | RmlState 增加 `hovered_elements: HashSet<ElementId>`、`focused_elements` 临时状态（不入持久状态） |
| 4.5 transition 基础支持 | `crates/engine/src/css/mapper.rs` + `crates/engine/src/compiler/codegen/node.rs` | 解析 `transition: property duration timing-function`，映射到 GPUI Animation 系统 |
| 4.6 @keyframes 解析 | `crates/engine/src/css/parser.rs`、`ast.rs` | 新增 AtRule::Keyframes 节点，解析 `@keyframes name { from {} to {} }` |
| 4.7 animation 基础支持 | `crates/engine/src/css/mapper.rs` + `compiler/codegen/node.rs` | 解析 `animation: name duration timing-function iteration-count`，映射到 GPUI Animation |
| 4.8 伪类与动画 demo | `demo/src/cases/css_pseudo_animation_case.rml` | 演示 :hover 颜色过渡 + @keyframes 入场动画 |

**验证**：
- `:hover` 在 demo 中生效（鼠标悬停切换样式）
- `:focus` 在 input 上生效（聚焦时边框变色）
- `transition: background-color 0.3s` 产生平滑过渡
- `@keyframes fadeIn { from { opacity: 0 } to { opacity: 1 } }` + `animation: fadeIn 0.5s` 生效

**交付物**：3 个伪类支持 + transition + @keyframes + animation 基础

---

### M5：运行时关键能力（5 周）

**目标**：补齐生产应用必需的运行时能力——on_prop_change、checkbox/select 状态、多窗口管理、循环防护、切换防抖。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 5.1 on_prop_change 生命周期 | `crates/core/src/lifecycle.rs` + `crates/macros/src/component.rs` + `crates/ui/src/state.rs` | 在 ILifecycle trait 增加 `on_prop_change(&mut self, field: &str, cx: &mut Context<Self>)` 默认空实现；`#[component]` 宏扫描 `fn on_prop_change_*` 方法（模式匹配），在 `bump_version` 后注入调用；不新增 `#[on_prop_change]` 宏（Phase C 约束） |
| 5.2 checkbox 状态管理 | `crates/ui/src/state.rs` + `crates/engine/src/compiler/checkbox/` | RmlState 增加 `checkbox_states: HashMap<String, bool>`；`<checkbox model={field}>` codegen 生成双向绑定路径 |
| 5.3 radio 状态管理 | 同上 + `crates/engine/src/compiler/radio/` | RadioGroup 持有 `selected_value: SharedString`；`<radio model={field} value={v}>` codegen 生成 group 内互斥逻辑 |
| 5.4 select 状态管理 | 同上 + `crates/engine/src/compiler/select/` | RmlState 增加 `select_states: HashMap<String, SharedString>`；`<select model={field}>` 双向绑定 |
| 5.5 多窗口管理器 | `crates/app/src/window_manager.rs` + `crates/app/src/application.rs` | 新增 `WindowManager` 持有 `HashMap<WindowId, WindowHandle>`；RmlApplication 增加 `.window::<W>()` 方法注册次要窗口；提供 `open_window`/`close_window`/`get_window` API |
| 5.6 窗口关闭确认生命周期 | `crates/core/src/window.rs` + `crates/app/src/window_manager.rs` | IWindow trait 增加 `on_closing(&mut self) -> bool` 默认 true；WindowManager 在关闭前调用，返回 false 取消关闭 |
| 5.7 绑定循环防护 | `crates/ui/src/state.rs` | RmlState 增加 `binding_stack: Vec<String>`；`bump_version` 时检测循环（同字段在栈中已存在则跳过 + warning） |
| 5.8 主题/i18n 切换防抖 | `crates/core/src/{theme.rs, i18n.rs}` | 切换方法增加 100ms 防抖（基于 GPUI Timer），避免连续切换触发多次 refresh_windows |
| 5.9 非颜色 CSS 变量支持 | `crates/core/src/theme.rs` + `crates/engine/src/css/mapper.rs` | ThemeState 支持 `--spacing: 8px`/`--font-size: 14px` 等非颜色变量；`var()` 解析时按类型分发（颜色/长度/无单位） |
| 5.10 on_rendered 生命周期 | `crates/core/src/lifecycle.rs` + `crates/macros/src/component.rs` | ILifecycle 增加 `on_rendered(&mut self, cx: &mut Context<Self>)` 默认空；宏检测 `fn on_rendered` 方法名并在 render 后注入调用 |
| 5.11 运行时能力 demo | `demo/src/cases/` | on_prop_change_case、multi_window_case、checkbox_radio_select_case |

**验证**：
- `fn on_prop_change_count(&mut self, cx)` 在 `count` 字段变化时被调用
- `<checkbox model={agree}>` 双向绑定生效
- `<select model={category}>` 选择后 ViewModel 字段更新
- 多窗口应用可打开/关闭次要窗口
- 窗口关闭前可拦截（on_closing 返回 false）
- 连续切换主题 10 次只触发 1 次 refresh_windows

**交付物**：on_prop_change + 3 类组件状态 + 多窗口管理 + 循环防护 + 防抖 + 非颜色变量 + on_rendered

---

### M6：热重载（P1，6 周）

**目标**：实现 .rml/.css 模板热重载，保留 ViewModel 状态，秒级反馈。

**范围**：

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 6.1 file watcher 模块 | 新增 `crates/engine/src/watch/` | 基于 `notify` crate 监听 `src/**/*.rml` 与 `**/*.rmlcss` 文件变化 |
| 6.2 增量编译触发 | `crates/engine/src/watch/compiler.rs` | 文件变化时调用 `compile()` 单文件重编译，输出到内存缓冲 |
| 6.3 IPC 通信层 | 新增 `crates/engine/src/watch/ipc.rs` | 通过 Unix Socket（macOS/Linux）/ Named Pipe（Windows）通知运行中应用 |
| 6.4 运行时 Render 替换 | 新增 `crates/core/src/runtime.rs` | 应用侧监听 IPC，收到通知后替换当前 View 的 Render 实现，保留 Entity 句柄 |
| 6.5 状态保留机制 | `crates/ui/src/state.rs` | 热重载时保留 RmlState 完整字段（field_versions/field_errors/input_states/computed_cache 全部保留） |
| 6.6 热重载错误处理 | `crates/engine/src/watch/` + `crates/core/src/runtime.rs` | 热重载失败时不崩溃，保持上一个有效状态，窗口角落显示错误提示 |
| 6.7 Builder.hot_reload 启用 | `crates/engine/src/build/mod.rs` | `Builder.hot_reload(true)` 真正生效，启动 watch 线程 |
| 6.8 Cargo feature | `crates/engine/Cargo.toml` + `crates/core/Cargo.toml` | 新增 `hot-reload` feature，仅 dev profile 启用 |
| 6.9 热重载 demo | `demo/src/cases/hot_reload_demo.rml` | 演示修改 .rml 后 UI 实时更新 |
| 6.10 热重载集成测试 | `crates/engine/tests/hot_reload.rs` | 端到端测试：修改 .rml → 验证 UI 更新 |

**验证**：
- 修改 .rml 文件后 1 秒内看到 UI 更新
- ViewModel 状态在热重载后保留（如已填写的表单字段）
- 热重载失败时不崩溃，显示错误提示
- `Builder.hot_reload(true)` 启用后 watch 线程运行

**交付物**：完整热重载系统 + demo + 集成测试

---

## Assumptions & Decisions（假设与决策）

### 假设

1. **团队规模**：假设 2-3 名开发人员全职投入，每个迭代周期 4-6 周
2. **gpui-component 版本**：假设保持 v0.5.2，无重大破坏性升级
3. **GPUI 上游**：假设 GPUI 框架保持稳定，无重大 API 变化
4. **热重载范围**：仅支持 .rml/.css 热重载，不支持 .rml.rs code-behind 热重载（需重新编译 Rust）
5. **伪类实现方式**：通过事件监听 + 运行时样式切换实现，不依赖 GPUI 原生伪类支持
6. **动画实现方式**：基于 GPUI Animation 系统封装，不实现完整的 CSS 动画时间线

### 决策

1. **on_prop_change 实现方式**：通过 `#[component]` 宏扫描 `fn on_prop_change_*` 方法名模式实现，不新增 `#[on_prop_change]` 宏（Phase C 约束）。宏识别 `fn on_prop_change_<field_name>(&mut self, cx: &mut Context<Self>)` 模式，在 `bump_version("<field_name>")` 后注入调用。

2. **show 语义修正**：改为生成 `.when(!cond, |d| d.hidden())`，保留布局空间但隐藏视觉。与 `if`（不渲染）明确区分。

3. **once 指令实现**：基于 RmlState 的 `rendered_once: HashSet<&'static str>`，key 由 codegen 基于元素路径生成。

4. **html 指令降级方案**：GPUI 无原生 HTML 渲染能力，先降级为 `Label::new(raw.to_string())`，未来 GPUI 支持后再升级。

5. **key 指令消费方式**：用于生成稳定 element_id（基于 key 哈希而非数组索引），与 GPUI element ID intern 协作，优化列表重渲染。

6. **伪类实现方式**：CSS 解析层识别伪类；codegen 为匹配元素生成事件监听 + 运行时样式切换。不引入 CSS 引擎级伪类状态机。

7. **多窗口管理器设计**：`WindowManager` 持有 `HashMap<WindowId, WindowHandle>`，与 RmlApplication 解耦。主窗口外其他窗口通过 `.window::<W>()` 注册，运行时 `open_window::<W>()` 打开。

8. **热重载 IPC 选型**：跨平台使用 `interprocess` crate（Unix Socket + Named Pipe 统一抽象），避免平台条件编译。

9. **循环防护策略**：基于 `binding_stack` 检测，发现循环时跳过 bump_version + 输出 warning。不做 WPF 式的精确回环检测（复杂度高）。

10. **非颜色 CSS 变量**：ThemeState 扩展 `non_color_vars: HashMap<String, String>`，`var()` 解析时按变量类型分发（颜色走 Rgba 路径，长度走 px 路径，其他走字符串内联）。

### 取舍

| 取舍点 | 选择 | 理由 |
|--------|------|------|
| MultiBinding/PriorityBinding | 不纳入 | 复杂度高，使用场景少，可后续按需补 |
| RoutedCommand/InputGesture | 不纳入 | 命令路由与 GPUI 事件模型差异大 |
| ControlTemplate | 不纳入 | 需重新设计模板系统，工作量超本次范围 |
| 可视化设计器 | 不纳入 | 独立项目，非框架核心 |
| LSP 扩展 | 不纳入 | 已有基础 LSP，扩展独立规划 |
| 热重载范围 | 仅 .rml/.css | code-behind 热重载复杂度高，收益有限 |
| 伪类实现 | 事件监听 + 运行时切换 | 不依赖 GPUI 原生支持，可控性强 |
| 动画实现 | 基于 GPUI Animation | 不重新实现 CSS 动画时间线 |
| on_prop_change | 宏扫描方法名模式 | Phase C 约束下不新增宏 |

---

## Verification（整体验证）

### 验证矩阵

| 维度 | M1 | M2 | M3 | M4 | M5 | M6 |
|------|----|----|----|----|----|----|
| 单元测试 | ✅ 指令测试 | ✅ 组件测试 | ✅ CSS 测试 | ✅ 伪类测试 | ✅ 生命周期测试 | ✅ 热重载测试 |
| 集成测试 | — | ✅ 端到端 | — | — | ✅ 多窗口 | ✅ 热重载 E2E |
| Demo 案例 | ✅ 指令 demo | ✅ 28 个新 demo | ✅ CSS demo | ✅ 伪类动画 demo | ✅ 运行时 demo | ✅ 热重载 demo |
| Benchmark | — | — | — | — | — | ✅ 编译性能 |

### 生产级验收标准

| 标准 | 目标 | 验证方式 |
|------|------|---------|
| 指令 codegen 完整度 | 9/9 指令全部生效 | 测试用例验证每个指令行为 |
| 组件集成度 | 47 个完整集成（35+12） | tags.rs 注册数 / 47 可用组件 |
| CSS 属性覆盖率 | ≥ 70% | mapper.rs 已映射属性 / 60 常用属性 |
| CSS 三层架构 | 应用层/页面层/内联层 | demo 演示三层叠加 |
| 伪类支持 | :hover/:focus/:active | demo 验证 |
| 动画支持 | transition + @keyframes | demo 验证 |
| 生命周期完整度 | on_loaded/on_unloaded/on_prop_change/on_rendered | 测试用例 |
| 多窗口管理 | open/close/get_window | 集成测试 |
| 热重载延迟 | < 1 秒 | 端到端测试 |
| 状态保留 | ViewModel 状态不丢失 | 热重载后验证 |
| 错误诊断 | 100% 携带 Span + line/col | 测试用例 |

### 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| GPUI Animation API 限制 | 中 | M4 动画实现前先验证 GPUI Animation 能力，必要时降级 |
| 伪类事件监听性能 | 中 | 仅对匹配伪类规则的元素挂接事件，避免全量监听 |
| 热重载状态保留复杂度 | 高 | M6 限定范围，先支持简单状态，复杂状态（如异步任务）不保留 |
| 多窗口管理器与 GPUI 窗口模型差异 | 中 | M5 初期先调研 GPUI 多窗口能力，必要时简化 API |
| on_prop_change 宏扫描误识别 | 低 | 严格匹配 `fn on_prop_change_<field>` 模式，field 必须是已声明字段 |
| 非颜色 CSS 变量类型推断 | 中 | 类型推断失败时降级为字符串内联 |

---

## 迭代节奏建议

| 迭代 | 周期 | 重点 | 可交付 |
|------|------|------|---------|
| M1 | 4 周 | 语法缺陷修复 | 9 个指令全部生效 + style 内联 + 错误诊断 |
| M2 | 6 周 | 半集成组件补全 | 12 个新完整集成组件 + 28 个 demo |
| M3 | 5 周 | CSS 三层架构 + 关键能力 | 三层 CSS + 70% 覆盖率 + calc + 属性选择器 |
| M4 | 4 周 | CSS 伪类 + 动画 | 3 个伪类 + transition + @keyframes |
| M5 | 5 周 | 运行时关键能力 | on_prop_change + 多窗口 + 循环防护 + 防抖 |
| M6 | 6 周 | 热重载 | .rml/.css 热重载 + 状态保留 |

**总周期：** 30 周（约 7.5 个月）

---

## 附录：关键文件清单

### M1 涉及文件
- [crates/engine/src/compiler/codegen/node.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs)
- [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)
- [crates/engine/src/parser/{mod.rs, tokenizer.rs, ast.rs}](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/)
- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- [crates/ui/src/state.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs)
- [crates/macros/src/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs)

### M2 涉及文件
- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- [crates/engine/src/compiler/props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
- [crates/engine/src/compiler/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)
- [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)
- [crates/engine/src/compiler/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/)（新增 tooltip/popover/radio/select/form/dialog/alert_dialog/list/notification 模块）
- [demo/src/cases/](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/)

### M3 涉及文件
- [crates/engine/src/parser/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/)
- [crates/engine/src/compiler/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)
- [crates/engine/src/css/{matcher.rs, mapper.rs, parser.rs, ast.rs}](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/)
- [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)

### M4 涉及文件
- [crates/engine/src/css/{parser.rs, ast.rs, matcher.rs, mapper.rs}](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/)
- [crates/engine/src/compiler/codegen/node.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs)
- [crates/ui/src/state.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs)

### M5 涉及文件
- [crates/core/src/lifecycle.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lifecycle.rs)
- [crates/macros/src/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs)
- [crates/ui/src/state.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs)
- 新增 `crates/app/src/window_manager.rs`
- [crates/app/src/application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs)
- [crates/core/src/{window.rs, theme.rs, i18n.rs}](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/)
- [crates/engine/src/compiler/{checkbox, radio, select}/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/)

### M6 涉及文件
- 新增 `crates/engine/src/watch/`
- 新增 `crates/core/src/runtime.rs`
- [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)
- [crates/ui/src/state.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs)
- [crates/engine/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/Cargo.toml)
- [crates/core/Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/Cargo.toml)
