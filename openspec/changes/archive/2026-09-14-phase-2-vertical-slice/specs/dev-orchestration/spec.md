# Spec Delta: dev-orchestration

## Purpose

定义开发编排的可观测行为：`pnpm dev` 一键启动（主进程 + 前端 + sidecar 三进程运行时）、就绪探测、环境变量约定消费、全链退出与冒烟验证脚本——支撑 Roadmap「验证方式」的命令化执行。

## ADDED Requirements

### Requirement: 一键启动三进程运行时

`pnpm dev` 从仓库根执行时 SHALL 拉起主进程（监听 127.0.0.1:4765）与前端 dev server（127.0.0.1:1420），sidecar 由主进程负责拉起（编排脚本不直接管理 sidecar 生命周期）；启动完成后 SHALL 输出两端可访问地址。

#### Scenario: 启动后三进程在位

- **WHEN** 在仓库根执行 `pnpm dev` 并等待就绪
- **THEN** 主进程与前端 dev server 可访问，且进程列表中存在主进程拉起的 sidecar 子进程

### Requirement: 就绪探测

编排脚本 SHALL 在报告就绪前对主进程（经认证的只读 API）与前端（dev server 端口）分别完成就绪探测；探测超时 MUST 以非零退出码失败并输出诊断信息（哪个端点未就绪）。

#### Scenario: 就绪后立即可用

- **WHEN** `pnpm dev` 输出就绪信息
- **THEN** 此刻对 4765 的契约请求与对 1420 的页面访问均立即成功，无需额外等待

#### Scenario: 启动失败可诊断

- **WHEN** 主进程因配置错误无法就绪
- **THEN** 编排脚本在超时后非零退出，错误输出指明 4765 未就绪

### Requirement: 环境变量约定消费

端口、dev token 与数据目录 SHALL 经环境变量配置（`.env` 由编排脚本加载后注入子进程，各进程不自行读取 `.env` 文件）；相对路径的数据目录 `./vault` 锚定为仓库根下（编排脚本保证子进程工作目录一致）。

#### Scenario: .env 生效

- **WHEN** `.env` 中将 token 配置为非默认值并以该 token 请求主进程
- **THEN** 请求通过鉴权（配置被编排链完整消费）

### Requirement: 全链退出

`pnpm dev` 运行中收到中断信号（Ctrl-C / SIGINT）时，编排脚本 MUST 终止其拉起的全部子进程；主进程退出时 MUST 终止其拉起的 sidecar——不残留孤儿进程。

#### Scenario: Ctrl-C 无孤儿进程

- **WHEN** `pnpm dev` 运行中按下 Ctrl-C
- **THEN** 主进程、前端 dev server 与 sidecar 全部退出，进程列表无残留

### Requirement: 冒烟验证脚本

仓库 SHALL 提供一个脚本，机械执行 Roadmap Phase-2 的验证链——创建笔记（断言 201 与 ULID 响应）→ 文件落盘断言 → SQLite 索引行断言 → 检索命中断言 → 错误 token 401 断言——任一环节失败即非零退出并指明环节。

#### Scenario: 干净数据目录全链通过

- **WHEN** 在空数据目录上运行冒烟脚本
- **THEN** 全部断言通过、退出码 0，输出含各环节通过标记

#### Scenario: 断言失败定位

- **WHEN** 检索环节未命中预期内容（如索引未写入）
- **THEN** 脚本非零退出，错误输出定位到检索环节
