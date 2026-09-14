# Design: phase-2-vertical-slice

## Context

Phase-1 已交付可执行契约（`src-tauri/crates/contracts` 契约源 + 生成管线 + `contracts/mock/` Mock 服务），仓库现状：app/ 与 sidecar/ 均为占位入口（`export {}`），无任何真实运行时。本切片打通「创建笔记 → 落盘 vault → SQLite 索引 → 检索命中 → 前端可见」，验证的是环境、序列化、鉴权、SSE、跨源、多窗口数据同源这些**全部后续阶段的公共底座**。

约束来自冻结项：SPEC §3.1 数据目录布局、ADR-2（sidecar 零监听、stdio 驱动）、ADR-5（文件真相源、SQLite 仅索引）、IDR-02 S6 / IDR-03（utoipa-axum 注解原样迁移、openapi.json 零 diff 证明）、Roadmap 总则 4（端口 / token / 数据目录约定）、Wire 纪律（错误信封、ULID、ISO-8601）。

## Goals / Non-Goals

**Goals:**

- 三端可运行骨架：主进程（真实实现）+ sidecar 壳 + 前端页面，`pnpm dev` 一键拉起
- 底座问题在此阶段全部暴露并有解：跨源（CORS）、SSE 鉴权（EventSource 无 header）、IPC 流纯净性（stdout 专用）、相对路径锚定（CWD）、契约注解迁移（零 diff）
- 验证命令可脚本化复述：固定 seed ULID + dev-smoke 断言链

**Non-Goals（除 proposal 所列外，design 层边界）:**

- 不做 server crate 的完整模块化分层（Phase-3 重构存储层时再立架构）；本阶段 handler 内聚即可
- 不做 SQLite 迁移版本化框架（手写建表 SQL + 简单 `PRAGMA user_version`，Phase-3 推倒重来）
- 前端不做路由、状态库、设计 token（Phase-11）；组件不抽库；Phase-2 页面定位为链路验证骨架，**显式豁免**消费 DESIGN.md 设计系统（色板 / 胶囊按钮 / 排版 / 暗色模式）——视觉规范全量接管属 Phase-11 应用骨架阶段（显式豁免，非默认忽略）

## Decisions

### D1 主进程形态：lib + bin 的 server crate，浏览器验证，Tauri 壳留 Phase-11

`src-tauri/crates/server` 以 **lib 形态**实现全部逻辑（`serve(config) -> (ready, shutdown)`），附 thin bin 直接运行。Phase-2 的 `pnpm dev` 用 bin + 浏览器 tab 作为「主窗」替身；Phase-11 将 lib 挂进 Tauri 主进程 setup（`tokio::spawn`），bin 长期保留为后端独立调试入口。

- 备选 A（纯 bin，不留 lib）：Phase-11 嵌入 Tauri 时需结构性迁移，弃
- 备选 B（Phase-2 即建 Tauri 壳，`tauri dev` 开真窗口）：编译重、`pnpm dev` 编排复杂化；且 Phase-11 前端验证基线是连 Mock（:4766），壳的 baseUrl 切换会提前纠缠，弃
- 已接受的代价：Tauri 进程内嵌 axum 的细节（tokio runtime 共存、生命周期）不在本阶段验证——属 Phase-11「应用骨架」职责

### D2 crate 结构与注解迁移（IDR-03 首次执行）

```
src-tauri/crates/
├── contracts/          # 契约源（既有）：类型 + 未实装域的 path stub
└── server/             # 新增：领域服务 lib + bin
    ├── src/lib.rs      #   serve()：路由组装（utoipa-axum OpenApiRouter）+ 状态 + 生命周期
    ├── src/bin/jotline-server.rs   # thin bin：读 env → serve()
    └── tests/          #   in-process API 语义测试（tower::ServiceExt，不占端口）
```

- **注解迁移**：notes 三端点 + `GET /api/projects` + `/api/stream` 的 `#[utoipa::path]` 注解从 contracts stub **原样迁至** server 真实 handler，contracts 中对应 stub 删除；gen bin 的 ApiDoc 组装改为**聚合两处**（contracts 剩余 stub + server 已实装 handler），openapi.json 零 diff 为迁移正确性证明（spike S6 已验证可行）
- server 依赖 contracts（类型复用），方向单向；契约铁律不变——接口定义仍只存在一份（注解物理位置随实装迁移，IDR-03 既定策略）

### D3 SQLite 最小范围与选型：rusqlite + FTS5 unicode61

