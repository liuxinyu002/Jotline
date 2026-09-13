# 《项目随手记》技术规范书 v0.5

| 项 | 内容 |
|---|---|
| 版本 | **v0.7**｜ 冻结范围：架构 / 目录 / ADR / 数据模型 / 跨模块契约 / Wire 纪律 |
| 依据 | PRD v1.3（评审稿）+ 规范评审讨论纪要（v0.1 → v0.2 → v0.3 三轮）+ 记忆机制方案讨论纪要（v0.4 输入）+ 记忆机制定稿评审纪要 |
| 取舍标准 | 不可逆决策、数据格式、跨模块契约 → 本稿冻结；模块内可逆细节 → 实施时以技术设计稿确定，走 IDR 机制（附录 B） |
| 相关文档 | PRD v1.3 ｜ v0.2（归档，模块设计输入）｜ HTTP vs IPC 论证（ADR-1 补充稿）｜ Agent 进程归属论证（ADR-9 补充稿）｜ 记忆机制讨论纪要 ｜ 第一周验证计划（独立执行文档）｜ 日志与审计规范讨论纪要（v0.6 输入） |

**v0.6 → v0.7 变更**：新增 §5「Agent 资源加载边界」与「Agent 工具面」契约、ADR-12 / ADR-13——Sidecar 资源全部构建期内置：能力经 extensionFactories 注册，skills / prompts / themes 经 additional 显式路径指向随包资源，noExtensions / noSkills / noPromptTemplates / noContextFiles 关闭全部目录发现（含 AGENTS.md），打包内容由开发期确定，开发与运行时同规则；内置工具面 = read / edit / write / grep / find / ls，关闭 bash / powershell（无 shell 执行），六工具经 tool_call 门圈定在数据目录 vault 内，写路径直通为 V1 过渡态。

**v0.5 → v0.6 变更**：① 新增 §5.1 日志规范定稿——三运行时日志统一经主进程渲染落盘（Sidecar stderr NDJSON / 前端 `/api/log`），单一文件、中文 message + 英文 fields、隐私纪律为 scrubber 输入边界；② 新增 §5.2 审计规范定稿——V1 范围 = `reveal_credential` + 项目敏感开关变更，审计记录脱离 index.sqlite 落 `vault/audit.jsonl`（append-only，fail-closed）；③ §3.1 数据目录新增 audit.jsonl，§4.3 表清单移除 audit_log；④ §1.2 进程职责补日志/审计通道；⑤ 新增 ADR-10 / 11、DEC-22 / 23；⑥ 附录 A 新增日志实施延期项。

**v0.4 → v0.5 变更**：① §4.5 记忆机制定稿：条目 schema 极简化为 {id, text, explicit, created_at, forbid_tag?}（删除 kind/key/count/last_hit），删除固化阈值——纠正一次即生效；写入管线 = card.execute 代码 diff + sidecar 轻量 LLM 提取（target_id 语义匹配）；注入改记/查全量、看不注入；② §4.3 memories 索引表标注 V1 空置；③ §5 记忆契约行更新；④ DEC-19/20/21 定稿；⑤ 附录 A 新增记忆实施延期项。

**v0.3-final → v0.4 变更**：① 新增 §4.5 记忆机制技术规范（memory.json 格式契约、写入管线、注入分发、策略边界），Memory 字段定义以本稿为准；② §4.3 memories 索引表标注 V1 可空置；③ §5 新增记忆契约行；④ 新增 DEC-19 / 20 / 21；⑤ 附录 A 新增记忆实施延期项。

**v0.3 → v0.3-final 变更**：① DEC-01 关闭（HTTP 统一业务面），ADR-1 加注；② 新增 ADR-9（Agent 固定于 sidecar）；③ Wire 纪律冻结入 §3.2；④ V3 浏览器访问口径预留入 §5；⑤ 阻塞启动开放项清零，延期映射 +2 行。

---

## 1. 系统架构

### 1.1 拓扑

