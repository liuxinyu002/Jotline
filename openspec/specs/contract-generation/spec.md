# contract-generation Specification

## Purpose

定义「契约源 → 生成物」管线的可观测行为：接口定义的单一事实源、确定性生成、三端生成物覆盖与防漂移校验，使「契约先行」由机制而非纪律保证。

## Requirements

### Requirement: 契约单一事实源

接口定义（HTTP 面、SSE 事件信封、stdio JSON-RPC 消息、Agent 工具 Schema） SHALL 只存在于 Rust 契约源中，全部生成物（openapi.json、前端 TS 类型、工具 Schema）MUST 由契约源经生成命令机械产出，禁止手写第二份。

#### Scenario: 修改契约源后未重新生成

- **WHEN** 契约源发生修改但生成物未重新生成即提交
- **THEN** `contracts:check` 以非零退出码显红，CI 拒绝该提交

#### Scenario: 重新生成后零 diff 通过

- **WHEN** 契约源修改后执行生成命令并 commit 全部生成物
- **THEN** `contracts:check` 零 diff 通过（退出码 0）

### Requirement: 确定性生成

生成命令对同一份契约源 MUST 产出字节级一致的生成物（键排序稳定、无时间戳或随机内容混入），否则 CI diff 检查不成立。

#### Scenario: 同源两次生成比对

- **WHEN** 在未修改契约源的情况下连续执行两次生成命令
- **THEN** 两次产物之间 `git diff` 为空

### Requirement: 生成物覆盖三端消费形态

生成管线 SHALL 同时产出：`contracts/openapi.json`（完整 OpenAPI 文档）、前端可导入的 TS 类型包、sidecar 可导入的工具 Schema 文件集；工具 Schema MUST 为自包含 JSON Schema（$ref 在生成期展开，不依赖外部引用）。

#### Scenario: 工具 Schema 自包含

- **WHEN** 读取任一生成的工具 Schema 文件
- **THEN** 其内部不含未解析的 `$ref` 指向 openapi.json 外部文件

#### Scenario: TS 类型可被前端工程导入

- **WHEN** 前端占位工程执行 typecheck
- **THEN** 对契约类型的导入（含 SSE 信封与 stdio 消息类型）零错误

### Requirement: 非 HTTP 类型并入同一管线

SSE 事件信封与 stdio JSON-RPC 消息类型 SHALL 与 HTTP 类型经由同一条生成管线产出 TS 类型，不另建第二条手写或独立生成的类型链。

#### Scenario: SSE 信封类型存在

- **WHEN** 生成完成后检查 TS 类型包
- **THEN** 可导入 SSE 事件信封类型，且其字段与 Rust 契约源定义一致

### Requirement: Wire 禁用清单的机器检查

契约源 MUST 不含 `untagged` enum、`serde(flatten)`、字段级 camelCase rename；生成管线（或其 CI 检查）SHALL 在契约源出现上述模式时以非零退出码失败。

#### Scenario: 契约源引入禁用模式

- **WHEN** 契约源某 enum 添加 `#[serde(untagged)]`
- **THEN** CI 检查失败并指明违规位置

### Requirement: path 声明的演进保真

Phase-1 以 stub 形式（带 utoipa 注解的空函数）承载 path 声明；后续阶段将注解迁移至真实 handler 时，MUST 保持 openapi.json 零 diff，作为迁移未引入漂移的证明。

#### Scenario: stub 迁移零 diff

- **WHEN** Phase-2 实装某域时将 path 注解从 stub 迁至真实 handler 并重新生成
- **THEN** openapi.json 与迁移前零 diff（除非该域契约本身同期有意变更）

### Requirement: 漂移防线覆盖契约源物理位置

契约漂移防线（本地 pre-commit 拦截）SHALL 覆盖契约注解的全部物理位置——包括契约 crate 内未实装域的 path stub，与已实装域 handler 所在的实现 crate（IDR-03 注解随实装迁移后，实现 crate 即契约源的主位置）；任一位置的注解源改动在未重新生成契约产物时，提交 MUST 被本地防线拦截，不得仅依赖 CI 外环兜底。

#### Scenario: 改动已实装域 handler 注解后直接提交

- **WHEN** 开发者修改实现 crate 内某已实装 handler 的 path 注解（如描述、参数或响应声明）后直接提交，未重新生成契约产物
- **THEN** pre-commit 契约检查触发，检出生成物与已提交版本的 diff，提交以非零退出被拦截

#### Scenario: 改动未实装域 stub 后直接提交

- **WHEN** 开发者修改契约 crate 内未实装域的 path stub 后直接提交，未重新生成契约产物
- **THEN** pre-commit 契约检查同样拦截（既有防线行为，作为回归保障）

### Requirement: 路由注册与文档注册端点集合一致

契约生成 SHALL 以机器可执行的守卫保证：服务路由注册的端点集合与 OpenAPI 文档注册的端点集合一致；一侧新增或遗漏端点导致集合不一致时，守卫 MUST 失败并指出差集，端点不得静默从契约产物中消失。

#### Scenario: 新增 handler 漏登记文档注册

- **WHEN** 开发者新增领域 handler 并完成路由注册，但未将其加入 OpenAPI 文档注册清单，运行一致性守卫
- **THEN** 守卫以非零退出失败，并指出该端点「路由已注册、文档未登记」的差集

#### Scenario: 两集合一致

- **WHEN** 路由注册与文档注册的端点集合完全一致
- **THEN** 一致性守卫通过（零退出），契约再生成产物的零 diff 检查不受影响
