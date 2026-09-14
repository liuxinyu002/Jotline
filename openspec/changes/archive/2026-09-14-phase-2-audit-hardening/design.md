# Design: phase-2-audit-hardening

## Context

审计基线见 proposal（Why）。当前形态的关键事实：IDR-03 之后 5 个 `#[utoipa::path]` 注解与 gen bin 物理住在 server crate，contracts crate 只剩未实装域 stub；Mock 是唯一不依赖 `@jotline/contracts` 的 TS 端（Node 24 原生 TS 运行，`node --watch src/server.ts`），其 `ContractValidator` 只编译请求侧（请求体 / 查询参数 / 场景种子）；前端 App.tsx 三处 async 无网络级异常捕获。约束：不回迁注解（IDR-03 在役）；契约铁律（接口定义唯一事实源）不变；phase-2 的 Non-Goals（前端不抽组件库、server 不做完整分层）继续有效。

**已接受代价登记（IDR-03 内生）**：契约生成管线（`cargo run -p server --bin gen`）依赖 server 实现 crate 整体编译（含 axum / tokio full / rusqlite bundled）。备选「注解留在 contracts stub」与 IDR-03 矛盾，弃；备选「gen 独立 crate」不消除对 server lib 的传递依赖（聚合必须可见 handler 注解），仅增加一个空壳包，弃。缓解：CI rust-cache（已锚定仓库根 workspace）+ pre-commit 触发粗粒度换取的拦截完备性。

## Goals / Non-Goals

**Goals:**

- 契约防线重新覆盖注解真实物理位置；变换机械与注册一致性进入机器守卫（测试网）
- Mock 产出响应违约时可见失败；实体形状知识收敛回契约（编译期可见）
- 前端失败矩阵覆盖网络级异常；死代码与微双份清零

**Non-Goals:**

- 不动 phase-2 既定架构形态：server 单层 handler 结构、前端单文件组件、sidecar 骨架分发器（分别属 Phase-3 / Phase-11 / Phase-8）
- 不做存储 init 语义收口（seed 门控 / 索引重建锚点——Phase-3 设计前提，见审计报告）
- 不为响应校验 / SSE 锚点引入共享 fixture 文件（两实现各自锚定，spec 为共同参照）
- 不动 auth 中间件双职责与 AppState 形态（Phase-8 / Phase-3）

## Decisions

### D1 pre-commit 触发集：粗粒度（整个 server crate）

触发正则追加 `src-tauri/crates/server/`。备选细粒度（仅注解 handler 文件 + gen bin）省时但脆：新 handler 文件名不在清单即漏拦，且注解位置随域迁移漂移需持续维护清单。粗粒度的代价（server 任意改动触发 contracts:check）在增量编译下为秒级，可接受；换来「新域迁移注解零正则维护」。

### D2 gen 机械入库：server lib 的 `gendoc` 模块，不建独立 crate

`enforce_wire_discipline` / `strip_null` / `deref_refs` 移入 `server::gendoc`（pub 模块），bin/gen.rs 只留 main() 装配（定位仓库根 → 组装 → 落盘）。fixture 单测覆盖：oneOf 坍缩后 X 键提升覆盖兄弟键、`type:[T,null]` 坍缩、$ref 解引用兄弟键覆盖、循环 $ref 触发深度上限、查询参数剔 null 后 `required` 修正。备选独立 crate 不通过两 adapter 判据——单一消费者（gen bin），假设性缝，弃。

### D3 双注册守卫：一致性测试，不改 gen 聚合方式

新增测试：构造路由（复用 `router()` 的 `OpenApiRouter` 拆出的 path 集合）与 `ServerApiDoc::openapi()` 的 path 集合比对，差集非空即失败并打印两侧差集。备选「gen 直接消费 `split_for_parts()` 拆出的文档半边」需要 gen bin 构建带 state 的完整路由器，且 components 仍须从 contracts ApiDoc 合并——复杂度不成比例，弃（测试守卫已把失败形态从静默变为显形）。

### D4 响应侧校验：dispatch 后、发送前，Ajv 断言 + 可见失败

`ContractValidator` 扩展：启动时按操作 + 状态码编译响应体 validator（复用既有 Ajv 实例与 openapi.json 装载）。router 的语义分发结果在 `sendJson` 前校验；违约时不送出原响应，返回 500 错误信封（detail 指明操作、状态码与违约字段）并在日志输出违约详情——冒烟与开发期立即显形。剧本注入的预定义响应不经此路径（剧本数据已有启动期校验），SSE 帧不经此路径（帧纪律锚点见 D6）。备选「仅启动自检样例请求」覆盖面不足（运行时状态构造的响应才是漂移主源），弃。