```
┌────────────────────────────────────────────────────────┐
│  Tauri v2 应用（macOS 优先，签名 + 公证分发）              │
│                                                        │
│   主窗 Webview ┐  同一前端工程 · 共用 api-client          │
│   随手浮窗 Webview ┘  （无独立业务状态）                    │
│            │ HTTP REST + SSE（127.0.0.1 + token）       │
│            │ Tauri IPC 仅系统能力（窗口/托盘/快捷键/剪贴板）│
│   ┌────────┴─────────────────────────────┐              │
│   │  Rust 主进程（唯一监听者）              │              │
│   │  领域 API 网关 ｜ OCR 服务 ｜ 凭证加密库 │              │
│   │  SQLite(FTS5) ｜ 附件文件夹（真相源）    │              │
│   │  托盘 / 全局快捷键 / 通知 / updater     │              │
│   └───────┬──────────────────▲───────────┘              │
│     stdio JSON-RPC        │ HTTP（同 token）           │
│   ┌───────┴────────────────┴───────────┐              │
│   │  Node Sidecar：pi agent             │              │
│   │  （Bun 单文件，零监听端口）            │              │
│   │  provider 层出口 = 出站脱敏闸门        │              │
│   └──────┬──────────────┬───────────────┘              │
└──────────┼──────────────┼──────────────────────────────┘
     DeepSeek API     Ollama（敏感项目）
```

### 1.2 进程职责与通信

| 进程/模块 | 职责 | 通信 |
|---|---|---|
| 主窗 + 浮窗 Webview | UI；状态全部来自领域 API，不各自维护 | HTTP+SSE（业务）/ Tauri IPC（仅系统能力）/ POST /api/log（日志上报） |
| Rust 主进程 | 领域逻辑唯一实现；存储、OCR、凭证库、系统集成、**记忆账本（diff 检测 / 覆盖追加 / 原子落盘 / tags 过滤）**、**日志渲染与落盘（唯一写入者，§5.1）**、**审计账本（append-only 追加，§5.2）** | 唯一监听者；stdio 驱动 sidecar |
| Node Sidecar | Agent 会话、provider 层、脱敏闸门、**记忆 prompt 渲染（纯函数）** | 零监听；HTTP 回调主进程领域 API 与 /api/ocr；stderr 输出日志 NDJSON（不自渲染，§5.1） |

**通道模型**：Agent 参与的流程分两条通道——**通道 A 捕获管线**（操作卡片为载体，`card.execute` 为唯一确认与入库触发点）、**通道 B Agent 对话**（写操作经提案-行内确认）；两通道确认语义分离，底层共用同一套写 API，网关校验执行权来源（ADR-8）。提案机制、工具分级等细节见附录 A（对话模块设计稿）。

**日志与审计通道**：三个运行时（Webview / 主进程 / Sidecar）的日志统一经主进程渲染与落盘——Sidecar 走 stderr NDJSON，前端走 `POST /api/log`，主进程自身走 tracing；审计记录由主进程以 append-only 方式追加至 `vault/audit.jsonl`。规范见 §5.1 / §5.2。

---

## 2. ADR 摘要

| # | 决策 | 一句话理由 |
|---|---|---|
| ADR-1 | Rust 主进程是唯一监听者，一个 localhost 端口 + Bearer token 服务全部客户端；**前端业务面统一走 HTTP**（DEC-01 已采纳关闭，完整论证归档） | 浮窗/主窗/Agent 天然同源；网关唯一执法点；契约管线单一；V3 浏览器访问平滑过渡 |
| ADR-2 | Sidecar 零监听端口，stdio 驱动，HTTP 单向回调 | 子进程不新增网络面；Bun 纯 JS 约束不被破坏 |
| ADR-3 | 领域逻辑 = Rust 服务层单实现，UI 与 sidecar 只是不同调用方 | “浮窗一致性 100%”由架构保证，不靠逐功能对齐 |
| ADR-4 | OCR 在主进程内（Vision / RapidOCR+`ort`），接口化 /api/ocr | PRD v1.3 定案；bbox 同份数据服务高亮与归因 |
| ADR-5 | 文件夹 + Markdown 为真相源，SQLite 仅索引，可随时重建 | 本地优先；可移植；降级路径的兜底 |
| ADR-6 | 不建向量 RAG 层；`search_content` 为接口承诺，V1–V3 不换签名 | 避免过度建设；V3 可换实现 |
| ADR-7 | 出站脱敏闸门 = provider 出口的确定性函数 | 可单测、可 CI 回归；不在 prompt 层做脱敏 |
| ADR-8 | 双通道确认模型：A 卡片 execute / B 提案确认；网关校验执行权来源 | 确认语义分离、底层 API 统一；插件继承同层约束 |
| ADR-9 | **Agent 会话不进前端运行时，固定于 sidecar** | 双窗口无正确宿主；前端视为不可信运行时，执行权确认要求与 UI 运行时隔离；异步任务生命周期须独立于窗口（详版论证归档） |
| ADR-10 | **三运行时日志统一经主进程渲染与落盘**（Sidecar stderr NDJSON / 前端 `/api/log` / 主进程 tracing），主进程为唯一文件写入者 | 单一渲染点，dev/prod 格式天然一致；一份日志按关联字段串起跨进程流程；Sidecar 不自渲染（Bun 纯 JS 约束下零依赖，禁 pino transport） |
| ADR-11 | **审计记录脱离 index.sqlite**，落 `vault/audit.jsonl` append-only | 审计是记录不是索引——index.sqlite 可随时重建，审计不得随之丢失；append-only 文件与「文件夹为真相源」哲学一致 |
| ADR-12 | **Agent 资源 = 构建期内置**：能力经 extensionFactories 注册；skills / prompts / themes 经 additional 显式路径指向随包资源；noExtensions / noSkills / noPromptTemplates / noContextFiles 关闭全部目录发现（含 AGENTS.md），打包内容由开发期确定，开发与运行时同规则 | pi 默认自动扫描用户 / 项目扩展目录与 skills / prompts / AGENTS.md，外部代码与内容可经 before_provider_request / tool_result 绕过脱敏闸门，或经 system prompt 注入不可信指令；资源随版本发布，无需运行时发现 |
| ADR-13 | **Agent 内置工具面 = read / edit / write / grep / find / ls**，关闭 bash / powershell；六工具经能力模块 tool_call 门圈定在数据目录 vault 内；写路径直通为 V1 过渡态，迁移至领域 API 前接受无 provenance / 审计、FTS 索引靠重建兜底 | 复用 pi 现成读 / 写 / 查工具快速起步；关闭 shell 堵任意命令执行；tool_call 门堵注入驱动的任意路径读写（捕获内容是进模型上下文的不可信输入） |