- **表**（仅三张，Phase-3 推倒重来，索引可重建原则兜底）：`projects`、`items`（id / project_id / title / body / target_dir / format / tags / created_at / updated_at）、`items_fts`（FTS5 虚表，title + body，`content=''` 外部内容模式）
- **库选型 rusqlite**（同步 API + `Mutex<Connection>` 单连接）：本地单用户低负载，同步调用阻塞窗口微秒级；sqlx 的 async 收益在此场景为零且编译成本高。存储函数统一经 `tokio::task::spawn_blocking` 调用（默认包装，同步阻塞隔离于 async runtime 工作线程外，不留「实测异常再改」回路）。以 IDR 登记选型
- **bundled feature**：rusqlite 携带 bundled SQLite，避免依赖系统 sqlite3 的 FTS5 编译差异（macOS 系统 CLI 与库版本可能不一致）
- **检索边界**：unicode61 分词，验证范围英文词命中（Roadmap `q=hello`）；中文方案（trigram vs jieba，DEC-02）留 Phase-3 spike，本阶段不承诺
- **迁移机制**：`PRAGMA user_version` + 启动时建表 IF NOT EXISTS；不做版本化迁移框架

### D4 dev seed：固定 ULID，空库首启注入

数据目录无任何项目时，启动自动创建示例项目，**硬编码固定 ULID**（`prj_01J8Z3A7B4C5D6E7F8G9H0JKMN`，与契约测试样例一致，名称「切片示例」）；已有项目则跳过。固定值保证 Roadmap curl 命令与 dev-smoke 脚本可复述。Roadmap 文档中 `prj_seed` 字样随本 change 修订为该 ID。

- 备选（实装 `POST /api/projects` 手动造数）：每次 ID 随机，验证命令不可复述，且把 Phase-3 的实体域 API 提前拉进切片，弃

### D5 SSE 鉴权：query token 等价形态 + CORS

两个浏览器侧真实约束（Phase-1 全 curl 验证未暴露，本切片必须解决）：

1. **`EventSource` 不支持自定义 header** → SSE 端点鉴权接受 `?token=` query 参数，与 Bearer header 等价（PROTOCOL.md 鉴权节增补，wire-protocol delta 已列）；Mock 同步实现（几行改动，Phase-11 前端连 Mock 免撞墙）
2. **浏览器跨源**（:1420 → :4765）→ 主进程加 CORS 层（tower-http `CorsLayer`：允许 `http://localhost:*` / `http://127.0.0.1:*` origin、Authorization / Content-Type header、GET / POST / PATCH / OPTIONS 方法）；**Mock 服务同步补 CORS 中间件**（现状无任何 CORS 处理，Phase-11 必撞，本 change 顺手修复）

### D6 sidecar 骨架：env 注入、stdout 专用、stderr NDJSON、开发态直跑

- **拉起时序**：主进程 axum 就绪 → spawn `bun run sidecar/src/index.ts`（开发态直跑；Bun 编译单文件是 Phase-8 交付，不提前）→ 发 `health.ping` → 收 pong → sidecar 以环境变量中获得的端口 / token 回调 `GET /api/projects`（带 `X-Jotline-Sidecar: boot` 标识头）→ 主进程日志记「ADR-2 通道验证 ✓」
- **配置经 spawn 环境变量注入**（`JOTLINE_API_PORT` / `JOTLINE_DEV_TOKEN`）：零契约变更——不新增 stdio 协议方法，注册表保持仅 `health.ping`
- **stdout 纪律**：stdout 仅承载 JSON-RPC 消息（逐行 JSON）；**全部日志走 stderr 且为 NDJSON**（`{"ts","level","msg",…}`）——与 ADR-10 的 sidecar 日志通道设计同构，Phase-8 无缝接管；sidecar 代码中禁用 `console.log`（默认走 stdout，会污染 IPC 流），统一 `console.error` / `process.stderr.write`
- **boot 链超时与降级**：拉起链（spawn → pong → 回调 200）整体限时 5s，回调单步限时 2s；任一步超时或失败仅记 WARN 日志，主进程继续服务、不阻塞就绪、不重试（重启时序属 Phase-8，与崩溃处理哲学一致）
- **崩溃处理**：本阶段 sidecar 异常退出仅记日志（健康检查失败告警），崩溃重启时序骨架属 Phase-8

### D7 dev 编排：scripts/dev.mjs，零新依赖

- **职责**：加载 `.env`（手写 parse，格式同 `.env.example`）→ 并行 spawn（server bin 经 `cargo run -p server` + `pnpm --filter app dev`）→ 就绪探测（轮询 `GET /api/projects` 带 token 与 :1420 页面，超时非零退出并指明端点）→ 打印两端地址 → SIGINT 时 kill 全部子进程树
- **CWD 锚定**：子进程一律以仓库根为工作目录拉起，`./vault` 相对路径语义稳定；README 注明「直接在子目录 `cargo run` 会把 vault 落到子目录」
- **sidecar 不由编排管理**：其生命周期归主进程（架构定式），`pnpm dev` 的「三进程」指运行时结果而非编排直管三个
- **日志呈现**：`[core]` / `[web]` 前缀汇聚前台输出；sidecar 日志经主进程汇聚（其 stderr 由主进程转发）

