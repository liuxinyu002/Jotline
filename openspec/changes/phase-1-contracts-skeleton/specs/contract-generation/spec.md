# Spec Delta: contract-generation

## Purpose

定义「契约源 → 生成物」管线的可观测行为：接口定义的单一事实源、确定性生成、三端生成物覆盖与防漂移校验，使「契约先行」由机制而非纪律保证。

## ADDED Requirements

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