---

## 3. 项目目录

### 3.1 数据目录（真相源布局）

```
<DataDir>/vault/
├── notes/<item_id>.md                 # 笔记与摘要（真相源，含凭证占位符）
│   └── .revisions/<item_id>/          # 合并覆写前快照（有限保留）
├── attachments/ab/cd/<sha256>         # 附件，内容寻址，永不改写
├── inbox/                             # 待处理队列（所有降级路径的落点）
├── projects/<prj_id>/memory.json      # 项目层记忆（JSON 真相源）
├── memory/global.json                 # 全局层记忆
├── templates/*.json                   # 模板 bundle
├── audit.jsonl                        # 审计真相源，append-only（§5.2，ADR-11）
└── index.sqlite                       # 索引，可随时删除重建（审计不在其中）
```

### 3.2 代码仓库结构

```
repo/
├── app/          # 前端（主窗 + 浮窗，同一工程）
├── src-tauri/    # Rust 主进程（领域服务、OCR、凭证库、记忆账本）
├── sidecar/      # pi agent（Bun 工程，编译为 externalBin）
├── contracts/    # CI 生成物：OpenAPI / 前端 TS 类型 / 工具 Schema
└── docs/         # PRD、本规范、技术设计稿、IDR
```

**契约单一事实源**：接口定义只存在于 Rust 结构体，经 schemars/utoipa 生成 OpenAPI 与前端/Agent 消费物，**禁止手写第二份**——这是“Agent 即接口”不漂移的机制保证。

**Wire 纪律（冻结，发布后变更属 breaking）**：

- ID 一律 string（ULID）；时间一律 ISO-8601 UTC，禁整数时间戳；
- 判别联合用 tagged enum；API 类型禁 untagged enum、禁 serde(flatten)；
- 可选语义 = 字段省略，禁 required-nullable；
- 字段命名 snake_case（对齐 PRD 卡片示例），API 包根声明一次，禁字段级混用；
- 生成类型止于 HTTP 边界，UI 视图模型不生成；
- 生成物 commit 入库 + CI diff 检查（Rust 字段变更 → 前端/sidecar 同一 CI 显红）。

工具链建议默认 utoipa + openapi-typescript + openapi-fetch（IPC 层 tauri-specta），随第一周 D3 spike 验证后以 IDR 定稿（见附录 A）。

---

## 4. 数据模型

### 4.1 通用约定

- 实体 ID 用 ULID；库内时间 UTC ISO-8601，时区只在展示层转换；
- `Event.at` 必附 `at_source ∈ {content_time, derived_time, capture_time}`，非捕获时间必填（PRD 时间回退链的落库约束）；
- JSON 真相源（memory / templates）顶部带 `schema_version`；SQLite 中对应表仅为索引。

