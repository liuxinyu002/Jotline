//! projects 域 handler（列表）。
//!
//! `#[utoipa::path]` 注解与摘要自 contracts stub 原样迁入（IDR-03）。

use std::sync::Arc;

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::Json;
use contracts::common::{ListParams, PaginationMeta};
use contracts::entities::ProjectList;
use contracts::error::ErrorEnvelope;

use crate::error::ApiError;
use crate::AppState;

/// 列出项目。
#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "entities",
    params(ListParams),
    responses(
        (status = 200, description = "项目列表", body = ProjectList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
    )
)]
pub async fn list_projects(
    State(state): State<Arc<AppState>>,
    query: Result<Query<ListParams>, QueryRejection>,
) -> Result<Json<ProjectList>, ApiError> {
    let Query(params) = query
        .map_err(|e| ApiError::validation("查询参数校验失败", String::new(), e.to_string()))?;
    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);
    let storage = state.storage.clone();
    let (items, total) = tokio::task::spawn_blocking(move || storage.list_projects(limit, offset))
        .await
        .map_err(|e| ApiError::Internal(format!("存储任务失败：{e}")))?
        .map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok(Json(ProjectList {
        items,
        page: PaginationMeta {
            total,
            limit,
            offset,
        },
    }))
}
