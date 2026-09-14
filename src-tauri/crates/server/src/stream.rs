//! SSE 事件流端点（`GET /api/stream`）。
//!
//! `#[utoipa::path]` 注解与摘要自 contracts stub 原样迁入（IDR-03）。
//! 事件行遵循 wire-protocol：`event:` 行 = 信封 type、`id:` 行 = event_id、
//! `data:` 行 = 信封 JSON；空闲期心跳注释行（`axum::response::sse::KeepAlive`）。

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use contracts::error::ErrorEnvelope;
use contracts::stream::StreamEnvelope;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::AppState;

/// 订阅事件流（主窗与浮窗同源；Mock 以此验证双窗一致性）。
#[utoipa::path(
    get,
    path = "/api/stream",
    tag = "stream",
    responses(
        (
            status = 200,
            description = "SSE 事件流（data 行为 StreamEnvelope JSON；空闲期心跳注释行）",
            content(
                (StreamEnvelope = "text/event-stream"),
            ),
        ),
        (status = 401, description = "未认证", body = ErrorEnvelope),
    )
)]
pub async fn get_stream(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    tracing::info!(target: "rust.stream", "SSE 已订阅");
    let rx = state.events.subscribe();
    let events = BroadcastStream::new(rx).filter_map(|msg| match msg {
        Ok(envelope) => Some(Ok(frame_event(&envelope))),
        // 落后（Lagged）：跳过错过的事件，下一个事件自然刷新消费侧
        Err(_lagged) => None,
    });
    Sse::new(DisconnectLogged { inner: events }).keep_alive(
        KeepAlive::new()
            .text("heartbeat")
            .interval(Duration::from_secs(15)),
    )
}

/// 信封 → SSE 帧三件套（event / id / data）。
fn frame_event(envelope: &StreamEnvelope) -> Event {
    let (event_type, event_id) = match envelope {
        StreamEnvelope::NoteCreated { event_id, .. } => ("note_created", *event_id),
        StreamEnvelope::NoteUpdated { event_id, .. } => ("note_updated", *event_id),
        StreamEnvelope::ProjectCreated { event_id, .. } => ("project_created", *event_id),
        StreamEnvelope::ProjectUpdated { event_id, .. } => ("project_updated", *event_id),
        StreamEnvelope::TodoCreated { event_id, .. } => ("todo_created", *event_id),
        StreamEnvelope::TodoUpdated { event_id, .. } => ("todo_updated", *event_id),
        StreamEnvelope::CaptureStatusChanged { event_id, .. } => {
            ("capture_status_changed", *event_id)
        }
    };
    Event::default()
        .event(event_type)
        .id(event_id.to_string())
        .json_data(envelope)
        .expect("StreamEnvelope 序列化不可失败")
}

/// 流包装：订阅断开（流被 drop，含客户端断连）时记日志（design 日志点）。
struct DisconnectLogged<S> {
    inner: S,
}

impl<S> Drop for DisconnectLogged<S> {
    fn drop(&mut self) {
        tracing::info!(target: "rust.stream", "SSE 已断开");
    }
}

impl<S: Stream + Unpin> Stream for DisconnectLogged<S> {
    type Item = S::Item;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.inner).poll_next(cx)
    }
}
