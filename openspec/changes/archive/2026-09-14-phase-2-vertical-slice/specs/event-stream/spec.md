# Spec Delta: event-stream

## Purpose

定义真实主进程的 SSE 事件流行为：订阅与心跳维持、写操作后向全部订阅者广播、多订阅者数据同源——事件信封格式由 wire-protocol 冻结，本 spec 描述主进程的推送时机与订阅者可观测结果。

## ADDED Requirements

### Requirement: 订阅与心跳维持

`GET /api/stream` 认证通过后 SHALL 建立持久 SSE 连接（`text/event-stream`）；空闲期 MUST 周期性发送心跳注释行，保持连接不断开；事件行遵循 wire-protocol 的事件信封（`event:` 行 = 信封 type、`data:` 行 = 信封 JSON）。

#### Scenario: 空闲连接不断开

- **WHEN** 订阅者在超过心跳间隔的时间内保持连接且无事件发生
- **THEN** 订阅者收到心跳注释行，连接保持建立

### Requirement: 写操作事件广播

笔记创建的索引事务成功提交后（创建响应返回前），主进程 SHALL 向全部活跃 SSE 订阅者广播 `note_created` 事件，负载为符合契约的笔记响应。

#### Scenario: 创建后订阅者收到事件

- **WHEN** 订阅者 A 处于订阅中，另一客户端经 API 创建笔记成功
- **THEN** 订阅者 A 收到 `note_created` 事件，负载含新笔记的 id 与 title

### Requirement: 多订阅者数据同源

多个并发订阅者 SHALL 收到相同的事件序列；前端消费该事件刷新后，多个界面（多标签页）呈现的数据 MUST 与主进程当前状态一致——「多窗口数据同源」由事件广播保证，而非各端独立轮询。

#### Scenario: 双标签页同源

- **WHEN** 两个浏览器标签页同时订阅事件流，其一提交笔记创建
- **THEN** 两个标签页均收到 `note_created` 事件，刷新后列表一致

#### Scenario: EventSource 经 query token 订阅

- **WHEN** 前端以 `EventSource`（query 参数 token）订阅事件流
- **THEN** 连接建立成功且收到后续广播事件（与 Bearer header 形态等价）
