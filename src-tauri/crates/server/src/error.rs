//! 统一错误映射：`ApiError` → HTTP 状态 + 契约错误信封（`ErrorEnvelope`）。
//!
//! 错误码登记表见 contracts/PROTOCOL.md §4（401 / 404 / 422 / 500 均已登记）。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::error::{ErrorCode, ErrorEnvelope, FieldError};

/// 领域 API 错误（handler → 信封的唯一出口）。
#[derive(Debug)]
pub enum ApiError {
    /// 404：资源不存在
    NotFound(String),
    /// 422：请求体 / 参数校验失败（含字段定位）
    Validation(String, Vec<FieldError>),
    /// 500：服务端内部错误
    Internal(String),
}

impl ApiError {
    /// 构造 422（字段定位由 serde_path_to_error 的 path 给出）。
    pub fn validation(message: impl Into<String>, field: String, detail: String) -> Self {
        Self::Validation(
            message.into(),
            vec![FieldError {
                field,
                message: detail,
            }],
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, envelope) = match self {
            ApiError::NotFound(message) => (
                StatusCode::NOT_FOUND,
                ErrorEnvelope::new(ErrorCode::NotFound, message),
            ),
            ApiError::Validation(message, detail) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                ErrorEnvelope::validation(message, detail),
            ),
            ApiError::Internal(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorEnvelope::new(ErrorCode::Internal, message),
            ),
        };
        (status, Json(envelope)).into_response()
    }
}
