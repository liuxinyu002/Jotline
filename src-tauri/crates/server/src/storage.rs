//! vault 存储层（design D3/D4/D8/D9）：数据目录全布局初始化、SQLite 索引迁移、
//! dev 首启 seed、笔记真相源落盘与索引同步写、FTS5 检索。
//!
//! 文件为真相源（ADR-5）：`notes/<item_id>.md` 存正文；SQLite 仅为索引，可删除重建。
//! 全部函数为同步实现，调用侧经 `tokio::task::spawn_blocking` 隔离于 async runtime 之外。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::{SecondsFormat, Utc};
use contracts::common::{Provenance, ProvenanceSource, Timestamp};
use contracts::entities::Project;
use contracts::ids::{NoteId, ProjectId, TemplateId};
use contracts::notes::{CreateNoteRequest, NoteFormat, NoteResponse, SearchHit, SearchQuery};
use rusqlite::{Connection, OptionalExtension, ToSql};

/// dev 首启 seed 固定项目 ID（design D4：硬编码 ULID，验证命令可复述）。
pub const SEED_PROJECT_ID: &str = "prj_01J8Z3A7B4C5D6E7F8G9H0JKMN";

/// dev 首启 seed 项目名称。
pub const SEED_PROJECT_NAME: &str = "切片示例";

/// 存储层错误（文件 IO / SQLite）。
#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Db(rusqlite::Error),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Io(e) => write!(f, "文件 IO 失败：{e}"),
            StorageError::Db(e) => write!(f, "SQLite 失败：{e}"),
        }
    }
}

impl From<std::io::Error> for StorageError {
    fn from(e: std::io::Error) -> Self {
        StorageError::Io(e)
    }
}

impl From<rusqlite::Error> for StorageError {
    fn from(e: rusqlite::Error) -> Self {
        StorageError::Db(e)
    }
}

/// 创建笔记失败语义（404 项目不存在 / 500 其余）。
#[derive(Debug)]
pub enum CreateNoteError {
    ProjectNotFound(String),
    Storage(StorageError),
}

impl std::fmt::Display for CreateNoteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CreateNoteError::ProjectNotFound(id) => write!(f, "项目不存在：{id}"),
            CreateNoteError::Storage(e) => write!(f, "{e}"),
        }
    }
}

/// 存储句柄（单连接 `Mutex`：本地单用户低负载，design D3）。
pub struct Storage {
    conn: Mutex<Connection>,
    vault: PathBuf,
}

/// 时间存储键：固定纳秒精度 + Z 后缀（字典序 = 时间序，供 after/before 过滤比较）。
fn ts_key(t: &Timestamp) -> String {
    t.to_rfc3339_opts(SecondsFormat::Nanos, true)
}

fn ts_parse(s: &str) -> rusqlite::Result<Timestamp> {
    Timestamp::parse_rfc3339(s).map_err(|e| invalid_value(0, e))
}

fn invalid_value(idx: usize, msg: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, msg.into())
}

