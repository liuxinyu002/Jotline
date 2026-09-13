# 《项目随手记》项目分期行动大纲

> **依据**：docs/PRD.refined.md（Refined v2.1）｜docs/SPEC.md（v0.7）｜DESIGN.md（Cool Mist 设计系统）
> **范围**：**V1 全量**（PRD §2.1 V1 行：随手浮窗、统一捕获管线、操作卡片、记忆机制、模板系统、时间线、检索、凭证库、待办中心基础版）。V2（批量导入 / PDF 解析 / 周报 / 飞书转发）与 V3（Hub 同步 / 经验沉淀）不在本大纲内，V1 交付后另立。
> **用法**：本大纲是蓝图与追踪清单。大纲确认后，逐阶段进入「详细规划」对话（每阶段约 3–8 轮、上限 10 轮），产出该阶段实施计划后再动手实施。**测试不进本大纲**——单元测试由开发过程中的专用命令负责添加，路线图不管理；但每阶段的「验证方式」是强制完成标准，不可省略。唯一例外是 [Integration] 轨：端到端联调与完整用户旅程验收本身就是该轨的核心内容。

---

## 0. 第零步结论：边界与范围识别

### 0.1 前后端边界

**结论：[FE] 不细分，[BE] 细分为两轨。**

- 本项目是**单一桌面应用，无管理端 / 用户端之分**：主窗与随手浮窗是**同一前端工程**（PRD §9 Tech Stack：浮窗与主窗同一工程、共用组件与状态层；SPEC §1.1：共用 api-client）。因此 [FE] 保持单一轨道，主窗 / 浮窗作为该轨内的交付面区分，不拆轨。
- 「后端」按**真实进程边界**细分为两轨，与 SPEC §3.2 的三工程目录结构一一对应：
  - **[BE-Core]** = `src-tauri/` Rust 主进程：领域 API 网关、SQLite + vault 存储、OCR 服务、凭证加密库、审计、日志渲染、记忆账本（ADR-1/3/4/5/10/11）。
  - **[BE-Agent]** = `sidecar/` Node Sidecar（pi agent，Bun 编译单文件）：Agent 会话、provider 层（DeepSeek / Ollama）、出站脱敏闸门、记忆 prompt 渲染（ADR-2/7/9/12/13）。
  - 拆分理由：两者是**不同技术栈（Rust / TS）、独立工程、独立工作量**，且 SPEC 已冻结其进程职责与通信协议（stdio JSON-RPC + HTTP 单向回调）；并行开发时契约边界清晰，集成风险隔离。
- **[Shared]** 承载 `contracts/`（跨三端契约：OpenAPI / 前端 TS 类型 / sidecar 工具 Schema），是全部轨道的公共前置。

### 0.2 交付范围

**结论：需要显式发布阶段。**

PRD §9 分发行明确要求「Developer ID 签名 + 公证，内部分发免右键打开」；NFR 平台行要求「updater 签名密钥发布前配置」。因此集成阶段之后设独立的发布阶段（Phase-20：打包、签名、公证、分发、全新环境走查）。发布范围限定 macOS（V1 不做 Windows）。

---

## 1. 阶段总览

| ID | 轨道 | 阶段名称 | 依赖 | 预估规划轮次 | 验证方式摘要 |
|---|---|---|---|---|---|
| Phase-1 | [Shared] | 契约与工作区骨架 | — | 5–6 | `pnpm contracts:build` + `pnpm mock` + curl 契约样例 / SSE 模拟流 / 401 |
| Phase-2 | [Shared] | 最小垂直切片 | Phase-1 | 4–5 | `pnpm dev` + curl create_note → vault 文件 + SQLite 行 + search 命中 + 前端界面可见 |
| Phase-3 | [BE-Core] | 存储与领域 API 基座 | Phase-2 | 6–8 | `pnpm seed` + 实体 curl 全链 + at_source 校验 400 + 删库重建后检索仍命中 |
| Phase-4 | [BE-Core] | OCR 服务 | Phase-2 | 3–4 | `screencapture` 造图 → `/api/ocr` 行流+bbox → 批量 job 轮询 → 同图缓存命中 |
| Phase-5 | [BE-Core] | 捕获管线与操作卡片 | Phase-3, Phase-4 | 5–7 | `AGENT_MODE=stub` 下 capture→card→execute 全链 + 多图聚合单卡 + 降级入 inbox |
| Phase-6 | [BE-Core] | 笔记深加工 | Phase-3 | 4–5 | append/merge/revert curl 链 + `.revisions` 快照 + item_sources 多来源行 |
| Phase-7 | [BE-Core] | 凭证库、审计、日志与记忆账本 | Phase-3, Phase-5 | 6–8 | reveal→剪贴板+audit.jsonl；fail-closed 拒绝；memory.json 写入；三通道日志 |
| Phase-8 | [BE-Agent] | Sidecar 基座与 Provider 层 | Phase-2 | 4–6 | Bun 单文件构建 + `sidecar:chat` 走 DeepSeek/Ollama + 脱敏出站核对 + stderr NDJSON |
| Phase-9 | [BE-Agent] | Agent 会话、工具面与提案机制 | Phase-8 | 5–7 | 会话内六工具圈定 vault、bash 关闭、越权路径拒绝、提案确认生效 |
| Phase-10 | [BE-Agent] | 捕获结构化与记忆管线 | Phase-8 | 6–8 | `sidecar:capture` 样例→卡片 JSON；密码转凭证卡；schema 失败降级；记忆渲染输出 |
| Phase-11 | [FE] | 应用骨架与多窗口体系 | Phase-1, Phase-2 | 4–5 | 连 Mock：主窗+浮窗双窗口同源数据；`pnpm typecheck` 契约类型零漂移 |
| Phase-12 | [FE] | 随手浮窗与系统集成 | Phase-11 | 5–6 | ⌥Space 唤起/切态/拖拽记忆/置顶/投递预览/空态三步卡 全操作路径 |
| Phase-13 | [FE] | 捕获交互与操作卡片 | Phase-11, Phase-12 | 6–8 | Mock 场景：投递→流式→卡片→对话调整→执行/丢弃（浮窗+主窗双面） |
| Phase-14 | [FE] | 目录、标签与文件浏览器 | Phase-11 | 5–6 | Mock：目录树/标签筛选/版本链切换/重复投递提示 |
| Phase-15 | [FE] | 时间线与检索 | Phase-11, Phase-14 | 4–6 | Mock：阶段与 group_name 折叠、作废交互、Cmd+P、OCR 文本命中高亮 |
| Phase-16 | [FE] | 待办中心与项目工作台 | Phase-11, Phase-12 | 4–5 | Mock：卡片墙/日历/勾销/溯源回跳/托盘与浮窗计数联动 |
| Phase-17 | [FE] | 设置、凭证库、记忆与模板管理页 | Phase-11 | 5–7 | Mock：改键生效、占位符 chip、复制确认流、记忆增删改、模板另存 |
| Phase-18 | [Integration] | 捕获链路端到端联调 | Phase-5, Phase-10, Phase-13–16 | 6–8 | 真实链路完整截图捕获旅程 + 三键仪式文案五处核对 + 性能冒烟 |
| Phase-19 | [Integration] | 查询、凭证、记忆与安全链路联调 | Phase-7, Phase-9, Phase-10, Phase-17, Phase-18 | 5–7 | 「查」引用回答、凭证全旅程+审计、记忆纠正闭环、敏感开关本地链路、脱敏冒烟 |
| Phase-20 | [Integration] | 打包、签名、公证与发布 | Phase-18, Phase-19 | 4–5 | `pnpm tauri build` + codesign/spctl 验证 + 全新环境安装走查 |

**依赖图**（Phase-2 完成后三线并行，末端汇合）：

```
P1 → P2 ─┬→ P3 ──→ P5 ──→ P7 ─────────────┐
         ├→ P4 ──↗                        │
         ├→ P8 ──→ P9                     │
         │      └→ P10                    │
         └→ P11 ─→ P12 ─→ P13             │
                ├→ P14 ─→ P15             │
                ├→ P16                    │
                └→ P17                    │
P18(I1) ← P5, P10, P13–P16               │
P19(I2) ← P7, P9, P10, P17, P18 ←────────┘
P20      ← P18, P19
```

---

## 2. 总则与执行纪律

