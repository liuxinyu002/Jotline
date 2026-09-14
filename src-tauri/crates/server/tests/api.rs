//! in-process API 语义测试（tower::ServiceExt，不占端口）。
//!
//! 覆盖 domain-api / event-stream spec：鉴权 401、创建 201/404/422 全语义、
//! 读取 200/404、检索命中与过滤、项目列表、SSE 订阅与双订阅者同源。

use std::path::Path;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use serde_json::{json, Value};
use tokio::sync::watch;
use tower::ServiceExt;

use server::{storage::SEED_PROJECT_ID, AppState};

fn test_state(dir: &Path) -> Arc<AppState> {
    Arc::new(AppState {
        token: "dev-token".into(),
        storage: Arc::new(server::storage::Storage::init(dir).unwrap()),
        events: tokio::sync::broadcast::channel(64).0,
        event_seq: std::sync::atomic::AtomicU64::new(0),
        sidecar_boot_fired: watch::channel(false).0,
    })
}

/// 与 lib::router 同构的完整应用（真实路由 + 中间件栈）。
fn test_app(dir: &Path) -> Router {
    server::router(test_state(dir))
}

async fn body_json(res: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn request(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        builder = builder.header("authorization", format!("Bearer {t}"));
    }
    if let Some(b) = body {
        builder = builder.header("content-type", "application/json");
        return builder.body(Body::from(b.to_string())).unwrap();
    }
    builder.body(Body::empty()).unwrap()
}

const AUTH: Option<&str> = Some("dev-token");

async fn create_note(
    app: &Router,
    project_id: &str,
    title: &str,
    body: &str,
) -> (StatusCode, Value) {
    let res = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/notes",
            AUTH,
            Some(json!({"project_id": project_id, "title": title, "body": body})),
        ))
        .await
        .unwrap();
    let status = res.status();
    (status, body_json(res).await)
}

