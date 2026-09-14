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

/// SSE 订阅路径（auth 中间件的 query-token 等价形态消费；
/// 下方 `#[utoipa::path]` 注解内的字面量与此同源——宏要求字面量，邻接维持局部性）。
pub const STREAM_PATH: &str = "/api/stream";

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use contracts::entities::Project;
    use contracts::notes::NoteResponse;
    use serde_json::json;
    use tokio_stream::once;

    /// 渲染单帧信封的线上文本（经 Sse 响应体，与真实发送路径同构）。
    async fn frame_text(envelope: StreamEnvelope) -> String {
        let sse = Sse::new(once(Ok::<_, Infallible>(frame_event(&envelope))));
        let bytes = axum::body::to_bytes(sse.into_response().into_body(), usize::MAX)
            .await
            .unwrap();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// 三件套格式（design D6）：信封 type → `event:` 行、event_id → `id:` 行、
    /// 信封 JSON → `data:` 行，帧尾空行分隔。
    #[tokio::test]
    async fn frame_event_renders_three_line_wire_format() {
        let note: NoteResponse = serde_json::from_value(json!({
            "id": "itm_01J8Z3NTE21000000000000000",
            "project_id": "prj_01J8Z3PRJ10000000000000000",
            "title": "UAT 冒烟测试记录",
            "body": "登录 / 下单 / 退款三链路冒烟全部通过。",
            "format": "markdown",
            "tags": ["环境"],
            "provenance": {"source": "manual", "origin": "手动创建", "imported": false},
            "created_at": "2025-06-12T06:32:00Z",
            "updated_at": "2025-06-12T06:32:00Z"
        }))
        .unwrap();
        let text = frame_text(StreamEnvelope::NoteCreated {
            event_id: 7,
            payload: note,
        })
        .await;

        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "event: note_created", "信封 type → event 行");
        assert_eq!(lines[1], "id: 7", "event_id → id 行");
        assert!(lines[2].starts_with("data: "), "信封 JSON → data 行");
        let payload: serde_json::Value =
            serde_json::from_str(&lines[2]["data: ".len()..]).unwrap();
        assert_eq!(payload["type"], "note_created");
        assert_eq!(payload["event_id"], 7);
        assert_eq!(payload["payload"]["title"], "UAT 冒烟测试记录");
        assert!(text.ends_with("\n\n"), "帧尾空行（SSE 帧分隔）");
    }

    /// 变体 → event 名映射抽查（ProjectUpdated → project_updated）。
    #[tokio::test]
    async fn frame_event_maps_variant_to_event_name() {
        let project: Project = serde_json::from_value(json!({
            "id": "prj_01J8Z3PRJ10000000000000000",
            "name": "AMS 产品实施项目",
            "sensitive": false,
            "template_id": "tpl_01J8Z3TPM10000000000000000",
            "created_at": "2025-06-01T02:00:00Z",
            "updated_at": "2025-06-10T08:00:00Z"
        }))
        .unwrap();
        let text = frame_text(StreamEnvelope::ProjectUpdated {
            event_id: 42,
            payload: project,
        })
        .await;
        assert!(text.starts_with("event: project_updated\nid: 42\n"));
    }
}
