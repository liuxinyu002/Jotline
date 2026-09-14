# Design: phase-1-contracts-skeleton

## Context

仓库现状：仅 `docs/`（PRD v2.1 / SPEC v0.7 / Roadmap / DESIGN.md）+ `openspec/`（空），**非 git 仓库，无任何代码**。约束来自 SPEC 冻结项：五目录结构（§3.2）、契约单一事实源 + 禁手写第二份（§3.2）、Wire 纪律（§3.2）、HTTP 统一业务面 + IPC 仅系统能力（DEC-01 / ADR-1）、Sidecar stdio 驱动零监听（ADR-2）。Roadmap 定位 Phase-1 为全部后续阶段的公共前置，并明确「契约漂移」为双轨并行头号风险，应对 = 契约源唯一 + 生成物 commit + CI diff 显红。

消费者地图决定详细度分配——Phase-2 立即消费契约类型 / stdio 协议 / dev 环境约定；工具 Schema 的真实消费者在 Phase-8/9；Mock 场景的大规模消费在 Phase-11~17。详见 tasks 中各交付物的优先级标注。

## Goals / Non-Goals

**Goals:**

- 防漂移机制物理成立：改契约源而不重新生成 → CI 显红；生成物确定性可 diff
- 一条管线覆盖三端消费物（openapi.json / 前端 TS 类型 / sidecar 工具 Schema / SSE 与 stdio 类型）
- Mock 支撑 Phase-11 的「双窗操作后刷新一致」验证（语义内核），同时满足 Roadmap 验证的 401 / SSE 模拟流 / 检索命中
- 开发环境约定一次定稿（端口 / token / 数据目录 / 根脚本），Phase-2 直接引用

**Non-Goals:**

- 不实现任何真实业务逻辑（领域 API / SQLite / OCR / sidecar 运行时均属 Phase-2+）
- 不做 vite / tauri 脚手架初始化（app/ 与 sidecar/ 仅 package.json + tsconfig 占位）
- 不交付 credential / memory / proposal 契约域（随对应阶段增补，Roadmap 总则 2）
- 不做 Windows 适配、不做分发（Phase-20）

## Decisions

### D1 契约源位置：`src-tauri/crates/contracts` + 根虚拟 Cargo manifest

SPEC §3.2 冻结 `contracts/ # CI 生成物`——契约源（Rust crate）放进去违背冻结描述；顶层新增 `crates/` 目录等于给冻结结构加第六目录（需先修 ADR）。唯一合规解：契约源住在 Rust workspace 内但不占 contracts/。

```
repo/
├── Cargo.toml              # 虚拟 manifest：members = ["src-tauri/crates/contracts"]
├── app/                    # 占位（package.json + tsconfig）
├── sidecar/                # 占位（package.json + tsconfig）
├── src-tauri/
│   └── crates/contracts/   # 契约源 crate（Phase-2 tauri 脚手架作为新 member 加入）
│       ├── src/lib.rs      #   类型 + utoipa 注解 + path stub + 工具注册表
│       └── src/bin/gen.rs  #   生成器（见 D3）
├── contracts/              # 纯生成物 + 协议 + Mock（commit 入库）
│   ├── openapi.json
│   ├── generated/          # TS 类型包源（@jotline/contracts）
│   ├── tools/              # 工具 Schema（自包含 JSON Schema）
│   ├── mock/               # Mock 引擎 + scenarios/
│   └── PROTOCOL.md
└── .github/workflows/contracts.yml
```

**纪律**：契约 crate 零 tauri 依赖（仅 utoipa + serde + chrono + ulid），保证 Linux CI 秒级跑 clippy 与生成。
**替代方案**：契约源住 contracts/（违背 SPEC §3.2 冻结描述，弃）；顶层 crates/（需先修 ADR 加目录，过度程序成本，弃）。

### D2 path 声明：先 stub，随实装迁移

Phase-1 无 handler，path 声明以带 `#[utoipa::path]` 注解的空函数承载；Phase-2/3/5 实装各域时注解迁至真实 handler（utoipa-axum `OpenApiRouter`）。迁移正确性证明 = openapi.json 零 diff（注解内容不变 → 产物不变）。依赖 spike S6；若 utoipa-axum 集成不顺，退化为永久 stub + CI diff 兜底——两条路都不破契约铁律，只是对齐义务从机制退化为人肉。以 IDR 记录演进策略。

### D3 一条管线覆盖 HTTP / SSE / stdio / 工具 Schema

