//! 通用类型基座：时间别名、溯源、时间来源、分页与列表查询参数。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::{ObjectBuilder, SchemaFormat, Type};
use utoipa::openapi::{KnownFormat, RefOr, Schema};
use utoipa::{IntoParams, PartialSchema, ToSchema};

/// 线上时间字段统一类型：ISO-8601 UTC（RFC 3339），禁整数时间戳；时区转换仅在展示层。
///
/// newtype 包装（而非 type 别名）——别名不被 utoipa 宏展开，无法命中 chrono 已知类型映射。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub DateTime<Utc>);

impl Timestamp {
    pub fn parse_rfc3339(s: &str) -> Result<Self, String> {
        s.parse::<DateTime<Utc>>()
            .map(Timestamp)
            .map_err(|e| format!("非法 RFC 3339 时间：{e}"))
    }
}

impl std::str::FromStr for Timestamp {
    type Err = chrono::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<DateTime<Utc>>().map(Timestamp)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(t: DateTime<Utc>) -> Self {
        Self(t)
    }
}

impl std::ops::Deref for Timestamp {
    type Target = DateTime<Utc>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialSchema for Timestamp {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .format(Some(SchemaFormat::KnownFormat(KnownFormat::DateTime)))
            .examples([serde_json::json!("2025-06-12T06:32:00Z")])
            .into()
    }
}

impl ToSchema for Timestamp {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("Timestamp")
    }
}

/// 时间取值来源（SPEC §4.1 时间回退链的落库约束；`at ≠ capture_time` 时必填）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AtSource {
    /// 内容明示时间（①）
    ContentTime,
    /// 内容可推导时间（②，如「昨天」相对当天）
    DerivedTime,
    /// 捕获时间（③）
    CaptureTime,
}

/// 内容来源方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceSource {
    /// 文本粘贴
    Paste,
    /// 截图
    Screenshot,
    /// 文件拖入
    File,
    /// 手动创建（非捕获管线）
    Manual,
}

/// 溯源信息（笔记 / 检索命中通用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Provenance {
    pub source: ProvenanceSource,
    /// 原始来源描述（如「微信群-客户群」「截图 ×3 · 已按顺序合并」）。
    pub origin: String,
    /// 是否导入回填（与实时记录视觉区分的依据）。
    pub imported: bool,
}

/// 分页元信息（列表信封共用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    /// 总条数
    pub total: u64,
    /// 本页条数上限
    pub limit: u32,
    /// 偏移量
    pub offset: u32,
}

/// 列表端点共用查询参数。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, IntoParams, ToSchema)]
#[into_params(parameter_in = Query)]
pub struct ListParams {
    /// 本页条数上限（默认 50）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// 偏移量（默认 0）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamp_serializes_rfc3339_utc() {
        let t = Timestamp::parse_rfc3339("2025-06-12T06:32:00Z").unwrap();
        let s = serde_json::to_string(&t).unwrap();
        assert_eq!(s, "\"2025-06-12T06:32:00Z\"");
        // 带时区偏移的输入归一化为 UTC（Z 后缀）
        let t2 = Timestamp::parse_rfc3339("2025-06-12T14:32:00+08:00").unwrap();
        let s2 = serde_json::to_string(&t2).unwrap();
        assert_eq!(s2, "\"2025-06-12T06:32:00Z\"");
    }

    #[test]
    fn at_source_snake_case() {
        assert_eq!(
            serde_json::to_string(&AtSource::ContentTime).unwrap(),
            "\"content_time\""
        );
        assert_eq!(
            serde_json::to_string(&AtSource::DerivedTime).unwrap(),
            "\"derived_time\""
        );
        assert_eq!(
            serde_json::to_string(&AtSource::CaptureTime).unwrap(),
            "\"capture_time\""
        );
    }
}
