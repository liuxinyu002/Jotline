# Spec Delta: wire-protocol

## Purpose

（继承 phase-1 定义，无变更）三端共享的线上协议约定：Wire 纪律、错误信封与错误码、鉴权、SSE 事件信封、stdio JSON-RPC 信封与 ID 前缀注册表。

## MODIFIED Requirements

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
