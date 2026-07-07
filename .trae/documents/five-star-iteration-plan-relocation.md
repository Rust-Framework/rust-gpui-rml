# 五星迭代计划迁移与重写计划

## Summary（摘要）

之前错误地将 RML 框架五星迭代计划写入了 `docs/iteration-plans/`，但 `docs/` 是面向最终开发者（用户）的章节式技术文档树（已有 01-overview 至 11-cookbook 共 11 章正式结构 + INDEX.md 总目录）。框架内部迭代计划应放入 `.trae/documents/`——这是项目既定约定，已有 100+ 份内部规划文档（如 `rml-iteration-plan-v2.md`、`rml-production-grade-iteration-plan.md`、`rml-iteration-architecture-analysis.md` 等）。

本计划：① 清理 `docs/iteration-plans/` 下的错误内容；② 在 `.trae/documents/` 下按既有命名与格式约定重写五星迭代计划。

---

## Current State Analysis（现状分析）

### 错误产物

上一轮对话在 `docs/iteration-plans/` 下创建了两个文件：

- [docs/iteration-plans/five-star-roadmap.md](../../docs/iteration-plans/five-star-roadmap.md)（约 1040 行）
- [docs/iteration-plans/INDEX.md](../../docs/iteration-plans/INDEX.md)（35 行）

这两个文件：
- 不属于 [docs/INDEX.md](../../docs/INDEX.md) 中规划的 11 章结构
- 与 docs 树"面向用户技术指导"的定位冲突
- INDEX.md 文件在 docs/ 下也是冗余（docs 已有顶层 INDEX.md 总目录，不缺子目录索引）

### 既有约定（Phase 1 探索结论）

**`.trae/documents/` 目录的命名与格式约定**（基于现有 100+ 份文档归纳）：

| 维度 | 约定 |
|---|---|
| 文件名 | kebab-case，常带 `rml-` 前缀，描述性名称，可带版本后缀（如 `-v2`） |
| 标题 | `# RML 框架...计划` 或 `# RML 框架...分析` |
| 头部元信息 | `> 视角：...` / `> 主题：...` / `> 基线：...` 三行块（见 `rml-iteration-architecture-analysis.md`） |
| 代码引用 | 全部使用相对路径可点击链接（相对于文档自身位置），附行号 `#Lxx-Lyy` |
| 里程碑命名 | M1/M2/M3...（主流）或 P0/P1/P2（少量使用，两者皆可） |
| 段落结构 | Summary / Current State Analysis / Proposed Changes / Verification（见 `rml-production-grade-iteration-plan.md`） |
| 任务表 | `| 任务 | 涉及文件 | 说明 |` 三列格式 |
| 验收标准 | 每个里程碑末尾独立章节 |
| 索引文件 | **无** — 每份文档独立存在，无 INDEX.md 子目录索引 |
| 语言 | 中文为主，关键术语保留英文 |

---

## Proposed Changes（拟变更）

### 变更 1：删除错误的 docs/iteration-plans/ 目录

**操作**：删除以下两个文件 + 空目录：

- `docs\iteration-plans\five-star-roadmap.md`
- `docs\iteration-plans\INDEX.md`
- `docs\iteration-plans\`（空目录）

**原因**：
- docs/ 是面向用户的章节式技术文档树，不接受内部迭代计划
- docs/INDEX.md 已规划 11 章结构，新增 `iteration-plans/` 子目录破坏既定布局
- `.trae/documents/` 才是项目既定的内部规划文档目录

**为什么全部删除而非保留内容**：内容将在变更 2 中按 `.trae/documents/` 既有约定重写，旧文件无保留价值。

### 变更 2：在 .trae/documents/ 下重写五星迭代计划

**新文件路径**：`.trae\documents\rml-five-star-iteration-plan.md`

**命名依据**：
- `rml-` 前缀：与 `rml-iteration-plan-v2.md`、`rml-production-grade-iteration-plan.md` 等保持一致
- `five-star-iteration-plan`：描述性名称，明确目标是「五星」
- 不加版本后缀：这是首版；若后续迭代，再加 `-v2`

**新文件结构（按 .trae/documents 既有约定重写）**：

```
# RML 框架五星迭代计划

