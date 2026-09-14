//! 契约源库——三端共享接口定义的唯一事实源。
//!
//! 纪律（SPEC §3.2 Wire 纪律，发布后变更属 breaking）：
//! - ID = `<前缀>_<ULID>` 字符串；时间 = ISO-8601 UTC（RFC 3339），禁整数时间戳
//! - 判别联合 = tagged enum；禁 untagged / serde(flatten) / 字段级 camelCase rename
//! - 可选语义 = 字段省略（`skip_serializing_if = "Option::is_none"`）
//! - 字段命名 snake_case
//!
//! 生成物由 `src/bin/gen.rs` 产出并 commit 入库；禁止手写第二份。

pub mod capture;
pub mod common;
pub mod entities;
pub mod error;
pub mod ids;
pub mod ipc;
pub mod notes;
pub mod stream;
pub mod tools;

use utoipa::OpenApi;

/// OpenAPI 文档组装——全部 components 与 path stub 的注册点。
///
/// 注：SSE 信封（`StreamEnvelope`）与 stdio JSON-RPC 消息（`Ipc*`）为非 HTTP 类型，
/// 并入 components 经同一管线产出 TS 类型（design D3，quirk 见 PROTOCOL.md）。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Jotline 领域 API",
        version = "0.1.0",
        description = "本地单用户领域 API（127.0.0.1 + Bearer token）。本文件为生成物，契约源见 src-tauri/crates/contracts。"
    ),
    servers(
        (url = "http://127.0.0.1:4765", description = "Rust 主进程（dev）"),
    ),
    paths(
        // 已实装域的 path 注解已迁至 server crate 真实 handler（IDR-03 首次执行：
        // notes create/read/search、projects list、stream），本处仅保留未实装域 stub；
        // gen bin（server::gen）聚合两处产出 openapi.json
        entities::upsert_project,
        entities::list_stages, entities::upsert_stage,
        entities::list_tags, entities::upsert_tag,
        entities::list_events, entities::list_timeline_groups,
        entities::list_templates, entities::list_todos,
        capture::submit_capture, capture::read_capture,
        capture::read_card, capture::update_card, capture::execute_card, capture::discard_card,
    ),
    components(
        schemas(
            // 通用
            common::PaginationMeta, common::ListParams, common::Provenance,
            common::ProvenanceSource, common::AtSource,
            // 错误
            error::ErrorCode, error::ErrorEnvelope, error::FieldError,
            // ID 前缀注册表（15 实体 + 跨实体引用）
            ids::ProjectId, ids::StageId, ids::TagId, ids::NoteId, ids::FileVersionId,
            ids::EventId, ids::CredentialId, ids::TodoId, ids::TimelineGroupId, ids::TemplateId,
            ids::ExperienceId, ids::CardId, ids::CaptureId, ids::MemoryId, ids::AuditId,
            ids::IdStr,
            // 笔记基础域
            notes::NoteFormat, notes::CreateNoteRequest, notes::NoteResponse,
            notes::SearchQuery, notes::SearchHit,
            // SSE
            stream::StreamEnvelope, stream::CaptureStatusPayload,
            // stdio JSON-RPC
            ipc::IpcRequest, ipc::IpcResponse, ipc::IpcNotification, ipc::IpcError,
            ipc::IpcHealthRequest, ipc::IpcHealthResponse, ipc::IpcHealthResult,
            // 实体域
            entities::Project, entities::ProjectUpsert, entities::ProjectList,
            entities::Stage, entities::StageUpsert, entities::StageList, entities::StageListParams,
            entities::Tag, entities::TagUpsert, entities::TagList,
            entities::Event, entities::EventList, entities::EventListParams,
            entities::TimelineGroup, entities::TimelineGroupList, entities::TimelineGroupListParams,
            entities::Template, entities::TemplateList,
            entities::Todo, entities::TodoList, entities::TodoListParams,
            entities::TodoStatus, entities::TodoSource,
            // 捕获域
            capture::CaptureSubmit, capture::CaptureResponse, capture::CaptureStatus,
            capture::CaptureStatusResponse, capture::CardStatus, capture::CardResponse,
            capture::CardContent, capture::CardEntry, capture::CardEvent, capture::CardTodo,
            capture::CardProvenance, capture::CardProcessing, capture::Confidence,
            capture::CardAction, capture::CardPatch, capture::CardExecutionResult,
        )
    ),
    tags(
        (name = "notes", description = "笔记基础域（create / read / search）"),
        (name = "stream", description = "SSE 事件流"),
        (name = "entities", description = "实体域（projects / stages / tags / events / todos …）"),
        (name = "capture", description = "捕获与操作卡片"),
    )
)]
pub struct ApiDoc;