impl Storage {
    /// 初始化数据目录全布局（SPEC §3.1）+ 索引迁移 + dev 首启 seed。
    ///
    /// 幂等：布局已存在时不破坏既有内容（`IF NOT EXISTS` + seed 仅空库注入）。
    pub fn init(data_dir: &Path) -> Result<Self, StorageError> {
        // 六个子目录一次性建齐（design D8：Phase-3 直接消费）
        for dir in [
            "notes",
            "attachments",
            "inbox",
            "projects",
            "memory",
            "templates",
        ] {
            fs::create_dir_all(data_dir.join(dir))?;
        }
        // audit.jsonl 占位（已有内容不覆盖）
        let audit = data_dir.join("audit.jsonl");
        if !audit.exists() {
            fs::write(&audit, b"")?;
        }
        let conn = Connection::open(data_dir.join("index.sqlite"))?;
        migrate(&conn)?;
        seed(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
            vault: data_dir.to_path_buf(),
        })
    }

    /// 项目列表（分页元数据由调用侧组装信封）。
    pub fn list_projects(
        &self,
        limit: u32,
        offset: u32,
    ) -> Result<(Vec<Project>, u64), StorageError> {
        let conn = self.conn.lock().expect("storage mutex poisoned");
        let total: u64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, sensitive, template_id, created_at, updated_at
             FROM projects ORDER BY created_at, id LIMIT ?1 OFFSET ?2",
        )?;
        let items = stmt
            .query_map(
                rusqlite::params![i64::from(limit), i64::from(offset)],
                |row| {
                    Ok(Project {
                        id: ProjectId::parse(&row.get::<_, String>(0)?)
                            .map_err(|e| invalid_value(0, e))?,
                        name: row.get(1)?,
                        sensitive: row.get::<_, i64>(2)? != 0,
                        template_id: match row.get::<_, Option<String>>(3)? {
                            Some(s) => {
                                Some(TemplateId::parse(&s).map_err(|e| invalid_value(3, e))?)
                            }
                            None => None,
                        },
                        created_at: ts_parse(&row.get::<_, String>(4)?)?,
                        updated_at: ts_parse(&row.get::<_, String>(5)?)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((items, total))
    }

    /// 创建笔记（design D9 双写顺序）：
    /// 1) 真相源文件 `notes/<item_id>.md` 先落盘；2) `items` + `items_fts` 同一事务提交；
    /// 3) 事务失败 → 删除孤儿文件并报错（不留「文件在索引缺」中间态）。
    pub fn create_note(&self, req: &CreateNoteRequest) -> Result<NoteResponse, CreateNoteError> {
        let inner = || -> Result<NoteResponse, CreateNoteError> {
            let now = Timestamp::from(Utc::now());
            let now_str = ts_key(&now);
            let note = NoteResponse {
                id: NoteId::from_ulid(&ulid::Ulid::new().to_string())
                    .map_err(|e| CreateNoteError::Storage(StorageError::Db(invalid_value(0, e))))?,
                project_id: req.project_id.clone(),
                title: req.title.clone(),
                body: req.body.clone(),
                target_dir: req.target_dir.clone(),
                format: req.format.unwrap_or(NoteFormat::Markdown),
                tags: req.tags.clone().unwrap_or_default(),
                provenance: Provenance {
                    source: ProvenanceSource::Manual,
                    origin: "manual".into(),
                    imported: false,
                },
                created_at: now,
                updated_at: now,
            };

            {
                let conn = self.conn.lock().expect("storage mutex poisoned");
                // 项目存在性校验（404 语义；未写任何文件）
                let exists = conn
                    .query_row(
                        "SELECT 1 FROM projects WHERE id = ?1",
                        [&note.project_id.as_str()],
                        |_| Ok(()),
                    )
                    .optional()
                    .map_err(|e| CreateNoteError::Storage(e.into()))?
                    .is_some();
                if !exists {
                    return Err(CreateNoteError::ProjectNotFound(
                        note.project_id.as_str().to_owned(),
                    ));
                }
            }

            // 1) 真相源文件（正文 verbatim；SPEC vault-storage「文件真相源落盘」）
            let file = self.note_file(&note.id);
            fs::write(&file, &note.body)
                .map_err(|e| CreateNoteError::Storage(StorageError::Io(e)))?;

            // 2) 索引事务（items + items_fts 同一事务）
            let tx_result = self.with_tx(|tx| {
                tx.execute(
                    "INSERT INTO items (id, project_id, title, body, target_dir, format, tags, provenance, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    rusqlite::params![
                        note.id.as_str(),
                        note.project_id.as_str(),
                        note.title,
                        note.body,
                        note.target_dir,
                        "markdown",
                        serde_json::to_string(&note.tags).expect("tags 序列化不可失败"),
                        serde_json::to_string(&note.provenance).expect("provenance 序列化不可失败"),
                        now_str,
                    ],
                )?;
                let rowid = tx.last_insert_rowid();
                tx.execute(
                    "INSERT INTO items_fts (rowid, title, body) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rowid, note.title, note.body],
                )?;
                Ok(())
            });

            if let Err(e) = tx_result {
                // 3) 事务失败 → 删除孤儿文件
                let _ = fs::remove_file(&file);
                return Err(CreateNoteError::Storage(e.into()));
            }
            Ok(note)
        };
        let note = inner()?;
        tracing::info!(target: "rust.notes", note_id = %note.id, "笔记已创建");
        Ok(note)
    }

    fn with_tx<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> rusqlite::Result<T>,
    ) -> rusqlite::Result<T> {
        let mut conn = self.conn.lock().expect("storage mutex poisoned");
        let tx = conn.transaction()?;
        let out = f(&tx)?;
        tx.commit()?;
        Ok(out)
    }

    /// 笔记真相源文件路径（`<vault>/notes/<item_id>.md`）。
    fn note_file(&self, id: &NoteId) -> PathBuf {
        self.vault.join("notes").join(format!("{}.md", id.as_str()))
    }

    fn row_to_note(row: &rusqlite::Row<'_>) -> rusqlite::Result<NoteResponse> {
        let format_raw: String = row.get(5)?;
        if format_raw != "markdown" {
            return Err(invalid_value(5, format!("未知笔记格式：{format_raw}")));
        }
        let tags_raw: String = row.get(6)?;
        let provenance_raw: String = row.get(7)?;
        Ok(NoteResponse {
            id: NoteId::parse(&row.get::<_, String>(0)?).map_err(|e| invalid_value(0, e))?,
            project_id: ProjectId::parse(&row.get::<_, String>(1)?)
                .map_err(|e| invalid_value(1, e))?,
            title: row.get(2)?,
            body: row.get(3)?,
            target_dir: row.get(4)?,
            format: NoteFormat::Markdown,
            tags: serde_json::from_str(&tags_raw).map_err(|e| invalid_value(6, e.to_string()))?,
            provenance: serde_json::from_str(&provenance_raw)
                .map_err(|e| invalid_value(7, e.to_string()))?,
            created_at: Timestamp::parse_rfc3339(&row.get::<_, String>(8)?)
                .map_err(|e| invalid_value(8, e))?,
            updated_at: Timestamp::parse_rfc3339(&row.get::<_, String>(9)?)
                .map_err(|e| invalid_value(9, e))?,
        })
    }

    const NOTE_COLUMNS: &'static str =
        "id, project_id, title, body, target_dir, format, tags, provenance, created_at, updated_at";

    /// 读取笔记（从索引行组装响应；真相源在文件）。
    pub fn read_note(&self, id: &NoteId) -> Result<Option<NoteResponse>, StorageError> {
        use rusqlite::OptionalExtension;
        let conn = self.conn.lock().expect("storage mutex poisoned");
        conn.query_row(
            &format!("SELECT {} FROM items WHERE id = ?1", Self::NOTE_COLUMNS),
            [id.as_str()],
            Self::row_to_note,
        )
        .optional()
        .map_err(StorageError::Db)
    }

    /// 全文检索（FTS5 + snippet + 元数据过滤 + top_k；design D3）。
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<SearchHit>, StorageError> {
        let conn = self.conn.lock().expect("storage mutex poisoned");
        // FTS5 MATCH：短语引用（内嵌双引号转义），防查询语法注入
        let phrase = format!("\"{}\"", query.q.replace('"', "\"\""));

        let mut sql = String::from(
            "SELECT i.id, i.title, i.provenance,
                    snippet(items_fts, 1, '', '', '…', 16) AS snippet,
                    -bm25(items_fts) AS score
             FROM items_fts JOIN items i ON i.rowid = items_fts.rowid
             WHERE items_fts MATCH ?",
        );
        let mut params: Vec<Box<dyn ToSql>> = vec![Box::new(phrase)];

        let add_filter = |sql: &mut String,
                          params: &mut Vec<Box<dyn ToSql>>,
                          clause: String,
                          value: Box<dyn ToSql>| {
            sql.push_str(" AND ");
            sql.push_str(&clause);
            params.push(value);
        };

        if let Some(project_id) = &query.project_id {
            add_filter(
                &mut sql,
                &mut params,
                "i.project_id = ?".into(),
                Box::new(project_id.as_str().to_owned()),
            );
        }
        if let Some(target_dir) = &query.target_dir {
            add_filter(
                &mut sql,
                &mut params,
                "i.target_dir = ?".into(),
                Box::new(target_dir.clone()),
            );
        }
        if let Some(after) = &query.after {
            add_filter(
                &mut sql,
                &mut params,
                "i.created_at >= ?".into(),
                Box::new(ts_key(after)),
            );
        }
        if let Some(before) = &query.before {
            add_filter(
                &mut sql,
                &mut params,
                "i.created_at <= ?".into(),
                Box::new(ts_key(before)),
            );
        }
        if let Some(tags) = &query.tags {
            if !tags.is_empty() {
                let placeholders = vec!["?"; tags.len()].join(", ");
                let clause = format!(
                    "(SELECT COUNT(*) FROM json_each(i.tags) WHERE value IN ({placeholders})) = {}",
                    tags.len()
                );
                sql.push_str(" AND ");
                sql.push_str(&clause);
                for t in tags {
                    params.push(Box::new(t.clone()));
                }
            }
        }
        sql.push_str(" ORDER BY score DESC, i.id LIMIT ?");
        params.push(Box::new(i64::from(query.top_k.unwrap_or(10))));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|p| p.as_ref())),
                |row| {
                    Ok(SearchHit {
                        id: NoteId::parse(&row.get::<_, String>(0)?)
                            .map_err(|e| invalid_value(0, e))?,
                        title: row.get(1)?,
                        snippet: row.get(3)?,
                        provenance: serde_json::from_str(&row.get::<_, String>(2)?)
                            .map_err(|e| invalid_value(2, e.to_string()))?,
                        score: row.get(4)?,
                    })
                },
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }
}

