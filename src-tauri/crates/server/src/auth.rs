//! Bearer 鉴权中间件（wire-protocol 鉴权约定）：
//! - 全部 `/api/*` 契约路由要求 `Authorization: Bearer <token>`，错 / 缺 → 401 统一错误信封
//! - SSE 端点（`/api/stream`）同时接受等价的 query 参数 token（`EventSource` 无法携带自定义 header）
//! - 顺带观测 sidecar boot 回调（`X-Jotline-Sidecar` 标识头，design D6 / ADR-2 通道验证）

use std::sync::Arc;

use axum::extract::State;
use axum::http::{StatusCode, Uri};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::error::{ErrorCode, ErrorEnvelope};

use crate::AppState;

/// sidecar 回调标识头（boot 链：sidecar → 主进程 HTTP 回调时携带）。
pub const SIDECAR_HEADER: &str = "x-jotline-sidecar";

/// 从 query string 提取 `token` 参数（仅 dev-token 等 URL 安全值，不做解码）。
fn query_token(query: &str) -> Option<&str> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == "token").then_some(v)
    })
}

/// `/api/*` 鉴权中间件。
pub async fn require_bearer(
    State(state): State<Arc<AppState>>,
    uri: Uri,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let header_ok = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == format!("Bearer {}", state.token));

    // SSE 端点：query token 与 Bearer header 等价（EventSource 技术约束）
    let query_ok = uri.path() == "/api/stream"
        && uri
            .query()
            .and_then(query_token)
            .is_some_and(|t| t == state.token);

    if !header_ok && !query_ok {
        tracing::debug!(target: "rust.server", path = uri.path(), "鉴权拒绝");
        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorEnvelope::new(
                ErrorCode::Unauthorized,
                "未携带或错误的 Bearer token（dev 模式固定 dev-token）",
            )),
        )
            .into_response();
    }

    // sidecar boot 回调观测：通知拉起链（design D6）
    if req
        .headers()
        .get(SIDECAR_HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == "boot")
    {
        tracing::info!(target: "rust.sidecar", path = uri.path(), "sidecar 回调（X-Jotline-Sidecar: boot）");
        state.sidecar_boot_fired.send_if_modified(|v| {
            if *v {
                false
            } else {
                *v = true;
                true
            }
        });
    }

    next.run(req).await
}
