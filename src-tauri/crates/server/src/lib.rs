//! Jotline 主进程（Phase-2 垂直切片）：axum 领域服务 lib + thin bin（design D1/D2）。
//!
//! 形态：`serve(config)` 绑定监听后返回 [`Server`] 句柄（就绪即返回，不阻塞）；
//! Phase-11 将本 lib 挂进 Tauri 主进程 setup（`tokio::spawn`），bin 长期保留为
//! 后端独立调试入口。日志通道按 SPEC §5.1 定稿：tracing + tracing-subscriber 控制台渲染。
//!
//! 契约注解（IDR-03）：已实装域的 `#[utoipa::path]` 注解位于各 handler；未实装域
//! 仍在 contracts crate 的 path stub。gen bin 聚合两处产出 openapi.json（零 diff 纪律）。

pub mod auth;
pub mod error;
pub mod gendoc;
pub mod notes;
pub mod projects;
pub mod sidecar;
pub mod storage;
pub mod stream;

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::http::{header, Method, StatusCode, Uri};
use axum::middleware;
use axum::response::{IntoResponse, Response};
use axum::Json;
use contracts::error::{ErrorCode, ErrorEnvelope};
use contracts::notes::NoteResponse;
use contracts::stream::StreamEnvelope;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, watch};
use tower_http::cors::{AllowOrigin, CorsLayer};
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};

/// 领域 API 端口（Roadmap 总则 4：127.0.0.1:4765）。
pub const DEFAULT_PORT: u16 = 4765;
/// dev 固定 Bearer token（对齐 .env.example）。
pub const DEFAULT_TOKEN: &str = "dev-token";
/// 数据目录默认值（= vault 根，相对启动目录；design D8）。
pub const DEFAULT_DATA_DIR: &str = "./vault";

/// 仓库根定位（编译期锚定：server crate 上溯三级）+ `pnpm-workspace.yaml` 校验。
/// gen bin 与 sidecar 共用（design D8：单一 helper，不再各持一份）。
pub fn repo_root() -> Result<PathBuf, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .ok_or("无法定位仓库根目录")?;
    if !root.join("pnpm-workspace.yaml").exists() {
        return Err(format!("仓库根定位失败：{}", root.display()));
    }
    Ok(root.to_path_buf())
}

/// 主进程共享状态（领域 API 单实例）。
pub struct AppState {
    pub token: String,
    /// 存储句柄（vault 根 + SQLite 索引）。
    pub storage: Arc<storage::Storage>,
    /// SSE 广播通道（全部订阅者同源）。
    pub events: broadcast::Sender<StreamEnvelope>,
    /// 事件序号（单调递增 event_id）。
    pub event_seq: AtomicU64,
    /// sidecar boot 回调是否已观测（auth 中间件置位；拉起链等待，design D6）。
    pub sidecar_boot_fired: watch::Sender<bool>,
}

impl AppState {
    /// 广播 note_created（design D9：索引事务提交后、返回 201 前）。
    pub fn broadcast_note_created(&self, note: &NoteResponse) {
        let event_id = self.event_seq.fetch_add(1, Ordering::Relaxed) + 1;
        let _ = self.events.send(StreamEnvelope::NoteCreated {
            event_id,
            payload: note.clone(),
        });
    }
}

/// server 已实装域的 OpenAPI 文档（gen bin 与 contracts 剩余 stub 聚合，design D2）。
#[derive(OpenApi)]
#[openapi(paths(
    notes::create_note,
    notes::read_note,
    notes::search_content,
    projects::list_projects,
    stream::get_stream,
))]
pub struct ServerApiDoc;

/// 服务配置（端口 / token / 数据目录；默认值对齐 .env.example）。
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub token: String,
    pub data_dir: PathBuf,
}