- **SSE 信封与 stdio JSON-RPC 消息类型**注册进 OpenAPI components（即使无 path 引用），openapi-typescript 为全部 components 生成 TS——避免第二条 TS 生成链（ts-rs 等）的工具维护成本。代价：openapi.json 混入非 HTTP 类型，在 PROTOCOL.md 文档化此 quirk。
- **生成器**：契约 crate 内 `src/bin/gen.rs`（非 utoipa-cli）——需要自定义输出（工具 Schema 的 $ref deref、多文件写出、键排序控制），utoipa-cli 只输出 openapi.json 无法胜任。
- **工具注册表**：Rust 侧 `Vec<ToolSpec>` 静态表（工具名 → description + 参数 schema 类型引用），gen bin deref 展开为自包含 JSON Schema 写入 `contracts/tools/`。Phase-1 仅注册 2–3 个样例工具（list_projects / create_note / search_content）验证机制；全量工具面等 Phase-9/10 工具设计定稿。
- **TS 类型包**：`contracts/generated/` 即 `@jotline/contracts` workspace 包，app 与 sidecar 均经包名导入（sidecar 的 Bun 对 workspace 包支持良好，避免相对路径碎片化）。

### D4 Wire 纪律机械化

| 纪律 | 机制 |
|---|---|
| 禁 untagged / flatten / camelCase rename | CI 脚本 grep 契约源，命中即红 |
| 可选 = 字段省略 | `#[serde(skip_serializing_if)]` + utoipa value_type 组合，spike S2 验证生成 `field?: T` |
| ULID string | `Id<T>` newtype（ulid crate），Mock 场景数据经 Schema pattern 校验 |
| ISO-8601 UTC | `chrono::DateTime<Utc>`（serde RFC3339），utoipa chrono feature |
| snake_case | Rust 字段天然成立，零成本 |
| tagged enum | `#[serde(tag = "type")]`；含点 tag 值（`card.batch`）进 spike S1 |

### D5 Mock 形态：语义内核 + 剧本覆写

Roadmap Phase-11 验证要求「操作后双窗刷新一致」——剧本式 Mock（路由返回硬编码）结构上做不到（需为每个场景手写 SSE 时序 = 在 Mock 里手写第二份领域语义）。故：

```
Mock 引擎（contracts/mock/，轻量 Node 服务，监听 127.0.0.1:4766）
├── 路由层：读 openapi.json 匹配（契约外路由 404）+ Bearer dev-token 鉴权
├── 校验层：启动时按契约 Schema 校验场景数据，失败拒绝启动
├── 语义内核：内存状态（场景播种）+ 写操作变更状态 + SSE 事件广播 + 心跳
└── 剧本层：场景级覆写——延迟注入 / 错误注入 / 预定义响应序列
场景选择：X-Mock-Scenario 请求头（请求级）+ 控制端点 /_mock/*（全局切换/重置）
端口策略：默认常量 4766（Roadmap 开发环境约定），支持 MOCK_PORT 环境变量覆盖；.env.example 中端口为文档性占位，Mock 不强制读取
```

**边界钳制**：语义只覆盖 Phase-1 契约域内实体（projects / notes / events / todos 基础读写），不实现业务规则（推断 / 聚合 / 降级）；各 FE 阶段用场景文件增量喂。引擎框架（hono vs bare node http）为实施期选择，不进本设计。

### D6 tauri-specta 不采纳

DEC-01 关闭后 IPC 面仅剩系统能力（tauri-plugin 官方自带类型），自定义 command ≈ 0~1 个（拿 token，可由 initialization_script 替代）——生成器没有可生成的面。按 SPEC 自身决策程序（§3.2「随第一周 D3 spike 验证后以 IDR 定稿」）定稿为不采纳：SPEC §3.2 工具链行修订 + 附录 B IDR 登记。

### D7 CI：GitHub Actions，Linux runner，分层布防

```
内环（本地，秒级）：pre-commit hook 跑 contracts:check
外环（CI，2–3 分钟）：.github/workflows/contracts.yml
  job contracts: pnpm contracts:build → git diff --exit-code
                 + 场景数据 Schema 校验 + mock 冒烟（起 mock 跑 4 条 curl 断言）
  job rust:      cargo fmt --check + clippy -p contracts（D1 保证 Linux 可跑）
  job ts:        biome check + typecheck（app / mock / generated 消费侧）
锁（一次性）：     main 分支 required check = contracts job（仓库推送后开启，显式任务）
```

### D8 首批契约域详细度分层