1. **契约先行（铁律）**：接口定义只存在于 Rust 契约源（schemars/utoipa），经生成管线产出 OpenAPI / 前端 TS 类型 / sidecar 工具 Schema，**禁止手写第二份**。开发期间如需调整契约，流程固定为：**先修改契约源 → 重新生成并 commit → CI diff 检查同步受影响一侧（含 Mock 场景）→ 再实施代码**。严禁绕过契约直接改动任何一侧。
2. **阶段开工前置**：每个 [BE-Core] / [BE-Agent] / [FE] 阶段开工前，其涉及域的契约与 Mock 场景必须已入库（Phase-1 交付首批核心域；凭证 / 记忆 / 提案等域随对应阶段前的契约增补进入，走同一管线）。FE 阶段所需 Mock 场景数据集缺失时，先补契约场景再开发 UI。
3. **验证基线**：[BE-Core] / [BE-Agent] 阶段以终端命令（curl / driver 脚本 / sqlite3 / 文件检查）验证真实实现；[FE] 阶段在 Mock 服务上以界面操作路径验证（验证的是「前端行为符合契约场景」，真实行为验收留给 [Integration]）；[Integration] 以完整用户旅程 + 终端核对（audit.jsonl / app.log / pbpaste）验收。
4. **开发环境约定**（Phase-1 已定稿，后续阶段直接引用）：主进程 `127.0.0.1:4765`，Mock 服务 `127.0.0.1:4766`，前端 dev `:1420`；dev 模式固定 Bearer `dev-token`；数据目录 `JOTLINE_DATA_DIR`（默认 `./vault`）；根脚本 `pnpm dev` / `pnpm mock` / `pnpm contracts:build` / `pnpm contracts:check` 等。
5. **DEC / IDR 衔接**：SPEC 附录 A 的延期决策在各对应阶段的设计稿中定稿并落 IDR（各阶段「上下文」中标注）。与 ADR 冲突时先修订 ADR 再实施。

---

## 3. 阶段详述

### 轨道 [Shared]：契约与切片

- [x] **Phase-1 [Shared] 契约与工作区骨架** —— 一切共享物以契约为唯一来源

  - **依赖**：无（起点）
  - **上下文**：SPEC 冻结了 Wire 纪律与「契约单一事实源」机制，但契约源、生成管线、Mock 服务尚不存在。本阶段建立承载全部后续工作的最小工作区，并让「可执行契约」先于任何业务代码存在。工具链按 SPEC 建议默认（utoipa + openapi-typescript + openapi-fetch，IPC 层 tauri-specta），本阶段 spike 验证后以 **IDR 定稿**（对应 SPEC「第一周 D3 spike」）。
  - **核心目标**：
    - 建 monorepo 五目录骨架（`app/` `src-tauri/` `sidecar/` `contracts/` `docs/`）+ 根工作区（pnpm workspace、CI、lint）
    - Rust 契约源 crate 起步：类型定义 + utoipa 注解 + 生成脚本（OpenAPI → 前端 TS 类型 / sidecar 工具 Schema），生成物 commit 入库 + CI diff 检查
    - 定稿 Wire 纪律落地方式（ULID string、ISO-8601 UTC、tagged enum、可选=字段省略、snake_case、禁 untagged/flatten）与错误码表、鉴权约定（localhost + Bearer token）、SSE 事件信封格式
    - 定义主进程 ↔ sidecar stdio JSON-RPC 协议（健康检查、结构化请求/响应）入 contracts
    - **首批契约域**：鉴权、笔记基础（create/read/search）、实体域（projects / stages / tags / items / events / todos / timeline_groups / templates）、捕获域（capture / card）
    - 交付**可运行 Mock 服务**：读 OpenAPI + 场景数据集（`contracts/mock/scenarios/`），支持 SSE 流式模拟、延迟注入、错误注入
  - **预期交付物**：monorepo 骨架与 CI；契约源 + 生成管线 + `contracts:check`；错误码 / 枚举 / 鉴权约定文档；Mock 服务（`pnpm mock`）+ 首批场景数据集；开发环境约定（端口 / token / 数据目录）
  - **验证方式**：
    ```bash
    pnpm contracts:build   # 生成 openapi.json / app 前端类型 / sidecar 工具 Schema
    pnpm contracts:check   # CI diff 检查：生成物与契约源一致（零 diff 通过）
    pnpm mock              # 启动契约 Mock（127.0.0.1:4766）
    curl -s http://127.0.0.1:4766/api/projects -H "Authorization: Bearer dev-token"
    # → 200，返回契约样例数据（ULID / ISO-8601 字段抽查符合 Wire 纪律）
    curl -s "http://127.0.0.1:4766/api/search?q=oracle" -H "Authorization: Bearer dev-token"
    # → 200，带 snippet / provenance 的命中列表
    curl -sN http://127.0.0.1:4766/api/stream -H "Authorization: Bearer dev-token"
    # → 依次收到 3 条 SSE 模拟事件后正常关闭
    curl -s http://127.0.0.1:4766/api/projects   # 无 token → 401
    ```

- [ ] **Phase-2 [Shared] 最小垂直切片** —— 打通 前端 → API → 数据库 完整链路

  - **依赖**：Phase-1
  - **上下文**：本项目无注册登录（本地单用户 + localhost token），故切片选**最简笔记路径**：创建笔记 → 落盘 vault → SQLite 索引 → 检索命中 → 前端可见。目的不是做功能，而是尽早暴露环境、代理、序列化、鉴权、SSE、多窗口数据同源等集成层面的问题——这些是本项目全部后续阶段的公共底座。
  - **核心目标**：
    - `src-tauri/` Rust 主进程骨架：axum 监听 127.0.0.1:4765 + Bearer token 校验、SQLite 迁移、vault 目录初始化（按 SPEC §3.1 布局）、最小三个 API（create_note / read / search）+ 一个最小 SSE 端点
    - `sidecar/` Bun 工程骨架：可被主进程经 stdio 拉起、健康检查 ping-pong、能 HTTP 回调主进程（验证 ADR-2 通道）
    - `app/` React + Tauri v2 骨架：单页面（笔记列表 + 创建表单 + 搜索框）、api-client（openapi-fetch）、EventSource 通路
    - 一键启动 `pnpm dev`（三进程拉起 + 就绪探测）与 `.env` 约定（端口 / token / `JOTLINE_DATA_DIR` / `DEEPSEEK_API_KEY` 占位）
  - **预期交付物**：三端可运行骨架；`pnpm dev` 启动链；环境变量与启动文档（README「本地开发」节）
  - **验证方式**：
    ```bash
    pnpm dev    # 一键启动 Rust 主进程(:4765) + sidecar 壳 + 前端(:1420)
    curl -s -X POST http://127.0.0.1:4765/api/notes \
      -H "Authorization: Bearer dev-token" -H "Content-Type: application/json" \
      -d '{"project_id":"prj_seed","title":"切片验证","body":"hello jotline"}'
    # → 201，返回 {"id":"itm_01J…"}（ULID、时间 ISO-8601 UTC）
    ls "$JOTLINE_DATA_DIR/notes/"        # 出现 itm_01J….md 文件
    sqlite3 "$JOTLINE_DATA_DIR/index.sqlite" "select id from items;"   # 索引行存在
    curl -s "http://127.0.0.1:4765/api/search?q=hello" -H "Authorization: Bearer dev-token"
    # → 命中刚创建的笔记
    curl -s http://127.0.0.1:4765/api/notes -H "Authorization: Bearer wrong"   # → 401
    ```
    界面路径：打开主窗 → 创建表单输入「切片验证」→ 保存 → 列表出现该笔记 → 搜索框输入 `hello` → 命中显示。造数：以上 curl 即造数；界面操作与其等价。

### 轨道 [BE-Core]：Rust 主进程

