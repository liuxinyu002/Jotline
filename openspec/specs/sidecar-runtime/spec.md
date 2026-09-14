# sidecar-runtime Specification

## Purpose

定义 sidecar 进程骨架的可观测行为：被主进程 spawn、stdio JSON-RPC 健康检查往返、经 HTTP 回调主进程领域 API（ADR-2 通道验证）——进程模型为零监听、stdio 驱动；本阶段为骨架，pi agent 运行时与 provider 属后续阶段。

## Requirements

### Requirement: 由主进程 spawn 且零监听

sidecar SHALL 由主进程在启动后（领域 API 就绪后）经子进程方式拉起，以开发态脚本形式运行；sidecar MUST 不监听任何网络端口（零监听约束，ADR-2），与主进程的全部双向通信经 stdio 与对主进程的出站 HTTP 回调完成。

#### Scenario: 主进程启动即拉起 sidecar

- **WHEN** 主进程完成启动（领域 API 开始监听）
- **THEN** 系统进程列表中出现 sidecar 子进程，其父进程为主进程

### Requirement: 健康检查往返

主进程向 sidecar 发送 `health.ping` JSON-RPC 请求后，sidecar MUST 返回同一 id 的成功响应（JSON-RPC 2.0 信封，遵循 wire-protocol 的 stdio 协议）。

#### Scenario: ping-pong 往返

- **WHEN** 主进程发送 id 为任意字符串的 health ping
- **THEN** 收到同 id 的 pong 响应，result 含状态字段

### Requirement: HTTP 回调通道验证

sidecar 的回调目标（主进程端口与 token）SHALL 经主进程 spawn 时注入的环境变量获得，不占用 stdio 协议方法（零契约变更）；ping-pong 成功后 sidecar MUST 以携带 token 的请求回调 `GET /api/projects`，且回调请求携带可标识来源的请求头，使主进程侧可观测「通道验证完成」。

#### Scenario: 回调成功且可观测

- **WHEN** 主进程拉起 sidecar 并完成 ping-pong
- **THEN** 主进程日志出现 sidecar 来源的项目列表回调记录（含标识头），回调返回 200

#### Scenario: 配置不经协议通道

- **WHEN** sidecar 启动并完成回调
- **THEN** 全程未发生任何用于传递端口 / token 的 stdio 协议方法调用（注册表保持仅 health.ping 不变）

### Requirement: stdout 专用与日志走 stderr

sidecar 的 stdout SHALL 仅承载 JSON-RPC 消息（IPC 专用通道）；全部诊断日志 MUST 输出至 stderr，且 stderr 输出为结构化 NDJSON 行——stdout IPC 流中不得出现任何非 JSON-RPC 行。

#### Scenario: IPC 流纯净

- **WHEN** sidecar 运行期间持续读取其 stdout
- **THEN** 每一行均为可解析的 JSON-RPC 消息（请求 / 响应 / 通知），无日志行混入

#### Scenario: 诊断日志可采集

- **WHEN** sidecar 完成一次健康检查与回调
- **THEN** 其 stderr 出现对应的结构化日志行（含时间戳与级别字段）