impl ServerConfig {
    /// 从环境变量构造（`JOTLINE_API_PORT` / `JOTLINE_DEV_TOKEN` / `JOTLINE_DATA_DIR`）。
    pub fn from_env() -> Self {
        Self {
            port: std::env::var("JOTLINE_API_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(DEFAULT_PORT),
            token: std::env::var("JOTLINE_DEV_TOKEN").unwrap_or_else(|_| DEFAULT_TOKEN.into()),
            data_dir: std::env::var("JOTLINE_DATA_DIR")
                .unwrap_or_else(|_| DEFAULT_DATA_DIR.into())
                .into(),
        }
    }
}

/// serve() 句柄：就绪地址 + 关停信号 + sidecar 子进程。
pub struct Server {
    pub addr: SocketAddr,
    shutdown_tx: watch::Sender<bool>,
    join: tokio::task::JoinHandle<()>,
    sidecar: Option<sidecar::SidecarHandle>,
}

impl Server {
    /// 关停：停止接受连接并断开存量连接（SSE 长连接随服务任务结束关闭），
    /// 随后杀掉 sidecar（不留孤儿进程，task 5.3）。
    pub async fn shutdown(mut self) {
        let _ = self.shutdown_tx.send(true);
        let _ = self.join.await;
        if let Some(mut sc) = self.sidecar.take() {
            sc.kill().await;
        }
    }
}

/// 启动领域服务：初始化存储 → 绑定 `127.0.0.1:{port}` → 立即返回（就绪语义）。
pub async fn serve(config: ServerConfig) -> std::io::Result<Server> {
    let data_dir = config.data_dir.clone();
    let init_dir = data_dir.clone();
    let storage = tokio::task::spawn_blocking(move || storage::Storage::init(&init_dir))
        .await
        .expect("存储初始化任务 panic")
        .map_err(|e| {
            std::io::Error::other(format!("数据目录初始化失败（{}）：{e}", data_dir.display()))
        })?;
    let state = Arc::new(AppState {
        token: config.token.clone(),
        storage: Arc::new(storage),
        events: broadcast::channel(64).0,
        event_seq: AtomicU64::new(0),
        sidecar_boot_fired: watch::channel(false).0,
    });
    let app = router(state.clone());
    let listener = TcpListener::bind(("127.0.0.1", config.port)).await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);
    let join = tokio::spawn(async move {
        tokio::select! {
            _ = axum::serve(listener, app) => {}
            _ = shutdown_rx.changed() => {}
        }
    });
    tracing::info!(
        target: "rust.server",
        addr = %addr,
        data_dir = %config.data_dir.display(),
        "主进程已监听"
    );
    // axum 就绪后拉起 sidecar（boot 链后台执行，不阻塞就绪，design D6）
    let sidecar = sidecar::spawn_and_boot(state, config.port).await;
    Ok(Server {
        addr,
        shutdown_tx,
        join,
        sidecar,
    })
}

/// 路由组装：契约路由（注解携带全路径 `/api/...`，鉴权中间件内层）
/// + 契约外 404 信封 + CORS 外层。
pub fn router(state: Arc<AppState>) -> axum::Router {
    // utoipa-axum OpenApiRouter：路由与 path 注解同源注册（IDR-03）
    let api: OpenApiRouter = api_routes()
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_bearer,
        ))
        .with_state(state);
    let (api, _api_doc) = api.split_for_parts();
    axum::Router::new()
        .merge(api)
        .fallback(fallback)
        .layer(cors_layer())
}

/// 契约路由注册链（不含鉴权层与 state 绑定；双注册一致性测试消费，design D3）。
fn api_routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(notes::create_note))
        .routes(routes!(notes::read_note))
        .routes(routes!(notes::search_content))
        .routes(routes!(projects::list_projects))
        .routes(routes!(stream::get_stream))
}

/// 契约外路由统一 404 错误信封。
async fn fallback(method: Method, uri: Uri) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorEnvelope::new(
            ErrorCode::NotFound,
            format!("契约外路由：{} {}", method, uri.path()),
        )),
    )
        .into_response()
}

/// CORS 层（design D5）：前端 dev server（:1420）→ 主进程（:4765）跨源。
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.to_str().is_ok_and(|o| {
                o.starts_with("http://localhost:") || o.starts_with("http://127.0.0.1:")
            })
        }))
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
}

/// 初始化日志（SPEC §5.1：tracing-subscriber 控制台渲染；中文 message + 英文 fields）。
/// dev 默认级别：应用 target（rust.*）DEBUG、其余 INFO；`RUST_LOG` 可整体覆盖。
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    tracing_subscriber::fmt()
        .with_target(true)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,rust=debug")),
        )
        .init();
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use utoipa::OpenApi;

    /// 双注册一致性守卫（design D3）：路由注册（`OpenApiRouter` 拆出的 path 集合）
    /// == `ServerApiDoc` path 集合。一侧新增或遗漏端点即失败并打印两侧差集，
    /// 堲「漏登记 = 端点静默从契约消失」。
    #[test]
    fn route_registration_matches_doc_registration() {
        let (_, route_doc) = super::api_routes().split_for_parts();
        let route_paths: BTreeSet<String> = route_doc.paths.paths.keys().cloned().collect();
        let doc_paths: BTreeSet<String> = super::ServerApiDoc::openapi()
            .paths
            .paths
            .keys()
            .cloned()
            .collect();

        let route_only: Vec<_> = route_paths.difference(&doc_paths).collect();
        let doc_only: Vec<_> = doc_paths.difference(&route_paths).collect();
        assert!(
            route_only.is_empty() && doc_only.is_empty(),
            "路由注册与文档注册不一致：仅路由注册 = {route_only:?}，仅文档注册 = {doc_only:?}"
        );
    }
}
