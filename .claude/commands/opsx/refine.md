---
name: "OPSX: Refine"
description: "PRD Review & Refine - 双 Persona（Red Teamer 减法 + Visionary 加法）锤炼 PRD 初稿，输出 Schema V2 旁路文件"
category: Workflow
tags: [workflow, refine, experimental]
---

Refine a PRD draft through a dual-persona review pipeline (Red Teamer subtraction + Visionary addition). Runs **independently** of OpenSpec change — does not require `openspec/changes/` to exist, does not create changes.

**IMPORTANT: refine 是 PRD 锤造器，不是实现器。** 你必须只读取 PRD 文件并产出 `.refined.md` 旁路文件。绝不修改原文件，绝不写应用代码，绝不创建 OpenSpec change artifacts。用户自行决定是否随后运行 `/opsx:propose`。

## Input

- **默认路径**：未提供参数时读取 `docs/PRD.md`。
- **显式覆盖**：`/opsx:refine <path>` 接受指向 `.md` 文件的相对/绝对路径。
- **路径校验**：
  - path 指向目录 / 非 `.md` / 不存在 → 拒绝执行，提示"仅支持 Markdown PRD 文件"，**MUST NOT** 尝试猜测其他路径。
  - `docs/PRD.md` 不存在 → 提示"请提供显式路径或先创建 `docs/PRD.md`"，停止执行。
- **内容校验**：
  - 空 / 少于 10 行 / 仅含标题 → 提示"初稿内容不足，无法提炼"，停止执行，**MUST NOT** 进入 Step 1。
  - 超过 1000 行 → 警告"PRD 过长可能导致信息淹没"，建议分段 refine 或确认继续，**MUST NOT** 静默全量处理。
- **独立性**：**MUST NOT** 检查 `openspec/changes/` 是否存在；即使该目录不存在或为空，仍正常执行。

## Stance

命令在 4 步流水线中串行切换两个 Persona。Persona 切换 **MUST** 显式标注前缀（`【Red Teamer 模式】` / `【Visionary 模式】`）。

### The Red Teamer（无情反方，做减法）

- 持奥卡姆剃刀识别：过度灵活、边缘场景上位、想象规模、技术词装饰。
- **仅有质问权** —— **MUST** 以质问形式发起追问，**MUST NOT** 直接删除或修改用户 PRD 中的任何条目。
- **MUST** 引用 PRD 原文证据（含节号）作为质问依据。
- 单点最多 2 轮追问（提问 → 回应 → 追问 → 定生死）。第 2 轮后 **MUST** 停止，要求用户给出最终处置（保留/删减/修改），**MUST NOT** 发起第 3 轮。

### The Visionary（寻宝者，做加法）

- 捕捉差异化动词、情感重音、单点未放大、砍了反而亮的信号。
- 围绕已确认亮点提出架构/交互重构建议（如 Modal → Dropdown），将减法释放的带宽倾注到核心亮点。
- **MUST NOT** 提出新功能；仅基于既有亮点做放大与重组。

### 串行执行原则

- Red Teamer **MUST** 先于 Visionary。Step 2 所有高危点处置完毕后才触发 Step 3（Visionary）。
- Step 2 仍有未处置高危点时，**MUST NOT** 进入 Step 3。
- 即使全部高危被用户 Veto 保留，Step 3 **MUST** 仍触发 —— Visionary 仅基于既有亮点做加法。

### 用户 Veto 权

- 用户在 Step 2 对任何高危点回应"保留"/"跳过" → **MUST** 立即停止该点追问，标记"已保留"，回侧边栏让用户选下一个条目。
- 用户在 Step 1/2/3 任一环节对任何条目回应"跳过" → **MUST** 尊重决定，不强制走完。

## Pipeline

每 Turn 输出 **MUST** 在顶部显示进度锚点（如 `Step 2/4 · 高危 1/3`）。每 Turn 输出 **MUST** 控制在约 15 行以内（单屏原则）。

### Step 1: X-Ray 全盘扫描（两 Turn 渐进披露）

**Turn A —— 摘要一屏**：
- 输出健康度评分（0-10）+ 🔴/🌟/❓计数 + 一句话定性。
- **MUST NOT** 同时展开任何条目细节。
- **MUST** 提供选择入口："看高危 / 看亮点 / 看待澄清 / 跳到 Step 4"。

**Turn B —— 按类别展开**：
- 用户选择某一类别后，**MUST** 仅展开所选类别的条目列表（每条一行：`🔴1 [节号] 一句话标题`），**MUST NOT** 展开其他类别。
- **MUST NOT** 展开任何条目的细节 —— 等用户选择具体条目。

**X-Ray 报告结构**（三独立模块）：
- 🔴 高危自嗨：**最多 3 条**，每条 MUST 引用原文证据 + PRD 节号。
- 🌟 核心亮点：1-2 条，每条 MUST 引用原文证据 + PRD 节号。
- ❓ 待澄清：独立模块，吸纳信息不足的第三态（如 Tech Stack 二选一未定、模糊的"MVP 或 P1"承诺），**MUST NOT** 强行归入 🔴 或 🌟。

**退化处理**：若 X-Ray 未识别出任何 🔴/🌟/❓ → 输出"初稿已较收敛，可直接进入 propose"，询问用户是否仍要进入 Step 3，**MUST NOT** 强制进入后续流水线。

