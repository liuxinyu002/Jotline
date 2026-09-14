//! sidecar 拉起链与生命周期（design D6，sidecar-runtime spec）：
//!
//! spawn（bun 开发态直跑，env 注入回调配置，CWD=仓库根）→ `health.ping` 往返
//! → sidecar HTTP 回调观测（auth 中间件置位 watch）→ 日志「ADR-2 通道验证 ✓」。
//! 任一步超时或失败仅记 WARN，主进程继续服务、不阻塞就绪、不重试（重启时序属 Phase-8）。
//!
//! 生命周期：主进程退出（SIGINT 显式 kill / panic 经 kill_on_drop 兜底）时杀掉 sidecar。
//! stdout 纪律：IPC 流每行必须是可解析 JSON-RPC（reader 任务持续观测，违规记 WARN）；
//! stderr：NDJSON 日志由主进程解析后按级别转发（target = sidecar，SPEC §5.1 拓扑）。

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;

use contracts::ipc::{IpcHealthRequest, IpcResponse};

use crate::AppState;

/// 拉起链（spawn → pong → 回调观测）整体限时（design D6）。
const BOOT_CHAIN_TIMEOUT: Duration = Duration::from_secs(5);
/// 回调观测单步限时（design D6）。
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(2);

/// boot 链 ping 请求 id（pong 匹配锚点）。
const BOOT_PING_ID: &str = "boot-1";

/// sidecar 子进程句柄（持有 stdin 槽防止 IPC 流 EOF；随主进程退出被 kill）。
pub struct SidecarHandle {
    child: Child,
    /// stdin 保活槽（boot 链用毕归还；drop 即 sidecar 读到 EOF）。
    _stdin: Arc<Mutex<Option<ChildStdin>>>,
}

impl SidecarHandle {
    /// 杀掉 sidecar（主进程退出路径；幂等）。
    pub async fn kill(&mut self) {
        if self.child.start_kill().is_ok() {
            let _ = self.child.wait().await;
            tracing::info!(target: "rust.sidecar", "sidecar 已终止");
        }
    }
}

/// stderr NDJSON 行形态（sidecar 日志契约，task 5.1）。
#[derive(Deserialize)]
struct SidecarLogLine {
    level: String,
    msg: String,
}
/// 拉起 sidecar 并启动 boot 链。失败仅记 WARN 并返回 `None`（不阻塞主进程就绪）。
pub async fn spawn_and_boot(state: Arc<AppState>, port: u16) -> Option<SidecarHandle> {
    let repo_root = match crate::repo_root() {
        Ok(root) => root,
        Err(e) => {
            tracing::warn!(
                target: "rust.sidecar",
                error = %e,
                "仓库根定位失败，sidecar 不拉起（不影响主进程服务）"
            );
            return None;
        }
    };
    let mut command = Command::new("bun");
    command
        .args(["run", "sidecar/src/index.ts"])
        .current_dir(&repo_root)
        .env("JOTLINE_API_PORT", port.to_string())
        .env("JOTLINE_DEV_TOKEN", &state.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true); // panic / 任何 drop 路径兜底（task 5.3）

    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                target: "rust.sidecar",
                error = %e,
                "sidecar 拉起失败（bun 是否在 PATH？不影响主进程服务）"
            );
            return None;
        }
    };
    tracing::info!(target: "rust.sidecar", pid = child.id().unwrap_or(0), "sidecar 已拉起");

    let stdin_slot: Arc<Mutex<Option<ChildStdin>>> = Arc::new(Mutex::new(child.stdin.take()));
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    // stdout reader：IPC 流纯净性观测 + pong 转发
    let (ipc_tx, ipc_rx) = mpsc::channel::<IpcResponse>(16);
    if let Some(stdout) = stdout {
        tokio::spawn(read_stdout(stdout, ipc_tx));
    }
    // stderr reader：NDJSON 解析 → tracing 转发（target = sidecar）
    if let Some(stderr) = stderr {
        tokio::spawn(read_stderr(stderr));
    }

    // boot 链（后台任务：不阻塞 serve() 返回 / 就绪语义）
    tokio::spawn(boot_chain(state.clone(), stdin_slot.clone(), ipc_rx));

    Some(SidecarHandle {
        child,
        _stdin: stdin_slot,
    })
}