| 域 | 详细度 | 依据 |
|---|---|---|
| 鉴权 / 错误信封 / 通用类型 | 全量定稿 | 三端公共，Phase-2 立即用 |
| 笔记基础 create/read/search | 全量定稿 | Phase-2 切片直接消费 |
| SSE 信封 + 首批事件 | 全量定稿 | Phase-2 SSE 端点用 |
| stdio JSON-RPC | 信封 + health 定稿，方法注册表预留 | Phase-2 仅需 ping-pong |
| projects / stages / tags | 全量（PRD 字段齐） | Phase-3 消费 |
| capture / card（含 PRD §7.3 卡片 schema） | 全量 | PRD 已具体；Phase-5/10 消费 |
| events / timeline_groups / templates | 骨架（核心字段） | 细节属 Phase-3 设计 |
| todos | 骨架，status 为可扩展 enum（open/done 起步） | DEC-09 未定，不冻进契约 |
| credential / memory / proposal | 不做 | 总则 2：随对应阶段增补 |

**判断标准**：PRD/SPEC 已冻结字段的 → 全量；依赖未定 DEC 的 → 留弹性；消费者两个 phase 之外 → 不做。

## Spike 清单（实施早期执行，结论落 IDR）

| # | 验证项 | 通过判据 | 不通过的退路 |
|---|---|---|---|
| S1 | utoipa tagged enum（含 `card.batch` 点号 tag）→ openapi-typescript union | 样例 payload serde↔TS round-trip 类型测试通过，narrowing 可用 | 改用无点 tag 值（card_batch），PROTOCOL.md 登记 |
| S2 | Option + skip_serializing_if 生成 `field?: T` | 生成类型无 `\| null` | 探索 utoipa value_type 显式标注；仍不行则该纪律降级并在 IDR 记录偏差 |
| S3 | chrono DateTime\<Utc\> → RFC3339 string | 生成类型为 string(date-time) | —（预期无风险） |
| S4 | openapi-fetch 消费生成类型 | app 占位工程 typecheck 通过 | 换原生 fetch + 手动 as（损失类型安全，最后手段） |
| S5 | 生成确定性 | 同源两次 gen，git diff 为空 | gen bin 显式排序键；utoipa 输出不稳定则后处理排序 |
| S6 | utoipa-axum 集成预研 | stub 注解可迁至 OpenApiRouter 且零 diff | 退化为永久 stub（D2 已备退路） |
| S7 | OpenAPI 3.0 vs 3.1 | openapi-typescript / Ajv / openapi-fetch 对 3.1 支持无坑 | utoipa 锁 3.0 输出 |

## Risks / Trade-offs

- [utoipa Option 语义坑（S2）] → spike 前置在所有域类型编写之前；退路已列
- [生成不确定性使 CI diff 失真（S5）] → spike 5 为 CI 前置；必要时 gen bin 后处理排序
- [契约 crate 住在 src-tauri 树下造成 [Shared] 轨归属混淆] → tasks 与文档明确「轨道归属看交付物不看目录」；Phase-2 tauri 脚手架加入时维持 crates/ 子目录约定
- [语义 Mock 范围蔓延（越写越像真后端）] → D8 边界钳制写进 spec（语义仅契约域内基础读写）；FE 阶段需求超出时先补契约场景文件而非扩引擎
- [OpenAPI 3.1 生态坑（S7）] → spike 定稿输出版本，锁定后写入 PROTOCOL.md
- [branch protection 需仓库推送后才能配置，易被遗忘] → tasks 显式单列（推送 GitHub 后开启 required check）

## Migration Plan

绿地项目，无迁移。实施顺序（tasks 承载细节）：git init + 骨架提交 → spike S1–S7（先于大批类型编写）→ 通用类型 + 全量域 → gen bin + 生成物 → PROTOCOL.md → Mock 引擎 + 场景 → CI + pre-commit → 文档回写（SPEC §3.2 修订 + IDR）。每步以 `contracts:build` / `contracts:check` / `pnpm mock` 验证（Roadmap Phase-1 验证命令为完成标准）。

## Open Questions

- Mock 引擎框架选型（hono vs bare node:http）——实施期定，不影响 specs（路由/校验/语义行为已由 spec 锁定）
- OpenAPI 输出版本（3.0 vs 3.1）——spike S7 定
- 首批 SSE 事件集的具体条目（card.created 等）——随 Phase-1 契约域编写定稿，注册表机制已冻结
- pre-commit 工具（简单 shell 脚本 vs husky/lefthook）——实施期定，倾向零依赖脚本
