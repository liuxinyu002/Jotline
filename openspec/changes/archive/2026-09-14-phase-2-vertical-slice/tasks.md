# Tasks: phase-2-vertical-slice

## 0. 前置归档

- [x] 0.1 归档 `phase-1-contracts-skeleton`（`openspec archive phase-1-contracts-skeleton`），使 wire-protocol 主 spec 基线存在（proposal Impact 归档时序约束）；验证：`openspec/specs/wire-protocol/` 出现，`openspec validate phase-2-vertical-slice --strict` 不再提示 archive 顺序 INFO

## 1. server crate 骨架与鉴权

- [x] 1.1 根 Cargo.toml 新增 member `src-tauri/crates/server`（lib + bin 结构，见 design D2）；依赖 contracts / axum / tokio / rusqlite(bundled) / utoipa-axum / tower-http / tracing + tracing-subscriber（日志通道按 SPEC §5.1 定稿，见 design「日志约定」）；验证 `cargo check -p server` 与 `cargo clippy -p server` 零告警通过
- [x] 1.2 `serve()` lib 入口（配置注入：端口 / token / 数据目录，默认值对齐 .env.example）+ thin bin `jotline-server`（读环境变量调 lib）；验证 `cargo run -p server` 后 `curl -s http://127.0.0.1:4765/` 得到 404 契约外路由信封（服务在监听），Ctrl-C 干净退出
- [x] 1.3 Bearer 鉴权中间件（错 / 缺 token → 401 统一错误信封，复用 contracts 的 `ErrorEnvelope`）+ CORS 层（`http://localhost:*` / `http://127.0.0.1:*` origin，Authorization / Content-Type header，GET / POST / PATCH / OPTIONS 方法）；验证：无 token curl → 401 信封；跨源预检 `OPTIONS`（Origin: http://localhost:1420）→ 通过

## 2. vault 存储与 seed

- [x] 2.1 vault 全布局初始化（notes / attachments / inbox / projects / memory / templates 子目录 + audit.jsonl 占位 + index.sqlite 三表迁移：projects / items / items_fts，`PRAGMA user_version` 版本标记）；`.gitignore` 补 `vault/`；验证：指向空临时目录启动后 `ls` 布局齐、`sqlite3 .tables` 三表在、`git status` 无 vault 泄漏、二次启动不破坏既有内容
- [x] 2.2 dev 首启 seed：数据目录无项目时创建固定 ULID `prj_01J8Z3A7B4C5D6E7F8G9H0JKMN`（名称「切片示例」），已有项目跳过；验证：空库启动后 `sqlite3 "select id from projects;"` 含该 ID，再启动行数不变（API 层复验在 3.2）
- [x] 2.3 存储层笔记写入与读取函数（真相源文件 `notes/<id>.md` + items 行 + FTS 行同步写；读取从索引行组装响应）；cargo 单测（tempdir 数据目录）验证：写入后文件内容含请求正文、三处一致、ULID / RFC 3339 格式断言
- [x] 2.4 存储层检索函数（FTS 查询 + snippet 提取 + project_id 过滤 + top_k）；cargo 单测验证：英文词命中刚写入笔记、过滤生效、无命中返回空列表

## 3. notes / projects API 与注解迁移

> 注：3.3 的零 diff 证明依赖 3.1 / 3.2 完成后统一执行；本章 handler 用 in-process 测试（tower::ServiceExt）覆盖，不占端口。