- [ ] **Phase-3 [BE-Core] 存储与领域 API 基座** —— 实体、真相源与检索

  - **依赖**：Phase-2
  - **上下文**：SPEC §4 数据模型与 §3.1 数据目录布局已冻结，本阶段把它们变成可调用的领域 API。**中文 FTS 方案（DEC-02）在本阶段 spike 实测定稿**（trigram vs jieba 分词 vs 双方案并用，以真实中文样本实测检索命中与延迟 <1s 为准）。网关统一织入（SPEC §5）中 provenance、执行权来源校验、本地埋点（track_events）在本阶段以中间件落地；审计织入留 Phase-7。
  - **核心目标**：
    - vault 完整布局落地：notes / attachments（内容寻址）/ inbox / projects / memory / templates / audit.jsonl 占位 / index.sqlite
    - SQLite 全量表迁移（SPEC §4.3 清单）+ 文件真相源读写 + **索引可重建**（`pnpm rebuild-index`：删除 index.sqlite 后从文件全量重建）
    - 实体域 API：list_projects / upsert_tags / add_timeline_event（含 at_source 强制校验）/ event.void / timeline_group.* / todo.*（基础二态，DEC-09 状态机细化留待办中心设计稿）/ 模板 bundle 加载与 `pnpm seed`（内置「产品实施交付」8 阶段模板 + 示例项目 prj_ams）
    - search_content 实现（FTS5 + 元数据过滤，接口承诺签名不变）+ OCR 文本入索引的预留
    - 网关中间件：token 鉴权（切片已有）、执行权来源校验、provenance 织入、track_events 埋点
  - **预期交付物**：存储层与全部实体 API；seed 命令与首套模板 bundle；中文 FTS 方案 IDR；重建命令
  - **验证方式**：
    ```bash
    pnpm seed     # 载入模板 + 创建示例项目 prj_ams
    curl -s http://127.0.0.1:4765/api/projects -H "Authorization: Bearer dev-token"   # → prj_ams 可见，含阶段序列
    curl -s -X POST …/api/notes -d '{…prj_ams 下创建笔记…}'          # 同 Phase-2 方式
    curl -s -X POST …/api/tags -d '{"name":"oracle"}'                 # → 标签池新增
    curl -s -X POST …/api/events -d '{"project_id":"prj_ams","type":"会议","at":"2025-06-12T06:32:00Z"}'
    # → 400：非捕获时间且缺 at_source（Wire 校验生效）
    curl -s -X POST …/api/events -d '{…,"at_source":"content_time"}'  # → 201
    curl -s -X POST …/api/events/<id>/void                             # → 时间线查询中该事件带 voided 标记
    curl -s "…/api/search?q=oracle&project_id=prj_ams" -H "Authorization: Bearer dev-token"   # 过滤命中
    rm "$JOTLINE_DATA_DIR/index.sqlite" && pnpm rebuild-index
    curl -s "…/api/search?q=oracle" -H "Authorization: Bearer dev-token"   # 重建后仍命中（库=索引非真相源）
    ```

- [ ] **Phase-4 [BE-Core] OCR 服务** —— `/api/ocr` 单图 + 批量任务

  - **依赖**：Phase-2（与 Phase-3 可并行）
  - **上下文**：PRD §7.1 架构中 OCR 位于 Rust 主进程内（ADR-4），接口承诺已冻结（带 bbox 行流）。macOS 用 Vision 引擎（零模型分发）。**DEC-04 bbox 坐标系、DEC-05 低置信阈值在本阶段结合 spike 定稿**。批量 jobs 的优先级队列（浮窗单图 > 后台批量、并发 2–4、失败重试、断点续跑）在本阶段实现。
  - **核心目标**：
    - `POST /api/ocr` 单图同步：返回 `lines[{text, confidence, bbox, reading_order}] + engine/版本`
    - `POST /api/ocr/jobs` + `GET /api/ocr/jobs/<id>`：批量任务、并发限制、重试、断点续跑、优先级队列
    - ocr_cache（key = 图片 SHA-256，同图免二次 OCR）
    - OCR 文本写回捕获上下文供后续入索引（衔接 Phase-5/Phase-3）
    - 服务仅 localhost + 进程 token（NFR）
  - **预期交付物**：OCR 服务全部接口与队列；bbox / 阈值 IDR；fixtures 样例图集（`src-tauri/fixtures/ocr/`，含清晰 / 低置信 / 多栏样例）
  - **验证方式**：
    ```bash
    screencapture -x /tmp/ocr-test.png    # 造数：截一张含文字的屏幕
    curl -s -X POST http://127.0.0.1:4765/api/ocr -H "Authorization: Bearer dev-token" -F "image=@/tmp/ocr-test.png"
    # → {"lines":[{"text":"…","confidence":0.98,"bbox":[…],"reading_order":1}…],"engine":"vision","elapsed_ms":…}
    #   单图 <3s（NFR 粗测）
    curl -s -X POST …/api/ocr/jobs -H "Authorization: Bearer dev-token" -H "Content-Type: application/json" \
      -d '{"images":["/tmp/ocr-test.png","/tmp/ocr-test.png"]}'
    # → {"job_id":"…"}；GET …/api/ocr/jobs/<job_id> 轮询至 status=done，results 含两图行流
    # 缓存命中：同图第二次单图调用，日志出现 ocr_cache hit（或 elapsed 显著下降）
    ```

- [ ] **Phase-5 [BE-Core] 捕获管线与操作卡片（主进程侧）** —— 通道 A 的骨架与执行

  - **依赖**：Phase-3、Phase-4
  - **上下文**：捕获管线横跨主进程（确定性预处理、卡片存储、执行入库）与 sidecar（Agent 结构化）。本阶段只做**主进程侧**，用 **stub agent**（按 stdio 契约返回固定结构化卡片）代替真实 sidecar 验证全链——这正是契约先行的并行 seam。**DEC-03 文件夹拖入行为在本阶段设计稿定稿**。降级总原则（LLM 不可达 / 解析失败 → 原文入 inbox，永不阻塞捕获）在本阶段落地。
  - **核心目标**：
    - `capture.submit`：输入路由（文本直通 / 图片→OCR / 文件→存储+SHA-256 去重）；capture 状态机（submitted → processing → card_ready / degraded）
    - 投递聚合：10s 窗口连续多图归并单捕获单元（OCR 行流按序拼接 + 重叠区去重，origin 标注「截图 ×N」）
    - 时间锚行正则提取（`昨天 14:30` 等）喂时间回退链①（纯规则，不做说话人归因）
    - `card.*`：待审 / 已执行 / 已丢弃状态机、对话调整落草稿（不写记忆）、`card.execute` 入库（create_note + add_timeline_event + todos 创建 + FTS 索引，provenance.processing 事实写入）
    - inbox 降级落点与待处理队列；stub agent 模式（`AGENT_MODE=stub` / `AGENT_MODE=offline`）
  - **预期交付物**：捕获与卡片全部主进程 API；聚合与降级路径；stub 模式；DEC-03 IDR
  - **验证方式**：
    ```bash
    AGENT_MODE=stub pnpm dev
    curl -s -X POST …/api/capture -H "Authorization: Bearer dev-token" -H "Content-Type: application/json" \
      -d '{"type":"text","text":"昨天和客户开了UAT评审会，会上确认周四前要部署新版本"}'
    # → {"capture_id":"cap_01J…"}；轮询 GET …/api/captures/cap_01J… → status=card_ready，关联卡片 crd_…
    curl -s "…/api/cards/crd_…" -H "Authorization: Bearer dev-token"    # 卡片结构（摘要/事件/待办/建议）
    curl -s -X PATCH …/api/cards/crd_… -H "Authorization: Bearer dev-token" \
      -H "Content-Type: application/json" -d '{"entries":[{"target_dir":"会议纪要/"}]}'   # 对话调整
    curl -s -X POST …/api/cards/crd_…/execute -H "Authorization: Bearer dev-token"
    # → vault/notes 新笔记、events 新增"会议"事件（at_source=content_time）、todos 新增"周四前部署"待办
    # 投递聚合：10s 内连续 POST 两张图片 capture → 单 capture、单卡片 entries×2、origin="截图 ×2 · 已按顺序合并"
    AGENT_MODE=offline pnpm dev   # 重启后提交任意文本
    # → vault/inbox/ 出现原文转储，capture status=degraded（不阻塞、不报错给捕获方）
    ```

- [ ] **Phase-6 [BE-Core] 笔记深加工** —— 追加、合并、版本链

  - **依赖**：Phase-3（与 Phase-4 / Phase-5 可并行）
  - **上下文**：note.append / merge_draft / merge / revisions / revert 与 item_sources 分节溯源、note_revisions 快照是 SPEC 冻结的协作安全机制，也是「Agent 即接口」下对话改写笔记的安全底座。**前置：DEC-14 模板内容清单（含 doc_kind 标记）需先评审定稿**（叙事型仅追加 / 资产型结构化合并的判定依据）；合并安全协议细则随本阶段设计稿定稿。
  - **核心目标**：
    - note.append（追加节）+ note.merge_draft（生成合并建议 diff）+ note.merge（执行合并，Agent 触发必快照）
    - note.revisions（版本列表）/ note.revert（回滚）；`.revisions/` 有限保留策略
    - item_sources 分节溯源：合并 / 追加后每节一条（capture_id、section_anchor、origin、at）
    - 叙事型 / 资产型 doc_kind 行为分流（DEC-14）
  - **预期交付物**：笔记深加工 API 全集；快照与回滚；分节溯源；DEC-14 模板内容清单 IDR
  - **验证方式**：
    ```bash
    # 承接 Phase-3 造数（pnpm seed 后已有一篇 prj_ams 笔记 itm_…）
    curl -s -X POST …/api/notes/itm_…/append -H "Authorization: Bearer dev-token" \
      -d '{"body":"## 补充\n今天确认了回滚方案"}'          # → 笔记追加一节
    curl -s -X POST …/api/notes/itm_…/merge_draft -H "Authorization: Bearer dev-token" \
      -d '{"source":"…另一篇笔记 id…"}'                    # → 返回合并建议 diff
    curl -s -X POST …/api/notes/itm_…/merge -H "Authorization: Bearer dev-token" -d '{…确认参数…}'
    ls "$JOTLINE_DATA_DIR/notes/.revisions/itm_…/"         # 覆写前快照存在
    curl -s …/api/notes/itm_…/revisions -H "Authorization: Bearer dev-token"   # → 版本列表
    curl -s -X POST …/api/notes/itm_…/revert -d '{"revision_id":"…"}'          # → 正文回滚为快照内容
    sqlite3 "$JOTLINE_DATA_DIR/index.sqlite" \
      "select capture_id, origin from item_sources where item_id='itm_…';"     # → 多来源分节行
    ```

