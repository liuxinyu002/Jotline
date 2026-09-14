# Proposal: phase-2-audit-hardening

## Why

2026-09-14 架构审计（`docs/audit/2026-09-14_architecture-review.html`，commit cde26cd 基线）发现：phase-2 垂直切片落地后，IDR-03 把契约注解与 gen bin 迁入 server crate，但围绕契约的四道防线（pre-commit 触发集、变换机械单测、端点注册守卫、构建耦合登记）没有跟着迁移边界；Mock 作为唯一不消费生成类型的 TS 端，响应侧零校验且已有两处实际违约；前端三处网络级异常无捕获会永久卡死交互态。全部为小时级收口修复，无阻断项——趁 phase-2 未归档、注解位置记忆新鲜时做成本最低。

## What Changes

- **契约防线对齐**（顺应 IDR-03，不回迁注解）：
  - pre-commit 触发正则补上 `src-tauri/crates/server/`（粗粒度：整个 crate，新域迁移注解无需改正则）
  - gen 的 Wire 纪律机械（`enforce_wire_discipline` / `strip_null` / `deref_refs`）从 bin 移入 server lib 可测模块 + fixture 单测（oneOf 坍缩、兄弟键覆盖、循环 $ref、查询参数 required 修正），bin 只留装配
  - 新增一致性测试：路由注册（`routes!`）path 集合 == `ServerApiDoc` path 集合，堵「漏登记 = 端点静默从契约消失」
  - 在 design 登记契约管线依赖 server 实现 crate 编译这笔 IDR-03 内生代价
- **Mock 契约保真收口**：
  - 修复两处实际违约：upsert 更新路径 `created_at` 丢失（`Object.assign` undefined 剔除改为解构剔除）；`executeCard` 事件 `note_id` 挂靠前一个 entry 的笔记（改为仅本 entry 创建笔记时关联）。四场景零覆盖这两条路径，修复不破现有冒烟
  - 响应侧校验：语义内核产出响应经契约 Schema（Ajv）断言，违约以可见失败暴露（不再静默送出违约响应）
  - mock 引入 workspace 依赖 `@jotline/contracts`，手写实体镜像改为引用 / `satisfies` 生成类型——契约变更在 mock 编译期可见
- **Mock 配置与等价性锚点**：
  - Mock 的 dev token 改读 `JOTLINE_DEV_TOKEN`（缺省 `dev-token`，与 server / sidecar / app 同法），`mock-smoke.sh` 同步；`.env.example` / `README` 文档口径一致化
  - `mock-smoke.sh` 增补断言：SSE 心跳行存在、event_id 单调递增、CORS 预检（OPTIONS + Origin → ACAO/ACAM/ACAH）
  - mock 内核关键行为断言从 bash 内联 python 迁入 `node --test`（mock 包内，复用 `buildEngine` / `loadScenarios` 的 DI 面）
  - server 侧 `frame_event` 补单测（event / id / data 三件套行格式）
- **前端失败路径修补**（补 phase-2 tasks 6.3 未竟范围，不引入抽象——triple 形态留给 Phase-11）：
  - `App.tsx` 三处网络级异常捕获：`handleSubmit` / `handleSearch` 用 try-finally 复位交互态并渲染错误；`useEffect` 的 `listProjects()` 补 `.catch` 置 `projectsError`
  - 补项目列表 loading 态（最小实现：一个状态 + 条件渲染）
- **清理批处理**（删除测试全部通过的死代码与微双份）：
  - 删：`contracts::ErrorEnvelope::status()`（含其单测；生产映射实际由 `ApiError` 完成）、Mock `SemanticState.seq` / `resetEventSeq`、`SseHub.reset()`、`app/src/api-client.ts` 的 `readNote`（均零生产调用方）
  - 合并：`repo_root()` 双份 → server lib 单一 helper（统一校验）；`"/api/stream"` 路径提为共享常量（`auth.rs` 消费，注解字面量留在定义旁）
  - 签名：`storage.read_note(&str)` → `read_note(&NoteId)`（缝上类型不再退化）
  - 根 `Cargo.toml` 注释修正为描述现实（tauri 壳 Phase-11 加入）

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `contract-generation`: 新增两条 requirement——① 漂移防线（pre-commit 触发集）SHALL 覆盖契约注解的全部物理位置（IDR-03 后含 server crate），注解源改动未重新生成时提交 MUST 被拦截；② 生成管线 SHALL 以机器守卫保证路由注册与 OpenAPI 文档注册的端点集合一致，不一致 MUST 以失败暴露
- `mock-service`: 新增两条 requirement——① Mock SHALL 对产出响应按契约 Schema 校验，违约响应 MUST 以可见失败暴露（不静默送出）；② Mock 的 dev token SHALL 经环境变量 `JOTLINE_DEV_TOKEN` 注入（缺省 `dev-token`），与其他三端配置通道一致

## Impact

- **代码触点**：`scripts/pre-commit.sh`、`scripts/mock-smoke.sh`、`src-tauri/crates/server/`（gen 机械入库、一致性测试、常量与签名收口）、`src-tauri/crates/contracts/src/error.rs`（删 status()）、`contracts/mock/src/`（违约修复、响应校验、类型消费、token env、单测）、`contracts/mock/package.json`（+`@jotline/contracts` workspace 依赖）、`app/src/{App.tsx,api-client.ts}`、根 `Cargo.toml`、`.env.example`、`README.md`
- **依赖引入**：仅 mock 包新增 workspace 内部依赖 `@jotline/contracts`；零外部新依赖
- **归档时序约束**：建议 `phase-2-vertical-slice` 先行归档（其 28 项任务已全部完成、验证已过）——本 change 实施文件与其高度重叠，先行归档避免两 change 并行踩同一批文件
- **行为变更**：两处 Mock 违约修复会改变 mock 响应内容（`created_at` 恢复必填、事件挂靠修正）——这是向契约回归，现有四场景与冒烟断言零覆盖这两条路径，不受影响；App 失败路径为纯增强；死代码删除零行为影响
- **不做**（延后登记）：存储 init 语义收口（seed 门控 + ADR-5 索引重建锚点——Phase-3 存储层重构的设计前提，见审计报告候选 6）；sidecar `handleLine` 抽取与分发器注册表（Phase-8）；auth 中间件双职责拆分（Phase-8）；AppState 构造器与字段私有化（Phase-3）；spawn_blocking 桥接样板收敛（Phase-3 随存储重构自然吸收）
