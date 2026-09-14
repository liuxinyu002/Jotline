//! SSE 事件信封与首批事件（wire-protocol：统一信封结构冻结，事件集合可增补）。
//!
//! 传输形态：SSE `event:` 行 = 信封 `type`；`id:` 行 = `event_id`；`data:` 行 = 信封 JSON。

use crate::capture::CaptureStatus;
use crate::entities::{Project, Todo};
use crate::ids::CaptureId;
use crate::notes::NoteResponse;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// SSE 事件信封（tagged union：`type` 判别 + `event_id` + 类型化 `payload`）。
///
/// 事件注册表见 contracts/PROTOCOL.md；新增事件 = 新增变体（信封结构不变）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEnvelope {
    /// 笔记已创建
    NoteCreated {
        event_id: u64,
        payload: NoteResponse,
    },
    /// 笔记已更新（追加 / 合并）
    NoteUpdated {
        event_id: u64,
        payload: NoteResponse,
    },
    /// 项目已创建
    ProjectCreated { event_id: u64, payload: Project },
    /// 项目已更新（含敏感开关变更）
    ProjectUpdated { event_id: u64, payload: Project },
    /// 待办已创建
    TodoCreated { event_id: u64, payload: Todo },
    /// 待办已更新（勾销 / 恢复）
    TodoUpdated { event_id: u64, payload: Todo },
    /// 捕获状态变更（submitted → processing → card_ready / degraded）
    CaptureStatusChanged {
        event_id: u64,
        payload: CaptureStatusPayload,
    },
}

/// 捕获状态事件负载。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaptureStatusPayload {
    pub capture_id: CaptureId,
    pub status: CaptureStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_note() -> NoteResponse {
        NoteResponse {
            id: crate::ids::NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            project_id: crate::ids::ProjectId::parse("prj_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            title: "t".into(),
            body: "b".into(),
            target_dir: None,
            format: crate::notes::NoteFormat::Markdown,
            tags: vec![],
            provenance: crate::common::Provenance {
                source: crate::common::ProvenanceSource::Manual,
                origin: "manual".into(),
                imported: false,
            },
            created_at: "2025-06-12T06:32:00Z".parse().unwrap(),
            updated_at: "2025-06-12T06:32:00Z".parse().unwrap(),
        }
    }

    #[test]
    fn envelope_tagged_serialization() {
        let e = StreamEnvelope::NoteCreated {
            event_id: 1,
            payload: sample_note(),
        };
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["type"], "note_created");
        assert_eq!(v["event_id"], 1);
        assert!(v["payload"]["id"].is_string());
        // round-trip
        let back: StreamEnvelope = serde_json::from_value(v).unwrap();
        assert_eq!(back, e);
    }

    #[test]
    fn capture_status_event_roundtrip() {
        let e = StreamEnvelope::CaptureStatusChanged {
            event_id: 7,
            payload: CaptureStatusPayload {
                capture_id: crate::ids::CaptureId::parse("cap_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
                status: CaptureStatus::CardReady,
            },
        };
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.contains("\"capture_status_changed\""), "{s}");
        let back: StreamEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(back, e);
    }
}