/// boot 链：ping → pong → 回调观测 → 「ADR-2 通道验证 ✓」；任一步失败 WARN。
async fn boot_chain(
    state: Arc<AppState>,
    stdin_slot: Arc<Mutex<Option<ChildStdin>>>,
    mut ipc_rx: mpsc::Receiver<IpcResponse>,
) {
    let chain = async {
        // 1) 发 ping（从保活槽借出 stdin，用毕归还）
        let ping = IpcHealthRequest::new(BOOT_PING_ID);
        let mut stdin = {
            let mut slot = stdin_slot.lock().await;
            match slot.take() {
                Some(s) => s,
                None => return Err("stdin 不可用"),
            }
        };
        let line = format!(
            "{}\n",
            serde_json::to_string(&ping).expect("ping 序列化不可失败")
        );
        if stdin.write_all(line.as_bytes()).await.is_err() {
            *stdin_slot.lock().await = Some(stdin);
            return Err("ping 写入失败");
        }
        let _ = stdin.flush().await;
        *stdin_slot.lock().await = Some(stdin);

        // 2) 等 pong（同 id）
        loop {
            match ipc_rx.recv().await {
                Some(resp) if resp.id == BOOT_PING_ID => break,
                Some(_) => continue, // 未来的其他响应
                None => return Err("sidecar stdout 已关闭（进程退出？）"),
            }
        }
        tracing::info!(target: "rust.sidecar", id = BOOT_PING_ID, "sidecar pong 已收到");

        // 3) 等回调观测（auth 中间件置位；单步限时 2s）
        let mut boot_rx = state.sidecar_boot_fired.subscribe();
        if !*boot_rx.borrow_and_update()
            && timeout(CALLBACK_TIMEOUT, boot_rx.changed()).await.is_err()
        {
            return Err("回调观测超时（sidecar 未回调 /api/projects）");
        }
        Ok::<(), &str>(())
    };

    match timeout(BOOT_CHAIN_TIMEOUT, chain).await {
        Ok(Ok(())) => tracing::info!(target: "rust.sidecar", "ADR-2 通道验证 ✓"),
        Ok(Err(step)) => {
            tracing::warn!(target: "rust.sidecar", step, "ADR-2 通道验证未完成（不阻塞服务，不重试）");
        }
        Err(_) => {
            tracing::warn!(target: "rust.sidecar", "ADR-2 拉起链超时（不阻塞服务，不重试）");
        }
    }
}

/// stdout 读取：每行须为可解析 JSON-RPC（IPC 纯净性，task 5.3）；解析失败记 WARN。
async fn read_stdout(stdout: tokio::process::ChildStdout, ipc_tx: mpsc::Sender<IpcResponse>) {
    let mut lines = BufReader::new(stdout).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<IpcResponse>(trimmed) {
            Ok(resp) => {
                let _ = ipc_tx.send(resp).await;
            }
            Err(_) => {
                tracing::warn!(
                    target: "rust.sidecar",
                    line = trimmed.chars().take(100).collect::<String>(),
                    "sidecar stdout 出现非 JSON-RPC 行（IPC 流纯净性被破坏）"
                );
            }
        }
    }
}

/// stderr 读取：NDJSON → 按 level 转发 tracing（target = sidecar，SPEC §5.1 主进程汇聚）。
async fn read_stderr(stderr: tokio::process::ChildStderr) {
    let mut lines = BufReader::new(stderr).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<SidecarLogLine>(trimmed) {
            Ok(entry) => match entry.level.as_str() {
                "warn" => tracing::warn!(target: "sidecar", "{}", entry.msg),
                "error" => tracing::error!(target: "sidecar", "{}", entry.msg),
                _ => tracing::info!(target: "sidecar", "{}", entry.msg),
            },
            Err(_) => {
                tracing::warn!(target: "sidecar", line = trimmed, "sidecar stderr 非 NDJSON 行")
            }
        }
    }
}