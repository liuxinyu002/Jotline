//! ID 纪律与前缀注册表（SPEC §3.2 Wire 纪律）。
//!
//! 全部实体 ID 为 `<前缀>_<ULID>` 字符串（ULID = Crockford Base32，26 字符，去 I L O U）。
//! 前缀注册表随 `contracts/PROTOCOL.md` 登记；新增实体前缀必须先登记后使用。

use serde::{Deserialize, Deserializer, Serialize};
use utoipa::openapi::schema::{ObjectBuilder, Type};
use utoipa::openapi::{RefOr, Schema};
use utoipa::{PartialSchema, ToSchema};

/// ULID 主体的正则片段（Crockford Base32）。
pub const ULID_BODY: &str = "[0-9A-HJKMNP-TV-Z]{26}";

/// 校验 ULID 主体（26 字符 Crockford Base32）。
pub fn is_valid_ulid(ulid: &str) -> bool {
    ulid.len() == 26
        && ulid.bytes().all(|b| {
            matches!(
                b,
                b'0'..=b'9'
                    | b'A'..=b'H'
                    | b'J' | b'K'
                    | b'M' | b'N'
                    | b'P'..=b'T'
                    | b'V'..=b'Z'
            )
        })
}

/// 校验完整 ID（`<前缀>_<ULID>`）。
pub fn is_valid_id(prefix: &str, raw: &str) -> bool {
    match raw.split_once('_') {
        Some((p, ulid)) => p == prefix && is_valid_ulid(ulid),
        None => false,
    }
}

/// 定义一个实体 ID newtype（字符串 `<前缀>_<ULID>`，带 Schema pattern 与反序列化校验）。
macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[doc = concat!("实体 ID（`", $prefix, "_<ULID>`）。")]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// 注册表登记的前缀。
            pub const PREFIX: &'static str = $prefix;

            /// 从完整 ID 字符串解析（校验前缀与 ULID 格式）。
            pub fn parse(raw: &str) -> Result<Self, String> {
                if is_valid_id($prefix, raw) {
                    Ok(Self(raw.to_owned()))
                } else {
                    Err(format!(
                        concat!("非法 ", stringify!($name), "：{}（期望 `", $prefix, "_<26 位 ULID>`）"),
                        raw
                    ))
                }
            }

            /// 由裸 ULID 构造（自动拼接前缀）。
            pub fn from_ulid(ulid: &str) -> Result<Self, String> {
                if is_valid_ulid(ulid) {
                    Ok(Self(format!("{}_{}", $prefix, ulid)))
                } else {
                    Err(format!("非法 ULID：{ulid}（期望 26 位 Crockford Base32）"))
                }
            }

            /// 完整 ID 字符串。
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl std::str::FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::parse(s)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(d)?;
                $name::parse(&raw).map_err(serde::de::Error::custom)
            }
        }

        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> {
                ObjectBuilder::new()
                    .schema_type(Type::String)
                    .pattern(Some(concat!("^", $prefix, "_", "[0-9A-HJKMNP-TV-Z]{26}$")))
                    .examples([serde_json::json!(concat!($prefix, "_01J8Z3A7B4C5D6E7F8G9H0JKMN"))])
                    .into()
            }
        }

        impl ToSchema for $name {
            fn name() -> std::borrow::Cow<'static, str> {
                std::borrow::Cow::Borrowed(stringify!($name))
            }
        }
    };
}

// —— 前缀注册表（15 实体；与 contracts/PROTOCOL.md 一致，先登记后使用）——
define_id!(ProjectId, "prj"); // 项目
define_id!(StageId, "stg"); // 阶段
define_id!(TagId, "tag"); // 标签
define_id!(NoteId, "itm"); // 笔记 / 条目（item）
define_id!(FileVersionId, "ver"); // 文件版本
define_id!(EventId, "evt"); // 时间线事件
define_id!(CredentialId, "cred"); // 凭证
define_id!(TodoId, "tdo"); // 待办
define_id!(TimelineGroupId, "tlg"); // 时间线分组
define_id!(TemplateId, "tpl"); // 模板
define_id!(ExperienceId, "exp"); // 经验沉淀（V3 预留）
define_id!(CardId, "crd"); // 操作卡片
define_id!(CaptureId, "cap"); // 捕获
define_id!(MemoryId, "mem"); // 记忆条目
define_id!(AuditId, "aud"); // 审计记录

/// 跨实体 ID 引用（任意已登记前缀）——用于 source_ref 等指向多种实体的字段。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct IdStr(String);

impl IdStr {
    /// 解析（校验「三字母前缀 + ULID」形态）。
    pub fn parse(raw: &str) -> Result<Self, String> {
        let ok = raw.len() == 30
            && matches!(&raw[3..4], "_" )
            && raw[0..3].bytes().all(|b| b.is_ascii_lowercase())
            && is_valid_ulid(&raw[4..]);
        if ok {
            Ok(Self(raw.to_owned()))
        } else {
            Err(format!("非法实体 ID 引用：{raw}（期望 `<前缀>_<26 位 ULID>`）"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for IdStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for IdStr {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl<'de> Deserialize<'de> for IdStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        IdStr::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl PartialSchema for IdStr {
    fn schema() -> RefOr<Schema> {
        ObjectBuilder::new()
            .schema_type(Type::String)
            .pattern(Some("^[a-z]{3}_[0-9A-HJKMNP-TV-Z]{26}$".to_string()))
            .into()
    }
}

impl ToSchema for IdStr {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("IdStr")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrip() {
        let id = NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMN").unwrap();
        assert_eq!(id.as_str(), "itm_01J8Z3A7B4C5D6E7F8G9H0JKMN");
        assert_eq!(NoteId::PREFIX, "itm");
        // 前缀不匹配
        assert!(NoteId::parse("prj_01J8Z3A7B4C5D6E7F8G9H0JKMN").is_err());
        // 非法 ULID 字符（I / L / O / U 被排除）
        assert!(NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMI").is_err());
        assert!(NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKLM").is_err());
        assert!(NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMO").is_err());
        assert!(NoteId::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMU").is_err());
        // 长度
        assert!(NoteId::parse("itm_01J8Z3").is_err());
        assert!(NoteId::from_ulid("01J8Z3A7B4C5D6E7F8G9H0JKMN").is_ok());
        assert!(NoteId::from_ulid("01J8Z3").is_err());
    }

    #[test]
    fn id_str_accepts_any_prefix() {
        assert!(IdStr::parse("crd_01J8Z3A7B4C5D6E7F8G9H0JKMN").is_ok());
        assert!(IdStr::parse("itm_01J8Z3A7B4C5D6E7F8G9H0JKMN").is_ok());
        assert!(IdStr::parse("xxxx_01J8Z3A7B4C5D6E7F8G9H0JKMN").is_err());
    }

    #[test]
    fn serde_validates_on_deserialize() {
        let ok: NoteId = serde_json::from_str("\"itm_01J8Z3A7B4C5D6E7F8G9H0JKMN\"").unwrap();
        assert_eq!(ok.as_str(), "itm_01J8Z3A7B4C5D6E7F8G9H0JKMN");
        assert!(serde_json::from_str::<NoteId>("\"itm_bogus\"").is_err());
        let s = serde_json::to_string(&ok).unwrap();
        assert_eq!(s, "\"itm_01J8Z3A7B4C5D6E7F8G9H0JKMN\"");
    }
}