- [ ] **Phase-7 [BE-Core] 凭证库、审计、日志与记忆账本** —— 安全与账本横切面

  - **依赖**：Phase-3、Phase-5
  - **上下文**：三块横切能力集中在主进程：凭证三约束（SPEC §5 冻结）、审计账本（ADR-11 / §5.2）、三运行时日志（ADR-10 / §5.1）、记忆账本（§4.5 的机械动作部分：diff 检测、覆盖/追加、原子落盘、tags 过滤）。**DEC-06/07/12（凭证恢复码 / 剪贴板策略 / 是否开放查看明文）与日志实施细节随本阶段设计稿 + 安全评审定稿**。sidecar 的 stderr 通道在本阶段完成主进程侧解析，真实 sidecar 接入在 Phase-8 后回归。
  - **核心目标**：
    - 凭证库：keyring 抽象（macOS Keychain）、加密库（SQLCipher / libsodium 二选一，IDR 定稿）、credential.* API、主密码恢复路径
    - 凭证三约束：list 响应 schema 层面无 secret 字段；reveal 明文只进剪贴板（主进程写入，不回流前端 / 对话 / 日志）；confirm_token 仅 UI 确认动作可签发
    - 审计：vault/audit.jsonl append-only + fsync；两类动作（reveal_credential 带 source、sensitive_toggle 带 {from,to}）；fail-closed（审计写失败拒绝 reveal）
    - 日志：tracing 渲染落盘（单文件按天轮转保留 14 天）、`POST /api/log` 前端批量上报、sidecar stderr NDJSON 解析注入；级别语义与隐私纪律（scrubber 输入边界）
    - 记忆账本（主进程侧）：memory.* API、card.execute 的代码 diff 检测、temp+rename 原子写、落库路径 tags 确定性过滤
  - **预期交付物**：凭证库与三约束；审计账本与 fail-closed；日志三通道（主进程侧全量 + sidecar 解析）；记忆账本 API；相关 IDR（加密库选型 / 恢复码交互 / 剪贴板策略）
  - **验证方式**：
    ```bash
    curl -s -X POST …/api/credentials -H "Authorization: Bearer dev-token" -H "Content-Type: application/json" \
      -d '{"project_id":"prj_ams","name":"UAT数据库","username":"ops","secret":"Sup3rS3cret","url":"jdbc://…"}'
    # → {"id":"cred_…"}
    curl -s …/api/credentials -H "Authorization: Bearer dev-token"    # 列表：无 secret 字段（响应 JSON 里不存在该键）
    curl -s -X POST …/api/credentials/cred_…/reveal -H "Authorization: Bearer dev-token" \
      -d '{"confirm_token":"…"}'                                     # 经 UI 确认流签发的 token
    pbpaste    # → Sup3rS3cret（仅剪贴板可见，响应体不含明文）
    tail -1 "$JOTLINE_DATA_DIR/audit.jsonl"
    # → {"id":"aud_…","action":"reveal_credential","source":"main","target_id":"cred_…",…}
    curl -s -X PATCH …/api/projects/prj_ams -d '{"sensitive":true}'
    tail -1 "$JOTLINE_DATA_DIR/audit.jsonl"   # → sensitive_toggle，detail {"from":false,"to":true}
    # fail-closed：
    chmod 444 "$JOTLINE_DATA_DIR/audit.jsonl" && curl … reveal …   # → 请求被拒 + app.log 出现 ERROR；chmod 644 恢复后可取
    # 记忆：AGENT_MODE=stub 下执行一张与初始建议有差异的卡片（改 target_dir 后 execute）
    cat "$JOTLINE_DATA_DIR/projects/prj_ams/memory.json"   # → 出现新条目 {id,text,explicit,created_at}
    # 日志：
    tail -f ~/Library/Logs/<bundle-id>/app.log    # → rust.* / web.* 前缀行；中文 message + 英文 fields；
                                                  #    POST /api/log 上报后出现 web. 行（sidecar. 行待 Phase-8 接入后回归）
    ```

### 轨道 [BE-Agent]：Node Sidecar（pi agent）

- [ ] **Phase-8 [BE-Agent] Sidecar 基座与 Provider 层** —— 进程模型、双模型与脱敏闸门

  - **依赖**：Phase-2（与 [BE-Core] 各阶段可并行）
  - **上下文**：sidecar 的进程约束已被 ADR-2 / ADR-9 / ADR-12 冻结（零监听、stdio 驱动、构建期内置资源、关闭目录发现）。本阶段把「能跑、能出话、能脱敏」做实。**DEC-08（脱敏匹配数据载入方式）在 provider 层开发前定稿**：凭证精确匹配数据经领域 API 拉取，并行开发期以契约 Mock 数据驱动，Phase-7 完成后接真实源，Phase-19 联调验收。
  - **核心目标**：
    - Bun 编译单可执行文件（externalBin 形态，纯 JS 无原生 addon）+ 主进程拉起 / 健康检查 / 崩溃重启的时序骨架
    - pi agent 运行时装载：extensionFactories 注册位、noExtensions / noSkills / noPromptTemplates / noContextFiles 全关（ADR-12 资源加载边界）
    - provider 层：DeepSeek（OpenAI 兼容端点）+ Ollama 双通道切换；stderr NDJSON 日志输出（不自渲染）
    - **出站脱敏闸门**：provider 出口的确定性函数（秘密模式正则 + 凭证精确匹配接口），可独立调用测试
  - **预期交付物**：sidecar 构建产物与生命周期骨架；provider 双通道；脱敏闸门（正则部分完整 + 凭证匹配经接口注入）；driver 脚本 `pnpm sidecar:chat`；DEC-08 IDR
  - **验证方式**：
    ```bash
    pnpm --filter sidecar build       # Bun 单文件产物生成（sidecar/dist/）
    pnpm sidecar:chat "你好"           # driver：stdio 拉起 → 走 DeepSeek → 流式响应逐 token 打印
    # 脱敏闸门（正则部分 + Mock 凭证源）：
    pnpm sidecar:chat --redact-audit "请复述：数据库密码是 Sup3rS3cret，token=sk-abc123xyz"
    # driver 打印出站 payload：Sup3rS3cret / sk-abc123xyz 均被替换为 <redacted:…>，出站与日志无原文
    # 本地通道：
    PROVIDER=ollama pnpm sidecar:chat "你好"    # → 响应来自本地 Ollama，日志 provider=ollama
    # sidecar 运行期间 stderr 输出 NDJSON 行 {"ts","level","target","msg","fields"}
    # → 由主进程渲染进 app.log（target 注入 sidecar. 前缀；Phase-7 日志通道回归）
    ```

- [ ] **Phase-9 [BE-Agent] Agent 会话、工具面与提案机制** —— 通道 B 的完整语义

  - **依赖**：Phase-8
  - **上下文**：ADR-13 冻结内置工具面（read / edit / write / grep / find / ls，bash / powershell 关闭）与 tool_call 门（圈定 vault）；ADR-8 冻结双通道确认模型。本阶段实现交互式会话（通道 B）：写操作走提案-行内确认。提案机制细节与工具分级表随设计稿定稿（SPEC 附录 A 对应项）。
  - **核心目标**：
    - 会话管理：多轮上下文、流式输出、会话独立于窗口生命周期（ADR-9）
    - 六工具实现与 tool_call 门：路径解析圈定 `$JOTLINE_DATA_DIR/vault` 内；越权路径拒绝
    - 通道 B 提案：写操作（edit / write）生成 proposals（参数快照 + 过期时间），经 `proposal.confirm` 确认执行；网关校验执行权来源
    - SSE 会话流经主进程转发到前端（通路验证，UI 在 Phase-13 消费）
  - **预期交付物**：会话与工具面运行时；提案机制；`pnpm sidecar:session` 交互 driver；提案/分级 IDR
  - **验证方式**：
    ```bash
    pnpm sidecar:session    # 交互式会话（真实 pi agent + 工具面 + 真实领域 API 回调）
    > 找一下 vault 里提到 oracle 的笔记，读给我
      # agent 依次调用 grep / find / read，返回笔记内容（driver 可见工具调用轨迹）
    > 读 /etc/hosts
      # 拒绝：tool_call 门——路径在 vault 之外，工具返回越权错误而非文件内容
    > 执行 ls -la
      # 拒绝：bash/powershell 已关闭，agent 无此工具可用（工具面清单核对）
    > 在「运维笔记」里追加一行：下周二现场巡检
      # → 返回待确认提案；GET …/api/proposals → pending；
      # POST …/api/proposals/<id>/confirm → 笔记更新，provenance 记录提案来源
    ```

