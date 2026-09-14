//! 实体域：projects / stages / tags（全量，PRD 字段齐备）
//! + events / timeline_groups / templates / todos（骨架，核心字段）。

use crate::common::{AtSource, ListParams, PaginationMeta, Timestamp};
use crate::error::ErrorEnvelope;
use crate::ids::{
    EventId, IdStr, NoteId, ProjectId, StageId, TagId, TemplateId, TimelineGroupId, TodoId,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// ---------------------------------------------------------------------------
// projects（PRD FR-6：项目 = 阶段序列 + 目录树 + 事件类型 + 标签池；含 sensitive 标记）
// ---------------------------------------------------------------------------

/// 项目。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    /// 项目敏感开关（ON = 全链路本地模型；变更落审计）
    pub sensitive: bool,
    /// 项目源自的模板
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 项目创建 / 更新（`id` 缺省 = 新建）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProjectUpsert {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<ProjectId>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_id: Option<TemplateId>,
}

/// 项目列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct ProjectList {
    pub items: Vec<Project>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// stages（阶段序列）
// ---------------------------------------------------------------------------

/// 项目阶段（阶段序列由 position 决定）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Stage {
    pub id: StageId,
    pub project_id: ProjectId,
    pub name: String,
    /// 序列位置（从 1 起）
    pub position: u32,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 阶段创建 / 更新。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StageUpsert {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<StageId>,
    pub project_id: ProjectId,
    pub name: String,
    pub position: u32,
}

/// 阶段列表查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct StageListParams {
    /// 按项目过滤（必填）
    pub project_id: ProjectId,
}

/// 阶段列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct StageList {
    pub items: Vec<Stage>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// tags（全局池、多归属、使用计数）
// ---------------------------------------------------------------------------

/// 标签（全局池）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Tag {
    pub id: TagId,
    pub name: String,
    /// 使用计数（标签池面板展示）
    pub usage_count: u64,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 标签创建 / 更新（重命名 / 合并）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TagUpsert {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<TagId>,
    pub name: String,
}

/// 标签列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TagList {
    pub items: Vec<Tag>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// events（时间线事件，只增不改：纠错走作废 + 重建）
// ---------------------------------------------------------------------------

/// 时间线事件。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Event {
    pub id: EventId,
    pub project_id: ProjectId,
    /// 事件类型（模板供给，如「会议」「问题」）
    pub r#type: String,
    /// 阶段名（事件发生时所属阶段）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub at: Timestamp,
    /// 时间取值来源（SPEC §4.1：必附）
    pub at_source: AtSource,
    /// 分组名（Agent 建议系列事件时填入；无组事件不折叠）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_name: Option<String>,
    /// 关联笔记
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note_id: Option<NoteId>,
    /// 作废标记（只增不改：作废后保留可见）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voided_at: Option<Timestamp>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 事件列表查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct EventListParams {
    /// 按项目过滤（必填）
    pub project_id: ProjectId,
}

/// 事件列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct EventList {
    pub items: Vec<Event>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// timeline_groups（group_name 分组折叠）
// ---------------------------------------------------------------------------

/// 时间线分组。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TimelineGroup {
    pub id: TimelineGroupId,
    pub project_id: ProjectId,
    /// 分组名（= Event.group_name）
    pub name: String,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 分组列表查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct TimelineGroupListParams {
    /// 按项目过滤（必填）
    pub project_id: ProjectId,
}

/// 分组列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TimelineGroupList {
    pub items: Vec<TimelineGroup>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// templates（模板 bundle：阶段序列 + 目录树 + 事件类型 + 标签池）
// ---------------------------------------------------------------------------

/// 模板（预置「产品实施交付」，可编辑、可另存复用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Template {
    pub id: TemplateId,
    pub name: String,
    /// 阶段序列（有序）
    pub stage_names: Vec<String>,
    /// 目录树（模板预设路径）
    pub directory_paths: Vec<String>,
    /// 事件类型池
    pub event_types: Vec<String>,
    /// 标签池
    pub tag_names: Vec<String>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 模板列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TemplateList {
    pub items: Vec<Template>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// todos（待办：独立实体，与事件生命周期分离；status 为可扩展 enum，DEC-09）
// ---------------------------------------------------------------------------

/// 待办状态（基础二态；DEC-09 状态机细化随待办中心设计稿增补变体）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    /// 开放
    Open,
    /// 完成（可恢复）
    Done,
}

/// 待办来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TodoSource {
    /// 操作卡片待办行（Agent 解析）
    Card,
    /// 手动创建
    Manual,
    /// 批量导入（V2）
    Import,
}

/// 待办（活数据：逾期 = due_at 已过且未完成，不回写时间线事件）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Todo {
    pub id: TodoId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    pub title: String,
    /// 截止时间（解析失败置空：进卡片墙不进日历，不猜日期）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<Timestamp>,
    pub status: TodoStatus,
    /// 责任人（记录用途，不产生通知）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub source: TodoSource,
    /// 来源引用（如来源卡片的 crd_ ID）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<IdStr>,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
}

/// 待办列表查询。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct TodoListParams {
    /// 按项目过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<ProjectId>,
    /// 按状态过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TodoStatus>,
    /// 按来源过滤
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<TodoSource>,
}

