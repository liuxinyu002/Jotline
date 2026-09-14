# wire-protocol Specification

## Purpose

定义三端（主进程 / sidecar / 前端）共享的线上协议约定：Wire 纪律、错误信封与错误码、鉴权、SSE 事件信封、stdio JSON-RPC 信封与 ID 前缀注册表——全部以 `contracts/PROTOCOL.md` 为文档载体、以契约源为机器定义。

## Requirements

### Requirement: ID 纪律与前缀注册表

全部实体 ID SHALL 为带注册前缀的 ULID 字符串（如 `itm_01J8Z3…`），前缀在 PROTOCOL.md 注册表中登记，新增实体前缀 MUST 先登记后使用。

#### Scenario: 场景数据 ID 校验

- **WHEN** Mock 场景数据中某实体 ID 前缀未在注册表登记或 ULID 格式非法
- **THEN** 场景数据校验失败并指明违规条目

### Requirement: 时间纪律

线上全部时间字段 SHALL 为 ISO-8601 UTC（RFC 3339）字符串，禁止整数时间戳；时区转换仅发生在展示层。

#### Scenario: 响应时间字段格式

- **WHEN** 任一 API 响应包含时间字段
- **THEN** 该字段为 RFC 3339 UTC 字符串（如 `2025-06-12T06:32:00Z`）

### Requirement: 判别联合与可选语义

判别联合 SHALL 用 tagged enum（判别字段显式命名）；可选字段 MUST 表达为字段省略（TS 侧 `field?: T`），禁止 required-nullable（`field: T | null`）；字段命名 snake_case。

#### Scenario: 可选字段生成形态

- **WHEN** 契约源某字段为可选
- **THEN** 生成的 TS 类型中该字段为 `field?: T`，而非 `field?: T | null` 或 `field: T | null`

#### Scenario: tagged enum 生成形态

- **WHEN** 契约源定义判别联合（如卡片 entries 的 action 类型）
- **THEN** 生成的 TS 类型为带判别字段的 union，可直接做 narrowing

### Requirement: 错误信封与错误码表

全部非 2xx 响应 SHALL 使用统一错误信封（含机器可读 `code` 与人读 `message`，可选 `detail`）；错误码在 PROTOCOL.md 错误码表中登记，新增错误码 MUST 先登记后使用。

#### Scenario: 未认证请求

- **WHEN** 请求未携带 Bearer token 或 token 错误
- **THEN** 响应 401，错误信封 code 为登记的认证错误码

#### Scenario: 请求体校验失败

- **WHEN** 请求体不符合契约 Schema
- **THEN** 响应 4xx，错误信封含校验错误码与失败字段定位

### Requirement: 鉴权约定

领域 API 仅监听 localhost；请求 MUST 携带 `Authorization: Bearer <token>`；SSE 端点 MUST 同时接受等价的 query 参数 token（`/api/stream?token=<token>`），因浏览器 `EventSource` 无法携带自定义请求头——两种形态的认证效力等价；开发环境固定 token 为 `dev-token`；Mock 服务与真实主进程遵循同一约定。

#### Scenario: dev-token 访问 Mock

- **WHEN** 以 `Authorization: Bearer dev-token` 请求 Mock 任一契约路由
- **THEN** 请求通过鉴权进入路由处理

#### Scenario: query token 订阅 SSE

- **WHEN** 以 `/api/stream?token=dev-token` 订阅 Mock 或真实主进程的 SSE 端点
- **THEN** 连接建立成功，后续收到的事件与 Bearer header 形态订阅完全一致

#### Scenario: 无凭证订阅 SSE 被拒

- **WHEN** 不带任何 token（header 与 query 均无）订阅 SSE 端点
- **THEN** 响应 401 统一错误信封

### Requirement: SSE 事件信封

SSE 流 SHALL 使用统一事件信封（含事件 `type` 与 `payload`，type 在 PROTOCOL.md 事件注册表登记）；事件类型集合可随契约版本增补，信封结构冻结。

#### Scenario: SSE 模拟流

- **WHEN** 订阅 Mock 的 SSE 端点
- **THEN** 依次收到注册表内的模拟事件后正常关闭

### Requirement: stdio JSON-RPC 协议

主进程与 sidecar 间 stdio 通信 SHALL 使用 JSON-RPC 2.0 信封（request / response / notification / error 结构齐备）；首批方法集 SHALL 包含健康检查（ping-pong）；后续方法在方法注册表增补，信封结构冻结。

#### Scenario: 健康检查往返

- **WHEN** 主进程向 sidecar 发送 health ping 请求
- **THEN** sidecar 返回同一 id 的成功响应（JSON-RPC 2.0 信封）

### Requirement: 协议文档与契约源同源变更

`contracts/PROTOCOL.md` SHALL 与契约源在同一变更单元（同一 PR）内同步修订，协议约定的变更不得先于或滞后于契约源。

#### Scenario: 错误码登记与使用同步

- **WHEN** 契约源新增一个错误码
- **THEN** PROTOCOL.md 错误码表在同一提交中含该登记