- [ ] **Phase-10 [BE-Agent] 捕获结构化与记忆管线** —— 通道 A 的 Agent 侧

  - **依赖**：Phase-8（与 Phase-9 可并行；若结构化复用 Phase-9 的运行时集成则顺序执行）
  - **上下文**：实现「任何输入 = 原件 + 摘要 Markdown + 事件[] + 待办[]」的统一解析（PRD §4.4）：操作卡片 JSON Schema 约束输出、修复重试与降级、时间回退链、group_name 建议、凭证形态检测转 save_credential、长文本分块、投递聚合下游消费；以及记忆管线的 sidecar 侧（renderMemoryPrompt 纯函数 + 轻量 LLM 提取）。记忆提取 prompt 模板、显式规则识别标准、渲染文案随设计稿定稿（SPEC 附录 A 记忆实施细节项）。
  - **核心目标**：
    - 结构化请求处理（消费 Phase-5 的 stdio 契约）：OCR 行流 + 记忆注入 → 卡片 JSON（摘要 / 事件 with at_source / 待办 / 目录标签建议 / group_name / 通道标注 processing）
    - JSON Schema 校验失败 → 自动修复重试 1 次 → 仍失败降级原文（交主进程入 inbox，Phase-5 能力）
    - 提取规则落地：已发生→Event（时间指称 + 事实陈述）、未来→Todo（due_at 解析失败置 null，宁缺勿滥）、时间锚行、说话人不自动归因
    - 凭证形态正则 → action 改 save_credential + 原文段替换占位符
    - 长文本（>8K）分块（~4K）逐块提取合并单卡片多 entries；超时降级
    - renderMemoryPrompt 纯函数（explicit 加【规则】前缀、尾句「以当前说法为准」、空记忆零输出）；card.execute 差异触发的轻量 LLM 提取（target_id / text / explicit / forbid_tag）
  - **预期交付物**：结构化管线；卡片 schema 校验与降级回路；记忆渲染与提取；fixtures 样例输入集（`sidecar/fixtures/capture/`：含时间指称文本 / 密码文本 / 长文本 / 时间锚行 OCR 样例）；相关 IDR
  - **验证方式**：
    ```bash
    pnpm sidecar:capture sidecar/fixtures/capture/uat-incident.txt
    # → 打印卡片 JSON：摘要 + 事件(at_source=content_time) + 待办 + 目录/标签/group_name 建议；schema 校验通过
    pnpm sidecar:capture sidecar/fixtures/capture/with-password.txt
    # → action=save_credential；正文出现 [🔒 …](credential://…) 占位符，原文密码段被替换
    pnpm sidecar:capture sidecar/fixtures/capture/long-8k.txt
    # → 单卡片多 entries（card.batch 结构）
    CAPTURE_FORCE_SCHEMA_FAIL=1 pnpm sidecar:capture sidecar/fixtures/capture/uat-incident.txt
    # → 日志可见修复重试 1 次 → 仍失败 → 降级原文输出（主进程 inbox 落点由 Phase-5 验证）
    pnpm sidecar:render-memory "$JOTLINE_DATA_DIR"
    # seed 一条 explicit + 一条普通记忆后：输出自然语言区块，含【规则】前缀行 + 尾句「以当前说法为准」；
    # 清空 memory 文件后：零输出（不渲染占位）
    ```

### 轨道 [FE]：前端（主窗 + 浮窗，同一工程）

> 全轨在 Mock 服务（127.0.0.1:4766）上开发验证；设计基座（色板 / 字阶 / 组件 token）取自 DESIGN.md（Cool Mist）与 docs/design/，Phase-11 落地为代码 token。

- [ ] **Phase-11 [FE] 应用骨架与多窗口体系** —— 工程底座与双窗口同源

  - **依赖**：Phase-1、Phase-2
  - **上下文**：前端是单一 React + Tauri v2 工程，主窗与浮窗共用组件与状态层（PRD §9）。本阶段建立工程底座：路由、api-client（openapi-fetch，全部请求走契约生成类型）、SSE 订阅层、设计 token。多窗口数据同源（ADR-3：状态全部来自领域 API）在本阶段以 Mock 验证。
  - **核心目标**：
    - 工程骨架：React + Vite + Tauri v2、路由与主窗布局（项目侧栏 + 内容区）、api-client + token 注入 + SSE 订阅 hook
    - 多窗口体系：主窗 + 浮窗两个 Webview 窗口的创建 / 管理（窗口壳，浮窗三态内容后续阶段填充）
    - 设计 token 落地：DESIGN.md 色板 / 字阶 / 圆角 / 间距转 CSS token，基础组件（按钮 / 输入 / 列表项 / chip）
    - Mock 连接模式（`pnpm dev:web` → :4766）与真实模式的环境切换
  - **预期交付物**：前端工程骨架；双窗口管理；设计 token 与基础组件；api-client / SSE 层
  - **验证方式**：
    ```bash
    pnpm mock &        # 后台跑 Mock
    pnpm dev:web       # 前端连 Mock 启动（:1420）
    pnpm --filter app typecheck   # 契约生成类型与使用处一致，零手写漂移
    ```
    界面路径：主窗打开 → 项目侧栏列出 Mock 场景项目（prj_ams 等）→ 点击切换项目内容区；菜单「新建浮窗」（或开发入口）→ 浮窗窗口出现 → 其展示的列表数据与主窗同源（同 Mock 数据、操作后双窗刷新一致）。

- [ ] **Phase-12 [FE] 随手浮窗与系统集成** —— 全局唤起、窗口行为与空态仪式

  - **依赖**：Phase-11
  - **上下文**：浮窗是产品门面（PRD §4.1）：全局快捷键（⌥Space / ⌥Q 双绑定、设置页可改键）、托盘、顶部居中落位、拖拽记忆、置顶钉住、三态切换（⌥1/2/3，macOS 必须 `e.code` 判定——PRD §7.4 说明 8）、剪贴板图片自动预览、空态「看到什么，截什么」+ 首次三步卡。本阶段交付窗口行为与三态框架，三态的具体内容（卡片 / 问答 / 待办）由 Phase-13/15/16/17 填充。**DEC-10（浮窗「查」会话策略）随本阶段设计稿定稿**。
  - **核心目标**：
    - 全局快捷键注册（双绑定 + 改键通路数据结构）、托盘常驻（点击同效、角标计数接口、tooltip 三键仪式文案）、系统通知
    - 浮窗窗口行为：顶部居中默认落位、标题栏拖拽 + 位置持久化、置顶钉住（钉住时 Esc 不收起）、Esc 收起（输入中先失焦）
    - 三态框架：⌥1/2/3 切换（e.code）、记 / 查 / 看三个容器组件 + 空态文案（含「查」「看」空态）
    - 剪贴板图片检测 → 唤起自动进入投递预览（缩略图 + Enter 确认 / Esc 丢弃，确认后投递到 Mock capture）
    - 首次三步卡（任意操作后消失）与记态空态引导
  - **预期交付物**：系统集成全套（快捷键 / 托盘 / 通知）；浮窗窗口行为与三态框架；投递预览；空态与三步卡
  - **验证方式**（界面操作路径，Mock 模式）：
    1. 任意应用中按 `⌥Space` → 浮窗唤起，默认屏幕顶部居中，可交互延迟目测 <300ms；`⌥Q` 同效
    2. 按 `⌥1 / ⌥2 / ⌥3` → 记 / 查 / 看三态切换（切中文输入法状态下重复一遍，仍生效——e.code 判定）
    3. 焦点在输入框时按 `Esc` → 先失焦；再按 `Esc` → 浮窗收起；原应用焦点恢复
    4. 拖动浮窗到屏幕左上 → 收起重启应用 → 再唤起位置保持
    5. 开启钉住 → 按 `Esc` 不收起；关闭钉住 → `Esc` 收起
    6. `⌘⇧⌃4` 截取任意区域（截图进剪贴板）→ `⌥Space` 唤起 → 自动出现投递预览缩略图 → `Enter` 确认（Mock 记录 capture 调用）/ `Esc` 丢弃
    7. 托盘图标常驻；hover tooltip 出现「⌘⇧⌃4 截图 → ⌥Space 唤起 → ⌘V 粘贴」；点击托盘同效唤起
    8. 清空数据首次唤起 → 记态空态出现「看到什么，截什么 ✦」+ 三步卡；任意操作后三步卡消失

