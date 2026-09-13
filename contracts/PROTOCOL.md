# Jotline 线上协议（PROTOCOL）

> 三端（主进程 / sidecar / 前端）共享的线上协议约定。**机器定义在契约源**
> （`src-tauri/crates/contracts`），本文档为人读细则；两者随同一变更单元（同一 PR）同步修订。
> OpenAPI 文档：`contracts/openapi.json`（OpenAPI **3.1.0**，spike S7 定稿）。

## 1. Wire 纪律（SPEC §3.2 冻结，发布后变更属 breaking）

| 纪律 | 机制 | 机器保证 |
|---|---|---|
| ID = `<前缀>_<ULID>` 字符串 | `Id` newtype（前缀 + Crockford Base32 26 字符） | Schema `pattern` + Mock 场景校验 |
| 时间 = ISO-8601 UTC（RFC 3339），禁整数时间戳 | `Timestamp` newtype（chrono `DateTime<Utc>`） | Schema `format: date-time` |
| 判别联合 = tagged enum | `#[serde(tag = "type")]` | TS 判别 union 可 narrowing |
| 可选 = 字段省略，禁 required-nullable | `skip_serializing_if = "Option::is_none"` + 生成期 null 剔除 | TS `field?: T`（无 `\| null`） |
| 字段命名 snake_case | Rust 字段天然成立 | `wire-grep.sh` 检查 rename |
| 禁 untagged enum / serde(flatten) | — | `wire-grep.sh`（CI 显红并定位） |

> **quirk（design D3）**：utoipa 对 `Option<T>` 生成 `oneOf: [{type: null}, X]` /
> `type: [T, null]` 可空联合，与「可选 = 字段省略」冲突。gen bin 在生成期统一剔除
> null 形态并修正查询参数 `required`（`enforce_wire_discipline`），TS 侧因此得到
> `field?: T` 而非 `field?: T | null`。

## 2. ID 前缀注册表（15 实体；先登记后使用）

| 前缀 | 实体 | 示例 |
|---|---|---|
| `prj` | Project 项目 | `prj_01J8Z3A7B4C5D6E7F8G9H0JKMN` |
| `stg` | Stage 阶段 | `stg_01J8Z3…` |
| `tag` | Tag 标签 | `tag_01J8Z3…` |
| `itm` | Item 笔记 / 条目 | `itm_01J8Z3…` |
| `ver` | FileVersion 文件版本 | `ver_01J8Z3…` |
| `evt` | Event 时间线事件 | `evt_01J8Z3…` |
| `cred` | Credential 凭证 | `cred_01J8Z3…` |
| `tdo` | Todo 待办 | `tdo_01J8Z3…` |
| `tlg` | TimelineGroup 时间线分组 | `tlg_01J8Z3…` |
| `tpl` | Template 模板 | `tpl_01J8Z3…` |
| `exp` | Experience 经验沉淀（V3 预留） | `exp_01J8Z3…` |
| `crd` | Card 操作卡片 | `crd_01J8Z3…` |
| `cap` | Capture 捕获 | `cap_01J8Z3…` |
| `mem` | Memory 记忆条目 | `mem_01J8Z3…` |
| `aud` | Audit 审计记录 | `aud_01J8Z3…` |

跨实体引用（如 `Todo.source_ref`）用 `IdStr`：`^[a-z]{3}_[0-9A-HJKMNP-TV-Z]{26}$`。

## 3. 时间格式细则

- 全部时间字段 = RFC 3339 UTC 字符串（`2025-06-12T06:32:00Z`）；带偏移输入由 chrono 归一化为 `Z` 后缀。
- `Event.at` 必附 `at_source ∈ {content_time, derived_time, capture_time}`（时间回退链落库约束）。
- 时区转换仅发生在展示层；日志时间戳（本地带偏移）与技术日志规范一致，不属线上协议。

## 4. 错误信封与错误码表

全部非 2xx 响应使用统一信封：