/// 待办列表信封。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct TodoList {
    pub items: Vec<Todo>,
    pub page: PaginationMeta,
}

// ---------------------------------------------------------------------------
// path stub（Phase-2/3/5 实装时注解迁至真实 handler，openapi.json 零 diff 为迁移证明）
// ---------------------------------------------------------------------------

/// 创建 / 更新项目。
#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "entities",
    request_body = ProjectUpsert,
    responses(
        (status = 201, description = "项目已创建", body = Project),
        (status = 200, description = "项目已更新", body = Project),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub fn upsert_project() {}

/// 列出阶段（按 position 有序）。
#[utoipa::path(
    get,
    path = "/api/stages",
    tag = "entities",
    params(StageListParams),
    responses(
        (status = 200, description = "阶段序列", body = StageList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "查询参数校验失败", body = ErrorEnvelope),
    )
)]
pub fn list_stages() {}

/// 创建 / 更新阶段。
#[utoipa::path(
    post,
    path = "/api/stages",
    tag = "entities",
    request_body = StageUpsert,
    responses(
        (status = 201, description = "阶段已创建", body = Stage),
        (status = 200, description = "阶段已更新", body = Stage),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub fn upsert_stage() {}

/// 列出标签（全局池，含使用计数）。
#[utoipa::path(
    get,
    path = "/api/tags",
    tag = "entities",
    params(ListParams),
    responses(
        (status = 200, description = "标签池", body = TagList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
    )
)]
pub fn list_tags() {}

/// 创建 / 更新标签（重命名 / 合并）。
#[utoipa::path(
    post,
    path = "/api/tags",
    tag = "entities",
    request_body = TagUpsert,
    responses(
        (status = 201, description = "标签已创建", body = Tag),
        (status = 200, description = "标签已更新", body = Tag),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "请求体校验失败", body = ErrorEnvelope),
    )
)]
pub fn upsert_tag() {}

/// 列出时间线事件（含作废标记；导入回填视觉区分依据 imported）。
#[utoipa::path(
    get,
    path = "/api/events",
    tag = "entities",
    params(EventListParams),
    responses(
        (status = 200, description = "事件列表", body = EventList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "查询参数校验失败", body = ErrorEnvelope),
    )
)]
pub fn list_events() {}

/// 列出时间线分组（group_name 折叠）。
#[utoipa::path(
    get,
    path = "/api/timeline-groups",
    tag = "entities",
    params(TimelineGroupListParams),
    responses(
        (status = 200, description = "分组列表", body = TimelineGroupList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
        (status = 422, description = "查询参数校验失败", body = ErrorEnvelope),
    )
)]
pub fn list_timeline_groups() {}

/// 列出模板 bundle。
#[utoipa::path(
    get,
    path = "/api/templates",
    tag = "entities",
    params(ListParams),
    responses(
        (status = 200, description = "模板列表", body = TemplateList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
    )
)]
pub fn list_templates() {}

/// 列出待办（进行中 / 已完成；今日到期与逾期由消费侧按 due_at 计算）。
#[utoipa::path(
    get,
    path = "/api/todos",
    tag = "entities",
    params(TodoListParams),
    responses(
        (status = 200, description = "待办列表", body = TodoList),
        (status = 401, description = "未认证", body = ErrorEnvelope),
    )
)]
pub fn list_todos() {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_sensitive_flag_present() {
        let p = Project {
            id: ProjectId::parse("prj_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            name: "AMS 实施项目".into(),
            sensitive: true,
            template_id: Some(TemplateId::parse("tpl_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap()),
            created_at: "2025-06-12T06:32:00Z".parse().unwrap(),
            updated_at: "2025-06-12T06:32:00Z".parse().unwrap(),
        };
        let v: serde_json::Value = serde_json::to_value(&p).unwrap();
        assert_eq!(v["sensitive"], true);
        assert_eq!(v["template_id"], "tpl_01J8Z3A7B4C5D6E7F8G9H0JKMN");
    }

    #[test]
    fn todo_status_and_source_snake_case() {
        assert_eq!(
            serde_json::to_string(&TodoStatus::Open).unwrap(),
            "\"open\""
        );
        assert_eq!(
            serde_json::to_string(&TodoStatus::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&TodoSource::Card).unwrap(),
            "\"card\""
        );
    }

    #[test]
    fn event_at_source_required_on_wire() {
        let e = Event {
            id: EventId::parse("evt_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            project_id: ProjectId::parse("prj_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap(),
            r#type: "会议".into(),
            stage: Some("测试验证".into()),
            at: "2025-06-12T06:32:00Z".parse().unwrap(),
            at_source: AtSource::ContentTime,
            group_name: None,
            note_id: None,
            voided_at: None,
            created_at: "2025-06-12T06:32:00Z".parse().unwrap(),
            updated_at: "2025-06-12T06:32:00Z".parse().unwrap(),
        };
        let v: serde_json::Value = serde_json::to_value(&e).unwrap();
        assert_eq!(v["at_source"], "content_time");
        // optional 字段省略
        assert!(v.get("group_name").is_none());
        assert!(v.get("voided_at").is_none());
        let back: Event = serde_json::from_value(v).unwrap();
        assert_eq!(back, e);
    }
}