- [ ] **Phase-13 [FE] 捕获交互与操作卡片** —— 记态投递、对话调整与卡片全交互

  - **依赖**：Phase-11、Phase-12
  - **上下文**：操作卡片是「人只捕获、Agent 结构化」的确认载体（PRD §4.1 / §7.3）：摘要 Markdown、事件 / 待办行、目录标签建议、group_name 建议、通道标注（读 provenance.processing 而非设置页当前值）、多 entries、对话式调整（改草稿不落库）。本阶段同时覆盖主窗对话视图（深度模式）与浮窗「看」态（待审卡片就地执行 / 丢弃）。
  - **核心目标**：
    - 主窗对话视图：多轮对话、附件投递（拖拽文件 / 粘贴文本 / 粘贴图片）、流式输出渲染（消费 Phase-9 SSE 通路，Mock 场景驱动）
    - 操作卡片组件族：摘要 Markdown 渲染、事件行（at + at_source 标注 + 可改）、待办行、建议 chips（项目 / 目录 / 标签 / group_name）、通道标注行（`☁ deepseek-chat · 已脱敏` / `🔒 本地模型`）、多 entries 视图（「N 张截图 · 已按顺序合并」）
    - 对话调整：输入「放运维目录，加个 oracle 标签」→ 卡片字段即时更新（Mock 场景返回修订卡片）
    - 执行 / 丢弃：主窗与浮窗「看」双入口；未审角标计数（托盘 + 浮窗「看」徽标同步）
    - 说话人人工补充：卡片审查时的事件参与人编辑入口
  - **预期交付物**：对话视图；操作卡片组件族（主窗 / 浮窗共用）；调整与执行交互；角标计数；Mock 场景集（capture-card-stream / capture-card-revise / card-execute 等）
  - **验证方式**（Mock 场景驱动，界面操作路径）：
    1. 主窗对话视图粘贴一段文本 → 发送 → 流式输出逐段渲染（Mock SSE 场景）
    2. 卡片就绪 → 渲染：摘要 Markdown、事件行（黄色 at_source 标注「内容时间 · 可改」）、待办行、建议 chips、`☁ deepseek-chat · 已脱敏` 标注行
    3. 在对话输入框输入「放运维目录，加个 oracle 标签」发送 → 卡片 target_dir 变为运维目录、标签 chip 增加 oracle
    4. 事件行点击编辑 → 人工补充参与人「张工」
    5. 点「执行」→ Mock 记录 card.execute 调用（driver / Mock 控制台可见），卡片状态变已执行；未审角标计数减一，托盘与浮窗「看」徽标同步变化
    6. 浮窗「看」态：同一张待审卡片就地「执行 / 丢弃」→ 与主窗操作同效（Mock 调用记录一致）
    7. 拖入一个文件到主窗对话 → Mock 返回文件元数据卡（名称 / 类型 / 来源 / 大小）

- [ ] **Phase-14 [FE] 目录、标签与文件浏览器** —— 三视图之「放哪 / 是什么」

  - **依赖**：Phase-11
  - **上下文**：三视图信息组织（PRD §4.3）中目录（单归属）与标签（全局池、多归属）两维，加上文件浏览器（版本链、来源溯源、去重提示）。原件永不改写的展示原则在本阶段落地。
  - **核心目标**：
    - 目录树：模板目录结构渲染、展开 / 折叠、按目录筛选内容列表、单归属跳转
    - 标签系统 UI：全局池面板、多选筛选、重命名 / 合并操作、使用计数展示
    - 文件浏览器：文件 / 附件列表、版本链切换（草稿 / 评审中 / 已确认 / 已作废）、provenance 面板（方式 / 来源人 / 原始链接）、原件查看入口（图片原图 / 文件原样）
    - 去重提示：重复投递（SHA-256 / 内容哈希命中）→ 合并或跳过的选择 UI
  - **预期交付物**：目录树 / 标签 / 文件浏览器三个模块；去重交互；Mock 场景集（tag-rename / file-versions / duplicate 等）
  - **验证方式**（Mock 场景驱动，界面操作路径）：
    1. 主窗切到目录视图 → 展开 prj_ams 目录树 → 点击「问题与变更」→ 内容列表仅显示该目录条目
    2. 标签面板勾选「环境」+「数据」→ 列表显示两标签交集内容；重命名标签「环境」→「环境配置」→ 相关条目标签联动更新（Mock 场景）
    3. 打开一个文件条目 → 版本链显示 3 个版本（草稿 / 已确认）→ 切换版本内容区变化；provenance 面板显示来源「截图 · 微信群-客户群」
    4. 在对话视图再次拖入同一文件（Mock duplicate 场景返回 409 + 已有条目）→ 出现「已存在，合并 / 跳过」选择 → 选跳过后无新建

- [ ] **Phase-15 [FE] 时间线与检索** —— 三视图之「何时 / 谁」+ 找得到

  - **依赖**：Phase-11、Phase-14
  - **上下文**：时间线（PRD §4.3 / FR-4）：阶段预设折叠 + group_name 分组折叠、事件只增不改（作废标记 + 重建）、实时记录 / 导入回填区分。检索（FR-7 / §4.10）：全文（含 OCR 文本）+ 元数据过滤 + Cmd+P 切换器；浮窗「查」的快速搜索列表是轻量入口（问答深度入口在 Phase-13 已有对话容器，本阶段接搜索结果）。
  - **核心目标**：
    - 时间线视图：按阶段 / 项目周期预设折叠（模板供给，零手工）、group_name 分组折叠（无组事件不折叠）、作废标记交互（划线置灰 + 保留可见 + 溯源入口）、导入回填视觉区分
    - 检索 UI：搜索框 + 结果列表（snippet 高亮、provenance 展示）、元数据过滤（项目 / 目录 / 标签 / 时间范围）
    - Cmd+P 快速切换器：项目 / 目录 / 内容统一模糊匹配跳转
    - 浮窗「查」：关键词快速搜索命中列表（含 OCR 文本命中场景）
  - **预期交付物**：时间线视图；检索与过滤器；Cmd+P；浮窗快速搜索；Mock 场景集（timeline-groups / voided-events / ocr-hit 等）
  - **验证方式**（Mock 场景驱动，界面操作路径）：
    1. 主窗时间线 → 默认按阶段折叠 → 展开「测试验证」阶段 → 组「UAT 环境搭建」（group_name）整体折叠 / 展开；无组事件平铺不折叠
    2. 对某事件点「作废」→ 确认后条目划线置灰仍可见，时间线无物理删除；溯源面板可看
    3. 一条 imported=true 的事件带「导入回填」标记，与实时记录视觉区分
    4. `Cmd+P` → 输入「ams」→ 候选出现项目 / 目录 / 内容 → 回车跳转
    5. 搜索「oracle」→ 结果列表 snippet 中 oracle 高亮；Mock 场景里一条**仅 OCR 文本**含 oracle 的图片条目命中
    6. 浮窗「查」输入「oracle」→ 轻量命中列表（同 search_content 数据）；问句输入走 Phase-13 对话容器

- [ ] **Phase-16 [FE] 待办中心与项目工作台** —— 活数据视图与聚合入口

  - **依赖**：Phase-11、Phase-12
  - **上下文**：待办中心（FR-17 / §4.11）：独立实体、与事件生命周期分离；进行中卡片墙 + 日历视图 + 统计行 + 逐条溯源（含图片附件原图 + OCR 高亮被解析行——bbox 消费方）。工作台（FR-13 / §4.8）：今日流、项目卡片墙。托盘 / 浮窗「看」的今日到期 / 逾期聚合计数在本阶段闭环。**DEC-09（待办状态机）若超出基础二态，随本阶段设计稿定稿。**
  - **核心目标**：
    - 待办中心页：进行中卡片墙（今日到期高亮主卡 + 分类小卡 + 行卡）、统计行（进行中 / 今日到期 / 已完成 / 逾期）
    - 日历视图：按 due_at 分布、点日期查看当日清单
    - 待办操作：勾销完成 / 恢复、手动创建、状态与埋点调用（Mock 记录）
    - 溯源：来源徽章（卡片 / 纪要 / 导入 / 手动）回跳来源；图片附件查看原图 + OCR 行高亮（bbox 渲染）
    - 工作台：今日流（跨项目时间线聚合 + 查漏补记入口）、项目卡片墙（最近事件 / 待整理数 / 当前阶段）
    - 聚合计数：今日到期 / 逾期在托盘角标与浮窗「看」聚合展示，随待办状态联动
  - **预期交付物**：待办中心两视图；溯源与 bbox 高亮；工作台两模块；聚合计数；Mock 场景集（todo-wall / calendar / todo-source-ocr 等）
  - **验证方式**（Mock 场景驱动，界面操作路径）：
    1. 打开待办中心 → 卡片墙显示 Mock 场景待办；今日到期项为主卡高亮；统计行四数字与场景数据一致
    2. 切日历视图 → 点某个有 3 条待办的日期 → 当日清单展开
    3. 勾销一条待办 → 状态变完成、统计行变化、Mock 记录埋点调用；托盘角标计数减一
    4. 点一条来源为「卡片」的待办溯源徽章 → 回跳来源操作卡片（Mock todo-source 场景）
    5. 点一条图片来源待办的「查看原图」→ 原图打开 + OCR 对应解析行高亮（bbox 渲染，Mock 场景带坐标）
    6. 主窗工作台 → 今日流跨 prj_ams / prj_crm 两项目聚合显示今日事件；项目卡片墙显示当前阶段与待整理数
    7. 浮窗「看」→ 今日待办勾销入口可用，计数与主窗一致