```json
{
  "code": "validation_failed",
  "message": "请求体校验失败",
  "detail": [{ "field": "entries[0].title", "message": "不能为空" }]
}
```

- `code`：机器可读（下表登记，先登记后使用）；`message`：人读；`detail`：可选（422 携带字段定位）。
- 契约外路由 = 404 + `not_found` 信封（Mock 路由层强制）。

| code | HTTP | 语义 |
|---|---|---|
| `unauthorized` | 401 | 未携带 / 错误 Bearer token |
| `not_found` | 404 | 资源不存在（含契约外路由） |
| `conflict` | 409 | 状态冲突（重复投递、已执行卡片再执行等） |
| `validation_failed` | 422 | 请求体 / 参数不符合契约 Schema |
| `internal` | 500 | 服务端内部错误 |

## 5. 鉴权约定

- 领域 API 仅监听 localhost：主进程 `127.0.0.1:4765`，Mock `127.0.0.1:4766`（`MOCK_PORT` 可覆盖）。
- 请求必须携带 `Authorization: Bearer <token>`；开发环境固定 `dev-token`。
- Mock 与真实主进程遵循同一约定（错 token → 401 `unauthorized` 信封）。

## 6. SSE 事件信封与首批事件注册表

端点：`GET /api/stream`（`text/event-stream`）。传输形态：

```
event: note_created
id: 1
data: {"type":"note_created","event_id":1,"payload":{…NoteResponse…}}

: heartbeat
```

- `event:` 行 = 信封 `type`；`id:` 行 = `event_id`（单调递增）；`data:` 行 = 信封 JSON。
- 信封结构冻结：`{type, event_id, payload}`；新增事件 = 新增 `type`（注册表登记）。
- 空闲期发送 `: heartbeat` 注释行维持连接。

事件注册表（首批）：

| type | payload | 触发 |
|---|---|---|
| `note_created` | `NoteResponse` | 笔记创建（card.execute / 直接创建） |
| `note_updated` | `NoteResponse` | 笔记更新（追加 / 合并） |
| `project_created` | `Project` | 项目创建 |
| `project_updated` | `Project` | 项目更新（含敏感开关） |
| `todo_created` | `Todo` | 待办创建 |
| `todo_updated` | `Todo` | 待办更新（勾销 / 恢复） |
| `capture_status_changed` | `CaptureStatusPayload` | 捕获状态机迁移 |

## 7. stdio JSON-RPC 方法注册表（主进程 ↔ sidecar）

信封：JSON-RPC 2.0（`IpcRequest` / `IpcResponse` / `IpcNotification` / `IpcError`）；
`id` 统一为 string；`jsonrpc` 固定 `"2.0"`。信封结构冻结。

| 方法 | 方向 | 定稿状态 |
|---|---|---|
| `health.ping` | 主进程 → sidecar | **定稿**（返回同 id 的 `IpcHealthResponse`，result.status = "ok"） |
| `capture.structured` | 主进程 → sidecar | 预留（Phase-5/10 捕获结构化） |
| `memory.extract` | 主进程 → sidecar | 预留（Phase-10 记忆提取） |

## 8. 工具 Schema

Agent 工具参数 Schema 生成自契约源（`contracts/tools/<name>.schema.json`），生成期
解引用 `$ref`，自包含可直接供 Ajv 2020-12 / 模型 function-calling 使用。工具注册表
（Phase-1 样例三枚）：`list_projects` / `create_note` / `search_content`；全量工具面
随 Phase-9/10 工具设计定稿增补（工具层 = 领域 API，SPEC §5）。

## 9. 非 HTTP 类型并入说明

`StreamEnvelope`（SSE 信封）与 `Ipc*`（stdio JSON-RPC 消息）非 HTTP 类型，但注册进
OpenAPI components 经同一管线产出 TS 类型（design D3：避免第二条 TS 生成链）。在
`openapi.json` 中它们位于 `components.schemas`，与 HTTP 类型无路径关联——这是有意为之。