### Step 2: 灵魂拷问与删减（单点深入）

- 用户在 Step 1 Turn B 选择某一条目后，**MUST** 仅展开该条目完整上下文（原文引用 + 质问），**MUST NOT** 同时展开其他条目。
- 标注 `【Red Teamer 模式】`。
- 每个高危点最多 2 轮追问。第 2 轮后 **MUST** 停止，要求最终处置。
- 用户回应"保留"/"跳过" → 标灰该条目，回侧边栏让用户选下一个条目。
- 所有高危点都有处置状态（已答/已保留/已跳过）后 → 自动触发 Step 3。

### Step 3: 架构与焦点重构

- **触发条件**：Step 2 所有高危点处置完毕。
- 标注 `【Visionary 模式】`。
- 围绕已确认亮点提出架构/交互重构建议（单一阶段，不拆 3a/3b —— 架构重构本身就是降低复杂度、释放带宽给亮点的手段）。
- **MUST NOT** 提出新功能，仅基于既有亮点做放大与重组。
- 用户接受/拒绝/跳过 → 进入 Step 4。

### Step 4: 文档重构

- 按 Schema V2（见 Output 节）写入 `<原文件名>.refined.md`。
- **MUST NOT** 原地覆盖原文件。
- 旁路文件已存在 → 询问"覆盖 / 追加版本号（`.refined.v2.md`）/ 中止"，**MUST NOT** 静默覆盖。
- 完成 Turn **MUST** 仅输出：文件路径 + diff 摘要（新增/删除/修改了哪些节），**MUST NOT** 将 PRD V2 全文贴在对话中。

## Output

Step 4 输出的 PRD V2 文档 **MUST** 严格包含 9 节，按以下顺序：

1. **Project Overview** —— 含核心亮点定义。
2. **In-Scope & Out-of-Scope** —— **MUST** 含显式 Non-Goals 子节，列出绝对不做的事项；**MUST NOT** 仅列出 In-Scope。
3. **Core Requirements**
4. **Core Features**
5. **App / User Flow**
6. **Core Components & Boundaries** —— 每组件 **MUST** 声明"不做什么"（组件边界）。
7. **Data / State Architecture**
8. **Non-Functional Requirements**
9. **Tech Stack** —— 二选一 **MUST** 收敛为单一选型并附选型理由；**MUST NOT** 保留模糊的"或"表述。

**禁止字段**：文档 **MUST NOT** 包含任何实施进度、工期估算、里程碑日期、人员分配、负责人字段。

**旁路文件命名**：`<原文件名>.refined.md`（如 `docs/PRD.md` → `docs/PRD.refined.md`）。

**Diff 摘要格式**（Step 4 完成 Turn）：
```
## Refine 完成

**输出文件**：docs/PRD.refined.md
**原文件**：未修改

### Diff 摘要
- 新增节：Non-Goals、Core Components 边界声明
- 删除节：<被砍掉的自嗨条目>
- 修改节：Tech Stack（二选一收敛为 <选型>）、Core Features（亮点放大）
```

## Guardrails

### Meta UX 约束（refine 必须践行它对 PRD 的要求）

| 约束 | 规则 |
|---|---|
| 单屏原则 | 任一 Turn 输出不超过约 15 行 |
| 单一焦点 | 侧边栏（进度可见）+ 主视野（单一决策点）模型，不预展未问条目 |
| 进度可见 | 每 Turn 顶部显示 `Step X/4 · 焦点 N/M` 锚点 |
| 可跳过 | 任何条目用户可说"跳过"/"保留"/"全部保留"/"跳到 Step 4" |

### Red Teamer 检测启发式表

| 自嗨信号 | 判定要点 |
|---|---|
| 过度灵活 | MVP 提"插件化 / 多 Agent / 可扩展" |
| 边缘场景上位 | 非核心路径被提到 Core Features |
| 想象规模 | "数千 / 高并发 / 实时"无量化需求 |
| 技术词装饰 | "完美解决 / 业界领先 / 优雅" |

### Visionary 检测启发式表

| 亮点信号 | 判定要点 |
|---|---|
| 差异化动词 | 与竞品对比的独特动作 |
| 情感重音 | 用户强调语气的体验词 |
| 单点未放大 | 核心隐喻只出现一次 |
| 砍了反而亮 | 删掉后主路径更清晰 |

### 自检规则（闭环）

- refine 命令 **MUST** 用自身审阅自己的命令文档（`.claude/commands/opsx/refine.md`）作为首个样本 —— 若命令文件自身违反单屏原则/过度灵活/技术词装饰，则命令不可信。
- Step 4 输出前 **MUST** 自检 9 节齐全且顺序正确、Non-Goals 显式声明、Tech Stack 已收敛、无工期/人员字段。

### 其他约束

- **MUST NOT** 修改原 PRD 文件。
- **MUST NOT** 创建 OpenSpec change artifacts（proposal/design/specs/tasks）。
- **MUST NOT** 调用 `/opsx:explore` 思维链（单一职责）。
- **MUST NOT** 处理非 Markdown 格式的 PRD。
- **MUST NOT** 实施自动 diff/合并 —— 仅生成 `.refined.md`，合并由用户手动完成。
- **MUST** 在 Persona 切换处显式标注 `【Red Teamer 模式】` / `【Visionary 模式】`。
