# CONTEXT.md — Jotline 领域词汇表

> 命名与词汇归此文件；行为规约归 `openspec/specs/<capability>/spec.md`。两者冲突是分工问题，不是文件损坏。

## 能力（capability）种子

- contract-generation — 契约生成管线：Rust 契约源（类型 + utoipa 注解）经 gen 管线确定性产出三端产物（openapi.json / TS 类型包 / 工具 Schema），以零 diff 纪律防漂移。
- mock-service — 契约 Mock 替身：语义内核（创建 / 执行 / SSE 广播）+ 剧本覆写（延迟 / 错误注入 / 场景选择），Phase-11 前端连测基线。
- wire-protocol — 线上纪律：ID / 时间格式、错误信封、鉴权约定、SSE 事件信封、stdio JSON-RPC 注册表（PROTOCOL.md 为人读镜像）。

## 审计沉淀词汇（2026-09-14 架构审计）

- 契约防线 — 围绕契约零漂移的机器化守卫集合：pre-commit 触发集、contracts:check 零 diff、wire-grep 禁用清单、端点注册守卫。防线必须覆盖契约源的当前物理位置（IDR-03 之后注解住在 server crate）。
- 帧纪律 — SSE 行为约定：event / id / data 三件套格式、心跳间隔、event_id 单调性。契约只钳住 data 行的 JSON 形状（StreamEnvelope）；帧纪律靠等价性锚点（冒烟断言 / 单测）守卫，不靠契约。
- 响应侧校验 — Mock 对产出响应按契约 Schema 的运行时断言（对照入口校验：请求体 / 查询参数 / 场景种子）。违约启动即红，堵「语义内核产出绕过校验面」的缝。
