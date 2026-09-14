# domain-api Specification

## Purpose

定义领域 API 真实运行时的可观测语义：本地监听与鉴权执行、笔记 create / read / search 与项目 list 的请求校验、响应与错误信封——全部遵循 wire-protocol 的协议约定（Wire 纪律、错误码、鉴权），本 spec 描述真实主进程的行为兑现。

## Requirements

### Requirement: 本地监听与鉴权执行

主进程领域 API SHALL 仅监听 `127.0.0.1:4765`；未携带或携带错误 Bearer token 的请求 MUST 返回 401 统一错误信封。

#### Scenario: 错误 token 被拒

- **WHEN** 以 `Authorization: Bearer wrong` 请求任一契约路由
- **THEN** 响应 401，错误信封 code 为 `unauthorized`

### Requirement: 创建笔记

`POST /api/notes` 接收符合契约的请求体（project_id / title / body 必填，target_dir / tags / format 可选省略）；有效请求 MUST 返回 201 与笔记响应（ID 为 `itm_` 前缀 ULID、时间字段为 ISO-8601 UTC、可选字段缺省时响应中省略）；请求体不符合契约（必填缺失、类型不符、ID 格式非法）MUST 返回 422 错误信封并含失败字段定位；project_id 格式合法但指向不存在的项目 MUST 返回 404。

#### Scenario: 创建成功

- **WHEN** 提交合法的创建请求（title「切片验证」、body「hello jotline」、指向 seed 项目）
- **THEN** 响应 201，返回体含 `itm_` 前缀 ULID id 与 RFC 3339 UTC 时间字段

#### Scenario: 必填字段缺失

- **WHEN** 提交缺少 body 的创建请求
- **THEN** 响应 422，错误信封 detail 定位到缺失字段，不产生任何文件与索引写入

#### Scenario: 项目不存在

- **WHEN** 提交 project_id 为合法 ULID 格式但无对应项目的创建请求
- **THEN** 响应 404，错误信封 code 为 `not_found`

### Requirement: 读取笔记

`GET /api/notes/{note_id}` 对存在的笔记 MUST 返回 200 与笔记响应（字段与创建响应同构）；不存在的笔记 ID MUST 返回 404 错误信封。

#### Scenario: 读取刚创建的笔记

- **WHEN** 以创建返回的 ID 请求读取
- **THEN** 响应 200，title / body 与创建请求一致

#### Scenario: 读取不存在的笔记

- **WHEN** 以随机合法格式但不存在的 ID 请求读取
- **THEN** 响应 404，错误信封 code 为 `not_found`

### Requirement: 全文检索

`GET /api/search` SHALL 对笔记标题与正文做全文检索，返回带 `id / title / snippet / provenance / score` 的命中列表（SPEC §5 接口承诺字段）；元数据过滤（project_id 等）MUST 生效；top_k 缺省为契约默认值。本阶段检索验证范围为英文词命中；中文分词方案属后续阶段定稿，不在本 spec 承诺内。

#### Scenario: 命中刚创建的笔记

- **WHEN** 创建 body 为「hello jotline」的笔记后以 `q=hello` 检索
- **THEN** 命中列表包含该笔记，snippet 含命中上下文，字段符合接口承诺

#### Scenario: 元数据过滤生效

- **WHEN** 以 `q=<词>&project_id=<seed 项目>` 检索且命中内容仅存在于 seed 项目
- **THEN** 结果不含其他项目的条目

### Requirement: 项目列表

`GET /api/projects` SHALL 返回项目列表信封（items + 分页元数据），内容与数据目录当前状态一致（含 dev seed 的示例项目）。

#### Scenario: 列表包含 seed 项目

- **WHEN** 主进程启动（含首启 seed）后请求项目列表
- **THEN** 响应 200，items 含固定 ID 的示例项目，字段符合契约
