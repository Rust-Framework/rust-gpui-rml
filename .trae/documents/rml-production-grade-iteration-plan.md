# RML 框架生产级迭代计划

## Summary（摘要）

基于前期完成的《RML 框架组件支持分析》文档与本次工程化现状调研，制定将 RML 框架推进到生产级的 6 个迭代里程碑（M1~M6），按"先补缺陷、再扩能力、后强保障"的顺序递进。

**总目标：** 在 6 个迭代周期内，将 RML 从"功能可用"提升到"生产级"，覆盖 CSS 完整性、组件覆盖度、开发体验、稳定性、性能、生态六大维度。

**核心策略：**
- 优先修复已有功能的缺陷（半集成组件、缺失映射）
- 再扩展能力广度（CSS 属性、新组件）
- 最后强化保障（测试、LSP、热重载、性能）

---

## Current State Analysis（现状分析）

### 工程化基线

| 维度 | 现状 | 主要缺口 |
|------|------|---------|
| **测试** | 697 单测 + 4 集成测试 | 无 snapshot 测试；parser/codegen 主流程缺测；cache.rs 无测 |
| **错误诊断** | ParseError 携带行号；ValidationError/CodegenError 无 Span | 无颜色化；warning 通过 `eprintln!` 输出，不可收集 |
| **LSP** | completion/hover/definition/references 已实现 | 被 workspace 排除；无 formatting/rename/code action |
| **CLI** | 无独立 `rml` 命令 | 仅靠 build.rs |
| **性能** | JSON 缓存完整（三层哈希失效） | 无 benchmark |
| **热重载** | API `Builder.hot_reload(bool)` 存在 | 实现未落地（无 file watcher/IPC/状态保留） |
| **文档** | docs/ 11 章完整 + 16 demo 案例 | API 文档未独立部署 |
| **i18n** | I18nState + catalog + t()/t_static() + 切换 | 完整可用 |
| **主题** | ThemeState + set_style/set_theme + var() 运行时查询 | 完整可用 |

### 能力基线

| 维度 | 现状 | 主要缺口 |
|------|------|---------|
| **CSS 属性覆盖** | 22/~60 常用属性（37%） | 缺 max-w/h, flex-wrap/grow/shrink/basis, border, position, transform 等 |
| **CSS 选择器** | 9/15 类型（60%） | 缺伪类、属性选择器、相邻兄弟 |
| **CSS 分层** | 仅应用层（with_style）+ 内联 | 缺页面层 `<style source>` |
| **组件集成度** | 12 个完整集成（tags.rs 注册） | 12 个半集成（已 re-export 未注册）+ 25 个未触及 |
| **半集成组件** | Dialog/Form/List/Popover/Radio/Select/Tooltip/Notification 等 | 已 re-export 但 tags.rs 未注册 |
| **未注册组件的 Demo** | 17 个组件无独立 demo | Badge/Checkbox/Label/Separator/Tag/Progress/ProgressCircle/Slider/Switch/ButtonGroup/TitleBar/NativeStatusBar/Input/Tree/AvatarGroup/Card/CodeEditor |
| **属性映射完整性** | 90% 已注册属性已映射 | Card 的 extra/cover/footer 注册但映射缺失；TabBar 的 last-empty-space/track-scroll 同上 |
| **构建系统** | Builder API 完整 | strict 字段未生效；缓存仅文件级 |

### 关键文件锚点

