# Proposal: phase-1-contracts-skeleton

## Why

Roadmap Phase-1 [Shared]：项目已有冻结的 SPEC（Wire 纪律、ADR-1~13、契约单一事实源机制），但仓库尚不存在任何代码、契约、生成管线或 Mock。本阶段让「可执行契约」先于一切业务代码存在——它是三轨（BE-Core / BE-Agent / FE）并行的公共前置，也是「契约漂移」这一头号风险的机制化防线（契约源唯一 + 生成物 commit + CI diff 显红）。现在做，因为 Phase-2 最小垂直切片立即消费这些产物，且双轨并行一旦启动，防漂移机制必须已在位。

## What Changes

- **建 monorepo 五目录骨架**：`app/` `src-tauri/` `sidecar/` `contracts/` `docs/`（现状仅 docs）+ pnpm workspace + 根脚本（`contracts:build` / `contracts:check` / `mock` 等）+ CI；app/ 与 sidecar/ 仅最小占位（package.json / tsconfig），脚手架留 Phase-2
- **Rust 契约源 crate 起步**（`src-tauri/crates/contracts`，零 tauri 依赖）：类型定义 + utoipa 注解 + 工具注册表；path 声明先 stub（带注解的空函数），随 Phase-2/3/5 实装迁移至真 handler（utoipa-axum），迁移正确性以 openapi.json 零 diff 证明
- **生成管线**：Rust 源 → `openapi.json` → 前端 TS 类型+ 工具 Schema（生成期 deref $ref，自包含 JSON Schema）；SSE 信封与 stdio JSON-RPC 类型并入 components，一条管线覆盖；生成物 commit 入库，CI diff 检查（GitHub Actions，Linux runner）
- **协议文档 `contracts/PROTOCOL.md`**：Wire 纪律细则（tagged enum / Option=字段省略 / 禁 untagged·flatten·camelCase）、错误码表与错误信封、鉴权约定（localhost + Bearer dev-token）、SSE 事件信封与首批事件、stdio JSON-RPC 信封与方法注册表、ID 前缀注册表（15 实体）、时间格式细则；随契约源同一 PR 变更
- **首批契约域**（按成熟度分层）：鉴权/错误信封/通用类型与笔记基础（create/read/search）、SSE 信封全量定稿；stdio JSON-RPC 定稿信封+health、方法注册表预留；实体域（projects/stages/tags）与捕获域（capture/card，含 PRD §7.3 卡片 schema）全量；events/timeline_groups/templates 骨架；todos 骨架且 status 为可扩展 enum（DEC-09 留弹性）；credential/memory/proposal 不做（随对应阶段增补）
- **Mock 服务**（`contracts/mock/`，轻量 Node）：语义内核（读 openapi.json 路由匹配 + 内存状态 + 写操作语义生效 + SSE 事件广播）+ 剧本覆写层（延迟/错误注入）；场景数据集（`contracts/mock/scenarios/`）固定 ULID + 启动时 Ajv 校验防漂移；首批场景：default / auth-401 / stream-demo / search-oracle
- **工具链 spike 并 IDR 定稿**：utoipa tagged enum→TS union、Option+skip_serializing_if→`field?: T`、chrono→RFC3339、openapi-fetch 消费体验、生成确定性（同源两次 gen 零 diff）、utoipa-axum 集成预研；**tauri-specta 不采纳**（DEC-01 后 IPC 无业务面），SPEC §3.2 工具链行修订 + 附录 B IDR 登记
- **CI 布防**：pre-commit hook（内环）+ GitHub Actions contracts job（外环）+ branch protection 开启（锁，R5 显式任务）

## Capabilities

### New Capabilities

- `contract-generation`: 契约源到生成物的管线行为——单一事实源、确定性生成、零 diff 校验、禁止手写第二份的机制保证
- `wire-protocol`: 三端共享的线上协议约定——Wire 纪律、错误信封与错误码、鉴权、SSE 事件信封、stdio JSON-RPC 信封、ID 前缀
- `mock-service`: 契约 Mock 服务——场景加载与校验、语义状态、SSE 推送、故障注入、场景选择机制

### Modified Capabilities

（无——首个 change，无既有 specs）

## Impact

- **下游影响**：Phase-2 为首个消费者（api-client / stdio 协议 / dev 环境）；Phase-8/9 消费工具 Schema；Phase-11~17 全部 FE 阶段消费 Mock 与场景
- **文档影响**：SPEC §3.2 工具链建议行（移除 tauri-specta）+ 附录 B 新增 IDR（工具链定稿）；Roadmap Phase-1 勾选 + 总则 4 开发环境约定定稿时点由 Phase-2 前移至 Phase-1（本 change 定稿，Phase-2 起直接引用）
- **依赖引入**：Rust：utoipa（+chrono feature）、serde、chrono；Node：openapi-typescript、openapi-fetch（app 占位即声明）、Ajv、Biome；包管理 pnpm（workspace）+ bun（sidecar 构建期）
- **仓库状态**：当前非 git 仓库，实施首步 `git init`；目标托管 GitHub（CI 按 Linux runner 设计）
