//! 捕获域：capture 提交输入矩阵（text / image / file tagged union）与操作卡片
//! schema（PRD §7.3：entries / event / todos[] / provenance.processing / group_name
//! / at_source）。

use crate::common::{AtSource, ProvenanceSource, Timestamp};
use crate::error::ErrorEnvelope;
use crate::ids::{CaptureId, CardId, EventId, NoteId, ProjectId, TodoId};
use crate::notes::NoteFormat;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// 捕获状态（降级总原则：LLM 不可达 / 解析失败 → degraded，原文入 inbox，永不阻塞捕获）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    /// 已提交
    Submitted,
    /// 处理中（确定性预处理 + Agent 结构化）
    Processing,
    /// 卡片就绪（待审）
    CardReady,
    /// 已降级（原文入 inbox 待处理队列）
    Degraded,
}

/// 捕获提交输入矩阵（tagged union：type 判别）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CaptureSubmit {
    /// 文本粘贴（短文本即正文；>8K 分块 → card.batch 多 entries）
    Text { text: String },
    /// 截图（原图入附件库；OCR 全文 + Agent 摘要）
    Image {
        /// 图片原始字节（base64 编码）
        data_base64: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        filename: Option<String>,
    },
    /// 文件（原文件存储 + SHA-256 去重；V1 元数据卡）
    File {
        /// 文件原始字节（base64 编码）
        data_base64: String,
        filename: String,
        /// MIME 类型
        #[serde(default, skip_serializing_if = "Option::is_none")]
        media_type: Option<String>,
    },
}

/// 捕获提交响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaptureResponse {
    pub capture_id: CaptureId,
    pub status: CaptureStatus,
}

/// 捕获状态查询响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaptureStatusResponse {
    pub id: CaptureId,
    pub status: CaptureStatus,
    /// 关联待审卡片（status = card_ready 时存在）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_id: Option<CardId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

// ---------------------------------------------------------------------------
// 操作卡片（PRD §7.3）
// ---------------------------------------------------------------------------

/// 卡片状态（状态机：pending → executed / discarded）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CardStatus {
    /// 待审（卡片永不自动入库）
    Pending,
    /// 已执行
    Executed,
    /// 已丢弃
    Discarded,
}

/// 项目推断置信度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// 卡片条目动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CardAction {
    /// 创建笔记
    CreateNote,
    /// 保存文件
    SaveFile,
    /// 仅添加事件（无正文）
    AddEvent,
    /// 保存凭证（密码形态检测触发；原文段替换为占位符）
    SaveCredential,
}

/// 卡片执行时的处理通道事实（标注 UI 读取此字段而非设置页当前值）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardProcessing {
    /// 模型名（如 deepseek-chat / ollama:qwen）
    pub model: String,
    /// 是否云端通道
    pub cloud: bool,
    /// 是否经出站脱敏
    pub redacted: bool,
}

/// 卡片条目溯源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardProvenance {
    pub source: ProvenanceSource,
    /// 原始来源（截图聚合时为「截图 ×N · 已按顺序合并」）
    pub origin: String,
    pub imported: bool,
    /// 执行时写入的事实数据（待审卡片无此字段）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processing: Option<CardProcessing>,
}

/// 卡片事件行（已发生 → Event）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardEvent {
    /// 事件类型（模板供给，如「问题」「会议」）
    pub r#type: String,
    /// 阶段名
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub at: Timestamp,
    /// 时间取值来源（Agent 推断时间时必填）
    pub at_source: AtSource,
}

/// 卡片待办行（未来 → Todo；执行时创建实体进待办中心，原文待办行保留）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardTodo {
    pub title: String,
    /// 截止时间（解析失败置空：不猜日期）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due: Option<Timestamp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// 卡片条目（PRD §7.3 entries[]）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardEntry {
    pub action: CardAction,
    pub title: String,
    /// 目标目录
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<NoteFormat>,
    /// 标签建议
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 分组名建议（Agent 识别系列事件时填入，用户可改）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    /// 事件行（action = create_note / add_event 时可携带）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<CardEvent>,
    /// 待办行
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub todos: Option<Vec<CardTodo>>,
    pub provenance: CardProvenance,
    /// Markdown 正文
    pub body: String,
}