- CSS 映射：[crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)
- CSS 匹配：[crates/engine/src/css/matcher.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/matcher.rs)
- CSS 解析：[crates/engine/src/css/parser.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/parser.rs)
- 组件路由：[crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- 属性注册：[crates/engine/src/compiler/props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
- 组件 codegen：[crates/engine/src/compiler/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)
- 错误类型：[crates/engine/src/compiler/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L182-L228)
- 构建缓存：[crates/engine/src/build/cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/cache.rs)
- LSP 服务：[crates/lsp/src/server/connection.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/connection.rs#L135-L149)
- re-export：[crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs#L43-L89)
- 主题系统：[crates/core/src/theme.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)
- i18n 系统：[crates/core/src/i18n.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/i18n.rs)
- 应用入口：[crates/app/src/application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs)

---

## Proposed Changes（迭代规划）

### M1：基础完善（缺陷修复与已有能力补全）

**目标：** 让现有 23 个已注册组件全部达到 ⭐⭐⭐ 完整度，消除"已注册但未映射"的缺陷。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 1.1 补齐 Card 属性映射 | `crates/engine/src/compiler/card/` | 实现 `extra`/`cover`/`footer` 的 static 和 bind setter |
| 1.2 补齐 TabBar 属性映射 | `crates/engine/src/compiler/tab_bar/` | 确认 `last_empty_space`/`track_scroll` setter，缺失则补全 |
| 1.3 补齐 17 个组件独立 demo | `demo/src/cases/` | 为 Badge/Checkbox/Label/Separator/Tag/Progress/ProgressCircle/Slider/Switch/ButtonGroup/TitleBar/NativeStatusBar/Input/Tree/AvatarGroup/Card/CodeEditor 各新增一个案例 |
| 1.4 修复 h2 字号不一致 | `crates/engine/src/tags.rs` | h2 当前 `px(18)`，应为 `px(24)`（与 h3 颠倒） |
| 1.5 命名颜色扩展 | `crates/engine/src/css/parser.rs` | 从 11 种扩展至 CSS 标准 140+ 命名颜色 |
| 1.6 `flex: N` 数字支持 | `crates/engine/src/css/mapper.rs` | 当前仅支持 `flex: 1`，扩展为 `flex: <number>` → `.flex_grow(N).flex_shrink(0).flex_basis_0()` |
| 1.7 `strict` 字段生效 | `crates/engine/src/build/mod.rs` | 让 `Builder.strict(true)` 将 warning 升级为 error |
| 1.8 缺失单元测试补全 | `crates/engine/src/parser/mod.rs`、`compiler/codegen/mod.rs`、`build/cache.rs` | 为 parser 主流程、codegen 主入口、cache 增加单测 |

**验证：**
- 23 个已注册组件全部有独立 demo 且功能验证通过
- `cargo test -p rust-rml-engine` 全绿，覆盖率提升至 80%+
- `Builder.strict(true)` 在 demo 中触发 warning 时报错

**交付物：** 17 个新 demo + 修复的属性映射 + 完善的测试

---

### M2：半集成组件推进（tags.rs 注册 + codegen）

**目标：** 将 12 个"已 re-export 但未注册"的组件全部推进到完整集成，覆盖 Dialog/Form/Radio/Select/Tooltip/Popover/List/Notification 等核心交互组件。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 2.1 Tooltip 集成 | `tags.rs`、`props_registry.rs`、`component.rs`、`compiler/tooltip/` | Stateless，支持 `label`/`placement`/`trigger` 属性 |
| 2.2 Popover 集成 | `tags.rs`、`props_registry.rs`、`component.rs`、`compiler/popover/` | StatelessWithItems，支持 `trigger`/`placement`/`content` |
| 2.3 Radio + RadioGroup 集成 | 同上 | RadioGroup 为容器，Radio 为子项，支持 `value`/`disabled`/`on-change` |
| 2.4 Select 集成 | 同上 | Stateless，支持 `items`/`value`/`on-change`/`placeholder` |
| 2.5 Form + FormItem 集成 | 同上 | Form 为容器，FormItem 支持 `label`/`required`/`validate` |
| 2.6 Dialog 完整集成 | `tags.rs`、`compiler/dialog/` | rml_ui 已有 AlertDialog 封装，扩展属性映射 + `<dialog>` 根标签支持 |
| 2.7 List 集成 | `tags.rs`、`compiler/list/` | StatelessWithItems，支持 `items`/`render` |
| 2.8 Notification 集成 | `tags.rs`、`compiler/notification/` | 与 Root 集成，支持 `title`/`description`/`type`/`duration` |
| 2.9 Kbd 集成 | `tags.rs` | Stateless，简单展示组件 |
| 2.10 Icon 集成 | `tags.rs` | Stateless，支持 `name`/`size`/`color` |
| 2.11 各组件 demo 案例 | `demo/src/cases/` | 10 个新案例 |
| 2.12 组件集成回归测试 | `crates/engine/src/compiler/component.rs` | 为每个新组件增加 gen_component 测试 |

**验证：**
- 12 个新组件在 .rml 中可用且通过 demo 验证
- `cargo test -p rust-rml-engine` 全绿
- props_registry 的对齐测试 `component_props_tags_align_with_routing_table` 通过

**交付物：** 12 个完整集成组件 + 12 个 demo + 测试用例

---

### M3：CSS 三层架构与标准属性扩展

**目标：** 实现应用层/页面层/内联层三层 CSS 架构，扩展 CSS 标准属性至 60+ 覆盖率。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 3.1 `<style>` 标签解析 | `crates/engine/src/parser/`、`parser/ast.rs` | 识别 `<style>` 元素，支持 `source` 属性 + 文本子节点 |
| 3.2 CodegenCtx 持有页面样式 | `crates/engine/src/compiler/mod.rs` | 新增 `page_stylesheet: Option<StyleSheet>` 字段 |
| 3.3 多级样式表查询 | `crates/engine/src/css/matcher.rs` | `collect_matching_declarations` 先查 page 后查 global |
| 3.4 `<style source="..."/>` 文件加载 | `crates/engine/src/build/mod.rs` | 构建期读取外部 CSS，解析为 StyleSheet 注入 page_stylesheet |
| 3.5 优先级叠加规则 | `crates/engine/src/css/matcher.rs` | Layer 1 < Layer 2 < Layer 3，同层后者覆盖前者 |
| 3.6 CSS P0 属性扩展 | `crates/engine/src/css/mapper.rs` | max-width/max-height、flex-wrap、flex-grow/shrink/basis、align-self、align-content、overflow-x/y |
| 3.7 CSS P1 属性扩展 | `crates/engine/src/css/mapper.rs` | border 简写、border-width/color/style、outline、cursor、letter-spacing、font-style、font-family |
| 3.8 CSS 颜色函数完善 | `crates/engine/src/css/mapper.rs` | rgb()/rgba()/hsl()/hsla() 函数值完整内联映射 |
| 3.9 属性选择器支持 | `crates/engine/src/css/parser.rs`、`matcher.rs` | `[type="text"]`、`[disabled]` 选择器 |
| 3.10 CSS 分层 demo | `demo/src/cases/css_layering_case.rml` | 演示三层 CSS 叠加效果 |

**验证：**
- `<style source="/button.css"/>` 在 .rml 中可用且作用域正确
- CSS 属性覆盖率从 37% 提升至 70%+
- 属性选择器 `[disabled]` 匹配测试通过
- demo 演示三层叠加效果（全局样式被页面样式覆盖、被内联样式覆盖）

**交付物：** 三层 CSS 架构 + 38 个新 CSS 属性映射 + 属性选择器

---

### M4：开发体验（错误诊断 + LSP + CLI）

**目标：** 提供生产级开发体验，包含精准错误定位、LSP 全功能、独立 CLI 工具。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 4.1 错误类型携带 Span | `crates/engine/src/compiler/validator.rs`、`compiler/mod.rs` | ValidationError/CodegenError 携带 Span + line/column |
| 4.2 颜色化诊断输出 | 新增 `crates/engine/src/diagnostic.rs` | 类似 codespan-reporting 的颜色化错误显示，含上下文片段 |
| 4.3 结构化 warning 系统 | `crates/engine/src/compiler/warning.rs` | WarningCollector 收集 warning，支持降级为 error |
| 4.4 LSP 重新纳入 workspace | `Cargo.toml` | 移除 exclude，启用 LSP crate 构建 |
| 4.5 LSP formatting 支持 | `crates/lsp/src/server/` | 实现 `document_formatting_provider` |
| 4.6 LSP rename 支持 | `crates/lsp/src/server/` | 实现 rename provider |
| 4.7 LSP code action | `crates/lsp/src/server/` | 快速修复、提取组件等 |
| 4.8 LSP semantic tokens | `crates/lsp/src/server/` | 语法高亮 tokens |
| 4.9 CLI 工具 `rml` | 新增 `crates/cli/` | `rml check`/`rml build`/`rml fmt`/`rml watch` 子命令 |
| 4.10 诊断信息 i18n | `crates/engine/src/diagnostic.rs` | 错误消息支持多语言 |
| 4.11 Snapshot 测试框架 | `crates/engine/tests/` | 引入 `insta` 或 `expect_test`，为 codegen 输出建立 golden test |

**验证：**
- 错误消息显示 `error[E001]: at line 12:5 - <message>` + 上下文片段
- LSP 在 VSCode 中提供 formatting/rename/code action/semantic tokens
- `rml check` 在 CI 中运行，输出错误和 warning
- Snapshot 测试覆盖所有 codegen 路径，确保输出稳定

**交付物：** 颜色化诊断 + 完整 LSP + CLI 工具 + Snapshot 测试框架

---

### M5：生产级保障（性能 + 热重载 + 测试覆盖）

**目标：** 提供生产级性能保障、热重载落地、测试覆盖完整。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 5.1 热重载 file watcher | 新增 `crates/engine/src/watch/` | 监听 .rml/.css/.rml.rs 文件变化 |
| 5.2 热重载 IPC 通信 | `crates/engine/src/watch/ipc.rs` | 通过 Unix Socket / Named Pipe 通知运行时应用 |
| 5.3 热重载状态保留 | `crates/core/src/state.rs` | 序列化/反序列化 ViewModel 状态，重载后恢复 |
| 5.4 热重载 Render 替换 | `crates/core/src/runtime.rs` | 运行时替换 Render 实现，保留 Entity 句柄 |
| 5.5 性能 benchmark | 新增 `crates/engine/benches/` | criterion 基准测试：parse、compile、codegen、cache hit/miss |
| 5.6 增量编译优化 | `crates/engine/src/build/mod.rs` | 当 `<style>` 变化时仅重编译依赖该样式表的 .rml 文件 |
| 5.7 缓存粒度优化 | `crates/engine/src/build/cache.rs` | 从 engine_hash 整体失效改为模块级哈希失效 |
| 5.8 测试覆盖率报告 | `Cargo.toml`、CI 配置 | 引入 tarpaulin/llvm-cov，CI 输出覆盖率报告 |
| 5.9 集成测试套件 | `crates/engine/tests/` | 端到端测试：完整 .rml → 编译 → 验证生成的 Rust 代码可编译 |
| 5.10 模糊测试 | `crates/engine/fuzz/` | 对 parser/css parser 进行 fuzz testing |

**验证：**
- 修改 .rml 文件后 1 秒内看到 UI 更新（热重载）
- benchmark 显示编译 100 个 .rml 文件 < 5 秒
- 测试覆盖率 ≥ 85%
- Fuzz 测试 100 万次输入无 panic

**交付物：** 热重载系统 + benchmark 套件 + 覆盖率报告 + Fuzz 测试

---

### M6：生态扩展（新组件 + 文档 + 主题市场）

**目标：** 扩展组件生态覆盖 gpui-component 80% 能力，完善文档体系。

**范围：**

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 6.1 Phase 2 组件集成（10 个） | `tags.rs`、`compiler/` | Breadcrumb、Pagination、Stepper、Alert、Spinner、Skeleton、Link、Rating、Collapsible、HoverCard |
| 6.2 Phase 3 复杂组件集成（10 个） | 同上 | Dock、Sidebar、Combobox、Calendar、DatePicker、ColorPicker、InputNumber、Resizable、Scroll、VirtualList |
| 6.3 高级组件集成（5 个） | 同上 | Sheet、SearchableList、Chart、Setting、GroupBox |
| 6.4 主题市场 | `crates/app/src/theme_market.rs` | 内置 5+ 主题，支持运行时切换 + 用户自定义主题导入 |
| 6.5 文档站点 | 新增 `docs-site/` | 基于 mdbook 或 zola 的文档站点，部署到 GitHub Pages |
| 6.6 API 文档 | `crates/*/src/lib.rs` | 完善 rustdoc 注释，`cargo doc --open` 可读 |
| 6.7 组件 Storybook | 新增 `demo/src/storybook/` | 类似 Storybook 的组件展示页，每个组件展示所有变体/状态 |
| 6.8 高级 CSS 支持 | `crates/engine/src/css/` | `:hover`/`:focus` 伪类、`calc()` 函数、CSS 嵌套 |
| 6.9 动画系统 | `crates/engine/src/css/mapper.rs` | `transition`/`animation` 基础支持 |
| 6.10 阴影系统 | `crates/engine/src/css/mapper.rs` | `box-shadow`/`text-shadow` 支持 |

**验证：**
- 25 个新组件全部集成且有 demo
- 文档站点部署且内容完整
- Storybook 展示所有组件变体
- `:hover` 伪类在 demo 中生效
- 主题市场支持至少 5 种主题切换

**交付物：** 25 个新组件 + 文档站点 + Storybook + 高级 CSS + 动画/阴影

---

## Assumptions & Decisions（假设与决策）

### 假设

1. **团队规模：** 假设 2-3 名开发人员全职投入，每个迭代周期 4 周
2. **gpui-component 版本：** 假设保持 v0.5.2，无重大破坏性升级
3. **GPUI 上游：** 假设 GPUI 框架保持稳定，无重大 API 变化
4. **热重载范围：** 仅支持 .rml/.css 热重载，不支持 .rml.rs code-behind 热重载（需重新编译 Rust）
5. **目标平台：** Windows/macOS/Linux 三平台同等支持

### 决策

1. **迭代顺序：** M1（修复）→ M2（半集成）→ M3（CSS）→ M4（开发体验）→ M5（保障）→ M6（扩展）。先修复已有缺陷，再扩展能力，最后强化保障。
2. **CSS 分层架构：** 采用"应用层 + 页面层 + 内联层"三层模型，页面层通过 `<style source>` + 内联 `<style>` 双语法支持。
3. **组件集成优先级：** 已 re-export 的半集成组件优先（M2），gpui-component 未触及组件次之（M6）。
4. **LSP 重新纳入 workspace：** M4 中将 LSP crate 从 exclude 移除，作为正式产物维护。
5. **CLI 工具独立：** 新增 `crates/cli/` 提供独立 `rml` 命令，不依赖 build.rs。
6. **Snapshot 测试：** 采用 `insta` crate，对 codegen 输出建立 golden test，确保输出稳定。
7. **热重载范围限定：** M5 仅支持 .rml/.css 热重载，code-behind 变更仍需重新编译。
8. **主题市场：** M6 内置 5+ 主题，支持用户导入自定义主题 CSS 文件。

### 取舍

| 取舍点 | 选择 | 理由 |
|--------|------|------|
| 完整性 vs 速度 | 6 个迭代周期 | 平衡功能完整与发布节奏 |
| 热重载范围 | 仅 .rml/.css | code-behind 热重载复杂度高，收益有限 |
| CSS 标准化 | 优先 P0/P1 属性 | P2 定位/变换/动画依赖 GPUI 底层支持 |
| 组件集成深度 | 优先已 re-export 组件 | 减少封装成本，快速提升覆盖度 |
| LSP 范围 | 重新纳入 workspace | LSP 是生产级框架必备能力 |

---

## Verification（整体验证）

### 验证矩阵

| 维度 | M1 | M2 | M3 | M4 | M5 | M6 |
|------|----|----|----|----|----|----|
| 单元测试 | ✅ 补全 | ✅ 新增 | ✅ 新增 | ✅ Snapshot | ✅ 覆盖率 85% | ✅ 新增 |
| 集成测试 | — | — | — | — | ✅ 端到端 | — |
| Demo 案例 | ✅ 17 新增 | ✅ 12 新增 | ✅ 1 新增 | — | — | ✅ Storybook |
| Benchmark | — | — | — | — | ✅ 建立 | — |
| 文档 | — | — | ✅ CSS 章节 | ✅ 诊断章节 | ✅ 热重载章节 | ✅ 站点部署 |
| CI/CD | — | — | — | ✅ CLI 集成 | ✅ 覆盖率 | ✅ 文档部署 |

### 生产级验收标准

| 标准 | 目标 | 验证方式 |
|------|------|---------|
| CSS 属性覆盖率 | ≥ 70% | mapper.rs 已映射属性 / 60 常用属性 |
| 组件集成度 | ≥ 80%（gpui-component 覆盖） | tags.rs 注册数 / 49 可用组件 |
| 测试覆盖率 | ≥ 85% | tarpaulin/llvm-cov 报告 |
| 编译性能 | 100 个 .rml < 5 秒 | criterion benchmark |
| 热重载延迟 | < 1 秒 | 端到端测试 |
| 错误诊断准确性 | 100% 携带 Span | 测试用例验证 |
| LSP 功能完整性 | formatting/rename/code action | VSCode 集成测试 |
| 文档完整性 | 11 章 + API + Storybook | 文档站点部署 |

### 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| GPUI 上游 API 变化 | 高 | 锁定 gpui 版本，定期跟进 |
| gpui-component 破坏性升级 | 中 | 版本固定，升级前回归测试 |
| 热重载状态保留复杂度 | 高 | M5 限定范围，先支持简单状态 |
| LSP 重新纳入 workspace 构建问题 | 中 | M4 优先解决构建依赖 |
| CSS 伪类实现难度 | 高 | M6 优先 `:hover`/`:focus`，其余延后 |
| 团队资源不足 | 中 | 严格按优先级排序，可调整迭代范围 |

---

## 迭代节奏建议

| 迭代 | 周期 | 重点 | 可交付 |
|------|------|------|---------|
| M1 | 4 周 | 缺陷修复 + 测试补全 | 23 个完整 ⭐⭐⭐ 组件 |
| M2 | 6 周 | 半集成组件推进 | 12 个新完整集成组件 |
| M3 | 4 周 | CSS 三层架构 + 属性扩展 | 三层 CSS + 70% 覆盖率 |
| M4 | 6 周 | 开发体验 | 颜色化诊断 + 完整 LSP + CLI |
| M5 | 6 周 | 生产级保障 | 热重载 + benchmark + 85% 覆盖 |
| M6 | 8 周 | 生态扩展 | 25 个新组件 + 文档站点 + Storybook |

**总周期：** 34 周（约 8 个月）

---

## 附录：关键文件清单

### M1 涉及文件

- [crates/engine/src/compiler/card/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/card/)
- [crates/engine/src/compiler/tab_bar/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tab_bar/)
- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- [crates/engine/src/css/parser.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/parser.rs)
- [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)
- [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)
- [demo/src/cases/](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/)

### M2 涉及文件

- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- [crates/engine/src/compiler/props_registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)
- [crates/engine/src/compiler/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)
- [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)
- [crates/engine/src/compiler/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/)（新增 tooltip/popover/radio/select/form/dialog/list/notification 模块）

### M3 涉及文件

- [crates/engine/src/parser/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/)
- [crates/engine/src/parser/ast.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs)
- [crates/engine/src/compiler/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)
- [crates/engine/src/css/matcher.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/matcher.rs)
- [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)
- [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)

### M4 涉及文件

- [crates/engine/src/compiler/validator.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs)
- [crates/engine/src/compiler/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs)
- [crates/lsp/src/server/connection.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/lsp/src/server/connection.rs)
- [Cargo.toml](file:///e:/GitCode/RF/rust-gpui-rml/Cargo.toml)（移除 lsp exclude）
- 新增 `crates/cli/`
- 新增 `crates/engine/src/diagnostic.rs`
- 新增 `crates/engine/src/compiler/warning.rs`

### M5 涉及文件

- 新增 `crates/engine/src/watch/`
- 新增 `crates/engine/benches/`
- [crates/engine/src/build/cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/cache.rs)
- [crates/core/src/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/)（新增 state.rs/runtime.rs）

### M6 涉及文件

- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)
- [crates/engine/src/compiler/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/)（新增 25 个组件模块）
- [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)
- 新增 `docs-site/`
- 新增 `demo/src/storybook/`
