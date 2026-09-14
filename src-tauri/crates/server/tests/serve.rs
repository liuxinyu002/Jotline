//! serve() 集成测试（domain-api spec「本地监听与鉴权执行」的真实进程栈）：
//! 仅监听 127.0.0.1（port 0 临时端口）、真实 socket 上无 token → 401 错误信封、干净关停。
//!
//! sidecar 拉起链在无 bun 环境下仅记 WARN 并跳过（design D6 降级路径），不影响本测试。

use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use server::{serve, ServerConfig};

#[tokio::test]
async fn serve_binds_loopback_and_enforces_auth_on_real_socket() {
    let dir = tempfile::tempdir().unwrap();
    let server = serve(ServerConfig {
        port: 0, // 临时端口（仅验证绑定语义，不占固定 4765）
        token: "dev-token".into(),
        data_dir: dir.path().to_path_buf(),
    })
    .await
    .expect("serve 应就绪返回");

    // 仅监听回环地址（不暴露局域网）
    assert!(server.addr.ip().is_loopback(), "{}", server.addr);

    // 真实 socket：无 token 请求契约路由 → 401 统一错误信封
    let mut sock = tokio::net::TcpStream::connect(server.addr).await.unwrap();
    sock.write_all(
        format!(
            "GET /api/projects HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            server.addr
        )
        .as_bytes(),
    )
    .await
    .unwrap();
    let mut buf = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), sock.read_to_end(&mut buf))
        .await
        .expect("响应超时")
        .expect("读取失败");
    let head = String::from_utf8_lossy(&buf);
    assert!(head.starts_with("HTTP/1.1 401"), "{head}");
    assert!(head.contains("\"code\":\"unauthorized\""), "{head}");

    // 干净关停（不悬挂、不留孤儿进程句柄）
    tokio::time::timeout(Duration::from_secs(10), server.shutdown())
        .await
        .expect("关停超时");
}
