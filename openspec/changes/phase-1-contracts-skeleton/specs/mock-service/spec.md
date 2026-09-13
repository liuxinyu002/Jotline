# Spec Delta: mock-service

## Purpose

定义契约 Mock 服务（开发期领域 API 替身）的可观测行为：按契约路由与校验、语义状态与 SSE 广播、剧本覆写（延迟/错误注入）与场景选择机制——支撑 FE 轨在真实后端就绪前验证「前端行为符合契约场景」。

## ADDED Requirements

### Requirement: 契约路由与场景校验

Mock 服务 SHALL 以生成的 openapi.json 为路由清单（不实现契约外的路由），并在启动时按契约 Schema 校验全部场景数据；校验失败 MUST 拒绝启动并指明违规场景与字段。

#### Scenario: 场景数据违反契约

- **WHEN** 某场景数据中 `projects` 条目缺少必填字段或类型不符
- **THEN** Mock 启动失败，错误信息定位到具体场景文件与字段

#### Scenario: 契约外路由不存在

- **WHEN** 请求 openapi.json 中未声明的路径
- **THEN** 返回 404（错误信封），不提供任何契约外响应

### Requirement: 语义状态

Mock SHALL 维护内存状态并由场景数据播种；写操作（create/update）MUST 实际变更内存状态，后续读操作反映变更后的状态——使「操作后另一窗口刷新一致」可验证。

#### Scenario: 创建后可读

- **WHEN** 通过 Mock 创建一条资源后立即列表查询
- **THEN** 列表包含新创建的资源（字段符合契约）

### Requirement: SSE 事件广播

状态因写操作变更时，Mock SHALL 向全部活跃 SSE 订阅者广播对应的注册事件（信封遵循 wire-protocol）；空闲期 SHALL 发送心跳注释行维持连接。

#### Scenario: 双窗同源验证

- **WHEN** 窗口 A 经 Mock 执行写操作，窗口 B 处于 SSE 订阅中
- **THEN** 窗口 B 收到对应事件通知，刷新后数据与窗口 A 一致

#### Scenario: 心跳

- **WHEN** SSE 连接空闲超过心跳间隔
- **THEN** 订阅者收到心跳行，连接不中断

### Requirement: 剧本覆写与故障注入

场景 SHALL 可声明剧本：对指定路由注入延迟、注入错误（状态码+错误信封）、或返回预定义响应序列；剧本优先于语义内核的默认行为。

#### Scenario: 延迟注入

- **WHEN** 激活的场景对某路由声明 2000ms 延迟
- **THEN** 该路由响应耗时不低于声明值，其余路由不受影响

#### Scenario: 错误注入

- **WHEN** 激活的场景对某路由声明 409 注入
- **THEN** 该路由返回 409 与契约错误信封，而非语义状态结果

### Requirement: 场景选择与重置

Mock SHALL 支持按请求头选择场景（仅影响该请求）与经控制端点切换全局激活场景、重置内存状态至种子；控制端点 MUST 不出现在 openapi.json（非契约面）。

#### Scenario: 请求级场景选择

- **WHEN** 请求携带场景选择头指定某场景
- **THEN** 该请求按指定场景的剧本响应，后续无头请求不受影响

#### Scenario: 全局重置

- **WHEN** 调用控制端点重置
- **THEN** 内存状态回到场景种子（此前写操作产生的数据消失）

### Requirement: 场景数据可提交性

场景数据集 SHALL 以文件形式入库（`contracts/mock/scenarios/`），其中 ID 使用固定 ULID（不运行时生成），保证 commit 稳定与可校验。

#### Scenario: 重复启动数据一致

- **WHEN** Mock 两次启动同一场景
- **THEN** 播种后的状态数据（含 ID）完全一致