### 4.2 核心实体

PRD §4.3 沿用：Project / Stage / Tag / Item / FileVersion / Event / Credential / Todo / TimelineGroup / Template / Experience / Card，字段定义以 PRD 为准，本稿不重复；**Memory 除外——以本稿 §4.5 为准（覆盖 PRD §4.3 字段定义）**。

评审新增四个实体：

| 实体 | 用途 |
|---|---|
| item_sources | 笔记分节溯源：合并/追加后一篇笔记多来源，每节一条（capture_id、section_anchor、origin、at） |
| note_revisions | 合并覆写前的版本快照（Agent 触发必快照，保留有限版数；revert 可逆） |
| proposals | 通道 B 提案：参数快照 + 状态 + 过期时间，支撑确认与审计 |
| ocr_cache | 同图免二次 OCR（key = 图片 SHA-256） |

### 4.3 SQLite 表清单

`projects / stages / tags / items(含 content_hash) / item_tags / file_versions / events(at, at_source, voided_at) / todos(due_at, status, source, source_ref) / timeline_groups / timeline_group_members / cards(status, decisions) / credentials_idx(仅元数据+引用计数) / memories(索引；V1 允许空置，读路径全量扫文件，§4.5) / track_events / ocr_cache / item_sources / note_revisions / proposals`

**audit_log 不在本库**：审计是记录不是索引，真相源为 `vault/audit.jsonl`（append-only，重建不得丢失，§5.2 / ADR-11）。

### 4.4 文件级格式契约

- **凭证占位符**：`[🔒 名称](credential://<ULID>)`——文件内 label 仅为 fallback，渲染以凭证库当前名称实时查询；ID 全局寻址，引用可跨笔记存在；
- 笔记 Markdown 是可编辑的检索代理；附件原件永不改写。

### 4.5 记忆机制（memory.json 格式契约与读写规则）

本节为 PRD §3.2 的技术定稿，字段定义覆盖 PRD §4.3 Memory 实体。定位：记忆 = 用户当前偏好的本地账本。语义判断（识别纠正、转译记忆文本、匹配已有条目）由 LLM 完成；代码只做机械动作（diff 检测、覆盖/追加、原子落盘、tags 过滤）。每项设计必须让 Agent 更准或让用户更安心，两者都不占的不做（DEC-20）。

**存储**：`memory/global.json` + `projects/<prj_id>/memory.json`（§3.1 布局），JSON 数组 + 顶部 `schema_version`；写入一律 temp + rename 原子替换。scope 不落字段——文件位置即 scope。

**条目 schema（冻结）**：

```json
{
  "id": "mem_01J8Z3…",
  "text": "Oracle 相关的内容都归 AMS 项目",
  "explicit": false,
  "created_at": "2025-06-12T03:20:00Z"
}
```

标签黑名单条目附加可选字段：

```json
{
  "id": "mem_01J8Z9…",
  "text": "以后别建议「风险」这个标签",
  "explicit": true,
  "created_at": "…",
  "forbid_tag": "风险"
}
```

| 字段 | 用途 |
|---|---|
| id | ULID。LLM 更新已有条目的锚点（target_id）；管理页编辑/删除定位；原子写对象 |
| text | 记忆正文，自然语言单句（单行）。三用：prompt 渲染素材 / 管理页展示 / LLM 提取产物 |
| explicit | true = 显式规则：即时写入生效、渲染加【规则】前缀、永不自动失效 |
| created_at | 管理页排序 |
| forbid_tag | 可选，仅黑名单条目。落库路径对卡片 tags 的确定性过滤依据 |

无 count / kind / key / last_hit 字段（DEC-19/20/21）：无固化阈值故无计数；条目匹配由 LLM 语义完成故无 kind/key；无行为依赖故无 last_hit。

**写入管线（唯一触发点 = card.execute，两段式）**：

```
① 代码 diff(Agent 初始建议, 最终执行状态)   —— 确定性，零成本
   无差异 → 不写（Agent 一次说对 = 零成本）
   有差异 → 进入 ②

② sidecar 一次轻量 LLM 提取调用
   输入：差异上下文 + 现有记忆列表（本次已注入）
   输出：{ "target_id": <已有条目id | null>, "text": "…", "explicit": bool, "forbid_tag": "…" | null }
   → 提交 memory.* API：target_id 命中 → 覆盖旧值（弃旧，无历史版本）；null → 追加
   → temp + rename 原子写
```