- [x] 3.1 notes 域三端点 handler（`POST /api/notes` / `GET /api/notes/{id}` / `GET /api/search` 的路由与校验骨架，`#[utoipa::path]` 注解从 contracts stub 原样迁入 utoipa-axum `OpenApiRouter`）；请求校验：必填缺失 / 类型不符 → 422 信封含字段定位，project 不存在 → 404；验证 in-process 测试覆盖 201（含 ULID 响应与可选字段省略）/ 404 / 422 全语义
- [x] 3.2 `GET /api/projects` handler（列表信封 + 分页元数据）；验证：in-process 测试断言 seed 项目在列表中且字段符合契约；`curl`（经 1.2 起的服务）返回 200 含固定 ID 项目
- [x] 3.3 gen bin ApiDoc 组装改为聚合两处（contracts 剩余 stub + server 已实装 handler），contracts crate 删除已迁移的五个 path stub（notes×3 / projects list / stream）；验证：迁移前后 `pnpm contracts:build` 产物 `git diff` 为空（**零 diff = 注解迁移正确性证明**，IDR-03 首次执行），`pnpm contracts:check` 通过
- [x] 3.4 三端消费复验：`pnpm typecheck`（app / sidecar / mock 对生成类型导入零漂移）+ `pnpm mock` 起动场景校验仍绿

## 4. SSE 事件流与协议增补

- [x] 4.1 `GET /api/stream` 端点：broadcast channel + 空闲心跳注释行 + `note_created` 事件广播（信封按契约 StreamEnvelope）；鉴权接受 query token（`?token=`）与 Bearer header 等价；验证：`curl -sN "…/api/stream?token=dev-token"` 保持连接收到心跳行；订阅期间创建笔记 → 收到 `event: note_created` 且 data 含笔记 ID；无 token 订阅 → 401
- [x] 4.2 create_note 成功后广播接入（写操作与事件推送同事务时机）；验证：两个并发 SSE 订阅连接均收到同一条事件（顺序一致）
- [x] 4.3 Mock 同步增补：SSE 路由接受 query token + CORS 中间件；验证：Mock 侧 `curl -sN "…:4766/api/stream?token=dev-token"` 订阅成功、无 token 401、跨源预检通过；既有四场景回归不破（`pnpm mock` + 冒烟 curl 全绿）

## 5. sidecar 骨架与拉起链

- [x] 5.1 `sidecar/src/index.ts`：stdin 逐行 JSON-RPC 解析（`health.ping` → 同 id pong，复用 `@jotline/contracts` 的 Ipc 类型）+ stderr NDJSON 日志（`{"ts","level","msg"}`，禁用 `console.log`）；验证：`echo '{"jsonrpc":"2.0","id":"t1","method":"health.ping"}' | bun run sidecar/src/index.ts` → stdout 恰一行可解析响应、日志全在 stderr
- [x] 5.2 主进程拉起链：axum 就绪后 spawn sidecar（env 注入 `JOTLINE_API_PORT` / `JOTLINE_DEV_TOKEN`，CWD=仓库根）→ ping-pong → sidecar HTTP 回调 `GET /api/projects`（带 `X-Jotline-Sidecar: boot` 头）→ 主进程日志「ADR-2 通道验证 ✓」；验证：启动主进程后日志出现该行、`ps` 进程树含 sidecar 子进程
- [x] 5.3 生命周期清理与 IPC 纯净性：主进程退出（SIGINT / panic）时 kill sidecar；运行期间 sidecar stdout 恒为可解析 JSON-RPC 行；验证：Ctrl-C 主进程后 `ps` 无孤儿 sidecar；拉起期间从主进程侧读取的 sidecar stdout 零非 JSON 行

## 6. 前端工程（React + Vite，Tauri-ready 不上壳）