/// 卡片内容（tagged union；「card.batch」为预留批量容器，单条目卡片亦用此形态）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum CardContent {
    /// 批量容器：一张卡片 N 条目 + 一份汇总（长文本分块 / 投递聚合）
    #[serde(rename = "card.batch")]
    Batch {
        /// 项目推断结果
        #[serde(default, skip_serializing_if = "Option::is_none")]
        project: Option<ProjectId>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        confidence: Option<Confidence>,
        /// 推断理由（如「内容提及 AMS 系统 UAT 环境」）
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
        entries: Vec<CardEntry>,
    },
}

/// 卡片响应。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardResponse {
    pub id: CardId,
    pub status: CardStatus,
    pub content: CardContent,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 卡片对话调整（PATCH；改草稿不落库，免逐轮确认）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardPatch {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 条目整体替换（对话调整的载体）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entries: Option<Vec<CardEntry>>,
}

/// 卡片执行结果（入库产物定位）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CardExecutionResult {
    pub card_id: CardId,
    pub note_ids: Vec<NoteId>,
    pub event_ids: Vec<EventId>,
    pub todo_ids: Vec<TodoId>,
}

// ---------------------------------------------------------------------------
// path stub
// ---------------------------------------------------------------------------

/// 提交捕获（确定性预处理：图片→OCR、文件→存储+SHA-256 去重、文本→直通）。
#[utoipa::path(
    post,
    path = "/api/capture",
    tag = "capture",
    request_body = CaptureSubmit,
    responses(
        (status = 202, description = "已受理（异步处理）", body = CaptureResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub fn submit_capture() {}

/// 查询捕获状态（轮询至 card_ready / degraded）。
#[utoipa::path(
    get,
    path = "/api/captures/{capture_id}",
    tag = "capture",
    params(
        ("capture_id" = CaptureId, Path, description = "捕获 ID（cap_ 前缀）"),
    ),
    responses(
        (status = 200, description = "捕获状态", body = CaptureStatusResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "捕获不存在", body = ErrorEnvelope),
    )
)]
pub fn read_capture() {}

/// 读取操作卡片。
#[utoipa::path(
    get,
    path = "/api/cards/{card_id}",
    tag = "capture",
    params(
        ("card_id" = CardId, Path, description = "卡片 ID（crd_ 前缀）"),
    ),
    responses(
        (status = 200, description = "卡片详情", body = CardResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "卡片不存在", body = ErrorEnvelope),
    )
)]
pub fn read_card() {}

/// 卡片对话调整（改草稿不落库）。
#[utoipa::path(
    patch,
    path = "/api/cards/{card_id}",
    tag = "capture",
    params(
        ("card_id" = CardId, Path, description = "卡片 ID（crd_ 前缀）"),
    ),
    request_body = CardPatch,
    responses(
        (status = 200, description = "调整后的卡片", body = CardResponse),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "卡片不存在", body = ErrorEnvelope),
        (status = 409, description = "卡片已执行 / 已丢弃，不可调整", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub fn update_card() {}

/// 执行卡片（唯一确认与入库触发点：create_note + add_timeline_event + todos + FTS 索引）。
#[utoipa::path(
    post,
    path = "/api/cards/{card_id}/execute",
    tag = "capture",
    params(
        ("card_id" = CardId, Path, description = "卡片 ID（crd_ 前缀）"),
    ),
    responses(
        (status = 200, description = "执行结果（入库产物定位）", body = CardExecutionResult),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "卡片不存在", body = ErrorEnvelope),
        (status = 409, description = "卡片已执行 / 已丢弃", body = ErrorEnvelope),
    )
)]
pub fn execute_card() {}

/// 丢弃卡片。
#[utoipa::path(
    post,
    path = "/api/cards/{card_id}/discard",
    tag = "capture",
    params(
        ("card_id" = CardId, Path, description = "卡片 ID（crd_ 前缀）"),
    ),
    responses(
        (status = 204, description = "已丢弃"),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 404, description = "卡片不存在", body = ErrorEnvelope),
        (status = 409, description = "卡片已执行，不可丢弃", body = ErrorEnvelope),
    )
)]
pub fn discard_card() {}

