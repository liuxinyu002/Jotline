//! Agent 工具注册表（工具层 = 领域 API；全量工具面随 Phase-9/10 工具设计定稿增补）。
//!
//! Phase-1 注册三个样例工具验证机制：list_projects / create_note / search_content。
//! gen bin 将 `params` Schema 的 `$ref` 解引用后写出自包含 JSON Schema。

use utoipa::openapi::schema::ObjectBuilder;
use utoipa::openapi::{RefOr, Schema};
use utoipa::PartialSchema;

use crate::notes::{CreateNoteRequest, SearchQuery};

/// 工具规格（静态表条目）。
pub struct ToolSpec {
    /// 工具名（Agent 调用面）
    pub name: &'static str,
    /// 工具描述（Agent 选择依据）
    pub description: &'static str,
    /// 参数 Schema（gen 期解引用为自包含 JSON Schema）
    pub params: fn() -> RefOr<Schema>,
}

/// 输出 Schema（无参数工具 = 空对象）。
fn empty_object() -> RefOr<Schema> {
    ObjectBuilder::new().into()
}

/// Phase-1 样例工具注册表。
pub fn tools() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "list_projects",
            description: "列出全部项目（含敏感开关与当前阶段信息），供项目推断与项目切换使用。",
            params: empty_object,
        },
        ToolSpec {
            name: "create_note",
            description: "在指定项目下创建笔记（Markdown 正文 + 可选目录 / 标签）。",
            params: || <CreateNoteRequest as PartialSchema>::schema(),
        },
        ToolSpec {
            name: "search_content",
            description: "全文检索（含 OCR 文本），返回带 snippet 与 provenance 的命中列表。",
            params: || <SearchQuery as PartialSchema>::schema(),
        },
    ]
}