- 卡片对话调整过程不计入（改草稿非定论），仅最终执行时 diff；
- 显式规则（用户明说“以后都这样”）在对话中识别，同一路径写入，explicit = true；
- 浮窗与主窗发起的纠正同路由（同一 card.execute，天然同权）；
- 写入失败静默：不提示、不阻塞捕获，下次纠正自然重写。

**注入分发**：

| 场景 | 范围 |
|---|---|
| 记（卡片生成） | 全量：全局层 + 全部活跃项目层（项目推断与卡片生成是同一次 LLM 调用，不按候选项目裁剪） |
| 查（问答） | 全量（人物称呼 / 客户术语把口语化问句翻译为可检索词，FTS 字面匹配不懂黑话） |
| 看 / 其他 | 不注入 |

**渲染规则**：`renderMemoryPrompt(memories, scene)` 为 sidecar 内**纯函数**（JSON → 文本不经 LLM，可单测）：

- 渲染为自然语言短句，禁止 dump 原始 JSON；text 即素材；
- explicit 条目加【规则】前缀，置区块顶部；
- 区块尾部固定一句：“与用户当前说法冲突时，以当前说法为准”（防旧记忆劫持当前意图）；
- 单条压单行（strip 换行；text 超长截断 ≈100 字符）；
- 记忆为空 → 跳过整个区块，不渲染占位（冷启动结构由模板供给，记忆只做增量）；
- 每次生成实时渲染，不缓存。

**策略边界（冻结）**：

- 无固化阈值：纠正一次即记住、下次生效——人在环下错误记忆代价 = 一次可纠正的建议，阈值仅延迟学习、伤害体验（DEC-19）；
- 无 TTL、无休眠状态机、无历史版本：项目层随结项归档，全局层不设过期，覆盖即弃旧（DEC-20）；
- 管理页：设置页内纯本地 UI，读写走 `memory.*` 领域 API（与浮窗/主窗同源），以 text 逐条展示，可改可删；保险丝定位，不进任何主工作流；
- 标签黑名单双保险：prompt 注入（引导）+ 落库路径对卡片 tags 确定性过滤（保证），过滤在领域 API 层实现；
- 记忆注入与 V2 few-shot 为两个独立通道：记忆是规则，few-shot 是示例，不混用、评估分开；
- V1 读路径全量扫 JSON 文件（几十条量级），memories 索引表允许空置（结构保留，V2+ 按需启用）。

**埋点**：卡片落库时记录「是否命中记忆 + 是否仍被纠正」进 track_events（仅本地，用户无感），支撑 PRD §2.4 记忆有效性两指标（冷热对比 ≥ 15pp / 被引用仍纠正率 < 30%）。

---

## 5. 跨模块契约（冻结最小集）

