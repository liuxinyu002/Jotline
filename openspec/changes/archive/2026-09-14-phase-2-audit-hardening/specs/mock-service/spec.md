## ADDED Requirements

### Requirement: 响应侧契约校验
Mock SHALL 对语义内核产出的每个响应（状态码与响应体）按生成的 openapi.json 中对应操作的响应 Schema 校验；响应体违反契约时 MUST 以可见失败暴露（请求侧收到明确错误、日志记录违约字段），不得静默送出违约响应。剧本注入的预定义响应遵循既有场景数据校验要求，不在本要求范围。

#### Scenario: 语义内核产出违约响应
- **WHEN** 语义内核的写操作或读操作产出不符合契约 Schema 的响应体（如必填字段缺失）
- **THEN** Mock 不送出该违约响应，请求侧收到明确错误且日志记录违约字段，开发与冒烟阶段立即暴露

#### Scenario: 合规响应正常返回
- **WHEN** 语义内核产出符合契约 Schema 的响应
- **THEN** 校验通过、响应原样返回，行为与无校验时不可区分（dev 工具量级开销）

### Requirement: dev token 环境变量注入
Mock 的鉴权 token SHALL 经环境变量 `JOTLINE_DEV_TOKEN` 注入，未设置时缺省为 `dev-token`——与主进程、sidecar、前端三端的配置通道一致；修改 `.env` 中的 token 后，Mock 与其余三端 SHALL 同步生效，不得出现「改 token 后 Mock 模式全量 401」的配置断裂。

#### Scenario: 自定义 token 下 Mock 鉴权
- **WHEN** 以 `JOTLINE_DEV_TOKEN=custom-token` 启动 Mock，携带 `Authorization: Bearer custom-token` 请求任一契约路由
- **THEN** 鉴权通过，请求进入语义处理

#### Scenario: 缺省 token 回退
- **WHEN** 未设置 `JOTLINE_DEV_TOKEN` 启动 Mock（如 CI 环境），携带 `Authorization: Bearer dev-token` 请求
- **THEN** 鉴权通过（缺省值与文档固定值一致），既有冒烟链不受影响