> 视角：RML 架构师 × 框架设计者
> 主题：将「简洁易用 / 心智负担最低 / 高效发挥 Rust 优势 / 稳定」四维度从当前基线推向 5★
> 基线：v0.1.0，demo/ 41 case + shell + lsp 审查（2026-07-06）

## 一、Summary（摘要）
[总目标 + 四维度评分基线 + 五星目标表]

## 二、Current State Analysis（现状分析）
### 2.1 四维度当前基线
### 2.2 11 个根因（G1-G11）定位
[每个根因附相对路径链接指向具体代码位置]

## 三、五星目标定义（可验证标准）
### 3.1 简洁易用 5★
### 3.2 心智负担最低 5★
### 3.3 高效发挥 Rust 优势 5★（保持）
### 3.4 稳定 5★

## 四、迭代路线总览
[M1-M6 里程碑表，沿用 .trae/documents 主流的 M1/M2 命名]

## 五、M1：基础傻瓜化（P0 改进项）
### 5.1 M1-1：#[command] 宏自动注入 cx.notify()
- 目标 / 软件工程原理 / 设计方案 / 实施步骤 / 验证标准 / 风险与缓解 / 影响范围
- 涉及文件：[crates/macros/...]、[crates/core/...]、demo/src/cases/*.rml.rs
### 5.2 M1-2：#[contribute] 宏自动生成 IContribution impl
### 5.3 M1-3：<Input placeholder="..." /> codegen 透传

## 六、M2：核心心智负担消除（P1 改进项）
### 6.1 M2-1：#[observable] 字段属性替代 __rml_bump_version
### 6.2 M2-2：视觉 ability 自动注册
### 6.3 M2-3：on_loaded_deferred 钩子 + re-entrancy 自动吸收

## 七、M3：架构优化（P2 改进项）
### 7.1 M3-1：MainWindow 拆分
### 7.2 M3-2：on_rendered 钩子
### 7.3 M3-3：布尔属性语法统一
### 7.4 M3-4：IWorkbenchProvider 注册为 contribution

## 八、M4+：精雕细琢（P3 改进项，持续）
### 8.1 ContributionSlot 泛型化
### 8.2 命名规范统一
### 8.3 错误处理与稳定性强化

## 九、Verification（验证标准）
### 9.1 量化指标脚本
### 9.2 验证 case 映射表
### 9.3 回归测试套件

## 十、里程碑与时间线

## 附录 A：关键软件工程原理对照表
## 附录 B：术语表
## 附录 C：改进前后对比（counter_case 示例）
```

**与旧文档的关键差异**：

| 维度 | 旧文档（docs/iteration-plans/） | 新文档（.trae/documents/） |
|---|---|---|
| 位置 | docs/（用户文档树，错误） | .trae/documents/（内部规划，正确） |
| 命名 | `five-star-roadmap.md`（无 rml- 前缀） | `rml-five-star-iteration-plan.md`（带 rml- 前缀） |
| 索引 | 配套 INDEX.md | 无（独立文档） |
| 头部 | 文档定位块 | `> 视角/主题/基线` 三行块 |
| 段落标题 | 一/二/三... + Summary 等 | 沿用 `Summary / Current State Analysis / Proposed Changes / Verification` |
| 里程碑命名 | P0/P1/P2/P3 | M1/M2/M3/M4+（沿用主流约定） |
| 代码引用 | 部分文本引用 | 全部相对路径可点击链接，附行号 |
| 根因锚点 | 文字描述 | 每个根因附具体文件行号链接 |

### 变更 3：内容锚点强化

新文档的「现状分析」与「影响范围」部分，必须为每个根因和改进项附上**具体代码文件行号链接**，而非泛泛的描述。基于上一轮已读文件，已知的关键锚点：

| 根因 / 改进项 | 代码锚点 |
|---|---|
| G4 `__rml_bump_version` | [demo/src/cases/welcome_case.rml.rs#L84-L85](../../demo/src/cases/welcome_case.rml.rs#L84-L85)、[demo/src/cases/list_case.rml.rs#L79](../../demo/src/cases/list_case.rml.rs#L79)、[demo/src/shell/main_window.rml.rs#L411](../../demo/src/shell/main_window.rml.rs#L411) |
| G5 `cx.notify()` 不一致 | [demo/src/cases/counter_case.rml.rs#L59](../../demo/src/cases/counter_case.rml.rs#L59)、[demo/src/cases/expression_case.rml.rs#L68](../../demo/src/cases/expression_case.rml.rs#L68) |
| G6 ability cast 注册 | [demo/src/cases/status_bar_case.rml.rs#L141-L150](../../demo/src/cases/status_bar_case.rml.rs#L141-L150)、[demo/src/lsp/lsp_status.rs#L72-L82](../../demo/src/lsp/lsp_status.rs#L72-L82) |
| G7 re-entrancy 手动 defer | [demo/src/cases/welcome_case.rml.rs#L46-L53](../../demo/src/cases/welcome_case.rml.rs#L46-L53) |
| G2 Input placeholder 缺口 | [demo/src/cases/input_case.rml.rs#L42-L44](../../demo/src/cases/input_case.rml.rs#L42-L44) |
| G1 IContribution 样板 | [demo/src/cases/counter_case.rml.rs](../../demo/src/cases/counter_case.rml.rs)（impl IContribution 块） |
| MainWindow God Object | [demo/src/shell/main_window.rml.rs](../../demo/src/shell/main_window.rml.rs)（570 行） |

这些锚点必须在新文档中作为相对路径链接出现，使计划「grounded in actual code」而非抽象描述。

---

## Assumptions & Decisions（假设与决策）

### 假设

1. `.trae/documents/` 目录的 100+ 份现有文档命名约定具有代表性，新文档应遵循
2. 用户希望的「重写」是按 `.trae/documents/` 既有约定调整格式与位置，而非推翻五星迭代计划的实质内容（P0-1 NotifyGuard、P0-2 IContribution 自动生成、P1-1 Observable 字段等改进项设计本身仍然有效）
3. 五星迭代计划的实质内容（11 个改进项 + 4 阶段 + 6 里程碑）在迁移中保持完整，仅格式与位置变化

### 决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 新文件位置 | `.trae/documents/rml-five-star-iteration-plan.md` | 遵循既有约定，rml- 前缀 + 描述性名 |
| 是否保留 INDEX.md | 不保留 | `.trae/documents/` 既有文档均独立存在，无子目录索引约定 |
| 里程碑命名 | M1/M2/M3/M4+ | `.trae/documents` 主流约定（M1/M2 比 P0/P1 更常见） |
| 实质内容是否调整 | 保留 11 个改进项设计，仅格式与锚点强化 | 用户未否定改进项本身，仅纠正位置错误 |
| 旧文件处理 | 直接删除 | 内容将完整迁移到新位置，无保留价值 |

---

## Verification（验证步骤）

执行完成后，按以下步骤验证：

### 步骤 1：确认错误内容已清理

```powershell
Test-Path 'docs\iteration-plans'
# 期望：False（目录已删除）
```

### 步骤 2：确认新文档已创建

```powershell
Test-Path '.trae\documents\rml-five-star-iteration-plan.md'
# 期望：True
```

### 步骤 3：确认新文档符合 .trae/documents 约定

- 标题以 `# RML 框架...` 开头
- 头部含 `> 视角：` / `> 主题：` / `> 基线：` 三行块
- 包含 Summary / Current State Analysis / Proposed Changes / Verification 四大段
- 代码引用全部使用相对路径可点击链接
- 里程碑使用 M1/M2/M3 命名

### 步骤 4：确认实质内容完整

新文档应包含：
- 4 维度评分基线与五星目标
- 11 个根因（G1-G11）
- 11 个改进项设计（每个含七要素：目标 / 原理 / 方案 / 步骤 / 验证 / 风险 / 影响范围）
- 6 个里程碑（M0-M5/M6）
- 量化验证脚本 + 验证 case 映射表 + 回归测试套件
- 3 个附录（原理对照表 / 术语表 / counter_case 改进前后对比）

### 步骤 5：确认 docs/ 树未受污染

```powershell
Get-ChildItem 'docs' -Directory | Select-Object Name
# 期望：仅 01-overview 至 11-cookbook 共 11 个章节目录，无 iteration-plans
```

---

## 执行顺序

1. 删除 `docs/iteration-plans/five-star-roadmap.md`
2. 删除 `docs/iteration-plans/INDEX.md`
3. 删除空的 `docs/iteration-plans/` 目录
4. 创建 `.trae/documents/rml-five-star-iteration-plan.md`（按上述结构，迁移并强化内容）
5. 按 Verification 5 步验证
