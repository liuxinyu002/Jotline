//! 错误信封与错误码（wire-protocol：全部非 2xx 响应使用统一信封）。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 机器可读错误码（登记表见 contracts/PROTOCOL.md；新增错误码先登记后使用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// 401：未携带 / 错误 Bearer token
    Unauthorized,
    /// 404：资源不存在（含契约外路由）
    NotFound,
    /// 409：状态冲突（如重复投递、已执行卡片再次执行）
    Conflict,
    /// 422：请求体 / 参数不符合契约 Schema
    ValidationFailed,
    /// 500：服务端内部错误
    Internal,
}

/// 校验失败的字段定位（422 信封 detail 条目）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct FieldError {
    /// 字段路径（如 `entries[0].title`）
    pub field: String,
    /// 失败原因（人读）
    pub message: String,
}

/// 统一错误信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ErrorEnvelope {
    /// 机器可读错误码
    pub code: ErrorCode,
    /// 人读错误信息
    pub message: String,
    /// 可选明细（422 校验失败时携带字段定位）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<Vec<FieldError>>,
}

impl ErrorEnvelope {
    /// 构造无明细错误。
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            detail: None,
        }
    }

    /// 构造带字段定位的校验失败（422）。
    pub fn validation(message: impl Into<String>, detail: Vec<FieldError>) -> Self {
        Self {
            code: ErrorCode::ValidationFailed,
            message: message.into(),
            detail: if detail.is_empty() {
                None
            } else {
                Some(detail)
            },
        }
    }

    /// 该错误码对应的 HTTP 状态码。
    pub fn status(&self) -> u16 {
        match self.code {
            ErrorCode::Unauthorized => 401,
            ErrorCode::NotFound => 404,
            ErrorCode::Conflict => 409,
            ErrorCode::ValidationFailed => 422,
            ErrorCode::Internal => 500,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_codes_snake_case_and_status() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::Unauthorized).unwrap(),
            "\"unauthorized\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::ValidationFailed).unwrap(),
            "\"validation_failed\""
        );
        assert_eq!(
            ErrorEnvelope::new(ErrorCode::Unauthorized, "x").status(),
            401
        );
        assert_eq!(ErrorEnvelope::new(ErrorCode::NotFound, "x").status(), 404);
        assert_eq!(ErrorEnvelope::new(ErrorCode::Conflict, "x").status(), 409);
        assert_eq!(
            ErrorEnvelope::new(ErrorCode::ValidationFailed, "x").status(),
            422
        );
        assert_eq!(ErrorEnvelope::new(ErrorCode::Internal, "x").status(), 500);
    }

    #[test]
    fn optional_detail_omitted() {
        let e = ErrorEnvelope::new(ErrorCode::NotFound, "笔记不存在");
        let s = serde_json::to_string(&e).unwrap();
        assert!(!s.contains("detail"), "无明细时字段应省略：{s}");
        let e2 = ErrorEnvelope::validation(
            "请求体校验失败",
            vec![FieldError {
                field: "title".into(),
                message: "不能为空".into(),
            }],
        );
        let s2 = serde_json::to_string(&e2).unwrap();
        assert!(s2.contains("\"field\":\"title\""), "{s2}");
    }
}
