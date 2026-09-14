//! notes 域 handler（create / read / search）。
//!
//! `#[utoipa::path]` 注解与摘要自 contracts stub 原样迁入（IDR-03 首次执行，
//! openapi.json 零 diff 为迁移正确性证明）。

use std::sync::Arc;

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use contracts::error::ErrorEnvelope;
use contracts::ids::NoteId;
use contracts::notes::{CreateNoteRequest, NoteResponse, SearchHit, SearchQuery};
use serde_json::Value;

use crate::error::ApiError;
use crate::storage::CreateNoteError;
use crate::AppState;

/// 从反序列化错误消息提取字段名（`missing field \`x\`` 形态）。
fn extract_field(msg: &str) -> String {
    msg.split_once("missing field `")
        .map(|(_, rest)| rest.trim_end_matches('`').to_owned())
        .unwrap_or_default()
}

/// 创建笔记（Phase-2 实装：写 vault + SQLite 索引 + FTS）。
#[utoipa::path(
    post,
    path = "/api/notes",
    tag = "notes",
    request_body = CreateNoteRequest,
    responses(
        (status = 201, description = "笔记已创建", body = NoteResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub async fn create_note(
    State(state): State<Arc<AppState>>,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<NoteResponse>), ApiError> {
    let Json(value) =
        body.map_err(|e| ApiError::validation("请求体校验失败", String::new(), e.to_string()))?;
    // 经 serde_path_to_error 反序列化：422 信封携带字段路径定位
    let req: CreateNoteRequest = serde_path_to_error::deserialize(&value).map_err(|e| {
        // 顶层缺字段时 path 为 "."，从 serde 错误消息提取字段名
        let path = e.path().to_string();
        let field = if path.is_empty() || path == "." {
            extract_field(&e.inner().to_string())
        } else {
            path
        };
        ApiError::validation("请求体校验失败", field, e.inner().to_string())
    })?;

    let storage = state.storage.clone();
    let note = tokio::task::spawn_blocking(move || {
        storage.create_note(&req).map_err(|e| match e {
            CreateNoteError::ProjectNotFound(id) => ApiError::NotFound(format!("项目不存在：{id}")),
            CreateNoteError::Storage(err) => ApiError::Internal(err.to_string()),
        })
    })
    .await
    .map_err(|e| ApiError::Internal(format!("存储任务失败：{e}")))??;

    // 索引事务提交后、返回 201 前：广播 note_created（design D9 / event-stream spec）
    state.broadcast_note_created(&note);

    Ok((StatusCode::CREATED, Json(note)))
}

/// 读取笔记。
#[utoipa::path(
    get,
    path = "/api/notes/{note_id}",
    tag = "notes",
    params(
        ("note_id" = NoteId, Path, description = "笔记 ID（itm_ 前缀）"),
    ),
    responses(
        (status = 200, description = "笔记详情", body = NoteResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "笔记不存在", body = ErrorEnvelope),
    )
)]
pub async fn read_note(
    State(state): State<Arc<AppState>>,
    Path(note_id): Path<String>,
) -> Result<Json<NoteResponse>, ApiError> {
    // 非法格式（不可能存在的 ID）按 404 处理
    let note_id = NoteId::parse(&note_id)
        .map_err(|_| ApiError::NotFound(format!("笔记不存在：{note_id}")))?;
    let storage = state.storage.clone();
    let id = note_id.as_str().to_owned();
    let note = tokio::task::spawn_blocking(move || storage.read_note(&id))
        .await
        .map_err(|e| ApiError::Internal(format!("存储任务失败：{e}")))?
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    match note {
        Some(n) => Ok(Json(n)),
        None => Err(ApiError::NotFound(format!("笔记不存在：{note_id}"))),
    }
}

/// 全文检索（含 OCR 文本；元数据过滤；浮窗「查」与主窗共用同一接口）。
#[utoipa::path(
    get,
    path = "/api/search",
    tag = "notes",
    params(SearchQuery),
    responses(
        (status = 200, description = "命中列表（带 snippet / provenance）", body = [SearchHit]),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "查询参数校验失败", body = ErrorEnvelope),
    )
)]
pub async fn search_content(
    State(state): State<Arc<AppState>>,
    query: Result<Query<SearchQuery>, QueryRejection>,
) -> Result<Json<Vec<SearchHit>>, ApiError> {
    let Query(q) = query.map_err(|e| {
        ApiError::validation(
            "查询参数校验失败",
            extract_field(&e.to_string()),
            e.to_string(),
        )
    })?;
    let q_str = q.q.clone();
    let storage = state.storage.clone();
    let hits = tokio::task::spawn_blocking(move || storage.search(&q))
        .await
        .map_err(|e| ApiError::Internal(format!("存储任务失败：{e}")))?
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    tracing::debug!(target: "rust.notes", q = %q_str, hits = hits.len(), "检索完成");
    Ok(Json(hits))
}