| 契约 | 内容 |
|---|---|
| 工具层 = 领域 API | UI 与 Agent 共用同一 API；V2+ 插件必须经此层，禁止旁路，自动继承 provenance / 审计 / 脱敏 / 执行权校验 |
| **Agent 资源加载边界** | Sidecar 只加载仓库内资源：能力模块（构建期 extensionFactories 随 Bun 打包），skills / prompts / themes 经 additional 路径显式指向随包资源；loader 设 noExtensions / noSkills / noPromptTemplates / noContextFiles 关闭全部目录发现——`~/.pi/agent/*`、`.pi/*`、settings.json packages/extensions、AGENTS.md 一律不读，打包内容由开发期确定，开发与运行时同规则；扩展内工具实现只经领域 API |
| **Agent 工具面** | 内置 read / edit / write / grep / find / ls 保留，bash / powershell 关闭（无 shell 执行）；六工具经 tool_call 门圈定在数据目录 vault 内；能力模块工具经领域 API；写路径直通为 V1 过渡态，V2 前迁领域 API（ADR-13） |
| V1 API 清单（签名实施时定） | `list_projects / capture.submit / card.* / create_note / save_file / read_file / note.append / note.merge_draft / note.merge / note.revisions / note.revert / upsert_tags / add_timeline_event / event.void / timeline_group.* / search_content / list_credentials / reveal_credential / credential.* / todo.* / memory.* / proposal.confirm / report.datasource` |
| search_content 接口承诺 | `(query, filters, top_k) → [{id, title, snippet, provenance, score}]`，V1–V3 不变 |
| OCR 接口承诺 | `/api/ocr` 返回带 bbox 的行流 `{text, confidence, bbox, reading_order}`；批量任务与队列细节实施时定 |
| **记忆契约** | 写入唯一触发点 = `card.execute`（代码 diff 检测 → sidecar 轻量 LLM 提取 target_id / text / explicit / forbid_tag → `memory.*` 落库，覆盖即弃旧，§4.5）；注入按场景分发（记全量 / 查全量 / 看无）；`renderMemoryPrompt` 为 sidecar 纯函数，禁 dump JSON；黑名单在落库路径强制过滤；显式规则即时写入 |
| 凭证三约束 | ① list 的 schema 层面无 secret 字段；② reveal 明文只进剪贴板（主进程写入，不回流前端 / 对话 / 日志）；③ confirm_token 仅 UI 确认动作可签发，Agent 拿不到 |
| 网关统一织入 | 审计、provenance、本地埋点、执行权来源校验（`card_execute / proposal_confirm / ui_direct`）在网关层统一实现，业务代码不得散写 |
| **日志契约** | 三运行时统一经主进程渲染落盘：Sidecar stderr NDJSON（一行一条 `{ts, level, target, msg, fields}`）、前端 `POST /api/log`（批量，fire-and-forget，prod 仅 WARN/ERROR）；单一文件按天轮转保留 14 天；行格式与级别语义见 §5.1；隐私纪律：凭证明文 / token / 主密码 / 脱敏前文本任何级别不进日志（scrubber 输入边界，DEC-08） |
| **审计契约** | V1 范围 = `reveal_credential` + 项目敏感开关变更；真相源 `vault/audit.jsonl` append-only + fsync，不提供清除入口；fail-closed（审计写失败拒绝 reveal）；schema 与判据见 §5.2 |
| 降级总原则 | LLM 不可达 / 解析失败 / schema 校验失败 → 原文入 inbox，永不阻塞捕获；明细见 v0.2 §10 |
| **V3 浏览器访问口径（预留）** | 浏览器为**受限调用来源**：`reveal_credential` 一律拒绝；`list_credentials` 仅元数据且受项目成员制约束；hub 绑定 LAN 的前置条件 = 真认证 + 传输加密（PRD 附录 A TBD 不变）；非 hub 单机永不暴露 LAN 端口 |

### 5.1 日志规范（定稿）

**通道拓扑**：

| 来源 | 通道 | dev（Rust 控制台） | prod（文件） |
|---|---|---|---|
| Rust 主进程 | tracing 直写 | 控制台（ANSI 着色） | 文件 |
| Node Sidecar | stderr NDJSON → 主进程解析注入 | 同上渲染 | 同上 |
| Webview 前端 | `POST /api/log`（数组批量，fire-and-forget） | INFO 起上报 | 仅 WARN/ERROR |

- 主进程是**唯一渲染点与文件写入者**：一份日志文件按 `capture_id` 等关联字段串起跨进程流程；Sidecar / 前端的 target 由主进程注入来源前缀，Sidecar 自身不自渲染
- 单一文件：`~/Library/Logs/<bundle-id>/app.log`（macOS 惯例，Console.app 可见），按天轮转，保留 14 天，非阻塞写入（缓冲满丢弃并计数——诊断日志宁丢勿阻塞主流程，与「写入失败静默」哲学一致）
- `/api/log` 上报失败静默丢弃，不重试、不反制错误；前端 devtools console 保留原样输出，仅作前端调试辅助，不属于日志系统

**行格式**（console 与文件同构，文件无 ANSI、时间戳完整）：

```text
14:30:01.234 INFO  rust.ocr 单图识别完成 capture_id=01J8Z3 engine=rapidocr lines=42 elapsed_ms=1133
2025-06-12 14:30:01.234+08:00 INFO rust.ocr 单图识别完成 capture_id=01J8Z3 engine=rapidocr lines=42 elapsed_ms=1133
```

- **target 首段 = 来源**：`rust.` / `sidecar.` / `web.`（Sidecar NDJSON 的 target 不带前缀，由主进程注入）；不做列对齐、不截断
- **message**：中文、单行、谓语开头、**不含可变数据**——可变数据全部外置 fields；大 payload（prompt 全文、OCR 全文）任何级别不进日志，需样本时截断；错误原因链（anyhow / 底层库原文）保留英文，不翻译
- **fields**：key 英文 snake_case（对齐 Wire 纪律），值含空格才加引号
- **多行仅限 ERROR**：主行 + 4 空格缩进原因链（anyhow chain / panic backtrace / 前端 stack），其余级别严格单行