- [ ] **Phase-17 [FE] 设置、凭证库、记忆与模板管理页** —— 保险丝与安全页面

  - **依赖**：Phase-11
  - **上下文**：设置中心（FR-13 / §6 设置行：Provider 切换、敏感项目开关、快捷键改键、待办提醒、埋点查看 / 导出 / 清除）；凭证库管理页（§4.9：列表仅元数据、占位符渲染、复制确认流——与浮窗「查」凭证 list 同源）；记忆管理页「Agent 记住了什么」（§4.2：两层展示、编辑、删除，保险丝定位）；模板管理器（FR-6：编辑、另存复用）。
  - **核心目标**：
    - 设置中心：全部开关项 + 改键交互（双绑定可改）+ 埋点查看 / 导出 / 清除（仅本地语义）+ 设置页脚三键仪式文案
    - 凭证库页：项目分组列表（无明文）、新建 / 编辑凭证、占位符 chip 预览（读凭证库当前名称实时渲染）、「复制」按钮 → 行内显式确认流（confirm_token 签发）
    - 浮窗「查」凭证 list：与主窗同源（同 list_credentials 数据），仅名称 / 元数据
    - 记忆管理页：全局层 + 项目层两层列表、逐条人话展示、编辑、删除
    - 模板管理器：阶段 / 目录 / 事件类型 / 标签编辑、另存为新模板
  - **预期交付物**：设置中心；凭证库页 + 浮窗凭证 list；记忆管理页；模板管理器；Mock 场景集（settings / credentials / memory-pages / templates）
  - **验证方式**（Mock 场景驱动，界面操作路径）：
    1. 设置 → 快捷键改键：把 ⌥Space 改为 ⌘⇧Space → 保存后旧键失效、新键唤起浮窗（Mock 持久化场景）
    2. 设置 → 埋点查看：列表展示 Mock 埋点事件 → 导出下载 JSON → 清除后列表为空
    3. 凭证库页：新建凭证（表单无明文回显）→ 列表出现；笔记中的占位符 chip 实时渲染为「🔒 UAT 数据库 [复制]」
    4. 点「复制」→ 行内二次确认 → Mock 记录 reveal 调用（带 confirm_token）；取消则无调用
    5. 浮窗「查」输入「凭证」→ list 命中仅名称 / 账号 / 主机（与凭证页同源同数据）
    6. 记忆管理页：全局层显示「Oracle 相关的内容都归 AMS 项目」条目 → 编辑文案 → 保存；删除一条 → 列表移除（Mock 持久化）
    7. 模板管理器：编辑「产品实施交付」目录树 → 另存为「我的实施模板」→ 新建项目可选到它
    8. 设置页脚出现三键仪式完整文案

### 轨道 [Integration]：集成、联调与发布

- [ ] **Phase-18 [Integration] 捕获链路端到端联调（I1）** —— 从 Mock 切真实，走通产品灵魂流程

  - **依赖**：Phase-5、Phase-10、Phase-13、Phase-14、Phase-15、Phase-16
  - **上下文**：前端从 Mock（:4766）切换到真实后端（:4765），首次全链路真实运行：浮窗记态 → 真实 OCR（Vision）→ 真实 DeepSeek 结构化 → 操作卡片 → 执行入库 → 三视图 / 待办 / 检索四处可见。本阶段消化 Mock 与真实行为的全部差异（延迟、LLM 不确定性、SSE 节奏、错误路径）。性能预算在此首次全链路粗测（NFR：浮窗唤起 <300ms、单图 OCR <3s、卡片就绪 p50 ≤10s）。
  - **核心目标**：
    - 真实链路切换与问题修复（序列化 / 时序 / 重试 / SSE 重连 / 降级路径真实触发）
    - 完整用户旅程走查（见验证方式）作为整体验收
    - 三键仪式文案五处出现核对（浮窗空态 / 首次三步卡 / 托盘 tooltip / 卡片标注行 / 设置页脚）
    - 浮窗与主窗操作一致性抽查（「看」执行 vs 主窗执行，同卡片同结果）
    - 性能冒烟与热点定位；契约文档与实现的一致性核对与修订（第一轮）
  - **预期交付物**：真实链路可用的应用；联调问题清单与修复；性能冒烟记录；契约修订
  - **验证方式**（真实后端 + 真实 DeepSeek + 真实 OCR；造数：DEEPSEEK_API_KEY 配置好即可，无需种子数据——旅程本身即造数）：
    1. `⌘⇧⌃4` 截取一段微信聊天窗口（含文字与时间信息）→ `⌥Space` 唤起 → `⌘V` → 投递预览出现缩略图 → `Enter` 确认 → `Esc` 收起（目测 <1s 可收起）
    2. 等待托盘通知 + 角标 → `⌥3` 进「看」→ 待审卡片就绪：OCR 提取的文字进入摘要、事件行带 at_source 标注、`☁ deepseek-chat · 已脱敏` 标注行出现
    3. 对话调整「放运维目录，加个 oracle 标签」→ 卡片字段更新
    4. 点「执行」→ 依次核对四处：目录树「问题与变更」出现新笔记；时间线出现新事件（阶段折叠正确）；待办中心出现解析的待办；搜索 OCR 中的关键词命中该条目
    5. 投递聚合：10s 内连截 3 张图投递 → 单卡片 entries×3、「截图 ×3 · 已按顺序合并」
    6. 降级路径真实触发：断网（或 kill sidecar）后投递文本 → inbox 出现原文、无报错弹窗；恢复后正常
    7. 性能粗测：浮窗唤起可交互 <300ms；单图 OCR <3s；文本卡片就绪 p50 ≤10s（各 5 次取样）
    8. `pnpm contracts:check` 零 diff；卡片 JSON 与 contracts schema 一致

- [ ] **Phase-19 [Integration] 查询、凭证、记忆与安全链路联调（I2）** —— 三约束、审计与越用越准的真实闭环

  - **依赖**：Phase-7、Phase-9、Phase-10、Phase-17、Phase-18
  - **上下文**：捕获链路（I1）之外的三条硬链路：①「查」问答（US-3：附原文引用与时间、工具调用 ≤5 次）；②凭证全旅程（US-4 + 凭证三约束 + 审计 + 脱敏闸门人工冒烟——NFR 要求功能上线前冒烟）；③记忆闭环（US-2：纠正一次即记住、下次生效）。主窗 / 浮窗同权（审计 source 字段区分）在此验证。脱敏闸门的凭证精确匹配接真实凭证库（Phase-8 的 DEC-08 收口）。
  - **核心目标**：
    - 「查」旅程真实走查（浮窗轻量 + 主窗深度双入口，共用 search_content）
    - 凭证旅程真实走查（捕获密码文本 → save_credential → 占位符 → 复制确认 → 剪贴板 → 审计）
    - 敏感开关 ON → Ollama 全链路本地处理 → 卡片 `🔒 本地模型` 标注
    - 记忆闭环：纠正 → card.execute diff → memory 写入 → 下次同类捕获建议命中（「按你的习惯」）
    - 脱敏闸门人工冒烟：出站 payload 与 app.log 全量检查无凭证明文（scrubber 边界）
    - 契约一致性核对（第二轮）
  - **预期交付物**：三条链路可用的应用；审计与脱敏冒烟记录；记忆闭环验证记录；契约修订
  - **验证方式**（真实后端；造数：旅程①②③本身即造数）：
    1. 浮窗「查」问「上次数据库连接问题怎么解决的」→ 回答附原文引用 + 时间 + 来源；app.log 中该问答工具调用 ≤5 次
    2. 浮窗「查」凭证 list → 仅名称 / 账号 / 主机；无任何明文字段
    3. 在记态粘贴「UAT 数据库账号 ops，密码 Sup3rS3cret」→ 卡片 action 自动变为 save_credential、密码段替换为占位符 → 执行 → 凭证库出现该条目、笔记中占位符 chip 渲染
    4. 凭证页（及浮窗）点「复制」→ 行内确认 → `pbpaste` 得到 `Sup3rS3cret`；`tail vault/audit.jsonl` 出现 reveal_credential 记录，浮窗触发的 source=floating、主窗的 source=main
    5. 设置中把 prj_ams 敏感开关置 ON → `tail vault/audit.jsonl` 出现 sensitive_toggle {"from":false,"to":true} → 投递一张截图 → 卡片标注 `🔒 本地模型`（Ollama 处理，日志 provider=ollama）
    6. 记忆闭环：捕获一条 Oracle 内容 → 调整「放运维目录」→ 执行 → `cat vault/memory/global.json` 出现新条目 → 再次捕获同类内容 → 卡片建议直接命中运维目录并注明「按你的习惯」
    7. 脱敏冒烟：开启云端通道重复步骤 3 → 抓取 driver / 日志中的出站 payload → 全文检索 `Sup3rS3cret` 零命中（app.log 同查）
    8. 双通道一致性：同一张卡片在浮窗「看」与主窗分别执行各一次 → 两者的入库结果与审计 / 埋点记录一致