#[cfg(test)]
mod tests {
    use super::*;

    /// PRD §7.3 样例卡片（逐字段对齐）。
    fn sample_card_json() -> serde_json::Value {
        serde_json::json!({
            "id": "crd_01J8Z3A7B4C5D6E7F8G9H0JKMN",
            "status": "pending",
            "content": {
                "type": "card.batch",
                "project": "prj_01J8Z3A7B4C5D6E7F8G9H0JKMN",
                "confidence": "high",
                "reason": "内容提及 AMS 系统 UAT 环境",
                "entries": [{
                    "action": "create_note",
                    "title": "UAT 环境数据库连接问题处理",
                    "target_dir": "问题与变更/",
                    "format": "markdown",
                    "tags": ["环境", "数据"],
                    "group_name": "UAT 环境搭建",
                    "event": {
                        "type": "问题",
                        "stage": "测试验证",
                        "at": "2025-06-12T06:30:00Z",
                        "at_source": "content_time"
                    },
                    "todos": [{ "title": "部署手册补充「监听自检」步骤" }],
                    "provenance": {
                        "source": "paste",
                        "origin": "微信群-客户群",
                        "imported": false
                    },
                    "body": "## 现象\n…"
                }]
            },
            "created_at": "2025-06-12T06:32:00Z",
            "updated_at": "2025-06-12T06:32:00Z"
        })
    }

    #[test]
    fn sample_card_deserializes_into_contract_type() {
        let card: CardResponse = serde_json::from_value(sample_card_json()).unwrap();
        assert_eq!(card.status, CardStatus::Pending);
        match &card.content {
            CardContent::Batch {
                project,
                confidence,
                reason,
                entries,
            } => {
                assert!(project.is_some());
                assert_eq!(*confidence, Some(Confidence::High));
                assert_eq!(reason.as_deref(), Some("内容提及 AMS 系统 UAT 环境"));
                assert_eq!(entries.len(), 1);
                let e = &entries[0];
                assert_eq!(e.action, CardAction::CreateNote);
                assert_eq!(e.group_name.as_deref(), Some("UAT 环境搭建"));
                let ev = e.event.as_ref().unwrap();
                assert_eq!(ev.at_source, AtSource::ContentTime);
                assert_eq!(ev.stage.as_deref(), Some("测试验证"));
                let td = e.todos.as_ref().unwrap();
                assert_eq!(td.len(), 1);
                assert!(td[0].due.is_none() && td[0].owner.is_none());
                assert!(e.provenance.processing.is_none(), "待审卡片无 processing");
            }
        }
        // round-trip：可选字段省略语义保持
        let back = serde_json::to_value(&card).unwrap();
        let orig = sample_card_json();
        assert_eq!(back, orig);
    }

    #[test]
    fn capture_submit_tagged_union() {
        let text: CaptureSubmit = serde_json::from_value(serde_json::json!({
            "type": "text", "text": "昨天和客户开了UAT评审会"
        }))
        .unwrap();
        match &text {
            CaptureSubmit::Text { text } => assert!(text.contains("UAT")),
            _ => panic!("应解析为 Text 变体"),
        }
        let img: CaptureSubmit = serde_json::from_value(serde_json::json!({
            "type": "image", "data_base64": "aGVsbG8="
        }))
        .unwrap();
        assert!(matches!(img, CaptureSubmit::Image { .. }));
        let file: CaptureSubmit = serde_json::from_value(serde_json::json!({
            "type": "file", "data_base64": "aGVsbG8=", "filename": "纪要.docx"
        }))
        .unwrap();
        assert!(matches!(file, CaptureSubmit::File { .. }));
        // 未知 type 拒绝
        assert!(serde_json::from_value::<CaptureSubmit>(serde_json::json!({
            "type": "audio", "data_base64": "x"
        }))
        .is_err());
    }

    #[test]
    fn capture_statuses_snake_case() {
        assert_eq!(
            serde_json::to_string(&CaptureStatus::CardReady).unwrap(),
            "\"card_ready\""
        );
        assert_eq!(
            serde_json::to_string(&CardStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&CardAction::SaveCredential).unwrap(),
            "\"save_credential\""
        );
    }
}