// query_map 闭包列序修正辅助：provenance 在 SELECT 第 3 列、snippet 第 4 列（见上方 SQL）。
// （ rusqlite 闭包内列索引必须与 SELECT 顺序一致——见 search 实现。）

/// 索引迁移（`PRAGMA user_version` 版本标记；Phase-3 重构存储层时推倒重来）。
fn migrate(conn: &Connection) -> Result<(), StorageError> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version >= 1 {
        return Ok(());
    }
    conn.execute_batch(
        // items_fts 为外部内容模式（content='items'）：索引不重复存正文，
        // snippet() 可从内容表取原文（design D3）
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            sensitive INTEGER NOT NULL,
            template_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            target_dir TEXT,
            format TEXT NOT NULL,
            tags TEXT NOT NULL,
            provenance TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
            title, body, content='items', content_rowid='rowid'
        );
        PRAGMA user_version = 1;",
    )?;
    Ok(())
}

/// dev 首启 seed：数据目录无任何项目时创建固定 ULID 示例项目（design D4）。
fn seed(conn: &Connection) -> Result<(), StorageError> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }
    let now = ts_key(&Timestamp::from(Utc::now()));
    conn.execute(
        "INSERT INTO projects (id, name, sensitive, template_id, created_at, updated_at)
         VALUES (?1, ?2, 0, NULL, ?3, ?3)",
        rusqlite::params![SEED_PROJECT_ID, SEED_PROJECT_NAME, now],
    )?;
    tracing::info!(target: "rust.server", project_id = SEED_PROJECT_ID, "dev 首启 seed 已创建示例项目");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::ids::ProjectId;

    fn temp_storage() -> (tempfile::TempDir, Storage) {
        let dir = tempfile::tempdir().unwrap();
        let storage = Storage::init(dir.path()).unwrap();
        (dir, storage)
    }

    fn seed_project_id() -> ProjectId {
        ProjectId::parse(SEED_PROJECT_ID).unwrap()
    }

    fn create_req(title: &str, body: &str) -> CreateNoteRequest {
        CreateNoteRequest {
            project_id: seed_project_id(),
            title: title.into(),
            body: body.into(),
            target_dir: None,
            tags: None,
            format: None,
        }
    }

    #[test]
    fn init_creates_full_layout_and_tables() {
        let (dir, _storage) = temp_storage();
        for d in [
            "notes",
            "attachments",
            "inbox",
            "projects",
            "memory",
            "templates",
        ] {
            assert!(dir.path().join(d).is_dir(), "缺少子目录 {d}");
        }
        assert!(
            dir.path().join("audit.jsonl").is_file(),
            "缺少 audit.jsonl 占位"
        );
        let db = dir.path().join("index.sqlite");
        assert!(db.is_file(), "缺少 index.sqlite");
        let conn = Connection::open(&db).unwrap();
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert!(tables.contains(&"projects".into()), "{tables:?}");
        assert!(tables.contains(&"items".into()), "{tables:?}");
        assert!(tables.contains(&"items_fts".into()), "{tables:?}");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, 1);
    }

    #[test]
    fn seed_creates_fixed_project_once() {
        let (dir, _storage) = temp_storage();
        {
            let conn = Connection::open(dir.path().join("index.sqlite")).unwrap();
            let id: String = conn
                .query_row("SELECT id FROM projects", [], |r| r.get(0))
                .unwrap();
            assert_eq!(id, SEED_PROJECT_ID);
        }
        // 二次初始化：不重复 seed，不破坏既有内容
        Storage::init(dir.path()).unwrap();
        {
            let conn = Connection::open(dir.path().join("index.sqlite")).unwrap();
            let count: i64 = conn
                .query_row("SELECT COUNT(*) FROM projects", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 1);
        }
    }

    #[test]
    fn init_is_idempotent_on_existing_content() {
        let (dir, storage) = temp_storage();
        // 手工放入既有文件与目录内容，二次启动必须原样保留
        let note_file = dir.path().join("notes").join("itm_manual.md");
        fs::write(&note_file, "既有内容").unwrap();
        fs::write(dir.path().join("audit.jsonl"), "{\"keep\":true}\n").unwrap();
        drop(storage);
        Storage::init(dir.path()).unwrap();
        assert_eq!(fs::read_to_string(&note_file).unwrap(), "既有内容");
        assert_eq!(
            fs::read_to_string(dir.path().join("audit.jsonl")).unwrap(),
            "{\"keep\":true}\n"
        );
    }

    #[test]
    fn list_projects_returns_seed() {
        let (_dir, storage) = temp_storage();
        let (items, total) = storage.list_projects(50, 0).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].id.as_str(), SEED_PROJECT_ID);
        assert_eq!(items[0].name, SEED_PROJECT_NAME);
        assert!(!items[0].sensitive);
        assert!(items[0].template_id.is_none());
    }

    // —— 2.3 笔记写入与读取 ——

    #[test]
    fn create_note_writes_file_and_index_consistently() {
        let (dir, storage) = temp_storage();
        let note = storage
            .create_note(&create_req("切片验证", "hello jotline"))
            .unwrap();

        // ID：itm_ 前缀 ULID
        assert!(note.id.as_str().starts_with("itm_"), "{}", note.id.as_str());
        assert_eq!(note.id.as_str().len(), 30);
        // 时间：RFC 3339 UTC（Z 后缀）
        assert!(note
            .created_at
            .to_rfc3339_opts(SecondsFormat::AutoSi, true)
            .ends_with('Z'));
        // 可选字段缺省语义：target_dir 省略、tags 空、format 补默认
        assert!(note.target_dir.is_none());
        assert!(note.tags.is_empty());
        assert_eq!(note.format, NoteFormat::Markdown);

        // 真相源文件：内容含请求正文
        let file = dir
            .path()
            .join("notes")
            .join(format!("{}.md", note.id.as_str()));
        let content = fs::read_to_string(&file).unwrap();
        assert_eq!(content, "hello jotline");

        // 三处一致：items 行 + FTS 行 + 读取组装
        {
            let conn = Connection::open(dir.path().join("index.sqlite")).unwrap();
            let (title, body): (String, String) = conn
                .query_row(
                    "SELECT title, body FROM items WHERE id = ?1",
                    [note.id.as_str()],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert_eq!(title, "切片验证");
            assert_eq!(body, "hello jotline");
            let fts_count: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM items_fts JOIN items ON items.rowid = items_fts.rowid WHERE items.id = ?1",
                    [note.id.as_str()],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(fts_count, 1, "FTS 行缺失");
        }
        let read_back = storage.read_note(&note.id).unwrap().unwrap();
        assert_eq!(read_back.id, note.id);
        assert_eq!(read_back.title, "切片验证");
        assert_eq!(read_back.body, "hello jotline");
        assert_eq!(read_back.provenance.source, ProvenanceSource::Manual);
    }

    #[test]
    fn create_note_project_not_found_leaves_no_trace() {
        let (dir, storage) = temp_storage();
        let mut req = create_req("孤儿", "x");
        req.project_id = ProjectId::parse("prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ").unwrap();
        let err = storage.create_note(&req).unwrap_err();
        assert!(matches!(err, CreateNoteError::ProjectNotFound(_)));
        // 无文件写入
        let entries = fs::read_dir(dir.path().join("notes")).unwrap().count();
        assert_eq!(entries, 0);
        // 无索引行
        let conn = Connection::open(dir.path().join("index.sqlite")).unwrap();
        let items: i64 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert_eq!(items, 0);
    }

    #[test]
    fn read_note_missing_returns_none() {
        let (_dir, storage) = temp_storage();
        let missing = NoteId::parse("itm_01ZZZZZZZZZZZZZZZZZZZZZZZZ").unwrap();
        let found = storage.read_note(&missing).unwrap();
        assert!(found.is_none());
    }

    // —— 2.4 检索 ——

    fn search_q(storage: &Storage, q: &str) -> Vec<SearchHit> {
        storage
            .search(&SearchQuery {
                q: q.into(),
                ..SearchQuery::default()
            })
            .unwrap()
    }

    #[test]
    fn search_hits_recently_created_note() {
        let (_dir, storage) = temp_storage();
        storage
            .create_note(&create_req("切片验证", "hello jotline"))
            .unwrap();
        storage
            .create_note(&create_req("另一条", "oracle 连接串超时"))
            .unwrap();

        let hits = search_q(&storage, "hello");
        assert_eq!(hits.len(), 1);
        assert!(hits[0].id.as_str().starts_with("itm_"));
        assert_eq!(hits[0].title, "切片验证");
        assert!(hits[0].snippet.contains("hello"), "{}", hits[0].snippet);
        assert!(hits[0].score > 0.0);

        // 无命中 → 空列表
        assert!(search_q(&storage, "nonexistent").is_empty());
    }

    #[test]
    fn search_project_filter_excludes_other_projects() {
        let (_dir, storage) = temp_storage();
        // seed 项目一条 + 另一项目一条（直接写库造第二项目）
        {
            let conn = Connection::open(_dir.path().join("index.sqlite")).unwrap();
            conn.execute(
                "INSERT INTO projects (id, name, sensitive, template_id, created_at, updated_at)
                 VALUES ('prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ', '他项目', 0, NULL, '2025-01-01T00:00:00.000000000Z', '2025-01-01T00:00:00.000000000Z')",
                [],
            )
            .unwrap();
        }
        let mut other = create_req("他项目笔记", "hello from other project");
        other.project_id = ProjectId::parse("prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ").unwrap();
        storage
            .create_note(&create_req("本项目笔记", "hello in seed project"))
            .unwrap();
        storage.create_note(&other).unwrap();

        // 无过滤：两条命中
        assert_eq!(search_q(&storage, "hello").len(), 2);
        // project_id 过滤：仅 seed 项目
        let filtered = storage
            .search(&SearchQuery {
                q: "hello".into(),
                project_id: Some(seed_project_id()),
                ..SearchQuery::default()
            })
            .unwrap();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].title, "本项目笔记");
    }

    #[test]
    fn search_tags_and_top_k_filters() {
        let (_dir, storage) = temp_storage();
        let mut req = create_req("带标签", "tagged note about oracle");
        req.tags = Some(vec!["环境".into(), "数据".into()]);
        storage.create_note(&req).unwrap();
        storage
            .create_note(&create_req("无标签", "oracle again"))
            .unwrap();

        // tags 交集过滤（两标签都要满足）
        let both = storage
            .search(&SearchQuery {
                q: "oracle".into(),
                tags: Some(vec!["环境".into(), "数据".into()]),
                ..SearchQuery::default()
            })
            .unwrap();
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].title, "带标签");

        // 单标签交集不满足
        let none = storage
            .search(&SearchQuery {
                q: "oracle".into(),
                tags: Some(vec!["不存在".into()]),
                ..SearchQuery::default()
            })
            .unwrap();
        assert!(none.is_empty());

        // top_k 截断
        let limited = storage
            .search(&SearchQuery {
                q: "oracle".into(),
                top_k: Some(1),
                ..SearchQuery::default()
            })
            .unwrap();
        assert_eq!(limited.len(), 1);
    }

    #[test]
    fn search_treats_fts_operators_and_quotes_as_literal_phrases() {
        let (_dir, storage) = temp_storage();
        storage
            .create_note(&create_req("操作符", "hello OR world"))
            .unwrap();
        storage
            .create_note(&create_req("引号", "say \"hi\" jotline"))
            .unwrap();

        // q 含 FTS 操作符：按字面短语匹配，不产生查询语法错误
        let hits = search_q(&storage, "hello OR world");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "操作符");

        // q 含内嵌双引号：转义后仍命中原词
        let hits = search_q(&storage, "say \"hi\"");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "引号");

        // 字面短语语义：词序不连续不命中（非隐式 AND）
        assert!(search_q(&storage, "hello world").is_empty());
    }
}