### D5 Mock 类型消费：workspace 依赖 + `satisfies`，Ajv 仍是运行时真值

`contracts/mock/package.json` 增加 `"@jotline/contracts": "workspace:*"`；`state.ts` 的手写实体镜像改为引用生成类型（`satisfies components["schemas"][…]` 或直接类型标注）。收紧可能在两处已知违约之外暴露更多镜像漂移——这正是目的，随修即可。Ajv 运行时校验地位不变（类型只管编译期）。

### D6 token / SSE / CORS 锚点：三端各自锚定，零共享 fixture

- Mock：`DEV_TOKEN` 改为 `process.env.JOTLINE_DEV_TOKEN ?? "dev-token"`（与 `MOCK_PORT` 同法）；`mock-smoke.sh` 的 `AUTH` 同步读环境变量带缺省
- `mock-smoke.sh` 增补：SSE 订阅期间执行一次写操作 → 断言收到心跳注释行、连续两条事件 `id:` 行严格递增；OPTIONS 预检（带 Origin + Access-Control-Request-Method）→ 断言 ACAO 回显与 allow-headers/methods
- server 侧 `frame_event` 单测：断言 event / id / data 三行格式（信封 type → event 行、event_id → id 行、JSON → data 行）

### D7 Mock 单测载体：`node --test`（与运行时一致，零新依赖）

mock-smoke 中的 bash 内联 `python3 -c` JSON 形状断言迁入 mock 包 `node --test`（`test/` 目录，scripts 加 `"test": "node --test test/"`），复用 `buildEngine` / `loadScenarios` 的 DI 面（注入内存 readJson / 校验器）构造引擎级单测：种子播种、写后读一致、SSE 广播、剧本覆写优先级。mock-smoke 保留端到端 curl 链（验证运行中的服务），两者职责分离——单测管形状与语义，冒烟管进程与协议。

### D8 清理形态（删除测试通过项的落地方式）

- `contracts::ErrorEnvelope::status()`：连同其单测删除（错误码 → HTTP 状态映射由 `ApiError::into_response` 单点承担；PROTOCOL.md §4 登记表不变）
- Mock 死代码：`SemanticState.seq` / `resetEventSeq` / `SseHub.reset()` 删除（event_id 归 `SseHub.nextId` 单点所有）
- `api-client.readNote` 删除（零调用方；Phase-11 有真实列表端点时按需重建）
- `repo_root()`：server lib 单一 helper（含 `pnpm-workspace.yaml` 校验），gen bin 与 sidecar.rs 共用
- SSE 路径：`stream.rs` 导出 `pub const STREAM_PATH`，`auth.rs` 消费；`#[utoipa::path]` 注解内字面量留在常量定义旁（宏要求字面量，局部性以同文件邻接达成）
- `storage.read_note` 签名 `&str` → `&NoteId`（调用方不再打回字符串）
- 根 `Cargo.toml` 注释改为描述现实：server = 主进程 lib + bin，tauri 壳 Phase-11 加入

## Risks / Trade-offs

- [粗粒度触发让 server 全部提交都跑 contracts:check] → 增量编译 + 生成 + 场景校验为秒级；这是把「漏拦漂移」换成「多跑秒级检查」的明确交换（D1）
- [响应校验为每个响应增加运行时开销] → dev 工具量级（Ajv validator 启动期编译一次，单次校验微秒级）；不加开关配置面——简单优先，出问题再议
- [`satisfies` 收紧暴露超出预期的镜像漂移] → 逐个修复即达成目的（编译期可见性）；若出现结构性不匹配（如 Mock 内部形态确需更宽），以生成类型为准调整内部形态
- [两处 Mock 违约修复改变响应内容] → 已验证四场景零覆盖这两条路径（upsert 更新 / executeCard 均无场景触达），冒烟链不破；这是向契约回归的行为变更
- [单测迁移期间冒烟与单测短暂双轨] → 任务顺序先建单测后收缩 mock-smoke 对应段，全程至少一层在线

## Migration Plan

纯开发工具链 + bug 修复，无数据迁移。实施顺序即任务分组：契约防线（Rust）→ Mock 保真与配置 → 前端修补 → 清理 → 全量验证（contracts:check 零 diff / typecheck / lint / mock 单测 / 双冒烟 / cargo test --workspace）。回滚 = revert 单 commit。归档时序：建议 `phase-2-vertical-slice` 先行归档（任务已全数完成），本 change 再实施，避免两 change 并行踩同一批文件。
