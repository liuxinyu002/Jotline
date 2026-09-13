//! 笔记基础域（Phase-2 垂直切片直接消费）：create / read / search。
//!
//! search 按 SPEC §5 冻结承诺逐字段对齐：
//! `(query, filters, top_k) → [{id, title, snippet, provenance, score}]`。

use crate::common::{Provenance, Timestamp};
use crate::error::ErrorEnvelope;
use crate::ids::{NoteId, ProjectId};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

/// 笔记格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NoteFormat {
    /// Markdown（知识类内容全文 Markdown）
    Markdown,
}

/// 创建笔记请求。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CreateNoteRequest {
    pub project_id: ProjectId,
    pub title: String,
    /// Markdown 正文
    pub body: String,
    /// 目标目录（单归属；缺省落模板默认目录）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_dir: Option<String>,
    /// 标签（Agent 优先复用已有标签）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<NoteFormat>,
}

/// 笔记响应（create / read 共用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct NoteResponse {
    pub id: NoteId,
    pub project_id: ProjectId,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_dir: Option<String>,
    pub format: NoteFormat,
    pub tags: Vec<String>,
    pub provenance: Provenance,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 检索请求（GET /api/search 查询参数；同时作为 search_content 工具的参数 Schema）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct SearchQuery {
    /// 检索词（口语化问句会被翻译为可检索词）
    pub q: String,
    /// 按项目过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 按目录过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_dir: Option<String>,
    /// 按标签过滤（多选交集）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    /// 时间范围下界（含）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Timestamp>,
    /// 时间范围上界（含）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Timestamp>,
    /// 返回条数上限（默认 10）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
}

/// 检索命中条目（SPEC §5 接口承诺，V1–V3 不变）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SearchHit {
    pub id: NoteId,
    pub title: String,
    /// 命中片段（关键词高亮素材）
    pub snippet: String,
    pub provenance: Provenance,
    /// 相关性得分
    pub score: f64,
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
pub fn create_note() {}

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
pub fn read_note() {}

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
pub fn search_content() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_note_request_optional_fields_omit() {
        let r = CreateNoteRequest {
            project_id: ProjectId::parse("prj_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            title: "切片验证".into(),
            body: "hello jotline".into(),
            target_dir: None,
            tags: None,
            format: None,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(
            !s.contains("target_dir") && !s.contains("tags") && !s.contains("format"),
            "{s}"
        );
        let back: CreateNoteRequest = serde_json::from_str(&s).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn search_hit_serialization() {
        let h = SearchHit {
            id: NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            title: "UAT 环境数据库连接问题处理".into(),
            snippet: "…oracle 连接串超时…".into(),
            provenance: crate::common::Provenance {
                source: crate::common::ProvenanceSource::Screenshot,
                origin: "微信群-客户群".into(),
                imported: false,
            },
            score: 0.87,
        };
        let v: serde_json::Value = serde_json::to_value(&h).unwrap();
        // SPEC §5 冻结承诺字段齐备
        for k in ["id", "title", "snippet", "provenance", "score"] {
            assert!(v.get(k).is_some(), "缺少字段 {k}");
        }
    }
}
