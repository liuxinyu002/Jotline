# Proposal: phase-2-vertical-slice

## Why

Roadmap Phase-2 [Shared] 最小垂直切片。Phase-1 交付了可执行契约（契约源 / 生成管线 / Mock 服务），但仓库尚无任何真实实现：没有可运行的主进程、没有数据库、没有前端工程、没有 sidecar 运行时。本阶段打通「前端 → API → 数据库」完整链路——最简笔记路径：创建笔记 → 落盘 vault → SQLite 索引 → 检索命中 → 前端可见。目的不是做功能，而是**尽早暴露环境、序列化、鉴权、SSE、多窗口数据同源等集成层面的问题**，它们是 Phase-3~17 三线并行开发的公共底座。现在做，因为 BE-Core / BE-Agent / FE 三轨的第一批阶段全部依赖本切片验证过的运行时形态。

## What Changes

- **Rust 主进程骨架**（新 crate `src-tauri/crates/server`，lib + bin 形态）：axum 监听 `127.0.0.1:4765` + Bearer 校验（401 错误信封）；vault 数据目录按 SPEC §3.1 全布局一次性初始化；SQLite 最小迁移（projects / items / FTS 虚表）+ dev 首启 seed 固定 ULID 示例项目；实装 notes 域三 API（create / read / search）与 `GET /api/projects`；FTS5 检索（unicode61 分词，英文验证；中文命中不承诺，留 Phase-3 DEC-02 定稿）；SSE 端点（心跳 + `note_created` 广播）
- **契约注解迁移首执行**（IDR-03 策略落地）：notes / projects / stream 域的 `#[utoipa::path]` 注解从 contracts crate 空 stub 原样迁至 server 真实 handler（utoipa-axum `OpenApiRouter`），gen bin 组装改为聚合两处，迁移正确性以 **openapi.json 零 diff** 证明
- **sidecar Bun 工程骨架**：由主进程经 stdio spawn；JSON-RPC `health.ping` → pong 往返；经 HTTP 回调主进程领域 API（ADR-2 通道验证，回调带 `X-Jotline-Sidecar` 标识头）；配置（端口 / token）经 spawn 环境变量注入，**零契约变更**；全部日志走 stderr（stdout 为 IPC 专用通道）
- **前端工程骨架**（`app/`，React + Vite，Tauri-ready 但不上 Tauri 壳——窗口体系留 Phase-11）：单页面（笔记列表 + 创建表单 + 搜索框）；api-client（openapi-fetch）从占位接通真实主进程；EventSource 订阅 `note_created` 实现多标签页数据同源
- **`pnpm dev` 一键启动编排**：`scripts/dev.mjs`（零新依赖）读 `.env` → 并行拉起 server bin 与 vite → 就绪探测 → Ctrl-C 全链退出；`scripts/dev-smoke.sh` 将 Roadmap 验证 curl 链脚本化；README「本地开发」节更新
- **协议与文档修订**：PROTOCOL.md 鉴权节增补 **SSE 端点支持 query 参数 token**（浏览器 `EventSource` 无法携带自定义 header，与 Bearer header 等价），Mock 服务同步实现；SPEC §3.1 澄清 `JOTLINE_DATA_DIR` 语义 = vault 根目录（消除 `./vault/vault` 歧义）；Roadmap Phase-2 验证命令中 `prj_seed` 字样修订为实际固定 ULID；`.gitignore` 补 `vault/`

## Capabilities

### New Capabilities

- `vault-storage`: 数据目录（vault）的存储行为——SPEC §3.1 全布局初始化、文件真相源落盘（notes/<id>.md）、SQLite 索引与真相源分离、dev 首启 seed
- `domain-api`: 领域 API 的真实运行时语义——鉴权中间件、notes create/read/search 与 projects list 的请求校验与响应、错误信封
- `event-stream`: 真实主进程的 SSE 行为——心跳维持、写操作后向全部订阅者广播注册事件、多订阅者数据同源
- `sidecar-runtime`: sidecar 进程骨架行为——被主进程 spawn、stdio JSON-RPC 健康检查、HTTP 回调通道（ADR-2）
- `dev-orchestration`: 开发编排行为——`pnpm dev` 三进程拉起与就绪探测、`.env` 约定消费、全链退出、dev-smoke 验证脚本

### Modified Capabilities

- `wire-protocol`: 「鉴权约定」requirement 增补——SSE 端点 MUST 接受与 Bearer header 等价的 query 参数 token（`EventSource` 技术约束）；Mock 与真实主进程同步实现

> 注：主 specs 库当前为空（phase-1 change 尚未归档）。`wire-protocol` 的 modified delta 以 phase-1 change 内的同名 spec 为基线；归档时序须先于本 change 合入（见 Impact）。

## Impact

- **下游影响**：Phase-3~7（BE-Core）在 server crate 上扩展存储与领域 API；Phase-8~10（BE-Agent）在 sidecar 骨架上建立 provider 与会话运行时；Phase-11+（FE）建 Tauri 工程时复用 api-client 与 dev 编排，并新增 Mock 连接模式
- **文档影响**：SPEC §3.1（vault 语义澄清）、Roadmap（Phase-2 勾选 + `prj_seed` 字样修订）、contracts/PROTOCOL.md（SSE 鉴权增补）、`.env.example`（注释澄清）、README（本地开发节）、`.gitignore`（vault/）
- **依赖引入**：Rust workspace 新增 server crate（axum / tokio / rusqlite / utoipa-axum / tower-http / tracing 等，选型见 design）；`app/` 从占位升级为 React + Vite 工程：新引入 react / react-dom / vite / @vitejs/plugin-react（openapi-fetch 已在 workspace），均按仓库惯例以精确版本写入 package.json（不带 `^` / `~`）；sidecar 仍为 Bun 开发态直跑（Bun 编译单文件留 Phase-8）；dev 编排零新依赖
- **归档时序约束**：本 change 依赖 `phase-1-contracts-skeleton` 先行归档（`openspec archive`），使 `wire-protocol` 主 spec 基线存在
- **不做**（Non-Goals，见 design）：Tauri 窗口壳与多窗口体系（Phase-11）、全量 SQLite 表与中文 FTS（Phase-3）、pi agent 真实运行时与 provider（Phase-8）、捕获管线（Phase-5）、凭证 / 审计 / 记忆（Phase-7）