### D8 vault 语义与一次性全布局

`JOTLINE_DATA_DIR` 语义定稿为 **vault 根目录**（非父目录），默认 `./vault`——与 Roadmap 验证命令（`ls $JOTLINE_DATA_DIR/notes/`）和 `.env.example` 注释（「vault 落点」）自洽；SPEC §3.1 图中 `<DataDir>/vault/` 措辞随本 change 微调为「vault 根」表述，消除 `./vault/vault` 歧义。启动时按 SPEC §3.1 **一次性初始化全布局**（六个子目录 + audit.jsonl 占位 + index.sqlite；`notes/.revisions` 属 Phase-3 合并覆写快照，非首启布局），成本为零，Phase-3 直接消费。`vault/` 进 `.gitignore`。

### D9 创建写入顺序与失败语义（双写一致性）

创建的双写（真相源文件 + SQLite 索引）按以下顺序定义，消除半提交态与未定义错误：

1. 先落真相源文件 `notes/<id>.md`
2. 再开 SQLite 事务（`items` + `items_fts` 同一事务提交）
3. 事务失败 → 删除刚写入的孤儿文件 + 返回 500 错误信封（code `internal`）——不留下「文件存在但索引缺失」的中间态
4. 事务提交后 → 广播 `note_created` → 返回 201；event-stream spec 的「同事务时机」据此落地为「索引事务提交后、返回 201 前」

500（`internal`）由此补入领域 API 的错误映射（401 / 404 / 422 已有）。

## 日志约定（SPEC §5.1 对齐）

- **设施**：主进程采用 `tracing` + `tracing-subscriber`（dev 控制台渲染，SPEC §5.1 定稿通道，不自建日志）
- **日志点**（关键路径）：启动（端口 / 数据目录）、鉴权失败、笔记创建、检索、SSE 订阅与断开、sidecar 拉起与回调
- **target 命名**：`rust.server` / `rust.notes` / `rust.stream` / `rust.sidecar`（对齐 §5.1 示例 `rust.ocr` 风格）
- **行格式**：中文 message + 英文 fields（§5.1 定稿）；sidecar stderr NDJSON 的 `msg` 同此约定

## Risks / Trade-offs

- [rusqlite 同步调用阻塞 async runtime] → 存储函数默认经 `spawn_blocking` 包装（见 D3），阻塞隔离于 async runtime 之外；单连接 `Mutex` 下本地单用户低负载，窗口本就微小
- [注解迁移零 diff 不成立（utoipa-axum 生成差异）] → spike S6 已预验证；若意外不成立，退路为永久 stub + CI diff 兜底（IDR-02 D2 既定退路，不破契约铁律）
- [query token 出现在 URL 中（凭证泄漏面）] → 仅 localhost 单用户 dev 场景，无服务器访问日志；生产 token 机制升级（Phase-7+）时该形态重新评估
- [Phase-3 推倒 SQLite 表结构] → 有意为之（索引可重建，ADR-5 兜底）；代价是 Phase-3 丢弃本阶段索引数据——dev 数据本就可抛弃
- [浏览器 tab 替代真窗口，集成面有盲区（Tauri webview 行为差异）] → 已接受（D1）；Phase-11 的 Tauri 骨架阶段消化，风险隔离在 FE 轨
- [sidecar 开发态依赖宿主机 bun] → dev 环境本就要求 pnpm/bun 工具链（`.node-version` 已钉）；用户免装 Node 是 Phase-8 externalBin 的目标，本阶段不背

## Migration Plan

1. **前置**：`phase-1-contracts-skeleton` 先行归档（`openspec archive`），使 wire-protocol 主 spec 基线存在
2. 实施顺序即依赖顺序（见 tasks.md 分组）：crate 骨架 → 存储 → API → SSE → sidecar → 前端 → 编排 → 冒烟/CI
3. **文档回写**（同一 PR / change 收尾）：SPEC §3.1（vault 语义）、PROTOCOL.md（SSE 鉴权增补）、Roadmap（Phase-2 勾选 + `prj_seed` 修订）、`.env.example`（注释澄清）、README（本地开发节）、`.gitignore`（vault/）
4. **回滚**：本阶段全部为新增（新 crate / 新工程 / 新脚本），回滚 = revert；唯一存量改动是 Mock 补 CORS 与 PROTOCOL.md 增补，均为纯增量

## Open Questions

- server crate 内部模块划分（`storage.rs` / `notes.rs` / `stream.rs`…）：实施时按代码量自然成形，不预设计
- dev-smoke 是否支持 `--data-dir` 参数隔离测试数据：实施时看需要，不影响行为契约
- 前端 React 版本（18 / 19）：实施时取当前稳定版，Phase-11 无迁移成本即可