**级别语义（写死）**：`INFO` = 一次业务动作完成（卡片就绪 / 入库完成 / 降级触发）；`DEBUG` = 管线内部步骤；`WARN` = 可自动恢复（重试 / 降级 / 丢日志计数）；`ERROR` = 需人工介入。默认级别：dev = DEBUG，prod = INFO（prod 默认级别使用户捕获内容天然不落盘）。

**时间戳**：日志为给人看的调试数据，用**本地时间带偏移**（`+08:00`）——与 §4.1 数据 UTC 纪律刻意区分，不构成冲突。

**Sidecar stderr NDJSON 契约（一行一条）**：

```json
{"ts":"2025-06-12T14:30:03.890+08:00","level":"warn","target":"provider.deepseek","msg":"请求重试","fields":{"capture_id":"01J8Z3","attempt":"1/3","status":429,"wait_ms":2000}}
```

**隐私纪律（硬约束，即 scrubber 的输入边界，DEC-08）**：凭证明文、token、主密码、脱敏前文本，**任何级别不得进日志**。

### 5.2 审计规范（定稿）

**判据（冻结）**：audit 只记录**事后无法从数据本身恢复的动作**；业务操作的追溯已由 provenance / note_revisions / event.void 等数据结构承载，audit 一律不双写。

**V1 范围（冻结，两条）**：

| action | 触发 | 记录 |
|---|---|---|
| `reveal_credential` | 凭证明文取用（复制按钮确认后） | `source` 标注 main / floating（浮窗与主窗同权） |
| `sensitive_toggle` | 项目敏感开关变更 | `detail` 必记 `{from, to}`——隐私通道变更的追溯锚点：低频事件一行成本，回答「何时起该项目的捕获走上云通道」 |

**存储（ADR-11）**：`vault/audit.jsonl`，append-only 追加 + fsync，永不改写；**脱离 index.sqlite**（审计是记录不是索引，重建不得丢失）；**不提供清除入口**（与 track_events「可查看/导出/清除」语义严格区分）；V1 不提供审计查看 UI（文件即界面）。

**记录 schema（冻结）**：

```json
{"id":"aud_01J8Z3…","at":"2025-06-12T06:32:00Z","action":"reveal_credential","source":"floating","target_id":"cred_01J8Y…"}
{"id":"aud_01J8Z4…","at":"2025-06-12T06:35:00Z","action":"sensitive_toggle","source":"main","target_id":"prj_ams","detail":{"from":false,"to":true}}
```

- `detail` 可选（字段省略 = 无，对齐可选语义），`sensitive_toggle` 必带
- `at` 用 UTC ISO-8601（§4.1 数据纪律，与技术日志的本地时间刻意区分）
- 凭证任何明文片段不得出现在记录中（凭证三约束②的延伸）

**fail-closed**：reveal 流程 = 确认 → 追加审计（fsync）→ 写剪贴板；**审计写入失败则拒绝本次取用**并落 ERROR 日志——有缺口的审计等于没有审计。

**V3 展望（不冻结）**：hub 多用户时增加 actor 字段；凭证写操作、成员变更、数据导出是否入审届时议。

---

## 6. 决策记录

### 6.1 已关闭

| # | 决策 | 结论 |
|---|---|---|
| DEC-01 | 前端业务面：HTTP+SSE 统一 vs 纯 Tauri IPC | **HTTP 统一业务面**（ADR-1；IPC 仅系统能力） |
| DEC-11 | 凭证占位符格式 | link 语法（§4.4） |
| DEC-13 | 合并/追加建议上 V1 | 采纳 |
| DEC-15 | 叙事型文档仅追加，结构化合并限资产型文档 | 采纳 |
| DEC-16 | 卡片内调整对话归通道 A 延伸（改草稿不落库，免逐轮确认） | 采纳 |
| DEC-17 | Agent 运行时归属 | **固定于 sidecar，禁入前端运行时**（ADR-9） |
| DEC-18 | V3 浏览器调用来源定位 | **受限调用来源**（§5 预留口径） |
| DEC-19 | 记忆写入时机与生效门槛 | 唯一触发点 = card.execute（代码 diff → sidecar 轻量 LLM 提取 → `memory.*` 落库）；纠正一次即记住、下次生效，无固化阈值（§4.5） |
| DEC-20 | 记忆生命周期与形态 | 无 TTL、无证据链、无历史版本、覆盖即弃旧；schema = `{id, text, explicit, created_at, forbid_tag?}`，语义匹配由 LLM 完成——当前偏好账本，非审计记录（§4.5） |
| DEC-21 | 记忆注入与渲染 | 记/查全量注入、看态不注入；renderMemoryPrompt 纯函数自然语言渲染，禁 dump JSON（§4.5） |
| DEC-22 | 日志通道与格式 | 三运行时统一经主进程渲染落盘（ADR-10）；message 中文 + fields 英文 snake_case；单文件按天轮转保留 14 天；时间戳本地带偏移；隐私纪律 = scrubber 输入边界（§5.1） |
| DEC-23 | 审计范围与存储 | V1 = `reveal_credential` + 敏感开关变更，判据 = 「效果不落在可追溯数据结构上的动作」；脱离 index.sqlite 落 vault/audit.jsonl append-only，fail-closed，不提供清除（§5.2，ADR-11） |