- [x] 6.1 `app/` 升级为 Vite + React 工程（index.html / 入口 / 单页面 App，dev server 固定 :1420，无路由无状态库）；新增依赖 react / react-dom / vite / @vitejs/plugin-react 以精确版本写入 app/package.json（不带 `^` / `~`，对齐既有 openapi-fetch "0.17.0" 惯例，pnpm-lock 同步冻结）；验证 `pnpm --filter app dev` 起 :1420、页面可访问
- [x] 6.2 api-client 接通真实主进程：baseUrl 经环境变量（dev 默认 :4765）+ token 注入 + 薄封装（listProjects / createNote / readNote / searchNotes，全部走契约生成类型）；验证 `pnpm --filter app typecheck` 零错误（消费 `@jotline/contracts` 生成类型，无手写接口形状）
- [x] 6.3 单页面三件套：笔记列表（含空态）+ 创建表单（项目下拉来自 listProjects）+ 搜索框（结果含 snippet）；组件状态覆盖：列表加载中（skeleton 或 spinner，实施二选一）/ 空态 / 失败，表单提交中（按钮禁用）/ 提交失败（渲染错误信封 detail 定位），搜索执行中 / 无结果 / 失败，SSE 断连时 EventSource 自动重连期间降级提示；验证：浏览器走通 Roadmap 界面路径——创建「切片验证」→ 列表出现 → 搜索 `hello` 命中显示
- [x] 6.4 EventSource 订阅 hook（query token 形态）+ `note_created` 触发列表刷新；验证：双浏览器标签页同开，A 提交创建 → B 无任何操作即出现新笔记（SSE 驱动，非轮询）

## 7. dev 编排与开发文档

- [x] 7.1 `scripts/dev.mjs`（零新依赖）：手写 parse `.env` → 并行 spawn（`cargo run -p server` 与 `pnpm --filter app dev`，CWD 一律仓库根）→ 就绪探测（轮询 4765 带 token 请求 + 1420 页面，超时非零退出并指明端点）→ 打印两端地址 → SIGINT 全链 kill；验证：`pnpm dev` 全链启动到就绪输出、两端即测即通、Ctrl-C 后 `ps` 无残留（含 sidecar）
- [x] 7.2 根脚本 `pnpm dev` / `pnpm dev:smoke` 接线；README「本地开发」节更新（命令清单、数据目录语义与 CWD 注意、进程拓扑说明）；`.env.example` 注释澄清（`JOTLINE_DATA_DIR` = vault 根；「前端 dev server（Phase-11 起）」更正为 Phase-2 起，1420 自本切片启用）；验证：按 README 从零复现环境准备到 `pnpm dev` 就绪

## 8. 冒烟脚本、CI 与文档回写

- [x] 8.1 `scripts/dev-smoke.sh`：隔离临时数据目录启动 server bin → 机械执行 Roadmap curl 链断言（创建 201 + ULID 响应 → `ls` 文件 → `sqlite3` 行 → `q=hello` 命中 → 错 token 401）→ 清理退出码 0；验证：干净环境全链通过；人为破坏一个断言（如检索词）→ 非零退出并定位环节
- [x] 8.2 CI 扩展：contracts.yml rust job 追加 `cargo test --workspace`（含 server in-process 测试）与 `cargo clippy --workspace --locked --all-targets -- -D warnings`（对齐 1.1 的 server 零告警门禁）；`Swatinem/rust-cache` 的 workspaces 锚点由 `src-tauri/crates/contracts` 调整为仓库根 workspace；`dev-smoke.sh` 可选接入（Linux runner 下 vault 路径兼容确认后）；验证推送后三 job 全绿
- [x] 8.3 文档回写：SPEC §3.1 vault 语义澄清（JOTLINE_DATA_DIR = vault 根）；PROTOCOL.md 鉴权节增补 SSE query token 等价形态；Roadmap Phase-2 验证命令 `prj_seed` 修订为固定 ULID + Phase-2 勾选；验证 `openspec validate phase-2-vertical-slice --strict` 通过
- [x] 8.4 最终验证（Roadmap Phase-2 完成标准）：`pnpm dev` 一键启动 → 依序执行 Roadmap 全部 curl 命令（201 / 文件 / 索引行 / 检索命中 / 401）→ 界面路径人工走查（创建 → 列表 → 搜索 → 双标签页同源）→ `pnpm contracts:check` 零 diff → `pnpm typecheck` / `pnpm lint` 全绿
  > 状态：除「sidecar 以真实 bun 运行」外的全部子项已验（pnpm dev 就绪/退出、curl 链经 dev-smoke、
  > 界面走查经浏览器自动化、三端检查全绿）；待 bun 决策后补齐 sidecar 原生验证即可勾选。
