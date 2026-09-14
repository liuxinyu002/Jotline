# Tasks: phase-2-audit-hardening

## 1. 契约防线对齐（Rust 侧）

- [x] 1.1 pre-commit 触发正则补上 `src-tauri/crates/server/`（粗粒度，design D1）；验证：暂存 server crate 内任一改动后运行 `scripts/pre-commit.sh`，输出「执行 contracts:check」（此前输出「跳过」）
- [x] 1.2 gen 机械入库：`enforce_wire_discipline` / `strip_null` / `deref_refs` 移入 `server::gendoc` pub 模块，`bin/gen.rs` 只留 main() 装配（design D2）；验证：`cargo run -p server --bin gen` 后 `git diff contracts/` 为空（零 diff 纪律不破）+ `cargo clippy -p server -- -D warnings` 零告警
- [x] 1.3 `gendoc` fixture 单测：oneOf 坍缩后 X 键提升覆盖兄弟键、`type:[T,null]` 坍缩为标量、$ref 解引用兄弟键覆盖、循环 $ref 触发深度上限报错、查询参数剔 null 后 `required` 修正（design D2）；验证：`cargo test -p server gendoc` 全绿
- [x] 1.4 双注册一致性测试：路由注册（`OpenApiRouter` 拆出的 path 集合）== `ServerApiDoc` path 集合，差集非空即失败并打印两侧差集（design D3）；验证：`cargo test -p server` 通过；负向——临时注释 `ServerApiDoc` 中一条 path → 测试红并指出该端点差集

## 2. Mock 单测基建与违约修复

- [x] 2.1 mock 包测试接线：`test/` 目录 + `"test": "node --test test/"` 脚本，首批引擎级单测复用 `buildEngine` / `loadScenarios` DI（种子播种、写后读一致、SSE 广播、剧本覆写优先级，design D7）；验证：`pnpm --filter @jotline/mock test` 全绿
- [x] 2.2 响应侧校验：`ContractValidator` 启动期按操作 + 状态码编译响应体 validator，router 语义分发结果在发送前断言，违约返回 500 错误信封（detail 含操作 / 状态码 / 违约字段）并记日志（design D4）；验证：单测构造违约响应（故意缺必填字段）→ 断言收到 500 信封且 detail 定位到字段
- [x] 2.3 修 upsert 更新路径 `created_at` 丢失：`Object.assign` + `undefined` 剔除改为解构剔除（design 见 proposal）；验证：单测先红（2.2 校验器抓住 PATCH 后响应缺 `created_at`）→ 修复后绿，且更新后 list 响应含 `created_at`
- [x] 2.4 修 `executeCard` 事件挂靠：`note_id` 仅在本 entry 创建笔记时关联，不取 `noteIds.at(-1)` 误挂前驱；验证：单测三例——entry 同含 create_note 与 event → note_id 指向本 entry 笔记；entry 仅 event 且前 entry 有笔记 → note_id 不误挂；首 entry 仅 event → note_id 缺省

## 3. Mock 类型消费与配置收敛

- [x] 3.1 `contracts/mock/package.json` 增加 `"@jotline/contracts": "workspace:*"` 依赖（精确版本惯例对齐 workspace 协议）；验证：`pnpm install` 后 `pnpm --filter @jotline/mock typecheck` 通过（依赖可解析，镜像暂未收紧）
- [x] 3.2 `state.ts` 手写实体镜像改为引用 / `satisfies` 生成类型（design D5）；验证：`pnpm --filter @jotline/mock typecheck` 通过——暴露出的镜像漂移逐个以生成类型为准修复
- [x] 3.3 Mock dev token 环境变量化：`router.ts` 的 `DEV_TOKEN` 改读 `process.env.JOTLINE_DEV_TOKEN ?? "dev-token"`，`mock-smoke.sh` 的 `AUTH` 同步读环境变量带缺省（design D6）；验证：`JOTLINE_DEV_TOKEN=custom` 起 Mock → custom token 200 / dev-token 401；不带环境变量起 → dev-token 200（CI 兼容），mock-smoke 全绿
- [x] 3.4 `.env.example` 与 README token 口径一致化（删除「固定」与「按需修改」的自相矛盾，统一为「经 .env 注入、缺省 dev-token、四端一致」）；验证：按文档改 `.env` token → server / app / sidecar / Mock 四端同步生效（复述一次）

## 4. SSE / CORS 等价性锚点

- [x] 4.1 `mock-smoke.sh` SSE 断言：订阅期间执行一次写操作 → 断言收到心跳注释行、连续两条事件 `id:` 行严格递增（design D6）；验证：mock-smoke 绿；负向——临时改 `HEARTBEAT_MS` 为超窗口值 → 心跳断言红
- [x] 4.2 `mock-smoke.sh` CORS 预检断言：OPTIONS + Origin + Access-Control-Request-Method → 断言 ACAO 回显、allow-methods / allow-headers（design D6）；验证：mock-smoke 绿
- [x] 4.3 server 侧 `frame_event` 单测：信封 type → `event:` 行、event_id → `id:` 行、JSON → `data:` 行三件套格式（design D6）；验证：`cargo test -p server` 全绿

## 5. 前端失败路径修补

- [x] 5.1 `App.tsx` 三处网络级异常捕获：`handleSubmit` / `handleSearch` 以 try-finally 复位 `submitting` / `searching` 并渲染错误，`useEffect` 的 `listProjects()` 补 `.catch` 置 `projectsError`（design 见 proposal；不引入抽象）；验证：不起 server 时提交表单 → 按钮复位且 formError 显示；搜索同理；刷新页面 → 「项目加载失败」横幅显示
- [x] 5.2 补项目列表 loading 态（phase-2 tasks 6.3 未竟项，最小实现：一个状态 + 条件渲染）；验证：devtools 网络节流下进入页面 → 项目区显示加载指示，加载完成消失；加载失败显示失败态不显示加载态

## 6. 清理批处理

- [x] 6.1 Rust 侧清理：删 `contracts::ErrorEnvelope::status()` 及其单测；`repo_root()` 收敛为 server lib 单一 helper（含 `pnpm-workspace.yaml` 校验）供 gen bin 与 sidecar 共用；`stream.rs` 导出 `STREAM_PATH` 常量供 `auth.rs` 消费；`storage.read_note` 签名改 `&NoteId`；根 `Cargo.toml` 注释修正（design D8）；验证：`cargo test --workspace` + `cargo clippy --workspace --locked --all-targets -- -D warnings` 全绿 + `pnpm contracts:check` 零 diff
- [x] 6.2 TS 侧清理：删 Mock `SemanticState.seq` / `resetEventSeq` / `SseHub.reset()`、`app/src/api-client.ts` 的 `readNote`（均零调用方）；验证：`pnpm typecheck` + mock 单测 + mock-smoke 全绿

## 7. 全量验证

- [x] 7.1 全链回归：`pnpm contracts:check`（零 diff）、`pnpm typecheck`、`pnpm lint`、`pnpm --filter @jotline/mock test`、`bash scripts/mock-smoke.sh`、`bash scripts/dev-smoke.sh`、`cargo test --workspace`、`cargo clippy --workspace --locked --all-targets -- -D warnings`——依序全绿
- [x] 7.2 `openspec validate phase-2-audit-hardening --strict` 通过；归档时序确认：`phase-2-vertical-slice` 已归档（或确认无并行实施）后本 change 方可实施收尾