### 6.2 阻塞启动的开放项

**无**。v0.4 达到冻结条件。

### 6.3 延期至实施期的决策项

| # | 议题 | 触发点（详见附录 A） |
|---|---|---|
| DEC-02 | 中文 FTS 方案 | 第一周检索 spike 实测后 |
| DEC-03 | V1 文件夹拖入行为 | 捕获管线模块设计稿 |
| DEC-04 / 05 | bbox 坐标系 / OCR 低置信阈值 | OCR 模块设计稿（结合第一周 spike） |
| DEC-06 / 07 / 12 | 凭证恢复码交互 / 剪贴板策略 / 是否开放查看明文 | 凭证模块设计稿 + 安全评审 |
| DEC-08 | 脱敏匹配数据载入方式 | provider 层开发前 + 安全评审 |
| DEC-09 / 10 | 待办状态机 / 浮窗「查」会话策略 | 待办中心 / 浮窗模块设计稿 |
| DEC-14 | 模板内容清单（含 `doc_kind` 标记） | **优先：阻塞合并功能设计稿**，模板内容评审会 |

---

## 附录 A：延期内容与启用时机（v0.2 归档映射）

| 延期内容 | 启用时机 |
|---|---|
| OCR 接口细节、引擎路由、队列（原 TS-012~015，DEC-04/05） | OCR 模块设计稿（结合第一周 OCR spike 结果） |
| 捕获状态机、投递聚合、聊天归因规则（TS-009~011） | 捕获管线模块设计稿 |
| 中文检索方案（DEC-02） | 第一周检索 spike 实测后定 |
| 凭证链路细节：Chip 状态机、provisional 生命周期、恢复码、剪贴板策略（TS-028~031，DEC-06/07/12） | 凭证模块设计稿 + 安全评审 |
| 脱敏实现与日志 scrubber（TS-018/019，DEC-08） | provider 层开发前 + 安全评审 |
| 提案机制细节、工具分级表（TS-038） | 对话模块设计稿 |
| **记忆实施细节：提取 prompt 模板与 target_id 匹配策略、显式规则识别标准（“以后都这样”的判定）、渲染文案模板、管理页交互细节** | 对话 / 记忆模块设计稿 |
| 合并安全协议、记忆联动（TS-034~036 细则） | 合并功能设计稿（**依赖 DEC-14 模板清单先行评审**） |
| 资产型提取规则与模板骨架（TS-032/033，DEC-14） | 模板内容评审会 |
| **契约管线实现细节**：工具链定稿、生成脚本、Ajv 运行时校验 | contracts 模块设计稿 + 第一周 D3 spike |
| **Sidecar 生命周期管理**：token 引导、启动时序、SSE 重连、崩溃重启 | 通信/sidecar 模块设计稿 |
| 性能预算拆解、测试验收细则、黄金集扩充（TS-024/027） | 各模块设计稿附带 / 独立测试计划文档 |
| 待办状态机、浮窗会话策略（DEC-09/10） | 待办中心 / 浮窗模块设计稿 |
| **日志实施细节**：tracing layer 配置、文件轮转与保留参数、Sidecar logger 实现（自写，禁 pino transport——Bun 编译兼容性）、panic hook、/api/log 批量与失败策略、prod 日志级别开关 | 通信 / sidecar 模块设计稿 |

## 附录 B：实施决策记录（IDR）

实施期技术决策的沉淀机制：每个模块开发前的技术设计稿评审通过后，在此追加一行。**与 ADR 冲突时必须先修订 ADR 再实施。**

| IDR | 日期 | 模块 | 决策 | 依据/设计稿链接 |
|---|---|---|---|---|
| （空，随实施追加） | | | | |

---
