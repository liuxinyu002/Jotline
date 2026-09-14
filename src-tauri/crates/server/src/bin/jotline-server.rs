//! thin bin：读环境变量 → `serve()`（lib 形态见 design D1）。

#[tokio::main]
async fn main() -> std::io::Result<()> {
    server::init_tracing();
    let config = server::ServerConfig::from_env();
    let server = server::serve(config).await?;
    tokio::signal::ctrl_c().await?;
    tracing::info!(target: "rust.server", "收到中断信号，主进程退出");
    server.shutdown().await;
    Ok(())
}