- [ ] **Phase-20 [Integration] 打包、签名、公证与发布** —— 可分发的 V1

  - **依赖**：Phase-18、Phase-19
  - **上下文**：第零步已确认需要发布（PRD §9：Developer ID 签名 + 公证、内部分发；NFR：updater 签名密钥发布前配置）。本阶段收尾：打包链路、签名公证、全新环境走查、契约文档与最终实现的**一致性终核与修订归档**（用户骨架要求的收尾核对）。
  - **核心目标**：
    - `pnpm tauri build` 产出 .app + .dmg（含 sidecar externalBin 打包、OCR 引擎随包、资源内置——ADR-12 开发与运行时同规则验证）
    - Developer ID 签名 + 公证（spctl 通过，免「右键打开」）
    - updater 签名密钥配置与更新包产物
    - 全新环境安装走查（另一台 Mac 或全新用户目录）：安装 → 首次三步卡 → 完成一次完整截图捕获旅程
    - 契约终核：OpenAPI 与最终实现 diff 为零；PRD/SPEC 中与实现的偏差回写文档归档
  - **预期交付物**：签名公证后的 .dmg 与更新包；发布检查单；文档一致性修订；V1 发布
  - **验证方式**：
    ```bash
    pnpm tauri build
    codesign --verify --deep --strict "src-tauri/target/release/bundle/macos/Jotline.app"   # → 有效签名
    spctl -a -vv "src-tauri/target/release/bundle/macos/Jotline.app"                        # → accepted, notarized
    ls src-tauri/target/release/bundle/dmg/    # → Jotline_1.0.0_aarch64.dmg
    pnpm contracts:check                       # → 零 diff（契约终核）
    ```
    全新环境走查：把 DMG 拷到另一台 Mac（或新用户目录）→ 双击安装直接打开（无 Gatekeeper 警告）→ 首次唤起出现三步卡 → 配置 DeepSeek key → 完成一次「截图 → 唤起 → 粘贴 → 执行」旅程 → 托盘 / 时间线 / 检索均正常。

---

## 4. 关键假设与风险

### 4.1 关键假设

1. **大纲范围 = V1 全量**（PRD §2.1 V1 行）；V2 / V3 交付后另立大纲。OCR 精度降级路径按 PRD 约定：V1 图片不可读时存原图入待处理，OCR 随 V2 补齐，主流程不受阻。
2. **单人或多人均可执行**：依赖图定义偏序，单人开发时按拓扑序（大致为编号序）串行执行同样成立；多人时 [BE-Core] / [BE-Agent] / [FE] 三线并行。
3. **开发期环境**：macOS 开发机（Vision OCR 可用）；DeepSeek API key 可用（Phase-8 起）；本机 Ollama 可用（Phase-8 起本地通道验证）。
4. **契约工具链按 SPEC 建议默认**（utoipa + openapi-typescript + openapi-fetch；Mock 为自研轻量 Node 服务以支持 SSE / 延迟 / 错误注入——Phase-1 spike 后 IDR 定稿）。若 spike 推翻选型，仅影响 contracts 管线实现，不影响阶段结构。
5. **无注册登录体系**（本地单用户 + localhost token），故垂直切片选笔记 CRUD 而非注册登录。
6. **SPEC 冻结项不再重议**（Wire 纪律、ADR-1~13、数据模型、审计 / 日志 / 记忆契约）；延期决策（DEC-02~14）在对应阶段设计稿中定稿并落 IDR——各阶段「上下文」已标注映射。
7. **测试由开发过程专用命令管理**，不进路线图；每阶段「验证方式」为硬性完成标准。
8. **发布限定 macOS**（Developer ID 签名 + 公证 + DMG 内部分发 + updater）；Windows 适配不在本大纲。

### 4.2 双轨并行模式的特有风险与应对

| 风险 | 说明 | 应对 |
|---|---|---|
| **契约漂移** | 双轨并行时 BE-Core / BE-Agent / FE 对同一接口的理解随时间发散，集成期才发现 | 契约源唯一 + 生成物 commit + CI diff 显红（Phase-1 建立）；契约变更流程铁律：先改契约 → 同步两侧与 Mock → 再改代码；Phase-18/19 两轮 + Phase-20 终核一致性核对 |
| **Mock 与真实行为不一致** | Mock 无法模拟真实 OCR 延迟、LLM 输出不确定性、SSE 断流、DeepSeek 限流——FE 阶段「验证通过」≠ 集成可用 | Mock 支持延迟 / 错误 / 流式节奏注入（Phase-1 交付要求）；FE 验证只声明「符合契约场景」；真实行为验收显式留给 Phase-18/19，并为联调预留充足轮次（各 6–8 轮） |
| **集成期才暴露跨切面问题** | 性能预算（<300ms / <1s / p50 ≤10s）、脱敏闸门、SSE 重连、多窗口状态同步，单轨验证均不覆盖 | Phase-2 切片提前打通鉴权 / 序列化 / SSE / 双窗口同源四类公共风险；Phase-18 显式列性能冒烟清单；Phase-19 显式列脱敏与审计冒烟（NFR 要求的人工检查点） |
| **pi agent 集成风险** | Bun 编译、资源加载边界（ADR-12）、工具面收窄（ADR-13）存在未知兼容性问题，若在 Phase-9/10 才发现将大范围返工 | Phase-2 即拉起 sidecar 壳（stdio + HTTP 回调通路）；Phase-8 作为 [BE-Agent] 首阶段先行验证 pi 运行时装载与 Bun 构建，风险最早暴露 |
| **sidecar 跨轨依赖错位** | 脱敏闸门的凭证精确匹配需要凭证库（Phase-7），两轨并行时序可能错位 | DEC-08 接口先行：闸门以注入数据源实现，Phase-8 用 Mock 凭证数据开发，Phase-7 完成后接真实源，Phase-19 联调收口 |
| **中文检索质量** | FTS5 默认分词对中文无效，检索 <1s 且命中质量是核心体验 | DEC-02 在 Phase-3 以真实中文样本 spike 实测定稿（接口承诺不变，方案可替换）；OCR 文本命中场景在 FE Mock 场景与 Phase-18 真实链路双重验证 |
| **LLM 结构化输出可靠性** | 卡片 JSON 校验失败率未知，降级路径若成为常态则产品体验受损 | Phase-10 实现修复重试 1 次 + 降级入 inbox；Phase-18 真实链路观察降级比例，必要时在 Phase-18 内调 prompt / schema（属阶段内迭代，不改契约结构） |
| **阶段节奏** | FE 各阶段所需 Mock 场景若滞后，会造成 FE 空转或绕过契约手造数据 | 总则第 2 条：FE 阶段开工前置 = 契约与 Mock 场景已入库；场景缺失先补契约再开发 |

---

## 5. 下一步

本大纲为蓝图，各阶段的详细子任务将在逐期对话中共同探讨生成（每阶段约 3–8 轮）。

请确认：
1. 是否认可本大纲的整体结构（轨道划分、20 个阶段、依赖关系、颗粒度）？
2. 若有调整意见（阶段拆并、依赖修正、范围增删），请直接指出。

确认后，你随时可以回复 **「开始 Phase-X 的详细规划」**（可注明轨道，如「开始 Phase-3 [BE-Core] 的详细规划」）进入该阶段的深入探讨。建议从 **Phase-1 [Shared]** 开始。