// ---------------------------------------------------------------------------
// 鉴权（真实路由）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn auth_rejects_missing_and_wrong_token() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // 无 token → 401 统一错误信封（真实契约路由）
    let res = app
        .clone()
        .oneshot(request("GET", "/api/projects", None, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    let v = body_json(res).await;
    assert_eq!(v["code"], "unauthorized");

    // 错 token → 401
    let res = app
        .clone()
        .oneshot(request("GET", "/api/projects", Some("wrong"), None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);

    // 正确 token → 200
    let res = app
        .oneshot(request("GET", "/api/projects", AUTH, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 创建笔记（3.1 全语义）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_note_201_ulid_and_rfc3339() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let (status, v) = create_note(&app, SEED_PROJECT_ID, "切片验证", "hello jotline").await;
    assert_eq!(status, StatusCode::CREATED);

    // ID：itm_ 前缀 ULID
    let id = v["id"].as_str().unwrap();
    assert!(id.starts_with("itm_") && id.len() == 30, "{id}");
    // 时间：RFC 3339 UTC（Z 后缀，允许亚秒）
    assert_rfc3339_utc(v["created_at"].as_str().unwrap());
    assert_rfc3339_utc(v["updated_at"].as_str().unwrap());
    // 可选字段缺省省略（target_dir 不出现）
    let raw = v.to_string();
    assert!(!raw.contains("target_dir"), "{raw}");
    // 必返字段在
    assert_eq!(v["format"], "markdown");
    assert_eq!(v["tags"], json!([]));
    assert_eq!(v["provenance"]["source"], "manual");
    assert_eq!(v["title"], "切片验证");
    assert_eq!(v["body"], "hello jotline");

    // 真相源文件 + 索引行（vault-storage spec）
    let file = dir.path().join("notes").join(format!("{id}.md"));
    assert_eq!(std::fs::read_to_string(file).unwrap(), "hello jotline");
}

/// RFC 3339 UTC 断言（chrono 解析 + Z 后缀）。
fn assert_rfc3339_utc(s: &str) {
    contracts::common::Timestamp::parse_rfc3339(s)
        .unwrap_or_else(|e| panic!("非法 RFC 3339：{s}（{e}）"));
    assert!(s.ends_with('Z'), "非 UTC Z 后缀：{s}");
}

#[tokio::test]
async fn create_note_422_missing_field() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // 缺 body
    let res = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/notes",
            AUTH,
            Some(json!({"project_id": SEED_PROJECT_ID, "title": "切片验证"})),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v = body_json(res).await;
    assert_eq!(v["code"], "validation_failed");
    assert_eq!(v["detail"][0]["field"], "body", "{v}");
    // 不产生任何文件与索引写入
    assert_eq!(
        std::fs::read_dir(dir.path().join("notes")).unwrap().count(),
        0
    );
}

#[tokio::test]
async fn create_note_422_type_mismatch_and_invalid_id() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // 类型不符：title 传数字
    let res = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/notes",
            AUTH,
            Some(json!({"project_id": SEED_PROJECT_ID, "title": 123, "body": "x"})),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v = body_json(res).await;
    assert_eq!(v["detail"][0]["field"], "title", "{v}");

    // ID 格式非法：project_id 非 ULID
    let res = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/notes",
            AUTH,
            Some(json!({"project_id": "prj_seed", "title": "t", "body": "x"})),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v = body_json(res).await;
    assert_eq!(v["detail"][0]["field"], "project_id", "{v}");

    // 非法 JSON（语法错误）→ 422
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/notes")
                .header("authorization", "Bearer dev-token")
                .header("content-type", "application/json")
                .body(Body::from("{\"broken"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_note_404_project_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let (status, v) = create_note(&app, "prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ", "孤儿", "x").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(v["code"], "not_found");
}

// ---------------------------------------------------------------------------
// 读取笔记
// ---------------------------------------------------------------------------

#[tokio::test]
async fn read_note_200_and_404() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let (_, created) = create_note(&app, SEED_PROJECT_ID, "切片验证", "hello jotline").await;
    let id = created["id"].as_str().unwrap().to_owned();

    let res = app
        .clone()
        .oneshot(request("GET", &format!("/api/notes/{id}"), AUTH, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["title"], "切片验证");
    assert_eq!(v["body"], "hello jotline");
    assert_eq!(v["id"], id.as_str());

    // 不存在（合法格式随机 ID）→ 404
    let res = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/notes/itm_01ZZZZZZZZZZZZZZZZZZZZZZZZ",
            AUTH,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_json(res).await["code"], "not_found");
}

// ---------------------------------------------------------------------------
// 检索
// ---------------------------------------------------------------------------

#[tokio::test]
async fn search_hits_with_snippet_and_filter() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());
    create_note(&app, SEED_PROJECT_ID, "切片验证", "hello jotline").await;

    // 命中：snippet 含关键词上下文，字段符合接口承诺
    let res = app
        .clone()
        .oneshot(request("GET", "/api/search?q=hello", AUTH, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    let hits = v.as_array().unwrap();
    assert_eq!(hits.len(), 1);
    assert!(hits[0]["snippet"].as_str().unwrap().contains("hello"));
    for k in ["id", "title", "snippet", "provenance", "score"] {
        assert!(hits[0].get(k).is_some(), "缺少字段 {k}");
    }

    // project_id 过滤生效
    let res = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/search?q=hello&project_id={SEED_PROJECT_ID}"),
            AUTH,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 1);
    let res = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/search?q=hello&project_id=prj_01ZZZZZZZZZZZZZZZZZZZZZZZZ",
            AUTH,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(body_json(res).await.as_array().unwrap().len(), 0);

    // 无命中 → 空列表
    let res = app
        .clone()
        .oneshot(request("GET", "/api/search?q=nonexistent", AUTH, None))
        .await
        .unwrap();
    assert_eq!(body_json(res).await, json!([]));

    // 缺 q → 422
    let res = app
        .clone()
        .oneshot(request("GET", "/api/search", AUTH, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ---------------------------------------------------------------------------
// 项目列表（3.2）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_projects_contains_seed() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let res = app
        .oneshot(request("GET", "/api/projects", AUTH, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let v = body_json(res).await;
    assert_eq!(v["page"]["total"], 1);
    assert_eq!(v["page"]["limit"], 50);
    assert_eq!(v["page"]["offset"], 0);
    let p = &v["items"][0];
    assert_eq!(p["id"], SEED_PROJECT_ID);
    assert_eq!(p["name"], "切片示例");
    assert_eq!(p["sensitive"], false);
    assert!(p["created_at"].as_str().unwrap().ends_with('Z'));
    assert!(p.get("template_id").is_none(), "可选字段应省略：{p}");
}

// ---------------------------------------------------------------------------
// SSE（4.1 / 4.2）
// ---------------------------------------------------------------------------

/// 订阅 SSE 并返回响应（流未消费，订阅已注册）。
async fn subscribe(app: &Router, query: &str) -> axum::response::Response {
    let res = app
        .clone()
        .oneshot(request("GET", &format!("/api/stream{query}"), None, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    assert!(res
        .headers()
        .get("content-type")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("text/event-stream"));
    res
}

/// 读取 SSE 流至收到第一条事件帧（event: <type> 行）。
async fn first_event_frame(res: axum::response::Response) -> String {
    use tokio_stream::StreamExt;
    let mut stream = res.into_body().into_data_stream();
    let mut buf = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.unwrap();
        buf.push_str(&String::from_utf8_lossy(&chunk));
        if buf.contains("\n\n") {
            break;
        }
    }
    buf
}

#[tokio::test]
async fn sse_query_token_and_broadcast_to_multiple_subscribers() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // query token 订阅（EventSource 等价形态）
    let sub_a = subscribe(&app, "?token=dev-token").await;
    // Bearer header 订阅
    let sub_b = app
        .clone()
        .oneshot(request("GET", "/api/stream", AUTH, None))
        .await
        .unwrap();

    // 订阅期间创建笔记 → 广播
    create_note(&app, SEED_PROJECT_ID, "切片验证", "hello jotline").await;

    // 两个并发订阅均收到同一条事件（顺序一致）
    let frame_a = first_event_frame(sub_a).await;
    let frame_b = first_event_frame(sub_b).await;
    for frame in [&frame_a, &frame_b] {
        assert!(frame.contains("event: note_created\n"), "{frame}");
        assert!(frame.contains("id: 1\n"), "{frame}");
        assert!(frame.contains("\"type\":\"note_created\""), "{frame}");
        assert!(frame.contains("\"event_id\":1"), "{frame}");
    }
    // 负载含笔记 ID（itm_ 前缀）
    assert!(frame_a.contains("itm_"), "{frame_a}");

    // 无 token 订阅 → 401
    let res = app
        .clone()
        .oneshot(request("GET", "/api/stream", None, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(res).await["code"], "unauthorized");

    // 错 query token → 401
    let res = app
        .clone()
        .oneshot(request("GET", "/api/stream?token=wrong", None, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// D9 双写失败补偿：事务失败 → 删孤儿文件 + 500 internal 信封
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_note_500_internal_and_orphan_file_removed_on_tx_failure() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // 经第二连接注入 ABORT trigger，制造真实索引事务失败（design D9 步骤 3）
    {
        let conn = rusqlite::Connection::open(dir.path().join("index.sqlite")).unwrap();
        conn.execute_batch(
            "CREATE TRIGGER fail_items_insert BEFORE INSERT ON items
             BEGIN SELECT RAISE(ABORT, 'harden 注入的事务失败'); END;",
        )
        .unwrap();
    }

    let (status, v) = create_note(&app, SEED_PROJECT_ID, "触发失败", "hello jotline").await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{v}");
    assert_eq!(v["code"], "internal", "{v}");

    // 补偿语义：不留「文件在索引缺」中间态——notes/ 无孤儿文件
    assert_eq!(
        std::fs::read_dir(dir.path().join("notes")).unwrap().count(),
        0,
        "事务失败后真相源文件应被删除"
    );
    // 索引亦无残留行
    let conn = rusqlite::Connection::open(dir.path().join("index.sqlite")).unwrap();
    let items: i64 = conn
        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(items, 0);
}

// ---------------------------------------------------------------------------
// 并发创建：Mutex 单连接 + spawn_blocking 路径下多请求一致性
// ---------------------------------------------------------------------------

#[tokio::test]
async fn concurrent_note_creates_remain_consistent() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    const N: usize = 8;
    let mut handles = Vec::new();
    for i in 0..N {
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            create_note(
                &app,
                SEED_PROJECT_ID,
                &format!("并发 {i}"),
                &format!("body {i}"),
            )
            .await
        }));
    }
    let mut ids = Vec::new();
    for h in handles {
        let (status, v) = h.await.unwrap();
        assert_eq!(status, StatusCode::CREATED, "{v}");
        ids.push(v["id"].as_str().unwrap().to_owned());
    }

    // ID 全部唯一
    let mut unique = ids.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), N, "{ids:?}");

    // 文件数 = 索引行数 = N
    assert_eq!(
        std::fs::read_dir(dir.path().join("notes")).unwrap().count(),
        N
    );
    let conn = rusqlite::Connection::open(dir.path().join("index.sqlite")).unwrap();
    let items: i64 = conn
        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
        .unwrap();
    assert_eq!(items, N as i64);
    let fts: i64 = conn
        .query_row("SELECT COUNT(*) FROM items_fts", [], |r| r.get(0))
        .unwrap();
    assert_eq!(fts, N as i64);
}

// ---------------------------------------------------------------------------
// 契约外路由 404 信封 + CORS 预检（task 1.2 / 1.3 此前仅手工 curl 验证）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_route_returns_404_envelope() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let res = app
        .clone()
        .oneshot(request("GET", "/not-a-contract-route", None, None))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
    let v = body_json(res).await;
    assert_eq!(v["code"], "not_found", "{v}");
}

#[tokio::test]
async fn cors_preflight_allows_localhost_origins_only() {
    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    // 预检（允许的 origin）：2xx + 回显 allow-origin
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/notes")
                .header("origin", "http://localhost:1420")
                .header("access-control-request-method", "POST")
                .header(
                    "access-control-request-headers",
                    "authorization,content-type",
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.status().is_success());
    assert_eq!(
        res.headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok()),
        Some("http://localhost:1420")
    );

    // 预检（不允许的 origin）：不回任何 CORS 头
    let res = app
        .oneshot(
            Request::builder()
                .method("OPTIONS")
                .uri("/api/notes")
                .header("origin", "http://evil.example.com")
                .header("access-control-request-method", "POST")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(res.headers().get("access-control-allow-origin").is_none());
}

// ---------------------------------------------------------------------------
// SSE 空闲心跳（event-stream spec「空闲连接不断开」；KeepAlive 15s 间隔）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sse_idle_connection_receives_heartbeat() {
    use std::time::Duration;
    use tokio_stream::StreamExt;

    let dir = tempfile::tempdir().unwrap();
    let app = test_app(dir.path());

    let res = subscribe(&app, "?token=dev-token").await;
    let mut stream = res.into_body().into_data_stream();

    // 无事件发生：25s 内应收到首条心跳注释行（: heartbeat）
    let deadline = tokio::time::Instant::now() + Duration::from_secs(25);
    let mut buf = String::new();
    let comment = loop {
        let now = tokio::time::Instant::now();
        assert!(now < deadline, "25s 内未见心跳注释行");
        let chunk = tokio::time::timeout(deadline - now, stream.next())
            .await
            .expect("心跳超时")
            .expect("连接提前关闭")
            .expect("流读取失败");
        buf.push_str(&String::from_utf8_lossy(&chunk));
        if let Some(line) = buf.lines().find(|l| l.starts_with(':')) {
            break line.to_owned();
        }
    };
    assert!(comment.contains("heartbeat"), "{comment}");
}
